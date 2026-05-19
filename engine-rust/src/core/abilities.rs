use crate::core::events::{meta_get_bool, meta_with_move_source, BattleEvent};
use crate::core::state::{Action, BattleState, CreatureState};
use crate::core::utils::{get_active_creature, is_status_move};
use crate::data::moves::MoveData;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum WeatherKind {
    Sun,
    Rain,
    Sandstorm,
    Snow,
}

pub struct AbilityValueContext<'a> {
    pub move_data: Option<&'a MoveData>,
    pub category: Option<&'a str>,
    pub target: Option<&'a CreatureState>,
    pub weather: Option<&'a str>,
    pub turn: u32,
    pub stages: Option<i32>,
}

pub struct AbilityCheckContext<'a> {
    pub status_id: Option<&'a str>,
    pub r#type: Option<&'a str>,
    pub target_id: Option<&'a str>,
    pub action: Option<&'a Action>,
}

#[derive(Default)]
pub struct AbilityHookResult {
    pub state: Option<BattleState>,
    pub events: Vec<BattleEvent>,
    pub prevent_action: bool,
    pub override_action: Option<Action>,
}

pub fn ability_label(ability: &str) -> &str {
    match ability {
        "intimidate" => "いかく",
        "download" => "ダウンロード",
        "drought" => "ひでり",
        "moody" => "ムラっけ",
        "libero" => "リベロ",
        "receiver" => "レシーバー",
        "power_of_alchemy" => "かがくのちから",
        "slow_start" => "スロースタート",
        "magic_bounce" => "マジックミラー",
        "aroma_veil" => "アロマベール",
        "lightning_rod" => "ひらいしん",
        "static" => "せいでんき",
        "soundproof" => "ぼうおん",
        "stamina" => "じきゅうりょく",
        "cotton_down" => "わたげ",
        "berserk" => "ぎゃくじょう",
        "competitive" => "かちき",
        "opportunist" => "びんじょう",
        "unburden" => "かるわざ",
        "adaptability" => "てきおうりょく",
        other => other,
    }
}

fn ability_activation_log(creature_name: &str, ability: &str) -> BattleEvent {
    BattleEvent::Log {
        message: format!("{}の 特性『{}』！", creature_name, ability_label(ability)),
        meta: Map::new(),
    }
}

pub fn slow_start_log(creature_name: &str) -> BattleEvent {
    BattleEvent::Log {
        message: format!("{}は 調子が 上がらない！", creature_name),
        meta: Map::new(),
    }
}

pub fn run_ability_value_hook(
    state: &BattleState,
    player_id: &str,
    hook: &str,
    value: f32,
    ctx: AbilityValueContext<'_>,
) -> f32 {
    let Some(active) = get_active_creature(state, player_id) else {
        return value;
    };
    let Some(ability) = active.ability.as_deref() else {
        return value;
    };

    match (ability, hook) {
        ("thick_fat", "onDefensivePower") => {
            match ctx.move_data.and_then(|m| m.move_type.as_deref()) {
                Some("fire") | Some("ice") => value * 0.5,
                _ => value,
            }
        }
        ("fur_coat", "onModifyDefense") => {
            if ctx.category == Some("physical") {
                value * 2.0
            } else {
                value
            }
        }
        ("slow_start", "onModifyOffense") => {
            if ctx.category == Some("physical") && ctx.turn <= 5 {
                value * 0.5
            } else {
                value
            }
        }
        ("slow_start", "onModifySpeed") => {
            if ctx.turn <= 5 {
                value * 0.5
            } else {
                value
            }
        }
        ("sharpness", "onModifyPower") => {
            if ctx
                .move_data
                .map(|m| m.tags.iter().any(|t| t == "slicing"))
                .unwrap_or(false)
            {
                value * 1.5
            } else {
                value
            }
        }
        ("technician", "onModifyPower") => {
            if value <= 60.0 {
                value * 1.5
            } else {
                value
            }
        }
        ("steelworker", "onModifyPower") => {
            if ctx.move_data.and_then(|m| m.move_type.as_deref()) == Some("steel") {
                value * 1.5
            } else {
                value
            }
        }
        ("hustle", "onModifyPower") => {
            if ctx.category == Some("physical") {
                value * 1.5
            } else {
                value
            }
        }
        ("hustle", "onModifyAccuracy") => {
            if ctx.category == Some("physical") {
                value * 0.8
            } else {
                value
            }
        }
        ("pure_power", "onModifyPower") => {
            if ctx.category == Some("physical") {
                value * 2.0
            } else {
                value
            }
        }
        ("guts", "onModifyPower") => {
            if ctx.category == Some("physical") {
                if let Some(active) = get_active_creature(state, player_id) {
                    if !active.statuses.is_empty() {
                        return value * 1.5;
                    }
                }
            }
            value
        }
        ("merciless", "onModifyCritChance") => {
            if let Some(target) = ctx.target {
                if target
                    .statuses
                    .iter()
                    .any(|s| s.id == "poison" || s.id == "toxic")
                {
                    return 999.0;
                }
            }
            value
        }
        ("super_luck", "onModifyCritChance") => value + 1.0,
        ("compound_eyes", "onModifyAccuracy") => value * 1.3,
        ("quick_feet", "onModifySpeed") => {
            if let Some(active) = get_active_creature(state, player_id) {
                if !active.statuses.is_empty() {
                    return value * 1.5;
                }
            }
            value
        }
        ("unburden", "onModifySpeed") => {
            if active
                .ability_data
                .get("unburdenActivated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                value * 2.0
            } else {
                value
            }
        }
        ("swift_swim", "onModifySpeed") => {
            if ctx.weather == Some("rain") {
                value * 2.0
            } else {
                value
            }
        }
        ("chlorophyll", "onModifySpeed") => {
            if ctx.weather == Some("sun") {
                value * 2.0
            } else {
                value
            }
        }
        ("prankster", "onModifyPriority") => {
            if ctx.move_data.map(is_status_move).unwrap_or(false) {
                value + 1.0
            } else {
                value
            }
        }
        _ => value,
    }
}

pub fn run_ability_check_hook(
    state: &BattleState,
    player_id: &str,
    hook: &str,
    ctx: AbilityCheckContext<'_>,
    default_value: bool,
) -> bool {
    let Some(active) = get_active_creature(state, player_id) else {
        return default_value;
    };
    let Some(ability) = active.ability.as_deref() else {
        return default_value;
    };

    match (ability, hook) {
        ("immunity", "onCheckStatusImmunity") => {
            matches!(ctx.status_id, Some("poison") | Some("toxic"))
        }
        ("insomnia", "onCheckStatusImmunity") => ctx.status_id == Some("sleep"),
        ("own_tempo", "onCheckStatusImmunity") => ctx.status_id == Some("confusion"),
        ("own_tempo", "onImmunity") => ctx.r#type == Some("intimidate"),
        ("clear_body", "onImmunity") => ctx.r#type == Some("intimidate"),
        ("white_smoke", "onImmunity") => ctx.r#type == Some("intimidate"),
        ("hyper_cutter", "onImmunity") => ctx.r#type == Some("intimidate"),
        ("klutz", "onCheckItem") => false,
        ("unnerve", "onCheckItem") => false,
        ("shadow_tag", "onTrap") => {
            if let Some(target_id) = ctx.target_id {
                if target_id == player_id {
                    return false;
                }
                let target = get_active_creature(state, target_id);
                if target.and_then(|c| c.ability.as_deref()) == Some("shadow_tag") {
                    return false;
                }
                return true;
            }
            false
        }
        ("skill_link", "onSkillLink") => true,
        _ => default_value,
    }
}

pub fn modify_stages_with_ability(
    state: &BattleState,
    target_id: &str,
    stages: &HashMap<String, i32>,
) -> HashMap<String, i32> {
    let Some(active) = get_active_creature(state, target_id) else {
        return stages.clone();
    };
    let Some(ability) = active.ability.as_deref() else {
        return stages.clone();
    };

    match ability {
        "contrary" => stages.iter().map(|(k, v)| (k.clone(), -v)).collect(),
        "simple" => stages.iter().map(|(k, v)| (k.clone(), v * 2)).collect(),
        _ => stages.clone(),
    }
}

pub fn run_ability_hooks(
    state: &BattleState,
    player_id: &str,
    hook: &str,
    ctx: AbilityHookContext<'_>,
) -> AbilityHookResult {
    let Some(active) = get_active_creature(state, player_id) else {
        return AbilityHookResult::default();
    };
    let Some(ability) = active.ability.as_deref() else {
        return AbilityHookResult::default();
    };

    match (ability, hook) {
        ("intimidate", "onSwitchIn") => {
            if active
                .ability_data
                .get("intimidateUsed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return AbilityHookResult::default();
            }
            let next = mark_ability_used(state, player_id, "intimidateUsed");
            let mut events = vec![ability_activation_log(&active.name, ability)];
            for other in &next.players {
                if other.id == player_id {
                    continue;
                }
                if run_ability_check_hook(
                    &next,
                    &other.id,
                    "onImmunity",
                    AbilityCheckContext {
                        status_id: None,
                        r#type: Some("intimidate"),
                        target_id: None,
                        action: None,
                    },
                    false,
                ) {
                    continue;
                }
                let mut stages = HashMap::new();
                stages.insert("atk".to_string(), -1);
                events.push(BattleEvent::ModifyStage {
                    target_id: other.id.clone(),
                    stages,
                    clamp: true,
                    fail_if_no_change: false,
                    show_event: true,
                    meta: meta_with_move_source(None, Some(player_id)),
                });
            }
            AbilityHookResult {
                state: Some(next),
                events,
                prevent_action: false,
                override_action: None,
            }
        }
        ("download", "onSwitchIn") => {
            if active
                .ability_data
                .get("downloadUsed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return AbilityHookResult::default();
            }
            let target_player = state.players.iter().find(|p| p.id != player_id);
            let Some(target_player) = target_player else {
                return AbilityHookResult::default();
            };
            let Some(target) = get_active_creature(state, &target_player.id) else {
                return AbilityHookResult::default();
            };
            let raise = if target.defense < target.sp_defense {
                "atk"
            } else {
                "spa"
            };
            let next = mark_ability_used(state, player_id, "downloadUsed");
            let mut stages = HashMap::new();
            stages.insert(raise.to_string(), 1);
            AbilityHookResult {
                state: Some(next),
                events: vec![
                    ability_activation_log(&active.name, ability),
                    BattleEvent::ModifyStage {
                        target_id: player_id.to_string(),
                        stages,
                        clamp: true,
                        fail_if_no_change: false,
                        show_event: true,
                        meta: meta_with_move_source(None, Some(player_id)),
                    },
                ],
                prevent_action: false,
                override_action: None,
            }
        }
        ("drought", "onSwitchIn") => {
            if active
                .ability_data
                .get("droughtUsed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return AbilityHookResult::default();
            }
            let mut next = mark_ability_used(state, player_id, "droughtUsed");
            next = set_weather(&next, WeatherKind::Sun, Some(5));
            AbilityHookResult {
                state: Some(next),
                events: vec![
                    ability_activation_log(&active.name, ability),
                    BattleEvent::Log {
                        message: "日差しが 強く なった！".to_string(),
                        meta: Map::new(),
                    },
                ],
                prevent_action: false,
                override_action: None,
            }
        }
        ("slow_start", "onSwitchIn") => AbilityHookResult {
            state: None,
            events: vec![
                ability_activation_log(&active.name, ability),
                slow_start_log(&active.name),
            ],
            prevent_action: false,
            override_action: None,
        },
        ("moody", "onTurnEnd") => {
            let stats = ["atk", "def", "spa", "spd", "spe"];
            let up_index =
                (ctx.rng)().mul_add(stats.len() as f64, 0.0).floor() as usize % stats.len();
            let down_roll = (ctx.rng)().mul_add((stats.len() - 1) as f64, 0.0).floor() as usize
                % (stats.len() - 1);
            let down_index = if down_roll >= up_index {
                down_roll + 1
            } else {
                down_roll
            };
            let mut stages = HashMap::new();
            stages.insert(stats[up_index].to_string(), 2);
            stages.insert(stats[down_index].to_string(), -1);
            AbilityHookResult {
                state: None,
                events: vec![
                    ability_activation_log(&active.name, ability),
                    BattleEvent::ModifyStage {
                        target_id: player_id.to_string(),
                        stages,
                        clamp: true,
                        fail_if_no_change: false,
                        show_event: true,
                        meta: meta_with_move_source(None, Some(player_id)),
                    },
                ],
                prevent_action: false,
                override_action: None,
            }
        }
        ("libero", "onBeforeAction") => {
            let Some(action) = ctx.action else {
                return AbilityHookResult::default();
            };
            let move_id = action.move_id.as_deref();
            let Some(move_id) = move_id else {
                return AbilityHookResult::default();
            };

            // 眠り・氷状態なら発動しない
            if active
                .statuses
                .iter()
                .any(|s| s.id == "sleep" || s.id == "freeze")
            {
                return AbilityHookResult::default();
            }

            if active
                .ability_data
                .get("liberoUsed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return AbilityHookResult::default();
            }
            let Some(move_data) = ctx.move_data else {
                return AbilityHookResult::default();
            };
            let Some(move_type) = move_data.move_type.as_deref() else {
                return AbilityHookResult::default();
            };
            let mut next = state.clone();
            if let Some(player) = next.players.iter_mut().find(|p| p.id == player_id) {
                if let Some(creature) = player.team.get_mut(player.active_slot) {
                    creature.types = vec![move_type.to_string()];
                    creature
                        .ability_data
                        .insert("liberoUsed".to_string(), Value::Bool(true));
                }
            }
            AbilityHookResult {
                state: Some(next),
                events: vec![
                    ability_activation_log(&active.name, ability),
                    BattleEvent::Log {
                        message: format!("{}は {}タイプに 変化した！", active.name, move_type),
                        meta: meta_with_move_source(Some(move_id), Some(player_id)),
                    },
                ],
                prevent_action: false,
                override_action: None,
            }
        }
        ("receiver", "onSwitchIn") => copy_fainted_ability(state, player_id, "receiver"),
        ("power_of_alchemy", "onSwitchIn") => {
            copy_fainted_ability(state, player_id, "power_of_alchemy")
        }
        _ => AbilityHookResult::default(),
    }
}

pub struct AbilityHookContext<'a> {
    pub rng: &'a mut dyn FnMut() -> f64,
    pub action: Option<&'a Action>,
    pub move_data: Option<&'a MoveData>,
}

pub fn run_all_ability_hooks(
    state: &BattleState,
    hook: &str,
    ctx: AbilityHookContext<'_>,
) -> AbilityHookResult {
    let mut working_state = state.clone();
    let mut events = Vec::new();
    for player in &working_state.players.clone() {
        let result = run_ability_hooks(
            &working_state,
            &player.id,
            hook,
            AbilityHookContext {
                rng: ctx.rng,
                action: ctx.action,
                move_data: ctx.move_data,
            },
        );
        if let Some(new_state) = result.state {
            working_state = new_state;
        }
        events.extend(result.events);
    }
    AbilityHookResult {
        state: Some(working_state),
        events,
        prevent_action: false,
        override_action: None,
    }
}

pub fn apply_ability_event_modifiers(
    state: &BattleState,
    events: &[BattleEvent],
    move_db: &std::collections::HashMap<String, MoveData>,
    rng: &mut dyn FnMut() -> f64,
) -> Vec<BattleEvent> {
    let mut output = Vec::new();
    for event in events {
        let mut current_events = vec![event.clone()];
        if let Some(target_id) = event_target_id(event) {
            if let Some(target) = get_active_creature(state, &target_id) {
                if let Some(ability) = target.ability.as_deref() {
                    if ability == "aroma_veil" {
                        if let Some(replacement) = try_aroma_veil(event, state) {
                            current_events = replacement;
                        }
                    }
                    if ability == "magic_bounce" {
                        if let Some(replacement) = try_magic_bounce(event, state, move_db) {
                            current_events = replacement;
                        }
                    }
                    if ability == "lightning_rod" {
                        if let Some(replacement) = try_lightning_rod(event, state, move_db) {
                            current_events = replacement;
                        }
                    }
                }
            }
        }

        for processed in current_events {
            if let Some(target_id) = event_target_id(&processed) {
                if let Some(target) = get_active_creature(state, &target_id) {
                    if target.ability.as_deref() == Some("soundproof") {
                        let is_sound = event_meta_ref(&processed)
                            .and_then(|meta| meta_get_bool(meta, "sound"))
                            .unwrap_or(false);
                        if is_sound {
                            output.push(BattleEvent::Log {
                                message: format!(
                                    "{}の 特性『{}』！",
                                    target.name,
                                    ability_label("soundproof")
                                ),
                                meta: Map::new(),
                            });
                            output.push(BattleEvent::Log {
                                message: format!("{}は 音の技を 受けない！", target.name),
                                meta: Map::new(),
                            });
                            continue;
                        }
                    }
                }
            }
            output.push(processed.clone());
            for player in &state.players {
                if let Some(active) = get_active_creature(state, &player.id) {
                    if let Some(ability) = active.ability.as_deref() {
                        let reactions = match ability {
                            "stamina" => after_stamina(&processed, &player.id, &active.name),
                            "cotton_down" => {
                                after_cotton_down(state, &processed, &player.id, &active.name)
                            }
                            "berserk" => after_berserk(state, &processed, &player.id, &active.name),
                            "competitive" => {
                                after_competitive(&processed, &player.id, &active.name)
                            }
                            "opportunist" => {
                                after_opportunist(&processed, &player.id, &active.name)
                            }
                            "static" => {
                                after_static(&processed, &player.id, &active.name, move_db, rng)
                            }
                            _ => Vec::new(),
                        };
                        output.extend(reactions);
                    }
                }
            }
        }
    }
    output
}

pub fn get_weather(state: &BattleState) -> Option<WeatherKind> {
    state.field.global.iter().find_map(|e| match e.id.as_str() {
        "sun" => Some(WeatherKind::Sun),
        "rain" => Some(WeatherKind::Rain),
        "sandstorm" => Some(WeatherKind::Sandstorm),
        "snow" => Some(WeatherKind::Snow),
        _ => None,
    })
}

pub fn get_terrain_id(state: &BattleState) -> Option<&str> {
    state
        .field
        .global
        .iter()
        .rev()
        .find(|effect| is_terrain_id(&effect.id))
        .map(|effect| effect.id.as_str())
}

fn set_weather(state: &BattleState, weather: WeatherKind, turns: Option<i32>) -> BattleState {
    let mut next = state.clone();
    next.field.global.retain(|e| !is_weather_id(&e.id));
    let id = match weather {
        WeatherKind::Sun => "sun",
        WeatherKind::Rain => "rain",
        WeatherKind::Sandstorm => "sandstorm",
        WeatherKind::Snow => "snow",
    };
    next.field.global.push(crate::core::state::FieldEffect {
        id: id.to_string(),
        remaining_turns: turns,
        data: HashMap::new(),
    });
    next
}

pub fn is_weather_id(id: &str) -> bool {
    matches!(id, "sun" | "rain" | "sandstorm" | "snow")
}

pub fn is_terrain_id(id: &str) -> bool {
    matches!(
        id,
        "electric_terrain" | "grassy_terrain" | "misty_terrain" | "psychic_terrain"
    )
}

fn mark_ability_used(state: &BattleState, player_id: &str, key: &str) -> BattleState {
    let mut next = state.clone();
    if let Some(player) = next.players.iter_mut().find(|p| p.id == player_id) {
        if let Some(creature) = player.team.get_mut(player.active_slot) {
            creature
                .ability_data
                .insert(key.to_string(), Value::Bool(true));
        }
    }
    next
}

fn copy_fainted_ability(
    state: &BattleState,
    player_id: &str,
    ability_id: &str,
) -> AbilityHookResult {
    let ban = [
        "receiver",
        "power_of_alchemy",
        "trace",
        "wonder_guard",
        "forecast",
        "flower_gift",
        "multitype",
        "illusion",
        "imposter",
        "stance_change",
        "power_construct",
        "schooling",
        "comatose",
        "shields_down",
        "disguise",
        "battle_bond",
        "rk_system",
        "ice_face",
        "gulp_missile",
        "hung_switch",
        "commander",
        "quark_drive",
        "protosynthesis",
    ];
    let Some(player) = state.players.iter().find(|p| p.id == player_id) else {
        return AbilityHookResult::default();
    };
    let Some(last) = player.last_fainted_ability.as_deref() else {
        return AbilityHookResult::default();
    };
    if last == ability_id || ban.contains(&last) {
        return AbilityHookResult::default();
    }

    let mut next = state.clone();
    if let Some(player) = next.players.iter_mut().find(|p| p.id == player_id) {
        if let Some(creature) = player.team.get_mut(player.active_slot) {
            if creature.ability.as_deref() != Some(ability_id) {
                return AbilityHookResult::default();
            }
            if !creature.ability_data.contains_key("originalAbility") {
                creature.ability_data.insert(
                    "originalAbility".to_string(),
                    Value::String(creature.ability.clone().unwrap_or_default()),
                );
            }
            creature.ability = Some(last.to_string());
            creature
                .ability_data
                .insert("copiedAbility".to_string(), Value::String(last.to_string()));
        }
    }

    AbilityHookResult {
        state: Some(next),
        events: vec![
            ability_activation_log(&player.name, ability_id),
            BattleEvent::Log {
                message: format!("{}は {}を コピーした！", player.name, ability_label(last)),
                meta: Map::new(),
            },
        ],
        prevent_action: false,
        override_action: None,
    }
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
        | BattleEvent::CureAllStatus { target_id, .. } => Some(target_id.clone()),
        _ => None,
    }
}

fn event_meta_ref(event: &BattleEvent) -> Option<&Map<String, Value>> {
    match event {
        BattleEvent::Damage { meta, .. }
        | BattleEvent::ApplyStatus { meta, .. }
        | BattleEvent::RemoveStatus { meta, .. }
        | BattleEvent::ReplaceStatus { meta, .. }
        | BattleEvent::ModifyStage { meta, .. }
        | BattleEvent::ClearStages { meta, .. }
        | BattleEvent::ResetStages { meta, .. }
        | BattleEvent::CureAllStatus { meta, .. } => Some(meta),
        _ => None,
    }
}

fn try_aroma_veil(event: &BattleEvent, state: &BattleState) -> Option<Vec<BattleEvent>> {
    let target_id = event_target_id(event)?;
    let source_id = event_meta_source(event)?;
    if source_id == target_id {
        return None;
    }
    if !is_aroma_veil_blocked_event(event) {
        return None;
    }

    let target_name = get_active_creature(state, &target_id)
        .map(|creature| creature.name.clone())
        .unwrap_or_else(|| target_id.clone());

    Some(vec![
        ability_activation_log(&target_name, "aroma_veil"),
        BattleEvent::Log {
            message: format!("{}は メンタル攻撃を 受けない！", target_name),
            meta: Map::new(),
        },
    ])
}

fn is_aroma_veil_blocked_event(event: &BattleEvent) -> bool {
    match event {
        BattleEvent::ApplyStatus {
            status_id, meta, ..
        } => {
            matches!(
                status_id.as_str(),
                "torment" | "heal_block" | "disable_move" | "taunt" | "attract" | "infatuation"
            ) || (status_id == "lock_move"
                && meta
                    .get("moveId")
                    .and_then(|value| value.as_str())
                    .is_some_and(|move_id| move_id == "encore"))
        }
        _ => false,
    }
}

fn try_magic_bounce(
    event: &BattleEvent,
    _state: &BattleState,
    move_db: &HashMap<String, MoveData>,
) -> Option<Vec<BattleEvent>> {
    let target_id = event_target_id(event)?;
    let source_id = event_meta_source(event)?;
    if source_id == target_id {
        return None;
    }
    if event_meta_flag(event, "bounced") {
        return None;
    }
    let move_id = event_meta_move_id(event)?;
    let move_data = move_db.get(&move_id)?;
    if !is_reflectable_status_event(event) || !is_status_move(move_data) {
        return None;
    }
    let target_name = get_active_creature(_state, &target_id)
        .map(|creature| creature.name.clone())
        .unwrap_or_else(|| target_id.clone());

    let mut bounced_event = event.clone();
    set_event_target(&mut bounced_event, &source_id);
    set_event_meta(
        &mut bounced_event,
        "source",
        Value::String(target_id.clone()),
    );
    set_event_meta(&mut bounced_event, "bounced", Value::Bool(true));

    Some(vec![
        ability_activation_log(&target_name, "magic_bounce"),
        BattleEvent::Log {
            message: format!("{}は 技を 跳ね返した！", target_id),
            meta: Map::new(),
        },
        bounced_event,
    ])
}

fn try_lightning_rod(
    event: &BattleEvent,
    _state: &BattleState,
    move_db: &HashMap<String, MoveData>,
) -> Option<Vec<BattleEvent>> {
    let move_id = event_meta_move_id(event)?;
    let move_data = move_db.get(&move_id)?;
    if move_data.move_type.as_deref() != Some("electric") {
        return None;
    }
    let target_id = event_target_id(event)?;
    let target_name = get_active_creature(_state, &target_id)
        .map(|creature| creature.name.clone())
        .unwrap_or_else(|| target_id.clone());
    let mut stages = HashMap::new();
    stages.insert("spa".to_string(), 1);
    Some(vec![
        ability_activation_log(&target_name, "lightning_rod"),
        BattleEvent::ModifyStage {
            target_id: target_id.clone(),
            stages,
            clamp: true,
            fail_if_no_change: false,
            show_event: true,
            meta: Map::new(),
        },
        BattleEvent::Log {
            message: format!("{}が 電気の技を 吸い取った！", target_id),
            meta: Map::new(),
        },
    ])
}

fn after_stamina(event: &BattleEvent, player_id: &str, creature_name: &str) -> Vec<BattleEvent> {
    match event {
        BattleEvent::Damage { target_id, .. } if target_id == player_id => {
            let mut stages = HashMap::new();
            stages.insert("def".to_string(), 1);
            vec![
                BattleEvent::Log {
                    message: format!("{}の 特性『{}』！", creature_name, ability_label("stamina")),
                    meta: Map::new(),
                },
                BattleEvent::ModifyStage {
                    target_id: player_id.to_string(),
                    stages,
                    clamp: true,
                    fail_if_no_change: false,
                    show_event: true,
                    meta: Map::new(),
                },
            ]
        }
        _ => Vec::new(),
    }
}

fn after_cotton_down(
    state: &BattleState,
    event: &BattleEvent,
    player_id: &str,
    creature_name: &str,
) -> Vec<BattleEvent> {
    match event {
        BattleEvent::Damage {
            target_id,
            amount,
            meta,
        } if target_id == player_id
            && *amount > 0
            && event_meta_move_id(event).is_some()
            && event_meta_source(event).is_some_and(|source_id| source_id != player_id) =>
        {
            let mut events = vec![BattleEvent::Log {
                message: format!(
                    "{}の 特性『{}』！",
                    creature_name,
                    ability_label("cotton_down")
                ),
                meta: Map::new(),
            }];
            for other in &state.players {
                if other.id == player_id {
                    continue;
                }
                let mut stages = HashMap::new();
                stages.insert("spe".to_string(), -1);
                events.push(BattleEvent::ModifyStage {
                    target_id: other.id.clone(),
                    stages,
                    clamp: true,
                    fail_if_no_change: false,
                    show_event: true,
                    meta: Map::new(),
                });
            }
            events
        }
        _ => Vec::new(),
    }
}

fn after_static(
    event: &BattleEvent,
    player_id: &str,
    creature_name: &str,
    move_db: &HashMap<String, MoveData>,
    rng: &mut dyn FnMut() -> f64,
) -> Vec<BattleEvent> {
    let BattleEvent::Damage {
        target_id, amount, ..
    } = event
    else {
        return Vec::new();
    };
    if target_id != player_id || *amount <= 0 {
        return Vec::new();
    }
    let Some(source_id) = event_meta_source(event) else {
        return Vec::new();
    };
    if source_id == player_id {
        return Vec::new();
    }
    let Some(move_id) = event_meta_move_id(event) else {
        return Vec::new();
    };
    let Some(move_data) = move_db.get(&move_id) else {
        return Vec::new();
    };
    if !move_data.tags.iter().any(|tag| tag == "contact") {
        return Vec::new();
    }
    if rng() >= 0.3 {
        return Vec::new();
    }

    vec![
        BattleEvent::Log {
            message: format!("{}の 特性『{}』！", creature_name, ability_label("static")),
            meta: Map::new(),
        },
        BattleEvent::ApplyStatus {
            target_id: source_id,
            status_id: "paralysis".to_string(),
            duration: None,
            stack: false,
            data: HashMap::new(),
            meta: Map::new(),
        },
    ]
}

fn after_berserk(
    state: &BattleState,
    event: &BattleEvent,
    player_id: &str,
    creature_name: &str,
) -> Vec<BattleEvent> {
    match event {
        BattleEvent::Damage {
            target_id, amount, ..
        } if target_id == player_id => {
            if let Some(target) = get_active_creature(state, player_id) {
                if target.hp > target.max_hp / 2 && target.hp - amount <= target.max_hp / 2 {
                    let mut stages = HashMap::new();
                    stages.insert("spa".to_string(), 1);
                    return vec![
                        BattleEvent::Log {
                            message: format!(
                                "{}の 特性『{}』！",
                                creature_name,
                                ability_label("berserk")
                            ),
                            meta: Map::new(),
                        },
                        BattleEvent::ModifyStage {
                            target_id: player_id.to_string(),
                            stages,
                            clamp: true,
                            fail_if_no_change: false,
                            show_event: true,
                            meta: Map::new(),
                        },
                    ];
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn after_competitive(
    event: &BattleEvent,
    player_id: &str,
    creature_name: &str,
) -> Vec<BattleEvent> {
    match event {
        BattleEvent::ModifyStage {
            target_id,
            stages,
            meta,
            ..
        } if target_id == player_id => {
            if event_meta_flag_raw(meta, "competitive") {
                return Vec::new();
            }
            if stages.values().any(|v| *v < 0) {
                let mut new_stages = HashMap::new();
                new_stages.insert("spa".to_string(), 2);
                let mut meta = Map::new();
                meta.insert("competitive".to_string(), Value::Bool(true));
                return vec![
                    BattleEvent::Log {
                        message: format!(
                            "{}の 特性『{}』！",
                            creature_name,
                            ability_label("competitive")
                        ),
                        meta: Map::new(),
                    },
                    BattleEvent::ModifyStage {
                        target_id: player_id.to_string(),
                        stages: new_stages,
                        clamp: true,
                        fail_if_no_change: false,
                        show_event: true,
                        meta,
                    },
                ];
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn after_opportunist(
    event: &BattleEvent,
    player_id: &str,
    creature_name: &str,
) -> Vec<BattleEvent> {
    match event {
        BattleEvent::ModifyStage {
            target_id,
            stages,
            meta,
            ..
        } if target_id != player_id => {
            if event_meta_flag_raw(meta, "opportunist") {
                return Vec::new();
            }
            let boosts: HashMap<String, i32> = stages
                .iter()
                .filter(|(_, v)| **v > 0)
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            if boosts.is_empty() {
                return Vec::new();
            }
            let mut meta = Map::new();
            meta.insert("opportunist".to_string(), Value::Bool(true));
            vec![
                BattleEvent::Log {
                    message: format!(
                        "{}の 特性『{}』！",
                        creature_name,
                        ability_label("opportunist")
                    ),
                    meta: Map::new(),
                },
                BattleEvent::ModifyStage {
                    target_id: player_id.to_string(),
                    stages: boosts,
                    clamp: true,
                    fail_if_no_change: false,
                    show_event: true,
                    meta,
                },
            ]
        }
        _ => Vec::new(),
    }
}

fn event_meta_move_id(event: &BattleEvent) -> Option<String> {
    match event {
        BattleEvent::Damage { meta, .. }
        | BattleEvent::ApplyStatus { meta, .. }
        | BattleEvent::RemoveStatus { meta, .. }
        | BattleEvent::ReplaceStatus { meta, .. }
        | BattleEvent::ModifyStage { meta, .. }
        | BattleEvent::ClearStages { meta, .. }
        | BattleEvent::ResetStages { meta, .. }
        | BattleEvent::CureAllStatus { meta, .. }
        | BattleEvent::RandomMove { meta, .. }
        | BattleEvent::Log { meta, .. }
        | BattleEvent::ApplyFieldStatus { meta, .. }
        | BattleEvent::RemoveFieldStatus { meta, .. } => meta
            .get("moveId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn event_meta_source(event: &BattleEvent) -> Option<String> {
    match event {
        BattleEvent::Damage { meta, .. }
        | BattleEvent::ApplyStatus { meta, .. }
        | BattleEvent::RemoveStatus { meta, .. }
        | BattleEvent::ReplaceStatus { meta, .. }
        | BattleEvent::ModifyStage { meta, .. }
        | BattleEvent::ClearStages { meta, .. }
        | BattleEvent::ResetStages { meta, .. }
        | BattleEvent::CureAllStatus { meta, .. }
        | BattleEvent::RandomMove { meta, .. }
        | BattleEvent::Log { meta, .. }
        | BattleEvent::ApplyFieldStatus { meta, .. }
        | BattleEvent::RemoveFieldStatus { meta, .. } => meta
            .get("source")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn event_meta_flag(event: &BattleEvent, key: &str) -> bool {
    match event {
        BattleEvent::Damage { meta, .. }
        | BattleEvent::ApplyStatus { meta, .. }
        | BattleEvent::RemoveStatus { meta, .. }
        | BattleEvent::ReplaceStatus { meta, .. }
        | BattleEvent::ModifyStage { meta, .. }
        | BattleEvent::ClearStages { meta, .. }
        | BattleEvent::ResetStages { meta, .. }
        | BattleEvent::CureAllStatus { meta, .. }
        | BattleEvent::RandomMove { meta, .. }
        | BattleEvent::Log { meta, .. }
        | BattleEvent::ApplyFieldStatus { meta, .. }
        | BattleEvent::RemoveFieldStatus { meta, .. } => event_meta_flag_raw(meta, key),
        _ => false,
    }
}

fn event_meta_flag_raw(meta: &Map<String, Value>, key: &str) -> bool {
    meta.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn is_reflectable_status_event(event: &BattleEvent) -> bool {
    matches!(
        event,
        BattleEvent::ApplyStatus { .. }
            | BattleEvent::RemoveStatus { .. }
            | BattleEvent::ReplaceStatus { .. }
            | BattleEvent::ModifyStage { .. }
            | BattleEvent::ClearStages { .. }
            | BattleEvent::ResetStages { .. }
            | BattleEvent::CureAllStatus { .. }
            | BattleEvent::SetAbility { .. }
    )
}

fn set_event_target(event: &mut BattleEvent, target_id: &str) {
    match event {
        BattleEvent::Damage { target_id: t, .. }
        | BattleEvent::ApplyStatus { target_id: t, .. }
        | BattleEvent::RemoveStatus { target_id: t, .. }
        | BattleEvent::ReplaceStatus { target_id: t, .. }
        | BattleEvent::ModifyStage { target_id: t, .. }
        | BattleEvent::ClearStages { target_id: t, .. }
        | BattleEvent::ResetStages { target_id: t, .. }
        | BattleEvent::CureAllStatus { target_id: t, .. }
        | BattleEvent::SetAbility { target_id: t, .. } => {
            *t = target_id.to_string();
        }
        _ => {}
    }
}

fn set_event_meta(event: &mut BattleEvent, key: &str, value: Value) {
    let meta = match event {
        BattleEvent::Damage { meta, .. }
        | BattleEvent::ApplyStatus { meta, .. }
        | BattleEvent::RemoveStatus { meta, .. }
        | BattleEvent::ReplaceStatus { meta, .. }
        | BattleEvent::ModifyStage { meta, .. }
        | BattleEvent::ClearStages { meta, .. }
        | BattleEvent::ResetStages { meta, .. }
        | BattleEvent::CureAllStatus { meta, .. }
        | BattleEvent::RandomMove { meta, .. }
        | BattleEvent::SetAbility { meta, .. }
        | BattleEvent::Log { meta, .. }
        | BattleEvent::ApplyFieldStatus { meta, .. }
        | BattleEvent::RemoveFieldStatus { meta, .. } => meta,
        _ => return,
    };
    meta.insert(key.to_string(), value);
}
