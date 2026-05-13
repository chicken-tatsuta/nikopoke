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

fn make_creature(id: &str, name: &str, types: Vec<String>, moves: Vec<String>) -> CreatureState {
    CreatureState {
        id: id.to_string(),
        species_id: "testmon".to_string(),
        name: name.to_string(),
        level: 50,
        types,
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
fn bypass_protect_allows_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![
            effect("damage", json!({ "power": 40, "accuracy": 1.0 })),
            effect("bypass_protect", json!({})),
        ],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "poke".to_string(),
        name: Some("Poke".to_string()),
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

    let mut target = make_creature(
        "c2",
        "Beta",
        vec!["normal".to_string()],
        vec!["poke".to_string()],
    );
    target.statuses.push(Status {
        id: "protect".to_string(),
        remaining_turns: Some(1),
        data: HashMap::new(),
    });
    let state = make_state(
        make_creature(
            "c1",
            "Alpha",
            vec!["normal".to_string()],
            vec!["hit".to_string()],
        ),
        target,
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
            move_id: Some("poke".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert!(next.players[1].team[0].hp < 100);
}

#[test]
fn protect_blocks_damage_without_bypass() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
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
        id: "poke".to_string(),
        name: Some("Poke".to_string()),
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

    let mut target = make_creature(
        "c2",
        "Beta",
        vec!["normal".to_string()],
        vec!["poke".to_string()],
    );
    target.statuses.push(Status {
        id: "protect".to_string(),
        remaining_turns: Some(1),
        data: HashMap::new(),
    });
    let state = make_state(
        make_creature(
            "c1",
            "Alpha",
            vec!["normal".to_string()],
            vec!["hit".to_string()],
        ),
        target,
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
            move_id: Some("poke".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert_eq!(next.players[1].team[0].hp, 100);
}

#[test]
fn ignore_immunity_allows_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![
            effect("damage", json!({ "power": 40, "accuracy": 1.0 })),
            effect("ignore_immunity", json!({})),
        ],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "poke".to_string(),
        name: Some("Poke".to_string()),
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
        make_creature(
            "c1",
            "Alpha",
            vec!["normal".to_string()],
            vec!["hit".to_string()],
        ),
        make_creature(
            "c2",
            "Ghosty",
            vec!["ghost".to_string()],
            vec!["poke".to_string()],
        ),
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
            move_id: Some("poke".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert!(next.players[1].team[0].hp < 100);
}

#[test]
fn immunity_blocks_damage_without_ignore() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
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
        id: "poke".to_string(),
        name: Some("Poke".to_string()),
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
        make_creature(
            "c1",
            "Alpha",
            vec!["normal".to_string()],
            vec!["hit".to_string()],
        ),
        make_creature(
            "c2",
            "Ghosty",
            vec!["ghost".to_string()],
            vec!["poke".to_string()],
        ),
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
            move_id: Some("poke".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert_eq!(next.players[1].team[0].hp, 100);
}

#[test]
fn substitute_blocks_damage_without_bypass() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
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
        id: "poke".to_string(),
        name: Some("Poke".to_string()),
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

    let mut target = make_creature(
        "c2",
        "Beta",
        vec!["normal".to_string()],
        vec!["poke".to_string()],
    );
    target.statuses.push(Status {
        id: "substitute".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });
    let state = make_state(
        make_creature(
            "c1",
            "Alpha",
            vec!["normal".to_string()],
            vec!["hit".to_string()],
        ),
        target,
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
            move_id: Some("poke".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert_eq!(next.players[1].team[0].hp, 100);
}

#[test]
fn bypass_substitute_allows_damage_by_tag() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        tags: vec!["bypass_substitute".to_string()],
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "poke".to_string(),
        name: Some("Poke".to_string()),
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

    let mut target = make_creature(
        "c2",
        "Beta",
        vec!["normal".to_string()],
        vec!["poke".to_string()],
    );
    target.statuses.push(Status {
        id: "substitute".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });
    let state = make_state(
        make_creature(
            "c1",
            "Alpha",
            vec!["normal".to_string()],
            vec!["hit".to_string()],
        ),
        target,
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
            move_id: Some("poke".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert!(next.players[1].team[0].hp < 100);
}

#[test]
fn ignore_substitute_allows_damage_by_effect() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![
            effect("damage", json!({ "power": 40, "accuracy": 1.0 })),
            effect("ignore_substitute", json!({})),
        ],
        tags: Vec::new(),
        crit_rate: None,
    });
    move_db.insert(MoveData {
        id: "poke".to_string(),
        name: Some("Poke".to_string()),
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

    let mut target = make_creature(
        "c2",
        "Beta",
        vec!["normal".to_string()],
        vec!["poke".to_string()],
    );
    target.statuses.push(Status {
        id: "substitute".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });
    let state = make_state(
        make_creature(
            "c1",
            "Alpha",
            vec!["normal".to_string()],
            vec!["hit".to_string()],
        ),
        target,
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
            move_id: Some("poke".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    assert!(next.players[1].team[0].hp < 100);
}
