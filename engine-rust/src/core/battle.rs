use crate::core::abilities::{
    apply_ability_event_modifiers, get_weather, run_ability_check_hook, run_ability_hooks,
    run_ability_value_hook, AbilityCheckContext, AbilityHookContext, AbilityValueContext,
};
use crate::core::effects::{apply_effects, apply_events, has_item, EffectContext};
use crate::core::events::{apply_event, event_type, BattleEvent, EventTransform};
use crate::core::state::{Action, ActionType, BattleHistory, BattleState, BattleTurn};
use crate::core::statuses::{
    run_field_hooks, run_status_hooks, tick_field_effects, tick_statuses, StatusHookContext,
};
use crate::core::utils::{
    get_active_creature, get_active_creature_mut, is_status_move, stage_multiplier,
};
use crate::data::moves::{Effect, MoveData, MoveDatabase};
use crate::data::type_chart::TypeChart;
use serde_json::{Map, Value};
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Debug)]
pub struct BattleOptions {
    pub record_history: bool,
}

impl Default for BattleOptions {
    fn default() -> Self {
        Self {
            record_history: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BattleEngine {
    pub move_db: MoveDatabase,
    pub type_chart: TypeChart,
}

impl Default for BattleEngine {
    fn default() -> Self {
        Self {
            move_db: MoveDatabase::default(),
            type_chart: TypeChart::new(),
        }
    }
}

impl BattleEngine {
    pub fn new(move_db: MoveDatabase, type_chart: TypeChart) -> Self {
        Self {
            move_db,
            type_chart,
        }
    }

    pub fn apply_initial_switch_in_effects(
        &self,
        state: &BattleState,
        rng: &mut dyn FnMut() -> f64,
    ) -> BattleState {
        let mut next = state.clone();
        for player in state.players.clone() {
            next = apply_switch_in_effects(next, &player.id, rng, &self.type_chart);
        }
        next
    }

    pub fn step_battle(
        &self,
        state: &BattleState,
        actions: &[Action],
        rng: &mut dyn FnMut() -> f64,
        options: BattleOptions,
    ) -> BattleState {
        let mut next = state.clone();
        next.turn += 1;
        for player in &mut next.players {
            if let Some(active) = player.team.get_mut(player.active_slot) {
                active.volatile_data.remove("damagedThisTurn");
                active.volatile_data.remove("boostedThisTurn");
                active.volatile_data.remove("actedThisTurn");
                active.volatile_data.remove("selectedPriority");
                active.volatile_data.remove("selectedMoveCategory");
            }
        }
        let log_start = next.log.len();
        let mut rng_log = Vec::new();
        let mut rng_recorder = || {
            let v = rng();
            rng_log.push(v);
            v
        };

        next.log.push(format!("--- Turn {} ---", next.turn));

        let ability_start =
            run_all_ability(next.clone(), "onTurnStart", &mut rng_recorder, None, None);
        next = ability_start.state.unwrap_or(next);
        for event in ability_start.events {
            next = apply_event(&next, &event);
        }

        for player in next.players.clone() {
            let status_result = run_status_hooks(
                &next,
                &player.id,
                "onTurnStart",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: None,
                    move_data: None,
                    type_chart: &self.type_chart,
                },
            );
            next = status_result.state.unwrap_or(next);
            for event in status_result.events {
                next = apply_event(&next, &event);
            }
        }

        let field_start = run_field_hooks(
            &next,
            "onTurnStart",
            StatusHookContext {
                rng: &mut rng_recorder,
                action: None,
                move_data: None,
                type_chart: &self.type_chart,
            },
        );
        next = field_start.state.unwrap_or(next);
        for event in field_start.events {
            next = apply_event(&next, &event);
        }

        let mut seen_action_players = HashSet::new();
        let filtered_actions: Vec<Action> = actions
            .iter()
            .filter_map(|action| {
                if seen_action_players.insert(action.player_id.clone()) {
                    Some(action.clone())
                } else {
                    let player_name = next
                        .players
                        .iter()
                        .find(|p| p.id == action.player_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| action.player_id.clone());
                    next.log.push(format!(
                        "{}の 追加アクションは シングルバトルでは 無視される。",
                        player_name
                    ));
                    None
                }
            })
            .collect();

        let mut ordered: VecDeque<OrderedAction> = filtered_actions
            .iter()
            .map(|action| {
                if action.action_type == ActionType::Switch {
                    return OrderedAction {
                        action: action.clone(),
                        priority: 10000,
                        speed: 0,
                        rand: rng_recorder(),
                    };
                }
                let move_data = action
                    .move_id
                    .as_deref()
                    .and_then(|id| self.move_db.get(id));
                let base_priority = move_data.and_then(|m| m.priority).unwrap_or(0) as f32;
                let mut priority = run_ability_value_hook(
                    &next,
                    &action.player_id,
                    "onModifyPriority",
                    base_priority,
                    AbilityValueContext {
                        move_data,
                        category: move_data.and_then(|m| m.category.as_deref()),
                        target: None,
                        weather: None,
                        turn: next.turn,
                        stages: None,
                    },
                );
                if move_data
                    .is_some_and(|m| matches!(m.id.as_str(), "grassy_glide" | "grass_slider"))
                    && next
                        .field
                        .global
                        .iter()
                        .any(|effect| effect.id == "grassy_terrain")
                    && get_active_creature(&next, &action.player_id)
                        .is_some_and(is_grounded_for_field)
                {
                    priority += 1.0;
                }
                let priority = priority.round() as i32;
                OrderedAction {
                    action: action.clone(),
                    priority,
                    speed: creature_speed(&next, &action.player_id),
                    rand: rng_recorder(),
                }
            })
            .collect();

        let trick_room_active = next
            .field
            .global
            .iter()
            .any(|effect| effect.id == "trick_room");
        // Sort into a temporary Vec then convert back to VecDeque
        {
            let mut tmp: Vec<OrderedAction> = ordered.into_iter().collect();
            tmp.sort_by(|a, b| {
                let speed_order = if trick_room_active {
                    a.speed.cmp(&b.speed)
                } else {
                    b.speed.cmp(&a.speed)
                };
                b.priority
                    .cmp(&a.priority)
                    .then_with(|| speed_order)
                    .then_with(|| {
                        a.rand
                            .partial_cmp(&b.rand)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            ordered = tmp.into_iter().collect();
        }
        for ordered_action in &ordered {
            if let Some(active) =
                get_active_creature_mut(&mut next, &ordered_action.action.player_id)
            {
                active.volatile_data.insert(
                    "selectedPriority".to_string(),
                    Value::Number(ordered_action.priority.into()),
                );
                if ordered_action.action.action_type != ActionType::Switch {
                    if let Some(move_data) = ordered_action
                        .action
                        .move_id
                        .as_deref()
                        .and_then(|id| self.move_db.get(id))
                    {
                        if let Some(category) = move_data.category.as_deref() {
                            active.volatile_data.insert(
                                "selectedMoveCategory".to_string(),
                                Value::String(category.to_string()),
                            );
                        }
                    }
                }
            }
        }

        while let Some(ordered_action) = ordered.pop_front() {
            let mut action = ordered_action.action;
            let player_id = action.player_id.clone();
            let attacker_name = next
                .players
                .iter()
                .find(|p| p.id == player_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| player_id.clone());

            if action.action_type != ActionType::Switch {
                if let Some(active) = get_active_creature(&next, &action.player_id) {
                    if active.statuses.iter().any(|s| s.id == "pending_switch") {
                        next.log
                            .push(format!("{}は 交代しなければならない！", attacker_name));
                        continue;
                    }
                }
            }

            if action.action_type == ActionType::Switch {
                let Some(slot) = action.slot else {
                    next.log
                        .push(format!("{} tried to switch without a slot.", attacker_name));
                    continue;
                };
                let Some(player) = next.players.iter().find(|p| p.id == player_id) else {
                    next.log.push(format!(
                        "{} tried to switch but player not found.",
                        attacker_name
                    ));
                    continue;
                };
                if slot >= player.team.len() {
                    next.log.push(format!(
                        "{} tried to switch to an invalid slot.",
                        attacker_name
                    ));
                    continue;
                }
                if slot == player.active_slot {
                    next.log.push(format!(
                        "{} tried to switch to the active slot.",
                        attacker_name
                    ));
                    continue;
                }
                if let Some(target) = player.team.get(slot) {
                    if target.hp <= 0 {
                        next.log.push(format!(
                            "{} tried to switch to a fainted Pokémon.",
                            attacker_name
                        ));
                        continue;
                    }
                }

                if let Some(active) = get_active_creature(&next, &action.player_id) {
                    if active.hp > 0 {
                        let is_ghost = active.types.iter().any(|t| t == "ghost");
                        if !is_ghost {
                            let trapped_by_status = run_status_hooks(
                                &next,
                                &action.player_id,
                                "onTrap",
                                StatusHookContext {
                                    rng: &mut rng_recorder,
                                    action: Some(&action),
                                    move_data: None,
                                    type_chart: &self.type_chart,
                                },
                            )
                            .prevent_action;
                            if trapped_by_status {
                                next.log
                                    .push(format!("{}は 交代できなかった！", attacker_name));
                                continue;
                            }
                            let trapper = next.players.iter().find(|p| {
                                p.id != action.player_id
                                    && run_ability_check_hook(
                                        &next,
                                        &p.id,
                                        "onTrap",
                                        AbilityCheckContext {
                                            status_id: None,
                                            r#type: None,
                                            target_id: Some(&action.player_id),
                                            action: None,
                                        },
                                        false,
                                    )
                            });
                            if trapper.is_some() {
                                next.log
                                    .push(format!("{}は 交代できなかった！", attacker_name));
                                continue;
                            }
                        }
                    }
                }

                next = apply_event(
                    &next,
                    &BattleEvent::Switch {
                        player_id: action.player_id.clone(),
                        slot,
                    },
                );

                let switch_result = run_ability_hooks(
                    &next,
                    &action.player_id,
                    "onSwitchIn",
                    AbilityHookContext {
                        rng: &mut rng_recorder,
                        action: None,
                        move_data: None,
                    },
                );
                next = switch_result.state.unwrap_or(next);
                for event in switch_result.events {
                    next = apply_event(&next, &event);
                }
                next = apply_switch_in_field_effects(next, &action.player_id, &self.type_chart);
                continue;
            }

            if action.action_type == ActionType::UseItem {
                let can_use = run_ability_check_hook(
                    &next,
                    &action.player_id,
                    "onCheckItem",
                    AbilityCheckContext {
                        status_id: None,
                        r#type: None,
                        target_id: None,
                        action: Some(&action),
                    },
                    true,
                );
                if !can_use {
                    next.log
                        .push(format!("{}は 道具を使えない！", attacker_name));
                    continue;
                }
                let Some(active) = get_active_creature(&next, &action.player_id) else {
                    continue;
                };
                if !has_item(active) {
                    next.log
                        .push(format!("{}は 使う道具を 持っていない！", attacker_name));
                    continue;
                }
                next.log.push(format!("{}は 道具を使った！", attacker_name));
                continue;
            }

            let active = get_active_creature(&next, &player_id);
            if active.is_none() || active.unwrap().hp <= 0 {
                next.log.push(format!("{} cannot act.", attacker_name));
                continue;
            }

            let target_id = action.target_id.clone().or_else(|| {
                next.players
                    .iter()
                    .find(|p| p.id != player_id)
                    .map(|p| p.id.clone())
            });
            let Some(target_id) = target_id else {
                next.log
                    .push(format!("No valid target for {}.", attacker_name));
                continue;
            };

            let mut move_id = match action.move_id.as_deref() {
                Some(id) => id.to_string(),
                None => {
                    next.log
                        .push(format!("{} has no move selected.", attacker_name));
                    continue;
                }
            };

            let mut move_data = match self.move_db.get(&move_id) {
                Some(data) => data,
                None => {
                    next.log
                        .push(format!("{} tried unknown move {}.", attacker_name, move_id));
                    continue;
                }
            };

            let ability_before = run_ability_hooks(
                &next,
                &action.player_id,
                "onBeforeAction",
                AbilityHookContext {
                    rng: &mut rng_recorder,
                    action: Some(&action),
                    move_data: Some(move_data),
                },
            );
            if let Some(new_state) = ability_before.state {
                next = new_state;
            }
            for event in ability_before.events {
                next = apply_event(&next, &event);
            }
            if ability_before.prevent_action {
                continue;
            }
            if let Some(override_action) = ability_before.override_action {
                action = override_action;
                if let Some(new_move_id) = action.move_id.as_deref() {
                    if new_move_id != move_id {
                        if let Some(new_move_data) = self.move_db.get(new_move_id) {
                            move_id = new_move_id.to_string();
                            move_data = new_move_data;
                        } else {
                            next.log.push(format!(
                                "{} tried unknown move {}.",
                                attacker_name, new_move_id
                            ));
                            continue;
                        }
                    }
                } else {
                    next.log
                        .push(format!("{} has no move selected.", attacker_name));
                    continue;
                }
            }

            let status_before = run_status_hooks(
                &next,
                &action.player_id,
                "onBeforeAction",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: Some(&action),
                    move_data: Some(move_data),
                    type_chart: &self.type_chart,
                },
            );
            next = status_before.state.unwrap_or(next);
            for event in status_before.events {
                next = apply_event(&next, &event);
            }
            if status_before.prevent_action {
                if let Some(player) = next.players.iter_mut().find(|p| p.id == action.player_id) {
                    if let Some(active) = player.team.get_mut(player.active_slot) {
                        active.statuses.retain(|s| s.id != "flinch");
                    }
                }
                continue;
            }
            if let Some(override_action) = status_before.override_action {
                action = override_action;
                if let Some(new_move_id) = action.move_id.as_deref() {
                    if new_move_id != move_id {
                        if let Some(new_move_data) = self.move_db.get(new_move_id) {
                            move_id = new_move_id.to_string();
                            move_data = new_move_data;
                        } else {
                            next.log.push(format!(
                                "{} tried unknown move {}.",
                                attacker_name, new_move_id
                            ));
                            continue;
                        }
                    }
                } else {
                    next.log
                        .push(format!("{} has no move selected.", attacker_name));
                    continue;
                }
            }

            let field_before = run_field_hooks(
                &next,
                "onBeforeAction",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: Some(&action),
                    move_data: Some(move_data),
                    type_chart: &self.type_chart,
                },
            );
            next = field_before.state.unwrap_or(next);
            for event in field_before.events {
                next = apply_event(&next, &event);
            }

            if !move_data
                .steps
                .iter()
                .any(|e| matches!(e.effect_type.as_str(), "protect" | "endure"))
            {
                if let Some(active) = get_active_creature(&next, &player_id) {
                    if active.volatile_data.get("protectSuccessCount").is_some() {
                        let event = BattleEvent::SetVolatile {
                            target_id: player_id.clone(),
                            key: "protectSuccessCount".to_string(),
                            value: Value::Number(0.into()),
                        };
                        next = apply_event(&next, &event);
                    }
                }
            }

            if let Some(active) = get_active_creature_mut(&mut next, &player_id) {
                if !consume_move_pp(active, &move_id, move_data) {
                    let move_name = move_data.name.clone().unwrap_or_else(|| move_id.clone());
                    next.log.push(format!(
                        "{}の {}は PPが 足りない！",
                        attacker_name, move_name
                    ));
                    continue;
                }
                let previous_move = active
                    .volatile_data
                    .get("lastMove")
                    .and_then(|v| v.as_str());
                let previous_failed = active
                    .volatile_data
                    .get("lastMoveFailed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let previous_count = active
                    .volatile_data
                    .get("consecutiveMoveCount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let consecutive_count =
                    if previous_move == Some(move_id.as_str()) && !previous_failed {
                        previous_count + 1
                    } else {
                        1
                    };
                active.volatile_data.insert(
                    "consecutiveMoveCount".to_string(),
                    Value::Number(consecutive_count.into()),
                );
                active
                    .volatile_data
                    .insert("lastMove".to_string(), Value::String(move_id.clone()));
            }

            let mut effect_ctx = EffectContext {
                attacker_player_id: action.player_id.clone(),
                target_player_id: target_id.clone(),
                move_data: Some(move_data),
                rng: &mut rng_recorder,
                turn: next.turn,
                type_chart: &self.type_chart,
                bypass_protect: false,
                ignore_immunity: false,
                bypass_substitute: false,
                ignore_substitute: false,
                ignore_ability: false,
                is_sound: false,
                last_damage: None,
                switch_slot: action.slot,
            };
            let move_name = move_data.name.as_deref().unwrap_or(&move_id);
            next.log
                .push(format!("{}の {}！", attacker_name, move_name));

            if prankster_blocked_by_dark_type(&next, &player_id, &target_id, move_data) {
                let events = vec![BattleEvent::Log {
                    message: "しかし うまく 決まらなかった！".to_string(),
                    meta: Map::new(),
                }];
                next = apply_events(&next, &events);
                if let Some(active) = get_active_creature_mut(&mut next, &player_id) {
                    active
                        .volatile_data
                        .insert("lastMoveFailed".to_string(), Value::Bool(true));
                    active
                        .volatile_data
                        .insert("actedThisTurn".to_string(), Value::Bool(true));
                }
                continue;
            }

            let mut events = apply_effects(&next, &move_data.steps, &mut effect_ctx);

            events = apply_ability_event_modifiers(&next, &events, self.move_db.as_map());

            let transforms = collect_event_transforms(&next, &mut rng_recorder, &self.type_chart);
            events = apply_event_transforms(&events, &transforms);
            let turn = next.turn;
            events = expand_random_moves(
                &mut next,
                &events,
                &self.move_db,
                &mut rng_recorder,
                &action.player_id,
                &target_id,
                turn,
                &self.type_chart,
            );

            next = apply_events(&next, &events);
            for event in &events {
                if let BattleEvent::Switch { player_id, .. } = event {
                    next = apply_switch_in_effects(
                        next,
                        player_id,
                        &mut rng_recorder,
                        &self.type_chart,
                    );
                }
            }
            let failed = move_failed(&events);
            if let Some(active) = get_active_creature_mut(&mut next, &player_id) {
                active
                    .volatile_data
                    .insert("lastMoveFailed".to_string(), Value::Bool(failed));
                active
                    .volatile_data
                    .insert("actedThisTurn".to_string(), Value::Bool(true));
            }

            // after_you: if any player in the remaining queue has afterYouPending set,
            // move their action to the front so they act next.
            if !ordered.is_empty() {
                let after_you_player: Option<String> = ordered.iter().find_map(|oa| {
                    get_active_creature(&next, &oa.action.player_id)
                        .and_then(|c| c.volatile_data.get("afterYouPending"))
                        .and_then(|v| v.as_bool())
                        .filter(|&b| b)
                        .map(|_| oa.action.player_id.clone())
                });
                if let Some(pid) = after_you_player {
                    // Clear the flag
                    if let Some(player) = next.players.iter_mut().find(|p| p.id == pid) {
                        if let Some(active) = player.team.get_mut(player.active_slot) {
                            active.volatile_data.remove("afterYouPending");
                        }
                    }
                    // Move that player's action to the front
                    if let Some(idx) = ordered.iter().position(|oa| oa.action.player_id == pid) {
                        let bumped = ordered.remove(idx).unwrap();
                        ordered.push_front(bumped);
                    }
                }
            }

            if is_battle_over(&next) {
                break;
            }
        }

        let ability_end = run_all_ability(next.clone(), "onTurnEnd", &mut rng_recorder, None, None);
        next = ability_end.state.unwrap_or(next);
        for event in ability_end.events {
            next = apply_event(&next, &event);
        }

        // ターン終了時効果を順序通りに発動
        // 1. 天気ダメージ
        let weather_result = run_field_hooks(
            &next,
            "onWeatherEnd",
            StatusHookContext {
                rng: &mut rng_recorder,
                action: None,
                move_data: None,
                type_chart: &self.type_chart,
            },
        );
        next = weather_result.state.unwrap_or(next);
        for event in weather_result.events {
            next = apply_event(&next, &event);
        }

        // 2. ねがいごと
        for player in next.players.clone() {
            let wish_result = run_status_hooks(
                &next,
                &player.id,
                "onWishResolve",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: None,
                    move_data: None,
                    type_chart: &self.type_chart,
                },
            );
            next = wish_result.state.unwrap_or(next);
            for event in wish_result.events {
                next = apply_event(&next, &event);
            }
        }

        // 3. グラスフィールド回復
        let grassy_result = run_field_hooks(
            &next,
            "onGrassyTerrainHeal",
            StatusHookContext {
                rng: &mut rng_recorder,
                action: None,
                move_data: None,
                type_chart: &self.type_chart,
            },
        );
        next = grassy_result.state.unwrap_or(next);
        for event in grassy_result.events {
            next = apply_event(&next, &event);
        }

        // 4. 道具効果（たべのこし、くろいヘドロ）
        for player in next.players.clone() {
            let item_result = run_status_hooks(
                &next,
                &player.id,
                "onItemEndTurn",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: None,
                    move_data: None,
                    type_chart: &self.type_chart,
                },
            );
            next = item_result.state.unwrap_or(next);
            for event in item_result.events {
                next = apply_event(&next, &event);
            }
        }

        // 5. やどりぎのタネ
        for player in next.players.clone() {
            let leech_result = run_status_hooks(
                &next,
                &player.id,
                "onLeechSeed",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: None,
                    move_data: None,
                    type_chart: &self.type_chart,
                },
            );
            next = leech_result.state.unwrap_or(next);
            for event in leech_result.events {
                next = apply_event(&next, &event);
            }
        }

        // 6. 状態異常ダメージ（どく、やけど）
        for player in next.players.clone() {
            let status_result = run_status_hooks(
                &next,
                &player.id,
                "onStatusDamage",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: None,
                    move_data: None,
                    type_chart: &self.type_chart,
                },
            );
            next = status_result.state.unwrap_or(next);
            for event in status_result.events {
                next = apply_event(&next, &event);
            }
        }

        // 7. バインドダメージ
        for player in next.players.clone() {
            let bind_result = run_status_hooks(
                &next,
                &player.id,
                "onBindDamage",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: None,
                    move_data: None,
                    type_chart: &self.type_chart,
                },
            );
            next = bind_result.state.unwrap_or(next);
            for event in bind_result.events {
                next = apply_event(&next, &event);
            }
        }

        // その他のターン終了時効果（混乱解除など）
        for player in next.players.clone() {
            let result = run_status_hooks(
                &next,
                &player.id,
                "onTurnEnd",
                StatusHookContext {
                    rng: &mut rng_recorder,
                    action: None,
                    move_data: None,
                    type_chart: &self.type_chart,
                },
            );
            next = result.state.unwrap_or(next);
            for event in result.events {
                next = apply_event(&next, &event);
            }
        }

        let field_end = run_field_hooks(
            &next,
            "onTurnEnd",
            StatusHookContext {
                rng: &mut rng_recorder,
                action: None,
                move_data: None,
                type_chart: &self.type_chart,
            },
        );
        next = field_end.state.unwrap_or(next);
        for event in field_end.events {
            next = apply_event(&next, &event);
        }

        next = tick_statuses(&next);
        next = tick_field_effects(&next);

        if options.record_history {
            let turn_log = next.log[log_start..].to_vec();
            let history = next
                .history
                .get_or_insert(BattleHistory { turns: Vec::new() });
            history.turns.push(BattleTurn {
                turn: next.turn,
                actions: actions.to_vec(),
                log: turn_log,
                rng: rng_log,
            });
        }

        next
    }

    pub fn replace_fainted_pokemon(
        &self,
        state: &BattleState,
        player_id: &str,
        slot: usize,
        rng: &mut dyn FnMut() -> f64,
    ) -> BattleState {
        let mut next = state.clone();
        let Some(player) = next.players.iter().find(|p| p.id == player_id) else {
            next.log.push(format!(
                "{} tried to replace pokemon but player not found.",
                player_id
            ));
            return next;
        };
        if slot >= player.team.len() {
            next.log.push(format!(
                "{} tried to replace with an invalid slot.",
                player.name
            ));
            return next;
        }
        if slot == player.active_slot {
            next.log.push(format!(
                "{} tried to replace with the active slot.",
                player.name
            ));
            return next;
        }
        if player
            .team
            .get(slot)
            .is_some_and(|creature| creature.hp <= 0)
        {
            next.log.push(format!(
                "{} tried to replace with a fainted Pokémon.",
                player.name
            ));
            return next;
        }

        let Some(active) = get_active_creature(&next, player_id) else {
            return next;
        };
        let must_replace = active.hp <= 0
            || active
                .statuses
                .iter()
                .any(|status| status.id == "pending_switch");
        if !must_replace {
            next.log.push(format!("{}は まだ戦える！", active.name));
            return next;
        }

        next = apply_event(
            &next,
            &BattleEvent::Switch {
                player_id: player_id.to_string(),
                slot,
            },
        );

        let switch_result = run_ability_hooks(
            &next,
            player_id,
            "onSwitchIn",
            AbilityHookContext {
                rng,
                action: None,
                move_data: None,
            },
        );
        next = switch_result.state.unwrap_or(next);
        for event in switch_result.events {
            next = apply_event(&next, &event);
        }
        next = apply_switch_in_field_effects(next, player_id, &self.type_chart);
        next
    }
}

fn apply_switch_in_effects(
    mut state: BattleState,
    player_id: &str,
    rng: &mut dyn FnMut() -> f64,
    type_chart: &TypeChart,
) -> BattleState {
    let switch_result = run_ability_hooks(
        &state,
        player_id,
        "onSwitchIn",
        AbilityHookContext {
            rng,
            action: None,
            move_data: None,
        },
    );
    state = switch_result.state.unwrap_or(state);
    for event in switch_result.events {
        state = apply_event(&state, &event);
    }
    apply_switch_in_field_effects(state, player_id, type_chart)
}

fn move_failed(events: &[BattleEvent]) -> bool {
    if events.iter().any(|event| matches!(event, BattleEvent::Log { message, .. } if message.contains("はずれた") || message.contains("効かない"))) {
        return true;
    }
    let has_progress = events.iter().any(|event| match event {
        BattleEvent::Damage { amount, .. } => *amount > 0,
        BattleEvent::ApplyStatus { .. }
        | BattleEvent::ReplaceStatus { .. }
        | BattleEvent::ModifyStage { .. }
        | BattleEvent::ClearStages { .. }
        | BattleEvent::ResetStages { .. }
        | BattleEvent::CureAllStatus { .. }
        | BattleEvent::ApplyFieldStatus { .. }
        | BattleEvent::RemoveFieldStatus { .. }
        | BattleEvent::Switch { .. }
        | BattleEvent::SetVolatile { .. }
        | BattleEvent::SetAbility { .. }
        | BattleEvent::SwapAbilities { .. }
        | BattleEvent::SetItem { .. }
        | BattleEvent::SwapItems { .. }
        | BattleEvent::SetStages { .. }
        | BattleEvent::SwapStages { .. }
        | BattleEvent::AverageStats { .. }
        | BattleEvent::SwapAttackDefense { .. } => true,
        _ => false,
    });
    !has_progress
}

fn prankster_blocked_by_dark_type(
    state: &BattleState,
    attacker_id: &str,
    target_id: &str,
    move_data: &MoveData,
) -> bool {
    let Some(attacker) = get_active_creature(state, attacker_id) else {
        return false;
    };
    if attacker.ability.as_deref() != Some("prankster") || !is_status_move(move_data) {
        return false;
    }
    if !move_targets_opposing_active(move_data) {
        return false;
    }
    get_active_creature(state, target_id).is_some_and(|target| {
        effective_types_for_field(target)
            .iter()
            .any(|type_id| type_id == "dark")
    })
}

fn move_targets_opposing_active(move_data: &MoveData) -> bool {
    move_data.steps.iter().any(effect_targets_opposing_active)
}

fn effect_targets_opposing_active(effect: &Effect) -> bool {
    if effect_is_opposing_active_effect(effect) {
        let target = effect
            .data
            .get("target")
            .and_then(|value| value.as_str())
            .unwrap_or("target");
        if matches!(target, "target" | "opponent" | "all") {
            return true;
        }
    }

    ["then", "else", "steps", "beforeSteps", "afterSteps"]
        .iter()
        .filter_map(|key| effect.data.get(*key))
        .any(value_targets_opposing_active)
}

fn value_targets_opposing_active(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(value_targets_opposing_active),
        Value::Object(_) => serde_json::from_value::<Effect>(value.clone())
            .map(|effect| effect_targets_opposing_active(&effect))
            .unwrap_or(false),
        _ => false,
    }
}

fn effect_is_opposing_active_effect(effect: &Effect) -> bool {
    matches!(
        effect.effect_type.as_str(),
        "apply_status"
            | "replace_status"
            | "remove_status"
            | "modify_stage"
            | "clear_stages"
            | "reset_stages"
            | "set_ability"
            | "swap_abilities"
            | "set_item"
            | "swap_items"
            | "copy_stages"
            | "swap_stages"
            | "average_stages"
            | "disable_move"
            | "disable_last_move"
            | "force_switch"
    )
}

fn apply_switch_in_field_effects(
    mut state: BattleState,
    player_id: &str,
    type_chart: &TypeChart,
) -> BattleState {
    let effects = state
        .field
        .sides
        .get(player_id)
        .cloned()
        .unwrap_or_default();
    if effects.is_empty() {
        return state;
    }
    let Some(active) = get_active_creature(&state, player_id).cloned() else {
        return state;
    };
    if active.hp <= 0 {
        return state;
    }
    let grounded = is_grounded_for_field(&active);
    let mut events = Vec::new();
    let toxic_spikes_layers = effects
        .iter()
        .filter(|effect| effect.id == "toxic_spikes")
        .count();
    let spikes_layers = effects
        .iter()
        .filter(|effect| effect.id == "spikes")
        .count()
        .min(3);
    let mut spikes_handled = false;
    let mut toxic_spikes_handled = false;
    if grounded
        && toxic_spikes_layers > 0
        && active.types.iter().any(|pokemon_type| pokemon_type == "poison")
    {
        let mut meta = Map::new();
        meta.insert("sideId".to_string(), Value::String(player_id.to_string()));
        events.push(BattleEvent::Log {
            message: "足元の どくびしが 消え去った！".to_string(),
            meta: Map::new(),
        });
        events.push(BattleEvent::RemoveFieldStatus {
            status_id: "toxic_spikes".to_string(),
            meta,
        });
        toxic_spikes_handled = true;
    }
    for effect in effects {
        match effect.id.as_str() {
            "spikes" if grounded => {
                if !spikes_handled {
                    events.push(BattleEvent::Log {
                        message: format!("まきびしが 相手の {}に くいこんだ！", active.name),
                        meta: Map::new(),
                    });
                    let denominator = match spikes_layers {
                        0 | 1 => 8,
                        2 => 6,
                        _ => 4,
                    };
                    events.push(BattleEvent::Damage {
                        target_id: player_id.to_string(),
                        amount: (active.max_hp / denominator).max(1),
                        meta: Map::new(),
                    });
                    spikes_handled = true;
                }
            }
            "stealth_rock" => {
                let effectiveness =
                    type_chart.effectiveness("rock", &effective_types_for_field(&active));
                let amount = ((active.max_hp as f32 / 8.0) * effectiveness).floor() as i32;
                events.push(BattleEvent::Log {
                    message: format!("{}に尖った岩がくいこんだ！", active.name),
                    meta: Map::new(),
                });
                events.push(BattleEvent::Damage {
                    target_id: player_id.to_string(),
                    amount: amount.max(1),
                    meta: Map::new(),
                });
            }
            "toxic_spikes" if grounded && !toxic_spikes_handled => {
                toxic_spikes_handled = true;
                let status_id = if toxic_spikes_layers >= 2 {
                    "toxic"
                } else {
                    "poison"
                };
                if can_be_poisoned_by_toxic_spikes(&state, &active) {
                    let message = if status_id == "toxic" {
                        format!("{}は猛毒をあびた！", active.name)
                    } else {
                        format!("{}は毒をあびた！", active.name)
                    };
                    events.push(BattleEvent::Log {
                        message,
                        meta: Map::new(),
                    });
                }
                events.push(BattleEvent::ApplyStatus {
                    target_id: player_id.to_string(),
                    status_id: status_id.to_string(),
                    duration: None,
                    stack: false,
                    data: std::collections::HashMap::new(),
                    meta: Map::new(),
                });
            }
            "sticky_web" if grounded => {
                events.push(BattleEvent::Log {
                    message: format!("{}はねばねばネットにひっかかった！", active.name),
                    meta: Map::new(),
                });
                events.push(BattleEvent::ModifyStage {
                    target_id: player_id.to_string(),
                    stages: std::collections::HashMap::from([("spe".to_string(), -1)]),
                    clamp: true,
                    fail_if_no_change: false,
                    show_event: true,
                    meta: Map::new(),
                });
            }
            _ => {}
        }
    }
    for event in events {
        state = apply_event(&state, &event);
    }
    state
}

fn can_be_poisoned_by_toxic_spikes(
    state: &BattleState,
    active: &crate::core::state::CreatureState,
) -> bool {
    !active.types.iter().any(|t| t == "poison" || t == "steel")
        && !state
            .field
            .global
            .iter()
            .any(|effect| effect.id == "misty_terrain")
}

#[derive(Clone, Debug)]
struct OrderedAction {
    action: Action,
    priority: i32,
    speed: i32,
    rand: f64,
}

pub fn step_battle(
    state: &BattleState,
    actions: &[Action],
    rng: &mut dyn FnMut() -> f64,
    options: BattleOptions,
) -> BattleState {
    BattleEngine::default().step_battle(state, actions, rng, options)
}

pub fn apply_initial_switch_in_effects(
    state: &BattleState,
    rng: &mut dyn FnMut() -> f64,
) -> BattleState {
    BattleEngine::default().apply_initial_switch_in_effects(state, rng)
}

pub fn replace_fainted_pokemon(
    state: &BattleState,
    player_id: &str,
    slot: usize,
    rng: &mut dyn FnMut() -> f64,
) -> BattleState {
    BattleEngine::default().replace_fainted_pokemon(state, player_id, slot, rng)
}

pub fn is_battle_over(state: &BattleState) -> bool {
    for player in &state.players {
        let alive = player.team.iter().any(|c| c.hp > 0);
        if !alive {
            return true;
        }
    }
    false
}

pub fn determine_winner(state: &BattleState) -> Option<String> {
    if state.players.is_empty() {
        return None;
    }

    let alive_by_player: Vec<bool> = state
        .players
        .iter()
        .map(|player| player.team.iter().any(|creature| creature.hp > 0))
        .collect();
    let alive_count = alive_by_player.iter().filter(|alive| **alive).count();

    if alive_count == 1 {
        return alive_by_player
            .iter()
            .enumerate()
            .find_map(|(index, alive)| {
                if *alive {
                    Some(state.players[index].id.clone())
                } else {
                    None
                }
            });
    }

    if alive_count != 0 || state.players.len() != 2 {
        return None;
    }

    // Simultaneous faint rule fallback:
    // the creature that would be processed first faints first and loses.
    let p1 = &state.players[0];
    let p2 = &state.players[1];
    let p1_speed = creature_speed(state, &p1.id);
    let p2_speed = creature_speed(state, &p2.id);
    if p1_speed == p2_speed {
        return None;
    }

    let trick_room_active = state
        .field
        .global
        .iter()
        .any(|effect| effect.id == "trick_room");
    let first_faint_id = if trick_room_active {
        if p1_speed < p2_speed {
            &p1.id
        } else {
            &p2.id
        }
    } else if p1_speed > p2_speed {
        &p1.id
    } else {
        &p2.id
    };

    if first_faint_id == &p1.id {
        Some(p2.id.clone())
    } else {
        Some(p1.id.clone())
    }
}

pub fn determine_timeout_winner(state: &BattleState) -> Option<String> {
    if state.players.len() != 2 {
        return None;
    }

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct TimeoutScore {
        alive_count: i32,
        hp_ratio_milli_sum: i64,
        total_hp: i32,
    }

    let score = |player: &crate::core::state::PlayerState| -> TimeoutScore {
        let mut alive_count = 0;
        let mut hp_ratio_milli_sum = 0_i64;
        let mut total_hp = 0_i32;
        for creature in &player.team {
            let hp = creature.hp.max(0);
            total_hp += hp;
            if hp > 0 {
                alive_count += 1;
            }
            let max_hp = creature.max_hp.max(1) as i64;
            hp_ratio_milli_sum += (hp as i64 * 1000) / max_hp;
        }
        TimeoutScore {
            alive_count,
            hp_ratio_milli_sum,
            total_hp,
        }
    };

    let p1 = &state.players[0];
    let p2 = &state.players[1];
    let s1 = score(p1);
    let s2 = score(p2);

    use std::cmp::Ordering;
    match s1.cmp(&s2) {
        Ordering::Greater => Some(p1.id.clone()),
        Ordering::Less => Some(p2.id.clone()),
        Ordering::Equal => None,
    }
}

fn creature_speed(state: &BattleState, player_id: &str) -> i32 {
    let creature = get_active_creature(state, player_id);
    let Some(creature) = creature else {
        return 0;
    };
    let mut speed = creature.speed as f32 * stage_multiplier(creature.stages.spe);
    let side_tailwind = state
        .field
        .sides
        .get(player_id)
        .map(|effects| effects.iter().any(|effect| effect.id == "tailwind"))
        .unwrap_or(false);
    let global_tailwind = state
        .field
        .global
        .iter()
        .any(|effect| effect.id == "tailwind");
    if side_tailwind || global_tailwind {
        speed *= 2.0;
    }
    if creature.statuses.iter().any(|s| s.id == "paralysis") {
        speed *= 0.5;
    }
    let weather = get_weather(state);
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
                crate::core::abilities::WeatherKind::Sun => "sun",
                crate::core::abilities::WeatherKind::Rain => "rain",
                crate::core::abilities::WeatherKind::Sandstorm => "sandstorm",
                crate::core::abilities::WeatherKind::Snow => "snow",
            }),
            turn: state.turn,
            stages: None,
        },
    );
    speed.round() as i32
}

fn is_grounded_for_field(creature: &crate::core::state::CreatureState) -> bool {
    (creature.statuses.iter().any(|s| s.id == "roosted")
        || !effective_types_for_field(creature)
            .iter()
            .any(|t| t == "flying"))
        && creature.ability.as_deref() != Some("levitate")
        && !creature.statuses.iter().any(|s| s.id == "magnet_rise")
}

fn effective_types_for_field(creature: &crate::core::state::CreatureState) -> Vec<String> {
    creature
        .types
        .iter()
        .filter(|type_id| {
            let removed_status = format!("type_removed_{}", type_id);
            !creature
                .statuses
                .iter()
                .any(|status| status.id == removed_status)
                && !(type_id.as_str() == "flying"
                    && creature
                        .statuses
                        .iter()
                        .any(|status| status.id == "roosted"))
        })
        .cloned()
        .collect()
}

fn run_all_ability(
    state: BattleState,
    hook: &str,
    rng: &mut dyn FnMut() -> f64,
    action: Option<&Action>,
    move_data: Option<&MoveData>,
) -> crate::core::abilities::AbilityHookResult {
    crate::core::abilities::run_all_ability_hooks(
        &state,
        hook,
        AbilityHookContext {
            rng,
            action,
            move_data,
        },
    )
}

fn collect_event_transforms(
    state: &BattleState,
    rng: &mut dyn FnMut() -> f64,
    type_chart: &TypeChart,
) -> Vec<EventTransform> {
    let mut transforms = Vec::new();
    for player in state.players.clone() {
        let result = run_status_hooks(
            state,
            &player.id,
            "onEventTransform",
            StatusHookContext {
                rng,
                action: None,
                move_data: None,
                type_chart,
            },
        );
        transforms.extend(result.event_transforms);
    }
    let field_result = run_field_hooks(
        state,
        "onEventTransform",
        StatusHookContext {
            rng,
            action: None,
            move_data: None,
            type_chart,
        },
    );
    transforms.extend(field_result.event_transforms);
    transforms.sort_by(|a, b| b.priority.cmp(&a.priority));
    transforms
}

fn apply_event_transforms(
    events: &[BattleEvent],
    transforms: &[EventTransform],
) -> Vec<BattleEvent> {
    if transforms.is_empty() {
        return events.to_vec();
    }
    let mut result = Vec::new();
    for event in events {
        let mut cancelled = false;
        for transform in transforms {
            if transform.transform_type == "cancel_event" {
                if matches_transform(event, transform) {
                    cancelled = true;
                    break;
                }
            }
        }
        if cancelled {
            continue;
        }
        let mut replaced = false;
        for transform in transforms {
            if transform.transform_type == "replace_event" && matches_transform(event, transform) {
                result.extend(transform.to.clone());
                replaced = true;
                break;
            }
        }
        if !replaced {
            result.push(event.clone());
        }
    }
    result
}

fn matches_transform(event: &BattleEvent, transform: &EventTransform) -> bool {
    let ev_type = event_type(event);
    if let Some(target_type) = &transform.target_type {
        if target_type != ev_type {
            return false;
        }
    }
    if let Some(from) = &transform.from {
        if from != ev_type {
            return false;
        }
    }
    if let Some(target_id) = &transform.target_id {
        if let Some(event_target) = event_target_id(event) {
            if &event_target != target_id {
                return false;
            }
        } else {
            return false;
        }
    }
    if let Some(except_source) = &transform.except_source_id {
        if let Some(source) = event_source_id(event) {
            if &source == except_source {
                return false;
            }
        }
    }
    if let Some(meta_key) = &transform.require_absent_meta {
        if let Some(meta) = event_meta(event) {
            if meta
                .get(meta_key)
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return false;
            }
        }
    }
    if let Some(meta_key) = &transform.require_present_meta {
        let Some(meta) = event_meta(event) else {
            return false;
        };
        if !meta
            .get(meta_key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return false;
        }
    }
    true
}

fn event_target_id(event: &BattleEvent) -> Option<String> {
    match event {
        BattleEvent::Damage { target_id, .. }
        | BattleEvent::ApplyStatus { target_id, .. }
        | BattleEvent::RemoveStatus { target_id, .. }
        | BattleEvent::ReplaceStatus { target_id, .. }
        | BattleEvent::ModifyStage { target_id, .. }
        | BattleEvent::ClearStages { target_id, .. }
        | BattleEvent::ResetStages { target_id, .. }
        | BattleEvent::CureAllStatus { target_id, .. }
        | BattleEvent::SetAbility { target_id, .. }
        | BattleEvent::SetItem { target_id, .. }
        | BattleEvent::SetStages { target_id, .. }
        | BattleEvent::SwapAttackDefense { target_id, .. } => Some(target_id.clone()),
        _ => None,
    }
}

fn event_source_id(event: &BattleEvent) -> Option<String> {
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
        | BattleEvent::SwapAttackDefense { meta, .. } => {
            crate::core::events::meta_get_string(meta, "source")
        }
        _ => None,
    }
}

fn event_meta(event: &BattleEvent) -> Option<&Map<String, Value>> {
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

fn ensure_move_pp(
    creature: &mut crate::core::state::CreatureState,
    move_id: &str,
    move_data: &MoveData,
) -> Option<i32> {
    let Some(pp) = move_data.pp else {
        return None;
    };
    let entry = creature.move_pp.entry(move_id.to_string()).or_insert(pp);
    Some(*entry)
}

fn has_move_pp(
    creature: &mut crate::core::state::CreatureState,
    move_id: &str,
    move_data: &MoveData,
) -> bool {
    ensure_move_pp(creature, move_id, move_data).map_or(true, |pp| pp > 0)
}

fn consume_move_pp(
    creature: &mut crate::core::state::CreatureState,
    move_id: &str,
    move_data: &MoveData,
) -> bool {
    match ensure_move_pp(creature, move_id, move_data) {
        None => true,
        Some(pp) if pp > 0 => {
            creature.move_pp.insert(move_id.to_string(), pp - 1);
            true
        }
        _ => false,
    }
}

fn choose_random_move(
    state: &mut BattleState,
    move_db: &MoveDatabase,
    pool: &str,
    rng: &mut dyn FnMut() -> f64,
    attacker_id: Option<&str>,
) -> Option<String> {
    let mut candidates: Vec<String> = match pool {
        "self_moves" => {
            if let Some(id) = attacker_id {
                if let Some(active) = get_active_creature(state, id) {
                    if !active.moves.is_empty() {
                        active.moves.clone()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        }
        "recent_moves" => state
            .history
            .as_ref()
            .and_then(|history| {
                history
                    .turns
                    .iter()
                    .rev()
                    .flat_map(|turn| turn.actions.iter().rev())
                    .filter_map(|action| action.move_id.clone())
                    .find(|move_id| move_id != "copycat" && move_id != "metronome")
            })
            .into_iter()
            .collect(),
        "physical" => move_db
            .as_map()
            .iter()
            .filter(|(_, m)| m.category.as_deref() == Some("physical"))
            .map(|(id, _)| id.clone())
            .collect(),
        "special" => move_db
            .as_map()
            .iter()
            .filter(|(_, m)| m.category.as_deref() == Some("special"))
            .map(|(id, _)| id.clone())
            .collect(),
        "status" => move_db
            .as_map()
            .iter()
            .filter(|(_, m)| m.category.as_deref() == Some("status"))
            .map(|(id, _)| id.clone())
            .collect(),
        _ => move_db.as_map().keys().cloned().collect(),
    };

    if candidates.is_empty() {
        if pool == "recent_moves" {
            return None;
        }
        candidates = move_db.as_map().keys().cloned().collect();
    }

    let filtered: Vec<String> = if let Some(id) = attacker_id {
        if let Some(active) = get_active_creature_mut(state, id) {
            candidates
                .into_iter()
                .filter(|move_id| {
                    let Some(move_data) = move_db.get(move_id) else {
                        return false;
                    };
                    has_move_pp(active, move_id, move_data)
                })
                .collect()
        } else {
            candidates
        }
    } else {
        candidates
    };

    if filtered.is_empty() {
        return None;
    }
    let idx = ((rng)() * filtered.len() as f64).floor() as usize;
    filtered.get(idx).cloned()
}

fn expand_random_moves(
    state: &mut BattleState,
    events: &[BattleEvent],
    move_db: &MoveDatabase,
    rng: &mut dyn FnMut() -> f64,
    attacker_id: &str,
    target_id: &str,
    turn: u32,
    type_chart: &TypeChart,
) -> Vec<BattleEvent> {
    let mut expanded = Vec::new();
    let attacker_name = get_active_creature(state, attacker_id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| attacker_id.to_string());

    for event in events {
        match event {
            BattleEvent::RandomMove { pool, .. } => {
                let chosen_move_id =
                    choose_random_move(state, move_db, pool, rng, Some(attacker_id));
                let Some(chosen_move_id) = chosen_move_id else {
                    expanded.push(BattleEvent::Log {
                        message: format!(
                            "{}は ランダムに 技を出そうとしたが 失敗した！",
                            attacker_name
                        ),
                        meta: Map::new(),
                    });
                    continue;
                };
                let Some(chosen_move) = move_db.get(&chosen_move_id) else {
                    continue;
                };
                if let Some(active) = get_active_creature_mut(state, attacker_id) {
                    if !consume_move_pp(active, &chosen_move_id, chosen_move) {
                        let move_name = chosen_move
                            .name
                            .clone()
                            .unwrap_or_else(|| chosen_move_id.clone());
                        expanded.push(BattleEvent::Log {
                            message: format!("{}の {}は PPが 足りない！", attacker_name, move_name),
                            meta: Map::new(),
                        });
                        continue;
                    }
                }
                let move_name = chosen_move
                    .name
                    .clone()
                    .unwrap_or_else(|| chosen_move_id.clone());
                expanded.push(BattleEvent::Log {
                    message: format!("{} used {}! (random)", attacker_name, move_name),
                    meta: Map::new(),
                });

                let mut effect_ctx = EffectContext {
                    attacker_player_id: attacker_id.to_string(),
                    target_player_id: target_id.to_string(),
                    move_data: Some(chosen_move),
                    rng,
                    turn,
                    type_chart,
                    bypass_protect: false,
                    ignore_immunity: false,
                    bypass_substitute: false,
                    ignore_substitute: false,
                    ignore_ability: false,
                    is_sound: false,
                    last_damage: None,
                    switch_slot: None,
                };
                let mut sub_events = apply_effects(state, &chosen_move.steps, &mut effect_ctx);
                sub_events = apply_ability_event_modifiers(state, &sub_events, move_db.as_map());
                let transforms = collect_event_transforms(state, rng, type_chart);
                sub_events = apply_event_transforms(&sub_events, &transforms);
                expanded.extend(sub_events);
            }
            _ => expanded.push(event.clone()),
        }
    }
    expanded
}
