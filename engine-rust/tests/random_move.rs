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

fn make_creature(id: &str, name: &str, moves: Vec<String>, speed: i32) -> CreatureState {
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
        speed,
    }
}

#[test]
fn random_move_uses_self_moves_and_consumes_pp() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "metronome".to_string(),
        name: Some("Metronome".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(1),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("random_move", json!({ "pool": "self_moves" }))],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "tackle".to_string(),
        name: Some("Tackle".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(1),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": 0.5 }))],
        tags: Vec::new(),
        crit_rate: None,
    });

    let p1 = PlayerState {
        id: "p1".to_string(),
        name: "P1".to_string(),
        team: vec![make_creature(
            "c1",
            "Alpha",
            vec!["tackle".to_string()],
            100,
        )],
        active_slot: 0,
        last_fainted_ability: None,
    };
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "P2".to_string(),
        team: vec![make_creature("c2", "Beta", vec!["tackle".to_string()], 10)],
        active_slot: 0,
        last_fainted_ability: None,
    };

    let state = BattleState {
        players: vec![p1, p2],
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        turn: 0,
        log: Vec::new(),
        history: None,
    };

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("metronome".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: Some("tackle".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert_eq!(next.players[1].team[0].hp, 50);
    let p1_after = &next.players[0].team[0];
    assert_eq!(p1_after.move_pp.get("metronome").copied(), Some(0));
    assert_eq!(p1_after.move_pp.get("tackle").copied(), Some(0));
}
