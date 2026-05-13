use engine_rust::core::effects::{apply_effects, apply_events, EffectContext};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages, Status,
};
use engine_rust::core::statuses::{run_status_hooks, StatusHookContext};
use engine_rust::data::moves::Effect;
use engine_rust::data::type_chart::TypeChart;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

fn make_creature(id: &str, name: &str) -> CreatureState {
    CreatureState {
        id: id.to_string(),
        species_id: "testmon".to_string(),
        name: name.to_string(),
        level: 50,
        types: vec!["normal".to_string()],
        moves: vec!["tackle".to_string()],
        ability: Some("none".to_string()),
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

fn make_state() -> BattleState {
    let p1 = PlayerState {
        id: "p1".to_string(),
        name: "P1".to_string(),
        team: vec![make_creature("c1", "Alpha")],
        active_slot: 0,
        last_fainted_ability: None,
    };
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "P2".to_string(),
        team: vec![make_creature("c2", "Beta")],
        active_slot: 0,
        last_fainted_ability: None,
    };
    BattleState {
        players: vec![p1, p2],
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        turn: 0,
        log: Vec::new(),
        history: None,
    }
}

fn effect(effect_type: &str, data: Value) -> Effect {
    let map: Map<String, Value> = data.as_object().cloned().unwrap_or_default();
    Effect {
        effect_type: effect_type.to_string(),
        data: map,
    }
}

#[test]
fn modify_damage_scales_last_damage_event() {
    let state = make_state();
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
        switch_slot: None,
    };

    let effects = vec![
        effect("damage_ratio", json!({ "ratioMaxHp": 0.25 })),
        effect("modify_damage", json!({ "multiplier": 2.0 })),
    ];
    let events = apply_effects(&state, &effects, &mut ctx);
    let amount = events.iter().find_map(|event| match event {
        engine_rust::core::events::BattleEvent::Damage { amount, .. } => Some(*amount),
        _ => None,
    });
    assert_eq!(amount, Some(50));
}

#[test]
fn crit_scales_last_damage_event() {
    let state = make_state();
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
        switch_slot: None,
    };

    let effects = vec![
        effect("damage_ratio", json!({ "ratioMaxHp": 0.2 })),
        effect("crit", json!({ "mult": 2.0 })),
    ];
    let events = apply_effects(&state, &effects, &mut ctx);
    let amount = events.iter().find_map(|event| match event {
        engine_rust::core::events::BattleEvent::Damage { amount, .. } => Some(*amount),
        _ => None,
    });
    assert_eq!(amount, Some(40));
}

#[test]
fn cure_all_status_clears_statuses() {
    let mut state = make_state();
    if let Some(active) = state.players[1].team.get_mut(0) {
        active.statuses.push(Status {
            id: "burn".to_string(),
            remaining_turns: None,
            data: HashMap::new(),
        });
        active.statuses.push(Status {
            id: "poison".to_string(),
            remaining_turns: None,
            data: HashMap::new(),
        });
    }

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
        switch_slot: None,
    };

    let effects = vec![effect("cure_all_status", json!({ "target": "target" }))];
    let events = apply_effects(&state, &effects, &mut ctx);
    let next = apply_events(&state, &events);
    let statuses = &next.players[1].team[0].statuses;
    assert!(statuses.is_empty());
}

#[test]
fn lock_move_forces_specific_move() {
    let mut state = make_state();
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
        switch_slot: None,
    };

    let effects = vec![effect(
        "lock_move",
        json!({ "target": "target", "data": { "mode": "force_specific", "moveId": "tackle" } }),
    )];
    let events = apply_effects(&state, &effects, &mut ctx);
    state = apply_events(&state, &events);

    let action = Action {
        player_id: "p2".to_string(),
        action_type: ActionType::Move,
        move_id: Some("growl".to_string()),
        target_id: None,
        slot: None,
        priority: None,
    };
    let result = run_status_hooks(
        &state,
        "p2",
        "onBeforeAction",
        StatusHookContext {
            rng: &mut rng,
            action: Some(&action),
            move_data: None,
            type_chart: &type_chart,
        },
    );
    let override_action = result.override_action.expect("override action");
    assert_eq!(override_action.move_id.as_deref(), Some("tackle"));
}

#[test]
fn self_switch_marks_pending_switch() {
    let mut state = make_state();
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
        switch_slot: None,
    };

    let effects = vec![effect("self_switch", json!({}))];
    let events = apply_effects(&state, &effects, &mut ctx);
    state = apply_events(&state, &events);
    let statuses = &state.players[0].team[0].statuses;
    assert!(statuses.iter().any(|s| s.id == "pending_switch"));
}

#[test]
fn force_switch_randomly_switches_target() {
    // Create state with target having 2 Pokémon
    let p1 = PlayerState {
        id: "p1".to_string(),
        name: "P1".to_string(),
        team: vec![make_creature("c1", "Alpha")],
        active_slot: 0,
        last_fainted_ability: None,
    };
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "P2".to_string(),
        team: vec![make_creature("c2", "Beta"), make_creature("c3", "Gamma")],
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
        switch_slot: None,
    };

    let effects = vec![effect("force_switch", json!({ "target": "target" }))];
    let events = apply_effects(&state, &effects, &mut ctx);

    // Should emit Switch event directly (not pending_switch)
    let switch_event = events
        .iter()
        .find(|e| matches!(e, engine_rust::core::events::BattleEvent::Switch { .. }));
    assert!(
        switch_event.is_some(),
        "Expected Switch event to be emitted"
    );

    // Apply and check active slot changed
    let next_state = apply_events(&state, &events);
    assert_eq!(
        next_state.players[1].active_slot, 1,
        "Target should switch to slot 1"
    );
}

#[test]
fn force_switch_with_only_one_pokemon_logs_failure() {
    // State with only 1 Pokémon on target team
    let state = make_state();

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
        switch_slot: None,
    };

    let effects = vec![effect("force_switch", json!({ "target": "target" }))];
    let events = apply_effects(&state, &effects, &mut ctx);

    // Should emit Log event since no valid switch target
    let log_event = events
        .iter()
        .find(|e| matches!(e, engine_rust::core::events::BattleEvent::Log { .. }));
    assert!(
        log_event.is_some(),
        "Expected Log event when no switch available"
    );
}
