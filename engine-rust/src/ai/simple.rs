use crate::core::battle::{is_battle_over, step_battle, BattleOptions};
use crate::core::state::{Action, ActionType, BattleState};
use crate::core::utils::get_active_creature;
use crate::data::moves::MoveDatabase;

pub fn choose_highest_power(state: &BattleState, player_id: &str) -> Option<Action> {
    let player = state.players.iter().find(|p| p.id == player_id)?;
    let active = get_active_creature(state, player_id)?;
    if active.hp <= 0 {
        return None;
    }

    let target_id = state
        .players
        .iter()
        .find(|p| p.id != player_id)
        .map(|p| p.id.clone())?;

    let move_db = MoveDatabase::minimal();
    let mut best_move_id = active.moves.first()?.clone();
    let mut best_power = -1;

    for move_id in &active.moves {
        if let Some(move_data) = move_db.get(move_id) {
            let power = move_data.power.unwrap_or(0);
            if power > best_power {
                best_power = power;
                best_move_id = move_id.clone();
            }
        }
    }

    Some(Action {
        player_id: player.id.clone(),
        action_type: ActionType::Move,
        move_id: Some(best_move_id),
        target_id: Some(target_id),
        slot: None,
        kamishinka_form: None,
        priority: None,
    })
}

pub fn run_auto_battle(
    state: &BattleState,
    rng: &mut dyn FnMut() -> f64,
    chooser: fn(&BattleState, &str) -> Option<Action>,
) -> BattleState {
    let mut next = state.clone();
    let mut turns = 0;
    while !is_battle_over(&next) && turns < 100 {
        turns += 1;
        let mut actions = Vec::new();
        for player in &next.players {
            if let Some(action) = chooser(&next, &player.id) {
                actions.push(action);
            }
        }
        if actions.is_empty() {
            break;
        }
        next = step_battle(&next, &actions, rng, BattleOptions::default());
    }
    next
}
