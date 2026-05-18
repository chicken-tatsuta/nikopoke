use crate::ai::eval::evaluate_state;
use crate::core::battle::{is_battle_over, step_battle, BattleOptions};
use crate::core::state::{Action, ActionType, BattleState};
use crate::core::utils::get_active_creature;
use crate::data::moves::MoveDatabase;

struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value = (self.state >> 11) as u64;
        (value as f64) / ((1u64 << 53) as f64)
    }

    fn choose_index(&mut self, len: usize) -> usize {
        if len <= 1 {
            return 0;
        }
        let idx = (self.next_f64() * len as f64) as usize;
        idx.min(len - 1)
    }
}

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
    let mut actions = Vec::new();
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

pub fn get_best_move_mcts(
    state: &BattleState,
    player_id: &str,
    _iterations: usize,
) -> Option<Action> {
    let actions = available_actions(state, player_id);
    if actions.is_empty() {
        return None;
    }
    let Some(opp_id) = opponent_id(state, player_id) else {
        return actions.first().cloned();
    };

    let iterations = _iterations.max(1);
    let rollout_depth = 3usize;
    let mut rng = LcgRng::new(0x9e3779b97f4a7c15 ^ state.turn as u64);

    let mut best_action = None;
    let mut best_score = f32::NEG_INFINITY;
    for action in &actions {
        let mut total_score = 0.0;
        for _ in 0..iterations {
            let mut sim_state = state.clone();
            let opp_actions = available_actions(&sim_state, &opp_id);
            if opp_actions.is_empty() {
                total_score += evaluate_state(&sim_state, player_id);
                continue;
            }
            let opp_action = opp_actions[rng.choose_index(opp_actions.len())].clone();
            let mut step_rng = || rng.next_f64();
            sim_state = step_battle(
                &sim_state,
                &[action.clone(), opp_action],
                &mut step_rng,
                BattleOptions {
                    record_history: false,
                },
            );

            for _ in 0..rollout_depth {
                if is_battle_over(&sim_state) {
                    break;
                }
                let my_actions = available_actions(&sim_state, player_id);
                let opp_actions = available_actions(&sim_state, &opp_id);
                if my_actions.is_empty() || opp_actions.is_empty() {
                    break;
                }
                let my_action = my_actions[rng.choose_index(my_actions.len())].clone();
                let opp_action = opp_actions[rng.choose_index(opp_actions.len())].clone();
                let mut step_rng = || rng.next_f64();
                sim_state = step_battle(
                    &sim_state,
                    &[my_action, opp_action],
                    &mut step_rng,
                    BattleOptions {
                        record_history: false,
                    },
                );
            }
            total_score += evaluate_state(&sim_state, player_id);
        }
        let avg = total_score / iterations as f32;
        if avg > best_score {
            best_score = avg;
            best_action = Some(action.clone());
        }
    }
    best_action
}
