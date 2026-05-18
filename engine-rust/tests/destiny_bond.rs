use engine_rust::core::battle::{BattleEngine, BattleOptions};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, FieldState, PlayerState, StatStages,
};
use engine_rust::data::moves::{Effect, MoveData, MoveDatabase};
use engine_rust::data::type_chart::TypeChart;
use serde_json::{json, Value};
use std::collections::HashMap;

fn effect(effect_type: &str, data: Value) -> Effect {
    Effect {
        effect_type: effect_type.to_string(),
        data: data.as_object().cloned().unwrap_or_default(),
    }
}

fn status_move(id: &str, steps: Vec<Effect>) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("ghost".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps,
        tags: Vec::new(),
        crit_rate: None,
    }
}

fn damage_ratio_move(id: &str, ratio: f64, priority: i32) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(priority),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": ratio }))],
        tags: Vec::new(),
        crit_rate: None,
    }
}

fn creature(id: &str, name: &str, speed: i32, moves: &[&str]) -> CreatureState {
    CreatureState {
        id: id.to_string(),
        species_id: "testmon".to_string(),
        name: name.to_string(),
        level: 50,
        types: vec!["normal".to_string()],
        moves: moves.iter().map(|m| (*m).to_string()).collect(),
        ability: None,
        item: None,
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
        speed,
        weight_kg: 50.0,
    }
}

fn player(id: &str, active: CreatureState) -> PlayerState {
    PlayerState {
        id: id.to_string(),
        name: id.to_string(),
        team: vec![active],
        active_slot: 0,
        last_fainted_ability: None,
    }
}

fn action(player_id: &str, move_id: &str, target_id: &str) -> Action {
    Action {
        player_id: player_id.to_string(),
        action_type: ActionType::Move,
        move_id: Some(move_id.to_string()),
        target_id: Some(target_id.to_string()),
        slot: None,
        priority: None,
    }
}

fn engine() -> BattleEngine {
    let mut move_db = MoveDatabase::new();
    move_db.insert(status_move(
        "destiny_bond",
        vec![effect(
            "apply_status",
            json!({ "statusId": "destiny_bond", "target": "self" }),
        )],
    ));
    move_db.insert(status_move("wait", Vec::new()));
    move_db.insert(damage_ratio_move("quick_ko", 1.0, 1));
    BattleEngine::new(move_db, TypeChart::new())
}

fn battle_state() -> BattleState {
    BattleState {
        players: vec![
            player("p1", creature("c1", "User", 50, &["destiny_bond", "wait"])),
            player("p2", creature("c2", "Attacker", 100, &["quick_ko", "wait"])),
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

fn run_turn(engine: &BattleEngine, state: &BattleState, actions: &[Action]) -> BattleState {
    let mut rng = || 0.0;
    engine.step_battle(state, actions, &mut rng, BattleOptions::default())
}

fn active_has_status(state: &BattleState, player_id: &str, status_id: &str) -> bool {
    let player = state.players.iter().find(|p| p.id == player_id).unwrap();
    player.team[player.active_slot]
        .statuses
        .iter()
        .any(|s| s.id == status_id)
}

#[test]
fn destiny_bond_persists_until_users_next_action() {
    let engine = engine();
    let state = battle_state();

    let after_first = run_turn(
        &engine,
        &state,
        &[
            action("p1", "destiny_bond", "p2"),
            action("p2", "wait", "p1"),
        ],
    );
    assert!(active_has_status(&after_first, "p1", "destiny_bond"));

    let after_second = run_turn(
        &engine,
        &after_first,
        &[action("p1", "wait", "p2"), action("p2", "quick_ko", "p1")],
    );

    assert_eq!(after_second.players[0].team[0].hp, 0);
    assert_eq!(after_second.players[1].team[0].hp, 0);
    assert!(after_second
        .log
        .iter()
        .any(|line| line.contains("みちづれに なった")));
}

#[test]
fn destiny_bond_fails_only_after_a_successful_previous_use() {
    let engine = engine();
    let state = battle_state();

    let after_success = run_turn(
        &engine,
        &state,
        &[
            action("p1", "destiny_bond", "p2"),
            action("p2", "wait", "p1"),
        ],
    );
    assert!(active_has_status(&after_success, "p1", "destiny_bond"));

    let after_failed_repeat = run_turn(
        &engine,
        &after_success,
        &[
            action("p1", "destiny_bond", "p2"),
            action("p2", "wait", "p1"),
        ],
    );
    assert!(!active_has_status(
        &after_failed_repeat,
        "p1",
        "destiny_bond"
    ));
    assert!(after_failed_repeat
        .log
        .iter()
        .any(|line| line.contains("うまくきまらなかった")));

    let after_recovered = run_turn(
        &engine,
        &after_failed_repeat,
        &[
            action("p1", "destiny_bond", "p2"),
            action("p2", "wait", "p1"),
        ],
    );
    assert!(active_has_status(&after_recovered, "p1", "destiny_bond"));
}
