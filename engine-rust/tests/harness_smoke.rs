mod support;

use engine_rust::core::battle::BattleEngine;
use engine_rust::data::moves::{Effect, MoveData, MoveDatabase};
use engine_rust::data::type_chart::TypeChart;
use serde_json::{json, Map, Value};
use support::harness::{
    assert_active_hp, assert_no_diffs, battle_state, move_action, player, run_turn_with_seed,
    status, CreatureBuilder,
};

fn effect(effect_type: &str, data: Value) -> Effect {
    let map: Map<String, Value> = data.as_object().cloned().unwrap_or_default();
    Effect {
        effect_type: effect_type.to_string(),
        data: map,
    }
}

#[test]
fn harness_seeded_run_is_reproducible() {
    let mut move_db = MoveDatabase::new();
    move_db.insert(MoveData {
        id: "chip".to_string(),
        name: Some("Chip".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": 0.2 }))],
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

    let engine = BattleEngine::new(move_db, TypeChart::new());
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Alpha").moves(&["chip"]).build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Beta").moves(&["wait"]).build()],
        ),
    ]);

    let actions = vec![
        move_action("p1", "chip", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let next_a = run_turn_with_seed(&engine, &state, &actions, 42);
    let next_b = run_turn_with_seed(&engine, &state, &actions, 42);

    assert_no_diffs(&next_a, &next_b);
    assert_active_hp(&next_a, "p2", 80);
}

#[test]
fn harness_builder_can_set_initial_status() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Alpha")
                .with_status(status("burn", None))
                .build()],
        ),
        player("p2", "P2", vec![CreatureBuilder::new("c2", "Beta").build()]),
    ]);

    let p1_active = &state.players[0].team[state.players[0].active_slot];
    assert!(p1_active.statuses.iter().any(|s| s.id == "burn"));
}
