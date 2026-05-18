use engine_rust::core::abilities::{
    run_ability_check_hook, run_ability_value_hook, AbilityCheckContext, AbilityValueContext,
};
use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::events::{apply_event, BattleEvent};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages,
    Status,
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

        evs: EVStats::default(),
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

        weight_kg: 50.0,
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
fn adaptability_uses_double_stab_damage() {
    fn damage_with_ability(ability: Option<&str>) -> i32 {
        let mut move_db = MoveDatabase::new();
        move_db.insert(MoveData {
            id: "flame".to_string(),
            name: Some("Flame".to_string()),
            move_type: Some("fire".to_string()),
            category: Some("physical".to_string()),
            pp: Some(10),
            power: Some(60),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: vec![effect("damage", json!({ "power": 60, "accuracy": 1.0 }))],
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

        let mut attacker = make_creature("c1", "Alpha", ability, vec!["flame".to_string()]);
        attacker.types = vec!["fire".to_string()];
        attacker.attack = 100;
        let mut defender = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
        defender.hp = 200;
        defender.max_hp = 200;
        defender.defense = 100;

        let state = make_state(attacker, defender);
        let actions = vec![
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("flame".to_string()),
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

        let mut rng = || 0.0;
        let engine = BattleEngine::new(move_db, TypeChart::new());
        let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
        200 - next.players[1].team[0].hp
    }

    let normal_stab = damage_with_ability(None);
    let adaptability_stab = damage_with_ability(Some("adaptability"));

    assert!(normal_stab > 0);
    assert!(
        adaptability_stab > normal_stab,
        "adaptability damage ({adaptability_stab}) should exceed normal STAB ({normal_stab})"
    );
}

#[test]
fn cotton_down_triggers_on_contact_attack_damage() {
    let mut move_db = MoveDatabase::new();
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

    let state = make_state(
        make_creature("c1", "Alpha", None, vec!["contact_hit".to_string()]),
        make_creature("c2", "Beta", Some("cotton_down"), vec!["wait".to_string()]),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("contact_hit".to_string()),
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

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());

    assert_eq!(next.players[0].team[0].stages.spe, -1);
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Betaの 特性『わたげ』")));
}

#[test]
fn cotton_down_does_not_trigger_from_drain_recovery_or_non_contact_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "draining_beam".to_string(),
        name: Some("Draining Beam".to_string()),
        move_type: Some("grass".to_string()),
        category: Some("special".to_string()),
        pp: Some(10),
        power: Some(50),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![
            effect("damage", json!({ "power": 50, "accuracy": 1.0 })),
            effect(
                "heal_last_damage",
                json!({ "target": "self", "ratio": 0.5 }),
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

    let mut cotton_user = make_creature(
        "c1",
        "Alpha",
        Some("cotton_down"),
        vec!["draining_beam".to_string()],
    );
    cotton_user.hp = 60;
    let state = make_state(
        cotton_user,
        make_creature("c2", "Beta", None, vec!["wait".to_string()]),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("draining_beam".to_string()),
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

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());

    assert!(next.players[0].team[0].hp > 60);
    assert_eq!(next.players[1].team[0].stages.spe, 0);
    assert!(!next
        .log
        .iter()
        .any(|line| line.contains("Alphaの 特性『わたげ』")));
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
fn unburden_doubles_speed_after_held_item_is_lost() {
    let mut unburden = make_creature("c1", "Alpha", Some("unburden"), vec![]);
    unburden.item = Some("sitrus_berry".to_string());
    unburden.statuses.push(Status {
        id: "item".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });
    let state = make_state(unburden, make_creature("c2", "Beta", None, vec![]));
    let next = apply_event(
        &state,
        &BattleEvent::RemoveStatus {
            target_id: "p1".to_string(),
            status_id: "item".to_string(),
            meta: Map::new(),
        },
    );

    let value = run_ability_value_hook(
        &next,
        "p1",
        "onModifySpeed",
        90.0,
        AbilityValueContext {
            move_data: None,
            category: None,
            target: None,
            weather: None,
            turn: 1,
            stages: None,
        },
    );

    assert_eq!(value, 180.0);
}

#[test]
fn unburden_does_not_activate_when_starting_without_item() {
    let state = make_state(
        make_creature("c1", "Alpha", Some("unburden"), vec![]),
        make_creature("c2", "Beta", None, vec![]),
    );

    let value = run_ability_value_hook(
        &state,
        "p1",
        "onModifySpeed",
        90.0,
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
