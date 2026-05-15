use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages,
    Status,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_f64(&mut self) -> f64 {
        const DEN: f64 = (1u64 << 53) as f64;
        ((self.next_u64() >> 11) as f64) / DEN
    }
}

#[derive(Debug, Clone)]
pub struct CreatureBuilder {
    id: String,
    species_id: String,
    name: String,
    level: u32,
    types: Vec<String>,
    moves: Vec<String>,
    ability: Option<String>,
    item: Option<String>,
    hp: i32,
    max_hp: i32,
    attack: i32,
    defense: i32,
    sp_attack: i32,
    sp_defense: i32,
    speed: i32,
    statuses: Vec<Status>,
}

impl CreatureBuilder {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            species_id: "testmon".to_string(),
            name: name.to_string(),
            level: 50,
            types: vec!["normal".to_string()],
            moves: Vec::new(),
            ability: None,
            item: None,
            hp: 100,
            max_hp: 100,
            attack: 50,
            defense: 50,
            sp_attack: 50,
            sp_defense: 50,
            speed: 50,
            statuses: Vec::new(),
        }
    }

    pub fn species_id(mut self, species_id: &str) -> Self {
        self.species_id = species_id.to_string();
        self
    }

    pub fn level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    pub fn types(mut self, types: &[&str]) -> Self {
        self.types = types.iter().map(|v| (*v).to_string()).collect();
        self
    }

    pub fn moves(mut self, moves: &[&str]) -> Self {
        self.moves = moves.iter().map(|v| (*v).to_string()).collect();
        self
    }

    pub fn ability(mut self, ability: &str) -> Self {
        self.ability = Some(ability.to_string());
        self
    }

    pub fn item(mut self, item: &str) -> Self {
        self.item = Some(item.to_string());
        self
    }

    pub fn hp(mut self, hp: i32, max_hp: i32) -> Self {
        self.hp = hp;
        self.max_hp = max_hp;
        self
    }

    pub fn stats(
        mut self,
        attack: i32,
        defense: i32,
        sp_attack: i32,
        sp_defense: i32,
        speed: i32,
    ) -> Self {
        self.attack = attack;
        self.defense = defense;
        self.sp_attack = sp_attack;
        self.sp_defense = sp_defense;
        self.speed = speed;
        self
    }

    pub fn with_status(mut self, status: Status) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn build(self) -> CreatureState {
        CreatureState {
            id: self.id,
            species_id: self.species_id,
            name: self.name,
            level: self.level,
            types: self.types,
            moves: self.moves,
            ability: self.ability,
            item: self.item,
            evs: EVStats::default(),
            hp: self.hp,
            max_hp: self.max_hp,
            stages: StatStages::default(),
            statuses: self.statuses,
            move_pp: HashMap::new(),
            ability_data: HashMap::new(),
            volatile_data: HashMap::new(),
            attack: self.attack,
            defense: self.defense,
            sp_attack: self.sp_attack,
            sp_defense: self.sp_defense,
            speed: self.speed,
            weight_kg: 50.0,
        }
    }
}

pub fn status(status_id: &str, remaining_turns: Option<i32>) -> Status {
    Status {
        id: status_id.to_string(),
        remaining_turns,
        data: HashMap::new(),
    }
}

pub fn player(id: &str, name: &str, team: Vec<CreatureState>) -> PlayerState {
    PlayerState {
        id: id.to_string(),
        name: name.to_string(),
        team,
        active_slot: 0,
        last_fainted_ability: None,
    }
}

pub fn player_with_active(
    id: &str,
    name: &str,
    team: Vec<CreatureState>,
    active_slot: usize,
) -> PlayerState {
    PlayerState {
        id: id.to_string(),
        name: name.to_string(),
        team,
        active_slot,
        last_fainted_ability: None,
    }
}

pub fn battle_state(players: Vec<PlayerState>) -> BattleState {
    BattleState {
        players,
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        turn: 0,
        log: Vec::new(),
        history: None,
    }
}

pub fn move_action(player_id: &str, move_id: &str, target_id: &str) -> Action {
    Action {
        player_id: player_id.to_string(),
        action_type: ActionType::Move,
        move_id: Some(move_id.to_string()),
        target_id: Some(target_id.to_string()),
        slot: None,
        priority: None,
    }
}

pub fn switch_action(player_id: &str, slot: usize) -> Action {
    Action {
        player_id: player_id.to_string(),
        action_type: ActionType::Switch,
        move_id: None,
        target_id: None,
        slot: Some(slot),
        priority: None,
    }
}

pub fn run_turn_with_seed(
    engine: &BattleEngine,
    state: &BattleState,
    actions: &[Action],
    seed: u64,
) -> BattleState {
    let mut rng = SeededRng::new(seed);
    let mut rng_fn = || rng.next_f64();
    engine.step_battle(state, actions, &mut rng_fn, BattleOptions::default())
}

pub fn run_turns_with_seed(
    engine: &BattleEngine,
    mut state: BattleState,
    turns: &[Vec<Action>],
    seed: u64,
) -> BattleState {
    let mut rng = SeededRng::new(seed);
    let mut rng_fn = || rng.next_f64();
    for actions in turns {
        state = engine.step_battle(&state, actions, &mut rng_fn, BattleOptions::default());
    }
    state
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSnapshot {
    pub active_slot: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub statuses: BTreeSet<String>,
    pub stages: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSnapshot {
    pub players: BTreeMap<String, ActiveSnapshot>,
    pub field_statuses: BTreeSet<String>,
}

pub fn snapshot(state: &BattleState) -> BattleSnapshot {
    let mut players = BTreeMap::new();
    for player in &state.players {
        if let Some(active) = player.team.get(player.active_slot) {
            let statuses = active
                .statuses
                .iter()
                .map(|s| s.id.clone())
                .collect::<BTreeSet<_>>();
            let mut stages = BTreeMap::new();
            stages.insert("atk".to_string(), active.stages.atk);
            stages.insert("def".to_string(), active.stages.def);
            stages.insert("spa".to_string(), active.stages.spa);
            stages.insert("spd".to_string(), active.stages.spd);
            stages.insert("spe".to_string(), active.stages.spe);
            stages.insert("accuracy".to_string(), active.stages.accuracy);
            stages.insert("evasion".to_string(), active.stages.evasion);
            stages.insert("crit".to_string(), active.stages.crit);
            players.insert(
                player.id.clone(),
                ActiveSnapshot {
                    active_slot: player.active_slot,
                    hp: active.hp,
                    max_hp: active.max_hp,
                    statuses,
                    stages,
                },
            );
        }
    }
    let field_statuses = state
        .field
        .global
        .iter()
        .map(|f| f.id.clone())
        .collect::<BTreeSet<_>>();
    BattleSnapshot {
        players,
        field_statuses,
    }
}

pub fn diff_snapshots(lhs: &BattleState, rhs: &BattleState) -> Vec<String> {
    let left = snapshot(lhs);
    let right = snapshot(rhs);
    let mut diffs = Vec::new();

    let player_ids = left
        .players
        .keys()
        .chain(right.players.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for player_id in player_ids {
        match (left.players.get(&player_id), right.players.get(&player_id)) {
            (Some(l), Some(r)) => {
                if l.active_slot != r.active_slot {
                    diffs.push(format!(
                        "[{}] active_slot differs: {} != {}",
                        player_id, l.active_slot, r.active_slot
                    ));
                }
                if l.hp != r.hp || l.max_hp != r.max_hp {
                    diffs.push(format!(
                        "[{}] hp differs: {}/{} != {}/{}",
                        player_id, l.hp, l.max_hp, r.hp, r.max_hp
                    ));
                }
                if l.statuses != r.statuses {
                    diffs.push(format!(
                        "[{}] statuses differ: {:?} != {:?}",
                        player_id, l.statuses, r.statuses
                    ));
                }
                if l.stages != r.stages {
                    diffs.push(format!(
                        "[{}] stages differ: {:?} != {:?}",
                        player_id, l.stages, r.stages
                    ));
                }
            }
            (Some(_), None) => diffs.push(format!("[{}] missing on rhs snapshot", player_id)),
            (None, Some(_)) => diffs.push(format!("[{}] missing on lhs snapshot", player_id)),
            (None, None) => {}
        }
    }

    if left.field_statuses != right.field_statuses {
        diffs.push(format!(
            "[field] statuses differ: {:?} != {:?}",
            left.field_statuses, right.field_statuses
        ));
    }

    diffs
}

pub fn assert_no_diffs(lhs: &BattleState, rhs: &BattleState) {
    let diffs = diff_snapshots(lhs, rhs);
    assert!(
        diffs.is_empty(),
        "state snapshots differ:\n{}",
        diffs.join("\n")
    );
}

pub fn assert_active_hp(state: &BattleState, player_id: &str, expected_hp: i32) {
    let player = state
        .players
        .iter()
        .find(|p| p.id == player_id)
        .unwrap_or_else(|| panic!("player '{}' not found", player_id));
    let active = player
        .team
        .get(player.active_slot)
        .unwrap_or_else(|| panic!("active slot missing for player '{}'", player_id));
    assert_eq!(
        active.hp, expected_hp,
        "unexpected hp for player '{}'",
        player_id
    );
}

pub fn assert_active_has_status(state: &BattleState, player_id: &str, status_id: &str) {
    let player = state
        .players
        .iter()
        .find(|p| p.id == player_id)
        .unwrap_or_else(|| panic!("player '{}' not found", player_id));
    let active = player
        .team
        .get(player.active_slot)
        .unwrap_or_else(|| panic!("active slot missing for player '{}'", player_id));
    let has_status = active.statuses.iter().any(|s| s.id == status_id);
    assert!(
        has_status,
        "player '{}' active should have status '{}'",
        player_id, status_id
    );
}

pub fn assert_field_has_status(state: &BattleState, status_id: &str) {
    let has_status = state.field.global.iter().any(|s| s.id == status_id);
    assert!(has_status, "field should have status '{}'", status_id);
}

pub fn json_number_i32(value: i32) -> Value {
    Value::Number(value.into())
}
