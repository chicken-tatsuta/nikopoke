use crate::core::battle::{is_battle_over, step_battle, BattleOptions};
use crate::core::state::{Action, ActionType, BattleState, CreatureState, PlayerState};
use crate::core::utils::get_active_creature;
use crate::data::moves::{Effect, MoveData, MoveDatabase};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
type SearchTimer = Instant;

#[cfg(target_arch = "wasm32")]
struct SearchTimer;

#[cfg(not(target_arch = "wasm32"))]
fn start_search_timer() -> SearchTimer {
    Instant::now()
}

#[cfg(target_arch = "wasm32")]
fn start_search_timer() -> SearchTimer {
    SearchTimer
}

#[cfg(not(target_arch = "wasm32"))]
fn record_search_elapsed(stats: &mut VegaStats, timer: &SearchTimer) {
    stats.elapsed_ns += timer.elapsed().as_nanos();
}

#[cfg(target_arch = "wasm32")]
fn record_search_elapsed(_stats: &mut VegaStats, _timer: &SearchTimer) {}

const WIN_SCORE: f32 = 1_000_000.0;
const DEFAULT_BRANCH_LIMIT: usize = 4;
const TACTICAL_CONFIRMED_KO_BONUS: f32 = 5_000.0;
const TACTICAL_FAST_KO_BONUS: f32 = 2_500.0;
const TACTICAL_SELF_DEATH_PENALTY: f32 = -3_500.0;
const TACTICAL_DEAD_SWITCH_PENALTY: f32 = -5_000.0;
const TACTICAL_SAFE_SWITCH_BONUS: f32 = 1_200.0;

#[derive(Clone, Copy, Debug)]
pub struct VegaParams {
    pub alive: f32,
    pub hp: f32,
    pub hp_ratio: f32,
    pub active_hp: f32,
    pub outgoing: f32,
    pub incoming: f32,
    pub ko_fast: f32,
    pub ko_slow: f32,
    pub risk_fast: f32,
    pub risk_slow: f32,
    pub speed: f32,
    pub bench: f32,
    pub stage: f32,
    pub status: f32,
    pub switch_pressure: f32,
    pub switch_danger: f32,
    pub switch_hp: f32,
    pub action_damage: f32,
    pub action_priority_ko: f32,
    pub action_accuracy: f32,
}

pub const DEFAULT_PARAMS: VegaParams = VegaParams {
    alive: 520.0,
    hp: 1.0,
    hp_ratio: 278.94696,
    active_hp: 61.97765,
    outgoing: 264.4011,
    incoming: 129.00804,
    ko_fast: 327.92148,
    ko_slow: 54.109386,
    risk_fast: 163.9226,
    risk_slow: 231.91446,
    speed: 21.1501,
    bench: 12.346732,
    stage: 1.0882639,
    status: 1.1856047,
    switch_pressure: 77.31166,
    switch_danger: 199.47752,
    switch_hp: 96.20694,
    action_damage: 185.30957,
    action_priority_ko: 114.034775,
    action_accuracy: 20.0,
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct VegaStats {
    pub searches: u64,
    pub elapsed_ns: u128,
    pub root_actions: u64,
    pub nodes_entered: u64,
    pub leaf_evals: u64,
    pub ordered_actions_calls: u64,
    pub ordered_actions_scored: u64,
    pub ordered_actions_returned: u64,
    pub step_battle_calls: u64,
    pub alpha_cutoffs: u64,
    pub completed_depth: u64,
    pub aborted: bool,
}

struct VegaContext<'a> {
    move_db: &'a MoveDatabase,
    params: VegaParams,
    branch_limit: usize,
    node_budget: u64,
    tt: Option<&'a RefCell<TranspositionTable>>,
}

struct TacticalContext {
    incoming_to_active: DamageSummary,
}

// --- Transposition Table ---

#[derive(Clone, Copy, Debug)]
enum TTFlag {
    Exact,
}

#[derive(Clone, Debug)]
struct TTEntry {
    hash: u64,
    depth: usize,
    score: f32,
    flag: TTFlag,
}

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    mask: usize,
}

const TT_DEFAULT_SIZE: usize = 1 << 16; // 65536 entries

impl TranspositionTable {
    pub fn new() -> Self {
        Self::with_size(TT_DEFAULT_SIZE)
    }

    pub fn with_size(size: usize) -> Self {
        let size = size.next_power_of_two();
        Self {
            entries: vec![None; size],
            mask: size - 1,
        }
    }

    fn probe(&self, hash: u64) -> Option<&TTEntry> {
        let idx = (hash as usize) & self.mask;
        self.entries[idx]
            .as_ref()
            .filter(|entry| entry.hash == hash)
    }

    fn store(&mut self, hash: u64, depth: usize, score: f32, flag: TTFlag) {
        let idx = (hash as usize) & self.mask;
        // Always replace: deeper or newer entries are preferred
        let should_replace = self.entries[idx]
            .as_ref()
            .map_or(true, |existing| existing.depth <= depth || existing.hash != hash);
        if should_replace {
            self.entries[idx] = Some(TTEntry {
                hash,
                depth,
                score,
                flag,
            });
        }
    }

    pub fn clear(&mut self) {
        self.entries.fill(None);
    }
}

fn hash_battle_state(state: &BattleState, player_id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // Hash each player
    for player in &state.players {
        // player identity relative to us
        let is_self = player.id == player_id;
        is_self.hash(&mut hasher);
        player.active_slot.hash(&mut hasher);
        for creature in &player.team {
            creature.id.hash(&mut hasher);
            creature.hp.hash(&mut hasher);
            // stages
            creature.stages.atk.hash(&mut hasher);
            creature.stages.def.hash(&mut hasher);
            creature.stages.spa.hash(&mut hasher);
            creature.stages.spd.hash(&mut hasher);
            creature.stages.spe.hash(&mut hasher);
            // statuses (sorted for consistency)
            let mut status_ids: Vec<&str> = creature.statuses.iter().map(|s| s.id.as_str()).collect();
            status_ids.sort_unstable();
            for sid in &status_ids {
                sid.hash(&mut hasher);
            }
            // PP matters for search
            let mut pp_pairs: Vec<(&str, i32)> = creature
                .move_pp
                .iter()
                .map(|(k, v)| (k.as_str(), *v))
                .collect();
            pp_pairs.sort_unstable_by_key(|(k, _)| *k);
            for (k, v) in &pp_pairs {
                k.hash(&mut hasher);
                v.hash(&mut hasher);
            }
        }
    }
    // Field state
    let mut global_ids: Vec<&str> = state.field.global.iter().map(|e| e.id.as_str()).collect();
    global_ids.sort_unstable();
    for gid in &global_ids {
        gid.hash(&mut hasher);
    }
    for (side_id, effects) in &state.field.sides {
        side_id.hash(&mut hasher);
        let mut eids: Vec<&str> = effects.iter().map(|e| e.id.as_str()).collect();
        eids.sort_unstable();
        for eid in &eids {
            eid.hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub fn get_best_move_vega(state: &BattleState, player_id: &str, depth: usize) -> Option<Action> {
    get_best_move_vega_with_params(state, player_id, depth, DEFAULT_PARAMS)
}

pub fn get_best_move_vega_with_params(
    state: &BattleState,
    player_id: &str,
    depth: usize,
    params: VegaParams,
) -> Option<Action> {
    get_best_move_vega_with_options(state, player_id, depth, params, DEFAULT_BRANCH_LIMIT)
}

pub fn get_best_move_vega_with_options(
    state: &BattleState,
    player_id: &str,
    depth: usize,
    params: VegaParams,
    branch_limit: usize,
) -> Option<Action> {
    let move_db = MoveDatabase::default();
    get_best_move_vega_with_options_and_db_ref(
        state,
        player_id,
        depth,
        params,
        branch_limit,
        &move_db,
    )
}

pub fn get_best_move_vega_with_options_and_db(
    state: &BattleState,
    player_id: &str,
    depth: usize,
    params: VegaParams,
    branch_limit: usize,
    move_db: MoveDatabase,
) -> Option<Action> {
    get_best_move_vega_with_options_and_db_ref(
        state,
        player_id,
        depth,
        params,
        branch_limit,
        &move_db,
    )
}

pub fn get_best_move_vega_with_options_and_db_ref(
    state: &BattleState,
    player_id: &str,
    depth: usize,
    params: VegaParams,
    branch_limit: usize,
    move_db: &MoveDatabase,
) -> Option<Action> {
    let mut stats = VegaStats::default();
    get_best_move_vega_with_options_and_db_ref_and_stats(
        state,
        player_id,
        depth,
        params,
        branch_limit,
        move_db,
        &mut stats,
    )
}

pub fn get_best_move_vega_with_options_and_db_ref_and_stats(
    state: &BattleState,
    player_id: &str,
    depth: usize,
    params: VegaParams,
    branch_limit: usize,
    move_db: &MoveDatabase,
    stats: &mut VegaStats,
) -> Option<Action> {
    stats.searches += 1;
    let started_at = start_search_timer();
    let ctx = VegaContext {
        move_db,
        params,
        branch_limit: branch_limit.max(1),
        node_budget: u64::MAX,
        tt: None,
    };
    let actions = ordered_actions(state, player_id, &ctx, stats);
    stats.root_actions += actions.len() as u64;
    if actions.is_empty() {
        record_search_elapsed(stats, &started_at);
        return None;
    }

    let Some(opp_id) = opponent_id(state, player_id) else {
        record_search_elapsed(stats, &started_at);
        return actions.first().cloned();
    };

    let mut best_action = actions.first().cloned();
    let mut best_score = f32::NEG_INFINITY;
    let mut alpha = f32::NEG_INFINITY;
    let search_depth = depth.max(1);

    for action in actions.iter().take(ctx.branch_limit) {
        let score = worst_opponent_reply(
            state,
            player_id,
            opp_id.as_str(),
            action,
            &ctx,
            search_depth - 1,
            alpha,
            stats,
        );
        if stats.aborted {
            break;
        }
        if score > best_score {
            best_score = score;
            best_action = Some(action.clone());
        }
        alpha = alpha.max(best_score);
    }

    record_search_elapsed(stats, &started_at);
    best_action
}

/// Iterative deepening search: starts at depth 1 and goes deeper until
/// the node budget is exhausted. Returns the best action from the deepest
/// fully completed iteration.
pub fn get_best_move_vega_iterative(
    state: &BattleState,
    player_id: &str,
    max_depth: usize,
    node_budget: u64,
    params: VegaParams,
    branch_limit: usize,
    move_db: &MoveDatabase,
    stats: &mut VegaStats,
) -> Option<Action> {
    stats.searches += 1;
    let started_at = start_search_timer();
    let max_depth = max_depth.max(1);
    let mut best_action: Option<Action> = None;
    let tt = RefCell::new(TranspositionTable::new());

    for depth in 1..=max_depth {
        let mut iter_stats = VegaStats::default();
        let remaining = node_budget.saturating_sub(stats.nodes_entered);
        if remaining < 100 {
            break;
        }
        let ctx = VegaContext {
            move_db,
            params,
            branch_limit: branch_limit.max(1),
            node_budget: remaining,
            tt: Some(&tt),
        };
        let actions = ordered_actions(state, player_id, &ctx, &mut iter_stats);
        if actions.is_empty() {
            break;
        }
        let Some(opp_id) = opponent_id(state, player_id) else {
            best_action = actions.first().cloned();
            break;
        };

        let mut depth_best: Option<Action> = actions.first().cloned();
        let mut depth_best_score = f32::NEG_INFINITY;
        let mut alpha = f32::NEG_INFINITY;
        let mut aborted = false;

        for action in actions.iter().take(ctx.branch_limit) {
            let score = worst_opponent_reply(
                state,
                player_id,
                opp_id.as_str(),
                action,
                &ctx,
                depth.saturating_sub(1),
                alpha,
                &mut iter_stats,
            );
            if iter_stats.aborted {
                aborted = true;
                break;
            }
            if score > depth_best_score {
                depth_best_score = score;
                depth_best = Some(action.clone());
            }
            alpha = alpha.max(depth_best_score);
        }

        // Merge iter_stats into main stats
        stats.nodes_entered += iter_stats.nodes_entered;
        stats.leaf_evals += iter_stats.leaf_evals;
        stats.step_battle_calls += iter_stats.step_battle_calls;
        stats.alpha_cutoffs += iter_stats.alpha_cutoffs;
        stats.ordered_actions_calls += iter_stats.ordered_actions_calls;
        stats.ordered_actions_scored += iter_stats.ordered_actions_scored;
        stats.ordered_actions_returned += iter_stats.ordered_actions_returned;

        if !aborted {
            best_action = depth_best;
            stats.completed_depth = depth as u64;
        } else {
            stats.aborted = true;
            break;
        }
    }

    record_search_elapsed(stats, &started_at);
    best_action
}

fn worst_opponent_reply(
    state: &BattleState,
    player_id: &str,
    opponent_id: &str,
    action: &Action,
    ctx: &VegaContext,
    depth_left: usize,
    alpha: f32,
    stats: &mut VegaStats,
) -> f32 {
    stats.nodes_entered += 1;
    if stats.nodes_entered >= ctx.node_budget {
        stats.aborted = true;
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }
    let opponent_actions = ordered_actions(state, opponent_id, ctx, stats);
    if opponent_actions.is_empty() {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    let mut worst = f32::INFINITY;
    for opponent_action in opponent_actions.iter() {
        let actions = vec![action.clone(), opponent_action.clone()];
        let mut rng = || 0.42;
        stats.step_battle_calls += 1;
        let next = step_battle(
            state,
            &actions,
            &mut rng,
            BattleOptions {
                record_history: false,
            },
        );
        let score = if depth_left == 0 {
            quiescence_eval(&next, player_id, ctx, 2, stats)
        } else {
            best_continuation(&next, player_id, ctx, depth_left, stats)
        };
        if stats.aborted {
            break;
        }

        worst = worst.min(score);
        if worst <= alpha {
            stats.alpha_cutoffs += 1;
            break;
        }
    }

    worst
}

/// Quiescence search: if the position is tactically unstable (someone can KO
/// or a forced switch is needed), extend the search up to `qs_depth` extra plies.
/// Otherwise return the static evaluation immediately.
fn quiescence_eval(
    state: &BattleState,
    player_id: &str,
    ctx: &VegaContext,
    qs_depth: usize,
    stats: &mut VegaStats,
) -> f32 {
    if stats.nodes_entered >= ctx.node_budget {
        stats.aborted = true;
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }
    if qs_depth == 0 || is_battle_over(state) {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    // Check if the position is tactically unstable
    let player = get_player(state, player_id);
    let opponent = get_opponent(state, player_id);
    let is_unstable = match (player, opponent) {
        (Some(p), Some(o)) => {
            let p_active = p.team.get(p.active_slot);
            let o_active = o.team.get(o.active_slot);
            match (p_active, o_active) {
                (Some(pa), Some(oa)) => {
                    // Someone is dead → forced switch, keep searching
                    pa.hp <= 0
                        || oa.hp <= 0
                        // Either side can KO this turn
                        || max_expected_damage(pa, oa, ctx.move_db).damage >= oa.hp as f32
                        || max_expected_damage(oa, pa, ctx.move_db).damage >= pa.hp as f32
                }
                _ => false,
            }
        }
        _ => false,
    };

    if !is_unstable {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    // Position is unstable — search one more ply
    best_continuation_qs(state, player_id, ctx, qs_depth, stats)
}

/// Like best_continuation but for quiescence: narrower search (top 2 moves only).
fn best_continuation_qs(
    state: &BattleState,
    player_id: &str,
    ctx: &VegaContext,
    qs_depth: usize,
    stats: &mut VegaStats,
) -> f32 {
    stats.nodes_entered += 1;
    if stats.nodes_entered >= ctx.node_budget {
        stats.aborted = true;
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }
    if is_battle_over(state) {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    let Some(opp_id) = opponent_id(state, player_id) else {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    };
    let actions = ordered_actions(state, player_id, ctx, stats);
    if actions.is_empty() {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    // Narrower search in quiescence: only top 2 moves
    let qs_branch = ctx.branch_limit.min(2);
    let mut best = f32::NEG_INFINITY;
    let mut alpha = f32::NEG_INFINITY;
    for action in actions.iter().take(qs_branch) {
        let score = worst_opponent_reply_qs(
            state,
            player_id,
            opp_id.as_str(),
            action,
            ctx,
            qs_depth,
            alpha,
            stats,
        );
        if stats.aborted {
            break;
        }
        best = best.max(score);
        alpha = alpha.max(best);
    }

    best
}

fn worst_opponent_reply_qs(
    state: &BattleState,
    player_id: &str,
    opponent_id: &str,
    action: &Action,
    ctx: &VegaContext,
    qs_depth: usize,
    alpha: f32,
    stats: &mut VegaStats,
) -> f32 {
    stats.nodes_entered += 1;
    if stats.nodes_entered >= ctx.node_budget {
        stats.aborted = true;
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }
    let opponent_actions = ordered_actions(state, opponent_id, ctx, stats);
    if opponent_actions.is_empty() {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    let qs_branch = ctx.branch_limit.min(2);
    let mut worst = f32::INFINITY;
    for opponent_action in opponent_actions.iter().take(qs_branch) {
        let actions = vec![action.clone(), opponent_action.clone()];
        let mut rng = || 0.42;
        stats.step_battle_calls += 1;
        let next = step_battle(
            state,
            &actions,
            &mut rng,
            BattleOptions {
                record_history: false,
            },
        );
        let score = quiescence_eval(&next, player_id, ctx, qs_depth - 1, stats);
        if stats.aborted {
            break;
        }

        worst = worst.min(score);
        if worst <= alpha {
            stats.alpha_cutoffs += 1;
            break;
        }
    }

    worst
}

fn best_continuation(
    state: &BattleState,
    player_id: &str,
    ctx: &VegaContext,
    depth_left: usize,
    stats: &mut VegaStats,
) -> f32 {
    stats.nodes_entered += 1;
    if stats.nodes_entered >= ctx.node_budget {
        stats.aborted = true;
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }
    if is_battle_over(state) {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    // TT probe
    let hash = if ctx.tt.is_some() {
        let h = hash_battle_state(state, player_id);
        if let Some(tt_ref) = ctx.tt {
            if let Ok(tt) = tt_ref.try_borrow() {
                if let Some(entry) = tt.probe(h) {
                    if entry.depth >= depth_left {
                        return entry.score;
                    }
                }
            }
        }
        h
    } else {
        0
    };

    let Some(opp_id) = opponent_id(state, player_id) else {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    };
    let actions = ordered_actions(state, player_id, ctx, stats);
    if actions.is_empty() {
        stats.leaf_evals += 1;
        return evaluate_state_vega(state, player_id, ctx);
    }

    let mut best = f32::NEG_INFINITY;
    let mut alpha = f32::NEG_INFINITY;
    for action in actions.iter().take(ctx.branch_limit) {
        let score = worst_opponent_reply(
            state,
            player_id,
            opp_id.as_str(),
            action,
            ctx,
            depth_left.saturating_sub(1),
            alpha,
            stats,
        );
        if stats.aborted {
            break;
        }
        best = best.max(score);
        alpha = alpha.max(best);
    }

    // TT store
    if !stats.aborted {
        if let Some(tt_ref) = ctx.tt {
            if let Ok(mut tt) = tt_ref.try_borrow_mut() {
                tt.store(hash, depth_left, best, TTFlag::Exact);
            }
        }
    }

    best
}

fn ordered_actions(
    state: &BattleState,
    player_id: &str,
    ctx: &VegaContext,
    stats: &mut VegaStats,
) -> Vec<Action> {
    stats.ordered_actions_calls += 1;
    let actions = available_actions(state, player_id, ctx.move_db);
    stats.ordered_actions_scored += actions.len() as u64;
    let tactical_context = tactical_context(state, player_id, ctx);

    let mut scored: Vec<(Action, f32)> = actions
        .into_iter()
        .map(|action| {
            let score =
                tactical_action_score(state, player_id, &action, ctx, tactical_context.as_ref())
                    + action_ordering_score(state, player_id, &action, ctx);
            (action, score)
        })
        .collect();

    let limit = ctx.branch_limit.min(scored.len());
    if limit == 0 {
        return Vec::new();
    }

    if scored.len() > limit {
        scored.select_nth_unstable_by(limit, |(_, score_a), (_, score_b)| {
            score_b
                .partial_cmp(score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);
    }

    scored.sort_by(|(_, score_a), (_, score_b)| {
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stats.ordered_actions_returned += scored.len() as u64;
    scored.into_iter().map(|(action, _)| action).collect()
}

fn available_actions(state: &BattleState, player_id: &str, move_db: &MoveDatabase) -> Vec<Action> {
    let Some(player) = state.players.iter().find(|p| p.id == player_id) else {
        return Vec::new();
    };

    let switch_actions: Vec<Action> = player
        .team
        .iter()
        .enumerate()
        .filter(|(idx, mon)| *idx != player.active_slot && mon.hp > 0)
        .map(|(idx, _)| Action {
            player_id: player.id.clone(),
            action_type: ActionType::Switch,
            move_id: None,
            target_id: None,
            slot: Some(idx),
            priority: None,
        })
        .collect();

    if needs_switch(state, player_id) {
        return switch_actions;
    }

    let Some(active) = get_active_creature(state, player_id) else {
        return switch_actions;
    };
    if active.hp <= 0 {
        return switch_actions;
    }

    let target_id = opponent_id(state, player_id);
    let mut actions = Vec::new();
    for move_id in &active.moves {
        if !move_has_pp(active, move_id, move_db) {
            continue;
        }
        actions.push(Action {
            player_id: player.id.clone(),
            action_type: ActionType::Move,
            move_id: Some(move_id.clone()),
            target_id: target_id.clone(),
            slot: None,
            priority: None,
        });
    }

    if actions.is_empty() {
        switch_actions
    } else {
        actions.extend(switch_actions);
        actions
    }
}

fn action_ordering_score(
    state: &BattleState,
    player_id: &str,
    action: &Action,
    ctx: &VegaContext,
) -> f32 {
    let Some(player) = get_player(state, player_id) else {
        return 0.0;
    };
    let Some(opponent) = get_opponent(state, player_id) else {
        return 0.0;
    };
    let Some(active) = player.team.get(player.active_slot) else {
        return 0.0;
    };
    let Some(target) = opponent.team.get(opponent.active_slot) else {
        return 0.0;
    };

    match action.action_type {
        ActionType::Switch => {
            let Some(slot) = action.slot else {
                return -1000.0;
            };
            let Some(candidate) = player.team.get(slot) else {
                return -1000.0;
            };
            let pressure = max_expected_damage(candidate, target, &ctx.move_db).ratio;
            let danger = max_expected_damage(target, candidate, &ctx.move_db).ratio;
            pressure * ctx.params.switch_pressure - danger * ctx.params.switch_danger
                + hp_ratio(candidate) * ctx.params.switch_hp
        }
        ActionType::Move => {
            let Some(move_id) = action.move_id.as_deref() else {
                return 0.0;
            };
            let Some(move_data) = ctx.move_db.get(move_id) else {
                return 0.0;
            };
            if move_data.category.as_deref() == Some("status") {
                return status_move_ordering_score(state, player_id, active, target, move_data, ctx);
            }
            let damage = expected_move_damage(
                active,
                target,
                move_data,
                Some(state),
                Some(opponent.id.as_str()),
            );
            let ratio = if target.hp > 0 {
                damage / target.hp as f32
            } else {
                0.0
            };
            let priority = move_data.priority.unwrap_or(0);
            ratio * ctx.params.action_damage
                + if priority > 0 && damage >= target.hp as f32 {
                    ctx.params.action_priority_ko
                } else {
                    0.0
                }
                + accuracy(move_data) * ctx.params.action_accuracy
        }
        ActionType::UseItem => 0.0,
    }
}

fn status_move_ordering_score(
    state: &BattleState,
    player_id: &str,
    active: &CreatureState,
    target: &CreatureState,
    move_data: &MoveData,
    ctx: &VegaContext,
) -> f32 {
    let mut score = accuracy(move_data) * ctx.params.action_accuracy;

    for step in &move_data.steps {
        score += match step.effect_type.as_str() {
            "modify_stage" => score_stage_change_step(active, target, step, ctx),
            "apply_status" => score_apply_status_step(target, step, ctx),
            "apply_field_status" => score_field_status_step(state, player_id, step),
            "remove_field_status" => score_remove_field_step(state, player_id, step),
            "damage_ratio" => score_healing_step(active, step),
            "protect" => 40.0,
            "self_switch" => 30.0,
            _ => 0.0,
        };
    }

    score
}

fn score_stage_change_step(
    active: &CreatureState,
    target: &CreatureState,
    step: &Effect,
    ctx: &VegaContext,
) -> f32 {
    let target_str = step
        .data
        .get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let Some(stages) = step.data.get("stages").and_then(|v| v.as_object()) else {
        return 0.0;
    };

    let is_self = target_str == "self";
    let creature = if is_self { active } else { target };
    let mut score = 0.0;

    for (stat, val) in stages {
        let amount = val.as_i64().unwrap_or(0) as i32;
        let current = match stat.as_str() {
            "atk" => creature.stages.atk,
            "def" => creature.stages.def,
            "spa" => creature.stages.spa,
            "spd" => creature.stages.spd,
            "spe" => creature.stages.spe,
            "accuracy" => creature.stages.accuracy,
            "evasion" => creature.stages.evasion,
            _ => 0,
        };

        let effective = if is_self {
            let new_stage = (current + amount).clamp(-6, 6);
            (new_stage - current) as f32
        } else {
            let new_stage = (current + amount).clamp(-6, 6);
            (current - new_stage) as f32
        };
        if effective <= 0.0 {
            continue;
        }

        let base = match stat.as_str() {
            "atk" => {
                if is_self {
                    if uses_category(active, ctx.move_db, "physical") {
                        75.0
                    } else {
                        15.0
                    }
                } else if uses_category(target, ctx.move_db, "physical") {
                    55.0
                } else {
                    15.0
                }
            }
            "spa" => {
                if is_self {
                    if uses_category(active, ctx.move_db, "special") {
                        75.0
                    } else {
                        15.0
                    }
                } else if uses_category(target, ctx.move_db, "special") {
                    55.0
                } else {
                    15.0
                }
            }
            "spe" => 55.0,
            "def" | "spd" => 35.0,
            "evasion" => 30.0,
            "accuracy" => 25.0,
            _ => 15.0,
        };

        score += effective * base;
    }

    score
}

fn score_apply_status_step(
    target: &CreatureState,
    step: &Effect,
    ctx: &VegaContext,
) -> f32 {
    let target_str = step
        .data
        .get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if target_str != "target" {
        return 0.0;
    }

    let status_id = step
        .data
        .get("statusId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Check type immunity
    if let Some(immune_types) = step.data.get("immuneTypes").and_then(|v| v.as_array()) {
        for immune_type in immune_types.iter().filter_map(|v| v.as_str()) {
            if target
                .types
                .iter()
                .any(|t| t.eq_ignore_ascii_case(immune_type))
            {
                return -100.0;
            }
        }
    }

    // Already has a major status
    let has_major = target.statuses.iter().any(|s| {
        matches!(
            s.id.as_str(),
            "sleep"
                | "freeze"
                | "frozen"
                | "paralysis"
                | "paralyze"
                | "burn"
                | "burned"
                | "poison"
                | "poisoned"
                | "toxic"
                | "badly_poison"
                | "badly_poisoned"
        )
    });
    let is_major = matches!(
        status_id,
        "sleep"
            | "freeze"
            | "paralysis"
            | "paralyze"
            | "burn"
            | "toxic"
            | "poison"
            | "badly_poison"
    );
    if has_major && is_major {
        return -50.0;
    }

    match status_id {
        "sleep" => 130.0,
        "freeze" | "frozen" => 140.0,
        "paralysis" | "paralyze" => 95.0,
        "burn" | "burned" => {
            if uses_category(target, ctx.move_db, "physical") {
                110.0
            } else {
                55.0
            }
        }
        "poison" | "poisoned" => 65.0,
        "toxic" | "badly_poison" | "badly_poisoned" => 115.0,
        "confusion" | "confused" => 45.0,
        "leech_seed" => {
            if target.statuses.iter().any(|s| s.id == "leech_seed") {
                -50.0
            } else {
                75.0
            }
        }
        _ => 25.0,
    }
}

fn score_field_status_step(state: &BattleState, player_id: &str, step: &Effect) -> f32 {
    let status_id = step
        .data
        .get("statusId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Check if hazard already exists on opponent's side
    let opp_id = opponent_id(state, player_id);
    let already_set = opp_id.as_deref().map_or(false, |oid| {
        state
            .field
            .sides
            .get(oid)
            .map_or(false, |effects| effects.iter().any(|e| e.id == status_id))
    });
    // Check if screen already exists on our side
    let self_already = state
        .field
        .sides
        .get(player_id)
        .map_or(false, |effects| effects.iter().any(|e| e.id == status_id));

    if already_set || self_already {
        return -30.0;
    }

    match status_id {
        "stealth_rock" => 85.0,
        "spikes" | "toxic_spikes" => 65.0,
        "sticky_web" => 55.0,
        "reflect" | "light_screen" | "aurora_veil" => 75.0,
        _ => 25.0,
    }
}

fn score_remove_field_step(state: &BattleState, player_id: &str, step: &Effect) -> f32 {
    let status_id = step
        .data
        .get("statusId")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Value removing hazards from our side
    let our_side = state
        .field
        .sides
        .get(player_id)
        .map_or(false, |effects| effects.iter().any(|e| e.id == status_id));
    if our_side {
        return match status_id {
            "stealth_rock" => 40.0,
            "spikes" | "toxic_spikes" => 30.0,
            "sticky_web" => 25.0,
            _ => 10.0,
        };
    }
    0.0
}

fn score_healing_step(active: &CreatureState, step: &Effect) -> f32 {
    let target_str = step
        .data
        .get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if target_str != "self" {
        return 0.0;
    }

    let ratio = step
        .data
        .get("ratioMaxHp")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if ratio >= 0.0 {
        return 0.0; // Self-damage, not healing
    }

    let heal_ratio = -ratio as f32;
    let missing = 1.0 - hp_ratio(active);
    let effective = heal_ratio.min(missing);
    effective * 140.0
}

fn tactical_action_score(
    state: &BattleState,
    player_id: &str,
    action: &Action,
    ctx: &VegaContext,
    tactical_context: Option<&TacticalContext>,
) -> f32 {
    let Some(player) = get_player(state, player_id) else {
        return 0.0;
    };
    let Some(opponent) = get_opponent(state, player_id) else {
        return 0.0;
    };
    let Some(active) = player.team.get(player.active_slot) else {
        return 0.0;
    };
    let Some(target) = opponent.team.get(opponent.active_slot) else {
        return 0.0;
    };

    match action.action_type {
        ActionType::Move => tactical_move_score(
            state,
            opponent,
            active,
            target,
            action,
            ctx,
            tactical_context,
        ),
        ActionType::Switch => tactical_switch_score(player, target, action, ctx, tactical_context),
        ActionType::UseItem => 0.0,
    }
}

fn tactical_context(
    state: &BattleState,
    player_id: &str,
    ctx: &VegaContext,
) -> Option<TacticalContext> {
    let player = get_player(state, player_id)?;
    let opponent = get_opponent(state, player_id)?;
    let active = player.team.get(player.active_slot)?;
    let opponent_active = opponent.team.get(opponent.active_slot)?;
    Some(TacticalContext {
        incoming_to_active: max_expected_damage(opponent_active, active, ctx.move_db),
    })
}

fn tactical_move_score(
    state: &BattleState,
    opponent: &PlayerState,
    active: &CreatureState,
    target: &CreatureState,
    action: &Action,
    ctx: &VegaContext,
    tactical_context: Option<&TacticalContext>,
) -> f32 {
    let Some(move_id) = action.move_id.as_deref() else {
        return 0.0;
    };
    let Some(move_data) = ctx.move_db.get(move_id) else {
        return 0.0;
    };

    let damage = expected_move_damage(
        active,
        target,
        move_data,
        Some(state),
        Some(opponent.id.as_str()),
    );
    let can_ko = target.hp > 0 && damage >= target.hp as f32;
    let moves_first = moves_before(active, target, move_data);
    let mut score = 0.0;
    if can_ko {
        score += TACTICAL_CONFIRMED_KO_BONUS;
        if moves_first {
            score += TACTICAL_FAST_KO_BONUS;
        }
    }

    if tactical_context
        .map(|context| context.incoming_to_active.damage >= active.hp as f32)
        .unwrap_or(false)
        && !(can_ko && moves_first)
    {
        score += TACTICAL_SELF_DEATH_PENALTY;
    }

    score
}

fn tactical_switch_score(
    player: &PlayerState,
    opponent_active: &CreatureState,
    action: &Action,
    ctx: &VegaContext,
    tactical_context: Option<&TacticalContext>,
) -> f32 {
    let Some(slot) = action.slot else {
        return 0.0;
    };
    let Some(candidate) = player.team.get(slot) else {
        return 0.0;
    };

    let incoming_to_candidate = max_expected_damage(opponent_active, candidate, ctx.move_db);
    if incoming_to_candidate.damage >= candidate.hp as f32 {
        return TACTICAL_DEAD_SWITCH_PENALTY;
    }

    if tactical_context
        .map(|context| context.incoming_to_active.damage >= context_active_hp(player) as f32)
        .unwrap_or(false)
    {
        return TACTICAL_SAFE_SWITCH_BONUS;
    }

    0.0
}

fn context_active_hp(player: &PlayerState) -> i32 {
    player
        .team
        .get(player.active_slot)
        .map(|active| active.hp)
        .unwrap_or(0)
}

fn moves_before(attacker: &CreatureState, defender: &CreatureState, move_data: &MoveData) -> bool {
    let priority = move_data.priority.unwrap_or(0);
    if priority != 0 {
        return priority > 0;
    }
    attacker.speed >= defender.speed
}

fn evaluate_state_vega(state: &BattleState, player_id: &str, ctx: &VegaContext) -> f32 {
    let Some(player) = get_player(state, player_id) else {
        return 0.0;
    };
    let Some(opponent) = get_opponent(state, player_id) else {
        return 0.0;
    };

    let player_alive = alive_count(player);
    let opponent_alive = alive_count(opponent);
    if player_alive == 0 {
        return -WIN_SCORE;
    }
    if opponent_alive == 0 {
        return WIN_SCORE;
    }

    score_player(state, player, opponent, ctx) - score_player(state, opponent, player, ctx)
}

fn score_player(
    state: &BattleState,
    player: &PlayerState,
    opponent: &PlayerState,
    ctx: &VegaContext,
) -> f32 {
    let active = player.team.get(player.active_slot);
    let opponent_active = opponent.team.get(opponent.active_slot);
    let mut score = 0.0;

    for (slot, creature) in player.team.iter().enumerate() {
        score += score_creature(creature, &ctx.move_db, ctx.params);
        if slot == player.active_slot && creature.hp > 0 {
            score += hp_ratio(creature) * ctx.params.active_hp;
        }
    }

    if let (Some(active), Some(opponent_active)) = (active, opponent_active) {
        if active.hp > 0 && opponent_active.hp > 0 {
            let outgoing = max_expected_damage(active, opponent_active, &ctx.move_db);
            let incoming = max_expected_damage(opponent_active, active, &ctx.move_db);
            score += outgoing.ratio * ctx.params.outgoing;
            score -= incoming.ratio * ctx.params.incoming;

            if outgoing.damage >= opponent_active.hp as f32 {
                score += if active.speed >= opponent_active.speed || outgoing.priority_ko {
                    ctx.params.ko_fast
                } else {
                    ctx.params.ko_slow
                };
            }
            if incoming.damage >= active.hp as f32 {
                score -= if opponent_active.speed >= active.speed || incoming.priority_ko {
                    ctx.params.risk_fast
                } else {
                    ctx.params.risk_slow
                };
            }
            if active.speed > opponent_active.speed {
                score += ctx.params.speed;
            }

            score += best_bench_matchup(player, opponent_active, &ctx.move_db) * ctx.params.bench;
            score -= side_penalty(state, player.id.as_str());
            score += side_penalty(state, opponent.id.as_str());
        }
    }

    score
}

fn score_creature(creature: &CreatureState, move_db: &MoveDatabase, params: VegaParams) -> f32 {
    if creature.hp <= 0 {
        return -140.0;
    }

    params.alive
        + creature.hp.max(0) as f32 * params.hp
        + hp_ratio(creature) * params.hp_ratio
        + score_stages(creature, move_db) * params.stage
        + score_statuses(creature, move_db) * params.status
}

fn score_stages(creature: &CreatureState, move_db: &MoveDatabase) -> f32 {
    let physical_weight = if uses_category(creature, move_db, "physical") {
        1.0
    } else {
        0.35
    };
    let special_weight = if uses_category(creature, move_db, "special") {
        1.0
    } else {
        0.35
    };
    let stages = &creature.stages;

    stages.atk as f32 * 18.0 * physical_weight
        + stages.spa as f32 * 18.0 * special_weight
        + stages.def as f32 * 12.0
        + stages.spd as f32 * 12.0
        + stages.spe as f32 * 15.0
        + stages.accuracy as f32 * 10.0
        + stages.evasion as f32 * 12.0
}

fn score_statuses(creature: &CreatureState, move_db: &MoveDatabase) -> f32 {
    let mut score = 0.0;
    for status in &creature.statuses {
        score += match status.id.as_str() {
            "sleep" => -170.0,
            "freeze" | "frozen" => -190.0,
            "paralysis" | "paralyze" | "paralyzed" => -95.0,
            "burn" | "burned" => {
                if uses_category(creature, move_db, "physical") {
                    -115.0
                } else {
                    -55.0
                }
            }
            "poison" | "poisoned" => -75.0,
            "toxic" | "badly_poison" | "badly_poisoned" => -125.0,
            "confusion" | "confused" => -45.0,
            "leech_seed" => -80.0,
            "substitute" => 85.0,
            _ => 0.0,
        };
    }
    score
}

struct DamageSummary {
    damage: f32,
    ratio: f32,
    priority_ko: bool,
}

fn max_expected_damage(
    attacker: &CreatureState,
    defender: &CreatureState,
    move_db: &MoveDatabase,
) -> DamageSummary {
    let mut best_damage: f32 = 0.0;
    let mut priority_ko = false;

    for move_id in &attacker.moves {
        if !move_has_pp(attacker, move_id, move_db) {
            continue;
        }
        let Some(move_data) = move_db.get(move_id) else {
            continue;
        };
        if move_data.category.as_deref() == Some("status") {
            continue;
        }

        let damage = expected_move_damage(attacker, defender, move_data, None, None);
        best_damage = best_damage.max(damage);
        if move_data.priority.unwrap_or(0) > 0 && damage >= defender.hp as f32 {
            priority_ko = true;
        }
    }

    DamageSummary {
        damage: best_damage,
        ratio: if defender.hp > 0 {
            best_damage / defender.hp as f32
        } else {
            0.0
        },
        priority_ko,
    }
}

fn expected_move_damage(
    attacker: &CreatureState,
    defender: &CreatureState,
    move_data: &MoveData,
    state: Option<&BattleState>,
    defender_player_id: Option<&str>,
) -> f32 {
    if move_data.category.as_deref() == Some("status") {
        return 0.0;
    }
    let Some(power) = move_data.power else {
        return 0.0;
    };
    if power <= 0 {
        return 0.0;
    }

    let category = move_data.category.as_deref().unwrap_or("physical");
    let (attack_stat, defense_stat, attack_stage, defense_stage) = if category == "special" {
        (
            attacker.sp_attack,
            defender.sp_defense,
            attacker.stages.spa,
            defender.stages.spd,
        )
    } else {
        (
            attacker.attack,
            defender.defense,
            attacker.stages.atk,
            defender.stages.def,
        )
    };

    let attack = (attack_stat as f32 * stage_multiplier(attack_stage)).max(1.0);
    let defense = (defense_stat as f32 * stage_multiplier(defense_stage)).max(1.0);
    let move_type = move_data.move_type.as_deref().unwrap_or("");
    let power = power as f32 * weather_power_multiplier(state, move_type);
    let level = attacker.level as f32;
    let base = (((2.0 * level / 5.0 + 2.0) * power * attack / defense) / 50.0 + 2.0).max(1.0);

    let mut modifier = 0.925;
    if attacker
        .types
        .iter()
        .any(|t| t.eq_ignore_ascii_case(move_type))
    {
        modifier *= 1.5;
    }
    modifier *= type_effectiveness(move_type, &defender.types);
    modifier *= screen_multiplier(state, defender_player_id, category);
    if category == "physical"
        && attacker.statuses.iter().any(|s| s.id == "burn")
        && attacker.ability.as_deref() != Some("guts")
    {
        modifier *= 0.5;
    }

    (base * modifier * accuracy(move_data)).floor()
}

fn best_bench_matchup(
    player: &PlayerState,
    opponent_active: &CreatureState,
    move_db: &MoveDatabase,
) -> f32 {
    let current_score = player
        .team
        .get(player.active_slot)
        .map(|current| max_expected_damage(current, opponent_active, move_db).ratio)
        .unwrap_or(0.0);
    let mut best: f32 = 0.0;

    for (slot, creature) in player.team.iter().enumerate() {
        if slot == player.active_slot || creature.hp <= 0 {
            continue;
        }
        let pressure = max_expected_damage(creature, opponent_active, move_db).ratio;
        best = best.max(pressure - current_score);
    }

    best
}

fn side_penalty(state: &BattleState, player_id: &str) -> f32 {
    let Some(effects) = state.field.sides.get(player_id) else {
        return 0.0;
    };
    let mut penalty = 0.0;
    for effect in effects {
        penalty += match effect.id.as_str() {
            "stealth_rock" => 35.0,
            "spikes" => 25.0,
            "toxic_spikes" => 22.0,
            "sticky_web" => 18.0,
            "reflect" | "light_screen" | "aurora_veil" => -30.0,
            _ => 0.0,
        };
    }
    penalty
}

fn get_player<'a>(state: &'a BattleState, player_id: &str) -> Option<&'a PlayerState> {
    state.players.iter().find(|p| p.id == player_id)
}

fn get_opponent<'a>(state: &'a BattleState, player_id: &str) -> Option<&'a PlayerState> {
    state.players.iter().find(|p| p.id != player_id)
}

fn opponent_id(state: &BattleState, player_id: &str) -> Option<String> {
    get_opponent(state, player_id).map(|p| p.id.clone())
}

fn alive_count(player: &PlayerState) -> usize {
    player
        .team
        .iter()
        .filter(|creature| creature.hp > 0)
        .count()
}

fn needs_switch(state: &BattleState, player_id: &str) -> bool {
    let Some(active) = get_active_creature(state, player_id) else {
        return true;
    };
    active.hp <= 0 || active.statuses.iter().any(|s| s.id == "pending_switch")
}

fn move_has_pp(active: &CreatureState, move_id: &str, move_db: &MoveDatabase) -> bool {
    let Some(move_data) = move_db.get(move_id) else {
        return false;
    };
    let Some(pp) = move_data.pp else {
        return true;
    };
    match active.move_pp.get(move_id) {
        Some(remaining) => *remaining > 0,
        None => pp > 0,
    }
}

fn hp_ratio(creature: &CreatureState) -> f32 {
    creature.hp.max(0) as f32 / creature.max_hp.max(1) as f32
}

fn uses_category(creature: &CreatureState, move_db: &MoveDatabase, category: &str) -> bool {
    creature.moves.iter().any(|move_id| {
        move_db
            .get(move_id)
            .and_then(|move_data| move_data.category.as_deref())
            == Some(category)
    })
}

fn stage_multiplier(stage: i32) -> f32 {
    let clamped = stage.clamp(-6, 6);
    if clamped >= 0 {
        (2 + clamped) as f32 / 2.0
    } else {
        2.0 / (2 - clamped) as f32
    }
}

fn accuracy(move_data: &MoveData) -> f32 {
    let Some(accuracy) = move_data.accuracy else {
        return 1.0;
    };
    if accuracy > 1.0 {
        accuracy / 100.0
    } else {
        accuracy
    }
}

fn type_effectiveness(move_type: &str, target_types: &[String]) -> f32 {
    let mut multiplier = 1.0;
    for target_type in target_types {
        multiplier *= single_type_effectiveness(move_type, target_type);
    }
    multiplier
}

fn single_type_effectiveness(move_type: &str, target_type: &str) -> f32 {
    let move_type = move_type.to_ascii_lowercase();
    let target_type = target_type.to_ascii_lowercase();
    let strong = |types: &[&str]| types.contains(&target_type.as_str());
    match move_type.as_str() {
        "normal" => {
            if strong(&["ghost"]) {
                0.0
            } else if strong(&["rock", "steel"]) {
                0.5
            } else {
                1.0
            }
        }
        "fire" => {
            if strong(&["grass", "ice", "bug", "steel"]) {
                2.0
            } else if strong(&["fire", "water", "rock", "dragon"]) {
                0.5
            } else {
                1.0
            }
        }
        "water" => {
            if strong(&["fire", "ground", "rock"]) {
                2.0
            } else if strong(&["water", "grass", "dragon"]) {
                0.5
            } else {
                1.0
            }
        }
        "electric" => {
            if strong(&["ground"]) {
                0.0
            } else if strong(&["water", "flying"]) {
                2.0
            } else if strong(&["electric", "grass", "dragon"]) {
                0.5
            } else {
                1.0
            }
        }
        "grass" => {
            if strong(&["water", "ground", "rock"]) {
                2.0
            } else if strong(&[
                "fire", "grass", "poison", "flying", "bug", "dragon", "steel",
            ]) {
                0.5
            } else {
                1.0
            }
        }
        "ice" => {
            if strong(&["grass", "ground", "flying", "dragon"]) {
                2.0
            } else if strong(&["fire", "water", "ice", "steel"]) {
                0.5
            } else {
                1.0
            }
        }
        "fighting" => {
            if strong(&["ghost"]) {
                0.0
            } else if strong(&["normal", "ice", "rock", "dark", "steel"]) {
                2.0
            } else if strong(&["poison", "flying", "psychic", "bug", "fairy"]) {
                0.5
            } else {
                1.0
            }
        }
        "poison" => {
            if strong(&["steel"]) {
                0.0
            } else if strong(&["grass", "fairy"]) {
                2.0
            } else if strong(&["poison", "ground", "rock", "ghost"]) {
                0.5
            } else {
                1.0
            }
        }
        "ground" => {
            if strong(&["flying"]) {
                0.0
            } else if strong(&["fire", "electric", "poison", "rock", "steel"]) {
                2.0
            } else if strong(&["grass", "bug"]) {
                0.5
            } else {
                1.0
            }
        }
        "flying" => {
            if strong(&["grass", "fighting", "bug"]) {
                2.0
            } else if strong(&["electric", "rock", "steel"]) {
                0.5
            } else {
                1.0
            }
        }
        "psychic" => {
            if strong(&["dark"]) {
                0.0
            } else if strong(&["fighting", "poison"]) {
                2.0
            } else if strong(&["psychic", "steel"]) {
                0.5
            } else {
                1.0
            }
        }
        "bug" => {
            if strong(&["grass", "psychic", "dark"]) {
                2.0
            } else if strong(&[
                "fire", "fighting", "poison", "flying", "ghost", "steel", "fairy",
            ]) {
                0.5
            } else {
                1.0
            }
        }
        "rock" => {
            if strong(&["fire", "ice", "flying", "bug"]) {
                2.0
            } else if strong(&["fighting", "ground", "steel"]) {
                0.5
            } else {
                1.0
            }
        }
        "ghost" => {
            if strong(&["normal"]) {
                0.0
            } else if strong(&["psychic", "ghost"]) {
                2.0
            } else if strong(&["dark"]) {
                0.5
            } else {
                1.0
            }
        }
        "dragon" => {
            if strong(&["fairy"]) {
                0.0
            } else if strong(&["dragon"]) {
                2.0
            } else if strong(&["steel"]) {
                0.5
            } else {
                1.0
            }
        }
        "dark" => {
            if strong(&["psychic", "ghost"]) {
                2.0
            } else if strong(&["fighting", "dark", "fairy"]) {
                0.5
            } else {
                1.0
            }
        }
        "steel" => {
            if strong(&["ice", "rock", "fairy"]) {
                2.0
            } else if strong(&["fire", "water", "electric", "steel"]) {
                0.5
            } else {
                1.0
            }
        }
        "fairy" => {
            if strong(&["fighting", "dragon", "dark"]) {
                2.0
            } else if strong(&["fire", "poison", "steel"]) {
                0.5
            } else {
                1.0
            }
        }
        _ => 1.0,
    }
}

fn weather_power_multiplier(state: Option<&BattleState>, move_type: &str) -> f32 {
    let Some(state) = state else {
        return 1.0;
    };
    let weather = state
        .field
        .global
        .iter()
        .find(|effect| matches!(effect.id.as_str(), "sun" | "rain" | "sandstorm" | "snow"))
        .map(|effect| effect.id.as_str());
    match (weather, move_type) {
        (Some("sun"), "fire") => 1.5,
        (Some("sun"), "water") => 0.5,
        (Some("rain"), "water") => 1.5,
        (Some("rain"), "fire") => 0.5,
        _ => 1.0,
    }
}

fn screen_multiplier(
    state: Option<&BattleState>,
    defender_player_id: Option<&str>,
    category: &str,
) -> f32 {
    let (Some(state), Some(defender_player_id)) = (state, defender_player_id) else {
        return 1.0;
    };
    let Some(side_effects) = state.field.sides.get(defender_player_id) else {
        return 1.0;
    };
    if side_effects.iter().any(|effect| effect.id == "aurora_veil") {
        return 0.5;
    }
    if category == "physical" && side_effects.iter().any(|effect| effect.id == "reflect") {
        return 0.5;
    }
    if category == "special"
        && side_effects
            .iter()
            .any(|effect| effect.id == "light_screen")
    {
        return 0.5;
    }
    1.0
}
