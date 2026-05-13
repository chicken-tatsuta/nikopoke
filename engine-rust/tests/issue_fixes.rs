use engine_rust::core::abilities::{run_ability_hooks, AbilityHookContext};
use engine_rust::core::effects::{apply_effects, EffectContext};
use engine_rust::core::events::{apply_event, BattleEvent};
use engine_rust::core::state::{BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages};
use engine_rust::data::moves::MoveDatabase;
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
