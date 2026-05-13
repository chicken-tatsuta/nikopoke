# Lv2 AI Roadmap

## Context

Current AI levels:

- `lv1`: minimax depth 1
- `lv2`: MLP policy that maps state features to 6 fixed actions, 4 moves plus 2 switches

The 60-generation self-play model still loses heavily in direct `lv1` vs `lv2` tests. The latest benchmark was:

```text
lv1 wins: 77
lv2 wins: 11
draws: 12
```

This suggests the current MLP policy is not yet learning the tactical structure that `lv1` gets from even shallow minimax search.

## Why Lv2 Loses To Lv1

`lv1` has a simple but important advantage: it avoids obvious one-turn disasters. Even at depth 1, minimax can account for immediate outcomes such as:

- taking a confirmed KO
- avoiding a move that fails because of PP
- avoiding a switch target that immediately faints
- choosing a move that prevents the opponent from getting a free KO

The current `lv2` learns mostly from sparse final match results. That makes it easy for evolution to get stuck in local optima, especially when the losing pattern is a small tactical mistake repeated across many positions.

The likely major failure modes are:

- missing confirmed KO moves
- choosing low-value or PP-empty moves
- switching into immediate KO
- failing to switch when the active Pokemon is about to faint
- treating move choices and switch choices as fixed output slots rather than comparable actions

## Design Direction

The recommended direction is:

```text
rule bonus + action scorer + value model
```

The existing fixed 6-class policy should stop being the main decision-maker. It can remain as a helper, or be replaced entirely once action scoring is strong enough.

## Design Comparison

| Approach | Chance To Beat Lv1 | Rust Inference | Browser Inference | Cost | Notes |
| --- | --- | --- | --- | --- | --- |
| Rule bonus only | Medium-low | Easy | Easy | Low | Good for removing obvious mistakes, limited coverage |
| Current MLP policy | Low-medium | Easy | Easy | Done | Already underperforming after 60 generations |
| Rule + MLP action scorer | High | Easy | Easy | Medium | Best next step |
| XGBoost / LightGBM scorer | High | Medium-hard | Medium-hard | Medium-high | Strong fit for tabular features, but deployment is awkward |
| Policy-value | Very high | Medium | Medium-hard | High | Strongest long-term design, more training complexity |
| Rule + value model | High | Easy-medium | Easy-medium | Medium | Good cost/performance once rule scorer exists |

XGBoost and LightGBM are plausible from a modeling perspective, but the Rust and browser deployment cost is probably not worth it yet. A small MLP scorer plus rule bonus is easier to keep consistent across Rust self-play and frontend runtime.

## Phase 1: Rule Bonus

Goal: remove obvious tactical mistakes before asking learning to solve deeper strategy.

Add a deterministic `rule_bonus(state, action)` layer after model scoring:

```text
if action is PP-empty:
  exclude action

if action is a confirmed KO:
  large bonus

if action moves first and is a confirmed KO:
  extra bonus

if action switches into immediate KO:
  large penalty

if active Pokemon is likely to faint and action is a safe switch:
  bonus
```

This phase is mostly engineering, not ML. It should improve consistency and make later training less noisy.

Recommended first weights:

```text
confirmed KO: +2.0
first confirmed KO: +1.0
PP-empty move: -9999
immediate-death switch: -3.0
safe switch while threatened: +1.0
```

These values should be tuned by `lv1 vs lv2` benchmark results, not by intuition.

## Phase 2: Action Scorer

Goal: replace fixed-slot policy with a unified `(state, action) -> score` model.

Instead of outputting 6 logits for fixed slots, enumerate all legal actions and score each action independently:

```text
score = action_scorer(concat(state_features, action_features))
final_score = score + rule_bonus(state, action)
best_action = argmax(final_score)
```

This lets moves and switches compete in the same space. It also avoids overfitting to "move slot 1" or "switch slot 2" as arbitrary positions.

Action features should include:

- action type: move or switch
- move power, accuracy, priority
- move PP ratio, PP empty flag, low PP flag
- type effectiveness against opponent active
- STAB flag
- whether action can KO this turn
- whether user moves first
- whether user likely faints before acting
- for switches: target HP ratio
- for switches: target defensive matchup vs opponent
- for switches: target best offensive matchup vs opponent
- for switches: target speed ratio vs opponent
- for switches: whether target dies immediately

Training should prefer ranking loss over simple fixed-class policy loss:

```text
chosen action score > other legal action scores
```

Possible labels:

- `lv1` chosen action as imitation target
- self-play winning actions as positive examples
- one-step heuristic outcome as dense target

This phase is the most important one for beating `lv1`.

## Phase 3: Value Model

Goal: make `lv2` evaluate the board after an action, not only the action itself.

Add a second model:

```text
value_model(state) -> [-1, 1]
```

Then score actions with a one-step lookahead:

```text
next_state = simulate(state, action, opponent_guess)

final_score =
  action_scorer(state, action)
  + value_weight * value_model(next_state)
  + rule_bonus(state, action)
```

Initial `opponent_guess` can be `lv1` behavior. That gives `lv2` a practical target: beat a depth-1 minimax-like opponent.

Value model training data can come from self-play:

```text
(state, final_game_result)
```

Later, TD-style targets can be added, but final-result supervision is enough for a first version.

Recommended initial weight:

```text
value_weight = 0.5
```

Tune this against benchmark results.

## Deployment Notes

For the current codebase, prefer a small MLP implemented directly in Rust and TypeScript first.

ONNX can be considered later:

```text
Python training
  -> ONNX export
  -> Rust: ort
  -> Browser: onnxruntime-web
```

However, ONNX adds dependency and packaging complexity. It is probably premature until the action scorer and value architecture are proven.

Recommended near-term deployment:

- keep model weights as JSON
- implement the same tiny MLP evaluator in Rust and TypeScript
- keep rule bonus duplicated in Rust and TypeScript
- add tests or fixtures for feature parity between Rust and frontend

## Recommended Implementation Order

1. Add rule bonus to the current MLP policy.
2. Add `lv1` mixed evaluation to training fitness.
3. Replace fixed 6-output policy with action scorer.
4. Train action scorer with imitation data from `lv1`.
5. Fine-tune with self-play and `lv1` benchmark fitness.
6. Add value model and one-step lookahead.
7. Re-run `lv1 vs lv2` benchmark after each phase.

## Success Criteria

Track the same direct benchmark after every phase:

```text
100 battles
same team on both sides
lv1/lv2 side alternates each battle
80-turn cutoff treated as draw
```

Short-term target:

```text
lv2 wins >= 35 / 100
```

Medium-term target:

```text
lv2 wins >= lv1 wins
```

Long-term target:

```text
lv2 beats lv1 while also performing well against random teams and self-play opponents
```

Do not optimize only for the direct same-team benchmark. The final AI should still feel strong in normal player-facing battles.
