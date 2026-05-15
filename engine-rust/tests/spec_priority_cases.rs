mod support;

use engine_rust::core::battle::{determine_timeout_winner, determine_winner, BattleEngine};
use engine_rust::core::effects::{apply_effects, EffectContext};
use engine_rust::core::events::BattleEvent;
use engine_rust::core::state::{Action, ActionType, BattleState, FieldEffect};
use engine_rust::data::learnsets::LearnsetDatabase;
use engine_rust::data::moves::{Effect, MoveData, MoveDatabase};
use engine_rust::data::type_chart::TypeChart;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use support::harness::{
    assert_active_has_status, assert_active_hp, assert_field_has_status, assert_no_diffs,
    battle_state, move_action, player, run_turn_with_seed, run_turns_with_seed, status,
    switch_action, CreatureBuilder,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Priority {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Clone, Copy, Debug)]
struct CaseMeta {
    id: &'static str,
    priority: Priority,
    enabled: bool,
}

const CASES: &[CaseMeta] = &[
    CaseMeta {
        id: "P0-CRIT-DEF-STAGE-IGNORE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-CRIT-ATK-STAGE-IGNORE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-CRIT-WALL-BYPASS",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-FIELD-STATUS-ATTACH",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-FIELD-STATUS-NONSTACK-REFRESH",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-DAMAGE-ROLL-GOLDEN",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-TRICK-ROOM-ORDER",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-PRIORITY-VS-TRICK-ROOM",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-REFLECT-DAMAGE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-REFLECT-CATEGORY-BOUNDARY",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-LIGHT-SCREEN-DAMAGE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-LIGHT-SCREEN-CATEGORY-BOUNDARY",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-TAILWIND-SPEED",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-TOXIC-RESIDUAL",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-TOXIC-SWITCH-RESET",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-TOXIC-SWITCH-COUNTER-CLEARED",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-PROTECT-CHAIN-PROB",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-PROTECT-CHAIN-SUCCESS-COUNTER",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-PROTECT-RESET-ON-NONPROTECT",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-PROTECT-BLOCKS-DAMAGE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-PROTECT-FAIL-ALLOWS-DAMAGE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-SLEEP-SWITCH",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-SLEEP-WAKE-ON-COUNTER-ZERO",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-SLEEP-SWITCH-TURN-COUNTER",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-SWITCH-CLEANUP",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-MANUAL-NOOP-GATE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-WIN-SIMULTANEOUS-FAINT",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-WIN-SIMULTANEOUS-FAINT-SPEED-TIE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-WIN-TIMEOUT-RULE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-WIN-TIMEOUT-TOTAL-HP-TIEBREAK",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-WIN-TIMEOUT-EXACT-TIE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-WIN-SINGLE-ALIVE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P0-TOXIC-MIN-DAMAGE",
        priority: Priority::P0,
        enabled: true,
    },
    CaseMeta {
        id: "P1-LEARNSET-MOVE-REF",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-TARGET-LITERAL-LINT",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-STATUS-ID-LINT",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-ABILITY-STATUS-FIELD",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-ENDTURN-ORDER",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-MANUAL-REASON-TAXONOMY",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-TAILWIND-SIDE-SCOPE",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-TARGET-DEFAULT-OPPONENT",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-MOVE-PRIORITY-RANGE",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-MANUAL-REASON-NONEMPTY",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-SWITCH-ACTIVE-SLOT-REJECT",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-SWITCH-WITHOUT-SLOT-REJECT",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-USE-ITEM-NO-ITEM-REJECT",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P1-USE-ITEM-WITH-ITEM-LOG",
        priority: Priority::P1,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-REGISTRY-INTEGRITY",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-DOC-SYNC",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-ID-PREFIX-CHECK",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-DOC-UNIQUE",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-DOC-BIDIRECTIONAL-SYNC",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-DOC-TEST-FN-EXISTS",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-DOC-ROW-COUNT-MATCH",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-CASE-DOC-PRIORITY-COLUMN-CHECK",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P2-DOUBLE-MODEL-SMOKE",
        priority: Priority::P2,
        enabled: true,
    },
    CaseMeta {
        id: "P3-SEED-DETERMINISM",
        priority: Priority::P3,
        enabled: true,
    },
    CaseMeta {
        id: "P3-UNKNOWN-MOVE-GUARD",
        priority: Priority::P3,
        enabled: true,
    },
    CaseMeta {
        id: "P3-INVALID-SWITCH-SLOT-GUARD",
        priority: Priority::P3,
        enabled: true,
    },
    CaseMeta {
        id: "P3-FAINTED-SWITCH-SLOT-GUARD",
        priority: Priority::P3,
        enabled: true,
    },
    CaseMeta {
        id: "P3-TIMEOUT-NON-2P-NONE",
        priority: Priority::P3,
        enabled: true,
    },
    CaseMeta {
        id: "P3-WINNER-ALL-FAINT-NON-2P-NONE",
        priority: Priority::P3,
        enabled: true,
    },
    CaseMeta {
        id: "P3-UNKNOWN-PLAYER-ACTION-GUARD",
        priority: Priority::P3,
        enabled: true,
    },
    CaseMeta {
        id: "P3-MISSING-MOVE-ID-GUARD",
        priority: Priority::P3,
        enabled: true,
    },
];

fn effect(effect_type: &str, data: Value) -> Effect {
    let map: Map<String, Value> = data.as_object().cloned().unwrap_or_default();
    Effect {
        effect_type: effect_type.to_string(),
        data: map,
    }
}

fn wait_move() -> MoveData {
    MoveData {
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
    }
}

fn damage_move(id: &str, category: &str, power: i32, crit_rate: Option<i32>) -> MoveData {
    damage_move_with_priority(id, category, power, crit_rate, 0)
}

fn damage_move_with_priority(
    id: &str,
    category: &str,
    power: i32,
    crit_rate: Option<i32>,
    priority: i32,
) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some(category.to_string()),
        pp: Some(10),
        power: Some(power),
        accuracy: Some(1.0),
        priority: Some(priority),
        description: None,
        steps: vec![effect("damage", json!({ "power": power, "accuracy": 1.0 }))],
        tags: Vec::new(),
        crit_rate,
    }
}

fn sucker_punch_move() -> MoveData {
    MoveData {
        id: "sucker_punch".to_string(),
        name: Some("ふいうち".to_string()),
        move_type: Some("dark".to_string()),
        category: Some("physical".to_string()),
        pp: Some(5),
        power: Some(70),
        accuracy: Some(1.0),
        priority: Some(1),
        description: None,
        steps: vec![effect(
            "conditional",
            json!({
                "if": { "type": "target_selected_attacking_move" },
                "then": [{ "type": "damage", "power": 70, "accuracy": 1.0 }],
                "else": [{ "type": "log", "message": "しかし うまく きまらなかった！" }]
            }),
        )],
        tags: vec!["contact".to_string()],
        crit_rate: None,
    }
}

fn field_status_move(id: &str, status_id: &str) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "apply_field_status",
            json!({ "statusId": status_id, "duration": 5, "stack": false }),
        )],
        tags: Vec::new(),
        crit_rate: None,
    }
}

fn status_stage_move(id: &str, target: &str, stat: &str, delta: i32) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "modify_stage",
            json!({ "target": target, "stages": { stat: delta } }),
        )],
        tags: Vec::new(),
        crit_rate: None,
    }
}

fn protect_move(id: &str, priority: i32) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(priority),
        description: None,
        steps: vec![effect("protect", json!({}))],
        tags: Vec::new(),
        crit_rate: None,
    }
}

fn endure_move(id: &str, priority: i32) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: None,
        priority: Some(priority),
        description: None,
        steps: vec![effect("endure", json!({}))],
        tags: Vec::new(),
        crit_rate: None,
    }
}

fn status_move(id: &str, status_id: &str) -> MoveData {
    MoveData {
        id: id.to_string(),
        name: Some(id.to_string()),
        move_type: Some("normal".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "apply_status",
            json!({ "statusId": status_id, "target": "target", "chance": 1 }),
        )],
        tags: Vec::new(),
        crit_rate: None,
    }
}

fn make_engine(moves: Vec<MoveData>) -> BattleEngine {
    let mut move_db = MoveDatabase::new();
    for mv in moves {
        move_db.insert(mv);
    }
    BattleEngine::new(move_db, TypeChart::new())
}

fn active_hp(state: &BattleState, player_id: &str) -> i32 {
    let player = state
        .players
        .iter()
        .find(|p| p.id == player_id)
        .unwrap_or_else(|| panic!("player '{}' not found", player_id));
    let active = &player.team[player.active_slot];
    active.hp
}

fn field_status_count(state: &BattleState, status_id: &str) -> usize {
    state
        .field
        .global
        .iter()
        .filter(|effect| effect.id == status_id)
        .count()
}

fn effects_from_value(value: Option<&Value>) -> Vec<Effect> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn walk_effects<F>(effects: &[Effect], visit: &mut F)
where
    F: FnMut(&Effect),
{
    for effect in effects {
        visit(effect);
        match effect.effect_type.as_str() {
            "chance" | "conditional" => {
                let then_effects = effects_from_value(effect.data.get("then"));
                let else_effects = effects_from_value(effect.data.get("else"));
                walk_effects(&then_effects, visit);
                walk_effects(&else_effects, visit);
            }
            "repeat" | "delay" | "over_time" => {
                let nested = effects_from_value(
                    effect
                        .data
                        .get("steps")
                        .or_else(|| effect.data.get("effects")),
                );
                walk_effects(&nested, visit);
            }
            _ => {}
        }
    }
}

fn is_allowed_target_literal(value: &str) -> bool {
    if matches!(value, "self" | "target" | "all") {
        return true;
    }
    if let Some(rest) = value.strip_prefix('p') {
        return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit());
    }
    false
}

fn is_supported_status_id(status_id: &str) -> bool {
    matches!(
        status_id,
        "burn"
            | "poison"
            | "toxic"
            | "paralysis"
            | "sleep"
            | "freeze"
            | "confusion"
            | "flinch"
            | "protect"
            | "substitute"
            | "lock_move"
            | "disable_move"
            | "encore"
            | "taunt"
            | "torment"
            | "trapped"
            | "lock_on"
            | "destiny_bond"
            | "leech_seed"
            | "curse"
            | "yawn"
            | "bind"
            | "magnet_rise"
            | "imprison"
            | "aqua_ring"
            | "ingrain"
            | "throat_chop"
            | "wish"
            | "pending_switch"
            | "item"
            | "berry"
            | "berry_consumed"
            | "leftovers"
            | "black_sludge"
            | "grassy_terrain"
            | "electric_terrain"
            | "misty_terrain"
            | "psychic_terrain"
            | "gravity"
            | "rain"
            | "sun"
            | "sandstorm"
            | "trick_room"
            | "reflect"
            | "light_screen"
            | "aurora_veil"
            | "tailwind"
            | "mist"
            | "safeguard"
            | "healing_wish"
            | "spikes"
            | "toxic_spikes"
            | "stealth_rock"
            | "sticky_web"
    )
}

fn has_status_log(events: &[BattleEvent], pattern: &str) -> bool {
    events.iter().any(|event| match event {
        BattleEvent::Log { message, .. } => message.contains(pattern),
        _ => false,
    })
}

fn first_damage_amount(events: &[BattleEvent]) -> i32 {
    events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage { amount, .. } => Some(*amount),
            _ => None,
        })
        .expect("expected at least one damage event")
}

fn markdown_case_ids(doc: &str) -> Vec<String> {
    doc.lines()
        .filter_map(|line| {
            if !line.starts_with("| `P") {
                return None;
            }
            line.split('`').nth(1).map(|id| id.to_string())
        })
        .collect()
}

fn markdown_test_names(doc: &str) -> Vec<String> {
    doc.lines()
        .filter_map(|line| {
            if !line.starts_with("| `P") {
                return None;
            }
            line.split('`').nth(3).map(|name| name.to_string())
        })
        .collect()
}

fn markdown_case_rows(doc: &str) -> Vec<(String, String, String)> {
    doc.lines()
        .filter_map(|line| {
            if !line.starts_with("| `P") {
                return None;
            }
            let cols: Vec<&str> = line.split('|').map(|col| col.trim()).collect();
            if cols.len() < 6 {
                return None;
            }
            let id = cols[1].trim_matches('`').to_string();
            let priority = cols[2].to_string();
            let test_name = cols[4].trim_matches('`').to_string();
            Some((id, priority, test_name))
        })
        .collect()
}

#[test]
fn p0_crit_ignores_positive_def_stage() {
    let engine = make_engine(vec![
        damage_move("always_crit", "physical", 90, Some(3)),
        wait_move(),
    ]);

    let actions = vec![
        move_action("p1", "always_crit", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let state_no_boost = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["always_crit"])
                .stats(90, 50, 50, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(50, 100, 50, 50, 50)
                .build()],
        ),
    ]);

    let mut state_with_boost = state_no_boost.clone();
    state_with_boost.players[1].team[0].stages.def = 6;

    let next_no_boost = run_turn_with_seed(&engine, &state_no_boost, &actions, 7);
    let next_with_boost = run_turn_with_seed(&engine, &state_with_boost, &actions, 7);

    assert_eq!(
        active_hp(&next_no_boost, "p2"),
        active_hp(&next_with_boost, "p2"),
        "critical damage should ignore positive defense stages"
    );
}

#[test]
fn p0_spec_crit_ignores_negative_attack_stage() {
    let engine = make_engine(vec![
        damage_move("always_crit", "physical", 90, Some(3)),
        wait_move(),
    ]);

    let actions = vec![
        move_action("p1", "always_crit", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let base_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["always_crit"])
                .stats(90, 50, 50, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(50, 100, 50, 50, 50)
                .build()],
        ),
    ]);

    let mut state_with_drop = base_state.clone();
    state_with_drop.players[0].team[0].stages.atk = -6;

    let next_base = run_turn_with_seed(&engine, &base_state, &actions, 8);
    let next_with_drop = run_turn_with_seed(&engine, &state_with_drop, &actions, 8);

    assert_eq!(
        active_hp(&next_base, "p2"),
        active_hp(&next_with_drop, "p2"),
        "critical damage should ignore attack drops on attacker"
    );
}

#[test]
fn p0_spec_crit_bypasses_walls_while_non_crit_does_not() {
    let non_crit_engine = make_engine(vec![
        damage_move("strike", "physical", 80, None),
        wait_move(),
    ]);
    let crit_engine = make_engine(vec![
        damage_move("always_crit", "physical", 80, Some(3)),
        wait_move(),
    ]);

    let base_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["strike", "always_crit"])
                .stats(100, 50, 50, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(50, 100, 50, 50, 50)
                .build()],
        ),
    ]);
    let mut state_with_reflect = base_state.clone();
    state_with_reflect.field.global.push(FieldEffect {
        id: "reflect".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let non_crit_actions = vec![
        move_action("p1", "strike", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let crit_actions = vec![
        move_action("p1", "always_crit", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let non_crit_no_wall = run_turn_with_seed(&non_crit_engine, &base_state, &non_crit_actions, 31);
    let non_crit_with_wall =
        run_turn_with_seed(&non_crit_engine, &state_with_reflect, &non_crit_actions, 31);
    let crit_no_wall = run_turn_with_seed(&crit_engine, &base_state, &crit_actions, 31);
    let crit_with_wall = run_turn_with_seed(&crit_engine, &state_with_reflect, &crit_actions, 31);

    assert!(
        active_hp(&non_crit_with_wall, "p2") > active_hp(&non_crit_no_wall, "p2"),
        "non-crit damage should be reduced by reflect"
    );
    assert_eq!(
        active_hp(&crit_with_wall, "p2"),
        active_hp(&crit_no_wall, "p2"),
        "critical damage should bypass reflect"
    );
}

#[test]
fn p0_field_status_move_sets_status_on_field() {
    let engine = make_engine(vec![
        field_status_move("set_trick_room", "trick_room"),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Alpha")
                .moves(&["set_trick_room"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Beta").moves(&["wait"]).build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "set_trick_room", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let next = run_turn_with_seed(&engine, &state, &actions, 11);
    assert_field_has_status(&next, "trick_room");
}

#[test]
fn p0_spec_field_status_non_stack_replaces_existing_copy() {
    let engine = make_engine(vec![
        field_status_move("set_reflect", "reflect"),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Setter")
                .moves(&["set_reflect"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Dummy").moves(&["wait"]).build()],
        ),
    ]);

    let turns = vec![
        vec![
            move_action("p1", "set_reflect", "p2"),
            move_action("p2", "wait", "p1"),
        ],
        vec![
            move_action("p1", "set_reflect", "p2"),
            move_action("p2", "wait", "p1"),
        ],
    ];
    let next = run_turns_with_seed(&engine, state, &turns, 111);

    assert_eq!(
        field_status_count(&next, "reflect"),
        1,
        "non-stack field status should keep only one copy after reapplication"
    );
}

#[test]
fn p0_spec_damage_roll_matches_golden_fixture() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["raw_damage"])
                .stats(100, 100, 50, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(100, 100, 50, 50, 60)
                .build()],
        ),
    ]);

    let move_data = damage_move("raw_damage", "physical", 100, None);
    let damage_step = effect("damage", json!({ "power": 100, "accuracy": 1.0 }));
    let type_chart = TypeChart::new();

    let mut low_roll_rng = {
        let mut seq = vec![0.0, 0.99, 0.0].into_iter();
        move || seq.next().unwrap_or(0.0)
    };
    let mut low_ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(&move_data),
        rng: &mut low_roll_rng,
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
    let low_events = apply_effects(&state, &[damage_step.clone()], &mut low_ctx);

    let mut high_roll_rng = {
        let mut seq = vec![0.0, 0.99, 0.999999].into_iter();
        move || seq.next().unwrap_or(0.0)
    };
    let mut high_ctx = EffectContext {
        attacker_player_id: "p1".to_string(),
        target_player_id: "p2".to_string(),
        move_data: Some(&move_data),
        rng: &mut high_roll_rng,
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
    let high_events = apply_effects(&state, &[damage_step], &mut high_ctx);

    let low_damage = first_damage_amount(&low_events);
    let high_damage = first_damage_amount(&high_events);

    // Expected with L50, power 100, atk 100, def 100, STAB=1.5:
    // base = 46, final = floor(69 * roll).
    assert_eq!(low_damage, 58, "roll=0.85 should yield floor(69*0.85)=58");
    assert_eq!(high_damage, 69, "roll=1.00 should yield floor(69*1.00)=69");
}

#[test]
fn p0_spec_trick_room_reverses_action_order() {
    let engine = make_engine(vec![damage_move("one_shot", "physical", 400, None)]);

    let mut state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Fast")
                .moves(&["one_shot"])
                .stats(80, 50, 50, 50, 200)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Slow")
                .moves(&["one_shot"])
                .stats(80, 50, 50, 50, 20)
                .build()],
        ),
    ]);
    state.field.global.push(FieldEffect {
        id: "trick_room".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let actions = vec![
        move_action("p1", "one_shot", "p2"),
        move_action("p2", "one_shot", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 5);

    assert_active_hp(&next, "p1", 0);
    assert_active_hp(&next, "p2", 100);
}

#[test]
fn p0_spec_priority_still_overrides_speed_order_under_trick_room() {
    let engine = make_engine(vec![
        damage_move_with_priority("quick_hit", "physical", 400, None, 1),
        damage_move("one_shot", "physical", 400, None),
    ]);

    let mut state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "FastPriority")
                .moves(&["quick_hit"])
                .stats(80, 50, 50, 50, 200)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "SlowNormal")
                .moves(&["one_shot"])
                .stats(80, 50, 50, 50, 20)
                .build()],
        ),
    ]);
    state.field.global.push(FieldEffect {
        id: "trick_room".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let actions = vec![
        move_action("p1", "quick_hit", "p2"),
        move_action("p2", "one_shot", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 6);

    assert_active_hp(&next, "p1", 100);
    assert_active_hp(&next, "p2", 0);
}

#[test]
fn p0_spec_sucker_punch_hits_when_target_selected_physical_or_special_move() {
    for category in ["physical", "special"] {
        let engine = make_engine(vec![
            sucker_punch_move(),
            damage_move("strike", category, 40, None),
        ]);
        let state = battle_state(vec![
            player(
                "p1",
                "P1",
                vec![CreatureBuilder::new("c1", "Sucker")
                    .moves(&["sucker_punch"])
                    .build()],
            ),
            player(
                "p2",
                "P2",
                vec![CreatureBuilder::new("c2", "Attacker")
                    .moves(&["strike"])
                    .build()],
            ),
        ]);

        let next = run_turn_with_seed(
            &engine,
            &state,
            &[
                move_action("p1", "sucker_punch", "p2"),
                move_action("p2", "strike", "p1"),
            ],
            7,
        );

        assert!(
            active_hp(&next, "p2") < 100,
            "sucker punch should hit when target selected {category}"
        );
    }
}

#[test]
fn p0_spec_sucker_punch_fails_when_target_selected_status_move() {
    let engine = make_engine(vec![sucker_punch_move(), wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Sucker")
                .moves(&["sucker_punch"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Status")
                .moves(&["wait"])
                .build()],
        ),
    ]);

    let next = run_turn_with_seed(
        &engine,
        &state,
        &[
            move_action("p1", "sucker_punch", "p2"),
            move_action("p2", "wait", "p1"),
        ],
        8,
    );

    assert_active_hp(&next, "p2", 100);
    assert!(
        next.log
            .iter()
            .any(|line| line.contains("しかし うまく きまらなかった")),
        "failed sucker punch should be logged"
    );
}

#[test]
fn p0_spec_prankster_status_move_fails_against_dark_target() {
    let engine = make_engine(vec![
        status_stage_move("scary_prank", "target", "atk", -1),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Prankster")
                .moves(&["scary_prank"])
                .ability("prankster")
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "DarkTarget")
                .types(&["dark"])
                .moves(&["wait"])
                .stats(50, 50, 50, 50, 200)
                .build()],
        ),
    ]);

    let next = run_turn_with_seed(
        &engine,
        &state,
        &[
            move_action("p1", "scary_prank", "p2"),
            move_action("p2", "wait", "p1"),
        ],
        9,
    );

    let dark_target = &next.players[1].team[0];
    assert_eq!(
        dark_target.stages.atk, 0,
        "prankster target status move should not affect dark targets"
    );
    assert!(
        next.log
            .iter()
            .any(|line| line.contains("しかし うまく 決まらなかった")),
        "blocked prankster move should log failure"
    );
}

#[test]
fn p0_spec_prankster_self_and_field_moves_still_work_against_dark_target() {
    let engine = make_engine(vec![
        status_stage_move("self_boost", "self", "atk", 1),
        field_status_move("set_spikes", "spikes"),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Prankster")
                .moves(&["self_boost", "set_spikes"])
                .ability("prankster")
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "DarkTarget")
                .types(&["dark"])
                .moves(&["wait"])
                .stats(50, 50, 50, 50, 200)
                .build()],
        ),
    ]);

    let after_boost = run_turn_with_seed(
        &engine,
        &state,
        &[
            move_action("p1", "self_boost", "p2"),
            move_action("p2", "wait", "p1"),
        ],
        10,
    );
    assert_eq!(
        after_boost.players[0].team[0].stages.atk, 1,
        "prankster self-target boost should still work"
    );

    let after_field = run_turn_with_seed(
        &engine,
        &state,
        &[
            move_action("p1", "set_spikes", "p2"),
            move_action("p2", "wait", "p1"),
        ],
        11,
    );
    assert_field_has_status(&after_field, "spikes");
}

#[test]
fn p0_spec_reflect_reduces_physical_damage() {
    let engine = make_engine(vec![
        damage_move("strike", "physical", 80, None),
        wait_move(),
    ]);

    let base_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["strike"])
                .stats(100, 50, 50, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(50, 100, 50, 50, 50)
                .build()],
        ),
    ]);

    let mut state_with_reflect = base_state.clone();
    state_with_reflect.field.global.push(FieldEffect {
        id: "reflect".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let actions = vec![
        move_action("p1", "strike", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let next_without_reflect = run_turn_with_seed(&engine, &base_state, &actions, 17);
    let next_with_reflect = run_turn_with_seed(&engine, &state_with_reflect, &actions, 17);

    assert!(
        active_hp(&next_with_reflect, "p2") > active_hp(&next_without_reflect, "p2"),
        "reflect should reduce incoming physical damage"
    );
}

#[test]
fn p0_spec_reflect_does_not_reduce_special_damage() {
    let engine = make_engine(vec![damage_move("beam", "special", 80, None), wait_move()]);

    let base_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["beam"])
                .stats(50, 50, 100, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(50, 50, 50, 100, 50)
                .build()],
        ),
    ]);
    let mut state_with_reflect = base_state.clone();
    state_with_reflect.field.global.push(FieldEffect {
        id: "reflect".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let actions = vec![
        move_action("p1", "beam", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let next_without_reflect = run_turn_with_seed(&engine, &base_state, &actions, 23);
    let next_with_reflect = run_turn_with_seed(&engine, &state_with_reflect, &actions, 23);

    assert_eq!(
        active_hp(&next_with_reflect, "p2"),
        active_hp(&next_without_reflect, "p2"),
        "reflect should not reduce special-category damage"
    );
}

#[test]
fn p0_spec_light_screen_reduces_special_damage() {
    let engine = make_engine(vec![damage_move("beam", "special", 80, None), wait_move()]);

    let base_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["beam"])
                .stats(50, 50, 100, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(50, 50, 50, 100, 50)
                .build()],
        ),
    ]);
    let mut state_with_screen = base_state.clone();
    state_with_screen.field.global.push(FieldEffect {
        id: "light_screen".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let actions = vec![
        move_action("p1", "beam", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let next_without_screen = run_turn_with_seed(&engine, &base_state, &actions, 19);
    let next_with_screen = run_turn_with_seed(&engine, &state_with_screen, &actions, 19);

    assert!(
        active_hp(&next_with_screen, "p2") > active_hp(&next_without_screen, "p2"),
        "light_screen should reduce incoming special damage"
    );
}

#[test]
fn p0_spec_light_screen_does_not_reduce_physical_damage() {
    let engine = make_engine(vec![
        damage_move("strike", "physical", 80, None),
        wait_move(),
    ]);

    let base_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["strike"])
                .stats(100, 50, 50, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .stats(50, 100, 50, 50, 50)
                .build()],
        ),
    ]);
    let mut state_with_screen = base_state.clone();
    state_with_screen.field.global.push(FieldEffect {
        id: "light_screen".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let actions = vec![
        move_action("p1", "strike", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let next_without_screen = run_turn_with_seed(&engine, &base_state, &actions, 29);
    let next_with_screen = run_turn_with_seed(&engine, &state_with_screen, &actions, 29);

    assert_eq!(
        active_hp(&next_with_screen, "p2"),
        active_hp(&next_without_screen, "p2"),
        "light_screen should not reduce physical-category damage"
    );
}

#[test]
fn p0_spec_tailwind_changes_action_order_by_speed() {
    let engine = make_engine(vec![damage_move("one_shot", "physical", 400, None)]);

    let mut state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Slow")
                .moves(&["one_shot"])
                .stats(80, 50, 50, 50, 40)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Fast")
                .moves(&["one_shot"])
                .stats(80, 50, 50, 50, 60)
                .build()],
        ),
    ]);
    state.field.sides.insert(
        "p1".to_string(),
        vec![FieldEffect {
            id: "tailwind".to_string(),
            remaining_turns: Some(4),
            data: HashMap::new(),
        }],
    );

    let actions = vec![
        move_action("p1", "one_shot", "p2"),
        move_action("p2", "one_shot", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 41);

    assert_active_hp(&next, "p1", 100);
    assert_active_hp(&next, "p2", 0);
}

#[test]
fn p0_spec_toxic_damage_scales_each_turn() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Poisoned")
                .moves(&["wait"])
                .hp(96, 96)
                .with_status(status("toxic", None))
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Idle").moves(&["wait"]).build()],
        ),
    ]);

    let turns = vec![
        vec![
            move_action("p1", "wait", "p2"),
            move_action("p2", "wait", "p1"),
        ],
        vec![
            move_action("p1", "wait", "p2"),
            move_action("p2", "wait", "p1"),
        ],
    ];
    let next = run_turns_with_seed(&engine, state, &turns, 13);

    assert_active_hp(&next, "p1", 78);
}

#[test]
fn p0_spec_toxic_resets_counter_after_switch() {
    let engine = make_engine(vec![wait_move()]);
    let initial = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "Poisoned")
                    .moves(&["wait"])
                    .hp(96, 96)
                    .with_status(status("toxic", None))
                    .build(),
                CreatureBuilder::new("c3", "Bench")
                    .moves(&["wait"])
                    .hp(96, 96)
                    .build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Idle").moves(&["wait"]).build()],
        ),
    ]);

    let turns = vec![
        vec![
            move_action("p1", "wait", "p2"),
            move_action("p2", "wait", "p1"),
        ],
        vec![switch_action("p1", 1), move_action("p2", "wait", "p1")],
        vec![switch_action("p1", 0), move_action("p2", "wait", "p1")],
    ];
    let next = run_turns_with_seed(&engine, initial, &turns, 101);

    assert_active_hp(&next, "p1", 84);
}

#[test]
fn p0_spec_toxic_counter_data_is_cleared_on_switch_out() {
    let engine = make_engine(vec![wait_move()]);
    let mut toxic = status("toxic", None);
    toxic
        .data
        .insert("counter".to_string(), Value::Number(4.into()));

    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "Poisoned")
                    .moves(&["wait"])
                    .with_status(toxic)
                    .build(),
                CreatureBuilder::new("c3", "Bench").moves(&["wait"]).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Idle").moves(&["wait"]).build()],
        ),
    ]);

    let actions = vec![switch_action("p1", 1), move_action("p2", "wait", "p1")];
    let next = run_turn_with_seed(&engine, &state, &actions, 115);
    let outgoing = &next.players[0].team[0];
    let toxic_status = outgoing
        .statuses
        .iter()
        .find(|status| status.id == "toxic")
        .expect("toxic should persist as non-volatile status");

    assert!(
        !toxic_status.data.contains_key("counter"),
        "toxic counter should be removed when switching out"
    );
}

#[test]
fn p0_spec_toxic_damage_has_minimum_of_one() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Fragile")
                .moves(&["wait"])
                .hp(3, 3)
                .with_status(status("toxic", None))
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Idle").moves(&["wait"]).build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "wait", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 116);

    assert_active_hp(&next, "p1", 2);
}

#[test]
fn p0_spec_protect_chain_probability_is_one_third_then_one_ninth() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![{
                let mut c = CreatureBuilder::new("c1", "Guard").moves(&["wait"]).build();
                c.volatile_data
                    .insert("protectSuccessCount".to_string(), Value::Number(1.into()));
                c
            }],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Dummy").moves(&["wait"]).build()],
        ),
    ]);

    let mut rng = || 0.4;
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
    let events = apply_effects(&state, &[effect("protect", json!({}))], &mut ctx);

    let reset_seen = events.iter().any(|event| match event {
        BattleEvent::SetVolatile { key, value, .. } => {
            key == "protectSuccessCount" && value == &Value::Number(0.into())
        }
        _ => false,
    });

    assert!(
        has_status_log(&events, "失敗") && reset_seen,
        "second chained protect at rng=0.4 should fail under 1/3 rule"
    );
}

#[test]
fn p0_spec_protect_chain_success_increments_counter() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![{
                let mut c = CreatureBuilder::new("c1", "Guard").moves(&["wait"]).build();
                c.volatile_data
                    .insert("protectSuccessCount".to_string(), Value::Number(1.into()));
                c
            }],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Dummy").moves(&["wait"]).build()],
        ),
    ]);

    let mut rng = || 0.2;
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
    let events = apply_effects(&state, &[effect("protect", json!({}))], &mut ctx);

    let next_counter = events.iter().find_map(|event| match event {
        BattleEvent::SetVolatile { key, value, .. } if key == "protectSuccessCount" => {
            value.as_i64()
        }
        _ => None,
    });
    let applied_protect = events.iter().any(|event| match event {
        BattleEvent::ApplyStatus { status_id, .. } => status_id == "protect",
        _ => false,
    });

    assert_eq!(
        next_counter,
        Some(2),
        "successful chained protect should increment success counter"
    );
    assert!(
        applied_protect,
        "successful protect should apply protect status"
    );
}

#[test]
fn p0_spec_non_protect_move_resets_protect_chain_counter() {
    let engine = make_engine(vec![
        damage_move("strike", "physical", 40, None),
        wait_move(),
    ]);
    let mut state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Guard")
                .moves(&["strike"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Dummy").moves(&["wait"]).build()],
        ),
    ]);
    state.players[0].team[0]
        .volatile_data
        .insert("protectSuccessCount".to_string(), Value::Number(2.into()));

    let actions = vec![
        move_action("p1", "strike", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 117);
    let counter = next.players[0].team[0]
        .volatile_data
        .get("protectSuccessCount")
        .and_then(|value| value.as_i64());

    assert_eq!(
        counter,
        Some(0),
        "using a non-protect move should reset protect chain counter"
    );
}

#[test]
fn p0_spec_protect_blocks_incoming_damage_when_used_first() {
    let engine = make_engine(vec![
        protect_move("protect_plus4", 4),
        damage_move("strike", "physical", 90, None),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Guard")
                .moves(&["protect_plus4"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Attacker")
                .moves(&["strike"])
                .stats(100, 50, 50, 50, 50)
                .build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "protect_plus4", "p2"),
        move_action("p2", "strike", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 119);

    assert_active_hp(&next, "p1", 100);
    assert!(
        next.log.iter().any(|line| line.contains("守った")),
        "protect resolution should be visible in battle log"
    );
}

#[test]
fn p0_spec_failed_protect_does_not_block_incoming_damage() {
    let engine = make_engine(vec![
        protect_move("protect_plus4", 4),
        damage_move("strike", "physical", 90, None),
    ]);
    let mut state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Guard")
                .moves(&["protect_plus4"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Attacker")
                .moves(&["strike"])
                .stats(100, 50, 50, 50, 50)
                .build()],
        ),
    ]);
    state.players[0].team[0]
        .volatile_data
        .insert("protectSuccessCount".to_string(), Value::Number(2.into()));

    let actions = vec![
        move_action("p1", "protect_plus4", "p2"),
        move_action("p2", "strike", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 0);

    assert!(
        active_hp(&next, "p1") < 100,
        "failed protect should allow incoming damage"
    );
    let counter = next.players[0].team[0]
        .volatile_data
        .get("protectSuccessCount")
        .and_then(|value| value.as_i64());
    assert_eq!(
        counter,
        Some(0),
        "failed protect should reset protectSuccessCount"
    );
}

#[test]
fn p0_spec_endure_survives_lethal_damage_at_one_hp() {
    let engine = make_engine(vec![
        endure_move("endure_plus4", 4),
        damage_move("one_shot", "physical", 400, None),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Endurer")
                .moves(&["endure_plus4"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Attacker")
                .moves(&["one_shot"])
                .stats(100, 50, 50, 50, 50)
                .build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "endure_plus4", "p2"),
        move_action("p2", "one_shot", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 120);

    assert_active_hp(&next, "p1", 1);
    assert!(
        next.log.iter().any(|line| line.contains("こらえた")),
        "endure should log that the user endured lethal damage"
    );
    assert!(
        !next.players[0].team[0]
            .statuses
            .iter()
            .any(|status| status.id == "pending_switch"),
        "endure should prevent fainting from lethal damage"
    );
}

#[test]
fn p0_spec_endure_does_not_block_non_damage_status_moves() {
    let engine = make_engine(vec![
        endure_move("endure_plus4", 4),
        status_move("poison_touch", "poison"),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Endurer")
                .moves(&["endure_plus4"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Poisoner")
                .moves(&["poison_touch"])
                .build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "endure_plus4", "p2"),
        move_action("p2", "poison_touch", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 121);

    assert_active_has_status(&next, "p1", "poison");
}

#[test]
fn p0_spec_endure_shares_protect_chain_counter() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![{
                let mut c = CreatureBuilder::new("c1", "Guard").moves(&["wait"]).build();
                c.volatile_data
                    .insert("protectSuccessCount".to_string(), Value::Number(1.into()));
                c
            }],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Dummy").moves(&["wait"]).build()],
        ),
    ]);

    let mut rng = || 0.4;
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
    let events = apply_effects(&state, &[effect("endure", json!({}))], &mut ctx);

    let reset_seen = events.iter().any(|event| match event {
        BattleEvent::SetVolatile { key, value, .. } => {
            key == "protectSuccessCount" && value == &Value::Number(0.into())
        }
        _ => false,
    });

    assert!(
        has_status_log(&events, "こらえられなかった") && reset_seen,
        "endure should use the same consecutive-use failure odds as protect"
    );
}

#[test]
fn p0_spec_sleep_persists_when_switched_out() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "Sleeper")
                    .moves(&["wait"])
                    .with_status(status("sleep", None))
                    .build(),
                CreatureBuilder::new("c3", "Reserve")
                    .moves(&["wait"])
                    .build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);

    let actions = vec![switch_action("p1", 1), move_action("p2", "wait", "p1")];
    let next = run_turn_with_seed(&engine, &state, &actions, 21);

    let outgoing = &next.players[0].team[0];
    assert!(
        outgoing.statuses.iter().any(|s| s.id == "sleep"),
        "sleep should remain on switched-out Pokemon"
    );
}

#[test]
fn p0_spec_sleep_wakes_and_allows_action_when_counter_reaches_zero() {
    let engine = make_engine(vec![
        damage_move("one_shot", "physical", 400, None),
        wait_move(),
    ]);
    let mut sleep = status("sleep", None);
    sleep
        .data
        .insert("turns".to_string(), Value::Number(1.into()));
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Sleeper")
                .moves(&["one_shot"])
                .with_status(sleep)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Target")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "one_shot", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 122);

    assert_active_hp(&next, "p2", 0);
    assert!(
        !next.players[0].team[next.players[0].active_slot]
            .statuses
            .iter()
            .any(|status| status.id == "sleep"),
        "sleep status should be removed after waking"
    );
}

#[test]
fn p0_spec_sleep_turn_counter_persists_through_switch() {
    let engine = make_engine(vec![wait_move()]);
    let mut sleep = status("sleep", None);
    sleep
        .data
        .insert("turns".to_string(), Value::Number(2.into()));
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "Sleeper")
                    .moves(&["wait"])
                    .with_status(sleep)
                    .build(),
                CreatureBuilder::new("c3", "Reserve")
                    .moves(&["wait"])
                    .build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);

    let actions = vec![switch_action("p1", 1), move_action("p2", "wait", "p1")];
    let next = run_turn_with_seed(&engine, &state, &actions, 121);
    let outgoing = &next.players[0].team[0];
    let turns = outgoing
        .statuses
        .iter()
        .find(|status| status.id == "sleep")
        .and_then(|status| status.data.get("turns"))
        .and_then(|value| value.as_i64());

    assert_eq!(
        turns,
        Some(2),
        "sleep turn counter should be preserved while switched out"
    );
}

#[test]
fn p0_spec_switch_clears_volatile_data_and_stages_while_preserving_non_volatile_status() {
    let engine = make_engine(vec![wait_move()]);

    let mut active = CreatureBuilder::new("c1", "SetupMon")
        .moves(&["wait"])
        .ability("copied_ability")
        .with_status(status("burn", None))
        .with_status(status("protect", Some(1)))
        .build();
    active.stages.atk = 4;
    active
        .volatile_data
        .insert("protectSuccessCount".to_string(), Value::Number(2.into()));
    active.ability_data.insert(
        "originalAbility".to_string(),
        Value::String("original_ability".to_string()),
    );

    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                active,
                CreatureBuilder::new("c3", "Reserve")
                    .moves(&["wait"])
                    .build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);

    let actions = vec![switch_action("p1", 1), move_action("p2", "wait", "p1")];
    let next = run_turn_with_seed(&engine, &state, &actions, 33);
    let outgoing = &next.players[0].team[0];

    assert_eq!(
        outgoing.stages.atk, 0,
        "switch-out should clear stat stages"
    );
    assert!(
        outgoing.volatile_data.is_empty(),
        "switch-out should clear volatile_data"
    );
    assert!(
        outgoing.ability_data.is_empty(),
        "switch-out should clear temporary ability_data"
    );
    assert_eq!(
        outgoing.ability.as_deref(),
        Some("original_ability"),
        "switch-out should restore original ability when tracked"
    );
    assert!(
        outgoing.statuses.iter().any(|s| s.id == "burn"),
        "non-volatile status should persist through switch-out"
    );
    assert!(
        !outgoing.statuses.iter().any(|s| s.id == "protect"),
        "volatile status should be removed on switch-out"
    );
}

#[test]
fn p0_manual_effects_must_not_be_silent_noop() {
    let move_db =
        MoveDatabase::load_from_yaml_file(Path::new("data/moves.yaml")).expect("load moves.yaml");

    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "ManualUser")
                .moves(&["wait"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Target")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let type_chart = TypeChart::new();

    let mut offenders = Vec::new();
    for (move_id, move_data) in move_db.as_map() {
        walk_effects(&move_data.steps, &mut |effect| {
            if effect.effect_type != "manual" {
                return;
            }
            let manual_effect = effect.clone();
            let mut rng = || 0.0;
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
            let events = apply_effects(&state, &[manual_effect], &mut ctx);
            if events.is_empty() {
                offenders.push(move_id.to_string());
            }
        });
    }

    assert!(
        offenders.is_empty(),
        "manual effects produced no runtime events:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn p0_spec_simultaneous_faint_resolution_rule() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Fast")
                .hp(0, 100)
                .stats(50, 50, 50, 50, 120)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Slow")
                .hp(0, 100)
                .stats(50, 50, 50, 50, 40)
                .build()],
        ),
    ]);
    let winner = determine_winner(&state);
    assert_eq!(
        winner.as_deref(),
        Some("p2"),
        "faster side should faint first and lose in simultaneous-faint resolution"
    );

    let mut trick_room_state = state.clone();
    trick_room_state.field.global.push(FieldEffect {
        id: "trick_room".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let trick_room_winner = determine_winner(&trick_room_state);
    assert_eq!(
        trick_room_winner.as_deref(),
        Some("p1"),
        "under trick room, slower side should faint first and lose"
    );
}

#[test]
fn p0_spec_simultaneous_faint_speed_tie_is_draw() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "EqualA")
                .hp(0, 100)
                .stats(50, 50, 50, 50, 80)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "EqualB")
                .hp(0, 100)
                .stats(50, 50, 50, 50, 80)
                .build()],
        ),
    ]);

    assert_eq!(
        determine_winner(&state),
        None,
        "simultaneous faint with equal speed should resolve to draw"
    );
}

#[test]
fn p0_spec_timeout_resolution_rule() {
    let alive_count_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "A1").hp(50, 100).build(),
                CreatureBuilder::new("c3", "A2").hp(1, 100).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![
                CreatureBuilder::new("c2", "B1").hp(99, 100).build(),
                CreatureBuilder::new("c4", "B2").hp(0, 100).build(),
            ],
        ),
    ]);
    assert_eq!(
        determine_timeout_winner(&alive_count_state).as_deref(),
        Some("p1"),
        "timeout should prioritize remaining Pokemon count"
    );

    let hp_ratio_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "A1").hp(40, 100).build(),
                CreatureBuilder::new("c3", "A2").hp(0, 100).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![
                CreatureBuilder::new("c2", "B1").hp(39, 100).build(),
                CreatureBuilder::new("c4", "B2").hp(0, 100).build(),
            ],
        ),
    ]);
    assert_eq!(
        determine_timeout_winner(&hp_ratio_state).as_deref(),
        Some("p1"),
        "when alive count ties, timeout should compare HP percentage"
    );
}

#[test]
fn p0_spec_timeout_uses_total_hp_as_final_tiebreaker() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "A1").hp(40, 100).build(),
                CreatureBuilder::new("c3", "A2").hp(0, 50).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![
                CreatureBuilder::new("c2", "B1").hp(80, 200).build(),
                CreatureBuilder::new("c4", "B2").hp(0, 100).build(),
            ],
        ),
    ]);

    assert_eq!(
        determine_timeout_winner(&state).as_deref(),
        Some("p2"),
        "when alive count and HP ratio tie, timeout should compare total HP"
    );
}

#[test]
fn p0_spec_timeout_returns_none_on_exact_tie() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "A1").hp(40, 100).build(),
                CreatureBuilder::new("c3", "A2").hp(0, 100).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![
                CreatureBuilder::new("c2", "B1").hp(40, 100).build(),
                CreatureBuilder::new("c4", "B2").hp(0, 100).build(),
            ],
        ),
    ]);

    assert_eq!(
        determine_timeout_winner(&state),
        None,
        "exact timeout score tie should resolve to no winner"
    );
}

#[test]
fn p0_spec_winner_is_alive_side_when_only_one_side_has_remaining_creatures() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "AliveA").hp(1, 100).build(),
                CreatureBuilder::new("c3", "AliveB").hp(0, 100).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![
                CreatureBuilder::new("c2", "FaintedA").hp(0, 100).build(),
                CreatureBuilder::new("c4", "FaintedB").hp(0, 100).build(),
            ],
        ),
    ]);

    assert_eq!(
        determine_winner(&state).as_deref(),
        Some("p1"),
        "winner should be the only side with at least one living creature"
    );
}

#[test]
fn p1_spec_learnset_moves_must_exist_in_move_db() {
    let move_db =
        MoveDatabase::load_from_yaml_file(Path::new("data/moves.yaml")).expect("load moves.yaml");
    let learnsets = LearnsetDatabase::load_default().expect("load learnsets");

    let mut missing = Vec::new();
    for (species_id, moves) in learnsets.as_map() {
        for move_id in moves {
            if move_db.get(move_id).is_none() {
                missing.push(format!("{} -> {}", species_id, move_id));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "undefined move ids in learnsets:\n{}",
        missing.join("\n")
    );
}

#[test]
fn p1_spec_effect_targets_use_supported_literals() {
    let move_db =
        MoveDatabase::load_from_yaml_file(Path::new("data/moves.yaml")).expect("load moves.yaml");

    let mut invalid = Vec::new();
    for (move_id, move_data) in move_db.as_map() {
        walk_effects(&move_data.steps, &mut |effect| {
            if let Some(target) = effect.data.get("target").and_then(|v| v.as_str()) {
                if !is_allowed_target_literal(target) {
                    invalid.push(format!("{} -> {}", move_id, target));
                }
            }
        });
    }

    assert!(
        invalid.is_empty(),
        "unsupported target literals found:\n{}",
        invalid.join("\n")
    );
}

#[test]
fn p1_spec_effect_status_ids_use_supported_canonical_ids() {
    let move_db =
        MoveDatabase::load_from_yaml_file(Path::new("data/moves.yaml")).expect("load moves.yaml");

    let mut invalid = Vec::new();
    for (move_id, move_data) in move_db.as_map() {
        walk_effects(&move_data.steps, &mut |effect| {
            if let Some(status_id) = effect.data.get("statusId").and_then(|v| v.as_str()) {
                if !is_supported_status_id(status_id) {
                    invalid.push(format!("{} -> {}", move_id, status_id));
                }
            }
        });
    }

    assert!(
        invalid.is_empty(),
        "unsupported status ids found:\n{}",
        invalid.join("\n")
    );
}

#[test]
fn p1_spec_ability_status_field_interaction_matrix() {
    let poison_move = MoveData {
        id: "poison_touch".to_string(),
        name: Some("Poison Touch".to_string()),
        move_type: Some("poison".to_string()),
        category: Some("status".to_string()),
        pp: Some(10),
        power: None,
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect(
            "apply_status",
            json!({
                "statusId": "poison",
                "target": "target",
                "chance": 1
            }),
        )],
        tags: Vec::new(),
        crit_rate: None,
    };
    let engine = make_engine(vec![
        wait_move(),
        poison_move,
        damage_move("strike", "physical", 80, None),
    ]);

    let mut immunity_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "ImmunityMon")
                .moves(&["wait", "strike"])
                .ability("immunity")
                .hp(80, 100)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Poisoner")
                .moves(&["poison_touch"])
                .build()],
        ),
    ]);
    immunity_state.field.global.push(FieldEffect {
        id: "grassy_terrain".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let immunity_actions = vec![
        move_action("p1", "wait", "p2"),
        move_action("p2", "poison_touch", "p1"),
    ];
    let immunity_next = run_turn_with_seed(&engine, &immunity_state, &immunity_actions, 301);
    assert_active_hp(&immunity_next, "p1", 86);
    let immunity_active = &immunity_next.players[0].team[immunity_next.players[0].active_slot];
    assert!(
        !immunity_active.statuses.iter().any(|s| s.id == "poison"),
        "immunity should block poison while grassy terrain still heals"
    );

    let mut normal_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "NormalMon")
                .moves(&["wait", "strike"])
                .hp(80, 100)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Poisoner")
                .moves(&["poison_touch"])
                .build()],
        ),
    ]);
    normal_state.field.global.push(FieldEffect {
        id: "grassy_terrain".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let normal_next = run_turn_with_seed(&engine, &normal_state, &immunity_actions, 301);
    assert_active_has_status(&normal_next, "p1", "poison");
    assert_active_hp(&normal_next, "p1", 74);

    let mut levitate_state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "LevitateMon")
                .moves(&["wait"])
                .ability("levitate")
                .hp(80, 100)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Idle").moves(&["wait"]).build()],
        ),
    ]);
    levitate_state.field.global.push(FieldEffect {
        id: "grassy_terrain".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let wait_actions = vec![
        move_action("p1", "wait", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let levitate_next = run_turn_with_seed(&engine, &levitate_state, &wait_actions, 302);
    assert_active_hp(&levitate_next, "p1", 80);

    let mut guts_base = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "GutsMon")
                .moves(&["strike"])
                .ability("guts")
                .stats(100, 60, 50, 50, 70)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Wall")
                .moves(&["wait"])
                .stats(60, 100, 50, 70, 50)
                .build()],
        ),
    ]);
    guts_base.field.global.push(FieldEffect {
        id: "reflect".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let mut guts_burned = guts_base.clone();
    guts_burned.players[0].team[0]
        .statuses
        .push(status("burn", None));
    let strike_actions = vec![
        move_action("p1", "strike", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let base_next = run_turn_with_seed(&engine, &guts_base, &strike_actions, 303);
    let burned_next = run_turn_with_seed(&engine, &guts_burned, &strike_actions, 303);
    assert!(
        active_hp(&burned_next, "p2") < active_hp(&base_next, "p2"),
        "guts + status should increase physical damage output even when reflect is active"
    );
}

#[test]
fn p1_spec_end_turn_effect_ordering() {
    let engine = make_engine(vec![wait_move()]);

    let mut wish_status = status("wish", None);
    wish_status
        .data
        .insert("triggerTurn".to_string(), Value::Number(1.into()));
    wish_status
        .data
        .insert("healAmount".to_string(), Value::Number(20.into()));

    let leftovers_status = status("leftovers", None);

    let mut leech_seed = status("leech_seed", None);
    leech_seed
        .data
        .insert("sourceId".to_string(), Value::String("p2".to_string()));

    let poison_status = status("poison", None);

    let mut bind_status = status("bind", None);
    bind_status.data.insert(
        "moveName".to_string(),
        Value::String("しめつける".to_string()),
    );

    let curse_status = status("curse", None);

    let mut state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Anchor")
                .moves(&["wait"])
                .hp(70, 100)
                .with_status(wish_status)
                .with_status(leftovers_status)
                .with_status(leech_seed)
                .with_status(poison_status)
                .with_status(bind_status)
                .with_status(curse_status)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Seeder")
                .moves(&["wait"])
                .hp(70, 100)
                .build()],
        ),
    ]);
    state.field.global.push(FieldEffect {
        id: "grassy_terrain".to_string(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });

    let actions = vec![
        move_action("p1", "wait", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 401);

    assert_active_hp(&next, "p1", 39);
    assert_active_hp(&next, "p2", 88);

    let find_log_index = |needle: &str| {
        next.log
            .iter()
            .position(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("log entry containing '{}' was not found", needle))
    };
    let wish_i = find_log_index("ねがいごとが かなった");
    let grassy_i = find_log_index("グラスフィールドの 恩恵を 受けている");
    let leftovers_i = find_log_index("たべのこしで 少し回復した");
    let leech_i = find_log_index("宿り木の種が");
    let poison_i = find_log_index("どくの ダメージを 受けている");
    let bind_i = find_log_index("しめつけるの ダメージを受けている");
    let curse_i = find_log_index("呪われている");

    assert!(
        wish_i < grassy_i
            && grassy_i < leftovers_i
            && leftovers_i < leech_i
            && leech_i < poison_i
            && poison_i < bind_i
            && bind_i < curse_i,
        "end-turn effects should resolve in the documented order"
    );
}

#[test]
fn p1_spec_manual_reason_uses_approved_taxonomy() {
    let move_db =
        MoveDatabase::load_from_yaml_file(Path::new("data/moves.yaml")).expect("load moves.yaml");
    let allowed_prefixes = [
        "Switching",
        "No supported effects inferred",
        "Multi-turn trapping/binding effects are not fully supported",
        "Unsupported ailment",
    ];

    let mut invalid = Vec::new();
    for (move_id, move_data) in move_db.as_map() {
        walk_effects(&move_data.steps, &mut |effect| {
            if effect.effect_type != "manual" {
                return;
            }
            let reason = effect
                .data
                .get("manualReason")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !allowed_prefixes
                .iter()
                .any(|prefix| reason.starts_with(prefix))
            {
                invalid.push(format!("{} -> {}", move_id, reason));
            }
        });
    }

    assert!(
        invalid.is_empty(),
        "manualReason out of taxonomy:\n{}",
        invalid.join("\n")
    );
}

#[test]
fn p1_spec_tailwind_only_boosts_the_owner_side() {
    let engine = make_engine(vec![damage_move("one_shot", "physical", 400, None)]);
    let mut state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Slow")
                .moves(&["one_shot"])
                .stats(80, 50, 50, 50, 40)
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Fast")
                .moves(&["one_shot"])
                .stats(80, 50, 50, 50, 60)
                .build()],
        ),
    ]);
    state.field.sides.insert(
        "p2".to_string(),
        vec![FieldEffect {
            id: "tailwind".to_string(),
            remaining_turns: Some(4),
            data: HashMap::new(),
        }],
    );

    let actions = vec![
        move_action("p1", "one_shot", "p2"),
        move_action("p2", "one_shot", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 4011);

    assert_active_hp(&next, "p1", 0);
    assert_active_hp(&next, "p2", 100);
}

#[test]
fn p1_spec_action_without_target_defaults_to_opponent() {
    let engine = make_engine(vec![
        damage_move("one_shot", "physical", 400, None),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["one_shot"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: Some("one_shot".to_string()),
            target_id: None,
            slot: None,
            priority: None,
        },
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 4012);

    assert_active_hp(&next, "p2", 0);
}

#[test]
fn p1_spec_move_priorities_remain_within_supported_bounds() {
    let move_db =
        MoveDatabase::load_from_yaml_file(Path::new("data/moves.yaml")).expect("load moves.yaml");

    let mut invalid = Vec::new();
    for (move_id, move_data) in move_db.as_map() {
        let priority = move_data.priority.unwrap_or(0);
        if !(-7..=5).contains(&priority) {
            invalid.push(format!("{} -> {}", move_id, priority));
        }
    }

    assert!(
        invalid.is_empty(),
        "move priorities are out of supported bounds (-7..=5):\n{}",
        invalid.join("\n")
    );
}

#[test]
fn p1_spec_manual_effects_have_non_empty_reason() {
    let move_db =
        MoveDatabase::load_from_yaml_file(Path::new("data/moves.yaml")).expect("load moves.yaml");

    let mut invalid = Vec::new();
    for (move_id, move_data) in move_db.as_map() {
        walk_effects(&move_data.steps, &mut |effect| {
            if effect.effect_type != "manual" {
                return;
            }
            let reason = effect
                .data
                .get("manualReason")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            if reason.is_empty() {
                invalid.push(move_id.to_string());
            }
        });
    }

    assert!(
        invalid.is_empty(),
        "manual effects must include a non-empty manualReason:\n{}",
        invalid.join("\n")
    );
}

#[test]
fn p1_spec_switch_to_active_slot_is_rejected() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "Lead").moves(&["wait"]).build(),
                CreatureBuilder::new("c3", "Bench").moves(&["wait"]).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![switch_action("p1", 0), move_action("p2", "wait", "p1")];
    let next = run_turn_with_seed(&engine, &state, &actions, 4101);

    assert_eq!(
        next.players[0].active_slot, 0,
        "switching to active slot should not change active slot"
    );
    assert!(
        next.log.iter().any(|line| line.contains("active slot")),
        "active-slot switch rejection should be logged"
    );
}

#[test]
fn p1_spec_switch_without_slot_is_rejected() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Lead").moves(&["wait"]).build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Switch,
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 4102);

    assert!(
        next.log.iter().any(|line| line.contains("without a slot")),
        "switch without slot should be logged and rejected"
    );
}

#[test]
fn p1_spec_use_item_without_item_is_rejected() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "NoItem")
                .moves(&["wait"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::UseItem,
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 4103);

    assert!(
        next.log
            .iter()
            .any(|line| line.contains("使う道具を 持っていない")),
        "use-item without item should be rejected with log"
    );
}

#[test]
fn p1_spec_use_item_with_item_emits_use_log() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "WithItem")
                .moves(&["wait"])
                .item("potion")
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::UseItem,
            move_id: None,
            target_id: None,
            slot: None,
            priority: None,
        },
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 4104);

    assert!(
        next.log.iter().any(|line| line.contains("道具を使った")),
        "use-item with held item should emit item-use log"
    );
}

#[test]
fn p2_case_registry_integrity() {
    let mut seen = HashSet::new();
    for case in CASES {
        assert!(seen.insert(case.id), "duplicate case id: {}", case.id);
    }

    assert!(
        CASES.iter().any(|case| case.priority == Priority::P0),
        "registry must include P0 cases"
    );
    assert!(
        CASES.iter().any(|case| case.priority == Priority::P1),
        "registry must include P1 cases"
    );
    assert!(
        CASES.iter().any(|case| case.priority == Priority::P2),
        "registry must include P2 cases"
    );
    assert!(
        CASES.iter().any(|case| case.priority == Priority::P3),
        "registry must include P3 cases"
    );
    assert!(
        CASES.iter().any(|case| case.enabled),
        "registry must include enabled cases"
    );
    assert!(
        CASES.iter().any(|case| !case.enabled) || CASES.iter().all(|case| case.enabled),
        "registry should either keep backlog cases or mark all cases enabled"
    );
}

#[test]
fn p2_case_registry_is_synced_with_markdown_table() {
    let doc = include_str!("P0_P2_TEST_CASES.md");
    for case in CASES {
        assert!(
            doc.contains(case.id),
            "missing case id '{}' in P0_P2_TEST_CASES.md",
            case.id
        );
    }
}

#[test]
fn p2_case_id_prefix_matches_priority_bucket() {
    for case in CASES {
        let expected_prefix = match case.priority {
            Priority::P0 => "P0-",
            Priority::P1 => "P1-",
            Priority::P2 => "P2-",
            Priority::P3 => "P3-",
        };
        assert!(
            case.id.starts_with(expected_prefix),
            "case '{}' should start with '{}'",
            case.id,
            expected_prefix
        );
    }
}

#[test]
fn p2_markdown_table_case_ids_are_unique() {
    let doc = include_str!("P0_P2_TEST_CASES.md");
    let case_ids = markdown_case_ids(doc);
    let mut seen = HashSet::new();
    for case_id in case_ids {
        assert!(
            seen.insert(case_id.clone()),
            "duplicate case id '{}' in markdown table",
            case_id
        );
    }
}

#[test]
fn p2_case_registry_and_markdown_table_are_bidirectionally_synced() {
    let doc = include_str!("P0_P2_TEST_CASES.md");
    let doc_ids: HashSet<String> = markdown_case_ids(doc).into_iter().collect();
    let registry_ids: HashSet<String> = CASES.iter().map(|case| case.id.to_string()).collect();

    let missing_in_doc: Vec<String> = registry_ids.difference(&doc_ids).cloned().collect();
    let missing_in_registry: Vec<String> = doc_ids.difference(&registry_ids).cloned().collect();

    assert!(
        missing_in_doc.is_empty(),
        "registry ids missing in markdown:\n{}",
        missing_in_doc.join("\n")
    );
    assert!(
        missing_in_registry.is_empty(),
        "markdown ids missing in registry:\n{}",
        missing_in_registry.join("\n")
    );
}

#[test]
fn p2_markdown_table_test_names_exist_in_source() {
    let doc = include_str!("P0_P2_TEST_CASES.md");
    let source = include_str!("spec_priority_cases.rs");

    for test_name in markdown_test_names(doc) {
        let signature = format!("fn {}(", test_name);
        assert!(
            source.contains(&signature),
            "test function '{}' listed in markdown but missing in source",
            test_name
        );
    }
}

#[test]
fn p2_markdown_table_row_count_matches_case_registry() {
    let doc = include_str!("P0_P2_TEST_CASES.md");
    let rows = markdown_case_rows(doc);
    assert_eq!(
        rows.len(),
        CASES.len(),
        "markdown table row count should match CASES registry length"
    );
}

#[test]
fn p2_markdown_priority_column_matches_case_id_prefix() {
    let doc = include_str!("P0_P2_TEST_CASES.md");
    for (case_id, priority_col, _) in markdown_case_rows(doc) {
        let id_prefix = case_id.split('-').next().unwrap_or_default().to_string();
        assert_eq!(
            priority_col, id_prefix,
            "priority column should match case id prefix for '{}'",
            case_id
        );
    }
}

#[test]
fn p2_spec_double_battle_model_smoke() {
    let chip = MoveData {
        id: "chip".to_string(),
        name: Some("Chip".to_string()),
        move_type: Some("normal".to_string()),
        category: Some("physical".to_string()),
        pp: Some(10),
        power: None,
        accuracy: Some(1.0),
        priority: Some(0),
        description: None,
        steps: vec![effect("damage_ratio", json!({ "ratioMaxHp": 0.25 }))],
        tags: Vec::new(),
        crit_rate: None,
    };
    let engine = make_engine(vec![chip, wait_move()]);

    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["chip"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .build()],
        ),
    ]);

    // P2 smoke: if a doubles-like action list provides two actions for one side,
    // single-battle mode must not let the same active creature act twice.
    let actions = vec![
        move_action("p1", "chip", "p2"),
        move_action("p1", "chip", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 551);

    assert_active_hp(&next, "p2", 75);
    assert!(
        next.log
            .iter()
            .any(|line| line.contains("追加アクション") && line.contains("無視")),
        "engine should log that duplicate per-player actions are ignored in single-battle mode"
    );
}

#[test]
fn p3_spec_same_seed_produces_identical_battle_state() {
    let engine = make_engine(vec![
        damage_move("strike", "physical", 80, None),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["strike"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "strike", "p2"),
        move_action("p2", "wait", "p1"),
    ];

    let next_a = run_turn_with_seed(&engine, &state, &actions, 9991);
    let next_b = run_turn_with_seed(&engine, &state, &actions, 9991);

    assert_no_diffs(&next_a, &next_b);
    assert_eq!(
        next_a.log, next_b.log,
        "battle log should also be deterministic"
    );
}

#[test]
fn p3_spec_unknown_move_is_logged_and_skipped() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["ghost_move"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "ghost_move", "p2"),
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 9992);

    assert_active_hp(&next, "p1", 100);
    assert_active_hp(&next, "p2", 100);
    assert!(
        next.log
            .iter()
            .any(|line| line.contains("tried unknown move ghost_move")),
        "unknown move should be logged and skipped"
    );
}

#[test]
fn p3_spec_invalid_switch_slot_is_rejected_without_changing_active_slot() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "Lead").moves(&["wait"]).build(),
                CreatureBuilder::new("c3", "Bench").moves(&["wait"]).build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![switch_action("p1", 99), move_action("p2", "wait", "p1")];
    let next = run_turn_with_seed(&engine, &state, &actions, 9993);

    assert_eq!(
        next.players[0].active_slot, 0,
        "invalid switch slot should not change active slot"
    );
    assert!(
        next.log.iter().any(|line| line.contains("invalid slot")),
        "invalid switch should be logged"
    );
}

#[test]
fn p3_spec_switch_to_fainted_slot_is_rejected_without_changing_active_slot() {
    let engine = make_engine(vec![wait_move()]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![
                CreatureBuilder::new("c1", "Lead").moves(&["wait"]).build(),
                CreatureBuilder::new("c3", "FaintedBench")
                    .moves(&["wait"])
                    .hp(0, 100)
                    .build(),
            ],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Opponent")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![switch_action("p1", 1), move_action("p2", "wait", "p1")];
    let next = run_turn_with_seed(&engine, &state, &actions, 9994);

    assert_eq!(
        next.players[0].active_slot, 0,
        "switching to a fainted slot should be rejected"
    );
    assert!(
        next.log.iter().any(|line| line.contains("fainted Pokémon")),
        "fainted-slot switch rejection should be logged"
    );
}

#[test]
fn p3_spec_timeout_winner_is_none_when_player_count_is_not_two() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Solo").hp(10, 100).build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Solo").hp(10, 100).build()],
        ),
        player(
            "p3",
            "P3",
            vec![CreatureBuilder::new("c3", "Solo").hp(10, 100).build()],
        ),
    ]);

    assert_eq!(
        determine_timeout_winner(&state),
        None,
        "timeout winner is only defined for two-player battles"
    );
}

#[test]
fn p3_spec_determine_winner_returns_none_for_three_way_all_faint() {
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Fainted").hp(0, 100).build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Fainted").hp(0, 100).build()],
        ),
        player(
            "p3",
            "P3",
            vec![CreatureBuilder::new("c3", "Fainted").hp(0, 100).build()],
        ),
    ]);

    assert_eq!(
        determine_winner(&state),
        None,
        "three-way all-faint should not use two-player simultaneous-faint fallback"
    );
}

#[test]
fn p3_spec_unknown_player_action_is_skipped_without_affecting_valid_actions() {
    let engine = make_engine(vec![
        damage_move("one_shot", "physical", 400, None),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["one_shot"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        move_action("p1", "one_shot", "p2"),
        move_action("ghost", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 9995);

    assert_active_hp(&next, "p2", 0);
    assert_active_hp(&next, "p1", 100);
}

#[test]
fn p3_spec_missing_move_id_is_logged_and_skipped() {
    let engine = make_engine(vec![
        damage_move("one_shot", "physical", 400, None),
        wait_move(),
    ]);
    let state = battle_state(vec![
        player(
            "p1",
            "P1",
            vec![CreatureBuilder::new("c1", "Attacker")
                .moves(&["one_shot"])
                .build()],
        ),
        player(
            "p2",
            "P2",
            vec![CreatureBuilder::new("c2", "Defender")
                .moves(&["wait"])
                .build()],
        ),
    ]);
    let actions = vec![
        Action {
            player_id: "p1".to_string(),
            action_type: ActionType::Move,
            move_id: None,
            target_id: Some("p2".to_string()),
            slot: None,
            priority: None,
        },
        move_action("p2", "wait", "p1"),
    ];
    let next = run_turn_with_seed(&engine, &state, &actions, 9996);

    assert_active_hp(&next, "p2", 100);
    assert!(
        next.log
            .iter()
            .any(|line| line.contains("has no move selected")),
        "missing move id should be logged and skipped"
    );
}
