use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::effects::{apply_effects, apply_events, EffectContext};
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

fn make_creature(id: &str, name: &str, moves: Vec<String>) -> CreatureState {
    CreatureState {
        kamishinka: None,
        id: id.to_string(),
        species_id: "testmon".to_string(),
        name: name.to_string(),
        level: 50,
        types: vec!["normal".to_string()],
        moves,
        ability: Some("none".to_string()),
        item: None,
        consumed_item: None,

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
                fainted_count: 0,
                kamishinka_used: false,
                id: "p1".to_string(),
                name: "P1".to_string(),
                team: vec![p1],
                active_slot: 0,
                last_fainted_ability: None,
            },
            PlayerState {
                fainted_count: 0,
                kamishinka_used: false,
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
        ignore_ability: false,
        is_sound: false,
        last_damage: None,
        move_blocked_by_protect: false,
        switch_slot: None,
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
            kamishinka_form: None,
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("tap".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            kamishinka_form: None,
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
            kamishinka_form: None,
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("tap".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            kamishinka_form: None,
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

fn use_substitute(state: &BattleState) -> BattleState {
    let engine = BattleEngine::new(MoveDatabase::load_default().unwrap(), TypeChart::new());
    engine.step_battle(
        state,
        &[Action {
            player_id: "p1".into(),
            action_type: ActionType::Move,
            move_id: Some("substitute".into()),
            target_id: Some("p2".into()),
            slot: None,
            priority: None,
            kamishinka_form: None,
        }],
        &mut || 0.5,
        BattleOptions::default(),
    )
}

#[test]
fn repeated_substitute_fails_without_cost_or_restoring_its_hp() {
    let mut s = make_state(
        make_creature("c1", "Alpha", vec!["substitute".into()]),
        make_creature("c2", "Beta", vec![]),
    );
    s = use_substitute(&s);
    assert_eq!(s.players[0].team[0].hp, 75);
    s.players[0].team[0]
        .statuses
        .iter_mut()
        .find(|s| s.id == "substitute")
        .unwrap()
        .data
        .insert("hp".into(), json!(7));
    s = use_substitute(&s);
    let c = &s.players[0].team[0];
    assert_eq!(c.hp, 75);
    assert_eq!(
        c.statuses.iter().filter(|s| s.id == "substitute").count(),
        1
    );
    assert_eq!(
        c.statuses
            .iter()
            .find(|s| s.id == "substitute")
            .unwrap()
            .data["hp"],
        json!(7)
    );
    assert_eq!(c.move_pp["substitute"], 8); // Failed use still consumes PP.
}

#[test]
fn substitute_requires_more_hp_than_the_cost() {
    for (hp, max_hp, succeeds) in [
        (24, 100, false),
        (25, 100, false),
        (26, 100, true),
        (25, 101, false),
        (26, 101, true),
        (1, 1, false),
    ] {
        let mut s = make_state(
            make_creature("c1", "Alpha", vec!["substitute".into()]),
            make_creature("c2", "Beta", vec![]),
        );
        s.players[0].team[0].hp = hp;
        s.players[0].team[0].max_hp = max_hp;
        let s = use_substitute(&s);
        let c = &s.players[0].team[0];
        assert_eq!(c.statuses.iter().any(|s| s.id == "substitute"), succeeds);
        assert_eq!(c.hp, if succeeds { hp - max_hp / 4 } else { hp });
    }
}

#[test]
fn substitute_can_be_created_again_after_the_previous_one_breaks() {
    use engine_rust::core::events::{apply_event, meta_with_move_source, BattleEvent};
    let s = make_state(
        make_creature("c1", "Alpha", vec!["substitute".into()]),
        make_creature("c2", "Beta", vec![]),
    );
    let s = use_substitute(&s);
    let s = apply_event(
        &s,
        &BattleEvent::Damage {
            target_id: "p1".into(),
            amount: 100,
            meta: meta_with_move_source(Some("tackle"), Some("p2")),
        },
    );
    assert_eq!(s.players[0].team[0].hp, 75);
    assert!(!s.players[0].team[0]
        .statuses
        .iter()
        .any(|s| s.id == "substitute"));
    let s = use_substitute(&s);
    assert_eq!(s.players[0].team[0].hp, 50);
    assert_eq!(
        s.players[0].team[0]
            .statuses
            .iter()
            .filter(|s| s.id == "substitute")
            .count(),
        1
    );
}

#[test]
fn direct_status_requests_cannot_stack_substitutes_even_with_stack_true() {
    use engine_rust::core::events::{apply_event, BattleEvent};
    let mut s = make_state(
        make_creature("c1", "Alpha", vec![]),
        make_creature("c2", "Beta", vec![]),
    );
    for stack in [false, true, true] {
        s = apply_event(
            &s,
            &BattleEvent::ApplyStatus {
                target_id: "p1".into(),
                status_id: "substitute".into(),
                duration: None,
                stack,
                data: HashMap::from([("hp".into(), json!(7))]),
                meta: Map::new(),
            },
        );
    }
    assert_eq!(
        s.players[0].team[0]
            .statuses
            .iter()
            .filter(|s| s.id == "substitute")
            .count(),
        1
    );
}
