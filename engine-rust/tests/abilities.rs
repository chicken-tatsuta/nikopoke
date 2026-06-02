use engine_rust::core::abilities::{
    run_ability_check_hook, run_ability_value_hook, AbilityCheckContext, AbilityValueContext,
};
use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::events::{apply_event, BattleEvent};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages,
    Status,
};
use engine_rust::core::statuses::{run_status_hooks, StatusHookContext};
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

fn damage_move(id: &str, move_type: &str, power: i32) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some(move_type.to_string()),
        category: Some("special".to_string()),
        pp: Some(10),
        power: Some(power),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": power, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
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
fn steely_spirit_boosts_steel_damage() {
    let state = make_state(
        make_creature("c1", "Alpha", Some("steely_spirit"), vec![]),
        make_creature("c2", "Beta", None, vec![]),
    );
    let move_data = MoveData {
        id: "steel_hit".to_string(),
        name: Some("Steel Hit".to_string()),
        move_type: Some("steel".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(80),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 80, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    };

    let value = run_ability_value_hook(
        &state,
        "p1",
        "onModifyPower",
        80.0,
        AbilityValueContext {
            move_data: Some(&move_data),
            category: Some("physical"),
            target: None,
            weather: None,
            turn: 1,
            stages: None,
        },
    );

    assert_eq!(value, 120.0);
}

#[test]
fn steely_spirit_boosts_opposing_steel_damage() {
    let state = make_state(
        make_creature("c1", "Alpha", None, vec![]),
        make_creature("c2", "Beta", Some("steely_spirit"), vec![]),
    );
    let move_data = MoveData {
        id: "steel_hit".to_string(),
        name: Some("Steel Hit".to_string()),
        move_type: Some("steel".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(80),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 80, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate: None,
    };

    let value = run_ability_value_hook(
        &state,
        "p1",
        "onModifyPower",
        80.0,
        AbilityValueContext {
            move_data: Some(&move_data),
            category: Some("physical"),
            target: None,
            weather: None,
            turn: 1,
            stages: None,
        },
    );

    assert_eq!(value, 120.0);
}

#[test]
fn boost_grass_increases_matching_attack_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(damage_move("leaf_hit", "grass", 80));
    move_db.insert(damage_move("wait", "normal", 0));

    let mut boosted_attacker = make_creature("c1", "Alpha", None, vec!["leaf_hit".to_string()]);
    boosted_attacker.item = Some("boost_grass".to_string());
    let mut normal_attacker = boosted_attacker.clone();
    normal_attacker.item = None;
    let mut target = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
    target.types = vec!["water".to_string()];

    let actions = vec![
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
    ];

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut boosted_rng = || 0.0;
    let boosted = engine.step_battle(
        &make_state(boosted_attacker, target.clone()),
        &actions,
        &mut boosted_rng,
        BattleOptions::default(),
    );
    let mut normal_rng = || 0.0;
    let normal = engine.step_battle(
        &make_state(normal_attacker, target),
        &actions,
        &mut normal_rng,
        BattleOptions::default(),
    );

    assert!(boosted.players[1].team[0].hp < normal.players[1].team[0].hp);
}

#[test]
fn boost_grass_does_not_increase_fire_attack_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(damage_move("fire_hit", "fire", 80));
    move_db.insert(damage_move("wait", "normal", 0));

    let mut boosted_attacker = make_creature("c1", "Alpha", None, vec!["fire_hit".to_string()]);
    boosted_attacker.item = Some("boost_grass".to_string());
    let mut normal_attacker = boosted_attacker.clone();
    normal_attacker.item = None;
    let target = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("fire_hit".to_string()),
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

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut boosted_rng = || 0.0;
    let boosted = engine.step_battle(
        &make_state(boosted_attacker, target.clone()),
        &actions,
        &mut boosted_rng,
        BattleOptions::default(),
    );
    let mut normal_rng = || 0.0;
    let normal = engine.step_battle(
        &make_state(normal_attacker, target),
        &actions,
        &mut normal_rng,
        BattleOptions::default(),
    );

    assert_eq!(boosted.players[1].team[0].hp, normal.players[1].team[0].hp);
}

#[test]
fn resist_grass_halves_super_effective_damage_and_is_consumed() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(damage_move("leaf_hit", "grass", 80));
    move_db.insert(damage_move("wait", "normal", 0));

    let attacker = make_creature("c1", "Alpha", None, vec!["leaf_hit".to_string()]);
    let mut resisted_target = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
    resisted_target.types = vec!["water".to_string()];
    resisted_target.item = Some("resist_grass".to_string());
    let mut normal_target = resisted_target.clone();
    normal_target.item = None;
    let actions = vec![
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
    ];

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut resisted_rng = || 0.0;
    let resisted = engine.step_battle(
        &make_state(attacker.clone(), resisted_target),
        &actions,
        &mut resisted_rng,
        BattleOptions::default(),
    );
    let mut normal_rng = || 0.0;
    let normal = engine.step_battle(
        &make_state(attacker, normal_target),
        &actions,
        &mut normal_rng,
        BattleOptions::default(),
    );

    assert!(resisted.players[1].team[0].hp > normal.players[1].team[0].hp);
    assert_eq!(resisted.players[1].team[0].item, None);
    assert_eq!(
        resisted.players[1].team[0].consumed_item.as_deref(),
        Some("resist_grass")
    );
    assert!(resisted
        .log
        .iter()
        .any(|line| line.contains("Betaの レジスト（くさ）で ダメージを半減した！")));
}

#[test]
fn resist_grass_does_not_trigger_when_grass_is_not_super_effective() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(damage_move("leaf_hit", "grass", 80));
    move_db.insert(damage_move("wait", "normal", 0));

    let attacker = make_creature("c1", "Alpha", None, vec!["leaf_hit".to_string()]);
    let mut target = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
    target.item = Some("resist_grass".to_string());
    let actions = vec![
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
    ];

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let next = engine.step_battle(
        &make_state(attacker, target),
        &actions,
        &mut rng,
        BattleOptions::default(),
    );

    assert_eq!(
        next.players[1].team[0].item.as_deref(),
        Some("resist_grass")
    );
    assert_eq!(next.players[1].team[0].consumed_item, None);
    assert!(!next
        .log
        .iter()
        .any(|line| line.contains("レジスト（くさ）")));
}

#[test]
fn resist_grass_only_triggers_once() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(damage_move("leaf_hit", "grass", 80));
    move_db.insert(damage_move("wait", "normal", 0));

    let attacker = make_creature("c1", "Alpha", None, vec!["leaf_hit".to_string()]);
    let mut target = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
    target.types = vec!["water".to_string()];
    target.item = Some("resist_grass".to_string());
    let actions = vec![
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
    ];

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let after_first = engine.step_battle(
        &make_state(attacker, target),
        &actions,
        &mut rng,
        BattleOptions::default(),
    );
    let hp_after_first = after_first.players[1].team[0].hp;
    let after_second =
        engine.step_battle(&after_first, &actions, &mut rng, BattleOptions::default());

    assert!(after_second.players[1].team[0].hp < hp_after_first);
    let resist_logs = after_second
        .log
        .iter()
        .filter(|line| line.contains("レジスト（くさ）"))
        .count();
    assert_eq!(resist_logs, 1);
}

#[test]
fn resist_consumption_persists_after_switching() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(damage_move("leaf_hit", "grass", 80));
    move_db.insert(damage_move("wait", "normal", 0));

    let attacker = make_creature(
        "c1",
        "Alpha",
        None,
        vec!["leaf_hit".to_string(), "wait".to_string()],
    );
    let mut target = make_creature("c2", "Beta", None, vec!["wait".to_string()]);
    target.types = vec!["water".to_string()];
    target.item = Some("resist_grass".to_string());
    let reserve = make_creature("c3", "Gamma", None, vec!["wait".to_string()]);
    let mut state = make_state(attacker, target);
    state.players[1].team.push(reserve);
    let attack_actions = vec![
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
    ];

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let after_first =
        engine.step_battle(&state, &attack_actions, &mut rng, BattleOptions::default());
    let after_switch_out = engine.step_battle(
        &after_first,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Switch,
                move_id: None,
                target_id: None,
                slot: Some(1),
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    let after_switch_in = engine.step_battle(
        &after_switch_out,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Switch,
                move_id: None,
                target_id: None,
                slot: Some(0),
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    let hp_before_second = after_switch_in.players[1].team[0].hp;
    let after_second = engine.step_battle(
        &after_switch_in,
        &attack_actions,
        &mut rng,
        BattleOptions::default(),
    );

    assert_eq!(
        after_switch_out.players[1].team[0].consumed_item.as_deref(),
        Some("resist_grass")
    );
    assert!(after_second.players[1].team[0].hp < hp_before_second);
    let resist_logs = after_second
        .log
        .iter()
        .filter(|line| line.contains("レジスト（くさ）"))
        .count();
    assert_eq!(resist_logs, 1);
}

#[test]
fn inner_focus_blocks_flinch_and_intimidate() {
    let state = make_state(
        make_creature("c1", "Alpha", Some("inner_focus"), vec![]),
        make_creature("c2", "Beta", None, vec![]),
    );

    assert!(run_ability_check_hook(
        &state,
        "p1",
        "onCheckStatusImmunity",
        AbilityCheckContext {
            status_id: Some("flinch"),
            r#type: None,
            target_id: None,
            action: None,
        },
        false,
    ));
    assert!(run_ability_check_hook(
        &state,
        "p1",
        "onImmunity",
        AbilityCheckContext {
            status_id: None,
            r#type: Some("intimidate"),
            target_id: None,
            action: None,
        },
        false,
    ));
}

#[test]
fn disguise_blocks_first_attack_damage_once() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(80),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 80, "accuracy": 1.0 }))],
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
        make_creature("c1", "Alpha", None, vec!["hit".to_string()]),
        make_creature("c2", "Beta", Some("disguise"), vec!["wait".to_string()]),
    );
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

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let after_first = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
    let after_second =
        engine.step_battle(&after_first, &actions, &mut rng, BattleOptions::default());

    assert_eq!(after_first.players[1].team[0].hp, 88);
    let messages: Vec<&str> = after_first.log.iter().map(String::as_str).collect();
    assert!(messages.contains(&"Betaの 特性『ばけのかわ』！"));
    assert!(messages.contains(&"Betaの ばけのかわが はがれた！"));
    assert!(after_first.players[1].team[0]
        .volatile_data
        .get("disguiseUsed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false));
    assert!(after_second.players[1].team[0].hp < 88);
}

#[test]
fn disguise_prevents_drain_recovery_from_blocked_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "draining_hit".to_string(),
        name: Some("Draining Hit".to_string()),
        move_type: Some("grass".to_string()),
        category: Some("special".to_string()),
        pp: Some(10),
        power: Some(80),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![
            effect("damage", json!({ "power": 80, "accuracy": 1.0 })),
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

    let mut attacker = make_creature("c1", "Alpha", None, vec!["draining_hit".to_string()]);
    attacker.hp = 50;
    let state = make_state(
        attacker,
        make_creature("c2", "Beta", Some("disguise"), vec!["wait".to_string()]),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("draining_hit".to_string()),
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

    assert_eq!(next.players[0].team[0].hp, 50);
    assert_eq!(next.players[1].team[0].hp, 88);
}

#[test]
fn disguise_does_not_reset_after_switching_out() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "hit".to_string(),
        name: Some("Hit".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: Some(80),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage", json!({ "power": 80, "accuracy": 1.0 }))],
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
            None,
            vec!["hit".to_string(), "wait".to_string()],
        ),
        make_creature("c2", "Beta", Some("disguise"), vec!["wait".to_string()]),
    );
    state.players[1]
        .team
        .push(make_creature("c3", "Gamma", None, vec!["wait".to_string()]));

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let after_first = engine.step_battle(
        &state,
        &[
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
        ],
        &mut rng,
        BattleOptions::default(),
    );
    let after_switch_out = engine.step_battle(
        &after_first,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Switch,
                move_id: None,
                target_id: None,
                slot: Some(1),
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    let after_switch_in = engine.step_battle(
        &after_switch_out,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("wait".to_string()),
                target_id: Some("p2".to_string()),
                slot: None,
                priority: None,
            },
            Action {
                player_id: "p2".to_string(),
                action_type: ActionType::Switch,
                move_id: None,
                target_id: None,
                slot: Some(0),
                priority: None,
            },
        ],
        &mut rng,
        BattleOptions::default(),
    );
    let after_second_hit = engine.step_battle(
        &after_switch_in,
        &[
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
        ],
        &mut rng,
        BattleOptions::default(),
    );

    assert_eq!(after_first.players[1].team[0].hp, 88);
    assert!(after_switch_out.players[1].team[0]
        .volatile_data
        .get("disguiseUsed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false));
    assert!(after_second_hit.players[1].team[0].hp < 88);
    let disguise_logs = after_second_hit
        .log
        .iter()
        .filter(|line| line.contains("Betaの 特性『ばけのかわ』！"))
        .count();
    assert_eq!(disguise_logs, 1);
}

#[test]
fn rattled_raises_speed_after_bug_dark_or_ghost_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "spook".to_string(),
        name: Some("Spook".to_string()),
        move_type: Some("ghost".to_string()),
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

    let mut defender = make_creature("c2", "Beta", Some("rattled"), vec!["wait".to_string()]);
    defender.types = vec!["psychic".to_string()];
    let state = make_state(
        make_creature("c1", "Alpha", None, vec!["spook".to_string()]),
        defender,
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("spook".to_string()),
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

    assert_eq!(next.players[1].team[0].stages.spe, 1);
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Betaの 特性『びびり』")));
}

#[test]
fn frisk_reveals_opposing_item_on_switch_in() {
    let active = make_creature("c1", "Alpha", Some("frisk"), vec![]);
    let mut target = make_creature("c2", "Beta", None, vec![]);
    target.item = Some("leftovers".to_string());
    let state = make_state(active, target);
    let mut rng = || 0.0;

    let result = engine_rust::core::abilities::run_ability_hooks(
        &state,
        "p1",
        "onSwitchIn",
        engine_rust::core::abilities::AbilityHookContext {
            rng: &mut rng,
            action: None,
            move_data: None,
        },
    );

    assert!(result.events.iter().any(
        |event| matches!(event, BattleEvent::Log { message, .. } if message.contains("leftovers"))
    ));
}

#[test]
fn drizzle_sets_rain_on_switch_in() {
    let active = make_creature("c1", "Reosan", Some("drizzle"), vec![]);
    let state = make_state(active, make_creature("c2", "Beta", None, vec![]));
    let mut rng = || 0.0;

    let result = engine_rust::core::abilities::run_ability_hooks(
        &state,
        "p1",
        "onSwitchIn",
        engine_rust::core::abilities::AbilityHookContext {
            rng: &mut rng,
            action: None,
            move_data: None,
        },
    );
    let next = result.state.expect("drizzle should update state");

    assert!(next
        .field
        .global
        .iter()
        .any(|effect| effect.id == "rain" && effect.remaining_turns == Some(5)));
    assert!(result.events.iter().any(
        |event| matches!(event, BattleEvent::Log { message, .. } if message.contains("雨が 降りはじめた"))
    ));
}

#[test]
fn hospitality_heals_self_on_switch_in() {
    let mut active = make_creature("c1", "Alpha", Some("hospitality"), vec![]);
    active.hp = 40;
    active.max_hp = 100;
    let state = make_state(active, make_creature("c2", "Beta", None, vec![]));
    let mut rng = || 0.0;

    let result = engine_rust::core::abilities::run_ability_hooks(
        &state,
        "p1",
        "onSwitchIn",
        engine_rust::core::abilities::AbilityHookContext {
            rng: &mut rng,
            action: None,
            move_data: None,
        },
    );

    assert!(result.events.iter().any(
        |event| matches!(event, BattleEvent::Damage { target_id, amount, .. } if target_id == "p1" && *amount == -25)
    ));
}

#[test]
fn early_bird_wakes_from_sleep_faster() {
    let mut sleep = Status {
        id: "sleep".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    };
    sleep
        .data
        .insert("elapsed".to_string(), Value::Number(1.into()));
    let mut active = make_creature("c1", "Alpha", Some("early_bird"), vec!["wait".to_string()]);
    active.statuses.push(sleep);
    let state = make_state(active, make_creature("c2", "Beta", None, vec![]));
    let move_data = MoveData {
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
    };
    let type_chart = TypeChart::new();
    let mut rng = || 0.99;

    let result = run_status_hooks(
        &state,
        "p1",
        "onBeforeAction",
        StatusHookContext {
            rng: &mut rng,
            action: None,
            move_data: Some(&move_data),
            type_chart: &type_chart,
        },
    );

    assert!(result.events.iter().any(
        |event| matches!(event, BattleEvent::RemoveStatus { status_id, .. } if status_id == "sleep")
    ));
}

#[test]
fn sniper_boosts_critical_hit_damage() {
    fn damage_with_ability(ability: Option<&str>) -> i32 {
        let mut move_db = MoveDatabase::new();
        move_db.insert(MoveData {
            id: "sure_crit".to_string(),
            name: Some("Sure Crit".to_string()),
            move_type: Some("dark".to_string()),
            category: Some("physical".to_string()),
            pp: Some(10),
            power: Some(60),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: vec![effect("damage", json!({ "power": 60, "accuracy": 1.0 }))],
            tags: Vec::new(),
            crit_rate: Some(3),
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

        let mut attacker = make_creature("c1", "Alpha", ability, vec!["sure_crit".to_string()]);
        attacker.types = vec!["dark".to_string()];
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
                move_id: Some("sure_crit".to_string()),
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
        let mut rng = || 0.5;
        let engine = BattleEngine::new(move_db, TypeChart::new());
        let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
        200 - next.players[1].team[0].hp
    }

    let normal_crit = damage_with_ability(None);
    let sniper_crit = damage_with_ability(Some("sniper"));

    assert!(sniper_crit > normal_crit);
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
fn cotton_down_triggers_on_attack_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "non_contact_hit".to_string(),
        name: Some("Non Contact Hit".to_string()),
        move_type: Some("normal".to_string()),
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
        make_creature("c1", "Alpha", None, vec!["non_contact_hit".to_string()]),
        make_creature("c2", "Beta", Some("cotton_down"), vec!["wait".to_string()]),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("non_contact_hit".to_string()),
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
fn stamina_does_not_trigger_from_self_hp_loss() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "self_cut".to_string(),
        name: Some("Self Cut".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "damage_ratio",
            json!({ "target": "self", "ratioMaxHp": 0.25 }),
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
        make_creature("c1", "Alpha", None, vec!["wait".to_string()]),
        make_creature("c2", "Beta", Some("stamina"), vec!["self_cut".to_string()]),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("wait".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: Some("self_cut".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());

    assert_eq!(next.players[1].team[0].stages.def, 0);
    assert!(!next
        .log
        .iter()
        .any(|line| line.contains("Betaの 特性『じきゅうりょく』")));
}

#[test]
fn rage_fist_hit_counter_does_not_increase_from_self_hp_loss() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "self_cut".to_string(),
        name: Some("Self Cut".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "damage_ratio",
            json!({ "target": "self", "ratioMaxHp": 0.25 }),
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
        make_creature("c1", "Alpha", None, vec!["wait".to_string()]),
        make_creature("c2", "Beta", None, vec!["self_cut".to_string()]),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("wait".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: Some("self_cut".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
    ];

    let mut rng = || 0.0;
    let engine = BattleEngine::new(move_db, TypeChart::new());
    let next = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());

    assert!(next.players[1].team[0]
        .volatile_data
        .get("moveHitsTaken")
        .is_none());
}

#[test]
fn static_can_paralyze_contact_attacker() {
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
        make_creature("c2", "Touma", Some("static"), vec!["wait".to_string()]),
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

    assert!(next.players[0].team[0]
        .statuses
        .iter()
        .any(|status| status.id == "paralysis"));
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Toumaの 特性『せいでんき』")));
}

#[test]
fn static_does_not_paralyze_non_contact_attacker() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "ranged_hit".to_string(),
        name: Some("Ranged Hit".to_string()),
        move_type: Some("normal".to_string()),
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
        make_creature("c1", "Alpha", None, vec!["ranged_hit".to_string()]),
        make_creature("c2", "Touma", Some("static"), vec!["wait".to_string()]),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("ranged_hit".to_string()),
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

    assert!(!next.players[0].team[0]
        .statuses
        .iter()
        .any(|status| status.id == "paralysis"));
    assert!(!next
        .log
        .iter()
        .any(|line| line.contains("Toumaの 特性『せいでんき』")));
}

#[test]
fn popping_habanero_burns_attacker_after_attack_damage() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "ranged_hit".to_string(),
        name: Some("Ranged Hit".to_string()),
        move_type: Some("normal".to_string()),
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
        make_creature("c1", "Alpha", None, vec!["ranged_hit".to_string()]),
        make_creature(
            "c2",
            "Tomoki",
            Some("popping_habanero"),
            vec!["wait".to_string()],
        ),
    );
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("ranged_hit".to_string()),
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

    assert!(next.players[0].team[0]
        .statuses
        .iter()
        .any(|status| status.id == "burn"));
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Tomokiの 特性『とびだすハバネロ』")));
}

#[test]
fn cotton_down_does_not_trigger_from_drain_recovery() {
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
