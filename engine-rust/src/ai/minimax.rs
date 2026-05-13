use crate::ai::eval::evaluate_state;
use crate::core::battle::{is_battle_over, step_battle, BattleOptions};
use crate::core::state::{Action, ActionType, BattleState};
use crate::core::utils::get_active_creature;
use crate::data::moves::MoveDatabase;

fn needs_switch(state: &BattleState, player_id: &str) -> bool {
    let Some(active) = get_active_creature(state, player_id) else {
        return true;
    };
    if active.hp <= 0 {
        return true;
    }
    active.statuses.iter().any(|s| s.id == "pending_switch")
}

fn opponent_id(state: &BattleState, player_id: &str) -> Option<String> {
    state
        .players
        .iter()
        .find(|p| p.id != player_id)
        .map(|p| p.id.clone())
}

fn move_has_pp(
    active: &crate::core::state::CreatureState,
    move_id: &str,
    move_db: &MoveDatabase,
) -> bool {
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

fn available_actions(state: &BattleState, player_id: &str) -> Vec<Action> {
    let player = state.players.iter().find(|p| p.id == player_id);
    let Some(player) = player else {
        return Vec::new();
    };

    let mut actions = Vec::new();
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
    let move_db = MoveDatabase::default();
    for move_id in &active.moves {
        if !move_has_pp(active, move_id, &move_db) {
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

fn evaluate_after_turn(state: &BattleState, max_player_id: &str, depth: usize) -> f32 {
    if depth == 0 || is_battle_over(state) {
        return evaluate_state(state, max_player_id);
    }

    let max_actions = available_actions(state, max_player_id);
    if max_actions.is_empty() {
        return evaluate_state(state, max_player_id);
    }
    let Some(opp_id) = opponent_id(state, max_player_id) else {
        return evaluate_state(state, max_player_id);
    };
    let opp_actions = available_actions(state, opp_id.as_str());
    if opp_actions.is_empty() {
        return evaluate_state(state, max_player_id);
    }

    let mut best = f32::NEG_INFINITY;
    for action in &max_actions {
        let mut worst = f32::INFINITY;
        for opp_action in &opp_actions {
            let actions = vec![action.clone(), opp_action.clone()];
            let mut rng = || 0.42;
            let next = step_battle(
                state,
                &actions,
                &mut rng,
                BattleOptions {
                    record_history: false,
                },
            );
            let score = evaluate_after_turn(&next, max_player_id, depth - 1);
            if score < worst {
                worst = score;
            }
        }
        if worst > best {
            best = worst;
        }
    }
    best
}

pub fn get_best_move_minimax(state: &BattleState, player_id: &str, depth: usize) -> Option<Action> {
    let max_actions = available_actions(state, player_id);
    if max_actions.is_empty() {
        return None;
    }
    let Some(opp_id) = opponent_id(state, player_id) else {
        return max_actions.first().cloned();
    };
    let opp_actions = available_actions(state, opp_id.as_str());
    if opp_actions.is_empty() {
        return max_actions.first().cloned();
    }

    let mut best_action = None;
    let mut best_score = f32::NEG_INFINITY;
    let search_depth = depth.max(1);
    for action in &max_actions {
        let mut worst = f32::INFINITY;
        for opp_action in &opp_actions {
            let actions = vec![action.clone(), opp_action.clone()];
            let mut rng = || 0.42;
            let next = step_battle(
                state,
                &actions,
                &mut rng,
                BattleOptions {
                    record_history: false,
                },
            );
            let score = evaluate_after_turn(&next, player_id, search_depth - 1);
            if score < worst {
                worst = score;
            }
        }
        if worst > best_score {
            best_score = worst;
            best_action = Some(action.clone());
        }
    }
    best_action
}
