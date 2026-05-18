use engine_rust::core::battle::BattleEngine;
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, FieldState, PlayerState, StatStages,
};
use engine_rust::data::moves::MoveDatabase;
use engine_rust::data::type_chart::TypeChart;
use std::collections::HashMap;
use std::path::Path;

fn create_creature(
    id: &str,
    species: &str,
    name: &str,
    types: Vec<&str>,
    ability: Option<&str>,
    moves: Vec<&str>,
    stats: (i32, i32, i32, i32, i32, i32), // hp, atk, def, spa, spd, spe
) -> CreatureState {
    let (hp, atk, def, spa, spd, spe) = stats;
    CreatureState {
        id: id.to_string(),
        species_id: species.to_string(),
        name: name.to_string(),
        level: 50,
        types: types.iter().map(|s| s.to_string()).collect(),
        max_hp: hp,
        hp,
        moves: moves.iter().map(|s| s.to_string()).collect(),
        stages: StatStages::default(),
        statuses: Vec::new(),
        item: None,
        ability: ability.map(|s| s.to_string()),
        volatile_data: HashMap::new(),
        ability_data: HashMap::new(),
        move_pp: HashMap::new(),
        attack: atk,
        defense: def,
        sp_attack: spa,
        sp_defense: spd,
        speed: spe,
        weight_kg: 60.0,
    }
}

fn create_battle(p1_team: Vec<CreatureState>, p2_team: Vec<CreatureState>) -> BattleState {
    let p1 = PlayerState {
        id: "p1".to_string(),
        name: "Player 1".to_string(),
        team: p1_team,
        active_slot: 0,
        last_fainted_ability: None,
    };
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "Player 2".to_string(),
        team: p2_team,
        active_slot: 0,
        last_fainted_ability: None,
    };
    BattleState {
        players: vec![p1, p2],
        turn: 0,
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        log: Vec::new(),
        history: None, // Simplified for test
    }
}

#[test]
fn test_lightning_rod_immunity() {
    let path = Path::new("data/moves.yaml");
    let move_db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db, type_chart);

    let pikachu = create_creature(
        "p1_pikachu",
        "pikachu",
        "Pikachu",
        vec!["electric"],
        Some("lightning_rod"),
        vec!["tackle"],
        (100, 55, 40, 50, 50, 90),
    );

    let opponent = create_creature(
        "p2_opp",
        "opponent",
        "Opponent",
        vec!["normal"],
        None,
        vec!["thunderbolt"],
        (100, 50, 50, 50, 50, 90),
    );

    let state = create_battle(vec![pikachu], vec![opponent]);

    // P2 uses Thunderbolt on P1
    let actions = vec![
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: Some("thunderbolt".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move, // Use Move but with no move (effectively pass/struggle/wait)
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
    ];

    let mut rng = || 0.5;
    let next_state = engine.step_battle(&state, &actions, &mut rng, Default::default());

    // Verify Pikachu took no damage (HP still 100)
    let pikachu_after = &next_state.players[0].team[0];
    assert_eq!(
        pikachu_after.hp, 100,
        "Pikachu should take no damage due to Lightning Rod"
    );

    // Verify Special Attack raised
    assert_eq!(pikachu_after.stages.spa, 1, "Pikachu SpA should raise by 1");

    // Verify Log
    let log_str = next_state.log.join("\n");
    assert!(
        log_str.contains("drew in the electricity") || log_str.contains("電気の技を 吸い取った"),
        "Log should mention Lightning Rod"
    );
}

#[test]
fn test_thick_fat_resistance() {
    let path = Path::new("data/moves.yaml");
    let move_db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db, type_chart);

    // Snorlax with Thick Fat
    let snorlax = create_creature(
        "p1_snorlax",
        "snorlax",
        "Snorlax",
        vec!["normal"],
        Some("thick_fat"),
        vec!["tackle"],
        (200, 110, 65, 65, 110, 30),
    );

    // Opponent with Flame Wheel (fire)
    let opponent = create_creature(
        "p2_opp",
        "opponent",
        "Opponent",
        vec!["fire"],
        None,
        vec!["flame_wheel"],
        (100, 50, 50, 50, 50, 90),
    );

    let state = create_battle(vec![snorlax], vec![opponent.clone()]);
    let actions = vec![Action {
        player_id: "p2".to_string(),
        action_type: ActionType::Move,
        move_id: Some("flame_wheel".to_string()),
        target_id: Some("p1".to_string()),
        slot: None,
        priority: None,
    }];

    let mut rng = || 0.5; // Average roll
    let next_state = engine.step_battle(&state, &actions, &mut rng, Default::default());

    let snorlax_hp = next_state.players[0].team[0].hp;
    let damage_thick_fat = 200 - snorlax_hp;

    // Control: Snorlax WITHOUT Thick Fat
    let snorlax_no_ability = create_creature(
        "p1_snorlax_no",
        "snorlax",
        "Snorlax",
        vec!["normal"],
        None,
        vec!["tackle"],
        (200, 110, 65, 65, 110, 30),
    );
    let state_control = create_battle(vec![snorlax_no_ability], vec![opponent.clone()]);
    let next_state_control =
        engine.step_battle(&state_control, &actions, &mut rng, Default::default());
    let snorlax_hp_control = next_state_control.players[0].team[0].hp;
    let damage_control = 200 - snorlax_hp_control;

    assert!(
        damage_thick_fat > 0,
        "Should deal damage. Snorlax HP: {}",
        snorlax_hp
    );
    assert!(
        damage_thick_fat < damage_control,
        "Thick Fat should reduce fire damage. With: {}, Without: {}",
        damage_thick_fat,
        damage_control
    );
}

#[test]
fn test_prankster_priority() {
    let path = Path::new("data/moves.yaml");
    let move_db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db, type_chart);

    // Murkrow with Prankster (Speed 91)
    let murkrow = create_creature(
        "p1_murkrow",
        "murkrow",
        "Murkrow",
        vec!["dark", "flying"],
        Some("prankster"),
        vec!["swagger"],
        (100, 85, 42, 85, 42, 91),
    );

    // Opponent with higher speed (Speed 120)
    let opponent = create_creature(
        "p2_fast",
        "fastmon",
        "FastMon",
        vec!["normal"],
        None,
        vec!["tackle"],
        (100, 50, 50, 50, 50, 120),
    );

    let state = create_battle(vec![murkrow], vec![opponent]);

    // Murkrow uses Swagger (Status), Opponent uses Tackle
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("swagger".to_string()),
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

    // Force RNG to 0.0.
    // 1. Prankster priority calc (maybe consumes RNG? No, run_ability_value_hook doesn't usually)
    // 2. Murkrow uses Swagger. Hit check: 0.0 < 0.85. Hit.
    // 3. FastMon turn. Confusion check: 0.0 < 0.33 (approx). Self-hit.
    let mut rng = || 0.0;
    let next_state = engine.step_battle(&state, &actions, &mut rng, Default::default());

    // Check log.
    let log_str = next_state.log.join("\n");

    // Murkrow should move first and apply confusion.
    // Then FastMon tries to move, but hits itself due to confusion (rng 0.0).
    // If FastMon went first, it would NOT be confused yet, so it would use Tackle successfully.

    // If FastMon hit itself, log should contain "confusion" related message.
    // "It hurt itself in its confusion!" (or Japanese equivalent)
    // And it should NOT contain "FastMon used Tackle" (because self-hit prevents move).

    println!("Log:\n{}", log_str);

    let self_hit = log_str.contains("hurt itself in its confusion")
        || log_str.contains("わけもわからず")
        || log_str.contains("自分を 攻撃した");

    // Check for "confused" message first. "FastMon became confused!" or similar might be missing as we discovered.
    // But the self-hit message should be there.

    // If I don't know the exact message, checking that Tackle DID NOT happen is a good proxy,
    // coupled with the fact that Murkrow's Swagger MUST have hit to cause confusion.

    let tackle_used = log_str.contains("used Tackle")
        || log_str.contains("たいあたり！")
        || log_str.contains("たいあたり ！");

    // Also check that Opponent has Attack +2 (Swagger effect).
    let opp_stats = &next_state.players[1].team[0].stages;
    assert_eq!(
        opp_stats.atk, 2,
        "Opponent Attack should be +2 from Swagger"
    );

    // If Tackle was used, it means FastMon went first OR confusion didn't trigger self-hit.
    // Since we forced rng=0.0, confusion SHOULD trigger self-hit IF it exists.
    assert!(!tackle_used, "FastMon should not successfully use Tackle due to confusion self-hit. Prankster should have made Murkrow go first.");
    assert!(self_hit, "FastMon should have hurt itself in confusion.");
}
#[test]
fn test_skill_link_multi_hit() {
    let path = Path::new("data/moves.yaml");
    let move_db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db, type_chart);

    // Cinccino with Skill Link
    let cinccino = create_creature(
        "p1_cinccino",
        "cinccino",
        "Cinccino",
        vec!["normal"],
        Some("skill_link"),
        vec!["fury_attack"],
        (100, 95, 60, 65, 60, 115),
    );

    let opponent = create_creature(
        "p2_opp",
        "opponent",
        "Opponent",
        vec!["normal"],
        None,
        vec!["tackle"],
        (200, 100, 100, 100, 100, 100),
    );

    let state = create_battle(vec![cinccino], vec![opponent]);

    let actions = vec![Action {
        player_id: "p1".to_string(),
        action_type: ActionType::Move,
        move_id: Some("fury_attack".to_string()),
        target_id: Some("p2".to_string()),
        slot: None,
        priority: None,
    }];

    let mut rng = || 0.0; // Usually low roll => low hits
    let next_state = engine.step_battle(&state, &actions, &mut rng, Default::default());

    let log_str = next_state.log.join("\n");
    // Count how many "hit" messages or damage events.
    // The log usually says "Hit 5 time(s)".

    assert!(
        log_str.contains("Hit 5 time(s)") || log_str.contains("5回 あたった"),
        "Skill Link should ensure 5 hits. Log: {}",
        log_str
    );
}
