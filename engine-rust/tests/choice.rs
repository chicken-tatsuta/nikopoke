use engine_rust::core::battle::{BattleEngine, BattleOptions};
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
        weight_kg: 60.0,
    }
}

fn make_state(p1_team: Vec<CreatureState>, p2_team: Vec<CreatureState>) -> BattleState {
    BattleState {
        players: vec![
            PlayerState {
                id: "p1".to_string(),
                name: "P1".to_string(),
                team: p1_team,
                active_slot: 0,
                last_fainted_ability: None,
            },
            PlayerState {
                id: "p2".to_string(),
                name: "P2".to_string(),
                team: p2_team,
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
fn pending_switch_blocks_non_switch_action() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": 0.5 }))],
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

    let mut p1 = make_creature("c1", "Alpha", vec!["hit".to_string()]);
    p1.statuses.push(Status {
        id: "pending_switch".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });

    let state = make_state(
        vec![p1],
        vec![make_creature("c2", "Beta", vec!["wait".to_string()])],
    );
    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("hit".to_string()),
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
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("must switch out") || line.contains("交代しなければならない")));
}

#[test]
fn self_switch_requires_choice_and_clears_after_switch() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "pivot".to_string(),
        name: Some("Pivot".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("self_switch", json!({}))],
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

    let state = make_state(
        vec![
            make_creature("c1", "Alpha", vec!["pivot".to_string()]),
            make_creature("c3", "Gamma", vec!["wait".to_string()]),
        ],
        vec![make_creature("c2", "Beta", vec!["wait".to_string()])],
    );
    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("pivot".to_string()),
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
    let has_pending = next.players[0].team[0]
        .statuses
        .iter()
        .any(|s| s.id == "pending_switch");
    assert!(has_pending);

    let follow_actions = vec![
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
    ];

    let final_state =
        engine.step_battle(&next, &follow_actions, &mut rng, BattleOptions::default());
    assert_eq!(final_state.players[0].active_slot, 1);
    let incoming = &final_state.players[0].team[1];
    assert!(!incoming.statuses.iter().any(|s| s.id == "pending_switch"));
}

#[test]
fn manual_switch_effect_sets_pending_switch() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "teleport".to_string(),
        name: Some("Teleport".to_string()),
        move_type: Some("psychic".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "manual",
            json!({ "manualReason": "Switching Pokemon is not supported in DSL" }),
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

    let state = make_state(
        vec![make_creature("c1", "Alpha", vec!["teleport".to_string()])],
        vec![make_creature("c2", "Beta", vec!["wait".to_string()])],
    );
    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("teleport".to_string()),
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
    let has_pending = next.players[0].team[0]
        .statuses
        .iter()
        .any(|s| s.id == "pending_switch");
    assert!(has_pending);
}
