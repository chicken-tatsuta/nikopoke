use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::effects::{apply_effects, apply_events, EffectContext};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, FieldState, PlayerState, StatStages, Status,
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

fn make_creature(id: &str, name: &str, moves: Vec<String>) -> CreatureState {
    CreatureState {
        id: id.to_string(),
        species_id: "testmon".to_string(),
        name: name.to_string(),
        level: 50,
        types: vec!["normal".to_string()],
        moves,
        ability: Some("none".to_string()),
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
fn substitute_initializes_hp_on_apply() {
    let state = make_state(
        make_creature("c1", "Alpha", vec!["wait".to_string()]),
        make_creature("c2", "Beta", vec!["wait".to_string()]),
    );
    let mut rng = || 0.0;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: None,
        rng: &mut rng,
        turn: 0,
        type_chart: &type_chart,
        bypass_protect: false,
        ignore_immunity: false,
        bypass_substitute: false,
        ignore_substitute: false,
        is_sound: false,
        last_damage: None,
    };

    let effects = vec![
        effect(
            "damage_ratio",
            json!({ "ratioMaxHp": 0.25, "target": "self" }),
        ),
        effect(
            "apply_status",
            json!({ "statusId": "substitute", "target": "self" }),
        ),
    ];
    let events = apply_effects(&state, &effects, &mut ctx);
    let next = apply_events(&state, &events);

    let active = &next.players[0].team[0];
    assert_eq!(active.hp, 75);
    let substitute = active
        .statuses
        .iter()
        .find(|s| s.id == "substitute")
        .expect("substitute should be applied");
    assert_eq!(substitute.data.get("hp"), Some(&Value::Number(25.into())));
}

#[test]
fn substitute_takes_damage_and_loses_hp() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "tap".to_string(),
        name: Some("Tap".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": 0.1 }))],
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

    let mut target = make_creature("c2", "Beta", vec!["wait".to_string()]);
    let mut data = HashMap::new();
    data.insert("hp".to_string(), Value::Number(12.into()));
    target.statuses.push(Status {
        id: "substitute".to_string(),
        remaining_turns: None,
        data,
    });

    let state = make_state(
        make_creature("c1", "Alpha", vec!["tap".to_string()]),
        target,
    );

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("tap".to_string()),
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
    let substitute = next.players[1].team[0]
        .statuses
        .iter()
        .find(|s| s.id == "substitute")
        .expect("substitute should remain");
    assert_eq!(substitute.data.get("hp"), Some(&Value::Number(2.into())));
}

#[test]
fn substitute_breaks_when_hp_depleted() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "tap".to_string(),
        name: Some("Tap".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": 0.1 }))],
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

    let mut target = make_creature("c2", "Beta", vec!["wait".to_string()]);
    let mut data = HashMap::new();
    data.insert("hp".to_string(), Value::Number(8.into()));
    target.statuses.push(Status {
        id: "substitute".to_string(),
        remaining_turns: None,
        data,
    });

    let state = make_state(
        make_creature("c1", "Alpha", vec!["tap".to_string()]),
        target,
    );

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("tap".to_string()),
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
    let has_substitute = next.players[1].team[0]
        .statuses
        .iter()
        .any(|s| s.id == "substitute");
    assert!(!has_substitute);
}
