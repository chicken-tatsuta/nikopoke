use crate::core::battle::{is_battle_over, step_battle, BattleOptions};
use crate::core::state::{Action, ActionType, BattleState, CreatureState, PlayerState};
use crate::core::utils::get_active_creature;
use crate::data::moves::{MoveData, MoveDatabase};

const WIN_SCORE: f32 = 1_000_000.0;
const DEFAULT_BRANCH_LIMIT: usize = 4;

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
    alive: 379.0,
    hp: 1.0,
    hp_ratio: 206.0,
    active_hp: 70.0,
    outgoing: 195.0,
    incoming: 156.0,
    ko_fast: 230.0,
    ko_slow: 121.0,
    risk_fast: 217.0,
    risk_slow: 144.0,
    speed: 28.5,
    bench: 56.0,
    stage: 1.05,
    status: 0.93,
    switch_pressure: 148.0,
    switch_danger: 114.0,
    switch_hp: 35.0,
    action_damage: 202.0,
    action_priority_ko: 128.0,
    action_accuracy: 20.0,
};

struct VegaContext {
    move_db: MoveDatabase,
    params: VegaParams,
    branch_limit: usize,
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
    get_best_move_vega_with_options_and_db(state, player_id, depth, params, branch_limit, move_db)
}

pub fn get_best_move_vega_with_options_and_db(
    state: &BattleState,
    player_id: &str,
    depth: usize,
    params: VegaParams,
    branch_limit: usize,
    move_db: MoveDatabase,
) -> Option<Action> {
    let ctx = VegaContext {
        move_db,
        params,
        branch_limit: branch_limit.max(1),
    };
    let actions = ordered_actions(state, player_id, &ctx);
    if actions.is_empty() {
        return None;
    }

    let Some(opp_id) = opponent_id(state, player_id) else {
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
        );
        if score > best_score {
            best_score = score;
            best_action = Some(action.clone());
        }
        alpha = alpha.max(best_score);
    }

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
) -> f32 {
    let opponent_actions = ordered_actions(state, opponent_id, ctx);
    if opponent_actions.is_empty() {
        return evaluate_state_vega(state, player_id, ctx);
    }

    let mut worst = f32::INFINITY;
    for opponent_action in opponent_actions.iter().take(ctx.branch_limit) {
        let actions = vec![action.clone(), opponent_action.clone()];
        let mut rng = || 0.42;
        let next = step_battle(
            state,
            &actions,
            &mut rng,
            BattleOptions {
                record_history: false,
            },
        );
        let score = if depth_left == 0 {
            evaluate_state_vega(&next, player_id, ctx)
        } else {
            best_continuation(&next, player_id, ctx, depth_left)
        };

        worst = worst.min(score);
        if worst <= alpha {
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
) -> f32 {
    if is_battle_over(state) {
        return evaluate_state_vega(state, player_id, ctx);
    }

    let Some(opp_id) = opponent_id(state, player_id) else {
        return evaluate_state_vega(state, player_id, ctx);
    };
    let actions = ordered_actions(state, player_id, ctx);
    if actions.is_empty() {
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
        );
        best = best.max(score);
        alpha = alpha.max(best);
    }

    best
}

fn ordered_actions(state: &BattleState, player_id: &str, ctx: &VegaContext) -> Vec<Action> {
    let mut actions = available_actions(state, player_id, &ctx.move_db);
    actions.sort_by(|a, b| {
        action_ordering_score(state, player_id, b, ctx)
            .partial_cmp(&action_ordering_score(state, player_id, a, ctx))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    actions
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
