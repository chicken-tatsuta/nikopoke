use engine_rust::core::effects::{apply_effects, EffectContext};
use engine_rust::core::events::BattleEvent;
use engine_rust::core::state::{
    BattleState, CreatureState, FieldEffect, FieldState, PlayerState, StatStages, Status,
};
use engine_rust::data::moves::{Effect, MoveDatabase};
use engine_rust::data::type_chart::TypeChart;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum EventKind {
    ApplyFieldStatus,
    ApplyStatus,
    ClearStages,
    CureAllStatus,
    Damage,
    ModifyStage,
    RandomMove,
    RemoveFieldStatus,
    RemoveStatus,
    ReplaceStatus,
    ResetStages,
    Switch,
}

#[derive(Default)]
struct Requirements {
    attacker_types: HashSet<String>,
    attacker_statuses: HashSet<String>,
    target_statuses: HashSet<String>,
    field_statuses: HashSet<String>,
    require_attacker_item: bool,
    require_target_item: bool,
    target_hp_ratio_lt: Option<f64>,
}

fn shuffle_with_seed(values: &mut [String], mut seed: u64) {
    if values.len() < 2 {
        return;
    }
    for idx in (1..values.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let target = (seed as usize) % (idx + 1);
        values.swap(idx, target);
    }
}

fn sample_move_names(move_db: &MoveDatabase) -> Vec<String> {
    let mut reader =
        csv::Reader::from_path("data/2期生男子種族値 - 技一覧.csv").expect("open move csv");
    let mut csv_names: Vec<String> = reader
        .records()
        .filter_map(|record| record.ok())
        .filter_map(|record| record.get(0).map(|name| name.trim().to_string()))
        .collect();

    let available_names: HashSet<String> = move_db
        .as_map()
        .values()
        .filter_map(|move_data| move_data.name.clone())
        .collect();
    csv_names.retain(|name| available_names.contains(name));

    let sample_size = ((csv_names.len() as f64) * 0.3).round() as usize;
    let sample_size = sample_size.max(1);
    shuffle_with_seed(&mut csv_names, 42);
    csv_names.truncate(sample_size);
    csv_names
}

fn build_name_to_id_map(move_db: &MoveDatabase) -> HashMap<String, String> {
    move_db
        .as_map()
        .iter()
        .filter_map(|(id, data)| data.name.as_ref().map(|name| (name.clone(), id.clone())))
        .collect()
}

fn create_creature(id: &str, name: &str, types: Vec<String>) -> CreatureState {
    CreatureState {
        id: id.to_string(),
        species_id: id.to_string(),
        name: name.to_string(),
        level: 50,
        types,
        max_hp: 100,
        hp: 100,
        moves: Vec::new(),
        stages: StatStages::default(),
        statuses: Vec::new(),
        item: None,
        ability: None,
        volatile_data: HashMap::new(),
        ability_data: HashMap::new(),
        move_pp: HashMap::new(),
        attack: 50,
        defense: 50,
        sp_attack: 50,
        sp_defense: 50,
        speed: 50,
        weight_kg: 60.0,
    }
}

fn build_state(requirements: &Requirements) -> BattleState {
    let mut attacker_types = vec!["normal".to_string()];
    for ty in &requirements.attacker_types {
        if !attacker_types.contains(ty) {
            attacker_types.push(ty.clone());
        }
    }
    let mut attacker = create_creature("attacker", "Attacker", attacker_types);
    let mut target = create_creature("target", "Target", vec!["normal".to_string()]);

    for status_id in &requirements.attacker_statuses {
        attacker.statuses.push(Status {
            id: status_id.clone(),
            remaining_turns: None,
            data: HashMap::new(),
        });
    }
    for status_id in &requirements.target_statuses {
        target.statuses.push(Status {
            id: status_id.clone(),
            remaining_turns: None,
            data: HashMap::new(),
        });
    }
    if requirements.require_attacker_item {
        attacker.item = Some("test_item".to_string());
    }
    if requirements.require_target_item {
        target.item = Some("test_item".to_string());
    }
    if let Some(ratio) = requirements.target_hp_ratio_lt {
        let desired = ((target.max_hp as f64) * ratio).floor() as i32 - 1;
        target.hp = desired.clamp(0, target.max_hp);
    }

    let bench_attacker = create_creature("attacker_bench", "Bench", vec!["normal".to_string()]);
    let bench_target = create_creature("target_bench", "Bench", vec!["normal".to_string()]);

    BattleState {
        players: vec![
            PlayerState {
                id: "p1".to_string(),
                name: "Player 1".to_string(),
                team: vec![attacker, bench_attacker],
                active_slot: 0,
                last_fainted_ability: None,
            },
            PlayerState {
                id: "p2".to_string(),
                name: "Player 2".to_string(),
                team: vec![target, bench_target],
                active_slot: 0,
                last_fainted_ability: None,
            },
        ],
        turn: 1,
        field: FieldState {
            global: requirements
                .field_statuses
                .iter()
                .cloned()
                .map(|id| FieldEffect {
                    id,
                    remaining_turns: None,
                    data: HashMap::new(),
                })
                .collect(),
            sides: HashMap::new(),
        },
        log: Vec::new(),
        history: None,
    }
}

fn event_kind(event: &BattleEvent) -> Option<EventKind> {
    match event {
        BattleEvent::ApplyFieldStatus { .. } => Some(EventKind::ApplyFieldStatus),
        BattleEvent::ApplyStatus { .. } => Some(EventKind::ApplyStatus),
        BattleEvent::ClearStages { .. } => Some(EventKind::ClearStages),
        BattleEvent::CureAllStatus { .. } => Some(EventKind::CureAllStatus),
        BattleEvent::Damage { .. } => Some(EventKind::Damage),
        BattleEvent::ModifyStage { .. } => Some(EventKind::ModifyStage),
        BattleEvent::RandomMove { .. } => Some(EventKind::RandomMove),
        BattleEvent::RemoveFieldStatus { .. } => Some(EventKind::RemoveFieldStatus),
        BattleEvent::RemoveStatus { .. } => Some(EventKind::RemoveStatus),
        BattleEvent::ReplaceStatus { .. } => Some(EventKind::ReplaceStatus),
        BattleEvent::ResetStages { .. } => Some(EventKind::ResetStages),
        BattleEvent::Switch { .. } => Some(EventKind::Switch),
        _ => None,
    }
}

fn expected_event_kind(effect: &Effect) -> Option<EventKind> {
    match effect.effect_type.as_str() {
        "apply_field_status" => effect
            .data
            .get("statusId")
            .and_then(|v| v.as_str())
            .map(|_| EventKind::ApplyFieldStatus),
        "apply_item" => Some(EventKind::ApplyStatus),
        "apply_status" => effect
            .data
            .get("statusId")
            .and_then(|v| v.as_str())
            .map(|_| EventKind::ApplyStatus),
        "delay" | "disable_move" | "lock_move" | "over_time" | "replace_pokemon"
        | "self_switch" => Some(EventKind::ApplyStatus),
        "clear_stages" => Some(EventKind::ClearStages),
        "cure_all_status" => Some(EventKind::CureAllStatus),
        "damage" | "damage_ratio" | "ohko" | "speed_based_damage" => Some(EventKind::Damage),
        "modify_stage" => effect
            .data
            .get("stages")
            .and_then(|v| v.as_object())
            .filter(|map| !map.is_empty())
            .map(|_| EventKind::ModifyStage),
        "random_move" => Some(EventKind::RandomMove),
        "remove_field_status" => effect
            .data
            .get("statusId")
            .and_then(|v| v.as_str())
            .map(|_| EventKind::RemoveFieldStatus),
        "remove_item" | "consume_item" => Some(EventKind::RemoveStatus),
        "remove_status" => effect
            .data
            .get("statusId")
            .and_then(|v| v.as_str())
            .map(|_| EventKind::RemoveStatus),
        "replace_status" => {
            if effect.data.get("from").and_then(|v| v.as_str()).is_some()
                && effect.data.get("to").and_then(|v| v.as_str()).is_some()
            {
                Some(EventKind::ReplaceStatus)
            } else {
                None
            }
        }
        "reset_stages" => Some(EventKind::ResetStages),
        "force_switch" => Some(EventKind::Switch),
        _ => None,
    }
}

fn collect_effects_from_value(value: Option<&Value>) -> Vec<Effect> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn record_condition_requirements(cond: &Value, requirements: &mut Requirements) -> bool {
    let Some(cond_map) = cond.as_object() else {
        return false;
    };
    let Some(cond_type) = cond_map.get("type").and_then(|v| v.as_str()) else {
        return false;
    };
    match cond_type {
        "target_has_status" => {
            if let Some(status_id) = cond_map.get("statusId").and_then(|v| v.as_str()) {
                if status_id == "item" || status_id == "berry" {
                    requirements.require_target_item = true;
                } else {
                    requirements.target_statuses.insert(status_id.to_string());
                }
                return true;
            }
        }
        "user_has_status" => {
            if let Some(status_id) = cond_map.get("statusId").and_then(|v| v.as_str()) {
                if status_id == "item" || status_id == "berry" {
                    requirements.require_attacker_item = true;
                } else {
                    requirements.attacker_statuses.insert(status_id.to_string());
                }
                return true;
            }
        }
        "target_hp_lt" => {
            if let Some(value) = cond_map.get("value").and_then(|v| v.as_f64()) {
                requirements.target_hp_ratio_lt = match requirements.target_hp_ratio_lt {
                    Some(existing) => Some(existing.min(value)),
                    None => Some(value),
                };
                return true;
            }
        }
        "field_has_status" => {
            if let Some(status_id) = cond_map.get("statusId").and_then(|v| v.as_str()) {
                requirements.field_statuses.insert(status_id.to_string());
                return true;
            }
        }
        "weather_is_sunny" => {
            requirements.field_statuses.insert("sun".to_string());
            return true;
        }
        "weather_is_raining" => {
            requirements.field_statuses.insert("rain".to_string());
            return true;
        }
        "weather_is_hail" => {
            requirements.field_statuses.insert("hail".to_string());
            return true;
        }
        "weather_is_sandstorm" => {
            requirements.field_statuses.insert("sandstorm".to_string());
            return true;
        }
        "user_type" => {
            if let Some(type_id) = cond_map.get("typeId").and_then(|v| v.as_str()) {
                requirements.attacker_types.insert(type_id.to_string());
                return true;
            }
        }
        "target_has_item" => {
            requirements.require_target_item = true;
            return true;
        }
        "user_has_item" => {
            requirements.require_attacker_item = true;
            return true;
        }
        _ => {}
    }
    false
}

fn collect_expectations(
    effects: &[Effect],
    requirements: &mut Requirements,
    expected: &mut HashSet<EventKind>,
) {
    for effect in effects {
        match effect.effect_type.as_str() {
            "chance" => {
                let p = effect.data.get("p").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let branch = if 0.0 <= p { "then" } else { "else" };
                let nested = collect_effects_from_value(effect.data.get(branch));
                collect_expectations(&nested, requirements, expected);
            }
            "repeat" => {
                let nested = collect_effects_from_value(
                    effect
                        .data
                        .get("steps")
                        .or_else(|| effect.data.get("effects")),
                );
                collect_expectations(&nested, requirements, expected);
            }
            "conditional" => {
                let cond = effect.data.get("if");
                let cond_supported = cond
                    .and_then(|value| Some(record_condition_requirements(value, requirements)))
                    .unwrap_or(false);
                let branch = if cond_supported { "then" } else { "else" };
                let nested = collect_effects_from_value(effect.data.get(branch));
                collect_expectations(&nested, requirements, expected);
            }
            "consume_item" => {
                if effect.data.get("target").and_then(|v| v.as_str()) == Some("self") {
                    requirements.require_attacker_item = true;
                } else {
                    requirements.require_target_item = true;
                }
                if let Some(kind) = expected_event_kind(effect) {
                    expected.insert(kind);
                }
            }
            "manual" => {
                if effect
                    .data
                    .get("manualReason")
                    .and_then(|v| v.as_str())
                    .map(|reason| reason.contains("Switching"))
                    .unwrap_or(false)
                {
                    expected.insert(EventKind::ApplyStatus);
                }
            }
            _ => {
                if let Some(kind) = expected_event_kind(effect) {
                    expected.insert(kind);
                }
            }
        }
    }
}

fn collect_actual_event_kinds(events: &[BattleEvent]) -> HashSet<EventKind> {
    events.iter().filter_map(event_kind).collect()
}

#[test]
fn sampled_move_effects_match_expected_events() {
    let move_db =
        MoveDatabase::load_from_yaml_file("data/moves.yaml".as_ref()).expect("load moves.yaml");
    let name_to_id = build_name_to_id_map(&move_db);
    let sampled_names = sample_move_names(&move_db);
    let type_chart = TypeChart::new();

    for name in sampled_names {
        let Some(move_id) = name_to_id.get(&name) else {
            panic!("missing moves.yaml id for {}", name);
        };
        let move_data = move_db
            .get(move_id)
            .unwrap_or_else(|| panic!("missing move data for {}", move_id));

        let mut requirements = Requirements::default();
        let mut expected = HashSet::new();
        collect_expectations(&move_data.steps, &mut requirements, &mut expected);

        let state = build_state(&requirements);
        let mut rng = || 0.0;
        let mut ctx = EffectContext {
            attacker_player_id: "p1".to_string(),
            target_player_id: "p2".to_string(),
            move_data: Some(move_data),
            rng: &mut rng,
            turn: state.turn,
            type_chart: &type_chart,
            bypass_protect: false,
            ignore_immunity: false,
            bypass_substitute: false,
            ignore_substitute: false,
            is_sound: false,
            last_damage: None,
        };

        let events = apply_effects(&state, &move_data.steps, &mut ctx);
        let actual = collect_actual_event_kinds(&events);

        for kind in expected {
            assert!(
                actual.contains(&kind),
                "expected event {:?} for move {} ({})",
                kind,
                name,
                move_id
            );
        }
    }
}
