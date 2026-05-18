use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::effects::{apply_effects, EffectContext};
use engine_rust::core::events::{apply_event, BattleEvent};
use engine_rust::core::state::{
    Action, ActionType, BattleHistory, BattleState, BattleTurn, CreatureState, FieldEffect,
    FieldState, PlayerState, StatStages, Status,
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
            ability: None,
            volatile_data: HashMap::new(),
            ability_data: HashMap::new(),
            move_pp: HashMap::new(),
            attack: 10,
            defense: 10,
            sp_attack: 10,
            sp_defense: 10,
            speed: 10,
            weight_kg: 60.0,
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
        is_sound: false,
        last_damage: None,
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
            ability: None,
            volatile_data: HashMap::new(),
            ability_data: HashMap::new(),
            move_pp: HashMap::new(),
            attack: 10,
            defense: 10,
            sp_attack: 10,
            sp_defense: 10,
            speed: 10,
            weight_kg: 60.0,
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
        is_sound: false,
        last_damage: None,
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
