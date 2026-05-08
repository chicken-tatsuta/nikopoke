use crate::core::effects::{apply_effects, apply_events};
use crate::core::events::{BattleEvent, EventTransform};
use crate::core::state::{Action, BattleState, Status};
use crate::core::utils::get_active_creature;
use crate::data::moves::{Effect, MoveData};
use crate::data::type_chart::TypeChart;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Default)]
pub struct StatusHookResult {
    pub state: Option<BattleState>,
    pub events: Vec<BattleEvent>,
    pub prevent_action: bool,
    pub override_action: Option<Action>,
    pub event_transforms: Vec<EventTransform>,
}

pub struct StatusHookContext<'a> {
    pub rng: &'a mut dyn FnMut() -> f64,
    pub action: Option<&'a Action>,
    pub move_data: Option<&'a MoveData>,
    pub type_chart: &'a TypeChart,
}

pub fn run_status_hooks(
    state: &BattleState,
    player_id: &str,
    hook: &str,
    ctx: StatusHookContext<'_>,
) -> StatusHookResult {
    let Some(active) = get_active_creature(state, player_id) else {
        return StatusHookResult::default();
    };

    let mut working_state = state.clone();
    let mut events = Vec::new();
    let mut prevent_action = false;
    let mut override_action = None;
    let mut event_transforms = Vec::new();

    let statuses = active.statuses.clone();
    for status in statuses {
        let result = match_status(&working_state, player_id, hook, &status, &mut StatusHookContext {
            rng: ctx.rng,
            action: ctx.action,
            move_data: ctx.move_data,
            type_chart: ctx.type_chart,
        });
        if let Some(next) = result.state {
            working_state = next;
        }
        events.extend(result.events);
        if result.prevent_action {
            prevent_action = true;
        }
        if result.override_action.is_some() {
            override_action = result.override_action;
        }
        event_transforms.extend(result.event_transforms);
    }

    StatusHookResult {
        state: Some(working_state),
        events,
        prevent_action,
        override_action,
        event_transforms,
    }
}

pub fn run_field_hooks(
    state: &BattleState,
    hook: &str,
    ctx: StatusHookContext<'_>,
) -> StatusHookResult {
    let mut working_state = state.clone();
    let mut events = Vec::new();
    let mut event_transforms = Vec::new();

    for effect in &state.field.global {
        let result = match_field_effect(&working_state, hook, effect.id.as_str(), effect, None, &mut StatusHookContext {
            rng: ctx.rng,
            action: ctx.action,
            move_data: ctx.move_data,
            type_chart: ctx.type_chart,
        });
        if let Some(next) = result.state {
            working_state = next;
        }
        events.extend(result.events);
        event_transforms.extend(result.event_transforms);
    }

    for (side_id, effects) in &state.field.sides {
        for effect in effects {
            let result = match_field_effect(&working_state, hook, effect.id.as_str(), effect, Some(side_id.as_str()), &mut StatusHookContext {
                rng: ctx.rng,
                action: ctx.action,
                move_data: ctx.move_data,
                type_chart: ctx.type_chart,
            });
            if let Some(next) = result.state {
                working_state = next;
            }
            events.extend(result.events);
            event_transforms.extend(result.event_transforms);
        }
    }

    StatusHookResult {
        state: Some(working_state),
        events,
        prevent_action: false,
        override_action: None,
        event_transforms,
    }
}

fn match_field_effect(
    state: &BattleState,
    hook: &str,
    status_id: &str,
    status: &crate::core::state::FieldEffect,
    owner_side: Option<&str>,
    ctx: &mut StatusHookContext<'_>,
) -> StatusHookResult {
    if status_id == "sandstorm" && hook == "onWeatherEnd" {
        let mut events = Vec::new();
        for player in &state.players {
            let Some(active) = get_active_creature(state, &player.id) else {
                continue;
            };
            if active.hp <= 0 || active.types.iter().any(|t| matches!(t.as_str(), "rock" | "steel" | "ground")) {
                continue;
            }
            let damage = (active.max_hp / 16).max(1);
            events.push(BattleEvent::Damage {
                target_id: player.id.clone(),
                amount: damage,
                meta: Map::new(),
            });
            events.push(BattleEvent::Log {
                message: format!("{}は すなあらしの ダメージを 受けている！", active.name),
                meta: Map::new(),
            });
        }
        return StatusHookResult {
            events,
            ..Default::default()
        };
    }

    // グラスフィールド回復は特別処理
    if status_id == "grassy_terrain" && hook == "onGrassyTerrainHeal" {
        let mut events = Vec::new();
        for player in &state.players {
            let active = get_active_creature(state, &player.id);
            if let Some(active) = active {
                if active.hp > 0 && active.hp < active.max_hp {
                    // 地面にいるポケモンのみ回復（ひこう・ふゆう除外は簡略化）
                    let is_flying = active.types.iter().any(|t| t == "flying");
                    let has_levitate = active.ability.as_deref() == Some("levitate");
                    if !is_flying && !has_levitate {
                        let heal = (active.max_hp / 16).max(1);
                        events.push(BattleEvent::Log {
                            message: format!("{}は グラスフィールドの 恩恵を 受けている！", active.name),
                            meta: Map::new(),
                        });
                        events.push(BattleEvent::Damage {
                            target_id: player.id.clone(),
                            amount: -heal,
                            meta: Map::new(),
                        });
                    }
                }
            }
        }
        return StatusHookResult {
            events,
            ..Default::default()
        };
    }

    let pseudo_status = Status {
        id: status_id.to_string(),
        remaining_turns: status.remaining_turns,
        data: {
            let mut data = status.data.clone();
            if let Some(owner_side) = owner_side {
                data.insert("ownerSideId".to_string(), Value::String(owner_side.to_string()));
            }
            data
        },
    };
    match_status(state, "", hook, &pseudo_status, ctx)
}

fn match_status(
    state: &BattleState,
    player_id: &str,
    hook: &str,
    status: &Status,
    ctx: &mut StatusHookContext<'_>,
) -> StatusHookResult {
    if status.id.starts_with("charging_") && hook == "onBeforeAction" {
        if let Some(move_id) = status.data.get("moveId").and_then(|v| v.as_str()) {
            if let Some(action) = ctx.action {
                let mut new_action = action.clone();
                new_action.move_id = Some(move_id.to_string());
                return StatusHookResult {
                    override_action: Some(new_action),
                    events: vec![BattleEvent::Log {
                        message: format!("{}は 力を 解き放つ！", get_active_creature(state, player_id).unwrap().name),
                        meta: Map::new(),
                    }],
                    ..Default::default()
                };
            }
        }
    }
    match status.id.as_str() {
        "burn" => match hook {
            "onStatusDamage" => {
                let active = get_active_creature(state, player_id).unwrap();
                let damage = (active.max_hp / 16).max(1);
                StatusHookResult {
                    events: vec![
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: damage,
                            meta: Map::new(),
                        },
                        BattleEvent::Log {
                            message: format!("{}は やけどのダメージを 受けている！", active.name),
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "poison" => match hook {
            "onStatusDamage" => {
                let active = get_active_creature(state, player_id).unwrap();
                let damage = (active.max_hp / 8).max(1);
                StatusHookResult {
                    events: vec![
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: damage,
                            meta: Map::new(),
                        },
                        BattleEvent::Log {
                            message: format!("{}は どくの ダメージを 受けている！", active.name),
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "toxic" => match hook {
            "onStatusDamage" => {
                let active = get_active_creature(state, player_id).unwrap();
                let counter = active
                    .statuses
                    .iter()
                    .find(|s| s.id == "toxic")
                    .and_then(|s| s.data.get("counter"))
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32)
                    .unwrap_or(1)
                    .max(1);
                let damage = ((active.max_hp * counter) / 16).max(1);

                let mut new_state = state.clone();
                if let Some(player) = new_state.players.iter_mut().find(|p| p.id == player_id) {
                    if let Some(active_mut) = player.team.get_mut(player.active_slot) {
                        if let Some(toxic) = active_mut.statuses.iter_mut().find(|s| s.id == "toxic") {
                            toxic
                                .data
                                .insert("counter".to_string(), Value::Number((counter + 1).into()));
                        }
                    }
                }

                StatusHookResult {
                    state: Some(new_state),
                    events: vec![
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: damage,
                            meta: Map::new(),
                        },
                        BattleEvent::Log {
                            message: format!("{}は もうどくの ダメージを 受けている！", active.name),
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "paralysis" => match hook {
            "onBeforeAction" => {
                if (ctx.rng)() < 0.25 {
                    StatusHookResult {
                        prevent_action: true,
                        events: vec![BattleEvent::Log {
                            message: format!("からだが しびれて 動けない！"),
                            meta: Map::new(),
                        }],
                        ..Default::default()
                    }
                } else {
                    StatusHookResult::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "sleep" => match hook {
            "onBeforeAction" => {
                let active = get_active_creature(state, player_id).unwrap();
                let mut status_idx = None;
                for (i, s) in active.statuses.iter().enumerate() {
                    if s.id == "sleep" {
                        status_idx = Some(i);
                        break;
                    }
                }

                if let Some(idx) = status_idx {
                    let can_act_while_asleep = ctx
                        .move_data
                        .is_some_and(|move_data| is_usable_while_asleep(&move_data.id));
                    let mut new_state = state.clone();
                    let player = new_state.players.iter_mut().find(|p| p.id == player_id).unwrap();
                    let active = player.team.get_mut(player.active_slot).unwrap();
                    let status = &mut active.statuses[idx];

                    let elapsed = status
                        .data
                        .get("elapsed")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
                        + 1;

                    if elapsed >= 3 || (elapsed >= 2 && (ctx.rng)() < (1.0 / 3.0)) {
                        return StatusHookResult {
                            events: vec![
                                BattleEvent::RemoveStatus {
                                    target_id: player_id.to_string(),
                                    status_id: "sleep".to_string(),
                                    meta: Map::new(),
                                },
                                BattleEvent::Log {
                                    message: format!("{}は 目を 覚ました！", active.name),
                                    meta: Map::new(),
                                },
                            ],
                            ..Default::default()
                        };
                    }

                    status.data.insert("elapsed".to_string(), Value::Number(elapsed.into()));
                    let name = active.name.clone();
                    if can_act_while_asleep {
                        return StatusHookResult {
                            state: Some(new_state),
                            events: vec![BattleEvent::Log {
                                message: format!("{}は 眠りながら 技を 出した！", name),
                                meta: Map::new(),
                            }],
                            ..Default::default()
                        };
                    }
                    return StatusHookResult {
                        state: Some(new_state),
                        prevent_action: true,
                        events: vec![BattleEvent::Log {
                            message: format!("{}は ぐうぐう 眠り続けている。", name),
                            meta: Map::new(),
                        }],
                        ..Default::default()
                    };
                }
                StatusHookResult::default()
            }
            _ => StatusHookResult::default(),
        },
        "freeze" => match hook {
            "onBeforeAction" => {
                let active = get_active_creature(state, player_id).unwrap();
                if ctx
                    .move_data
                    .is_some_and(|move_data| move_data.tags.iter().any(|tag| tag == "thaws_user"))
                {
                    return StatusHookResult {
                        events: vec![
                            BattleEvent::RemoveStatus {
                                target_id: player_id.to_string(),
                                status_id: "freeze".to_string(),
                                meta: Map::new(),
                            },
                            BattleEvent::Log {
                                message: format!("{}の こおりが とけた！", active.name),
                                meta: Map::new(),
                            },
                        ],
                        ..Default::default()
                    };
                }
                if (ctx.rng)() < 0.2 {
                    StatusHookResult {
                        events: vec![
                            BattleEvent::RemoveStatus {
                                target_id: player_id.to_string(),
                                status_id: "freeze".to_string(),
                                meta: Map::new(),
                            },
                            BattleEvent::Log {
                                message: format!("{}の こおりが とけた！", active.name),
                                meta: Map::new(),
                            },
                        ],
                        ..Default::default()
                    }
                } else {
                    StatusHookResult {
                        prevent_action: true,
                        events: vec![BattleEvent::Log {
                            message: format!("{}は 凍りついて 動けない！", active.name),
                            meta: Map::new(),
                        }],
                        ..Default::default()
                    }
                }
            }
            _ => StatusHookResult::default(),
        },
        "confusion" => match hook {
            "onBeforeAction" => {
                let active = get_active_creature(state, player_id).unwrap();
                if (ctx.rng)() < 0.33 {
                    let damage = ((active.max_hp as f32) * 0.1).floor() as i32;
                    StatusHookResult {
                        prevent_action: true,
                        events: vec![
                            BattleEvent::Log {
                                message: format!("わけもわからず 自分を 攻撃した！"),
                                meta: Map::new(),
                            },
                            BattleEvent::Damage {
                                target_id: player_id.to_string(),
                                amount: damage.max(1),
                                meta: Map::new(),
                            },
                        ],
                        ..Default::default()
                    }
                } else {
                    StatusHookResult::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "flinch" => match hook {
            "onBeforeAction" => {
                let active = get_active_creature(state, player_id);
                let name = active.map(|c| c.name.clone()).unwrap_or_else(|| "誰か".to_string());
                StatusHookResult {
                    prevent_action: true,
                    events: vec![BattleEvent::Log {
                        message: format!("{}は ひるんで 動けない！", name),
                        meta: Map::new(),
                    }],
                    ..Default::default()
                }
            }
            "onTurnEnd" => StatusHookResult {
                events: vec![BattleEvent::RemoveStatus {
                    target_id: player_id.to_string(),
                    status_id: "flinch".to_string(),
                    meta: Map::new(),
                }],
                ..Default::default()
            },
            _ => StatusHookResult::default(),
        },
        "protect" => match hook {
            "onEventTransform" => {
                let active = get_active_creature(state, player_id).unwrap();
                let mut transforms = Vec::new();
                let types = ["damage", "apply_status", "modify_stage", "set_ability"];
                for t in types {
                    transforms.push(EventTransform {
                        transform_type: "replace_event".to_string(),
                        from: Some(t.to_string()),
                        target_type: None,
                        target_id: Some(player_id.to_string()),
                        except_source_id: Some(player_id.to_string()),
                        require_absent_meta: Some("bypassProtect".to_string()),
                        require_present_meta: None,
                        to: vec![BattleEvent::Log {
                            message: format!("{}は 攻撃から 身を 守った！", active.name),
                            meta: Map::new(),
                        }],
                        priority: 0,
                    });
                }
                StatusHookResult {
                    event_transforms: transforms,
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "baneful_bunker" => match hook {
            "onEventTransform" => {
                let active = get_active_creature(state, player_id).unwrap();
                let source_id = state
                    .players
                    .iter()
                    .find(|player| player.id != player_id)
                    .map(|player| player.id.clone())
                    .unwrap_or_default();
                let mut transforms = Vec::new();
                let types = ["damage", "apply_status", "modify_stage", "set_ability"];
                for t in types {
                    let mut replacement = vec![BattleEvent::Log {
                        message: format!("{}は 攻撃から 身を 守った！", active.name),
                        meta: Map::new(),
                    }];
                    if t == "damage" && !source_id.is_empty() {
                        replacement.push(BattleEvent::ApplyStatus {
                            target_id: source_id.clone(),
                            status_id: "poison".to_string(),
                            duration: None,
                            stack: false,
                            data: HashMap::new(),
                            meta: Map::new(),
                        });
                        transforms.push(EventTransform {
                            transform_type: "replace_event".to_string(),
                            from: Some(t.to_string()),
                            target_type: None,
                            target_id: Some(player_id.to_string()),
                            except_source_id: Some(player_id.to_string()),
                            require_absent_meta: Some("bypassProtect".to_string()),
                            require_present_meta: Some("contact".to_string()),
                            to: replacement.clone(),
                            priority: 1,
                        });
                        replacement.truncate(1);
                    }
                    transforms.push(EventTransform {
                        transform_type: "replace_event".to_string(),
                        from: Some(t.to_string()),
                        target_type: None,
                        target_id: Some(player_id.to_string()),
                        except_source_id: Some(player_id.to_string()),
                        require_absent_meta: Some("bypassProtect".to_string()),
                        require_present_meta: None,
                        to: replacement,
                        priority: 0,
                    });
                }
                StatusHookResult {
                    event_transforms: transforms,
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "substitute" => match hook {
            "onEventTransform" => {
                let active = get_active_creature(state, player_id).unwrap();
                let mut transforms = Vec::new();
                let types = ["apply_status", "modify_stage"];
                for t in types {
                    transforms.push(EventTransform {
                        transform_type: "replace_event".to_string(),
                        from: Some(t.to_string()),
                        target_type: None,
                        target_id: Some(player_id.to_string()),
                        except_source_id: Some(player_id.to_string()),
                        require_absent_meta: Some("bypassSubstitute".to_string()),
                        require_present_meta: None,
                        to: vec![BattleEvent::Log {
                            message: format!("{}の みがわりが 攻撃を 受けた！", active.name),
                            meta: Map::new(),
                        }],
                        priority: 0,
                    });
                }
                StatusHookResult {
                    event_transforms: transforms,
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "lock_move" => match hook {
            "onBeforeAction" => {
                let data_mode = status.data.get("mode").and_then(|v| v.as_str());
                let mut target_move = status
                    .data
                    .get("moveId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                if data_mode == Some("force_last_move") && target_move.is_none() {
                    let active = get_active_creature(state, player_id).unwrap();
                    if let Some(Value::String(m)) = active.volatile_data.get("lastMove") {
                        target_move = Some(m.clone());
                    } else {
                        target_move = find_last_move_from_history(state, player_id);
                    }
                }

                if let Some(move_id) = target_move {
                    if data_mode == Some("force_specific") || data_mode == Some("force_last_move") {
                        if let Some(action) = ctx.action {
                            let mut new_action = action.clone();
                            new_action.move_id = Some(move_id.clone());
                            let active = get_active_creature(state, player_id).unwrap();
                            let message = if data_mode == Some("force_last_move") {
                                format!("{}は {}しか 出せなくなっている！", active.name, move_id)
                            } else {
                                format!("{}は {}を 出さざるをえない！", active.name, move_id)
                            };
                            return StatusHookResult {
                                override_action: Some(new_action),
                                events: vec![BattleEvent::Log {
                                    message,
                                    meta: Map::new(),
                                }],
                                ..Default::default()
                            };
                        }
                    }
                }
                StatusHookResult::default()
            }
            _ => StatusHookResult::default(),
        },
        "disable_move" => match hook {
            "onBeforeAction" => {
                let move_id = status.data.get("moveId").and_then(|v| v.as_str());
                if let (Some(move_id), Some(action)) = (move_id, ctx.action) {
                    if action.move_id.as_deref() == Some(move_id) {
                        return StatusHookResult {
                            prevent_action: true,
                            events: vec![BattleEvent::Log {
                                message: format!("{}は {}を 出すことができない！", get_active_creature(state, player_id).unwrap().name, move_id),
                                meta: Map::new(),
                            }],
                            ..Default::default()
                        };
                    }
                }
                StatusHookResult::default()
            }
            _ => StatusHookResult::default(),
        },
        "encore" => match hook {
            "onBeforeAction" => {
                let move_id = status.data.get("moveId").and_then(|v| v.as_str());
                if let (Some(move_id), Some(action)) = (move_id, ctx.action) {
                    if action.move_id.as_deref() != Some(move_id) {
                        let mut new_action = action.clone();
                        new_action.move_id = Some(move_id.to_string());
                        return StatusHookResult {
                            override_action: Some(new_action),
                            events: vec![BattleEvent::Log {
                                message: format!("{}は アンコールを 受けた！", get_active_creature(state, player_id).unwrap().name),
                                meta: Map::new(),
                            }],
                            ..Default::default()
                        };
                    }
                }
                StatusHookResult::default()
            }
            _ => StatusHookResult::default(),
        },
        "taunt" => match hook {
            "onBeforeAction" => {
                if let Some(move_data) = ctx.move_data {
                    if move_data.category.as_deref() == Some("status") {
                        return StatusHookResult {
                            prevent_action: true,
                            events: vec![BattleEvent::Log {
                                message: format!("ちょうはつされて {}を 出すことができない！", move_data.name.clone().unwrap_or_else(|| move_data.id.clone())),
                                meta: Map::new(),
                            }],
                            ..Default::default()
                        };
                    }
                }
                StatusHookResult::default()
            }
            _ => StatusHookResult::default(),
        },
        "torment" => match hook {
            "onBeforeAction" => {
                let Some(action) = ctx.action else {
                    return StatusHookResult::default();
                };
                let Some(active) = get_active_creature(state, player_id) else {
                    return StatusHookResult::default();
                };
                let last_move = active.volatile_data.get("lastMove").and_then(|v| v.as_str());
                if let (Some(last_move), Some(current_move)) = (last_move, action.move_id.as_deref()) {
                    if last_move == current_move {
                        return StatusHookResult {
                            prevent_action: true,
                            events: vec![BattleEvent::Log {
                                message: format!("{}は いちゃもんで 同じ技を 連続で 出せない！", active.name),
                                meta: Map::new(),
                            }],
                            ..Default::default()
                        };
                    }
                }
                StatusHookResult::default()
            }
            _ => StatusHookResult::default(),
        },
        "throat_chop" => match hook {
            "onBeforeAction" => {
                let Some(move_data) = ctx.move_data else {
                    return StatusHookResult::default();
                };
                let is_sound_move = move_data.tags.iter().any(|tag| tag == "sound");
                if !is_sound_move {
                    return StatusHookResult::default();
                }
                let active = get_active_creature(state, player_id).unwrap();
                let move_name = move_data.name.clone().unwrap_or_else(|| move_data.id.clone());
                StatusHookResult {
                    prevent_action: true,
                    events: vec![BattleEvent::Log {
                        message: format!("{}は じごくづきで {}を 出せない！", active.name, move_name),
                        meta: Map::new(),
                    }],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "trapped" => match hook {
            "onTrap" => StatusHookResult {
                prevent_action: true,
                ..Default::default()
            },
            _ => StatusHookResult::default(),
        },
        "mist" => match hook {
            "onEventTransform" => {
                let owner_side = status
                    .data
                    .get("ownerSideId")
                    .and_then(|v| v.as_str())
                    .unwrap_or(player_id)
                    .to_string();
                StatusHookResult {
                    event_transforms: vec![EventTransform {
                        transform_type: "replace_event".to_string(),
                        from: Some("modify_stage".to_string()),
                        target_type: None,
                        target_id: Some(owner_side),
                        except_source_id: Some(player_id.to_string()),
                        require_absent_meta: None,
                        require_present_meta: None,
                        to: vec![BattleEvent::Log {
                            message: "しろいきりが 能力変化を 防いだ！".to_string(),
                            meta: Map::new(),
                        }],
                        priority: 1,
                    }],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "leech_seed" => match hook {
            "onLeechSeed" => {
                let source_id = status.data.get("sourceId").and_then(|v| v.as_str());
                let Some(source_id) = source_id else { return StatusHookResult::default(); };
                let source = get_active_creature(state, source_id);
                if source.is_none() || source.unwrap().hp <= 0 {
                    return StatusHookResult::default();
                }
                let active = get_active_creature(state, player_id).unwrap();
                let damage = (active.max_hp / 8).max(1);
                StatusHookResult {
                    events: vec![
                        BattleEvent::Log {
                            message: format!("宿り木の種が {}の 体力を 削る！", active.name),
                            meta: Map::new(),
                        },
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: damage,
                            meta: Map::new(),
                        },
                        BattleEvent::Damage {
                            target_id: source_id.to_string(),
                            amount: -damage,
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "aqua_ring" | "ingrain" => match hook {
            "onTurnEnd" => {
                let active = get_active_creature(state, player_id).unwrap();
                if active.hp <= 0 || active.hp >= active.max_hp {
                    return StatusHookResult::default();
                }
                let heal = (active.max_hp / 16).max(1);
                StatusHookResult {
                    events: vec![
                        BattleEvent::Log {
                            message: format!("{}は 少し 回復した！", active.name),
                            meta: Map::new(),
                        },
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: -heal,
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "curse" => match hook {
            "onTurnEnd" => {
                let active = get_active_creature(state, player_id).unwrap();
                let damage = (active.max_hp / 4).max(1);
                StatusHookResult {
                    events: vec![
                        BattleEvent::Log {
                            message: format!("{}は 呪われている！", active.name),
                            meta: Map::new(),
                        },
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: damage,
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "yawn" => match hook {
            "onTurnEnd" => {
                let turns = status
                    .data
                    .get("turns")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);
                if turns > 0 {
                    let mut new_state = state.clone();
                    if let Some(player) = new_state.players.iter_mut().find(|p| p.id == player_id) {
                        if let Some(active) = player.team.get_mut(player.active_slot) {
                            if let Some(status_mut) = active.statuses.iter_mut().find(|s| s.id == "yawn") {
                                status_mut
                                    .data
                                    .insert("turns".to_string(), Value::Number((turns - 1).into()));
                            }
                        }
                    }
                    return StatusHookResult {
                        state: Some(new_state),
                        events: vec![BattleEvent::Log {
                            message: format!("{}は 眠たそうだ……", get_active_creature(state, player_id).unwrap().name),
                            meta: Map::new(),
                        }],
                        ..Default::default()
                    };
                }
                let min = 2;
                let max = 4;
                let duration = min + (((ctx.rng)() * ((max - min + 1) as f64)).floor() as i32);
                StatusHookResult {
                    events: vec![
                        BattleEvent::RemoveStatus {
                            target_id: player_id.to_string(),
                            status_id: "yawn".to_string(),
                            meta: Map::new(),
                        },
                        BattleEvent::ApplyStatus {
                            target_id: player_id.to_string(),
                            status_id: "sleep".to_string(),
                            duration: Some(duration),
                            stack: false,
                            data: HashMap::new(),
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        "charging_solar_beam" => match hook {
            "onBeforeAction" => {
                let data_mode = status.data.get("mode").and_then(|v| v.as_str());
                let move_id = status.data.get("moveId").and_then(|v| v.as_str());
                if data_mode == Some("force_specific") {
                    if let (Some(move_id), Some(action)) = (move_id, ctx.action) {
                        let mut new_action = action.clone();
                        new_action.move_id = Some(move_id.to_string());
                        return StatusHookResult {
                            override_action: Some(new_action),
                            ..Default::default()
                        };
                    }
                }
                StatusHookResult::default()
            }
            _ => StatusHookResult::default(),
        },
        "delayed_effect" => match hook {
            "onTurnStart" | "onTurnEnd" => handle_delayed(state, player_id, status, hook, ctx),
            _ => StatusHookResult::default(),
        },
        "over_time_effect" => match hook {
            "onTurnEnd" => handle_over_time(state, player_id, status, hook, ctx),
            _ => StatusHookResult::default(),
        },
        // ねがいごと - 次ターン開始時にHP回復
        "wish" => match hook {
            "onWishResolve" => {
                let trigger_turn = status.data.get("triggerTurn").and_then(|v| v.as_i64()).unwrap_or(0);
                if (state.turn as i64) < trigger_turn {
                    return StatusHookResult::default();
                }
                let heal_amount = status.data.get("healAmount").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let active = get_active_creature(state, player_id);
                if active.is_none() || active.unwrap().hp <= 0 {
                    return StatusHookResult::default();
                }
                let active = active.unwrap();
                StatusHookResult {
                    events: vec![
                        BattleEvent::Log {
                            message: format!("{}の ねがいごとが かなった！", active.name),
                            meta: Map::new(),
                        },
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: -heal_amount,
                            meta: Map::new(),
                        },
                        BattleEvent::RemoveStatus {
                            target_id: player_id.to_string(),
                            status_id: "wish".to_string(),
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        // バインド (まきつく、しめつける等) - ターン終了時ダメージ
        "bind" => match hook {
            "onBindDamage" => {
                let active = get_active_creature(state, player_id).unwrap();
                let damage = (active.max_hp / 8).max(1);
                let move_name = status.data.get("moveName").and_then(|v| v.as_str()).unwrap_or("バインド");
                StatusHookResult {
                    events: vec![
                        BattleEvent::Log {
                            message: format!("{}は {}の ダメージを受けている！", active.name, move_name),
                            meta: Map::new(),
                        },
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: damage,
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        // たべのこし - 毎ターンHP回復
        "leftovers" => match hook {
            "onItemEndTurn" => {
                let active = get_active_creature(state, player_id);
                if active.is_none() || active.unwrap().hp <= 0 {
                    return StatusHookResult::default();
                }
                let active = active.unwrap();
                if active.hp >= active.max_hp {
                    return StatusHookResult::default();
                }
                let heal = (active.max_hp / 16).max(1);
                StatusHookResult {
                    events: vec![
                        BattleEvent::Log {
                            message: format!("{}は たべのこしで 少し回復した！", active.name),
                            meta: Map::new(),
                        },
                        BattleEvent::Damage {
                            target_id: player_id.to_string(),
                            amount: -heal,
                            meta: Map::new(),
                        },
                    ],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        // くろいヘドロ - どくタイプは回復、それ以外はダメージ
        "black_sludge" => match hook {
            "onItemEndTurn" => {
                let active = get_active_creature(state, player_id);
                if active.is_none() || active.unwrap().hp <= 0 {
                    return StatusHookResult::default();
                }
                let active = active.unwrap();
                let is_poison = active.types.iter().any(|t| t == "poison");
                if is_poison {
                    if active.hp >= active.max_hp {
                        return StatusHookResult::default();
                    }
                    let heal = (active.max_hp / 16).max(1);
                    StatusHookResult {
                        events: vec![
                            BattleEvent::Log {
                                message: format!("{}は くろいヘドロで 少し回復した！", active.name),
                                meta: Map::new(),
                            },
                            BattleEvent::Damage {
                                target_id: player_id.to_string(),
                                amount: -heal,
                                meta: Map::new(),
                            },
                        ],
                        ..Default::default()
                    }
                } else {
                    let damage = (active.max_hp / 8).max(1);
                    StatusHookResult {
                        events: vec![
                            BattleEvent::Log {
                                message: format!("{}は くろいヘドロで ダメージを受けた！", active.name),
                                meta: Map::new(),
                            },
                            BattleEvent::Damage {
                                target_id: player_id.to_string(),
                                amount: damage,
                                meta: Map::new(),
                            },
                        ],
                        ..Default::default()
                    }
                }
            }
            _ => StatusHookResult::default(),
        },
        "safeguard" => match hook {
            "onEventTransform" => {
                let owner_side = status.data.get("ownerSideId").and_then(|v| v.as_str()).unwrap_or("");
                if owner_side.is_empty() {
                    return StatusHookResult::default();
                }
                StatusHookResult {
                    event_transforms: vec![EventTransform {
                        transform_type: "replace_event".to_string(),
                        from: Some("apply_status".to_string()),
                        target_type: None,
                        target_id: Some(owner_side.to_string()),
                        except_source_id: Some(owner_side.to_string()),
                        require_absent_meta: None,
                        require_present_meta: None,
                        to: vec![BattleEvent::Log {
                            message: "しろいベールが 状態異常を 防いだ！".to_string(),
                            meta: Map::new(),
                        }],
                        priority: 0,
                    }],
                    ..Default::default()
                }
            }
            _ => StatusHookResult::default(),
        },
        _ => StatusHookResult::default(),
    }
}

fn handle_delayed(
    state: &BattleState,
    player_id: &str,
    status: &Status,
    hook: &str,
    ctx: &mut StatusHookContext<'_>,
) -> StatusHookResult {
    let timing = status.data.get("timing").and_then(|v| v.as_str()).unwrap_or("turn_end");
    let trigger_turn = status.data.get("triggerTurn").and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
    if (state.turn as i64) < trigger_turn {
        return StatusHookResult::default();
    }
    if !matches_timing(hook, timing) {
        return StatusHookResult::default();
    }

    let target_id = status.data.get("targetId").and_then(|v| v.as_str()).unwrap_or(player_id);
    let attacker_id = status.data.get("sourceId").and_then(|v| v.as_str()).unwrap_or(player_id);
    if let Some(target) = get_active_creature(state, target_id) {
        if target.hp <= 0 {
            return StatusHookResult::default();
        }
    } else {
        return StatusHookResult::default();
    }

    let effects = effects_from_status(status);
    let mut effect_ctx = crate::core::effects::EffectContext {
        attacker_player_id: attacker_id.to_string(),
        target_player_id: target_id.to_string(),
        move_data: None,
        rng: ctx.rng,
        turn: state.turn,
        type_chart: ctx.type_chart,
        bypass_protect: false,
        ignore_immunity: false,
        bypass_substitute: false,
        ignore_substitute: false,
        ignore_ability: false,
        is_sound: false,
        last_damage: None,
    };
    let events = apply_effects(state, &effects, &mut effect_ctx);
    let new_state = apply_events(state, &events);

    StatusHookResult {
        state: Some(new_state),
        ..Default::default()
    }
}

fn handle_over_time(
    state: &BattleState,
    player_id: &str,
    status: &Status,
    hook: &str,
    ctx: &mut StatusHookContext<'_>,
) -> StatusHookResult {
    let timing = status.data.get("timing").and_then(|v| v.as_str()).unwrap_or("turn_end");
    if !matches_timing(hook, timing) {
        return StatusHookResult::default();
    }

    let target_id = status.data.get("targetId").and_then(|v| v.as_str()).unwrap_or(player_id);
    let attacker_id = status.data.get("sourceId").and_then(|v| v.as_str()).unwrap_or(player_id);
    if let Some(target) = get_active_creature(state, target_id) {
        if target.hp <= 0 {
            return StatusHookResult::default();
        }
    } else {
        return StatusHookResult::default();
    }
    let effects = effects_from_status(status);
    let mut effect_ctx = crate::core::effects::EffectContext {
        attacker_player_id: attacker_id.to_string(),
        target_player_id: target_id.to_string(),
        move_data: None,
        rng: ctx.rng,
        turn: state.turn,
        type_chart: ctx.type_chart,
        bypass_protect: false,
        ignore_immunity: false,
        bypass_substitute: false,
        ignore_substitute: false,
        ignore_ability: false,
        is_sound: false,
        last_damage: None,
    };
    let events = apply_effects(state, &effects, &mut effect_ctx);
    let new_state = apply_events(state, &events);

    StatusHookResult {
        state: Some(new_state),
        ..Default::default()
    }
}

fn matches_timing(hook: &str, timing: &str) -> bool {
    match timing.to_lowercase().as_str() {
        "turn_start" => hook == "onTurnStart",
        "turn_end" => hook == "onTurnEnd",
        _ => true,
    }
}

fn is_usable_while_asleep(move_id: &str) -> bool {
    matches!(move_id, "sleep_talk" | "snore")
}

fn effects_from_status(status: &Status) -> Vec<Effect> {
    match status.data.get("effects") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn tick_statuses(state: &BattleState) -> BattleState {
    let mut next = state.clone();
    for player in &mut next.players {
        if let Some(active) = player.team.get_mut(player.active_slot) {
            // Track statuses that will expire and need special handling
            let mut apply_confusion = false;
            
            for status in &mut active.statuses {
                if let Some(turns) = status.remaining_turns {
                    let new_turns = turns - 1;
                    status.remaining_turns = Some(new_turns);
                    
                    // Check if lock_move with confuseOnEnd is expiring
                    if new_turns <= 0 && status.id == "lock_move" {
                        if let Some(Value::Bool(true)) = status.data.get("confuseOnEnd") {
                            apply_confusion = true;
                        }
                    }
                }
            }
            
            active
                .statuses
                .retain(|s| s.remaining_turns.map(|t| t > 0).unwrap_or(true));
            
            // Apply confusion if needed (from expiring lock_move with confuseOnEnd)
            if apply_confusion && active.hp > 0 {
                // Major status effects are mutually exclusive in Nikimon battle rules.
                if !active.statuses.iter().any(|s| is_exclusive_major_status(&s.id)) {
                    let rng_data = HashMap::new();
                    // Duration 2-4 turns (pseudo-random based on turn number)
                    let duration = 2 + ((state.turn % 3) as i32);
                    active.statuses.push(Status {
                        id: "confusion".to_string(),
                        remaining_turns: Some(duration),
                        data: rng_data,
                    });
                    next.log.push(format!("{}は 混乱してしまった！", active.name));
                } else {
                    next.log.push("しかしうまく決まらなかった！".to_string());
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

pub fn tick_field_effects(state: &BattleState) -> BattleState {
    let mut next = state.clone();
    for effect in &mut next.field.global {
        if let Some(turns) = effect.remaining_turns {
            effect.remaining_turns = Some(turns - 1);
        }
    }
    next.field
        .global
        .retain(|e| e.remaining_turns.map(|t| t > 0).unwrap_or(true));
    for effects in next.field.sides.values_mut() {
        for effect in effects.iter_mut() {
            if let Some(turns) = effect.remaining_turns {
                effect.remaining_turns = Some(turns - 1);
            }
        }
        effects.retain(|e| e.remaining_turns.map(|t| t > 0).unwrap_or(true));
    }
    next
}

fn find_last_move_from_history(state: &BattleState, player_id: &str) -> Option<String> {
            if let Some(history) = &state.history {
                for turn in history.turns.iter().rev() {
                    for action in turn.actions.iter().rev() {
                        if action.player_id == player_id {
                            if let Some(move_id) = &action.move_id {
                                return Some(move_id.clone());
                            }
                        }
                    }
                }
            }
            None
        }
        
        
