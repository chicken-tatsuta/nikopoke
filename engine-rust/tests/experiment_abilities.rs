use engine_rust::core::battle::BattleEngine;
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages,
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
    stats: (i32, i32, i32, i32, i32, i32),
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
        consumed_item: None,

        evs: EVStats::default(),
        ability: ability.map(|s| s.to_string()),
        volatile_data: HashMap::new(),
        ability_data: HashMap::new(),
        move_pp: HashMap::new(),
        attack: atk,
        defense: def,
        sp_attack: spa,
        sp_defense: spd,
        speed: spe,

        weight_kg: 50.0,
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
        history: None,
    }
}

#[test]
fn test_contrary_self_debuff() {
    let path = Path::new("data/moves.yaml");
    let move_db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db, type_chart);

    // Serperior with Contrary using Leaf Storm
    let serperior = create_creature(
        "p1_serperior",
        "serperior",
        "Serperior",
        vec!["grass"],
        Some("contrary"),
        vec!["leaf_storm"],
        (100, 50, 50, 50, 50, 113),
    );

    let opponent = create_creature(
        "p2_opp",
        "opponent",
        "Opponent",
        vec!["normal"],
        None,
        vec!["tackle"],
        (100, 50, 50, 50, 50, 90),
    );

    let state = create_battle(vec![serperior], vec![opponent]);

    // Check Leaf Storm data
    assert!(
        engine.move_db.get("leaf_storm").is_some(),
        "Leaf Storm must exist"
    );

    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("leaf_storm".to_string()),
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move, // Do nothing
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
    ];

    let mut rng = || 0.5;
    let next_state = engine.step_battle(&state, &actions, &mut rng, Default::default());

    let serperior_after = &next_state.players[0].team[0];

    // Leaf Storm usually lowers SpA by 2. Contrary should invert this to +2.
    println!("Serperior SpA Stage: {}", serperior_after.stages.spa);
    assert_eq!(
        serperior_after.stages.spa, 2,
        "Contrary should turn -2 SpA into +2 SpA"
    );
}

#[test]
fn test_contrary_enemy_debuff() {
    let path = Path::new("data/moves.yaml");
    let move_db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db, type_chart);

    // Inkay with Contrary
    let inkay = create_creature(
        "p1_inkay",
        "inkay",
        "Inkay",
        vec!["dark", "psychic"],
        Some("contrary"),
        vec!["tackle"],
        (100, 50, 50, 50, 50, 45),
    );

    // Opponent with Metal Sound (lowers SpD by 2)
    let opponent = create_creature(
        "p2_opp",
        "opponent",
        "Opponent",
        vec!["steel"],
        None,
        vec!["metal_sound"],
        (100, 50, 50, 50, 50, 90),
    );

    let state = create_battle(vec![inkay], vec![opponent]);

    let actions = vec![
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: Some("metal_sound".to_string()),
            target_id: Some("p1".to_string()),
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
    ];

    let mut rng = || 0.5; // Hit
    let next_state = engine.step_battle(&state, &actions, &mut rng, Default::default());

    let inkay_after = &next_state.players[0].team[0];

    // Metal Sound lowers SpD by 2. Contrary should make it +2.
    println!("Inkay SpD Stage: {}", inkay_after.stages.spd);
    assert_eq!(
        inkay_after.stages.spd, 2,
        "Contrary should turn -2 SpD into +2 SpD from enemy move"
    );
}

#[test]
fn test_moody_turn_end() {
    let path = Path::new("data/moves.yaml");
    let move_db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db, type_chart);

    // Bidoof with Moody
    let bidoof = create_creature(
        "p1_bidoof",
        "bidoof",
        "Bidoof",
        vec!["normal"],
        Some("moody"),
        vec!["tackle"],
        (100, 50, 50, 50, 50, 50),
    );

    let opponent = create_creature(
        "p2_opp",
        "opponent",
        "Opponent",
        vec!["normal"],
        None,
        vec!["tackle"],
        (100, 50, 50, 50, 50, 50),
    );

    let state = create_battle(vec![bidoof], vec![opponent]);

    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
        Action {
            player_id: "p2".to_string(),
            action_type: ActionType::Move,
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
    ];

    let mut rng_call_count = 0;
    let mut rng = || {
        rng_call_count += 1;
        if rng_call_count % 2 == 1 {
            0.0 // index 0
        } else {
            0.2 // index 1
        }
    };

    let next_state = engine.step_battle(&state, &actions, &mut rng, Default::default());

    let bidoof_after = &next_state.players[0].team[0];

    // Atk should be +2, Def should be -1
    println!(
        "Bidoof Atk: {}, Def: {}",
        bidoof_after.stages.atk, bidoof_after.stages.def
    );
    assert_eq!(bidoof_after.stages.atk, 2, "Moody should raise Atk by 2");
    assert_eq!(bidoof_after.stages.def, -1, "Moody should lower Def by 1");

    let log_str = next_state.log.join("\n");
    println!("Log:\n{}", log_str);
}
