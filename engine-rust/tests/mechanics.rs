use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::effects::{apply_effects, EffectContext};
use engine_rust::core::events::{apply_event, BattleEvent};
use engine_rust::core::state::{
    Action, ActionType, BattleHistory, BattleState, BattleTurn, CreatureState, EVStats,
    FieldEffect, FieldState, PlayerState, StatStages, Status,
};
use engine_rust::core::statuses::{run_status_hooks, StatusHookContext};
use engine_rust::data::moves::{Effect, MoveData, MoveDatabase};
use engine_rust::data::type_chart::TypeChart;
use serde_json::{Map, Value};
use std::collections::HashMap;

fn create_test_state() -> BattleState {
    let p1 = PlayerState {
        id: "p1".to_string(),
        name: "Player 1".to_string(),
        team: vec![CreatureState {
            id: "c1".to_string(),
            species_id: "test_mon".to_string(),
            name: "Mon1".to_string(),
            level: 50,
            types: vec!["normal".to_string()],
            max_hp: 100,
            hp: 100,
            moves: vec!["tackle".to_string()],
            stages: StatStages::default(),
            statuses: Vec::new(),
            item: None,

            evs: EVStats::default(),
            ability: None,
            volatile_data: HashMap::new(),
            ability_data: HashMap::new(),
            move_pp: HashMap::new(),
            attack: 10,
            defense: 10,
            sp_attack: 10,
            sp_defense: 10,
            speed: 10,

            weight_kg: 50.0,
        }],
        active_slot: 0,
        last_fainted_ability: None,
    };
    BattleState {
        players: vec![p1],
        turn: 1,
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        log: Vec::new(),
        history: Some(BattleHistory { turns: Vec::new() }),
    }
}

#[test]
fn test_lock_move_force_last_move() {
    let mut state = create_test_state();

    // Add history
    let action = Action {
        player_id: "p1".to_string(),
        action_type: ActionType::Move,
        move_id: Some("ember".to_string()),
        target_id: None,
        slot: None,
        priority: None,
    };
    state.history.as_mut().unwrap().turns.push(BattleTurn {
        turn: 0,
        actions: vec![action.clone()],
        log: vec![],
        rng: vec![],
    });

    // Add lock_move status
    let mut data = HashMap::new();
    data.insert(
        "mode".to_string(),
        Value::String("force_last_move".to_string()),
    );
    state.players[0].team[0].statuses.push(Status {
        id: "lock_move".to_string(),
        remaining_turns: Some(3),
        data,
    });

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let ctx = StatusHookContext {
        rng: &mut rng,
        action: Some(&Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("tackle".to_string()),
            ..action.clone()
        }),
        move_data: None,
        type_chart: &type_chart,
    };

    let result = run_status_hooks(&state, "p1", "onBeforeAction", ctx);

    assert!(result.override_action.is_some());
    assert_eq!(
        result.override_action.unwrap().move_id,
        Some("ember".to_string())
    );
}

#[test]
fn test_apply_status_existing() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "burn".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });

    let event = BattleEvent::ApplyStatus {
        target_id: "p1".to_string(),
        status_id: "burn".to_string(),
        duration: None,
        stack: false,
        data: HashMap::new(),
        meta: Map::new(),
    };

    let next_state = apply_event(&state, &event);

    // Should verify log says already has status
    assert!(
        next_state.log.last().unwrap().contains("already has burn")
            || next_state.log.last().unwrap().contains("すでに burn状態だ")
    );
    // Status count should still be 1
    assert_eq!(next_state.players[0].team[0].statuses.len(), 1);
}

#[test]
fn non_major_status_can_coexist_with_major_status() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "taunt".to_string(),
        remaining_turns: Some(3),
        data: HashMap::new(),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "burn".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    let statuses = &next_state.players[0].team[0].statuses;
    assert!(statuses.iter().any(|status| status.id == "taunt"));
    assert!(statuses.iter().any(|status| status.id == "burn"));
}

#[test]
fn confusion_can_coexist_with_major_status() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "burn".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "confusion".to_string(),
            duration: Some(3),
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    let statuses = &next_state.players[0].team[0].statuses;
    assert!(statuses.iter().any(|status| status.id == "burn"));
    assert!(statuses.iter().any(|status| status.id == "confusion"));
}

#[test]
fn duplicate_check_only_applies_to_major_statuses() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "taunt".to_string(),
        remaining_turns: Some(3),
        data: HashMap::new(),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "taunt".to_string(),
            duration: Some(3),
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    assert_eq!(
        next_state.players[0].team[0]
            .statuses
            .iter()
            .filter(|status| status.id == "taunt")
            .count(),
        2
    );
}

#[test]
fn major_statuses_remain_mutually_exclusive() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "paralysis".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "burn".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    let statuses = &next_state.players[0].team[0].statuses;
    assert!(statuses.iter().any(|status| status.id == "paralysis"));
    assert!(!statuses.iter().any(|status| status.id == "burn"));
}

#[test]
fn yawn_does_not_apply_to_target_with_major_status() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "sleep".to_string(),
        remaining_turns: Some(2),
        data: HashMap::new(),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "yawn".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    let statuses = &next_state.players[0].team[0].statuses;
    assert!(statuses.iter().any(|status| status.id == "sleep"));
    assert!(!statuses.iter().any(|status| status.id == "yawn"));
}

#[test]
fn major_status_replaces_pending_yawn() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "yawn".to_string(),
        remaining_turns: None,
        data: HashMap::from([("turns".to_string(), Value::Number(1.into()))]),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "burn".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    let statuses = &next_state.players[0].team[0].statuses;
    assert!(statuses.iter().any(|status| status.id == "burn"));
    assert!(!statuses.iter().any(|status| status.id == "yawn"));
}

#[test]
fn duplicate_yawn_is_not_stacked() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "yawn".to_string(),
        remaining_turns: None,
        data: HashMap::from([("turns".to_string(), Value::Number(1.into()))]),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "yawn".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    assert_eq!(
        next_state.players[0].team[0]
            .statuses
            .iter()
            .filter(|status| status.id == "yawn")
            .count(),
        1
    );
}

#[test]
fn duplicate_minimize_is_not_stacked() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "minimize".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });

    let next_state = apply_event(
        &state,
        &BattleEvent::ApplyStatus {
            target_id: "p1".to_string(),
            status_id: "minimize".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );

    assert_eq!(
        next_state.players[0].team[0]
            .statuses
            .iter()
            .filter(|status| status.id == "minimize")
            .count(),
        1
    );
}

#[test]
fn test_replace_status_missing_from() {
    let state = create_test_state();
    // No status initially

    let event = BattleEvent::ReplaceStatus {
        target_id: "p1".to_string(),
        from: "sleep".to_string(),
        to: "burn".to_string(),
        duration: None,
        data: HashMap::new(),
        meta: Map::new(),
    };

    let next_state = apply_event(&state, &event);

    // Should have no status (did not add burn)
    assert_eq!(next_state.players[0].team[0].statuses.len(), 0);
}

#[test]
fn test_protect_event_transform() {
    let mut state = create_test_state();
    state.players[0].team[0].statuses.push(Status {
        id: "protect".to_string(),
        remaining_turns: Some(1),
        data: HashMap::new(),
    });

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let ctx = StatusHookContext {
        rng: &mut rng,
        action: None,
        move_data: None,
        type_chart: &type_chart,
    };

    let result = run_status_hooks(&state, "p1", "onEventTransform", ctx);

    // Find transform for damage
    let transform = result
        .event_transforms
        .iter()
        .find(|t| t.from.as_deref() == Some("damage"))
        .expect("Should have damage transform");

    assert_eq!(transform.except_source_id.as_deref(), Some("p1"));
}

#[test]
fn mist_filters_only_stage_drops_from_shell_smash() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "shell_smash".to_string(),
        name: Some("からをやぶる".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(15),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![Effect {
            effect_type: "modify_stage".to_string(),
            data: serde_json::json!({
                "target": "self",
                "stages": {
                    "def": -1,
                    "spd": -1,
                    "atk": 2,
                    "spa": 2,
                    "spe": 2
                }
            })
            .as_object()
            .cloned()
            .unwrap(),
        }],
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
        steps: Vec::new(),
        tags: Vec::new(),
        crit_rate: None,
    });

    let mut state = create_test_state();
    state.players[0].team[0].moves = vec!["shell_smash".to_string()];
    state.players.push(PlayerState {
        id: "p2".to_string(),
        name: "Player 2".to_string(),
        team: vec![CreatureState {
            id: "c2".to_string(),
            species_id: "test_mon_2".to_string(),
            name: "Mon2".to_string(),
            level: 50,
            types: vec!["normal".to_string()],
            max_hp: 100,
            hp: 100,
            moves: vec!["wait".to_string()],
            stages: StatStages::default(),
            statuses: Vec::new(),
            item: None,
            evs: EVStats::default(),
            ability: None,
            volatile_data: HashMap::new(),
            ability_data: HashMap::new(),
            move_pp: HashMap::new(),
            attack: 10,
            defense: 10,
            sp_attack: 10,
            sp_defense: 10,
            speed: 5,
            weight_kg: 50.0,
        }],
        active_slot: 0,
        last_fainted_ability: None,
    });
    state.field.sides.insert(
        "p1".to_string(),
        vec![FieldEffect {
            id: "mist".to_string(),
            remaining_turns: Some(5),
            data: HashMap::new(),
        }],
    );

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let mut rng = || 0.99;
    let next = engine.step_battle(
        &state,
        &[
            Action {
                player_id: "p1".to_string(),
                action_type: ActionType::Move,
                move_id: Some("shell_smash".to_string()),
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

    let stages = &next.players[0].team[0].stages;
    assert_eq!(stages.atk, 2);
    assert_eq!(stages.spa, 2);
    assert_eq!(stages.spe, 2);
    assert_eq!(stages.def, 0);
    assert_eq!(stages.spd, 0);
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("しろいきりが 能力下降を 防いだ")));
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Mon1の こうげきが ぐーんと 上がった")));
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Mon1の とくこうが ぐーんと 上がった")));
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Mon1の すばやさが ぐーんと 上がった")));
}

#[test]
fn test_protect_reset_on_failure() {
    let mut state = create_test_state();
    // Set protect success count to something high so it fails (chance 0.5^count)
    // count=1 -> 0.5 chance. count=2 -> 0.25 chance.
    // If rng=0.9, > 0.5, fails.
    state.players[0].team[0]
        .volatile_data
        .insert("protectSuccessCount".to_string(), Value::Number(1.into()));

    let mut rng = || 0.9; // Fail
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p1".to_string(),
        move_data: None,
        rng: &mut rng,
        turn: 1,
        type_chart: &type_chart,
        bypass_protect: false,
        ignore_immunity: false,
        bypass_substitute: false,
        ignore_substitute: false,
        ignore_ability: false,
        is_sound: false,
        last_damage: None,
        switch_slot: None,
    };

    let effect = Effect {
        effect_type: "protect".to_string(),
        data: Map::new(),
    };

    let events = apply_effects(&state, &[effect], &mut ctx);

    // Should find SetVolatile protectSuccessCount = 0
    let reset_event = events.iter().find(|e| match e {
        BattleEvent::SetVolatile { key, value, .. } => {
            key == "protectSuccessCount" && value == &Value::Number(0.into())
        }
        _ => false,
    });

    assert!(reset_event.is_some(), "Should contain reset event");
}

#[test]
fn test_parental_bond() {
    let mut state = create_test_state();
    state.players[0].team[0].ability = Some("parental_bond".to_string());

    // Add a dummy target player
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "Player 2".to_string(),
        team: vec![CreatureState {
            id: "c2".to_string(),
            species_id: "test_mon_2".to_string(),
            name: "Mon2".to_string(),
            level: 50,
            types: vec!["normal".to_string()],
            max_hp: 100,
            hp: 100,
            moves: vec![],
            stages: StatStages::default(),
            statuses: Vec::new(),
            item: None,

            evs: EVStats::default(),
            ability: None,
            volatile_data: HashMap::new(),
            ability_data: HashMap::new(),
            move_pp: HashMap::new(),
            attack: 10,
            defense: 10,
            sp_attack: 10,
            sp_defense: 10,
            speed: 10,

            weight_kg: 50.0,
        }],
        active_slot: 0,
        last_fainted_ability: None,
    };
    state.players.push(p2);

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: None,
        rng: &mut rng,
        turn: 1,
        type_chart: &type_chart,
        bypass_protect: false,
        ignore_immunity: false,
        bypass_substitute: false,
        ignore_substitute: false,
        ignore_ability: false,
        is_sound: false,
        last_damage: None,
        switch_slot: None,
    };

    let mut data = Map::new();
    data.insert("power".to_string(), Value::Number(40.into()));
    let effect = Effect {
        effect_type: "damage".to_string(),
        data,
    };

    let events = apply_effects(&state, &[effect], &mut ctx);

    // Should have 2 damage events
    let damage_events: Vec<&BattleEvent> = events
        .iter()
        .filter(|e| matches!(e, BattleEvent::Damage { .. }))
        .collect();
    assert_eq!(damage_events.len(), 2);

    // Second one should have parentalBond meta
    if let BattleEvent::Damage { meta, .. } = damage_events[1] {
        assert_eq!(meta.get("parentalBond"), Some(&Value::Bool(true)));
    } else {
        panic!("Second event is not damage");
    }
}
