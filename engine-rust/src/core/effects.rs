use crate::core::abilities::{
    run_ability_check_hook, run_ability_value_hook, AbilityCheckContext, AbilityValueContext, WeatherKind,
};
use crate::core::events::{
    apply_event, meta_with_move_source, BattleEvent,
};
use crate::core::state::BattleState;
use crate::core::utils::{get_active_creature, stage_multiplier};
use crate::data::moves::{Effect, MoveData};
use crate::data::type_chart::TypeChart;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub struct EffectContext<'a> {
    pub attacker_player_id: String,
    pub target_player_id: String,
    pub move_data: Option<&'a MoveData>,
    pub rng: &'a mut dyn FnMut() -> f64,
    pub turn: u32,
    pub type_chart: &'a TypeChart,
    pub bypass_protect: bool,
    pub ignore_immunity: bool,
    pub bypass_substitute: bool,
    pub ignore_substitute: bool,
    pub ignore_ability: bool,
    pub is_sound: bool,
    pub last_damage: Option<i32>,
}

pub fn apply_effects(state: &BattleState, steps: &[Effect], ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    apply_move_tag_flags(ctx);
    apply_effect_flags(ctx, steps);
    let mut events = Vec::new();
    let base_state = state.clone();
    let mut working_state = base_state.clone();
    let mut block_follow_up_effects = false;
    for effect in steps {
        if block_follow_up_effects && is_blocked_after_missed_damage(effect) {
            continue;
        }
        match effect.effect_type.as_str() {
            "modify_damage" => {
                apply_modify_damage(&mut events, effect, &working_state, ctx);
                clamp_damage_events_to_remaining_hp(&base_state, &mut events);
                update_last_damage_from_events(ctx, &events);
                working_state = apply_events(&base_state, &events);
            }
            "crit" => {
                apply_force_crit(&mut events, effect, &working_state, ctx);
                clamp_damage_events_to_remaining_hp(&base_state, &mut events);
                update_last_damage_from_events(ctx, &events);
                working_state = apply_events(&base_state, &events);
            }
            _ => {
                let mut effect_events = apply_effect(&working_state, effect, ctx);
                clamp_damage_events_to_remaining_hp(&working_state, &mut effect_events);
                update_last_damage_from_events(ctx, &effect_events);
                if effect.effect_type == "damage" && !damage_step_connected(&effect_events) {
                    ctx.last_damage = Some(0);
                    block_follow_up_effects = true;
                }
                working_state = apply_events(&working_state, &effect_events);
                events.extend(effect_events);
            }
        }
    }
    apply_meta_flags(&mut events, ctx);
    events
}

pub fn apply_events(state: &BattleState, events: &[BattleEvent]) -> BattleState {
    let mut next = state.clone();
    for event in events {
        next = apply_event(&next, event);
    }
    next
}

fn clamp_damage_events_to_remaining_hp(state: &BattleState, events: &mut [BattleEvent]) {
    let mut simulated_state = state.clone();
    for event in events.iter_mut() {
        if let BattleEvent::Damage { target_id, amount, meta } = event {
            if *amount > 0 {
                *amount = actual_positive_damage_amount(&simulated_state, target_id, meta, *amount);
            }
        }
        simulated_state = apply_event(&simulated_state, event);
    }
}

fn actual_positive_damage_amount(
    state: &BattleState,
    target_id: &str,
    meta: &Map<String, Value>,
    amount: i32,
) -> i32 {
    let Some(player) = state.players.iter().find(|player| player.id == *target_id) else {
        return amount;
    };
    let Some(active) = player.team.get(player.active_slot) else {
        return amount;
    };
    let source = meta.get("source").and_then(|value| value.as_str());
    let bypass_substitute = meta
        .get("bypassSubstitute")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let is_self = source == Some(target_id);
    if !bypass_substitute && !is_self {
        if let Some(substitute) = active.statuses.iter().find(|status| status.id == "substitute") {
            let substitute_hp = substitute
                .data
                .get("hp")
                .and_then(|value| value.as_i64())
                .map(|value| value as i32)
                .unwrap_or_else(|| ((active.max_hp as f64) * 0.25).floor().max(1.0) as i32);
            return amount.min(substitute_hp.max(0));
        }
    }
    amount.min(active.hp.max(0))
}

fn damage_step_connected(events: &[BattleEvent]) -> bool {
    events.iter().any(|event| matches!(event, BattleEvent::Damage { amount, .. } if *amount > 0))
}

fn is_blocked_after_missed_damage(effect: &Effect) -> bool {
    matches!(
        effect.effect_type.as_str(),
        "chance"
            | "apply_status"
            | "modify_stage"
            | "force_switch"
            | "self_switch"
            | "heal_last_damage"
            | "recoil_last_damage"
    )
}

fn apply_effect(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let effect_type = effect.effect_type.as_str();
    match effect_type {
        "protect" => apply_protect(state, effect, ctx),
        "damage" => apply_damage(state, effect, ctx),
        "heal_last_damage" => apply_last_damage_ratio(state, effect, ctx, true),
        "recoil_last_damage" => apply_last_damage_ratio(state, effect, ctx, false),
        "crash_if_no_damage" => apply_crash_if_no_damage(state, effect, ctx),
        "speed_based_damage" => apply_speed_based_damage(state, effect, ctx),
        "apply_status" => apply_status(state, effect, ctx),
        "remove_status" => apply_remove_status(effect, ctx),
        "replace_status" => apply_replace_status(state, effect, ctx),
        "modify_stage" => apply_modify_stage(effect, ctx),
        "clear_stages" => apply_clear_stages(effect, ctx),
        "reset_stages" => apply_reset_stages(effect, ctx),
        "disable_move" => apply_disable_move(state, effect, ctx),
        "disable_last_move" => apply_disable_last_move(state, effect, ctx),
        "damage_ratio" => apply_damage_ratio(state, effect, ctx),
        "hp_based_damage" => apply_hp_based_damage(state, effect, ctx),
        "pain_split_effect" => apply_pain_split_effect(state, ctx),
        "endeavor_effect" => apply_endeavor_effect(state, ctx),
        "final_gambit_effect" => apply_final_gambit_effect(state, ctx),
        "counter_effect" => apply_counter_effect(state, ctx),
        "mirror_coat_effect" => apply_mirror_coat_effect(state, ctx),
        "hp_ratio_damage" => apply_hp_ratio_damage(state, effect, ctx),
        "set_atk_max" => apply_set_atk_max(state, ctx),
        "copy_stages" => apply_copy_stages(state, ctx),
        "swap_stages" => apply_swap_stages(state, effect, ctx),
        "average_stages" => apply_average_stages(state, effect, ctx),
        "random_stage_boost" => apply_random_stage_boost(state, ctx),
        "swap_items" => apply_swap_items(state, ctx),
        "steal_item" => apply_steal_item(state, effect, ctx),
        "swap_abilities" => apply_swap_abilities(state, ctx),
        "set_ability" => apply_set_ability(state, effect, ctx),
        "suppress_ability" => apply_suppress_ability(state, effect, ctx),
        "haze_effect" => apply_haze_effect(state, ctx),
        "curse_effect" => apply_curse_effect(state, ctx),
        "swap_attack_defense" => apply_swap_attack_defense(ctx),
        "inverse_speed_based_damage" => apply_inverse_speed_based_damage(state, effect, ctx),
        "weight_based_damage" => apply_weight_based_damage(state, effect, ctx),
        "relative_weight_damage" => apply_relative_weight_damage(state, effect, ctx),
        "fling_effect" => apply_fling_effect(state, ctx),
        "beat_up_effect" => apply_beat_up_effect(state, ctx),
        "imprison_effect" => apply_imprison_effect(state, ctx),
        "healing_wish_effect" => apply_healing_wish_effect(state, ctx),
        "strength_sap_effect" => apply_strength_sap_effect(state, ctx),
        "charge_attack" => apply_charge_attack(state, effect, ctx),
        "triple_axel_effect" => apply_triple_axel_effect(state, effect, ctx),
        "after_you_effect" => apply_after_you_effect(state, ctx),
        "delay" | "wait" => apply_delay(state, effect, ctx),
        "over_time" => apply_over_time(state, effect, ctx),
        "chance" => apply_chance(state, effect, ctx),
        "repeat" => apply_repeat(state, effect, ctx),
        "conditional" => apply_conditional(state, effect, ctx),
        "log" => apply_log(effect, ctx),
        "apply_field_status" => apply_field_status(state, effect, ctx),
        "remove_field_status" => apply_remove_field_status(effect, ctx),
        "random_move" => apply_random_move(effect, ctx),
        "apply_item" => apply_apply_item(state, effect, ctx),
        "remove_item" => apply_remove_item(state, effect, ctx),
        "consume_item" => apply_consume_item(state, effect, ctx),
        "ohko" => apply_ohko(state, effect, ctx),
        "cure_all_status" => apply_cure_all_status(effect, ctx),
        "self_switch" => apply_self_switch(state, ctx),
        "force_switch" => apply_force_switch(state, effect, ctx),
        "replace_pokemon" => apply_replace_pokemon(ctx),
        "lock_move" => apply_lock_move(state, effect, ctx),
        "run_away" => apply_run_away(),
        "bypass_protect"
        | "bypass_substitute"
        | "ignore_immunity"
        | "ignore_ability"
        | "ignore_substitute"
        | "sound" => Vec::new(),
        "manual" => apply_manual_effect(effect, ctx),
        _ => Vec::new(),
    }
}

fn apply_after_you_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(target) = get_active_creature(state, &ctx.target_player_id) else {
        return Vec::new();
    };
    if target.hp <= 0 {
        return vec![BattleEvent::Log {
            message: "しかし うまく きまらなかった！".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    vec![
        BattleEvent::Log {
            message: format!("{}に おさきにどうぞ！", target.name),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        // Set a flag on the target creature; battle.rs picks this up to reorder remaining actions.
        BattleEvent::SetVolatile {
            target_id: ctx.target_player_id.clone(),
            key: "afterYouPending".to_string(),
            value: serde_json::Value::Bool(true),
        },
    ]
}

fn apply_manual_effect(effect: &Effect, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    let reason = effect.data.get("manualReason").and_then(|v| v.as_str()).unwrap_or("");
    if reason.contains("Switching") {
        return vec![BattleEvent::ApplyStatus {
            target_id: ctx.attacker_player_id.clone(),
            status_id: "pending_switch".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    let move_name = ctx
        .move_data
        .and_then(|m| m.name.as_deref())
        .unwrap_or("manual move");
    vec![BattleEvent::Log {
        message: if reason.is_empty() {
            format!("[MANUAL] {} has unresolved manual behavior.", move_name)
        } else {
            format!("[MANUAL] {}: {}", move_name, reason)
        },
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_protect(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(attacker) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };

    let success_count = attacker
        .volatile_data
        .get("protectSuccessCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let mut chance = 1.0;
    for _ in 0..success_count {
        chance /= 3.0;
    }

    if (ctx.rng)() > chance {
        return vec![
            BattleEvent::Log {
                message: format!("{}の まもりは 失敗した！", attacker.name),
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            },
            BattleEvent::SetVolatile {
                target_id: ctx.attacker_player_id.clone(),
                key: "protectSuccessCount".to_string(),
                value: Value::Number(0.into()),
            },
        ];
    }

    vec![
        BattleEvent::SetVolatile {
            target_id: ctx.attacker_player_id.clone(),
            key: "protectSuccessCount".to_string(),
            value: Value::Number((success_count + 1).into()),
        },
        BattleEvent::ApplyStatus {
        target_id: ctx.attacker_player_id.clone(),
        status_id: effect
            .data
            .get("statusId")
            .and_then(|v| v.as_str())
            .unwrap_or("protect")
            .to_string(),
        duration: Some(1),
        stack: false,
        data: HashMap::new(),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_damage(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(attacker) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };

    let mut accuracy = value_f64(effect.data.get("accuracy"), state, ctx).unwrap_or(1.0);
    if let Some(Value::Array(overrides)) = effect.data.get("accuracyIf") {
        for override_value in overrides {
            let Some(override_map) = override_value.as_object() else {
                continue;
            };
            if evaluate_condition(state, override_map.get("if"), ctx) {
                accuracy = value_f64(override_map.get("accuracy"), state, ctx).unwrap_or(accuracy);
                break;
            }
        }
    }
    if attacker.statuses.iter().any(|status| {
        status.id == "lock_on"
            && status
                .data
                .get("targetId")
                .and_then(|v| v.as_str())
                == Some(target_id.as_str())
    }) {
        accuracy = 1.0;
    }
    let move_category = get_move_category(ctx.move_data);
    accuracy = run_ability_value_hook(
        state,
        &ctx.attacker_player_id,
        "onModifyAccuracy",
        accuracy as f32,
        AbilityValueContext {
            move_data: ctx.move_data,
            category: move_category.as_deref(),
            target: Some(target),
            weather: None,
            turn: ctx.turn,
            stages: None,
        },
    ) as f64;

    if (ctx.rng)() > accuracy {
        return vec![BattleEvent::Log {
            message: "しかし はずれた！".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }

    let mut power = value_i32(effect.data.get("power"), state, ctx).unwrap_or(0);
    if evaluate_condition(state, effect.data.get("powerMultiplierIf"), ctx) {
        let multiplier = value_f64(effect.data.get("powerMultiplier"), state, ctx).unwrap_or(1.0);
        power = ((power as f64) * multiplier).round() as i32;
    }
    if let Some(per_stage) = value_i32(effect.data.get("powerPerPositiveStage"), state, ctx) {
        let base_power = value_i32(effect.data.get("basePower"), state, ctx).unwrap_or(power);
        power = base_power + per_stage * positive_stage_total(attacker);
    }
    if let Some(per_hit) = value_i32(effect.data.get("powerPerHitTaken"), state, ctx) {
        let hits = attacker
            .volatile_data
            .get("moveHitsTaken")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        power += per_hit * hits;
    }
    if effect
        .data
        .get("powerDoublesPerConsecutiveUse")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let count = attacker
            .volatile_data
            .get("consecutiveMoveCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(1) as u32;
        power = power.saturating_mul(2_i32.saturating_pow(count.saturating_sub(1)));
    }
    if let Some(max_power) = value_i32(effect.data.get("maxPower"), state, ctx) {
        power = power.min(max_power);
    }
    let charge_boost = ctx
        .move_data
        .and_then(|m| m.move_type.as_deref())
        .is_some_and(|move_type| move_type == "electric")
        && attacker.statuses.iter().any(|status| status.id == "charge");
    if charge_boost {
        power *= 2;
    }
    let attacker_id = ctx.attacker_player_id.clone();
    
    // Pass false for is_secondary_hit, let calc_damage handle crit logic
    let use_defensive_stat = effect
        .data
        .get("useDefensiveStat")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let offensive_stat = effect.data.get("useOffensiveStat").and_then(|v| v.as_str());
    let (amount, is_crit) = calc_damage(power, state, &attacker_id, &target_id, ctx, false, use_defensive_stat, offensive_stat);
    
    let mut events = Vec::new();

    if amount > 0 {
        if is_crit {
            events.push(BattleEvent::Log {
                message: "急所に あたった！".to_string(),
                meta: Map::new(),
            });
        }

        if let Some(move_type) = ctx.move_data.and_then(|m| m.move_type.as_deref()) {
            let target_types = effective_types(target);
            let eff = ctx.type_chart.effectiveness(move_type, &target_types);
            if eff > 1.0 {
                events.push(BattleEvent::Log {
                    message: "効果は 抜群だ！".to_string(),
                    meta: Map::new(),
                });
            } else if eff > 0.0 && eff < 1.0 {
                events.push(BattleEvent::Log {
                    message: "効果は 今ひとつの ようだ……".to_string(),
                    meta: Map::new(),
                });
            }
        }
    }

    let mut meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
    meta.insert("target".to_string(), Value::String(target_id.clone()));
    meta.insert("cancellable".to_string(), Value::Bool(true));
    if let Some(category) = move_category.as_deref() {
        meta.insert("category".to_string(), Value::String(category.to_string()));
    }
    events.push(BattleEvent::Damage {
        target_id: target_id.clone(),
        amount,
        meta,
    });
    if charge_boost {
        events.push(BattleEvent::RemoveStatus {
            target_id: attacker_id.clone(),
            status_id: "charge".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        });
    }

    if attacker.ability.as_deref() == Some("parental_bond") {
        let second_power = (power as f32 * 0.5).floor() as i32;
        // Pass true for is_secondary_hit, parental bond 2nd hit doesn't crit
        let (second_amount, _) = calc_damage(second_power, state, &attacker_id, &target_id, ctx, true, use_defensive_stat, offensive_stat);
        
        let mut second_meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
        second_meta.insert("target".to_string(), Value::String(ctx.target_player_id.clone()));
        second_meta.insert("cancellable".to_string(), Value::Bool(true));
        second_meta.insert("parentalBond".to_string(), Value::Bool(true));
        
        events.push(BattleEvent::Damage {
            target_id: ctx.target_player_id.clone(),
            amount: second_amount,
            meta: second_meta,
        });
    }

    events
}

fn apply_last_damage_ratio(
    state: &BattleState,
    effect: &Effect,
    ctx: &mut EffectContext<'_>,
    healing: bool,
) -> Vec<BattleEvent> {
    let Some(last_damage) = ctx.last_damage else {
        return Vec::new();
    };
    if last_damage <= 0 {
        return Vec::new();
    }
    let ratio = value_f64(effect.data.get("ratio"), state, ctx).unwrap_or(0.5);
    let mut amount = ((last_damage as f64) * ratio).floor() as i32;
    if amount <= 0 && ratio > 0.0 {
        amount = 1;
    }
    if healing {
        amount = -amount;
    }
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let mut meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
    meta.insert("target".to_string(), Value::String(target_id.clone()));
    vec![BattleEvent::Damage {
        target_id,
        amount,
        meta,
    }]
}

fn apply_crash_if_no_damage(
    state: &BattleState,
    effect: &Effect,
    ctx: &mut EffectContext<'_>,
) -> Vec<BattleEvent> {
    if ctx.last_damage.unwrap_or(0) > 0 {
        return Vec::new();
    }
    let Some(user) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let ratio = value_f64(effect.data.get("ratioMaxHp"), state, ctx).unwrap_or(0.5);
    let amount = ((user.max_hp as f64) * ratio).floor().max(1.0) as i32;
    let mut meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
    meta.insert("target".to_string(), Value::String(ctx.attacker_player_id.clone()));
    vec![
        BattleEvent::Log {
            message: format!("{}は 勢いあまって 地面に ぶつかった！", user.name),
            meta: meta.clone(),
        },
        BattleEvent::Damage {
            target_id: ctx.attacker_player_id.clone(),
            amount,
            meta,
        },
    ]
}

fn positive_stage_total(creature: &crate::core::state::CreatureState) -> i32 {
    [
        creature.stages.atk,
        creature.stages.def,
        creature.stages.spa,
        creature.stages.spd,
        creature.stages.spe,
        creature.stages.accuracy,
        creature.stages.evasion,
    ]
    .into_iter()
    .filter(|stage| *stage > 0)
    .sum()
}

fn apply_speed_based_damage(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let attacker_speed = compute_speed(state, &ctx.attacker_player_id, ctx.turn);
    let target_speed = compute_speed(state, &ctx.target_player_id, ctx.turn);
    let ratio = if target_speed <= 0.0 {
        f32::INFINITY
    } else {
        attacker_speed / target_speed
    };

    let mut chosen_power = value_i32(effect.data.get("basePower"), state, ctx).unwrap_or(0);
    if let Some(Value::Array(thresholds)) = effect.data.get("thresholds") {
        let mut parsed: Vec<(f32, i32)> = thresholds
            .iter()
            .filter_map(|v| {
                let ratio_val = v.get("ratio").and_then(|r| r.as_f64())? as f32;
                let power_val = v.get("power").and_then(|p| p.as_i64())? as i32;
                Some((ratio_val, power_val))
            })
            .collect();
        parsed.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (ratio_threshold, power) in parsed {
            if ratio >= ratio_threshold {
                chosen_power = power;
                break;
            }
        }
    }

    let mut cloned = effect.clone();
    cloned.data.insert("power".to_string(), Value::Number(chosen_power.into()));
    apply_damage(state, &cloned, ctx)
}

fn apply_status(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let status_id = match effect.data.get("statusId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Vec::new(),
    };

    let target_id = resolve_target(effect.data.get("target"), ctx);
    if is_item_status(&status_id) {
        return apply_item_status(state, &status_id, &target_id, ctx);
    }

    // Type immunity check (e.g. leech_seed vs Ghost)
    if let Some(Value::Array(immune_types)) = effect.data.get("immuneTypes") {
        if let Some(target) = get_active_creature(state, &target_id) {
            let immune = immune_types.iter().any(|t| {
                t.as_str().map(|s| target.types.iter().any(|ty| ty.eq_ignore_ascii_case(s))).unwrap_or(false)
            });
            if immune {
                return vec![BattleEvent::Log {
                    message: format!("{}には 効かないようだ……", target.name),
                    meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
                }];
            }
        }
    }

    let is_targeted_status_move = ctx
        .move_data
        .and_then(|move_data| move_data.category.as_deref())
        == Some("status")
        && target_id != ctx.attacker_player_id;
    if is_targeted_status_move {
        let mut accuracy = value_f64(effect.data.get("accuracy"), state, ctx)
            .or_else(|| ctx.move_data.and_then(|move_data| move_data.accuracy).map(|value| value as f64))
            .or_else(|| value_f64(effect.data.get("chance"), state, ctx))
            .unwrap_or(1.0);
        if status_id == "toxic"
            && get_active_creature(state, &ctx.attacker_player_id)
                .is_some_and(|attacker| attacker.types.iter().any(|t| t == "poison"))
        {
            accuracy = 1.0;
        }
        if (ctx.rng)() > accuracy {
            return vec![BattleEvent::Log {
                message: "しかし はずれた！".to_string(),
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            }];
        }
    }

    if let Some(chance) = value_f64(effect.data.get("chance"), state, ctx) {
        if !is_targeted_status_move && (ctx.rng)() > chance {
            return vec![BattleEvent::Log {
                message: format!("{}の {}は 効かなかった！",
                    get_active_creature(state, &ctx.attacker_player_id).map(|c| c.name.clone()).unwrap_or_else(|| "誰か".to_string()),
                    status_id),
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            }];
        }
    }

    let mut duration = value_i32(effect.data.get("duration"), state, ctx);
    if let Some(Value::Object(range)) = effect.data.get("duration") {
        if let (Some(min), Some(max)) = (range.get("min").and_then(|v| v.as_i64()), range.get("max").and_then(|v| v.as_i64())) {
            let span = (max - min + 1) as f64;
            duration = Some(min as i32 + ((ctx.rng)() * span).floor() as i32);
        }
    }

    let mut data = HashMap::new();
    if let Some(Value::Object(raw)) = effect.data.get("data") {
        for (k, v) in raw {
            let value = if k == "sideId" {
                match v.as_str() {
                    Some("self") => Value::String(ctx.attacker_player_id.clone()),
                    Some("target") => Value::String(ctx.target_player_id.clone()),
                    _ => v.clone(),
                }
            } else {
                v.clone()
            };
            data.insert(k.clone(), value);
        }
    }
    if let Some(Value::String(source)) = data.get("sourceId") {
        if source == "self" {
            data.insert(
                "sourceId".to_string(),
                Value::String(ctx.attacker_player_id.clone()),
            );
        }
    }
    if let Some(Value::String(target)) = data.get("targetId") {
        let resolved = match target.as_str() {
            "self" => Some(ctx.attacker_player_id.clone()),
            "target" => Some(ctx.target_player_id.clone()),
            _ => None,
        };
        if let Some(resolved) = resolved {
            data.insert("targetId".to_string(), Value::String(resolved));
        }
    }
    if let Some(Value::String(target)) = data.get("targetId") {
        if target == "target" {
            data.insert(
                "targetId".to_string(),
                Value::String(ctx.target_player_id.clone()),
            );
        }
    }
    if let Some(Value::String(moves)) = data.get("moves") {
        if moves == "self_moves" {
            if let Some(attacker) = get_active_creature(state, &ctx.attacker_player_id) {
                data.insert(
                    "moves".to_string(),
                    Value::Array(attacker.moves.iter().map(|move_id| Value::String(move_id.clone())).collect()),
                );
            }
        }
    }
    if status_id == "substitute" && !data.contains_key("hp") {
        if let Some(target) = get_active_creature(state, &target_id) {
            let hp = ((target.max_hp as f64) * 0.25).floor() as i32;
            data.insert("hp".to_string(), Value::Number(hp.max(1).into()));
        }
    }

    vec![BattleEvent::ApplyStatus {
        target_id,
        status_id: status_id.clone(),
        duration: if status_id == "sleep" { None } else { duration },
        stack: effect.data.get("stack").and_then(|v| v.as_bool()).unwrap_or(false),
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_remove_status(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let status_id = match effect.data.get("statusId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Vec::new(),
    };
    let target_id = resolve_target(effect.data.get("target"), ctx);
    vec![BattleEvent::RemoveStatus {
        target_id,
        status_id,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_replace_status(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let from = match effect.data.get("from").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Vec::new(),
    };
    let to = match effect.data.get("to").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Vec::new(),
    };
    let target_id = resolve_target(effect.data.get("target"), ctx);
    if from == "active" && to == "pending_switch" {
        return vec![BattleEvent::ApplyStatus {
            target_id,
            status_id: "pending_switch".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    let duration = value_i32(effect.data.get("duration"), state, ctx);
    let mut data = HashMap::new();
    if let Some(Value::Object(raw)) = effect.data.get("data") {
        for (k, v) in raw {
            data.insert(k.clone(), v.clone());
        }
    }
    vec![BattleEvent::ReplaceStatus {
        target_id,
        from,
        to,
        duration,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_modify_stage(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let mut stages = HashMap::new();
    if let Some(Value::Object(raw)) = effect.data.get("stages") {
        for (k, v) in raw {
            if let Some(delta) = v.as_i64() {
                stages.insert(k.clone(), delta as i32);
            }
        }
    }
    vec![BattleEvent::ModifyStage {
        target_id,
        stages,
        clamp: effect.data.get("clamp").and_then(|v| v.as_bool()).unwrap_or(true),
        fail_if_no_change: effect.data.get("fail_if_no_change").and_then(|v| v.as_bool()).unwrap_or(false),
        show_event: effect.data.get("show_event").and_then(|v| v.as_bool()).unwrap_or(true),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_clear_stages(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    vec![BattleEvent::ClearStages {
        target_id,
        show_event: effect.data.get("show_event").and_then(|v| v.as_bool()).unwrap_or(true),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_reset_stages(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    vec![BattleEvent::ResetStages {
        target_id,
        show_event: effect.data.get("show_event").and_then(|v| v.as_bool()).unwrap_or(true),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_disable_move(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let move_id = effect
        .data
        .get("moveId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            get_active_creature(state, &target_id)
                .and_then(|c| c.volatile_data.get("lastMove"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    if move_id.is_empty() {
        return Vec::new();
    }
    let mut data = HashMap::new();
    data.insert("moveId".to_string(), Value::String(move_id));
    vec![BattleEvent::ApplyStatus {
        target_id,
        status_id: "disable_move".to_string(),
        duration: value_i32(effect.data.get("duration"), state, ctx),
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_damage_ratio(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    // Support both ratioMaxHp (based on max HP) and ratioCurrentHp (based on current HP)
    let mut selected_ratio_max_hp = value_f64(effect.data.get("ratioMaxHp"), state, ctx).unwrap_or(0.0);
    if let Some(Value::Array(overrides)) = effect.data.get("ratioMaxHpIf") {
        for override_value in overrides {
            let Some(override_map) = override_value.as_object() else {
                continue;
            };
            if evaluate_condition(state, override_map.get("if"), ctx) {
                selected_ratio_max_hp = value_f64(override_map.get("ratio"), state, ctx)
                    .unwrap_or(selected_ratio_max_hp);
                break;
            }
        }
    }
    let mut amount = if let Some(ratio) = value_f64(effect.data.get("ratioCurrentHp"), state, ctx) {
        (target.hp as f64 * ratio).floor() as i32
    } else {
        (target.max_hp as f64 * selected_ratio_max_hp).floor() as i32
    };
    let ratio = value_f64(effect.data.get("ratioCurrentHp"), state, ctx)
        .or_else(|| Some(selected_ratio_max_hp))
        .unwrap_or(0.0);
    if amount == 0 && ratio != 0.0 {
        amount = if ratio > 0.0 { 1 } else { -1 };
    }
    if amount > 0 {
        amount = amount.max(1);
    } else if amount < 0 {
        amount = amount.min(-1);
    }
    let mut meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
    meta.insert("target".to_string(), Value::String(target_id.clone()));
    meta.insert("cancellable".to_string(), Value::Bool(true));
    vec![BattleEvent::Damage {
        target_id,
        amount,
        meta,
    }]
}

fn apply_hp_based_damage(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    let ratio = value_f64(effect.data.get("ratio"), state, ctx).unwrap_or(0.5);
    let amount = ((target.hp as f64) * ratio).floor() as i32;
    let mut meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
    meta.insert("target".to_string(), Value::String(target_id.clone()));
    meta.insert("cancellable".to_string(), Value::Bool(true));
    vec![BattleEvent::Damage {
        target_id,
        amount: amount.max(1),
        meta,
    }]
}

fn apply_pain_split_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let (Some(user), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    let average = ((user.hp + target.hp) / 2).max(1);
    vec![
        BattleEvent::Damage {
            target_id: ctx.attacker_player_id.clone(),
            amount: user.hp - average,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::Damage {
            target_id: ctx.target_player_id.clone(),
            amount: target.hp - average,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_endeavor_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let (Some(user), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    let amount = (target.hp - user.hp).max(0);
    let mut meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
    meta.insert("target".to_string(), Value::String(ctx.target_player_id.clone()));
    meta.insert("cancellable".to_string(), Value::Bool(true));
    vec![BattleEvent::Damage {
        target_id: ctx.target_player_id.clone(),
        amount,
        meta,
    }]
}

fn apply_final_gambit_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(user) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let amount = user.hp.max(0);
    vec![
        BattleEvent::Damage {
            target_id: ctx.target_player_id.clone(),
            amount,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::Damage {
            target_id: ctx.attacker_player_id.clone(),
            amount,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_reflective_counter(state: &BattleState, ctx: &mut EffectContext<'_>, expected_category: &str) -> Vec<BattleEvent> {
    let Some(user) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let last_amount = user
        .volatile_data
        .get("lastDamageTakenAmount")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(0);
    let last_category = user
        .volatile_data
        .get("lastDamageTakenCategory")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let last_source = user
        .volatile_data
        .get("lastDamageTakenSource")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if last_amount <= 0 || last_category != expected_category || last_source != ctx.target_player_id {
        return vec![BattleEvent::Log {
            message: "しかし うまく きまらなかった！".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    vec![BattleEvent::Damage {
        target_id: ctx.target_player_id.clone(),
        amount: last_amount * 2,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_counter_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    apply_reflective_counter(state, ctx, "physical")
}

fn apply_mirror_coat_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    apply_reflective_counter(state, ctx, "special")
}

fn apply_hp_ratio_damage(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(user) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let ratio = user.hp as f64 / user.max_hp.max(1) as f64;
    let mut chosen_power = 20;
    if let Some(Value::Array(thresholds)) = effect.data.get("thresholds") {
        let mut parsed: Vec<(f64, i32)> = thresholds
            .iter()
            .filter_map(|value| {
                let threshold = value.get("ratio").and_then(|v| v.as_f64())?;
                let power = value.get("power").and_then(|v| v.as_i64())? as i32;
                Some((threshold, power))
            })
            .collect();
        parsed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (threshold, power) in parsed {
            if ratio <= threshold {
                chosen_power = power;
                break;
            }
        }
    }
    let mut cloned = effect.clone();
    cloned.data.insert("power".to_string(), Value::Number(chosen_power.into()));
    apply_damage(state, &cloned, ctx)
}

fn apply_set_atk_max(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(user) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let cost = (user.max_hp / 2).max(1);
    if user.hp <= cost {
        return vec![BattleEvent::Log {
            message: format!("{}は HPが 足りない！", user.name),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    if user.stages.atk >= 6 {
        return vec![BattleEvent::Log {
            message: format!("{}の こうげきは もう あがらない！", user.name),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    vec![
        BattleEvent::Damage {
            target_id: ctx.attacker_player_id.clone(),
            amount: cost,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::ModifyStage {
            target_id: ctx.attacker_player_id.clone(),
            stages: HashMap::from([("atk".to_string(), 6 - user.stages.atk)]),
            clamp: true,
            fail_if_no_change: false,
            show_event: true,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_copy_stages(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let (Some(user), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    let mut stages = HashMap::new();
    for stat in ["atk", "def", "spa", "spd", "spe", "accuracy", "evasion", "crit"] {
        stages.insert(stat.to_string(), stage_value(&target.stages, stat) - stage_value(&user.stages, stat));
    }
    vec![BattleEvent::ModifyStage {
        target_id: ctx.attacker_player_id.clone(),
        stages,
        clamp: true,
        fail_if_no_change: false,
        show_event: true,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_swap_stages(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let stats = stat_list(effect);
    let (Some(user), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    let mut user_delta = HashMap::new();
    let mut target_delta = HashMap::new();
    for stat in stats {
        let user_stage = stage_value(&user.stages, &stat);
        let target_stage = stage_value(&target.stages, &stat);
        user_delta.insert(stat.clone(), target_stage - user_stage);
        target_delta.insert(stat, user_stage - target_stage);
    }
    vec![
        BattleEvent::ModifyStage {
            target_id: ctx.attacker_player_id.clone(),
            stages: user_delta,
            clamp: true,
            fail_if_no_change: false,
            show_event: true,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::ModifyStage {
            target_id: ctx.target_player_id.clone(),
            stages: target_delta,
            clamp: true,
            fail_if_no_change: false,
            show_event: true,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_average_stages(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let stats = stat_list(effect);
    let (Some(user), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    let mut user_delta = HashMap::new();
    let mut target_delta = HashMap::new();
    for stat in stats {
        let user_stage = stage_value(&user.stages, &stat);
        let target_stage = stage_value(&target.stages, &stat);
        let average = (user_stage + target_stage) / 2;
        user_delta.insert(stat.clone(), average - user_stage);
        target_delta.insert(stat, average - target_stage);
    }
    vec![
        BattleEvent::ModifyStage {
            target_id: ctx.attacker_player_id.clone(),
            stages: user_delta,
            clamp: true,
            fail_if_no_change: false,
            show_event: true,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::ModifyStage {
            target_id: ctx.target_player_id.clone(),
            stages: target_delta,
            clamp: true,
            fail_if_no_change: false,
            show_event: true,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_random_stage_boost(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(user) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let candidates: Vec<&str> = ["atk", "def", "spa", "spd", "spe", "accuracy", "evasion"]
        .into_iter()
        .filter(|stat| stage_value(&user.stages, stat) < 6)
        .collect();
    if candidates.is_empty() {
        return vec![BattleEvent::Log {
            message: format!("{}の 能力は もう 上がらない！", user.name),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    let idx = (((ctx.rng)() * candidates.len() as f64).floor() as usize).min(candidates.len() - 1);
    vec![BattleEvent::ModifyStage {
        target_id: ctx.attacker_player_id.clone(),
        stages: HashMap::from([(candidates[idx].to_string(), 2)]),
        clamp: true,
        fail_if_no_change: false,
        show_event: true,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_swap_items(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let (Some(user), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    if !has_item(user) && !has_item(target) {
        return vec![BattleEvent::Log {
            message: "しかし 交換するものが ない！".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    vec![BattleEvent::SwapItems {
        left_id: ctx.attacker_player_id.clone(),
        right_id: ctx.target_player_id.clone(),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_steal_item(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let user_id = resolve_target(effect.data.get("receiver"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    if !has_item(target) {
        return Vec::new();
    }
    if let Some(user) = get_active_creature(state, &user_id) {
        if has_item(user) {
            return Vec::new();
        }
    }
    let Some(item_id) = get_item_id(target) else {
        return Vec::new();
    };
    vec![
        BattleEvent::RemoveStatus {
            target_id: target_id.clone(),
            status_id: "item".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::RemoveStatus {
            target_id,
            status_id: "berry".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::SetItem {
            target_id: user_id,
            item_id: Some(item_id),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_swap_abilities(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let (Some(user), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    if user.ability.is_none() && target.ability.is_none() {
        return Vec::new();
    }
    vec![BattleEvent::SwapAbilities {
        left_id: ctx.attacker_player_id.clone(),
        right_id: ctx.target_player_id.clone(),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_set_ability(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    if get_active_creature(state, &target_id).is_none() {
        return Vec::new();
    }
    let ability_id = effect
        .data
        .get("abilityId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            if effect.data.get("copyUserAbility").and_then(|v| v.as_bool()) == Some(true) {
                get_active_creature(state, &ctx.attacker_player_id).and_then(|c| c.ability.clone())
            } else {
                None
            }
        });
    vec![BattleEvent::SetAbility {
        target_id,
        ability_id,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_suppress_ability(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    if get_active_creature(state, &target_id).is_none() {
        return Vec::new();
    }
    vec![BattleEvent::SetAbility {
        target_id,
        ability_id: None,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_haze_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    state
        .players
        .iter()
        .map(|player| BattleEvent::ResetStages {
            target_id: player.id.clone(),
            show_event: true,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        })
        .collect()
}

fn apply_curse_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(user) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    if user.types.iter().any(|t| t == "ghost") {
        let cost = (user.max_hp / 2).max(1);
        return vec![
            BattleEvent::Damage {
                target_id: ctx.attacker_player_id.clone(),
                amount: cost,
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            },
            BattleEvent::ApplyStatus {
                target_id: ctx.target_player_id.clone(),
                status_id: "curse".to_string(),
                duration: None,
                stack: false,
                data: HashMap::new(),
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            },
        ];
    }
    vec![BattleEvent::ModifyStage {
        target_id: ctx.attacker_player_id.clone(),
        stages: HashMap::from([
            ("atk".to_string(), 1),
            ("def".to_string(), 1),
            ("spe".to_string(), -1),
        ]),
        clamp: true,
        fail_if_no_change: false,
        show_event: true,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_swap_attack_defense(ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    vec![BattleEvent::SwapAttackDefense {
        target_id: ctx.attacker_player_id.clone(),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_delay(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let after_turns = value_i32(effect.data.get("turns"), state, ctx)
        .or_else(|| value_i32(effect.data.get("afterTurns"), state, ctx))
        .unwrap_or(0);
    let trigger_turn = ctx.turn as i32 + after_turns;
    let mut data = HashMap::new();
    data.insert("triggerTurn".to_string(), Value::Number(trigger_turn.into()));
    data.insert("sourceId".to_string(), Value::String(ctx.attacker_player_id.clone()));
    data.insert("targetId".to_string(), Value::String(target_id.clone()));
    let steps_value = effect
        .data
        .get("steps")
        .or_else(|| effect.data.get("then"));
    if let Some(Value::Array(steps_value)) = steps_value {
        data.insert("effects".to_string(), Value::Array(steps_value.clone()));
    }
    if let Some(Value::String(timing)) = effect.data.get("timing") {
        data.insert("timing".to_string(), Value::String(timing.clone()));
    }
    vec![BattleEvent::ApplyStatus {
        target_id,
        status_id: "delayed_effect".to_string(),
        duration: Some(after_turns + 1),
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_over_time(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let mut data = HashMap::new();
    if let Some(Value::Array(steps_value)) = effect.data.get("steps") {
        data.insert("effects".to_string(), Value::Array(steps_value.clone()));
    }
    if let Some(Value::String(timing)) = effect.data.get("timing") {
        data.insert("timing".to_string(), Value::String(timing.clone()));
    }
    data.insert("sourceId".to_string(), Value::String(ctx.attacker_player_id.clone()));
    data.insert("targetId".to_string(), Value::String(target_id.clone()));
    vec![BattleEvent::ApplyStatus {
        target_id,
        status_id: "over_time_effect".to_string(),
        duration: value_i32(effect.data.get("duration"), state, ctx),
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_chance(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let roll = (ctx.rng)();
    let p = value_f64(effect.data.get("p"), state, ctx).unwrap_or(0.0);
    if roll <= p {
        let steps = steps_from_value(effect.data.get("then"));
        return apply_effects(state, &steps, ctx);
    }
    let steps = steps_from_value(effect.data.get("else"));
    apply_effects(state, &steps, ctx)
}

fn apply_repeat(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let mut times = value_i32(effect.data.get("times"), state, ctx)
        .or_else(|| value_i32(effect.data.get("count"), state, ctx))
        .unwrap_or(1);
    if let Some(Value::Object(range)) = effect.data.get("times") {
        let min = range.get("min").and_then(|v| v.as_i64()).unwrap_or(1);
        let max = range.get("max").and_then(|v| v.as_i64()).unwrap_or(min);
        let is_skill_link = run_ability_check_hook(
            state,
            &ctx.attacker_player_id,
            "onSkillLink",
            AbilityCheckContext {
                status_id: None,
                r#type: None,
                target_id: None,
                action: None,
            },
            false,
        );
        if is_skill_link {
            times = max as i32;
        } else {
            let span = (max - min + 1) as f64;
            times = min as i32 + ((ctx.rng)() * span).floor() as i32;
        }
    }

    let steps = steps_from_value(effect.data.get("steps"));
    let mut collected = Vec::new();
    let mut working_state = state.clone();
    let mut hits = 0;
    for _ in 0..times {
        if let Some(target) = get_active_creature(&working_state, &ctx.target_player_id) {
            if target.hp <= 0 {
                break;
            }
        }
        let events = apply_effects(&working_state, &steps, ctx);
        working_state = apply_events(&working_state, &events);
        collected.extend(events);
        hits += 1;
    }
    if hits > 1 {
        collected.push(BattleEvent::Log {
            message: format!("{}回 あたった！", hits),
            meta: Map::new(),
        });
    }
    collected
}

fn apply_conditional(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let condition = effect.data.get("if");
    let result = evaluate_condition(state, condition, ctx);
    let next_key = if result { "then" } else { "else" };
    let steps = steps_from_value(effect.data.get(next_key));
    apply_effects(state, &steps, ctx)
}

fn apply_log(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    if let Some(message) = effect.data.get("message").and_then(|v| v.as_str()) {
        return vec![BattleEvent::Log {
            message: message.to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    Vec::new()
}

fn apply_field_status(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let status_id = match effect.data.get("statusId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Vec::new(),
    };
    let mut data = HashMap::new();
    if let Some(Value::Object(raw)) = effect.data.get("data") {
        for (k, v) in raw {
            data.insert(k.clone(), v.clone());
        }
    }
    if let Some(side) = effect.data.get("side").and_then(|v| v.as_bool()) {
        data.insert("scope".to_string(), Value::String("side".to_string()));
        let side_id = if side {
            ctx.attacker_player_id.clone()
        } else {
            ctx.target_player_id.clone()
        };
        data.insert("sideId".to_string(), Value::String(side_id));
    }
    let apply_event = BattleEvent::ApplyFieldStatus {
        status_id,
        duration: value_i32(effect.data.get("duration"), state, ctx),
        stack: effect.data.get("stack").and_then(|v| v.as_bool()).unwrap_or(false),
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    };
    if let Some(message) = field_status_setup_message(&apply_event) {
        return vec![
            BattleEvent::Log {
                message,
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            },
            apply_event,
        ];
    }
    vec![apply_event]
}

fn field_status_setup_message(event: &BattleEvent) -> Option<String> {
    let BattleEvent::ApplyFieldStatus { status_id, data, .. } = event else {
        return None;
    };
    if data.get("scope").and_then(|v| v.as_str()) != Some("side") {
        return None;
    }
    match status_id.as_str() {
        "stealth_rock" => Some("あいての 周りに 尖った岩が 漂った！".to_string()),
        "sticky_web" => Some("あいての あしもとに ねばねばネットが ちらばった！".to_string()),
        "toxic_spikes" => Some("あいての あしもとに どくびしが ちらばった！".to_string()),
        "spikes" => Some("あいての あしもとに まきびしが ちらばった！".to_string()),
        _ => None,
    }
}

fn apply_remove_field_status(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let status_id = match effect.data.get("statusId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Vec::new(),
    };
    let mut meta = meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id));
    if effect.data.get("side").and_then(|v| v.as_bool()) == Some(true) {
        meta.insert("sideId".to_string(), Value::String(ctx.attacker_player_id.clone()));
    }
    vec![BattleEvent::RemoveFieldStatus {
        status_id,
        meta,
    }]
}

fn apply_random_move(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let pool = effect
        .data
        .get("pool")
        .and_then(|v| v.as_str())
        .unwrap_or("all")
        .to_string();
    vec![BattleEvent::RandomMove {
        pool,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_apply_item(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    let item_id = effect
        .data
        .get("itemId")
        .and_then(|v| v.as_str())
        .unwrap_or("item")
        .to_string();
    let mut data = HashMap::new();
    data.insert("itemId".to_string(), Value::String(item_id.clone()));
    vec![BattleEvent::ApplyStatus {
        target_id,
        status_id: "item".to_string(),
        duration: None,
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }, BattleEvent::Log {
        message: format!("{}は {}を 手に入れた！", target.name, item_id),
        meta: Map::new(),
    }]
}

fn apply_remove_item(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    let had_item = has_item(target);
    vec![
        BattleEvent::Log {
            message: if had_item {
                format!("{}の 持っていた道具が なくなった！", target.name)
            } else {
                format!("{}は 道具を持っていない！", target.name)
            },
            meta: Map::new(),
        },
        BattleEvent::RemoveStatus {
            target_id: target_id.clone(),
            status_id: "item".to_string(),
            meta: Map::new(),
        },
        BattleEvent::RemoveStatus {
            target_id,
            status_id: "berry".to_string(),
            meta: Map::new(),
        },
    ]
}

fn apply_consume_item(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    if !has_item(target) {
        return vec![BattleEvent::Log {
            message: format!("{}は 道具を持っていない！", target.name),
            meta: Map::new(),
        }];
    }
    let item_id = get_item_id(target).unwrap_or_else(|| "item".to_string());
    let mut events = vec![
        BattleEvent::RemoveStatus {
            target_id: target_id.clone(),
            status_id: "item".to_string(),
            meta: Map::new(),
        },
        BattleEvent::RemoveStatus {
            target_id: target_id.clone(),
            status_id: "berry".to_string(),
            meta: Map::new(),
        },
    ];
    if effect.data.get("markBerryConsumed").and_then(|v| v.as_bool()).unwrap_or(false)
        || item_id.contains("berry")
    {
        events.push(BattleEvent::ApplyStatus {
            target_id,
            status_id: "berry_consumed".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        });
    }
    events.push(BattleEvent::Log {
        message: format!("{}の {}が 発動した！", target.name, item_id),
        meta: Map::new(),
    });
    events
}

fn apply_ohko(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(attacker) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let Some(target) = get_active_creature(state, &ctx.target_player_id) else {
        return Vec::new();
    };

    if effect.data.get("respectTypeImmunity").and_then(|v| v.as_bool()).unwrap_or(true)
        && !ctx.ignore_immunity
    {
        if let Some(move_type) = ctx.move_data.and_then(|m| m.move_type.as_deref()) {
            if ctx.type_chart.effectiveness(move_type, &target.types) == 0.0 {
                return vec![BattleEvent::Log {
                    message: "しかし 効かないようだ……".to_string(),
                    meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
                }];
            }
        }
    }

    if let Some(Value::Array(immune_types)) = effect.data.get("immuneTypes") {
        if immune_types.iter().any(|t| t.as_str().map(|s| target.types.iter().any(|ty| ty == s)).unwrap_or(false)) {
            return vec![BattleEvent::Log {
                message: format!("{}は {}には 効かないようだ……", target.name, move_name(ctx.move_data, effect)),
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            }];
        }
    }

    if effect.data.get("failIfTargetHigherLevel").and_then(|v| v.as_bool()).unwrap_or(true)
        && attacker.level < target.level
    {
        return vec![BattleEvent::Log {
            message: format!("{}には 効かないようだ……", move_name(ctx.move_data, effect)),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }

    let base_accuracy = effect
        .data
        .get("baseAccuracy")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.3);
    let mut accuracy = base_accuracy;
    if effect.data.get("levelScaling").and_then(|v| v.as_bool()).unwrap_or(true) {
        accuracy += (attacker.level as f64 - target.level as f64) / 100.0;
    }
    accuracy = accuracy.clamp(0.0, 1.0);

    let move_category = get_move_category(ctx.move_data);
    let accuracy = run_ability_value_hook(
        state,
        &ctx.attacker_player_id,
        "onModifyAccuracy",
        accuracy as f32,
        AbilityValueContext {
            move_data: ctx.move_data,
            category: move_category.as_deref(),
            target: Some(target),
            weather: None,
            turn: ctx.turn,
            stages: None,
        },
    ) as f64;

    let lock_on_applies = attacker.statuses.iter().any(|status| {
        status.id == "lock_on"
            && status
                .data
                .get("targetId")
                .and_then(|v| v.as_str())
                .map(|target| target == ctx.target_player_id.as_str())
                .unwrap_or(false)
    });
    let accuracy = if lock_on_applies { 1.0 } else { accuracy };

    if (ctx.rng)() > accuracy {
        return vec![BattleEvent::Log {
            message: "しかし はずれた！".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }

    vec![
        BattleEvent::Log {
            message: "一撃必殺！".to_string(),
            meta: Map::new(),
        },
        BattleEvent::Damage {
            target_id: ctx.target_player_id.clone(),
            amount: target.hp,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_cure_all_status(effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    vec![BattleEvent::CureAllStatus {
        target_id,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_disable_last_move(state: &BattleState, effect: &Effect, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    let move_id = target
        .volatile_data
        .get("lastMove")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default();
    if move_id.is_empty() {
        return vec![BattleEvent::Log {
            message: "しかし かなしばりに できなかった！".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    }
    let mut data = HashMap::new();
    data.insert("moveId".to_string(), Value::String(move_id));
    vec![BattleEvent::ApplyStatus {
        target_id,
        status_id: "disable_move".to_string(),
        duration: value_i32(effect.data.get("duration"), state, ctx),
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_inverse_speed_based_damage(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let attacker_speed = compute_speed(state, &ctx.attacker_player_id, ctx.turn);
    let target_speed = compute_speed(state, &ctx.target_player_id, ctx.turn);
    let ratio = if attacker_speed <= 0.0 {
        f32::INFINITY
    } else {
        target_speed / attacker_speed
    };
    let mut power = ((25.0 * ratio).floor() as i32).clamp(1, 150);
    if let Some(max_power) = effect.data.get("maxPower").and_then(|v| v.as_i64()) {
        power = power.min(max_power as i32);
    }
    let mut cloned = effect.clone();
    cloned.effect_type = "damage".to_string();
    cloned.data.insert("power".to_string(), Value::Number(power.into()));
    apply_damage(state, &cloned, ctx)
}

fn creature_weight(creature: &crate::core::state::CreatureState) -> f64 {
    if creature.weight_kg > 0.0 {
        creature.weight_kg
    } else {
        // max_hp-based fallback (50.0 kg average if no data)
        50.0
    }
}

fn apply_weight_based_damage(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(target) = get_active_creature(state, &ctx.target_player_id) else {
        return Vec::new();
    };
    let w = creature_weight(target);
    // Official Pokémon weight-to-power table (kg)
    let power = if w < 10.0 {
        20
    } else if w < 25.0 {
        40
    } else if w < 50.0 {
        60
    } else if w < 100.0 {
        80
    } else if w < 200.0 {
        100
    } else {
        120
    };
    let mut cloned = effect.clone();
    cloned.effect_type = "damage".to_string();
    cloned.data.insert("power".to_string(), Value::Number(power.into()));
    apply_damage(state, &cloned, ctx)
}

fn apply_relative_weight_damage(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let (Some(attacker), Some(target)) = (
        get_active_creature(state, &ctx.attacker_player_id),
        get_active_creature(state, &ctx.target_player_id),
    ) else {
        return Vec::new();
    };
    let atk_w = creature_weight(attacker);
    let tgt_w = creature_weight(target).max(0.1);
    let ratio = atk_w / tgt_w;
    // Official Pokémon heavy_slam weight-ratio table
    let power = if ratio >= 5.0 {
        120
    } else if ratio >= 4.0 {
        100
    } else if ratio >= 3.0 {
        80
    } else if ratio >= 2.0 {
        60
    } else {
        40
    };
    let mut cloned = effect.clone();
    cloned.effect_type = "damage".to_string();
    cloned.data.insert("power".to_string(), Value::Number(power.into()));
    apply_damage(state, &cloned, ctx)
}

fn apply_fling_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(attacker) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let Some(item_id) = get_item_id(attacker) else {
        return vec![BattleEvent::Log {
            message: "しかし 投げる道具が なかった！".to_string(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
    };
    let power = match item_id.as_str() {
        "iron_ball" => 130,
        "hard_stone" => 100,
        "poison_barb" | "sharp_beak" | "black_belt" => 70,
        _ => 30,
    };
    let mut damage_effect = Effect {
        effect_type: "damage".to_string(),
        data: Map::new(),
    };
    damage_effect.data.insert("power".to_string(), Value::Number(power.into()));
    damage_effect.data.insert("accuracy".to_string(), Value::Number(1.into()));
    let mut events = apply_damage(state, &damage_effect, ctx);
    events.push(BattleEvent::SetItem {
        target_id: ctx.attacker_player_id.clone(),
        item_id: None,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    });
    events
}

fn apply_beat_up_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(player) = state.players.iter().find(|p| p.id == ctx.attacker_player_id) else {
        return Vec::new();
    };
    // Official: each non-fainted, non-status-affected party member strikes once.
    // Power = member's base Attack / 10 + 5.  We use the stored `attack` stat as base.
    let member_powers: Vec<i32> = player
        .team
        .iter()
        .filter(|c| c.hp > 0 && !c.statuses.iter().any(|s| {
            matches!(s.id.as_str(), "burn" | "poison" | "toxic" | "paralysis" | "freeze" | "sleep")
        }))
        .map(|c| c.attack / 10 + 5)
        .collect();
    let member_powers = if member_powers.is_empty() {
        vec![10] // fallback: at least one hit at power 10
    } else {
        member_powers
    };
    let mut events = Vec::new();
    for power in member_powers {
        let mut damage_effect = Effect {
            effect_type: "damage".to_string(),
            data: Map::new(),
        };
        damage_effect.data.insert("power".to_string(), Value::Number(power.into()));
        damage_effect.data.insert("accuracy".to_string(), Value::Number(1.into()));
        events.extend(apply_damage(state, &damage_effect, ctx));
    }
    events
}

fn apply_imprison_effect(state: &BattleState, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(attacker) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let moves: Vec<Value> = attacker.moves.iter().cloned().map(Value::String).collect();
    let mut data = HashMap::new();
    data.insert("sourceId".to_string(), Value::String(ctx.attacker_player_id.clone()));
    data.insert("moves".to_string(), Value::Array(moves));
    vec![BattleEvent::ApplyStatus {
        target_id: ctx.target_player_id.clone(),
        status_id: "imprison".to_string(),
        duration: Some(5),
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_healing_wish_effect(state: &BattleState, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(attacker) = get_active_creature(state, &ctx.attacker_player_id) else {
        return Vec::new();
    };
    let mut data = HashMap::new();
    data.insert("sideId".to_string(), Value::String(ctx.attacker_player_id.clone()));
    vec![
        BattleEvent::ApplyFieldStatus {
            status_id: "healing_wish".to_string(),
            duration: None,
            stack: false,
            data,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::Damage {
            target_id: ctx.attacker_player_id.clone(),
            amount: attacker.hp,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_strength_sap_effect(state: &BattleState, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(target) = get_active_creature(state, &ctx.target_player_id) else {
        return Vec::new();
    };
    let stage = target.stages.atk.max(-6).min(6);
    let effective_atk = if stage >= 0 {
        (target.attack * (2 + stage) / 2).max(1)
    } else {
        (target.attack * 2 / (2 + stage.abs())).max(1)
    };
    vec![
        BattleEvent::Damage {
            target_id: ctx.attacker_player_id.clone(),
            amount: -effective_atk,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
        BattleEvent::ModifyStage {
            target_id: ctx.target_player_id.clone(),
            stages: HashMap::from([("atk".to_string(), -1)]),
            clamp: true,
            fail_if_no_change: false,
            show_event: true,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        },
    ]
}

fn apply_charge_attack(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let move_id = ctx.move_data.map(|m| m.id.as_str()).unwrap_or("charge_attack");
    let charge_status = effect
        .data
        .get("chargeStatusId")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| format!("charging_{}", move_id));
    let semi_status = effect.data.get("semiStatusId").and_then(|v| v.as_str()).map(str::to_string);
    let charged = get_active_creature(state, &ctx.attacker_player_id)
        .map_or(false, |c| c.statuses.iter().any(|s| s.id == charge_status));
    let skip_charge = evaluate_condition(state, effect.data.get("skipChargeIf"), ctx);
    if charged || skip_charge {
        let mut events = vec![BattleEvent::RemoveStatus {
            target_id: ctx.attacker_player_id.clone(),
            status_id: charge_status,
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        }];
        if let Some(semi_status) = semi_status {
            events.push(BattleEvent::RemoveStatus {
                target_id: ctx.attacker_player_id.clone(),
                status_id: semi_status,
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            });
        }
        let mut damage_effect = effect.clone();
        damage_effect.effect_type = "damage".to_string();
        damage_effect.data.insert(
            "power".to_string(),
            effect.data.get("power").cloned().unwrap_or(Value::Number(0.into())),
        );
        if let Some(accuracy) = effect.data.get("accuracy").cloned() {
            damage_effect.data.insert("accuracy".to_string(), accuracy);
        }
        events.extend(apply_damage(state, &damage_effect, ctx));
        let steps = steps_from_value(effect.data.get("afterSteps"));
        events.extend(apply_effects(state, &steps, ctx));
        return events;
    }

    let mut data = HashMap::new();
    data.insert("moveId".to_string(), Value::String(move_id.to_string()));
    let before_steps = steps_from_value(effect.data.get("beforeSteps"));
    let mut events = apply_effects(state, &before_steps, ctx);
    events.push(BattleEvent::ApplyStatus {
        target_id: ctx.attacker_player_id.clone(),
        status_id: charge_status,
        duration: Some(2),
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    });
    if let Some(semi_status) = semi_status {
        events.push(BattleEvent::ApplyStatus {
            target_id: ctx.attacker_player_id.clone(),
            status_id: semi_status,
            duration: Some(2),
            stack: false,
            data: HashMap::new(),
            meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
        });
    }
    events.push(BattleEvent::Log {
        message: "力を ためている！".to_string(),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    });
    events
}

fn apply_triple_axel_effect(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let accuracy = value_f64(effect.data.get("accuracy"), state, ctx).unwrap_or(0.9);
    let mut events = Vec::new();
    let mut working_state = state.clone();
    let powers = [20, 40, 60];
    let mut hits = 0;
    for power in powers {
        if (ctx.rng)() > accuracy {
            events.push(BattleEvent::Log {
                message: "しかし はずれた！".to_string(),
                meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
            });
            break;
        }
        let mut damage_effect = effect.clone();
        damage_effect.effect_type = "damage".to_string();
        damage_effect.data.insert("power".to_string(), Value::Number(power.into()));
        damage_effect.data.insert("accuracy".to_string(), Value::Number(1.into()));
        let hit_events = apply_damage(&working_state, &damage_effect, ctx);
        working_state = apply_events(&working_state, &hit_events);
        events.extend(hit_events);
        hits += 1;
        if get_active_creature(&working_state, &ctx.target_player_id).map_or(true, |c| c.hp <= 0) {
            break;
        }
    }
    if hits > 1 {
        events.push(BattleEvent::Log {
            message: format!("{}回 あたった！", hits),
            meta: Map::new(),
        });
    }
    events
}

fn apply_self_switch(state: &BattleState, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    if ctx.move_data.map(move_has_damage_step).unwrap_or(false) && ctx.last_damage.unwrap_or(0) <= 0 {
        return Vec::new();
    }

    let Some(player) = state.players.iter().find(|p| p.id == ctx.attacker_player_id) else {
        return Vec::new();
    };
    if player
        .team
        .get(player.active_slot)
        .map_or(true, |creature| creature.hp <= 0)
    {
        return Vec::new();
    }

    let has_available_switch = player
        .team
        .iter()
        .enumerate()
        .any(|(slot, creature)| slot != player.active_slot && creature.hp > 0);
    if !has_available_switch {
        return Vec::new();
    }

    apply_pending_switch(&ctx.attacker_player_id, ctx)
}

fn move_has_damage_step(move_data: &MoveData) -> bool {
    move_data.steps.iter().any(effect_has_damage_step)
}

fn effect_has_damage_step(effect: &Effect) -> bool {
    if matches!(
        effect.effect_type.as_str(),
        "damage"
            | "modify_damage"
            | "damage_ratio"
            | "hp_based_damage"
            | "hp_ratio_damage"
            | "speed_based_damage"
            | "inverse_speed_based_damage"
            | "weight_based_damage"
            | "relative_weight_damage"
            | "ohko"
            | "triple_axel_effect"
            | "charge_attack"
    ) {
        return true;
    }

    ["then", "else", "steps"]
        .iter()
        .filter_map(|key| effect.data.get(*key))
        .any(value_has_damage_step)
}

fn value_has_damage_step(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(value_has_damage_step),
        Value::Object(_) => serde_json::from_value::<Effect>(value.clone())
            .map(|effect| effect_has_damage_step(&effect))
            .unwrap_or(false),
        _ => false,
    }
}

fn apply_force_switch(state: &BattleState, effect: &Effect, ctx: &mut EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let Some(target) = get_active_creature(state, &target_id) else {
        return Vec::new();
    };
    if !ctx.bypass_protect && target.statuses.iter().any(|status| status.id == "protect") {
        return vec![BattleEvent::Log {
            message: format!("{}は 攻撃から 身を 守った！", target.name),
            meta: Map::new(),
        }];
    }
    if ctx.is_sound && target.ability.as_deref() == Some("soundproof") {
        return vec![
            BattleEvent::Log {
                message: format!("{}の 特性『ぼうおん』！", target.name),
                meta: Map::new(),
            },
            BattleEvent::Log {
                message: format!("{}は 音の技を 受けない！", target.name),
                meta: Map::new(),
            },
        ];
    }
    
    // Find the player being forced to switch
    let Some(player) = state.players.iter().find(|p| p.id == target_id) else {
        return Vec::new();
    };
    
    // Collect available slots (not active, HP > 0)
    let available_slots: Vec<usize> = player.team.iter().enumerate()
        .filter(|(i, c)| *i != player.active_slot && c.hp > 0)
        .map(|(i, _)| i)
        .collect();
    
    if available_slots.is_empty() {
        // No Pokémon to switch to
        return vec![BattleEvent::Log {
            message: format!("{} has no Pokémon to switch to!", player.name),
            meta: Map::new(),
        }];
    }
    
    // Randomly select from available slots
    let idx = ((ctx.rng)() * available_slots.len() as f64).floor() as usize;
    let slot = available_slots[idx.min(available_slots.len() - 1)];
    
    vec![BattleEvent::Switch {
        player_id: target_id.clone(),
        slot,
    }]
}

fn apply_replace_pokemon(ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    apply_pending_switch(&ctx.attacker_player_id, ctx)
}

fn apply_pending_switch(target_id: &str, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    vec![BattleEvent::ApplyStatus {
        target_id: target_id.to_string(),
        status_id: "pending_switch".to_string(),
        duration: None,
        stack: false,
        data: HashMap::new(),
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_lock_move(state: &BattleState, effect: &Effect, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    let target_id = resolve_target(effect.data.get("target"), ctx);
    let duration = value_i32(effect.data.get("duration"), state, ctx);
    let mut data = HashMap::new();
    if let Some(Value::Object(raw)) = effect.data.get("data") {
        for (k, v) in raw {
            data.insert(k.clone(), v.clone());
        }
    }
    vec![BattleEvent::ApplyStatus {
        target_id,
        status_id: "lock_move".to_string(),
        duration,
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }]
}

fn apply_run_away() -> Vec<BattleEvent> {
    Vec::new()
}

fn resolve_target(value: Option<&Value>, ctx: &EffectContext<'_>) -> String {
    match value.and_then(|v| v.as_str()) {
        Some("self") => ctx.attacker_player_id.clone(),
        Some("all") => ctx.target_player_id.clone(),
        Some("target") | None => ctx.target_player_id.clone(),
        Some(other) => other.to_string(),
    }
}

fn apply_item_status(state: &BattleState, status_id: &str, target_id: &str, ctx: &EffectContext<'_>) -> Vec<BattleEvent> {
    let Some(target) = get_active_creature(state, target_id) else {
        return Vec::new();
    };
    let item_id = status_id.to_string();
    let mut data = HashMap::new();
    data.insert("itemId".to_string(), Value::String(item_id.clone()));
    vec![BattleEvent::ApplyStatus {
        target_id: target_id.to_string(),
        status_id: "item".to_string(),
        duration: None,
        stack: false,
        data,
        meta: meta_with_move_source(ctx.move_data.map(|m| m.id.as_str()), Some(&ctx.attacker_player_id)),
    }, BattleEvent::Log {
        message: format!("{} gave {} to {}.",
            get_active_creature(state, &ctx.attacker_player_id).map(|c| c.name.clone()).unwrap_or_else(|| "Someone".to_string()),
            item_id,
            target.name),
        meta: Map::new(),
    }]
}

fn value_f64(value: Option<&Value>, state: &BattleState, ctx: &EffectContext<'_>) -> Option<f64> {
    match value? {
        Value::Number(num) => num.as_f64(),
        Value::String(raw) => eval_expression(raw, state, ctx),
        _ => None,
    }
}

fn eval_expression(raw: &str, state: &BattleState, ctx: &EffectContext<'_>) -> Option<f64> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }

    // 1. Variable or Float (if not starting with paren)
    if !s.starts_with('(') {
        if let Some(v) = resolve_variable(s, state, ctx) {
            return Some(v);
        }
        return s.parse::<f64>().ok();
    }

    // 2. Parentheses ( (a op b) )
    if s.starts_with('(') && s.ends_with(')') {
        let inner = s[1..s.len() - 1].trim();

        // Find top-level operator
        let mut depth = 0;
        let mut op_idx = None;
        let mut found_op = ' ';

        for (i, c) in inner.char_indices().rev() {
            match c {
                ')' => depth += 1,
                '(' => depth -= 1,
                '+' | '-' | '*' | '/' | '^' if depth == 0 => {
                    op_idx = Some(i);
                    found_op = c;
                    break;
                }
                _ => {}
            }
        }

        if let Some(idx) = op_idx {
            let left = eval_expression(&inner[..idx], state, ctx)?;
            let right = eval_expression(&inner[idx + 1..], state, ctx)?;
            return match found_op {
                '+' => Some(left + right),
                '-' => Some(left - right),
                '*' => Some(left * right),
                '/' => {
                    if right != 0.0 {
                        Some(left / right)
                    } else {
                        Some(0.0)
                    }
                }
                '^' => Some(left.powf(right)),
                _ => Some(left),
            };
        } else {
            return eval_expression(inner, state, ctx);
        }
    }

    None
}

fn value_i32(value: Option<&Value>, state: &BattleState, ctx: &EffectContext<'_>) -> Option<i32> {
    value_f64(value, state, ctx).map(|v| v.round() as i32)
}

fn resolve_variable(raw: &str, state: &BattleState, ctx: &EffectContext<'_>) -> Option<f64> {
    let key = raw.strip_prefix('$')?;
    match key {
        "user.hp" => get_active_creature(state, &ctx.attacker_player_id).map(|c| c.hp as f64),
        "user.max_hp" => get_active_creature(state, &ctx.attacker_player_id).map(|c| c.max_hp as f64),
        "target.hp" => get_active_creature(state, &ctx.target_player_id).map(|c| c.hp as f64),
        "target.max_hp" => get_active_creature(state, &ctx.target_player_id).map(|c| c.max_hp as f64),
        "damage" | "last_damage" => ctx.last_damage.map(|d| d as f64),
        _ => None,
    }
}

fn steps_from_value(value: Option<&Value>) -> Vec<Effect> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn move_name(move_data: Option<&MoveData>, effect: &Effect) -> String {
    if let Some(name) = move_data.and_then(|m| m.name.clone()) {
        return name;
    }
    effect
        .data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("move")
        .to_string()
}

fn get_move_category(move_data: Option<&MoveData>) -> Option<String> {
    if let Some(move_data) = move_data {
        if let Some(cat) = move_data.category.clone() {
            return Some(cat);
        }
        let has_damage = move_data
            .steps
            .iter()
            .any(|effect| effect.effect_type == "damage");
        return Some(if has_damage { "physical" } else { "status" }.to_string());
    }
    None
}

fn apply_modify_damage(
    events: &mut Vec<BattleEvent>,
    effect: &Effect,
    state: &BattleState,
    ctx: &EffectContext<'_>,
) {
    let multiplier = value_f64(effect.data.get("multiplier"), state, ctx).unwrap_or(1.0);
    if multiplier == 1.0 {
        return;
    }
    for event in events.iter_mut().rev() {
        if let BattleEvent::Damage { amount, .. } = event {
            let scaled = (*amount as f64) * multiplier;
            *amount = scaled.round() as i32;
            break;
        }
    }
}

fn apply_force_crit(
    events: &mut Vec<BattleEvent>,
    effect: &Effect,
    state: &BattleState,
    ctx: &EffectContext<'_>,
) {
    let multiplier = value_f64(effect.data.get("multiplier"), state, ctx)
        .or_else(|| value_f64(effect.data.get("mult"), state, ctx))
        .unwrap_or(1.5);
    for event in events.iter_mut().rev() {
        if let BattleEvent::Damage { amount, .. } = event {
            let scaled = (*amount as f64) * multiplier;
            *amount = scaled.round() as i32;
            break;
        }
    }
}

fn apply_effect_flags(ctx: &mut EffectContext<'_>, effects: &[Effect]) {
    for effect in effects {
        match effect.effect_type.as_str() {
            "bypass_protect" => ctx.bypass_protect = true,
            "ignore_immunity" => ctx.ignore_immunity = true,
            "ignore_ability" => ctx.ignore_ability = true,
            "bypass_substitute" => ctx.bypass_substitute = true,
            "ignore_substitute" => {
                ctx.ignore_substitute = true;
                ctx.bypass_substitute = true;
            }
            "sound" => ctx.is_sound = true,
            _ => {}
        }
    }
}

fn apply_move_tag_flags(ctx: &mut EffectContext<'_>) {
    let Some(move_data) = ctx.move_data else {
        return;
    };
    for tag in &move_data.tags {
        match tag.as_str() {
            "sound" => ctx.is_sound = true,
            "ignore_ability" => ctx.ignore_ability = true,
            "bypass_substitute" | "bypass-substitute" => ctx.bypass_substitute = true,
            _ => {}
        }
    }
}

fn apply_meta_flags(events: &mut [BattleEvent], ctx: &EffectContext<'_>) {
    for event in events {
        if let Some(meta) = event_meta_mut(event) {
            if let Some(category) = ctx.move_data.and_then(|m| m.category.as_deref()) {
                meta.entry("category".to_string())
                    .or_insert_with(|| Value::String(category.to_string()));
            }
            if let Some(move_type) = ctx.move_data.and_then(|m| m.move_type.as_deref()) {
                meta.entry("moveType".to_string())
                    .or_insert_with(|| Value::String(move_type.to_string()));
            }
            if ctx.bypass_protect {
                meta.insert("bypassProtect".to_string(), Value::Bool(true));
            }
            if ctx.ignore_immunity {
                meta.insert("ignoreImmunity".to_string(), Value::Bool(true));
            }
            if ctx.bypass_substitute {
                meta.insert("bypassSubstitute".to_string(), Value::Bool(true));
            }
            if ctx.ignore_substitute {
                meta.insert("ignoreSubstitute".to_string(), Value::Bool(true));
            }
            if ctx.is_sound {
                meta.insert("sound".to_string(), Value::Bool(true));
            }
            if ctx
                .move_data
                .is_some_and(|move_data| move_data.tags.iter().any(|tag| tag == "contact"))
            {
                meta.insert("contact".to_string(), Value::Bool(true));
            }
        }
    }
}

fn update_last_damage_from_events(ctx: &mut EffectContext<'_>, events: &[BattleEvent]) {
    for event in events.iter().rev() {
        if let BattleEvent::Damage { amount, .. } = event {
            ctx.last_damage = Some(*amount);
            break;
        }
    }
}

fn event_meta_mut(event: &mut BattleEvent) -> Option<&mut Map<String, Value>> {
    match event {
        BattleEvent::Log { meta, .. }
        | BattleEvent::Damage { meta, .. }
        | BattleEvent::ApplyStatus { meta, .. }
        | BattleEvent::RemoveStatus { meta, .. }
        | BattleEvent::ReplaceStatus { meta, .. }
        | BattleEvent::ModifyStage { meta, .. }
        | BattleEvent::ClearStages { meta, .. }
        | BattleEvent::ResetStages { meta, .. }
        | BattleEvent::CureAllStatus { meta, .. }
        | BattleEvent::ApplyFieldStatus { meta, .. }
        | BattleEvent::RemoveFieldStatus { meta, .. }
        | BattleEvent::RandomMove { meta, .. }
        | BattleEvent::SetAbility { meta, .. }
        | BattleEvent::SwapAbilities { meta, .. }
        | BattleEvent::SetItem { meta, .. }
        | BattleEvent::SwapItems { meta, .. }
        | BattleEvent::SetStages { meta, .. }
        | BattleEvent::SwapStages { meta, .. }
        | BattleEvent::AverageStats { meta, .. }
        | BattleEvent::SwapAttackDefense { meta, .. } => Some(meta),
        _ => None,
    }
}

fn evaluate_condition(state: &BattleState, cond: Option<&Value>, ctx: &EffectContext<'_>) -> bool {
    let Some(Value::Object(cond_map)) = cond else {
        return false;
    };
    let Some(Value::String(cond_type)) = cond_map.get("type") else {
        return false;
    };
    match cond_type.as_str() {
        "target_has_status" => {
            let target = get_active_creature(state, &ctx.target_player_id);
            let status_id = cond_map.get("statusId").and_then(|v| v.as_str()).unwrap_or("");
            if is_item_status(status_id) {
                return target.map_or(false, |c| has_item(c));
            }
            target.map_or(false, |c| c.statuses.iter().any(|s| s.id == status_id))
        }
        "target_has_any_status" => {
            let target = get_active_creature(state, &ctx.target_player_id);
            let statuses: Vec<String> = cond_map
                .get("statusIds")
                .and_then(|v| v.as_array())
                .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            target.map_or(false, |c| c.statuses.iter().any(|s| statuses.iter().any(|id| id == &s.id)))
        }
        "target_hp_lt" => {
            let target = get_active_creature(state, &ctx.target_player_id);
            if let Some(target) = target {
                let ratio = target.hp as f64 / target.max_hp as f64;
                let value = value_f64(cond_map.get("value"), state, ctx).unwrap_or(0.0);
                ratio < value
            } else {
                false
            }
        }
        "field_has_status" => {
            let status_id = cond_map.get("statusId").and_then(|v| v.as_str()).unwrap_or("");
            state.field.global.iter().any(|e| e.id == status_id)
        }
        "weather_is_sunny" => weather_has_any(state, &["sunny_weather", "sunny_day", "sun"]),
        "weather_is_raining" => weather_has_any(state, &["rain", "rainy_weather", "rain_dance"]),
        "weather_is_hail" => weather_has_any(state, &["hail", "hail_weather", "snow"]),
        "weather_is_sandstorm" => weather_has_any(state, &["sandstorm", "sandstorm_weather"]),
        "user_type" => {
            let type_id = cond_map.get("typeId").and_then(|v| v.as_str()).unwrap_or("");
            get_active_creature(state, &ctx.attacker_player_id)
                .map_or(false, |c| effective_types(c).iter().any(|t| t == type_id))
        }
        "user_has_status" => {
            let status_id = cond_map.get("statusId").and_then(|v| v.as_str()).unwrap_or("");
            get_active_creature(state, &ctx.attacker_player_id)
                .map_or(false, |c| c.statuses.iter().any(|s| s.id == status_id))
        }
        "target_has_item" => get_active_creature(state, &ctx.target_player_id).map_or(false, |c| has_item(c)),
        "user_has_item" => get_active_creature(state, &ctx.attacker_player_id).map_or(false, |c| has_item(c)),
        "user_has_no_item" => get_active_creature(state, &ctx.attacker_player_id).map_or(false, |c| !has_item(c)),
        "target_has_major_status" => {
            let major_statuses = [
                "burn",
                "poison",
                "toxic",
                "badly_poison",
                "badly_poisoned",
                "paralysis",
                "paralyzed",
                "freeze",
                "frozen",
                "sleep",
                "asleep",
                "confusion",
                "confused",
            ];
            get_active_creature(state, &ctx.target_player_id)
                .map_or(false, |c| c.statuses.iter().any(|s| major_statuses.contains(&s.id.as_str())))
        }
        "all" => cond_map
            .get("conditions")
            .and_then(|v| v.as_array())
            .map_or(false, |conditions| conditions.iter().all(|condition| evaluate_condition(state, Some(condition), ctx))),
        "any" => cond_map
            .get("conditions")
            .and_then(|v| v.as_array())
            .map_or(false, |conditions| conditions.iter().any(|condition| evaluate_condition(state, Some(condition), ctx))),
        "not" => !evaluate_condition(state, cond_map.get("condition"), ctx),
        "target_fainted" => get_active_creature(state, &ctx.target_player_id).map_or(false, |c| c.hp <= 0),
        "target_boosted_this_turn" => get_active_creature(state, &ctx.target_player_id).map_or(false, |c| {
            c.volatile_data
                .get("boostedThisTurn")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        }),
        "user_damaged_this_turn" => get_active_creature(state, &ctx.attacker_player_id).map_or(false, |c| {
            c.volatile_data
                .get("damagedThisTurn")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        }),
        "user_last_move_failed" => get_active_creature(state, &ctx.attacker_player_id).map_or(false, |c| {
            c.volatile_data
                .get("lastMoveFailed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        }),
        "user_first_turn_active" => get_active_creature(state, &ctx.attacker_player_id).map_or(false, |c| {
            c.volatile_data
                .get("turnEntered")
                .and_then(|v| v.as_i64())
                .map(|turn| state.turn as i64 - turn <= 1)
                .unwrap_or(state.turn <= 1)
        }),
        "ally_fainted_last_turn" => state
            .field
            .sides
            .get(&ctx.attacker_player_id)
            .map_or(false, |effects| {
                effects.iter().any(|effect| {
                    effect.id == "ally_fainted"
                        && effect
                            .data
                            .get("turn")
                            .and_then(|v| v.as_i64())
                            .is_some_and(|turn| turn + 1 == state.turn as i64)
                })
            }),
        "target_selected_priority_positive" => get_active_creature(state, &ctx.target_player_id).map_or(false, |c| {
            c.volatile_data
                .get("selectedPriority")
                .and_then(|v| v.as_i64())
                .is_some_and(|priority| priority > 0)
        }),
        "target_has_not_acted_this_turn" => get_active_creature(state, &ctx.target_player_id).map_or(false, |c| {
            !c.volatile_data
                .get("actedThisTurn")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        }),
        _ => false,
    }
}

fn weather_has_any(state: &BattleState, ids: &[&str]) -> bool {
    state.field.global.iter().any(|e| ids.contains(&e.id.as_str()))
}

fn compute_speed(state: &BattleState, player_id: &str, turn: u32) -> f32 {
    let Some(creature) = get_active_creature(state, player_id) else {
        return 0.0;
    };
    let stage = creature.stages.spe;
    let mut speed = creature.speed as f32 * stage_multiplier(stage);
    let side_tailwind = state
        .field
        .sides
        .get(player_id)
        .map(|effects| effects.iter().any(|effect| effect.id == "tailwind"))
        .unwrap_or(false);
    let global_tailwind = state.field.global.iter().any(|effect| effect.id == "tailwind");
    if side_tailwind || global_tailwind {
        speed *= 2.0;
    }
    if creature.statuses.iter().any(|s| s.id == "paralysis") {
        speed *= 0.5;
    }
    let weather = crate::core::abilities::get_weather(state);
    speed = run_ability_value_hook(
        state,
        player_id,
        "onModifySpeed",
        speed,
        AbilityValueContext {
            move_data: None,
            category: None,
            target: None,
            weather: weather.as_ref().map(|w| match w {
                WeatherKind::Sun => "sun",
                WeatherKind::Rain => "rain",
                WeatherKind::Sandstorm => "sandstorm",
                WeatherKind::Snow => "snow",
            }),
            turn,
            stages: None,
        },
    );
    speed
}

fn calc_damage(power: i32, state: &BattleState, attacker_id: &str, target_id: &str, ctx: &mut EffectContext<'_>, is_secondary_hit: bool, use_defensive_stat: bool, offensive_stat: Option<&str>) -> (i32, bool) {
    let Some(attacker) = get_active_creature(state, attacker_id) else {
        return (0, false);
    };
    let Some(target) = get_active_creature(state, target_id) else {
        return (0, false);
    };
    let power = power.max(0) as f32;
    if power <= 0.0 {
        return (0, false);
    }

    let category = get_move_category(ctx.move_data).unwrap_or_else(|| "physical".to_string());
    let mut crit_stage = ctx.move_data.and_then(|m| m.crit_rate).unwrap_or(0) as f32;
    crit_stage += attacker.stages.crit as f32;
    crit_stage = run_ability_value_hook(
        state,
        attacker_id,
        "onModifyCritChance",
        crit_stage,
        AbilityValueContext {
            move_data: ctx.move_data,
            category: Some(&category),
            target: Some(target),
            weather: None,
            turn: ctx.turn,
            stages: None,
        },
    );
    // 急所ランクの確率設定
    // ランク0: 1/24 (~4.17%)
    // ランク1: 1/8 (12.5%)
    // ランク2: 1/2 (50%)
    // ランク3+: 100%
    let crit_chance = if crit_stage <= 0.0 {
        1.0 / 24.0
    } else if crit_stage <= 1.0 {
        1.0 / 8.0
    } else if crit_stage <= 2.0 {
        1.0 / 2.0
    } else {
        1.0
    };
    
    let is_crit = if is_secondary_hit {
        false
    } else if crit_chance >= 1.0 {
        true
    } else {
        (ctx.rng)() < crit_chance
    };

    let mut move_power = if ctx.ignore_ability {
        power
    } else {
        run_ability_value_hook(
            state,
            attacker_id,
            "onModifyPower",
            power,
            AbilityValueContext {
                move_data: ctx.move_data,
                category: Some(&category),
                target: Some(target),
                weather: None,
                turn: ctx.turn,
                stages: None,
            },
        )
    };

    if !ctx.ignore_ability {
        move_power = run_ability_value_hook(
            state,
            target_id,
            "onDefensivePower",
            move_power,
            AbilityValueContext {
                move_data: ctx.move_data,
                category: Some(&category),
                target: Some(attacker),
                weather: None,
                turn: ctx.turn,
                stages: None,
            },
        );
    }
    if let Some(move_type) = ctx.move_data.and_then(|m| m.move_type.as_deref()) {
        let weather = crate::core::abilities::get_weather(state);
        match (weather, move_type) {
            (Some(WeatherKind::Sun), "fire") => move_power *= 1.5,
            (Some(WeatherKind::Sun), "water") => move_power *= 0.5,
            (Some(WeatherKind::Rain), "water") => move_power *= 1.5,
            (Some(WeatherKind::Rain), "fire") => move_power *= 0.5,
            _ => {}
        }
        if move_type == "electric"
            && state.field.global.iter().any(|effect| effect.id == "electric_terrain")
            && is_grounded_for_field(attacker)
        {
            move_power *= 1.3;
        }
        if move_type == "grass"
            && state.field.global.iter().any(|effect| effect.id == "grassy_terrain")
            && is_grounded_for_field(attacker)
        {
            move_power *= 1.3;
        }
        if move_type == "dragon"
            && state.field.global.iter().any(|effect| effect.id == "misty_terrain")
            && is_grounded_for_field(target)
        {
            move_power *= 0.5;
        }
    }

    let (offense_key, defense_key, stage_key_offense, stage_key_defense) = if offensive_stat == Some("defense") {
        (attacker.defense, target.defense, attacker.stages.def, target.stages.def)
    } else if offensive_stat == Some("spDefense") || offensive_stat == Some("spd") {
        (attacker.sp_defense, target.defense, attacker.stages.spd, target.stages.def)
    } else if category == "special" && !use_defensive_stat {
        (attacker.sp_attack, target.sp_defense, attacker.stages.spa, target.stages.spd)
    } else if category == "special" {
        (attacker.sp_attack, target.defense, attacker.stages.spa, target.stages.def)
    } else {
        (attacker.attack, target.defense, attacker.stages.atk, target.stages.def)
    };

    let mut atk_stage = stage_key_offense;
    let mut def_stage = stage_key_defense;
    
    // 急所の場合:
    // - 攻撃側の攻撃/特攻マイナスランクを無視
    // - 防御側の防御/特防プラスランクを無視
    if is_crit && atk_stage < 0 {
        atk_stage = 0;
    }
    // 急所の場合、相手の防御・特防上昇ランクを無視（0として扱う）
    if is_crit && def_stage > 0 {
        def_stage = 0;
    }

    if attacker.ability.as_deref() == Some("unaware") {
        def_stage = 0;
    }
    if !ctx.ignore_ability && target.ability.as_deref() == Some("unaware") {
        atk_stage = 0;
    }

    let attack = offense_key as f32 * stage_multiplier(atk_stage);
    let mut defense = (defense_key as f32 * stage_multiplier(def_stage)).max(1.0);

    match crate::core::abilities::get_weather(state) {
        Some(WeatherKind::Sandstorm)
            if category == "special" && target.types.iter().any(|t| t == "rock") =>
        {
            defense *= 1.5;
        }
        Some(WeatherKind::Snow)
            if category == "physical" && target.types.iter().any(|t| t == "ice") =>
        {
            defense *= 1.5;
        }
        _ => {}
    }

    let attack = run_ability_value_hook(
        state,
        attacker_id,
        "onModifyOffense",
        attack,
        AbilityValueContext {
            move_data: ctx.move_data,
            category: Some(&category),
            target: Some(target),
            weather: None,
            turn: ctx.turn,
            stages: Some(atk_stage),
        },
    );

    let defense = if ctx.ignore_ability {
        defense
    } else {
        run_ability_value_hook(
            state,
            target_id,
            "onModifyDefense",
            defense,
            AbilityValueContext {
                move_data: ctx.move_data,
                category: Some(&category),
                target: Some(attacker),
                weather: None,
                turn: ctx.turn,
                stages: Some(def_stage),
            },
        )
    };

    let level = attacker.level as f32;
    let base = (((2.0 * level / 5.0 + 2.0) * move_power * attack / defense) / 50.0 + 2.0).max(1.0);
    // Damage roll uses the official 16-step range [85, 100].
    let roll_index = (((ctx.rng)() * 16.0).floor() as i32).clamp(0, 15);
    let roll = (85 + roll_index) as f32 / 100.0;

    let mut modifier = 1.0;
    if let Some(move_type) = ctx.move_data.and_then(|m| m.move_type.as_deref()) {
        let attacker_types = effective_types(attacker);
        let target_types = effective_types(target);
        if attacker_types.iter().any(|t| t.eq_ignore_ascii_case(move_type)) {
            modifier *= 1.5;
        }
        let gravity_active = state.field.global.iter().any(|e| e.id == "gravity");
        let magnet_rise_active = target.statuses.iter().any(|s| s.id == "magnet_rise");
        if move_type == "ground" && magnet_rise_active && !gravity_active {
            return (0, false);
        }
        let mut effectiveness = ctx.type_chart.effectiveness(move_type, &target_types);
        if let Some(move_data) = ctx.move_data {
            if move_data.id == "freeze_dry" && target_types.iter().any(|t| t == "water") {
                effectiveness *= 4.0;
            }
        }
        if effectiveness == 0.0 {
            if ctx.ignore_immunity || (gravity_active && move_type == "ground") {
                effectiveness = 1.0;
            } else {
                return (0, false);
            }
        }
        modifier *= effectiveness;
    }

    // 壁補正（リフレクター/ひかりのかべ/オーロラベール）
    // まず target 側の side 効果を参照し、無ければ global も参照する。
    let target_side_effects = state.field.sides.get(target_id);
    let side_has = |status_id: &str| {
        target_side_effects
            .map(|effects| effects.iter().any(|e| e.id == status_id))
            .unwrap_or(false)
            || state.field.global.iter().any(|e| e.id == status_id)
    };
    if !is_crit {
        let has_aurora_veil = side_has("aurora_veil");
        if category == "physical" && (side_has("reflect") || has_aurora_veil) {
            modifier *= 0.5;
        }
        if category == "special" && (side_has("light_screen") || has_aurora_veil) {
            modifier *= 0.5;
        }
    }

    if category == "physical"
        && attacker.statuses.iter().any(|s| s.id == "burn")
        && attacker.ability.as_deref() != Some("guts")
    {
        modifier *= 0.5;
    }

    if is_crit {
        modifier *= 1.5;
    }
    let damage = (base * roll * modifier).floor() as i32;
    (damage.max(1), is_crit)
}

fn is_item_status(status_id: &str) -> bool {
    status_id == "item" || status_id == "berry"
}

pub fn has_item(creature: &crate::core::state::CreatureState) -> bool {
    if creature.item.is_some() {
        return true;
    }
    creature
        .statuses
        .iter()
        .any(|s| s.id == "item" || s.id == "berry")
}

fn is_grounded_for_field(creature: &crate::core::state::CreatureState) -> bool {
    (creature.statuses.iter().any(|s| s.id == "roosted")
        || !effective_types(creature).iter().any(|t| t == "flying"))
        && creature.ability.as_deref() != Some("levitate")
        && !creature.statuses.iter().any(|s| s.id == "magnet_rise")
}

fn effective_types(creature: &crate::core::state::CreatureState) -> Vec<String> {
    creature
        .types
        .iter()
        .filter(|type_id| {
            let removed_status = format!("type_removed_{}", type_id);
            !creature.statuses.iter().any(|status| status.id == removed_status)
                && !(type_id.as_str() == "flying"
                    && creature.statuses.iter().any(|status| status.id == "roosted"))
        })
        .cloned()
        .collect()
}

fn get_item_id(creature: &crate::core::state::CreatureState) -> Option<String> {
    if let Some(item) = &creature.item {
        return Some(item.clone());
    }
    creature
        .statuses
        .iter()
        .find(|s| s.id == "item" || s.id == "berry")
        .and_then(|s| s.data.get("itemId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn stat_list(effect: &Effect) -> Vec<String> {
    effect
        .data
        .get("stats")
        .and_then(|v| v.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn stage_value(stages: &crate::core::state::StatStages, stat: &str) -> i32 {
    match stat {
        "atk" => stages.atk,
        "def" => stages.def,
        "spa" => stages.spa,
        "spd" => stages.spd,
        "spe" => stages.spe,
        "accuracy" | "acc" => stages.accuracy,
        "evasion" | "eva" => stages.evasion,
        "crit" => stages.crit,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::{create_battle_state, CreatureState, EVStats, PlayerState, StatStages};
    use serde_json::json;

    fn test_creature(id: &str) -> CreatureState {
        CreatureState {
            id: id.to_string(),
            species_id: id.to_string(),
            name: id.to_string(),
            level: 50,
            types: vec!["normal".to_string()],
            moves: Vec::new(),
            ability: None,
            item: None,
            evs: EVStats::default(),
            hp: 100,
            max_hp: 100,
            stages: StatStages::default(),
            statuses: Vec::new(),
            move_pp: HashMap::new(),
            ability_data: HashMap::new(),
            volatile_data: HashMap::new(),
            attack: 100,
            defense: 100,
            sp_attack: 100,
            sp_defense: 100,
            speed: 100,
            weight_kg: 50.0,
        }
    }

    fn test_state() -> BattleState {
        create_battle_state(vec![
            PlayerState {
                id: "player".to_string(),
                name: "player".to_string(),
                team: vec![test_creature("attacker")],
                active_slot: 0,
                last_fainted_ability: None,
            },
            PlayerState {
                id: "opponent".to_string(),
                name: "opponent".to_string(),
                team: vec![test_creature("target")],
                active_slot: 0,
                last_fainted_ability: None,
            },
        ])
    }

    fn test_move() -> MoveData {
        MoveData {
            id: "secondary_test".to_string(),
            name: Some("Secondary Test".to_string()),
            move_type: Some("normal".to_string()),
            category: Some("physical".to_string()),
            pp: Some(10),
            power: Some(40),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            crit_rate: None,
        }
    }

    fn effect(value: Value) -> Effect {
        serde_json::from_value(value).expect("valid effect")
    }

    fn effect_context<'a>(
        rng: &'a mut dyn FnMut() -> f64,
        type_chart: &'a TypeChart,
        move_data: &'a MoveData,
    ) -> EffectContext<'a> {
        EffectContext {
            attacker_player_id: "player".to_string(),
            target_player_id: "opponent".to_string(),
            move_data: Some(move_data),
            rng,
            turn: 1,
            type_chart,
            bypass_protect: false,
            ignore_immunity: false,
            bypass_substitute: false,
            ignore_substitute: false,
            ignore_ability: false,
            is_sound: false,
            last_damage: None,
        }
    }

    #[test]
    fn missed_damage_does_not_roll_secondary_chance() {
        let state = test_state();
        let type_chart = TypeChart::new();
        let move_data = test_move();
        let mut rng = || 0.5;
        let mut ctx = effect_context(&mut rng, &type_chart, &move_data);
        let steps = vec![
            effect(json!({ "type": "damage", "power": 40, "accuracy": 0.0 })),
            effect(json!({
                "type": "chance",
                "p": 1.0,
                "then": [{ "type": "apply_status", "statusId": "burn" }]
            })),
        ];

        let events = apply_effects(&state, &steps, &mut ctx);

        assert!(events.iter().any(|event| matches!(event, BattleEvent::Log { message, .. } if message == "しかし はずれた！")));
        assert!(!events.iter().any(|event| matches!(event, BattleEvent::ApplyStatus { .. })));
        assert_eq!(ctx.last_damage, Some(0));
    }

    #[test]
    fn hit_damage_allows_secondary_chance() {
        let state = test_state();
        let type_chart = TypeChart::new();
        let move_data = test_move();
        let mut rng = || 0.0;
        let mut ctx = effect_context(&mut rng, &type_chart, &move_data);
        let steps = vec![
            effect(json!({ "type": "damage", "power": 40, "accuracy": 1.0 })),
            effect(json!({
                "type": "chance",
                "p": 1.0,
                "then": [{ "type": "apply_status", "statusId": "burn" }]
            })),
        ];

        let events = apply_effects(&state, &steps, &mut ctx);

        assert!(events.iter().any(|event| matches!(event, BattleEvent::Damage { amount, .. } if *amount > 0)));
        assert!(events.iter().any(|event| matches!(event, BattleEvent::ApplyStatus { status_id, .. } if status_id == "burn")));
    }

    #[test]
    fn damage_reference_uses_actual_hp_lost() {
        let mut state = test_state();
        state.players[1].team[0].hp = 10;
        let type_chart = TypeChart::new();
        let move_data = test_move();
        let mut rng = || 0.0;
        let mut ctx = effect_context(&mut rng, &type_chart, &move_data);
        let steps = vec![
            effect(json!({ "type": "damage_ratio", "ratioMaxHp": 3.0 })),
            effect(json!({ "type": "heal_last_damage", "target": "self", "ratio": 0.5 })),
        ];

        let events = apply_effects(&state, &steps, &mut ctx);

        assert!(events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage { target_id, amount, .. } if target_id == "opponent" && *amount == 10
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage { target_id, amount, .. } if target_id == "player" && *amount == -5
        )));
    }
}
