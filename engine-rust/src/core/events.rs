use crate::core::abilities::{
    ability_label, get_weather, is_weather_id, modify_stages_with_ability, run_ability_check_hook,
    AbilityCheckContext, WeatherKind,
};
use crate::core::state::{BattleState, CreatureState, StatStages, Status};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum BattleEvent {
    Log {
        message: String,
        meta: Map<String, Value>,
    },
    Damage {
        target_id: String,
        amount: i32,
        meta: Map<String, Value>,
    },
    ApplyStatus {
        target_id: String,
        status_id: String,
        duration: Option<i32>,
        stack: bool,
        data: HashMap<String, Value>,
        meta: Map<String, Value>,
    },
    RemoveStatus {
        target_id: String,
        status_id: String,
        meta: Map<String, Value>,
    },
    ReplaceStatus {
        target_id: String,
        from: String,
        to: String,
        duration: Option<i32>,
        data: HashMap<String, Value>,
        meta: Map<String, Value>,
    },
    ModifyStage {
        target_id: String,
        stages: HashMap<String, i32>,
        clamp: bool,
        fail_if_no_change: bool,
        show_event: bool,
        meta: Map<String, Value>,
    },
    ClearStages {
        target_id: String,
        show_event: bool,
        meta: Map<String, Value>,
    },
    ResetStages {
        target_id: String,
        show_event: bool,
        meta: Map<String, Value>,
    },
    CureAllStatus {
        target_id: String,
        meta: Map<String, Value>,
    },
    ApplyFieldStatus {
        status_id: String,
        duration: Option<i32>,
        stack: bool,
        data: HashMap<String, Value>,
        meta: Map<String, Value>,
    },
    RemoveFieldStatus {
        status_id: String,
        meta: Map<String, Value>,
    },
    Switch {
        player_id: String,
        slot: usize,
    },
    RandomMove {
        pool: String,
        meta: Map<String, Value>,
    },
    SetVolatile {
        target_id: String,
        key: String,
        value: Value,
    },
    SetAbility {
        target_id: String,
        ability_id: Option<String>,
        meta: Map<String, Value>,
    },
    SwapAbilities {
        left_id: String,
        right_id: String,
        meta: Map<String, Value>,
    },
    SetItem {
        target_id: String,
        item_id: Option<String>,
        meta: Map<String, Value>,
    },
    SwapItems {
        left_id: String,
        right_id: String,
        meta: Map<String, Value>,
    },
    SetStages {
        target_id: String,
        stages: HashMap<String, i32>,
        meta: Map<String, Value>,
    },
    SwapStages {
        left_id: String,
        right_id: String,
        stage_keys: Vec<String>,
        meta: Map<String, Value>,
    },
    AverageStats {
        left_id: String,
        right_id: String,
        stat_keys: Vec<String>,
        meta: Map<String, Value>,
    },
    SwapAttackDefense {
        target_id: String,
        meta: Map<String, Value>,
    },
}

#[derive(Clone, Debug)]
pub struct EventTransform {
    pub transform_type: String,
    pub from: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub except_source_id: Option<String>,
    pub require_absent_meta: Option<String>,
    pub require_present_meta: Option<String>,
    pub to: Vec<BattleEvent>,
    pub priority: i32,
}

impl Default for EventTransform {
    fn default() -> Self {
        Self {
            transform_type: String::new(),
            from: None,
            target_type: None,
            target_id: None,
            except_source_id: None,
            require_absent_meta: None,
            require_present_meta: None,
            to: Vec::new(),
            priority: 0,
        }
    }
}

pub fn event_type(event: &BattleEvent) -> &str {
    match event {
        BattleEvent::Log { .. } => "log",
        BattleEvent::Damage { .. } => "damage",
        BattleEvent::ApplyStatus { .. } => "apply_status",
        BattleEvent::RemoveStatus { .. } => "remove_status",
        BattleEvent::ReplaceStatus { .. } => "replace_status",
        BattleEvent::ModifyStage { .. } => "modify_stage",
        BattleEvent::ClearStages { .. } => "clear_stages",
        BattleEvent::ResetStages { .. } => "reset_stages",
        BattleEvent::CureAllStatus { .. } => "cure_all_status",
        BattleEvent::ApplyFieldStatus { .. } => "apply_field_status",
        BattleEvent::RemoveFieldStatus { .. } => "remove_field_status",
        BattleEvent::Switch { .. } => "switch",
        BattleEvent::RandomMove { .. } => "random_move",
        BattleEvent::SetVolatile { .. } => "set_volatile",
        BattleEvent::SetAbility { .. } => "set_ability",
        BattleEvent::SwapAbilities { .. } => "swap_abilities",
        BattleEvent::SetItem { .. } => "set_item",
        BattleEvent::SwapItems { .. } => "swap_items",
        BattleEvent::SetStages { .. } => "set_stages",
        BattleEvent::SwapStages { .. } => "swap_stages",
        BattleEvent::AverageStats { .. } => "average_stats",
        BattleEvent::SwapAttackDefense { .. } => "swap_attack_defense",
    }
}

pub fn apply_event(state: &BattleState, event: &BattleEvent) -> BattleState {
    let mut next = state.clone();
    match event {
        BattleEvent::Log { message, .. } => {
            next.log.push(message.clone());
        }
        BattleEvent::Damage {
            target_id, amount, ..
        } => {
            let meta = event_meta(event);
            let source = meta.and_then(|meta| meta_get_string(meta, "source"));
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    if *amount > 0 {
                        let bypass_substitute = meta
                            .and_then(|meta| meta_get_bool(meta, "bypassSubstitute"))
                            .unwrap_or(false);
                        let is_self = source.as_deref() == Some(target_id.as_str());
                        if !bypass_substitute && !is_self {
                            if let Some(index) =
                                active.statuses.iter().position(|s| s.id == "substitute")
                            {
                                let current = active.statuses[index]
                                    .data
                                    .get("hp")
                                    .and_then(|v| v.as_i64())
                                    .map(|v| v as i32)
                                    .unwrap_or_else(|| substitute_hp_from_max(active.max_hp));
                                let remaining = current - *amount;
                                if remaining > 0 {
                                    active.statuses[index]
                                        .data
                                        .insert("hp".to_string(), Value::Number(remaining.into()));
                                    next.log.push(format!(
                                        "{}の みがわりが 攻撃を 受けた！",
                                        active.name
                                    ));
                                } else {
                                    active.statuses.remove(index);
                                    next.log.push(format!(
                                        "{}の みがわりは 壊れてしまった！",
                                        active.name
                                    ));
                                }
                                return next;
                            }
                        }
                    }
                    let mut effective_amount = *amount;
                    let is_move_damage = meta.and_then(|meta| meta.get("moveId")).is_some();
                    let endured = is_move_damage
                        && effective_amount > 0
                        && active.hp > 0
                        && active.hp - effective_amount <= 0
                        && active.statuses.iter().any(|s| s.id == "endure");
                    if endured {
                        effective_amount = (active.hp - 1).max(0);
                        next.log.push(format!("{}は こらえた！", active.name));
                    } else if effective_amount > 0 {
                        effective_amount = effective_amount.min(active.hp.max(0));
                    }

                    let new_hp = active.hp - effective_amount;
                    active.hp = new_hp.clamp(0, active.max_hp);
                    if effective_amount > 0 {
                        if let Some(meta) = event_meta(event) {
                            if meta.get("moveId").is_some() {
                                active.volatile_data.insert(
                                    "lastDamageTakenAmount".to_string(),
                                    Value::Number(effective_amount.into()),
                                );
                                if let Some(category) = meta_get_string(meta, "category") {
                                    active.volatile_data.insert(
                                        "lastDamageTakenCategory".to_string(),
                                        Value::String(category),
                                    );
                                }
                                if let Some(source) = source.clone() {
                                    active.volatile_data.insert(
                                        "lastDamageTakenSource".to_string(),
                                        Value::String(source.clone()),
                                    );
                                    if source != *target_id {
                                        let hits = active
                                            .volatile_data
                                            .get("moveHitsTaken")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(0)
                                            + 1;
                                        active.volatile_data.insert(
                                            "moveHitsTaken".to_string(),
                                            Value::Number(hits.into()),
                                        );
                                    }
                                }
                            }
                        }
                        active
                            .volatile_data
                            .insert("damagedThisTurn".to_string(), Value::Bool(true));
                        next.log.push(format!(
                            "{}は {}ダメージ 受けた！",
                            active.name, effective_amount
                        ));
                    } else if *amount < 0 {
                        next.log
                            .push(format!("{}の HPが {}回復した！", active.name, -amount));
                    } else if endured {
                        active
                            .volatile_data
                            .insert("damagedThisTurn".to_string(), Value::Bool(true));
                    } else {
                        next.log
                            .push(format!("{}には 効かないようだ……", active.name));
                    }
                    if active.hp <= 0 {
                        let had_destiny_bond =
                            active.statuses.iter().any(|s| s.id == "destiny_bond");
                        next.log.push(format!("{}は たおれた！", active.name));
                        player.last_fainted_ability = active.ability.clone();
                        let effects = next.field.sides.entry(target_id.clone()).or_default();
                        effects.retain(|effect| effect.id != "ally_fainted");
                        let mut data = HashMap::new();
                        data.insert("turn".to_string(), Value::Number((next.turn as i64).into()));
                        effects.push(crate::core::state::FieldEffect {
                            id: "ally_fainted".to_string(),
                            remaining_turns: None,
                            data,
                        });
                        if !active.statuses.iter().any(|s| s.id == "pending_switch") {
                            active.statuses.push(Status {
                                id: "pending_switch".to_string(),
                                remaining_turns: None,
                                data: HashMap::new(),
                            });
                        }
                        if had_destiny_bond {
                            if let Some(source_id) = source.clone() {
                                if source_id != *target_id {
                                    if let Some(source_player) =
                                        next.players.iter_mut().find(|p| p.id == source_id)
                                    {
                                        if let Some(source_active) =
                                            source_player.team.get_mut(source_player.active_slot)
                                        {
                                            if source_active.hp > 0 {
                                                source_active.hp = 0;
                                                next.log.push(format!(
                                                    "{}は みちづれに なった！",
                                                    source_active.name
                                                ));
                                                if !source_active
                                                    .statuses
                                                    .iter()
                                                    .any(|s| s.id == "pending_switch")
                                                {
                                                    source_active.statuses.push(Status {
                                                        id: "pending_switch".to_string(),
                                                        remaining_turns: None,
                                                        data: HashMap::new(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        BattleEvent::ApplyStatus {
            target_id,
            status_id,
            duration,
            stack,
            data,
            ..
        } => {
            if let Some(player) = next.players.iter().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get(player.active_slot) {
                    if status_blocked_by_type_or_field(&next, active, status_id) {
                        next.log
                            .push(format!("{}には {}は 効かない！", active.name, status_id));
                        return next;
                    }
                }
            }
            if run_ability_check_hook(
                &next,
                target_id,
                "onCheckStatusImmunity",
                AbilityCheckContext {
                    status_id: Some(status_id),
                    r#type: None,
                    target_id: None,
                    action: None,
                },
                false,
            ) {
                if let Some(player) = next.players.iter().find(|p| p.id == *target_id) {
                    if let Some(active) = player.team.get(player.active_slot) {
                        next.log
                            .push(format!("{}には {}は 効かない！", active.name, status_id));
                    }
                }
                return next;
            }
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    if status_id == "item" || status_id == "berry" {
                        if let Some(Value::String(item_id)) = data.get("itemId") {
                            active.item = Some(item_id.clone());
                        }
                    }
                    if is_exclusive_major_status(status_id)
                        && active
                            .statuses
                            .iter()
                            .any(|status| is_exclusive_major_status(&status.id))
                    {
                        next.log.push("しかしうまく決まらなかった！".to_string());
                        return next;
                    }
                    if !stack {
                        if let Some(_existing) = active.statuses.iter().find(|s| s.id == *status_id)
                        {
                            next.log
                                .push(format!("{}は すでに {}状態だ！", active.name, status_id));
                            return next;
                        }
                    }
                    active.statuses.push(Status {
                        id: status_id.clone(),
                        remaining_turns: *duration,
                        data: data.clone(),
                    });
                    if status_id == "item" || status_id == "berry" {
                        update_unburden_after_item_change(active, false);
                    }
                }
            }
        }
        BattleEvent::RemoveStatus {
            target_id,
            status_id,
            ..
        } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    let had_item = creature_has_item(active);
                    active.statuses.retain(|s| s.id != *status_id);
                    if status_id == "item" || status_id == "berry" {
                        active.item = None;
                        update_unburden_after_item_change(active, had_item);
                    }
                }
            }
        }
        BattleEvent::ReplaceStatus {
            target_id,
            from,
            to,
            duration,
            data,
            ..
        } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    if !active.statuses.iter().any(|s| s.id == *from) {
                        return next;
                    }
                    active.statuses.retain(|s| s.id != *from);
                    active.statuses.push(Status {
                        id: to.clone(),
                        remaining_turns: *duration,
                        data: data.clone(),
                    });
                }
            }
        }
        BattleEvent::ModifyStage {
            target_id,
            stages,
            clamp,
            fail_if_no_change,
            ..
        } => {
            let adjusted = modify_stages_with_ability(&next, target_id, stages);
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    let mut changed = false;
                    let boosted = adjusted.values().any(|delta| *delta > 0);
                    for (key, delta) in adjusted {
                        let stage_ref = stage_ref_mut(&mut active.stages, &key);
                        if let Some(stage_ref) = stage_ref {
                            let mut new_val = *stage_ref + delta;
                            if *clamp {
                                new_val = new_val.clamp(-6, 6);
                            }
                            if new_val != *stage_ref {
                                *stage_ref = new_val;
                                changed = true;
                            }
                        }
                    }
                    if changed && boosted {
                        active
                            .volatile_data
                            .insert("boostedThisTurn".to_string(), Value::Bool(true));
                    }
                    if *fail_if_no_change && !changed {
                        // noop
                    }
                }
            }
        }
        BattleEvent::ClearStages { target_id, .. } | BattleEvent::ResetStages { target_id, .. } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    active.stages = StatStages::default();
                }
            }
        }
        BattleEvent::CureAllStatus { target_id, .. } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    active.statuses.clear();
                }
            }
        }
        BattleEvent::ApplyFieldStatus {
            status_id,
            duration,
            stack,
            data,
            ..
        } => {
            let scope = data
                .get("scope")
                .and_then(|v| v.as_str())
                .unwrap_or("global");
            if scope == "side" {
                let side_id = data
                    .get("sideId")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                if !side_id.is_empty() {
                    let effects = next.field.sides.entry(side_id).or_default();
                    if !*stack {
                        effects.retain(|e| e.id != *status_id);
                    }
                    effects.push(crate::core::state::FieldEffect {
                        id: status_id.clone(),
                        remaining_turns: *duration,
                        data: data.clone(),
                    });
                }
            } else {
                if !*stack {
                    if is_weather_id(status_id) {
                        next.field.global.retain(|e| !is_weather_id(&e.id));
                    }
                    next.field.global.retain(|e| e.id != *status_id);
                }
                next.field.global.push(crate::core::state::FieldEffect {
                    id: status_id.clone(),
                    remaining_turns: *duration,
                    data: data.clone(),
                });
            }
        }
        BattleEvent::RemoveFieldStatus { status_id, meta } => {
            let side_id = meta_get_string(meta, "sideId");
            if let Some(side_id) = side_id {
                if let Some(effects) = next.field.sides.get_mut(&side_id) {
                    effects.retain(|e| e.id != *status_id);
                }
            } else {
                next.field.global.retain(|e| e.id != *status_id);
            }
        }
        BattleEvent::Switch { player_id, slot } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *player_id) {
                if *slot < player.team.len() {
                    let mut baton_pass_stages = None;
                    let mut shed_tail_substitute = None;
                    if let Some(outgoing) = player.team.get_mut(player.active_slot) {
                        if outgoing
                            .volatile_data
                            .get("batonPass")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            baton_pass_stages = Some(outgoing.stages.clone());
                        }
                        if outgoing
                            .volatile_data
                            .get("shedTail")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            shed_tail_substitute = outgoing
                                .statuses
                                .iter()
                                .find(|status| status.id == "substitute")
                                .cloned();
                        }
                        outgoing.stages = StatStages::default();
                        // Non-volatile statuses that persist on switch.
                        let non_volatile =
                            ["burn", "poison", "toxic", "paralysis", "freeze", "sleep"];
                        outgoing
                            .statuses
                            .retain(|s| non_volatile.contains(&s.id.as_str()));
                        for status in &mut outgoing.statuses {
                            if status.id == "toxic" {
                                // Toxic ramp resets when switching out.
                                status.data.remove("counter");
                            }
                        }
                        if let Some(original) = outgoing
                            .ability_data
                            .get("originalAbility")
                            .and_then(|v| v.as_str())
                        {
                            outgoing.ability = Some(original.to_string());
                        }
                        outgoing.ability_data.clear();
                        outgoing.volatile_data.clear();
                    }
                    player.active_slot = *slot;
                    if let Some(incoming) = player.team.get_mut(player.active_slot) {
                        if let Some(stages) = baton_pass_stages {
                            incoming.stages = stages;
                        }
                        incoming.statuses.retain(|s| s.id != "pending_switch");
                        if let Some(substitute) = shed_tail_substitute {
                            incoming.statuses.retain(|s| s.id != "substitute");
                            incoming.statuses.push(substitute);
                        }
                        incoming.volatile_data.insert(
                            "turnEntered".to_string(),
                            Value::Number((next.turn as i64).into()),
                        );
                        let healing_wish = next
                            .field
                            .sides
                            .get_mut(player_id)
                            .and_then(|effects| {
                                let found =
                                    effects.iter().any(|effect| effect.id == "healing_wish");
                                effects.retain(|effect| effect.id != "healing_wish");
                                found.then_some(())
                            })
                            .is_some();
                        if healing_wish {
                            incoming.hp = incoming.max_hp;
                            incoming.statuses.clear();
                            next.log
                                .push(format!("{}は いやしのねがいで 回復した！", incoming.name));
                        }
                        next.log.push(format!(
                            "{}は {}を 繰り出した！",
                            player.name, incoming.name
                        ));
                    }
                }
            }
        }
        BattleEvent::RandomMove { .. } => {
            // Placeholder: move selection handled at action level.
        }
        BattleEvent::SetVolatile {
            target_id,
            key,
            value,
        } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    active.volatile_data.insert(key.clone(), value.clone());
                }
            }
        }
        BattleEvent::SetAbility {
            target_id,
            ability_id,
            ..
        } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    if !active.ability_data.contains_key("originalAbility") {
                        if let Some(current) = &active.ability {
                            active.ability_data.insert(
                                "originalAbility".to_string(),
                                Value::String(current.clone()),
                            );
                        }
                    }
                    active.ability = ability_id.clone();
                    let message = match ability_id {
                        Some(ability_id) => {
                            format!(
                                "{}の 特性は『{}』に なった！",
                                active.name,
                                ability_label(ability_id)
                            )
                        }
                        None => format!("{}の 特性が 消えた！", active.name),
                    };
                    next.log.push(message);
                    if ability_id.as_deref() == Some("slow_start") {
                        next.log
                            .push(format!("{}は 調子が 上がらない！", active.name));
                    }
                }
            }
        }
        BattleEvent::SwapAbilities {
            left_id, right_id, ..
        } => {
            let left_idx = next.players.iter().position(|p| p.id == *left_id);
            let right_idx = next.players.iter().position(|p| p.id == *right_id);
            if let (Some(left_idx), Some(right_idx)) = (left_idx, right_idx) {
                let left_slot = next.players[left_idx].active_slot;
                let right_slot = next.players[right_idx].active_slot;
                let left_current = next.players[left_idx].team[left_slot].ability.clone();
                let right_current = next.players[right_idx].team[right_slot].ability.clone();
                for (idx, current) in [
                    (left_idx, left_current.clone()),
                    (right_idx, right_current.clone()),
                ] {
                    let active_slot = next.players[idx].active_slot;
                    let active = &mut next.players[idx].team[active_slot];
                    if !active.ability_data.contains_key("originalAbility") {
                        if let Some(current) = current {
                            active
                                .ability_data
                                .insert("originalAbility".to_string(), Value::String(current));
                        }
                    }
                }
                next.players[left_idx].team[left_slot].ability = right_current.clone();
                next.players[right_idx].team[right_slot].ability = left_current.clone();
                if right_current.as_deref() == Some("slow_start") {
                    let name = next.players[left_idx].team[left_slot].name.clone();
                    next.log.push(format!("{}は 調子が 上がらない！", name));
                }
                if left_current.as_deref() == Some("slow_start") {
                    let name = next.players[right_idx].team[right_slot].name.clone();
                    next.log.push(format!("{}は 調子が 上がらない！", name));
                }
            }
        }
        BattleEvent::SetItem {
            target_id, item_id, ..
        } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    let had_item = creature_has_item(active);
                    active
                        .statuses
                        .retain(|s| s.id != "item" && s.id != "berry");
                    active.item = item_id.clone();
                    update_unburden_after_item_change(active, had_item);
                }
            }
        }
        BattleEvent::SwapItems {
            left_id, right_id, ..
        } => {
            let left_idx = next.players.iter().position(|p| p.id == *left_id);
            let right_idx = next.players.iter().position(|p| p.id == *right_id);
            if let (Some(left_idx), Some(right_idx)) = (left_idx, right_idx) {
                let left_slot = next.players[left_idx].active_slot;
                let right_slot = next.players[right_idx].active_slot;
                let left_had_item = creature_has_item(&next.players[left_idx].team[left_slot]);
                let right_had_item = creature_has_item(&next.players[right_idx].team[right_slot]);
                let left_item = next.players[left_idx].team[left_slot].item.clone();
                let right_item = next.players[right_idx].team[right_slot].item.clone();
                next.players[left_idx].team[left_slot]
                    .statuses
                    .retain(|s| s.id != "item" && s.id != "berry");
                next.players[right_idx].team[right_slot]
                    .statuses
                    .retain(|s| s.id != "item" && s.id != "berry");
                next.players[left_idx].team[left_slot].item = right_item;
                next.players[right_idx].team[right_slot].item = left_item;
                update_unburden_after_item_change(
                    &mut next.players[left_idx].team[left_slot],
                    left_had_item,
                );
                update_unburden_after_item_change(
                    &mut next.players[right_idx].team[right_slot],
                    right_had_item,
                );
            }
        }
        BattleEvent::SetStages {
            target_id, stages, ..
        } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    for (key, value) in stages {
                        if let Some(stage_ref) = stage_ref_mut(&mut active.stages, key) {
                            *stage_ref = *value;
                        }
                    }
                }
            }
        }
        BattleEvent::SwapStages {
            left_id,
            right_id,
            stage_keys,
            ..
        } => {
            let left_values = current_stage_values(&next, left_id, stage_keys);
            let right_values = current_stage_values(&next, right_id, stage_keys);
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *left_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    for (key, value) in &right_values {
                        if let Some(stage_ref) = stage_ref_mut(&mut active.stages, key) {
                            *stage_ref = *value;
                        }
                    }
                }
            }
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *right_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    for (key, value) in &left_values {
                        if let Some(stage_ref) = stage_ref_mut(&mut active.stages, key) {
                            *stage_ref = *value;
                        }
                    }
                }
            }
        }
        BattleEvent::AverageStats {
            left_id,
            right_id,
            stat_keys,
            ..
        } => {
            let left_values = current_stat_values(&next, left_id, stat_keys);
            let right_values = current_stat_values(&next, right_id, stat_keys);
            let averaged: HashMap<String, i32> = stat_keys
                .iter()
                .map(|key| {
                    let left = left_values.get(key).copied().unwrap_or(0);
                    let right = right_values.get(key).copied().unwrap_or(0);
                    (key.clone(), (left + right) / 2)
                })
                .collect();
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *left_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    for (key, value) in &averaged {
                        if let Some(stat_ref) = stat_ref_mut(active, key) {
                            *stat_ref = *value;
                        }
                    }
                }
            }
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *right_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    for (key, value) in &averaged {
                        if let Some(stat_ref) = stat_ref_mut(active, key) {
                            *stat_ref = *value;
                        }
                    }
                }
            }
        }
        BattleEvent::SwapAttackDefense { target_id, .. } => {
            if let Some(player) = next.players.iter_mut().find(|p| p.id == *target_id) {
                if let Some(active) = player.team.get_mut(player.active_slot) {
                    let already_swapped = active
                        .volatile_data
                        .get("powerTrick")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if already_swapped {
                        let original_attack = active
                            .volatile_data
                            .get("powerTrickAttack")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let original_defense = active
                            .volatile_data
                            .get("powerTrickDefense")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        if let (Some(original_attack), Some(original_defense)) =
                            (original_attack, original_defense)
                        {
                            active.attack = original_attack;
                            active.defense = original_defense;
                        }
                        active.volatile_data.remove("powerTrick");
                        active.volatile_data.remove("powerTrickAttack");
                        active.volatile_data.remove("powerTrickDefense");
                    } else {
                        let original_attack = active.attack;
                        let original_defense = active.defense;
                        active
                            .volatile_data
                            .insert("powerTrick".to_string(), Value::Bool(true));
                        active.volatile_data.insert(
                            "powerTrickAttack".to_string(),
                            Value::Number(original_attack.into()),
                        );
                        active.volatile_data.insert(
                            "powerTrickDefense".to_string(),
                            Value::Number(original_defense.into()),
                        );
                        active.attack = original_defense;
                        active.defense = original_attack;
                    }
                }
            }
        }
    }
    next
}

fn is_exclusive_major_status(status_id: &str) -> bool {
    matches!(
        status_id,
        "burn"
            | "poison"
            | "toxic"
            | "badly_poison"
            | "badly_poisoned"
            | "paralysis"
            | "paralyzed"
            | "freeze"
            | "frozen"
            | "sleep"
            | "asleep"
            | "confusion"
            | "confused"
    )
}

fn status_blocked_by_type_or_field(
    state: &BattleState,
    active: &crate::core::state::CreatureState,
    status_id: &str,
) -> bool {
    let grounded = !active.types.iter().any(|t| t == "flying")
        && active.ability.as_deref() != Some("levitate")
        && !active.statuses.iter().any(|s| s.id == "magnet_rise");
    if grounded
        && status_id == "sleep"
        && state
            .field
            .global
            .iter()
            .any(|effect| effect.id == "electric_terrain")
    {
        return true;
    }
    if grounded
        && matches!(
            status_id,
            "burn" | "poison" | "toxic" | "badly_poisoned" | "paralysis" | "freeze" | "sleep"
        )
        && state
            .field
            .global
            .iter()
            .any(|effect| effect.id == "misty_terrain")
    {
        return true;
    }
    if status_id == "freeze" && matches!(get_weather(state), Some(WeatherKind::Sun)) {
        return true;
    }
    match status_id {
        "burn" => active.types.iter().any(|t| t == "fire"),
        "paralysis" => active.types.iter().any(|t| t == "electric"),
        "poison" | "toxic" | "badly_poisoned" => {
            active.types.iter().any(|t| t == "poison" || t == "steel")
        }
        "leech_seed" => active.types.iter().any(|t| t == "grass"),
        _ => false,
    }
}

fn stage_ref_mut<'a>(stages: &'a mut StatStages, key: &str) -> Option<&'a mut i32> {
    match key {
        "atk" => Some(&mut stages.atk),
        "def" => Some(&mut stages.def),
        "spa" => Some(&mut stages.spa),
        "spd" => Some(&mut stages.spd),
        "spe" => Some(&mut stages.spe),
        "accuracy" | "acc" => Some(&mut stages.accuracy),
        "evasion" | "eva" => Some(&mut stages.evasion),
        "crit" => Some(&mut stages.crit),
        _ => None,
    }
}

fn stat_ref_mut<'a>(
    creature: &'a mut crate::core::state::CreatureState,
    key: &str,
) -> Option<&'a mut i32> {
    match key {
        "atk" | "attack" => Some(&mut creature.attack),
        "def" | "defense" => Some(&mut creature.defense),
        "spa" | "sp_attack" => Some(&mut creature.sp_attack),
        "spd" | "sp_defense" => Some(&mut creature.sp_defense),
        "spe" | "speed" => Some(&mut creature.speed),
        _ => None,
    }
}

fn current_stage_values(
    state: &BattleState,
    player_id: &str,
    stage_keys: &[String],
) -> HashMap<String, i32> {
    let mut values = HashMap::new();
    if let Some(player) = state.players.iter().find(|p| p.id == player_id) {
        if let Some(active) = player.team.get(player.active_slot) {
            for key in stage_keys {
                let value = match key.as_str() {
                    "atk" => active.stages.atk,
                    "def" => active.stages.def,
                    "spa" => active.stages.spa,
                    "spd" => active.stages.spd,
                    "spe" => active.stages.spe,
                    "accuracy" | "acc" => active.stages.accuracy,
                    "evasion" | "eva" => active.stages.evasion,
                    "crit" => active.stages.crit,
                    _ => continue,
                };
                values.insert(key.clone(), value);
            }
        }
    }
    values
}

fn current_stat_values(
    state: &BattleState,
    player_id: &str,
    stat_keys: &[String],
) -> HashMap<String, i32> {
    let mut values = HashMap::new();
    if let Some(player) = state.players.iter().find(|p| p.id == player_id) {
        if let Some(active) = player.team.get(player.active_slot) {
            for key in stat_keys {
                let value = match key.as_str() {
                    "atk" | "attack" => active.attack,
                    "def" | "defense" => active.defense,
                    "spa" | "sp_attack" => active.sp_attack,
                    "spd" | "sp_defense" => active.sp_defense,
                    "spe" | "speed" => active.speed,
                    _ => continue,
                };
                values.insert(key.clone(), value);
            }
        }
    }
    values
}

pub fn meta_with_move_source(move_id: Option<&str>, source: Option<&str>) -> Map<String, Value> {
    let mut meta = Map::new();
    if let Some(move_id) = move_id {
        meta.insert("moveId".to_string(), Value::String(move_id.to_string()));
    }
    if let Some(source) = source {
        meta.insert("source".to_string(), Value::String(source.to_string()));
    }
    meta
}

pub fn meta_get_string(meta: &Map<String, Value>, key: &str) -> Option<String> {
    meta.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub fn meta_get_bool(meta: &Map<String, Value>, key: &str) -> Option<bool> {
    meta.get(key).and_then(|v| v.as_bool())
}

pub fn meta_get_i32(meta: &Map<String, Value>, key: &str) -> Option<i32> {
    meta.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

fn substitute_hp_from_max(max_hp: i32) -> i32 {
    let hp = ((max_hp as f64) * 0.25).floor() as i32;
    hp.max(1)
}

fn event_meta(event: &BattleEvent) -> Option<&Map<String, Value>> {
    match event {
        BattleEvent::Damage { meta, .. }
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

fn creature_has_item(creature: &CreatureState) -> bool {
    creature.item.is_some()
        || creature
            .statuses
            .iter()
            .any(|status| status.id == "item" || status.id == "berry")
}

fn update_unburden_after_item_change(creature: &mut CreatureState, had_item: bool) {
    if creature.ability.as_deref() != Some("unburden") {
        return;
    }
    if creature_has_item(creature) {
        creature.ability_data.remove("unburdenActivated");
    } else if had_item {
        creature
            .ability_data
            .insert("unburdenActivated".to_string(), Value::Bool(true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::{
        BattleState, CreatureState, EVStats, FieldState, PlayerState, StatStages, Status,
    };
    use std::collections::HashMap;

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
        BattleState {
            players: vec![PlayerState {
                id: "player".to_string(),
                name: "player".to_string(),
                team: vec![test_creature("outgoing"), test_creature("incoming")],
                active_slot: 0,
                last_fainted_ability: None,
            }],
            field: FieldState {
                global: Vec::new(),
                sides: HashMap::new(),
            },
            turn: 1,
            log: Vec::new(),
            history: None,
        }
    }

    #[test]
    fn baton_pass_switch_carries_stat_stages_to_incoming_creature() {
        let mut state = test_state();
        state.players[0].team[0].stages.atk = 2;
        state.players[0].team[0].stages.def = -1;
        state.players[0].team[0].stages.spe = 3;
        state.players[0].team[0]
            .volatile_data
            .insert("batonPass".to_string(), Value::Bool(true));

        let next = apply_event(
            &state,
            &BattleEvent::Switch {
                player_id: "player".to_string(),
                slot: 1,
            },
        );

        let outgoing = &next.players[0].team[0];
        let incoming = &next.players[0].team[1];
        assert_eq!(outgoing.stages.atk, 0);
        assert_eq!(incoming.stages.atk, 2);
        assert_eq!(incoming.stages.def, -1);
        assert_eq!(incoming.stages.spe, 3);
    }

    #[test]
    fn shed_tail_switch_carries_substitute_to_incoming_creature() {
        let mut state = test_state();
        state.players[0].team[0].statuses.push(Status {
            id: "substitute".to_string(),
            remaining_turns: None,
            data: HashMap::from([("hp".to_string(), Value::Number(25.into()))]),
        });
        state.players[0].team[0]
            .volatile_data
            .insert("shedTail".to_string(), Value::Bool(true));

        let next = apply_event(
            &state,
            &BattleEvent::Switch {
                player_id: "player".to_string(),
                slot: 1,
            },
        );

        let outgoing = &next.players[0].team[0];
        let incoming = &next.players[0].team[1];
        assert!(!outgoing
            .statuses
            .iter()
            .any(|status| status.id == "substitute"));
        assert!(incoming.statuses.iter().any(|status| {
            status.id == "substitute"
                && status.data.get("hp").and_then(|value| value.as_i64()) == Some(25)
        }));
    }

    #[test]
    fn normal_switch_resets_stat_stages_without_carrying_them() {
        let mut state = test_state();
        state.players[0].team[0].stages.atk = 2;
        state.players[0].team[0].stages.spe = 3;

        let next = apply_event(
            &state,
            &BattleEvent::Switch {
                player_id: "player".to_string(),
                slot: 1,
            },
        );

        assert_eq!(next.players[0].team[0].stages.atk, 0);
        assert_eq!(next.players[0].team[1].stages.atk, 0);
        assert_eq!(next.players[0].team[1].stages.spe, 0);
    }

    #[test]
    fn damage_log_and_tracking_are_capped_to_remaining_hp() {
        let mut state = test_state();
        state.players[0].team[0].hp = 68;

        let mut meta = Map::new();
        meta.insert("moveId".to_string(), Value::String("heavy_hit".to_string()));
        let next = apply_event(
            &state,
            &BattleEvent::Damage {
                target_id: "player".to_string(),
                amount: 100,
                meta,
            },
        );

        let active = &next.players[0].team[0];
        assert_eq!(active.hp, 0);
        assert_eq!(
            active
                .volatile_data
                .get("lastDamageTakenAmount")
                .and_then(|value| value.as_i64()),
            Some(68)
        );
        assert!(
            next.log.iter().any(|line| line.contains("68ダメージ")),
            "damage log should show actual HP lost, not raw incoming damage"
        );
    }
}
