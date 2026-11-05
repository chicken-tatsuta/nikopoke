//! Data-driven, battle-local evolution. The shared species database is never mutated.
use crate::core::factory::calc_stat;
use crate::core::state::{Action, ActionType, BattleState, KamishinkaState, PlayerState};
use crate::data::{moves::MoveDatabase, species::BaseStats};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KamishinkaForm {
    pub base_species_id: String,
    pub id: String,
    pub name: String,
    pub base_stats: BaseStats,
    pub types: Vec<String>,
    pub ability: String,
    /// No artwork has been supplied yet. Null is deliberately not invented art.
    pub image: Option<String>,
}

pub fn forms() -> &'static [KamishinkaForm] {
    static FORMS: OnceLock<Vec<KamishinkaForm>> = OnceLock::new();
    FORMS.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/kamishinka.json"))
            .expect("invalid kamishinka registry")
    })
}

pub fn available_forms(player: &PlayerState) -> Vec<&'static KamishinkaForm> {
    let Some(active) = player.team.get(player.active_slot) else {
        return vec![];
    };
    if player.kamishinka_used
        || active.hp <= 0
        || active.kamishinka.is_some()
        || active.item.as_deref() != Some("kamidama")
        || active.statuses.iter().any(|s| s.id == "pending_switch")
    {
        return vec![];
    }
    forms()
        .iter()
        .filter(|f| f.base_species_id == active.species_id)
        .collect()
}

/// Called once per player's validated move request, before speed/priority resolution.
/// Incapacitating statuses deliberately do not prevent evolution.
pub fn apply(state: &mut BattleState, action: &Action, moves: &MoveDatabase) -> bool {
    if action.action_type != ActionType::Move {
        return false;
    }
    let Some(form_id) = action.kamishinka_form.as_deref() else {
        return false;
    };
    let Some(player) = state.players.iter_mut().find(|p| p.id == action.player_id) else {
        return false;
    };
    let Some(form) = available_forms(player)
        .into_iter()
        .find(|f| f.id == form_id)
    else {
        return false;
    };
    let active = &mut player.team[player.active_slot];
    let Some(move_id) = action.move_id.as_deref() else {
        return false;
    };
    // A forged move must not consume the evolution right.
    if moves.get(move_id).is_none()
        || !active.moves.iter().any(|id| id == move_id)
        || active.move_pp.get(move_id).is_some_and(|pp| *pp <= 0)
    {
        return false;
    }
    active.kamishinka = Some(KamishinkaState {
        form_id: form.id.clone(),
        original_name: active.name.clone(),
        original_ability: active.ability.clone(),
        original_types: active.types.clone(),
        original_stats: [
            active.attack,
            active.defense,
            active.sp_attack,
            active.sp_defense,
            active.speed,
        ],
    });
    active.name = form.name.clone();
    active.types = form.types.clone();
    active.ability = Some(form.ability.clone());
    // Only old-ability bookkeeping is reset; PP/statuses/stages/volatile state persist.
    active.ability_data.clear();
    let s = &form.base_stats;
    let ev = &active.evs;
    let calc = |base, ev| calc_stat(base, false, active.level as i32, 31, ev);
    active.attack = calc(s.atk, ev.atk);
    active.defense = calc(s.def, ev.def);
    active.sp_attack = calc(s.spa, ev.spa);
    active.sp_defense = calc(s.spd, ev.spd);
    active.speed = calc(s.spe, ev.spe);
    player.kamishinka_used = true;
    state
        .log
        .push(format!("{}は カミシンカした！", active.name));
    true
}

/// Explicit lifecycle reset, also used when starting a fresh battle from an old state.
pub fn reset_player(player: &mut PlayerState) {
    player.kamishinka_used = false;
    for creature in &mut player.team {
        if let Some(original) = creature.kamishinka.take() {
            creature.name = original.original_name;
            creature.ability = original.original_ability;
            creature.types = original.original_types;
            [
                creature.attack,
                creature.defense,
                creature.sp_attack,
                creature.sp_defense,
                creature.speed,
            ] = original.original_stats;
            creature.ability_data.clear();
        }
    }
}

/// Share evolution candidates across search algorithms; ordinary moves remain available.
pub fn append_move_variants(player: &PlayerState, actions: &mut Vec<Action>) {
    let base_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.action_type == ActionType::Move && a.kamishinka_form.is_none())
        .cloned()
        .collect();
    for form in available_forms(player) {
        actions.extend(base_actions.iter().cloned().map(|mut action| {
            action.kamishinka_form = Some(form.id.clone());
            action
        }));
    }
}
