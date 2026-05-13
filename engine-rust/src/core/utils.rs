use crate::core::state::{BattleState, CreatureState};
use crate::data::moves::MoveData;

pub fn stage_multiplier(stage: i32) -> f32 {
    let s = stage.clamp(-6, 6);
    if s >= 0 {
        (2.0 + s as f32) / 2.0
    } else {
        2.0 / (2.0 - s as f32)
    }
}

pub fn is_status_move(move_data: &MoveData) -> bool {
    match move_data.category.as_deref() {
        Some("status") => true,
        Some("physical") | Some("special") => false,
        _ => !move_data
            .steps
            .iter()
            .any(|effect| effect.effect_type == "damage"),
    }
}

pub fn get_active_creature<'a>(
    state: &'a BattleState,
    player_id: &str,
) -> Option<&'a CreatureState> {
    let player = state.players.iter().find(|p| p.id == player_id)?;
    player.team.get(player.active_slot)
}

pub fn get_active_creature_mut<'a>(
    state: &'a mut BattleState,
    player_id: &str,
) -> Option<&'a mut CreatureState> {
    let idx = state.players.iter().position(|p| p.id == player_id)?;
    let active_slot = state.players[idx].active_slot;
    state.players[idx].team.get_mut(active_slot)
}
