use crate::ai::vega::{
    get_best_move_vega_iterative, get_best_move_vega_with_options_and_db_ref, DEFAULT_PARAMS,
};
use crate::ai::{get_best_move_mcts, get_best_move_minimax};
use crate::core::battle::{
    apply_initial_switch_in_effects, is_battle_over, replace_fainted_pokemon, step_battle,
    BattleOptions,
};
use crate::core::factory::{create_creature, CreateCreatureOptions};
use crate::core::state::{
    create_battle_state, Action, ActionType, BattleHistory, BattleState, BattleTurn,
    CreatureState, FieldEffect, EVStats, FieldState, PlayerState, Status,
};
use crate::data::learnsets::LearnsetDatabase;
use crate::data::moves::MoveDatabase;
use crate::data::species::SpeciesDatabase;
use js_sys::Math;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

static SPECIES_DB: Lazy<SpeciesDatabase> =
    Lazy::new(|| SpeciesDatabase::load_default().unwrap_or_else(|_| SpeciesDatabase::new()));
static LEARNSETS_DB: Lazy<LearnsetDatabase> =
    Lazy::new(|| LearnsetDatabase::load_default().unwrap_or_default());
static MOVE_DB: Lazy<MoveDatabase> =
    Lazy::new(|| MoveDatabase::load_default().unwrap_or_else(|_| MoveDatabase::minimal()));

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EVStatsWire {
    #[serde(default)]
    hp: Option<i32>,
    #[serde(default)]
    atk: Option<i32>,
    #[serde(default)]
    def: Option<i32>,
    #[serde(default)]
    spa: Option<i32>,
    #[serde(default)]
    spd: Option<i32>,
    #[serde(default)]
    spe: Option<i32>,
}

impl From<EVStatsWire> for EVStats {
    fn from(wire: EVStatsWire) -> Self {
        Self {
            hp: wire.hp.unwrap_or(0),
            atk: wire.atk.unwrap_or(0),
            def: wire.def.unwrap_or(0),
            spa: wire.spa.unwrap_or(0),
            spd: wire.spd.unwrap_or(0),
            spe: wire.spe.unwrap_or(0),
        }
    }
}

impl From<EVStats> for EVStatsWire {
    fn from(evs: EVStats) -> Self {
        Self {
            hp: Some(evs.hp),
            atk: Some(evs.atk),
            def: Some(evs.def),
            spa: Some(evs.spa),
            spd: Some(evs.spd),
            spe: Some(evs.spe),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCreatureOptionsWire {
    moves: Option<Vec<String>>,
    ability: Option<String>,
    name: Option<String>,
    level: Option<u32>,
    item: Option<String>,
    evs: Option<EVStatsWire>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusWire {
    id: String,
    remaining_turns: Option<i32>,
    #[serde(default)]
    data: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FieldEffectWire {
    id: String,
    remaining_turns: Option<i32>,
    #[serde(default)]
    data: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatureStateWire {
    id: String,
    species_id: String,
    name: String,
    level: u32,
    types: Vec<String>,
    moves: Vec<String>,
    ability: Option<String>,
    item: Option<String>,
    #[serde(default)]
    evs: EVStatsWire,
    hp: i32,
    max_hp: i32,
    stages: crate::core::state::StatStages,
    #[serde(default)]
    statuses: Vec<StatusWire>,
    #[serde(default)]
    move_pp: HashMap<String, i32>,
    #[serde(default)]
    ability_data: HashMap<String, Value>,
    #[serde(default)]
    volatile_data: HashMap<String, Value>,
    attack: i32,
    defense: i32,
    sp_attack: i32,
    sp_defense: i32,
    speed: i32,
    #[serde(default)]
    weight_kg: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerStateWire {
    id: String,
    name: String,
    team: Vec<CreatureStateWire>,
    #[serde(default)]
    active_slot: usize,
    #[serde(default)]
    last_fainted_ability: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FieldStateWire {
    #[serde(default)]
    global: Vec<FieldEffectWire>,
    #[serde(default)]
    sides: HashMap<String, Vec<FieldEffectWire>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionWire {
    #[serde(rename = "type")]
    action_type: String,
    player_id: String,
    move_id: Option<String>,
    target_id: Option<String>,
    slot: Option<usize>,
    priority: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BattleTurnWire {
    turn: u32,
    actions: Vec<ActionWire>,
    log: Vec<String>,
    rng: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BattleHistoryWire {
    turns: Vec<BattleTurnWire>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BattleStateWire {
    players: Vec<PlayerStateWire>,
    field: FieldStateWire,
    turn: u32,
    #[serde(default)]
    log: Vec<String>,
    history: Option<BattleHistoryWire>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StepBattleOptionsWire {
    record_history: Option<bool>,
}

fn js_err(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

fn action_type_from_js(value: &str) -> Result<ActionType, String> {
    match value {
        "move" => Ok(ActionType::Move),
        "switch" => Ok(ActionType::Switch),
        "use_item" => Ok(ActionType::UseItem),
        other => Err(format!("Unknown action type: {}", other)),
    }
}

fn action_type_to_js(value: &ActionType) -> &'static str {
    match value {
        ActionType::Move => "move",
        ActionType::Switch => "switch",
        ActionType::UseItem => "use_item",
    }
}

fn default_moves(species_id: &str) -> Vec<String> {
    let Some(learnset) = LEARNSETS_DB.get(species_id) else {
        return Vec::new();
    };
    learnset
        .iter()
        .filter(|move_id| MOVE_DB.get(move_id.as_str()).is_some())
        .take(4)
        .cloned()
        .collect()
}

fn normalize_requested_moves(species_id: &str, requested_moves: &[String]) -> Vec<String> {
    let learnset = LEARNSETS_DB.get(species_id);

    let mut selected = Vec::new();

    for move_id in requested_moves {
        if selected.len() >= 4 {
            break;
        }

        if selected.iter().any(|selected_id| selected_id == move_id) {
            continue;
        }

        if MOVE_DB.get(move_id.as_str()).is_some() {
            selected.push(move_id.clone());
        }
    }

    if selected.len() >= 4 {
        return selected;
    }

    if let Some(learnset) = learnset {
        for move_id in learnset {
            if selected.len() >= 4 {
                break;
            }

            if selected.iter().any(|selected_id| selected_id == move_id) {
                continue;
            }

            if MOVE_DB.get(move_id.as_str()).is_some() {
                selected.push(move_id.clone());
            }
        }
    }

    selected
}
impl From<Status> for StatusWire {
    fn from(status: Status) -> Self {
        Self {
            id: status.id,
            remaining_turns: status.remaining_turns,
            data: status.data,
        }
    }
}

impl From<StatusWire> for Status {
    fn from(status: StatusWire) -> Self {
        Self {
            id: status.id,
            remaining_turns: status.remaining_turns,
            data: status.data,
        }
    }
}

impl From<FieldEffect> for FieldEffectWire {
    fn from(effect: FieldEffect) -> Self {
        Self {
            id: effect.id,
            remaining_turns: effect.remaining_turns,
            data: effect.data,
        }
    }
}

impl From<FieldEffectWire> for FieldEffect {
    fn from(effect: FieldEffectWire) -> Self {
        Self {
            id: effect.id,
            remaining_turns: effect.remaining_turns,
            data: effect.data,
        }
    }
}

impl From<CreatureState> for CreatureStateWire {
    fn from(creature: CreatureState) -> Self {
        Self {
            id: creature.id,
            species_id: creature.species_id,
            name: creature.name,
            level: creature.level,
            types: creature.types,
            moves: creature.moves,
            ability: creature.ability,
            item: creature.item,
            evs: EVStatsWire::from(creature.evs),
            hp: creature.hp,
            max_hp: creature.max_hp,
            stages: creature.stages,
            statuses: creature
                .statuses
                .into_iter()
                .map(StatusWire::from)
                .collect(),
            move_pp: creature.move_pp,
            ability_data: creature.ability_data,
            volatile_data: creature.volatile_data,
            attack: creature.attack,
            defense: creature.defense,
            sp_attack: creature.sp_attack,
            sp_defense: creature.sp_defense,
            speed: creature.speed,
            weight_kg: creature.weight_kg,
        }
    }
}

impl From<CreatureStateWire> for CreatureState {
    fn from(creature: CreatureStateWire) -> Self {
        Self {
            id: creature.id,
            species_id: creature.species_id,
            name: creature.name,
            level: creature.level,
            types: creature.types,
            moves: creature.moves,
            ability: creature.ability,
            item: creature.item,
            evs: EVStats::from(creature.evs),
            hp: creature.hp,
            max_hp: creature.max_hp,
            stages: creature.stages,
            statuses: creature.statuses.into_iter().map(Status::from).collect(),
            move_pp: creature.move_pp,
            ability_data: creature.ability_data,
            volatile_data: creature.volatile_data,
            attack: creature.attack,
            defense: creature.defense,
            sp_attack: creature.sp_attack,
            sp_defense: creature.sp_defense,
            speed: creature.speed,
            weight_kg: creature.weight_kg,
        }
    }
}

impl From<PlayerState> for PlayerStateWire {
    fn from(player: PlayerState) -> Self {
        Self {
            id: player.id,
            name: player.name,
            team: player
                .team
                .into_iter()
                .map(CreatureStateWire::from)
                .collect(),
            active_slot: player.active_slot,
            last_fainted_ability: player.last_fainted_ability,
        }
    }
}

impl From<PlayerStateWire> for PlayerState {
    fn from(player: PlayerStateWire) -> Self {
        Self {
            id: player.id,
            name: player.name,
            team: player.team.into_iter().map(CreatureState::from).collect(),
            active_slot: player.active_slot,
            last_fainted_ability: player.last_fainted_ability,
        }
    }
}

impl From<FieldState> for FieldStateWire {
    fn from(field: FieldState) -> Self {
        Self {
            global: field
                .global
                .into_iter()
                .map(FieldEffectWire::from)
                .collect(),
            sides: field
                .sides
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(FieldEffectWire::from).collect()))
                .collect(),
        }
    }
}

impl From<FieldStateWire> for FieldState {
    fn from(field: FieldStateWire) -> Self {
        Self {
            global: field.global.into_iter().map(FieldEffect::from).collect(),
            sides: field
                .sides
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(FieldEffect::from).collect()))
                .collect(),
        }
    }
}

impl From<Action> for ActionWire {
    fn from(action: Action) -> Self {
        Self {
            action_type: action_type_to_js(&action.action_type).to_string(),
            player_id: action.player_id,
            move_id: action.move_id,
            target_id: action.target_id,
            slot: action.slot,
            priority: action.priority,
        }
    }
}

impl TryFrom<ActionWire> for Action {
    type Error = String;

    fn try_from(action: ActionWire) -> Result<Self, Self::Error> {
        Ok(Self {
            player_id: action.player_id,
            action_type: action_type_from_js(&action.action_type)?,
            move_id: action.move_id,
            target_id: action.target_id,
            slot: action.slot,
            priority: action.priority,
        })
    }
}

impl From<BattleTurn> for BattleTurnWire {
    fn from(turn: BattleTurn) -> Self {
        Self {
            turn: turn.turn,
            actions: turn.actions.into_iter().map(ActionWire::from).collect(),
            log: turn.log,
            rng: turn.rng,
        }
    }
}

impl TryFrom<BattleTurnWire> for BattleTurn {
    type Error = String;

    fn try_from(turn: BattleTurnWire) -> Result<Self, Self::Error> {
        Ok(Self {
            turn: turn.turn,
            actions: turn
                .actions
                .into_iter()
                .map(Action::try_from)
                .collect::<Result<_, _>>()?,
            log: turn.log,
            rng: turn.rng,
        })
    }
}

impl From<BattleHistory> for BattleHistoryWire {
    fn from(history: BattleHistory) -> Self {
        Self {
            turns: history
                .turns
                .into_iter()
                .map(BattleTurnWire::from)
                .collect(),
        }
    }
}

impl TryFrom<BattleHistoryWire> for BattleHistory {
    type Error = String;

    fn try_from(history: BattleHistoryWire) -> Result<Self, Self::Error> {
        Ok(Self {
            turns: history
                .turns
                .into_iter()
                .map(BattleTurn::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<BattleState> for BattleStateWire {
    fn from(state: BattleState) -> Self {
        Self {
            players: state
                .players
                .into_iter()
                .map(PlayerStateWire::from)
                .collect(),
            field: FieldStateWire::from(state.field),
            turn: state.turn,
            log: state.log,
            history: state.history.map(BattleHistoryWire::from),
        }
    }
}

impl TryFrom<BattleStateWire> for BattleState {
    type Error = String;

    fn try_from(state: BattleStateWire) -> Result<Self, Self::Error> {
        Ok(Self {
            players: state.players.into_iter().map(PlayerState::from).collect(),
            field: FieldState::from(state.field),
            turn: state.turn,
            log: state.log,
            history: match state.history {
                Some(history) => Some(BattleHistory::try_from(history)?),
                None => None,
            },
        })
    }
}

#[wasm_bindgen(js_name = createCreature)]
pub fn create_creature_wasm(species_id: String, options: JsValue) -> Result<JsValue, JsValue> {
    let options: CreateCreatureOptionsWire = if options.is_undefined() || options.is_null() {
        CreateCreatureOptionsWire::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(js_err)?
    };
    let species = SPECIES_DB
        .get(species_id.as_str())
        .ok_or_else(|| js_err(format!("Unknown species id: {}", species_id)))?;

    let requested_moves = options.moves.clone().unwrap_or_default();
    let selected_moves = normalize_requested_moves(species_id.as_str(), &requested_moves);

    let evs = options.evs.clone().map(EVStats::from);
    let build_options = |moves: Vec<String>| CreateCreatureOptions {
        moves: if moves.is_empty() { None } else { Some(moves) },
        ability: options.ability.clone(),
        name: options.name.clone(),
        level: options.level,
        item: options.item.clone(),
        evs: evs.clone(),
    };

    let creature = create_creature(
        species,
        build_options(selected_moves),
        &LEARNSETS_DB,
        &MOVE_DB,
    )
    .or_else(|_| {
        let fallback_moves = default_moves(species_id.as_str());
        create_creature(
            species,
            build_options(fallback_moves),
            &LEARNSETS_DB,
            &MOVE_DB,
        )
    })
    .map_err(js_err)?;

    serde_wasm_bindgen::to_value(&CreatureStateWire::from(creature)).map_err(js_err)
}

#[wasm_bindgen(js_name = createBattleState)]
pub fn create_battle_state_wasm(players: JsValue) -> Result<JsValue, JsValue> {
    let players_wire: Vec<PlayerStateWire> =
        serde_wasm_bindgen::from_value(players).map_err(js_err)?;
    let players: Vec<PlayerState> = players_wire.into_iter().map(PlayerState::from).collect();
    let state = create_battle_state(players);
    let mut rng = || Math::random();
    let state = apply_initial_switch_in_effects(&state, &mut rng);
    serde_wasm_bindgen::to_value(&BattleStateWire::from(state)).map_err(js_err)
}

#[wasm_bindgen(js_name = stepBattle)]
pub fn step_battle_wasm(
    state: JsValue,
    actions: JsValue,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let state_wire: BattleStateWire = serde_wasm_bindgen::from_value(state).map_err(js_err)?;
    let actions_wire: Vec<ActionWire> = serde_wasm_bindgen::from_value(actions).map_err(js_err)?;
    let options_wire: StepBattleOptionsWire = if options.is_undefined() || options.is_null() {
        StepBattleOptionsWire::default()
    } else {
        serde_wasm_bindgen::from_value(options).map_err(js_err)?
    };
    let state = BattleState::try_from(state_wire).map_err(js_err)?;
    let actions: Vec<Action> = actions_wire
        .into_iter()
        .map(Action::try_from)
        .collect::<Result<_, _>>()
        .map_err(js_err)?;
    let mut rng = || Math::random();
    let options = BattleOptions {
        record_history: options_wire.record_history.unwrap_or(true),
    };
    let next_state = step_battle(&state, &actions, &mut rng, options);
    serde_wasm_bindgen::to_value(&BattleStateWire::from(next_state)).map_err(js_err)
}

#[wasm_bindgen(js_name = replaceFaintedPokemon)]
pub fn replace_fainted_pokemon_wasm(
    state: JsValue,
    player_id: String,
    slot: usize,
) -> Result<JsValue, JsValue> {
    let state_wire: BattleStateWire = serde_wasm_bindgen::from_value(state).map_err(js_err)?;
    let state = BattleState::try_from(state_wire).map_err(js_err)?;
    let mut rng = || Math::random();
    let next_state = replace_fainted_pokemon(&state, player_id.as_str(), slot, &mut rng);
    serde_wasm_bindgen::to_value(&BattleStateWire::from(next_state)).map_err(js_err)
}

#[wasm_bindgen(js_name = isBattleOver)]
pub fn is_battle_over_wasm(state: JsValue) -> Result<bool, JsValue> {
    let state_wire: BattleStateWire = serde_wasm_bindgen::from_value(state).map_err(js_err)?;
    let state = BattleState::try_from(state_wire).map_err(js_err)?;
    Ok(is_battle_over(&state))
}

#[wasm_bindgen(js_name = getBestMoveMinimax)]
pub fn get_best_move_minimax_wasm(
    state: JsValue,
    player_id: String,
    depth: usize,
) -> Result<JsValue, JsValue> {
    let state_wire: BattleStateWire = serde_wasm_bindgen::from_value(state).map_err(js_err)?;
    let state = BattleState::try_from(state_wire).map_err(js_err)?;
    let action = get_best_move_minimax(&state, player_id.as_str(), depth);
    serde_wasm_bindgen::to_value(&action.map(ActionWire::from)).map_err(js_err)
}

#[wasm_bindgen(js_name = getBestMoveVega)]
pub fn get_best_move_vega_wasm(
    state: JsValue,
    player_id: String,
    depth: usize,
) -> Result<JsValue, JsValue> {
    let branch_limit = if depth >= 3 { 2 } else { 4 };
    get_best_move_vega_with_branch_wasm(state, player_id, depth, branch_limit)
}

#[wasm_bindgen(js_name = getBestMoveVegaWithBranch)]
pub fn get_best_move_vega_with_branch_wasm(
    state: JsValue,
    player_id: String,
    depth: usize,
    branch_limit: usize,
) -> Result<JsValue, JsValue> {
    let state_wire: BattleStateWire = serde_wasm_bindgen::from_value(state).map_err(js_err)?;
    let state = BattleState::try_from(state_wire).map_err(js_err)?;
    let action = get_best_move_vega_with_options_and_db_ref(
        &state,
        player_id.as_str(),
        depth,
        DEFAULT_PARAMS,
        branch_limit,
        &MOVE_DB,
    );
    serde_wasm_bindgen::to_value(&action.map(ActionWire::from)).map_err(js_err)
}

#[wasm_bindgen(js_name = getBestMoveVegaIterative)]
pub fn get_best_move_vega_iterative_wasm(
    state: JsValue,
    player_id: String,
    max_depth: usize,
    node_budget: u32,
) -> Result<JsValue, JsValue> {
    let state_wire: BattleStateWire = serde_wasm_bindgen::from_value(state).map_err(js_err)?;
    let state = BattleState::try_from(state_wire).map_err(js_err)?;
    let mut stats = crate::ai::vega::VegaStats::default();
    let branch_limit = 4;
    let action = get_best_move_vega_iterative(
        &state,
        player_id.as_str(),
        max_depth,
        node_budget as u64,
        DEFAULT_PARAMS,
        branch_limit,
        &MOVE_DB,
        &mut stats,
    );
    serde_wasm_bindgen::to_value(&action.map(ActionWire::from)).map_err(js_err)
}

#[wasm_bindgen(js_name = getBestMoveMCTS)]
pub fn get_best_move_mcts_wasm(
    state: JsValue,
    player_id: String,
    iterations: usize,
) -> Result<JsValue, JsValue> {
    let state_wire: BattleStateWire = serde_wasm_bindgen::from_value(state).map_err(js_err)?;
    let state = BattleState::try_from(state_wire).map_err(js_err)?;
    let action = get_best_move_mcts(&state, player_id.as_str(), iterations);
    serde_wasm_bindgen::to_value(&action.map(ActionWire::from)).map_err(js_err)
}
