use engine_rust::core::{
    abilities::{
        apply_ability_event_modifiers, effective_ability, refresh_gas, run_ability_value_hook,
        AbilityValueContext,
    },
    battle::{BattleEngine, BattleOptions},
    events::{apply_event, meta_with_move_source, BattleEvent},
    factory::{calc_stat, create_creature, CreateCreatureOptions},
    kamishinka::{self, available_forms},
    state::{
        create_battle_state, Action, ActionType, BattleState, CreatureState, EVStats, FieldEffect,
        PlayerState, Status,
    },
};
use engine_rust::data::{
    learnsets::LearnsetDatabase, moves::MoveDatabase, species::SpeciesDatabase,
    type_chart::TypeChart,
};
use serde_json::{json, Map};
use std::collections::HashMap;

fn creature(id: &str) -> CreatureState {
    create_creature(
        SpeciesDatabase::load_default().unwrap().get(id).unwrap(),
        CreateCreatureOptions {
            moves: Some(vec!["tackle".into()]),
            item: Some("kamidama".into()),
            ability: Some("none".into()),
            evs: Some(EVStats {
                atk: 20,
                spa: 20,
                spe: 26,
                ..Default::default()
            }),
            ..Default::default()
        },
        &LearnsetDatabase::load_default().unwrap(),
        &MoveDatabase::load_default().unwrap(),
    )
    .unwrap()
}
fn player(id: &str, species: &[&str]) -> PlayerState {
    PlayerState {
        id: id.into(),
        name: id.into(),
        team: species.iter().map(|id| creature(id)).collect(),
        active_slot: 0,
        last_fainted_ability: None,
        kamishinka_used: false,
        fainted_count: 0,
    }
}
fn state(a: &[&str], b: &[&str]) -> BattleState {
    create_battle_state(vec![player("p1", a), player("p2", b)])
}
fn action(player: &str, form: Option<&str>) -> Action {
    Action {
        player_id: player.into(),
        action_type: ActionType::Move,
        move_id: Some("tackle".into()),
        target_id: Some(if player == "p1" { "p2" } else { "p1" }.into()),
        slot: None,
        priority: None,
        kamishinka_form: form.map(str::to_string),
    }
}
fn engine() -> BattleEngine {
    BattleEngine::new(MoveDatabase::load_default().unwrap(), TypeChart::new())
}
fn step(s: &BattleState, actions: &[Action]) -> BattleState {
    engine().step_battle(s, actions, &mut || 0.5, BattleOptions::default())
}
fn switch(player: &str, slot: usize) -> Action {
    Action {
        action_type: ActionType::Switch,
        slot: Some(slot),
        move_id: None,
        ..action(player, None)
    }
}

#[test]
fn all_eight_registered_forms_apply_exact_stats_without_touching_hp_or_state() {
    for form in kamishinka::forms() {
        let mut s = state(&[&form.base_species_id], &["futo"]);
        let c = &mut s.players[0].team[0];
        c.hp -= 30;
        c.stages.atk = 2;
        c.statuses.push(Status {
            id: "paralysis".into(),
            remaining_turns: None,
            data: HashMap::new(),
        });
        c.move_pp.insert("tackle".into(), 12);
        c.volatile_data.insert("customState".into(), json!(42));
        let old = c.clone();
        assert!(kamishinka::apply(
            &mut s,
            &action("p1", Some(&form.id)),
            &engine().move_db
        ));
        let c = &s.players[0].team[0];
        assert_eq!((c.hp, c.max_hp), (old.hp, old.max_hp));
        assert_eq!(c.name, form.name);
        assert_eq!(c.types, form.types);
        assert_eq!(c.ability.as_deref(), Some(form.ability.as_str()));
        assert_eq!(c.stages.atk, 2);
        assert_eq!(c.statuses[0].id, "paralysis");
        assert_eq!(c.move_pp, old.move_pp);
        assert_eq!(c.volatile_data, old.volatile_data);
        assert_eq!(c.item.as_deref(), Some("kamidama"));
        for (value, base, ev) in [
            (c.attack, form.base_stats.atk, c.evs.atk),
            (c.defense, form.base_stats.def, c.evs.def),
            (c.sp_attack, form.base_stats.spa, c.evs.spa),
            (c.sp_defense, form.base_stats.spd, c.evs.spd),
            (c.speed, form.base_stats.spe, c.evs.spe),
        ] {
            assert_eq!(value, calc_stat(base, false, 50, 31, ev));
        }
        assert!(s.players[0].kamishinka_used);
        assert_ne!(
            SpeciesDatabase::load_default()
                .unwrap()
                .get(&form.base_species_id)
                .unwrap()
                .name,
            form.name
        );
    }
}
#[test]
fn item_species_and_request_validation_do_not_spend_right() {
    for (id, item, form) in [
        ("nisiki", None, "kami_nisiki"),
        ("futo", Some("kamidama"), "kami_nisiki"),
        ("nisiki", Some("kamidama"), "kami_ayuma"),
        ("ikkun", Some("kamidama"), "bogus"),
    ] {
        let mut s = state(&[id], &["futo"]);
        s.players[0].team[0].item = item.map(str::to_string);
        assert!(!kamishinka::apply(
            &mut s,
            &action("p1", Some(form)),
            &engine().move_db
        ));
        assert!(!s.players[0].kamishinka_used);
    }
    let mut s = state(&["nisiki"], &["futo"]);
    let mut a = action("p1", Some("kami_nisiki"));
    a.move_id = Some("unknown".into());
    assert!(!kamishinka::apply(&mut s, &a, &engine().move_db));
    a.move_id = Some("tackle".into());
    s.players[0].team[0].move_pp.insert("tackle".into(), 0);
    assert!(!kamishinka::apply(&mut s, &a, &engine().move_db));
    a.action_type = ActionType::Switch;
    assert!(!kamishinka::apply(&mut s, &a, &engine().move_db));
}
#[test]
fn multiple_items_one_use_per_player_and_independent_opponents() {
    let s = state(&["nisiki", "ayuma"], &["nisiki", "ayuma"]);
    assert!(s
        .players
        .iter()
        .all(|p| p.team.iter().all(|c| c.item.as_deref() == Some("kamidama"))));
    let s = step(
        &s,
        &[
            action("p1", Some("kami_nisiki")),
            action("p1", Some("kami_nisiki")),
            action("p2", Some("kami_nisiki")),
        ],
    );
    assert!(s.players.iter().all(|p| p.kamishinka_used));
    assert_eq!(
        s.log
            .iter()
            .filter(|l| l.contains("カミシンカした"))
            .count(),
        2
    );
    let s = step(&s, &[switch("p1", 1), switch("p2", 1)]);
    assert!(s.players.iter().all(|p| available_forms(p).is_empty()));
    let s = step(
        &s,
        &[
            action("p1", Some("kami_ayuma")),
            action("p2", Some("kami_ayuma")),
        ],
    );
    assert!(s.players.iter().all(|p| p.team[1].kamishinka.is_none()));
}
#[test]
fn switching_and_fainting_do_not_reset_form_or_right() {
    let s = step(
        &state(&["nisiki", "futo"], &["futo"]),
        &[action("p1", Some("kami_nisiki"))],
    );
    let speed = s.players[0].team[0].speed;
    let s = step(&s, &[switch("p1", 1)]);
    let s = step(&s, &[switch("p1", 0)]);
    assert_eq!(s.players[0].team[0].speed, speed);
    assert!(s.players[0].team[0].kamishinka.is_some());
    let s = apply_event(
        &s,
        &BattleEvent::Damage {
            target_id: "p1".into(),
            amount: 9999,
            meta: Map::new(),
        },
    );
    assert!(s.players[0].kamishinka_used);
    assert_eq!(s.players[0].fainted_count, 1);
    let s = engine().replace_fainted_pokemon(&s, "p1", 1, &mut || 0.5);
    assert!(available_forms(&s.players[0]).is_empty());
}
#[test]
fn post_evolution_speed_controls_same_turn_even_if_paralyzed_or_asleep() {
    let mut s = state(&["nisiki"], &["futo"]);
    s.players[0].team[0].evs.spe = 0;
    s.players[0].team[0].speed = 80;
    s.players[1].team[0].speed = 110;
    let s = step(&s, &[action("p2", None), action("p1", Some("kami_nisiki"))]);
    let first = s
        .log
        .iter()
        .position(|l| l.contains("p1の たいあたり"))
        .unwrap();
    let second = s
        .log
        .iter()
        .position(|l| l.contains("p2の たいあたり"))
        .unwrap();
    assert!(first < second, "{:?}", s.log);
    let mut s = state(&["nisiki"], &["futo"]);
    s.players[0].team[0].statuses.push(Status {
        id: "sleep".into(),
        remaining_turns: Some(3),
        data: HashMap::new(),
    });
    let hp = s.players[1].team[0].hp;
    let s = step(&s, &[action("p1", Some("kami_nisiki"))]);
    assert!(s.players[0].kamishinka_used);
    assert_eq!(s.players[1].team[0].hp, hp);
}
#[test]
fn switches_happen_before_evolution_and_trick_room_reverses_new_speed() {
    let mut s = state(&["nisiki"], &["futo", "ayuma"]);
    s.field.global.push(FieldEffect {
        id: "trick_room".into(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let s = step(&s, &[action("p1", Some("kami_nisiki")), switch("p2", 1)]);
    let switch = s.log.iter().position(|l| l.contains("繰り出した")).unwrap();
    let evo = s
        .log
        .iter()
        .position(|l| l.contains("カミシンカした"))
        .unwrap();
    assert!(switch < evo);
    let mut s = state(&["nisiki"], &["futo"]);
    s.players[1].team[0].speed = 100;
    s.field.global.push(FieldEffect {
        id: "trick_room".into(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let s = step(&s, &[action("p1", Some("kami_nisiki")), action("p2", None)]);
    assert!(
        s.log
            .iter()
            .position(|l| l.contains("p2の たいあたり"))
            .unwrap()
            < s.log
                .iter()
                .position(|l| l.contains("p1の たいあたり"))
                .unwrap()
    );
}
#[test]
fn truant_alternates_and_switch_resets_loafing() {
    let mut s = state(&["ikkun", "futo"], &["futo"]);
    s.players[1].team[0].hp = 9999;
    s.players[1].team[0].max_hp = 9999;
    let s = step(&s, &[action("p1", Some("kami_ikkun_x"))]);
    let hp = s.players[1].team[0].hp;
    assert!(hp < 9999);
    let s = step(&s, &[action("p1", None)]);
    assert_eq!(s.players[1].team[0].hp, hp);
    let s = step(&s, &[action("p1", None)]);
    assert!(s.players[1].team[0].hp < hp);
    let s = step(&s, &[switch("p1", 1)]);
    let s = step(&s, &[switch("p1", 0)]);
    let hp = s.players[1].team[0].hp;
    let s = step(&s, &[action("p1", None)]);
    assert!(s.players[1].team[0].hp < hp);
}
fn value(s: &BattleState, hook: &str, category: &str) -> f32 {
    run_ability_value_hook(
        s,
        "p1",
        hook,
        100.0,
        AbilityValueContext {
            move_data: None,
            category: Some(category),
            target: None,
            weather: None,
            turn: s.turn,
            stages: None,
        },
    )
}
#[test]
fn solar_power_flare_boost_and_supreme_overlord_use_existing_hooks() {
    let mut s = state(&["haruta"], &["futo"]);
    s.field.global.push(FieldEffect {
        id: "sun".into(),
        remaining_turns: Some(5),
        data: HashMap::new(),
    });
    let original = s.players[0].team[0].hp;
    let s = step(&s, &[action("p1", Some("kami_haruta"))]);
    assert_eq!(s.players[0].team[0].hp, original - original / 8);
    assert_eq!(value(&s, "onModifyOffense", "special"), 150.0);
    let mut s = state(&["morimori"], &["futo"]);
    kamishinka::apply(
        &mut s,
        &action("p1", Some("kami_morimori")),
        &engine().move_db,
    );
    s.players[0].team[0].statuses.push(Status {
        id: "burn".into(),
        remaining_turns: None,
        data: HashMap::new(),
    });
    assert_eq!(value(&s, "onModifyPower", "special"), 150.0);
    assert_eq!(value(&s, "onModifyPower", "physical"), 100.0);
    let mut s = state(&["ayuma"], &["futo"]);
    s.players[0].fainted_count = 8;
    let s = step(&s, &[action("p1", Some("kami_ayuma"))]);
    assert_eq!(value(&s, "onModifyPower", "physical"), 150.0);
}
#[test]
fn gas_suppresses_hooks_and_restarts_entry_ability_when_it_leaves() {
    let mut s = state(&["makocchan", "futo"], &["macchan"]);
    s.players[1].team[0].ability = Some("intimidate".into());
    let s = step(&s, &[action("p1", Some("kami_makocchan"))]);
    assert_eq!(effective_ability(&s, &s.players[1].team[0]), None);
    let s = step(&s, &[switch("p1", 1)]);
    assert_eq!(s.players[0].team[1].stages.atk, -1);
    assert_eq!(
        effective_ability(&s, &s.players[1].team[0]),
        Some("intimidate")
    );
    let mut s = state(&["makocchan"], &["shiraishi"]);
    s.players[0].team[0].ability = Some("neutralizing_gas".into());
    s.players[1].team[0].ability = Some("disguise".into());
    assert_eq!(
        effective_ability(&s, &s.players[1].team[0]),
        Some("disguise")
    );
    s.players[1].team[0].ability = Some("solar_power".into());
    let s = refresh_gas(s, &mut || 0.5);
    let mut s = apply_event(
        &s,
        &BattleEvent::Damage {
            target_id: "p1".into(),
            amount: 9999,
            meta: Map::new(),
        },
    );
    s = refresh_gas(s, &mut || 0.5);
    assert_eq!(
        effective_ability(&s, &s.players[1].team[0]),
        Some("solar_power")
    );
}
#[test]
fn poison_touch_uses_contact_and_existing_status_immunity() {
    let mut s = state(&["morimitu"], &["futo"]);
    s.players[0].team[0].ability = Some("poison_touch".into());
    let event = BattleEvent::Damage {
        target_id: "p2".into(),
        amount: 10,
        meta: meta_with_move_source(Some("tackle"), Some("p1")),
    };
    let events =
        apply_ability_event_modifiers(&s, &[event], engine().move_db.as_map(), &mut || 0.0);
    for e in &events {
        s = apply_event(&s, e);
    }
    assert!(s.players[1].team[0]
        .statuses
        .iter()
        .any(|s| s.id == "poison"));
}
#[test]
fn fresh_battle_resets_form_and_right_without_consuming_item() {
    let s = state(&["nisiki"], &["futo"]);
    let original = s.players[0].team[0].clone();
    let s = step(&s, &[action("p1", Some("kami_nisiki"))]);
    let restored = create_battle_state(s.players);
    let p = &restored.players[0];
    assert!(!p.kamishinka_used);
    assert!(p.team[0].kamishinka.is_none());
    assert_eq!(p.team[0].speed, original.speed);
    assert_eq!(p.team[0].ability, original.ability);
    assert_eq!(p.team[0].item, original.item);
}
#[test]
fn state_serialization_retains_form_usage_and_request() {
    let s = step(
        &state(&["nisiki"], &["futo"]),
        &[action("p1", Some("kami_nisiki"))],
    );
    let restored: BattleState = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert!(restored.players[0].kamishinka_used);
    assert_eq!(
        restored.players[0].team[0]
            .kamishinka
            .as_ref()
            .unwrap()
            .form_id,
        "kami_nisiki"
    );
}

#[test]
fn tinted_lens_and_thick_fat_modify_real_damage_including_gas_suppression() {
    fn damage(ability: &str, move_id: &str, target_types: &[&str], defender_ability: &str) -> i32 {
        let mut s = state(&["nisiki"], &["futo"]);
        s.players[0].team[0].ability = Some(ability.into());
        s.players[0].team[0].moves = vec![move_id.into()];
        s.players[1].team[0].ability = Some(defender_ability.into());
        s.players[1].team[0].types = target_types.iter().map(|s| s.to_string()).collect();
        let old = s.players[1].team[0].hp;
        let mut a = action("p1", None);
        a.move_id = Some(move_id.into());
        let next = step(&s, &[a]);
        old - next.players[1].team[0].hp
    }
    let normal = damage("none", "confusion", &["steel"], "none");
    let tinted = damage("tinted_lens", "confusion", &["steel"], "none");
    assert!(normal > 0 && tinted >= normal * 2 && tinted <= normal * 2 + 1);
    assert_eq!(damage("tinted_lens", "confusion", &["dark"], "none"), 0);
    assert_eq!(
        damage("tinted_lens", "confusion", &["steel"], "neutralizing_gas"),
        normal
    );
    let normal = damage("none", "flamethrower", &["normal"], "none");
    let thick = damage("none", "flamethrower", &["normal"], "thick_fat");
    assert!(thick < normal && (normal - thick * 2).abs() <= 3);
    assert_eq!(
        damage("neutralizing_gas", "flamethrower", &["normal"], "thick_fat"),
        normal
    );
}

#[test]
fn gas_restarts_slow_start_and_opposing_steel_spirit_does_not_boost_overlord() {
    let mut s = state(&["morimori"], &["makocchan", "futo"]);
    s.turn = 10;
    s.players[0].team[0].ability = Some("slow_start".into());
    s.players[1].team[0].ability = Some("neutralizing_gas".into());
    let mut s = refresh_gas(s, &mut || 0.5);
    assert_eq!(value(&s, "onModifySpeed", "physical"), 100.0);
    s.players[1].active_slot = 1;
    s = refresh_gas(s, &mut || 0.5);
    assert_eq!(value(&s, "onModifySpeed", "physical"), 50.0);
    s.turn = 15;
    assert_eq!(value(&s, "onModifySpeed", "physical"), 100.0);
    s.players[0].team[0].ability = Some("supreme_overlord".into());
    s.players[0].team[0]
        .ability_data
        .insert("fallenAllies".into(), json!(3));
    s.players[1].team[1].ability = Some("steely_spirit".into());
    let db = engine().move_db;
    let m = db.get("iron_head").unwrap();
    let power = run_ability_value_hook(
        &s,
        "p1",
        "onModifyPower",
        100.0,
        AbilityValueContext {
            move_data: Some(m),
            category: Some("physical"),
            target: None,
            weather: None,
            turn: s.turn,
            stages: None,
        },
    );
    assert!((power - 130.0).abs() < 0.001);
}

#[test]
fn fainted_creatures_cannot_trigger_entry_abilities() {
    use engine_rust::core::abilities::{run_ability_hooks, AbilityHookContext};
    let mut s = state(&["macchan"], &["futo"]);
    s.players[0].team[0].ability = Some("intimidate".into());
    s.players[0].team[0].hp = 0;
    let result = run_ability_hooks(
        &s,
        "p1",
        "onSwitchIn",
        AbilityHookContext {
            rng: &mut || 0.5,
            action: None,
            move_data: None,
        },
    );
    assert!(result.events.is_empty());
    assert!(result.state.is_none());
}

#[test]
fn shared_ai_variants_keep_switches_and_expand_only_ordinary_moves() {
    let s = state(&["ikkun", "nisiki"], &["futo"]);
    let mut actions = vec![action("p1", None), switch("p1", 1)];
    kamishinka::append_move_variants(&s.players[0], &mut actions);
    assert_eq!(actions.len(), 4);
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.action_type == ActionType::Switch)
            .count(),
        1
    );
    assert_eq!(
        actions
            .iter()
            .filter_map(|a| a.kamishinka_form.as_deref())
            .collect::<Vec<_>>(),
        vec!["kami_ikkun_x", "kami_ikkun_y"]
    );
    let mut used = s.players[0].clone();
    used.kamishinka_used = true;
    let mut actions = vec![action("p1", None)];
    kamishinka::append_move_variants(&used, &mut actions);
    assert_eq!(actions.len(), 1);
}

#[test]
fn simultaneous_switches_use_outgoing_speed_and_trick_room() {
    for trick in [false, true] {
        let mut s = state(&["futo", "haruta"], &["futo", "reosan"]);
        s.players[0].team[0].speed = 40;
        s.players[1].team[0].speed = 200;
        s.players[0].team[1].ability = Some("drought".into());
        s.players[1].team[1].ability = Some("drizzle".into());
        if trick {
            s.field.global.push(FieldEffect {
                id: "trick_room".into(),
                remaining_turns: Some(5),
                data: HashMap::new(),
            });
        }
        let next = step(&s, &[switch("p1", 1), switch("p2", 1)]);
        let logs: Vec<_> = next
            .log
            .iter()
            .filter(|l| l.contains("繰り出した"))
            .collect();
        assert!(logs[0].starts_with(if trick { "p1" } else { "p2" }));
        assert!(next
            .field
            .global
            .iter()
            .any(|f| f.id == if trick { "rain" } else { "sun" }));
    }
}

#[test]
fn changed_abilities_start_and_old_runtime_state_ends() {
    use engine_rust::core::abilities::resolve_ability_changes;
    let mut s = state(&["futo", "nisiki"], &["futo"]);
    s.players[0].team[0].ability = None;
    s.players[0].team[0]
        .ability_data
        .insert("truantLoaf".into(), json!(true));
    s = apply_event(
        &s,
        &BattleEvent::SetAbility {
            target_id: "p1".into(),
            ability_id: Some("drought".into()),
            meta: Map::new(),
        },
    );
    s = resolve_ability_changes(s, &mut || 0.5);
    assert!(s.field.global.iter().any(|f| f.id == "sun"));
    assert!(!s.players[0].team[0].ability_data.contains_key("truantLoaf"));
    s = apply_event(
        &s,
        &BattleEvent::Switch {
            player_id: "p1".into(),
            slot: 1,
        },
    );
    assert!(s.players[0].team[0].ability.is_none());
    let mut s = state(&["futo"], &["futo"]);
    s.players[1].team[0].ability = Some("supreme_overlord".into());
    s.players[0].fainted_count = 3;
    s = apply_event(
        &s,
        &BattleEvent::SwapAbilities {
            left_id: "p1".into(),
            right_id: "p2".into(),
            meta: Map::new(),
        },
    );
    s = resolve_ability_changes(s, &mut || 0.5);
    assert!((value(&s, "onModifyPower", "physical") - 130.0).abs() < 0.001);
}

#[test]
fn sleep_counter_only_decreases_when_attempting_a_move() {
    use engine_rust::core::statuses::tick_statuses;
    let mut s = state(&["nisiki", "futo"], &["futo"]);
    s = apply_event(
        &s,
        &BattleEvent::ApplyStatus {
            target_id: "p1".into(),
            status_id: "sleep".into(),
            duration: Some(3),
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    );
    let s = tick_statuses(&s);
    assert_eq!(s.players[0].team[0].statuses[0].data["turns"], json!(3));
    let s = step(&s, &[switch("p1", 1)]);
    let s = step(&s, &[switch("p1", 0)]);
    assert_eq!(s.players[0].team[0].statuses[0].data["turns"], json!(3));
    let s = step(&s, &[action("p1", None)]);
    let s = step(&s, &[action("p1", None)]);
    assert_eq!(s.players[0].team[0].statuses[0].data["turns"], json!(1));
    let s = step(&s, &[action("p1", None)]);
    assert!(!s.players[0].team[0]
        .statuses
        .iter()
        .any(|s| s.id == "sleep"));
}

#[test]
fn poison_touch_is_blocked_by_dust_and_cloak_but_gas_suppresses_dust() {
    for (ability, item, gas, blocked) in [
        ("shield_dust", None, false, true),
        ("none", Some("covert_cloak"), false, true),
        ("shield_dust", None, true, false),
    ] {
        let mut s = state(&["morimitu"], &["futo"]);
        s.players[0].team[0].ability = Some("poison_touch".into());
        s.players[1].team[0].ability = Some(ability.into());
        s.players[1].team[0].item = item.map(str::to_string);
        // Check the shared predicate separately; Gas would suppress Poison Touch too.
        if gas {
            s.players[0].team[0].ability = Some("neutralizing_gas".into());
        }
        assert_eq!(
            engine_rust::core::abilities::blocks_secondary_effects(&s, &s.players[1].team[0]),
            blocked
        );
        if !gas {
            let event = BattleEvent::Damage {
                target_id: "p2".into(),
                amount: 10,
                meta: meta_with_move_source(Some("tackle"), Some("p1")),
            };
            let events =
                apply_ability_event_modifiers(&s, &[event], engine().move_db.as_map(), &mut || 0.0);
            assert!(!events.iter().any(
                |e| matches!(e,BattleEvent::ApplyStatus{status_id,..} if status_id=="poison")
            ));
        }
    }
}

#[test]
fn ignoring_defender_abilities_preserves_own_stab_and_power() {
    use engine_rust::data::moves::Effect;
    let damage = |own: &str, defender: &str, ignore: bool| {
        let mut e = engine();
        let mut m = e.move_db.get("flamethrower").unwrap().clone();
        m.id = "piercing_test".into();
        if ignore {
            m.steps.insert(
                0,
                Effect {
                    effect_type: "ignore_ability".into(),
                    data: Map::new(),
                },
            );
        }
        e.move_db.insert(m);
        let mut s = state(&["futo"], &["futo"]);
        s.players[0].team[0].types = vec!["fire".into()];
        s.players[1].team[0].types = vec!["normal".into()];
        s.players[0].team[0].moves = vec!["piercing_test".into()];
        s.players[0].team[0].ability = Some(own.into());
        s.players[1].team[0].ability = Some(defender.into());
        if own == "flare_boost" {
            s.players[0].team[0].statuses.push(Status {
                id: "burn".into(),
                remaining_turns: None,
                data: HashMap::new(),
            });
        }
        let hp = s.players[1].team[0].hp;
        let mut a = action("p1", None);
        a.move_id = Some("piercing_test".into());
        let next = e.step_battle(&s, &[a], &mut || 0.5, BattleOptions::default());
        hp - next.players[1].team[0].hp
    };
    for own in ["adaptability", "flare_boost"] {
        assert_eq!(damage(own, "thick_fat", true), damage(own, "none", false));
        assert!(damage(own, "thick_fat", false) < damage(own, "thick_fat", true));
    }
}

#[test]
fn integer_damage_stages_match_known_hand_calculations() {
    use engine_rust::core::damage::{apply_roll, base_damage, modify};
    assert_eq!(base_damage(50, 90, 137, 103), 54);
    assert_eq!(apply_roll(54, 85), 45);
    assert_eq!(modify(45, 1.5), 67); // 67.5 rounds down.
    assert_eq!(modify(67, 0.5), 33);
    assert_eq!(modify(100, 1.5), 150);
}

#[test]
fn rest_sleeps_for_two_actions_then_allows_the_third() {
    let mut s = state(&["futo"], &["futo"]);
    s.players[0].team[0].moves.push("rest".into());
    s.players[0].team[0].hp -= 20;
    let mut rest = action("p1", None);
    rest.move_id = Some("rest".into());
    let mut s = step(&s, &[rest]);
    let hp = s.players[1].team[0].hp;
    for _ in 0..2 {
        s = step(&s, &[action("p1", None)]);
        assert_eq!(s.players[1].team[0].hp, hp);
    }
    s = step(&s, &[action("p1", None)]);
    assert!(s.players[1].team[0].hp < hp);
}

#[test]
fn custom_alchemy_still_inherits_at_entry_and_starts_the_copied_ability() {
    let mut s = state(&["makocchan", "futo"], &["futo"]);
    s.players[0].team[0].ability = Some("power_of_alchemy".into());
    s.players[0].last_fainted_ability = Some("drought".into());
    let s = engine().apply_initial_switch_in_effects(&s, &mut || 0.5);
    assert_eq!(s.players[0].team[0].ability.as_deref(), Some("drought"));
    assert!(s.field.global.iter().any(|f| f.id == "sun"));
    let s = step(&s, &[switch("p1", 1)]);
    assert_eq!(
        s.players[0].team[0].ability.as_deref(),
        Some("power_of_alchemy")
    );
}

#[test]
fn real_damage_uses_integer_random_stab_and_critical_stages() {
    for (stab, crit, expected) in [(false, false, 50), (true, false, 75), (true, true, 112)] {
        let mut e = engine();
        let mut m = e.move_db.get("flamethrower").unwrap().clone();
        m.crit_rate = Some(if crit { 3 } else { 0 });
        e.move_db.insert(m);
        let mut s = state(&["futo"], &["futo"]);
        s.players[0].team[0].sp_attack = 137;
        s.players[1].team[0].sp_defense = 103;
        s.players[0].team[0].types = vec![if stab { "fire" } else { "normal" }.into()];
        s.players[1].team[0].types = vec!["normal".into()];
        s.players[0].team[0].moves = vec!["flamethrower".into()];
        s.players[1].team[0].hp = 9999;
        s.players[1].team[0].max_hp = 9999;
        let mut a = action("p1", None);
        a.move_id = Some("flamethrower".into());
        let next = e.step_battle(&s, &[a], &mut || 0.5, BattleOptions::default());
        assert_eq!(9999 - next.players[1].team[0].hp, expected);
    }
}
