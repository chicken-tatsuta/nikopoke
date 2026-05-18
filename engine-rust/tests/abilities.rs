use engine_rust::core::abilities::{
    run_ability_check_hook, run_ability_value_hook, AbilityCheckContext, AbilityValueContext,
};
use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, FieldState, PlayerState, StatStages,
};
use engine_rust::data::moves::{Effect, MoveData, MoveDatabase};
use engine_rust::data::type_chart::TypeChart;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

fn effect(effect_type: &str, data: Value) -> Effect {
    let map: Map<String, Value> = data.as_object().cloned().unwrap_or_default();
    Effect {
        effect_type: effect_type.to_string(),
        data: map,
    }
}

fn make_creature(id: &str, name: &str, ability: Option<&str>, moves: Vec<String>) -> CreatureState {
    CreatureState {
        id: id.to_string(),
        species_id: "testmon".to_string(),
        name: name.to_string(),
        level: 50,
        types: vec!["normal".to_string()],
        moves,
        ability: ability.map(|a| a.to_string()),
        item: None,
        hp: 100,
        max_hp: 100,
        stages: StatStages::default(),
        statuses: Vec::new(),
        move_pp: HashMap::new(),
        ability_data: HashMap::new(),
        volatile_data: HashMap::new(),
        attack: 50,
        defense: 50,
        sp_attack: 50,
        sp_defense: 50,
        speed: 50,
        weight_kg: 100.0,
    }
}

fn make_state(p1: CreatureState, p2: CreatureState) -> BattleState {
    BattleState {
        players: vec![
            PlayerState {
                id: "p1".to_string(),
                name: "P1".to_string(),
                team: vec![p1],
                active_slot: 0,
                last_fainted_ability: None,
            },
            PlayerState {
                id: "p2".to_string(),
                name: "P2".to_string(),
                team: vec![p2],
                active_slot: 0,
                last_fainted_ability: None,
            },
        ],
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        turn: 0,
        log: Vec::new(),
        history: None,
    }
}

#[test]
fn soundproof_blocks_sound_tagged_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "echo".to_string(),
        name: Some("Echo".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("special".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": 0.1 }))],
        tags: vec!["sound".to_string()],
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let state = make_state(
        make_creature("c1", "Alpha", None, vec!["echo".to_string()]),
        make_creature("c2", "Beta", Some("soundproof"), vec!["wait".to_string()]),
    );

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("echo".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: Some("wait".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert_eq!(next.players[1].team[0].hp, 100);
}

#[test]
fn technician_boosts_low_power_damage() {
    let state = make_state(
        make_creature("c1", "Alpha", Some("technician"), vec![]),
        make_creature("c2", "Beta", None, vec![]),
    );

    let value = run_ability_value_hook(
        &state,
        "p1",
        "onModifyPower",
        60.0,
        AbilityValueContext {
            move_data: None,
            category: None,
            target: None,
            weather: None,
            turn: 1,
            stages: None,
        },
    );

    assert_eq!(value, 90.0);
}

#[test]
fn shadow_tag_traps_other_creature() {
    let state = make_state(
        make_creature("c1", "Alpha", Some("shadow_tag"), vec![]),
        make_creature("c2", "Beta", None, vec![]),
    );

    let trapped = run_ability_check_hook(
        &state,
        "p1",
        "onTrap",
        AbilityCheckContext {
            status_id: None,
            r#type: None,
            target_id: Some("p2"),
            action: None,
        },
        false,
    );

    assert!(trapped);
}

#[test]
fn stamina_only_reacts_to_opponent_attack_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "substitute".to_string(),
        name: Some("Substitute".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![
            effect(
                "damage_ratio",
                json!({ "ratioMaxHp": 0.25, "target": "self" }),
            ),
            effect(
                "apply_status",
                json!({ "statusId": "substitute", "target": "self" }),
            ),
        ],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "tackle".to_string(),
        name: Some("Tackle".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut state = make_state(
        make_creature(
            "c1",
            "Alpha",
            Some("stamina"),
            vec!["substitute".to_string(), "wait".to_string()],
        ),
        make_creature(
            "c2",
            "Beta",
            None,
            vec!["tackle".to_string(), "wait".to_string()],
        ),
    );
    let mut rng = || 0.99;

    state = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("substitute".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(state.players[0].team[0].stages.def, 0);
    assert_eq!(state.players[0].team[0].hp, 75);
    assert_eq!(
        state.players[0].team[0]
            .volatile_data
            .get("rageFistHitCount")
            .and_then(|v| v.as_i64()),
        None
    );

    state.players[0].team[0].statuses.clear();
    state = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("tackle".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(state.players[0].team[0].stages.def, 1);
    assert_eq!(
        state.players[0].team[0]
            .volatile_data
            .get("rageFistHitCount")
            .and_then(|v| v.as_i64()),
        Some(1)
    );
}

#[test]
fn cotton_down_only_reacts_to_opponent_contact_attack_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "giga_drain".to_string(),
        name: Some("ギガドレイン".to_string()),
        move_type: Some("grass".to_string()),
        category: Some("special".to_string()),
        pp: Some(10),
        power: Some(75),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![
            effect("damage", json!({ "power": 75, "accuracy": 1.0 })),
            effect(
                "damage_ratio",
                json!({ "ratioMaxHp": -0.5, "target": "self" }),
            ),
        ],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "water_gun".to_string(),
        name: Some("Water Gun".to_string()),
        move_type: Some("water".to_string()),
        category: Some("special".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "contact_hit".to_string(),
        name: Some("Contact Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        tags: vec!["contact".to_string()],
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut state = make_state(
        make_creature(
            "c1",
            "Alpha",
            Some("cotton_down"),
            vec!["giga_drain".to_string(), "wait".to_string()],
        ),
        make_creature(
            "c2",
            "Beta",
            None,
            vec![
                "water_gun".to_string(),
                "contact_hit".to_string(),
                "wait".to_string(),
            ],
        ),
    );
    let mut rng = || 0.99;

    state = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("giga_drain".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(state.players[1].team[0].stages.spe, 0);

    state = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("water_gun".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(state.players[1].team[0].stages.spe, 0);

    state = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("contact_hit".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(state.players[1].team[0].stages.spe, -1);
}

#[test]
fn rage_fist_power_uses_attack_damage_hit_count_only() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "rage_fist".to_string(),
        name: Some("Rage Fist".to_string()),
        move_type: Some("ghost".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(50),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 50, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut base = make_state(
        make_creature("c1", "Alpha", None, vec!["rage_fist".to_string()]),
        make_creature("c2", "Beta", None, vec!["wait".to_string()]),
    );
    base.players[1].team[0].types = vec!["psychic".to_string()];
    let action = [
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("rage_fist".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: Some("wait".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];
    let mut rng = || 0.99;
    let no_count = engine.step_battle(&base, &action, &mut rng, BattleOptions::default());

    let mut counted = base.clone();
    counted.players[0].team[0]
        .volatile_data
        .insert("rageFistHitCount".to_string(), Value::Number(2.into()));
    let mut rng = || 0.99;
    let with_count = engine.step_battle(&counted, &action, &mut rng, BattleOptions::default());

    assert!(with_count.players[1].team[0].hp < no_count.players[1].team[0].hp);
}

#[test]
fn libero_changes_type_only_when_its_action_starts_after_taking_faster_hit() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "leaf_hit".to_string(),
        name: Some("Leaf Hit".to_string()),
        move_type: Some("grass".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "quick_hit".to_string(),
        name: Some("Quick Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    });

    let mut state = make_state(
        make_creature("c1", "Alpha", Some("libero"), vec!["leaf_hit".to_string()]),
        make_creature("c2", "Beta", None, vec!["quick_hit".to_string()]),
    );
    state.players[0].team[0].speed = 10;
    state.players[1].team[0].speed = 100;

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut rng = || 0.99;
    let next = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("leaf_hit".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("quick_hit".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );

    assert_eq!(next.players[0].team[0].types, vec!["grass".to_string()]);
    let damage_log = next
        .log
        .iter()
        .position(|line| line.contains("Alphaは") && line.contains("ダメージ"));
    let libero_log = next
        .log
        .iter()
        .position(|line| line.contains("Alphaは grassタイプに 変化した"));
    assert!(damage_log.is_some() && libero_log.is_some());
    assert!(damage_log.unwrap() < libero_log.unwrap());
}

#[test]
fn libero_type_is_restored_when_switching_out_and_back_in() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "leaf_hit".to_string(),
        name: Some("Leaf Hit".to_string()),
        move_type: Some("grass".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let mut libero = make_creature("c1", "Alpha", Some("libero"), vec!["leaf_hit".to_string()]);
    libero.ability_data.insert(
        "baseTypes".to_string(),
        Value::Array(vec![Value::String("normal".to_string())]),
    );
    let bench = make_creature("c3", "Gamma", None, vec!["wait".to_string()]);
    let foe = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
    let state = BattleState {
        players: vec![
            PlayerState {
                id: "p1".to_string(),
                name: "P1".to_string(),
                team: vec![libero, bench],
                active_slot: 0,
                last_fainted_ability: None,
            },
            PlayerState {
                id: "p2".to_string(),
                name: "P2".to_string(),
                team: vec![foe],
                active_slot: 0,
                last_fainted_ability: None,
            },
        ],
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        turn: 0,
        log: Vec::new(),
        history: None,
    };

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut rng = || 0.99;
    let changed = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("leaf_hit".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(changed.players[0].team[0].types, vec!["grass".to_string()]);

    let switched_out = engine.step_battle(
        &changed,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Switch,
                move_id: None,
                target_id: None,
                slot: Some(1),
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(
        switched_out.players[0].team[0].types,
        vec!["normal".to_string()]
    );

    let switched_back = engine.step_battle(
        &switched_out,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Switch,
                move_id: None,
                target_id: None,
                slot: Some(0),
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    assert_eq!(
        switched_back.players[0].team[0].types,
        vec!["normal".to_string()]
    );
    assert_eq!(switched_back.players[0].active_slot, 0);
}

#[test]
fn prankster_status_move_fails_against_dark_target() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "taunt".to_string(),
        name: Some("ちょうはつ".to_string()),
        move_type: Some("dark".to_string()),
        category: Some("status".to_string()),
        pp: Some(20),
        power: None,
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "apply_status",
            json!({ "statusId": "taunt", "target": "target", "duration": 3 }),
        )],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let mut state = make_state(
        make_creature("c1", "Alpha", Some("prankster"), vec!["taunt".to_string()]),
        make_creature("c2", "Beta", None, vec!["wait".to_string()]),
    );
    state.players[1].team[0].types = vec!["dark".to_string()];

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut rng = || 0.99;
    let next = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("taunt".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );

    assert!(next
        .log
        .iter()
        .any(|line| line.contains("うまく きまらなかった")));
    assert!(!next.players[1].team[0]
        .statuses
        .iter()
        .any(|status| status.id == "taunt"));
}

#[test]
fn prankster_targeted_status_effects_do_not_apply_to_dark_target() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "swagger".to_string(),
        name: Some("いばる".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(15),
        power: None,
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![
            effect(
                "apply_status",
                json!({ "statusId": "confusion", "target": "target" }),
            ),
            effect(
                "modify_stage",
                json!({ "target": "target", "stages": { "atk": 2 } }),
            ),
        ],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let mut state = make_state(
        make_creature(
            "c1",
            "Alpha",
            Some("prankster"),
            vec!["swagger".to_string()],
        ),
        make_creature("c2", "Beta", None, vec!["wait".to_string()]),
    );
    state.players[1].team[0].types = vec!["dark".to_string()];

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut rng = || 0.99;
    let next = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("swagger".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );

    assert!(next
        .log
        .iter()
        .any(|line| line.contains("うまく きまらなかった")));
    assert!(next.players[1].team[0].statuses.is_empty());
    assert_eq!(next.players[1].team[0].stages.atk, 0);
}

#[test]
fn prankster_field_status_is_not_blocked_by_dark_target() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "reflect".to_string(),
        name: Some("リフレクター".to_string()),
        move_type: Some("psychic".to_string()),
        category: Some("status".to_string()),
        pp: Some(20),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "apply_field_status",
            json!({ "statusId": "reflect", "duration": 5 }),
        )],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "wait".to_string(),
        name: Some("Wait".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![],
        tags: Vec::new(),
        crit_rate: None,
    });

    let mut state = make_state(
        make_creature(
            "c1",
            "Alpha",
            Some("prankster"),
            vec!["reflect".to_string()],
        ),
        make_creature("c2", "Beta", None, vec!["wait".to_string()]),
    );
    state.players[1].team[0].types = vec!["dark".to_string()];

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut rng = || 0.99;
    let next = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("reflect".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p1".to_string()),
                slot: None,
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );

    assert!(next
        .field
        .global
        .iter()
        .any(|effect| effect.id == "reflect"));
    assert!(!next
        .log
        .iter()
        .any(|line| line.contains("うまく きまらなかった")));
}
