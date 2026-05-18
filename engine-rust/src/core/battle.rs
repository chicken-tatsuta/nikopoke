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
use crate::core::utils::{get_active_creature, get_active_creature_mut, stage_multiplier};
use crate::data::moves::{MoveData, MoveDatabase};
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

    pub fn step_battle(
        &self,
        state: &BattleState,
        actions: &[Action],
        rng: &mut dyn FnMut() -> f64,
        options: BattleOptions,
    ) -> BattleState {
        let mut next = state.clone();
        next.turn += 1;
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
                let priority = run_ability_value_hook(
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
                )
                .round() as i32;
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

            if !move_data.steps.iter().any(|e| e.effect_type == "protect") {
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

            let destiny_bond_failed = prepare_destiny_bond_action(&mut next, &player_id, &move_id);

            if let Some(active) = get_active_creature_mut(&mut next, &player_id) {
                if !consume_move_pp(active, &move_id, move_data) {
                    let move_name = move_data.name.clone().unwrap_or_else(|| move_id.clone());
                    next.log.push(format!(
                        "{}の {}は PPが 足りない！",
                        attacker_name, move_name
                    ));
                    continue;
                }
                active
                    .volatile_data
                    .insert("lastMove".to_string(), Value::String(move_id.clone()));
            }

            let move_name = move_data.name.as_deref().unwrap_or(&move_id);
            next.log
                .push(format!("{}の {}！", attacker_name, move_name));
            if destiny_bond_failed {
                next.log.push("しかし うまくきまらなかった！".to_string());
                continue;
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
                is_sound: false,
                last_damage: None,
            };

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

            if move_id == "destiny_bond" {
                mark_destiny_bond_success(&mut next, &player_id);
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
            }),
            turn: state.turn,
            stages: None,
        },
    );
    speed.round() as i32
}

fn prepare_destiny_bond_action(state: &mut BattleState, player_id: &str, move_id: &str) -> bool {
    let Some(active) = get_active_creature_mut(state, player_id) else {
        return false;
    };
    let previous_destiny_bond_succeeded = active
        .volatile_data
        .get("destinyBondLastSucceeded")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    active.statuses.retain(|status| status.id != "destiny_bond");

    if move_id == "destiny_bond" {
        if previous_destiny_bond_succeeded {
            active
                .volatile_data
                .insert("destinyBondLastSucceeded".to_string(), Value::Bool(false));
            return true;
        }
    } else {
        active
            .volatile_data
            .insert("destinyBondLastSucceeded".to_string(), Value::Bool(false));
    }

    false
}

fn mark_destiny_bond_success(state: &mut BattleState, player_id: &str) {
    if let Some(active) = get_active_creature_mut(state, player_id) {
        active
            .volatile_data
            .insert("destinyBondLastSucceeded".to_string(), Value::Bool(true));
    }
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
        let mut transformed_event = event.clone();
        let mut stage_drop_logs = Vec::new();
        for transform in transforms {
            if transform.transform_type != "filter_stage_drops"
                || !matches_transform(&transformed_event, transform)
            {
                continue;
            }
            let BattleEvent::ModifyStage { stages, .. } = &mut transformed_event else {
                continue;
            };
            let original_len = stages.len();
            stages.retain(|_, value| *value >= 0);
            if stages.len() != original_len {
                stage_drop_logs.extend(transform.to.clone());
            }
        }
        result.extend(stage_drop_logs);
        if let BattleEvent::ModifyStage { stages, .. } = &transformed_event {
            if stages.is_empty() {
                continue;
            }
        }
        let mut replaced = false;
        for transform in transforms {
            if transform.transform_type == "replace_event"
                && matches_transform(&transformed_event, transform)
            {
                result.extend(transform.to.clone());
                replaced = true;
                break;
            }
        }
        if !replaced {
            result.push(transformed_event);
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
            BattleEvent::RandomMove { pool, meta } => {
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
                let source_move_id = meta.get("moveId").and_then(|value| value.as_str());
                let message = if source_move_id == Some("metronome") {
                    format!("ゆびをふったら {}が でた！", move_name)
                } else {
                    format!("{}の {}！", attacker_name, move_name)
                };
                expanded.push(BattleEvent::Log {
                    message,
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
                    is_sound: false,
                    last_damage: None,
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
