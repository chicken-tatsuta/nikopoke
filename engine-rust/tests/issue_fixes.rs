use engine_rust::core::abilities::{run_ability_hooks, AbilityHookContext};
use engine_rust::core::effects::{apply_effects, apply_events, EffectContext};
use engine_rust::core::events::{apply_event, BattleEvent};
use engine_rust::core::state::{
    BattleState, CreatureState, EVStats, FieldEffect, FieldState, PlayerState, StatStages, Status,
};
use engine_rust::data::moves::{Effect, MoveData, MoveDatabase};
use engine_rust::data::type_chart::TypeChart;
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
            moves: vec![],
            stages: StatStages::default(),
            statuses: Vec::new(),
            item: None,
            consumed_item: None,

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
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "Player 2".to_string(),
        team: vec![CreatureState {
            id: "c2".to_string(),
            species_id: "test_mon2".to_string(),
            name: "Mon2".to_string(),
            level: 50,
            types: vec!["normal".to_string()],
            max_hp: 100,
            hp: 100,
            moves: vec![],
            stages: StatStages::default(),
            statuses: Vec::new(),
            item: None,
            consumed_item: None,

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
        players: vec![p1, p2],
        turn: 1,
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        log: Vec::new(),
        history: None,
    }
}

#[test]
fn test_morning_sun_healing() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("morning_sun").unwrap();
    let mut state = create_test_state();
    state.players[0].team[0].hp = 20; // 20/100 HP

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };

    let events = apply_effects(&state, &move_data.steps, &mut ctx);

    // Should contain a damage event with a negative amount (healing)
    let heal_event = events.iter().find(|e| match e {
        BattleEvent::Damage { amount, .. } => *amount < 0,
        _ => false,
    });

    assert!(
        heal_event.is_some(),
        "Morning Sun should produce a healing event (negative damage)"
    );
    if let Some(BattleEvent::Damage { amount, .. }) = heal_event {
        assert_eq!(*amount, -50); // 50% of 100
    }

    let next_state = apply_event(&state, heal_event.unwrap());
    assert_eq!(next_state.players[0].team[0].hp, 70); // 20 + 50
}

#[test]
fn growth_raises_attack_and_sp_attack_by_two_under_sun() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("growth").unwrap();
    let mut state = create_test_state();
    state.field.global.push(FieldEffect {
        id: "sun".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };

    let events = apply_effects(&state, &move_data.steps, &mut ctx);
    let next_state = apply_events(&state, &events);
    let active = &next_state.players[0].team[0];
    assert_eq!(active.stages.atk, 2);
    assert_eq!(active.stages.spa, 2);
}

#[test]
fn synthesis_heals_two_thirds_under_sun_and_one_quarter_in_rain() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("synthesis").unwrap();
    let type_chart = TypeChart::new();

    for (weather_id, expected_amount) in [("sun", -66), ("rain", -25)] {
        let mut state = create_test_state();
        state.players[0].team[0].hp = 1;
        state.field.global.push(FieldEffect {
            id: weather_id.to_string(),
            remaining_turns: Some(5),
            data: HashMap::new(),
        });
        let mut rng = || 0.5;
        let mut ctx = EffectContext {
            attacker_player_id: "p1".to_string(),
            target_player_id: "p2".to_string(),
            move_data: Some(move_data),
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
            move_blocked_by_protect: false,
            switch_slot: None,
        };

        let events = apply_effects(&state, &move_data.steps, &mut ctx);
        let heal_amount = events
            .iter()
            .find_map(|event| match event {
                BattleEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .expect("synthesis should emit healing as negative damage");
        assert_eq!(heal_amount, expected_amount, "weather: {weather_id}");
    }
}

#[test]
fn knock_off_deals_more_damage_to_held_item_and_removes_it() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("knock_off").unwrap();
    let mut state = create_test_state();
    state.players[0].team[0].attack = 100;
    state.players[1].team[0].defense = 100;

    let baseline_damage = first_damage_amount_for_move(&state, move_data);

    state.players[1].team[0].item = Some("choice_scarf".to_string());
    let item_damage = first_damage_amount_for_move(&state, move_data);
    assert!(
        item_damage > baseline_damage,
        "knock off should deal more damage when the target has an item"
    );

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };

    let events = apply_effects(&state, &move_data.steps, &mut ctx);
    let next_state = apply_events(&state, &events);

    assert_eq!(next_state.players[1].team[0].item, None);
    assert!(
        next_state
            .log
            .iter()
            .any(|line| line.contains("Mon1は Mon2の こだわりスカーフを はたきおとした！")),
        "knock off should log item removal"
    );
}

#[test]
fn knock_off_does_not_remove_item_when_damage_step_misses() {
    let mut state = create_test_state();
    state.players[1].team[0].item = Some("choice_scarf".to_string());
    let move_data = MoveData {
        id: "knock_off".to_string(),
        name: Some("はたきおとす".to_string()),
        move_type: Some("dark".to_string()),
        category: Some("physical".to_string()),
        pp: Some(20),
        power: Some(65),
        accuracy: Some(0.0),
        priority: Some(0),
        description: None,
        steps: vec![
            Effect {
                effect_type: "damage".to_string(),
                data: serde_json::json!({ "power": 65, "accuracy": 0.0 })
                    .as_object()
                    .cloned()
                    .unwrap(),
            },
            Effect {
                effect_type: "remove_item".to_string(),
                data: serde_json::json!({ "target": "target", "requireDamage": true })
                    .as_object()
                    .cloned()
                    .unwrap(),
            },
        ],
        tags: vec!["contact".to_string()],
        crit_rate: None,
    };

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(&move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };

    let events = apply_effects(&state, &move_data.steps, &mut ctx);
    let next_state = apply_events(&state, &events);

    assert_eq!(
        next_state.players[1].team[0].item.as_deref(),
        Some("choice_scarf")
    );
}

#[test]
fn fling_uses_fixed_power_50_and_consumes_user_item() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("fling").unwrap();
    let mut state = create_test_state();
    state.players[0].team[0].item = Some("choice_scarf".to_string());
    state.players[0].team[0].attack = 100;
    state.players[1].team[0].defense = 100;

    let reference_move = MoveData {
        id: "fling_reference".to_string(),
        name: None,
        move_type: move_data.move_type.clone(),
        category: move_data.category.clone(),
        pp: move_data.pp,
        power: Some(50),
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![Effect {
            effect_type: "damage".to_string(),
            data: serde_json::json!({ "power": 50, "accuracy": 1.0 })
                .as_object()
                .cloned()
                .unwrap(),
        }],
        tags: Vec::new(),
        crit_rate: None,
    };

    assert_eq!(
        first_damage_amount_for_move(&state, move_data),
        first_damage_amount_for_move(&state, &reference_move)
    );

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };
    let events = apply_effects(&state, &move_data.steps, &mut ctx);
    let next_state = apply_events(&state, &events);

    assert_eq!(next_state.players[0].team[0].item, None);
    assert!(next_state
        .log
        .iter()
        .any(|line| line.contains("Mon1は こだわりスカーフを なげつけた！")));
}

#[test]
fn fling_with_flame_orb_burns_target() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("fling").unwrap();
    let mut state = create_test_state();
    state.players[0].team[0].item = Some("flame_orb".to_string());

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };
    let events = apply_effects(&state, &move_data.steps, &mut ctx);
    let next_state = apply_events(&state, &events);

    assert_eq!(next_state.players[0].team[0].item, None);
    assert!(next_state.players[1].team[0]
        .statuses
        .iter()
        .any(|status| status.id == "burn"));
}

#[test]
fn fling_does_not_consume_item_when_protected() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("fling").unwrap();
    let mut state = create_test_state();
    state.players[0].team[0].item = Some("flame_orb".to_string());
    state.players[1].team[0].statuses.push(Status {
        id: "protect".to_string(),
        remaining_turns: Some(1),
        data: HashMap::new(),
    });

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };
    let events = apply_effects(&state, &move_data.steps, &mut ctx);
    let next_state = apply_events(&state, &events);

    assert_eq!(
        next_state.players[0].team[0].item.as_deref(),
        Some("flame_orb")
    );
    assert!(!next_state.players[1].team[0]
        .statuses
        .iter()
        .any(|status| status.id == "burn"));
}

#[test]
fn poltergeist_logs_target_item_without_removing_it() {
    let move_db = MoveDatabase::load_default().unwrap();
    let move_data = move_db.get("poltergeist").unwrap();
    let mut state = create_test_state();
    state.players[1].team[0].types = vec!["psychic".to_string()];
    state.players[1].team[0].item = Some("choice_scarf".to_string());

    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };
    let events = apply_effects(&state, &move_data.steps, &mut ctx);
    let next_state = apply_events(&state, &events);

    assert!(next_state
        .log
        .iter()
        .any(|line| line.contains("Mon2に こだわりスカーフが おそいかかる！")));
    assert_eq!(
        next_state.players[1].team[0].item.as_deref(),
        Some("choice_scarf")
    );
}

#[test]
fn eruption_and_water_spout_use_continuous_user_hp_ratio_power() {
    let move_db = MoveDatabase::load_default().unwrap();
    for (move_id, expected_power) in [("eruption", 37), ("water_spout", 75)] {
        let move_data = move_db.get(move_id).unwrap();
        let mut state = create_test_state();
        state.players[0].team[0].hp = if move_id == "eruption" { 25 } else { 50 };
        state.players[0].team[0].sp_attack = 100;
        state.players[1].team[0].sp_defense = 100;

        let actual = first_damage_amount_for_move(&state, move_data);
        let reference_move = MoveData {
            id: format!("{move_id}_reference"),
            name: None,
            move_type: move_data.move_type.clone(),
            category: move_data.category.clone(),
            pp: move_data.pp,
            power: Some(expected_power),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: vec![Effect {
                effect_type: "damage".to_string(),
                data: serde_json::json!({ "power": expected_power, "accuracy": 1.0 })
                    .as_object()
                    .cloned()
                    .unwrap(),
            }],
            tags: Vec::new(),
            crit_rate: None,
        };
        let expected = first_damage_amount_for_move(&state, &reference_move);
        assert_eq!(
            actual, expected,
            "{move_id} should use floor(150 * current HP / max HP)"
        );
    }
}

fn first_damage_amount_for_move(state: &BattleState, move_data: &MoveData) -> i32 {
    let mut rng = || 0.5;
    let type_chart = TypeChart::new();
    let mut ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(move_data),
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
        move_blocked_by_protect: false,
        switch_slot: None,
    };
    apply_effects(state, &move_data.steps, &mut ctx)
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .expect("move should emit damage")
}

#[test]
fn test_power_of_alchemy_reset() {
    let mut state = create_test_state();
    state.players[0].team[0].ability = Some("power_of_alchemy".to_string());
    state.players[0].last_fainted_ability = Some("levitate".to_string());

    let mut rng = || 0.5;
    let result = run_ability_hooks(
        &state,
        "p1",
        "onSwitchIn",
        AbilityHookContext {
            rng: &mut rng,
            action: None,
            move_data: None,
        },
    );

    let state_after_switch_in = result.state.unwrap();
    let mon = &state_after_switch_in.players[0].team[0];
    assert_eq!(mon.ability.as_deref(), Some("levitate"));
    assert_eq!(
        mon.ability_data
            .get("originalAbility")
            .and_then(|v| v.as_str()),
        Some("power_of_alchemy")
    );

    // Now switch out
    let switch_out_event = BattleEvent::Switch {
        player_id: "p1".to_string(),
        slot: 1,
    };

    // Add another mon to team p1 for switching
    let mut p1_team = state_after_switch_in.players[0].team.clone();
    p1_team.push(p1_team[0].clone());
    p1_team[1].id = "c1_alt".to_string();
    let mut state_for_switch = state_after_switch_in.clone();
    state_for_switch.players[0].team = p1_team;

    let state_after_switch_out = apply_event(&state_for_switch, &switch_out_event);

    // Check if c1 (now in slot 0, inactive) has its ability restored
    let mon_after = &state_after_switch_out.players[0].team[0];
    assert_eq!(mon_after.ability.as_deref(), Some("power_of_alchemy"));
    assert!(mon_after.ability_data.is_empty());
}
