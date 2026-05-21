use engine_rust::core::effects::{apply_effects, apply_events, EffectContext};
use engine_rust::core::events::BattleEvent;
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages,
    Status,
};
use engine_rust::core::statuses::{run_status_hooks, StatusHookContext};
use engine_rust::data::moves::{Effect, MoveData};
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

fn make_revival_state() -> BattleState {
    let mut active = make_creature("c1", "Alpha");
    active.hp = 80;
    let mut fainted = make_creature("c2", "Beta");
    fainted.hp = 0;
    fainted.max_hp = 101;
    fainted.statuses.push(Status {
        id: "pending_switch".to_string(),
        remaining_turns: None,
        data: HashMap::new(),
    });
    let p1 = PlayerState {
        id: "p1".to_string(),
        name: "P1".to_string(),
        team: vec![active, fainted],
        active_slot: 0,
        last_fainted_ability: Some("none".to_string()),
    };
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "P2".to_string(),
        team: vec![make_creature("c3", "Gamma")],
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

fn test_move(id: &str, category: &str, accuracy: Option<f32>, tags: Vec<String>) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some(category.to_string()),
        pp: Some(10),
        power: Some(40),
        accuracy,
        priority: Some(0),
        description: None,
        steps: Vec::new(),
        tags,
        crit_rate: None,
    }
}

fn effect_context<'a>(
    rng: &'a mut dyn FnMut() -> f64,
    type_chart: &'a TypeChart,
    move_data: &'a MoveData,
) -> EffectContext<'a> {
    EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
        rng,
        turn: 0,
        type_chart,
        bypass_protect: false,
        ignore_immunity: false,
        bypass_substitute: false,
        ignore_substitute: false,
        ignore_ability: false,
        is_sound: false,
        last_damage: None,
        move_blocked_by_protect: false,
        switch_slot: None,
    }
}

#[test]
fn target_evasion_stage_reduces_damage_move_accuracy() {
    let mut state = make_state();
    state.players[1].team[0].stages.evasion = 1;
    let move_data = test_move("strike", "physical", Some(1.0), Vec::new());
    let type_chart = TypeChart::new();
    let mut rng = || 0.8;
    let mut ctx = effect_context(&mut rng, &type_chart, &move_data);

    let events = apply_effects(
        &state,
        &[effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        &mut ctx,
    );

    assert!(events.iter().any(
        |event| matches!(event, BattleEvent::Log { message, .. } if message == "しかし はずれた！")
    ));
    assert!(!events
        .iter()
        .any(|event| matches!(event, BattleEvent::Damage { .. })));
}

#[test]
fn attacker_accuracy_stage_increases_damage_move_accuracy() {
    let mut state = make_state();
    state.players[0].team[0].stages.accuracy = 1;
    state.players[1].team[0].stages.evasion = 1;
    let move_data = test_move("strike", "physical", Some(1.0), Vec::new());
    let type_chart = TypeChart::new();
    let mut rng = || 0.99;
    let mut ctx = effect_context(&mut rng, &type_chart, &move_data);

    let events = apply_effects(
        &state,
        &[effect("damage", json!({ "power": 40, "accuracy": 1.0 }))],
        &mut ctx,
    );

    assert!(events
        .iter()
        .any(|event| matches!(event, BattleEvent::Damage { .. })));
}

#[test]
fn target_evasion_stage_reduces_status_move_accuracy() {
    let mut state = make_state();
    state.players[1].team[0].stages.evasion = 1;
    let move_data = test_move("taunt_test", "status", Some(1.0), Vec::new());
    let type_chart = TypeChart::new();
    let mut rng = || 0.8;
    let mut ctx = effect_context(&mut rng, &type_chart, &move_data);

    let events = apply_effects(
        &state,
        &[effect(
            "apply_status",
            json!({ "statusId": "taunt", "target": "target" }),
        )],
        &mut ctx,
    );

    assert!(events.iter().any(
        |event| matches!(event, BattleEvent::Log { message, .. } if message == "しかし はずれた！")
    ));
    assert!(!events
        .iter()
        .any(|event| matches!(event, BattleEvent::ApplyStatus { .. })));
}

#[test]
fn always_hit_move_ignores_evasion_stage() {
    let mut state = make_state();
    state.players[1].team[0].stages.evasion = 6;
    let move_data = test_move(
        "aerial_ace_test",
        "physical",
        None,
        vec!["always_hit".to_string()],
    );
    let type_chart = TypeChart::new();
    let mut rng = || 0.99;
    let mut ctx = effect_context(&mut rng, &type_chart, &move_data);

    let events = apply_effects(
        &state,
        &[effect("damage", json!({ "power": 40 }))],
        &mut ctx,
    );

    assert!(events
        .iter()
        .any(|event| matches!(event, BattleEvent::Damage { .. })));
}

#[test]
fn revival_blessing_restores_first_fainted_bench_member() {
    let state = make_revival_state();
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

    let events = apply_effects(
        &state,
        &[effect(
            "revive_fainted",
            json!({ "target": "user", "ratioMaxHp": 0.5 }),
        )],
        &mut ctx,
    );
    let next = apply_events(&state, &events);
    let revived = &next.players[0].team[1];

    assert_eq!(revived.hp, 50);
    assert!(!revived
        .statuses
        .iter()
        .any(|status| status.id == "pending_switch"));
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("さいきのいのりで 復活した")));
}

#[test]
fn revival_blessing_uses_selected_fainted_slot() {
    let mut state = make_revival_state();
    let mut second_fainted = make_creature("c4", "Delta");
    second_fainted.hp = 0;
    second_fainted.max_hp = 120;
    state.players[0].team.push(second_fainted);
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
        switch_slot: Some(2),
    };

    let events = apply_effects(
        &state,
        &[effect(
            "revive_fainted",
            json!({ "target": "user", "ratioMaxHp": 0.5 }),
        )],
        &mut ctx,
    );
    let next = apply_events(&state, &events);

    assert_eq!(next.players[0].team[1].hp, 0);
    assert_eq!(next.players[0].team[2].hp, 60);
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("Deltaは さいきのいのりで 復活した")));
}

#[test]
fn revival_blessing_fails_without_fainted_bench_member() {
    let mut state = make_revival_state();
    state.players[0].team[1].hp = 10;
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

    let events = apply_effects(
        &state,
        &[effect(
            "revive_fainted",
            json!({ "target": "user", "ratioMaxHp": 0.5 }),
        )],
        &mut ctx,
    );
    let next = apply_events(&state, &events);

    assert_eq!(next.players[0].team[1].hp, 10);
    assert!(next
        .log
        .iter()
        .any(|line| line.contains("うまく きまらなかった")));
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
        move_blocked_by_protect: false,
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
        move_blocked_by_protect: false,
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
        move_blocked_by_protect: false,
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
        move_blocked_by_protect: false,
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
    state.players[0]
        .team
        .push(make_creature("c1_bench", "Alpha Bench"));
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
        move_blocked_by_protect: false,
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
        move_blocked_by_protect: false,
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
