use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatStages {
    pub atk: i32,
    pub def: i32,
    pub spa: i32,
    pub spd: i32,
    pub spe: i32,
    pub accuracy: i32,
    pub evasion: i32,
    pub crit: i32,
}

impl Default for StatStages {
    fn default() -> Self {
        Self {
            atk: 0,
            def: 0,
            spa: 0,
            spd: 0,
            spe: 0,
            accuracy: 0,
            evasion: 0,
            crit: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    pub id: String,
    pub remaining_turns: Option<i32>,
    #[serde(default)]
    pub data: HashMap<String, Value>,
}

/// EVStats represents effort values for each stat (max 32 per stat, 66 total)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EVStats {
    pub hp: i32,
    pub atk: i32,
    pub def: i32,
    pub spa: i32,
    pub spd: i32,
    pub spe: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreatureState {
    pub id: String,
    pub species_id: String,
    pub name: String,
    pub level: u32,
    pub types: Vec<String>,
    pub moves: Vec<String>,
    pub ability: Option<String>,
    pub item: Option<String>,
    #[serde(default)]
    pub evs: EVStats,
    pub hp: i32,
    pub max_hp: i32,
    pub stages: StatStages,
    #[serde(default)]
    pub statuses: Vec<Status>,
    #[serde(default)]
    pub move_pp: HashMap<String, i32>,
    #[serde(default)]
    pub ability_data: HashMap<String, Value>,
    #[serde(default)]
    pub volatile_data: HashMap<String, Value>,
    pub attack: i32,
    pub defense: i32,
    pub sp_attack: i32,
    pub sp_defense: i32,
    pub speed: i32,
    #[serde(default)]
    pub weight_kg: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: String,
    pub name: String,
    pub team: Vec<CreatureState>,
    pub active_slot: usize,
    #[serde(default)]
    pub last_fainted_ability: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldEffect {
    pub id: String,
    pub remaining_turns: Option<i32>,
    #[serde(default)]
    pub data: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldState {
    #[serde(default)]
    pub global: Vec<FieldEffect>,
    #[serde(default)]
    pub sides: HashMap<String, Vec<FieldEffect>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleHistory {
    pub turns: Vec<BattleTurn>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleTurn {
    pub turn: u32,
    pub actions: Vec<Action>,
    pub log: Vec<String>,
    pub rng: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleState {
    pub players: Vec<PlayerState>,
    pub field: FieldState,
    pub turn: u32,
    #[serde(default)]
    pub log: Vec<String>,
    pub history: Option<BattleHistory>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Move,
    Switch,
    UseItem,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub player_id: String,
    pub action_type: ActionType,
    pub move_id: Option<String>,
    pub target_id: Option<String>,
    pub slot: Option<usize>,
    pub priority: Option<i32>,
}

pub fn create_battle_state(players: Vec<PlayerState>) -> BattleState {
    BattleState {
        players: players
            .into_iter()
            .map(|mut player| {
                player.active_slot = 0;
                player
            })
            .collect(),
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        turn: 0,
        log: Vec::new(),
        history: None,
    }
}
