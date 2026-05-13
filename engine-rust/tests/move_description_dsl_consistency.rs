use engine_rust::data::moves::{Effect, MoveDatabase};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct EffectSummary {
    status_ids: HashSet<String>,
    stage_keys: HashSet<String>,
    field_status_ids: HashSet<String>,
    has_item_change: bool,
    has_heal: bool,
    has_switch: bool,
}

#[derive(Default)]
struct ExpectedEffects {
    status_ids: HashSet<String>,
    stage_keys: HashSet<String>,
    requires_field_status: bool,
    requires_item_change: bool,
    requires_heal: bool,
    requires_switch: bool,
}

fn normalize_text(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn shuffle_with_seed(values: &mut [String], mut seed: u64) {
    if values.len() < 2 {
        return;
    }
    for idx in (1..values.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let target = (seed as usize) % (idx + 1);
        values.swap(idx, target);
    }
}

fn build_name_to_id_map(move_db: &MoveDatabase) -> HashMap<String, String> {
    move_db
        .as_map()
        .iter()
        .filter_map(|(id, data)| data.name.as_ref().map(|name| (name.clone(), id.clone())))
        .collect()
}

fn collect_effects_from_value(value: Option<&Value>) -> Vec<Effect> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn collect_effect_summary(effects: &[Effect], summary: &mut EffectSummary) {
    for effect in effects {
        match effect.effect_type.as_str() {
            "apply_status" => {
                if let Some(status_id) = effect.data.get("statusId").and_then(|v| v.as_str()) {
                    summary.status_ids.insert(status_id.to_string());
                }
            }
            "replace_status" => {
                if let Some(status_id) = effect.data.get("to").and_then(|v| v.as_str()) {
                    summary.status_ids.insert(status_id.to_string());
                }
                if effect.data.get("from").and_then(|v| v.as_str()) == Some("active")
                    && effect.data.get("to").and_then(|v| v.as_str()) == Some("pending_switch")
                {
                    summary.has_switch = true;
                }
            }
            "modify_stage" => {
                if let Some(Value::Object(stages)) = effect.data.get("stages") {
                    for key in stages.keys() {
                        summary.stage_keys.insert(normalize_stage_key(key));
                    }
                }
            }
            "apply_field_status" => {
                if let Some(status_id) = effect.data.get("statusId").and_then(|v| v.as_str()) {
                    summary.field_status_ids.insert(status_id.to_string());
                }
            }
            "apply_item" | "remove_item" | "consume_item" => {
                summary.has_item_change = true;
            }
            "damage_ratio" => {
                if let Some(ratio) = effect.data.get("ratioMaxHp").and_then(|v| v.as_f64()) {
                    if ratio < 0.0 {
                        summary.has_heal = true;
                    }
                }
            }
            "self_switch" | "replace_pokemon" | "force_switch" => {
                summary.has_switch = true;
            }
            "manual" => {
                if effect
                    .data
                    .get("manualReason")
                    .and_then(|v| v.as_str())
                    .map(|reason| reason.contains("Switching"))
                    .unwrap_or(false)
                {
                    summary.has_switch = true;
                }
            }
            _ => {}
        }

        match effect.effect_type.as_str() {
            "chance" => {
                let nested_then = collect_effects_from_value(effect.data.get("then"));
                let nested_else = collect_effects_from_value(effect.data.get("else"));
                collect_effect_summary(&nested_then, summary);
                collect_effect_summary(&nested_else, summary);
            }
            "repeat" => {
                let nested = collect_effects_from_value(
                    effect
                        .data
                        .get("steps")
                        .or_else(|| effect.data.get("effects")),
                );
                collect_effect_summary(&nested, summary);
            }
            "conditional" => {
                let nested_then = collect_effects_from_value(effect.data.get("then"));
                let nested_else = collect_effects_from_value(effect.data.get("else"));
                collect_effect_summary(&nested_then, summary);
                collect_effect_summary(&nested_else, summary);
            }
            "delay" | "over_time" => {
                let nested = collect_effects_from_value(
                    effect
                        .data
                        .get("steps")
                        .or_else(|| effect.data.get("effects")),
                );
                collect_effect_summary(&nested, summary);
            }
            _ => {}
        }
    }
}

fn normalize_stage_key(key: &str) -> String {
    match key {
        "acc" => "accuracy".to_string(),
        "eva" => "evasion".to_string(),
        other => other.to_string(),
    }
}

fn parse_expected_effects(description: &str) -> ExpectedEffects {
    let text = normalize_text(description);
    let mut expected = ExpectedEffects::default();

    let status_patterns = [
        ("もうどく", "toxic"),
        ("どく", "poison"),
        ("ねむり", "sleep"),
        ("まひ", "paralysis"),
        ("やけど", "burn"),
        ("こおり", "freeze"),
        ("こんらん", "confusion"),
    ];
    for (jp, status_id) in status_patterns {
        let apply_phrase = format!("{}状態にする", jp);
        let become_phrase = format!("{}状態になる", jp);
        if text.contains(&apply_phrase) || text.contains(&become_phrase) {
            expected.status_ids.insert(status_id.to_string());
        }
    }
    if text.contains("ひるませる") || text.contains("ひるみ状態にする") {
        expected.status_ids.insert("flinch".to_string());
    }

    if has_stage_change(&text, "こうげき") {
        expected.stage_keys.insert("atk".to_string());
    }
    if has_stage_change(&text, "ぼうぎょ") {
        expected.stage_keys.insert("def".to_string());
    }
    if has_stage_change(&text, "とくこう") {
        expected.stage_keys.insert("spa".to_string());
    }
    if has_stage_change(&text, "とくぼう") {
        expected.stage_keys.insert("spd".to_string());
    }
    if has_stage_change(&text, "すばやさ") {
        expected.stage_keys.insert("spe".to_string());
    }
    if has_stage_change(&text, "命中率") {
        expected.stage_keys.insert("accuracy".to_string());
    }
    if has_stage_change(&text, "回避率") {
        expected.stage_keys.insert("evasion".to_string());
    }

    if text.contains("天気を")
        || text.contains("フィールドに")
        || text.contains("フィールドを")
        || text.contains("場の状態を")
    {
        expected.requires_field_status = true;
    }

    if text.contains("道具")
        && (text.contains("入れ替")
            || text.contains("奪")
            || text.contains("交換")
            || text.contains("渡")
            || text.contains("受け取"))
    {
        expected.requires_item_change = true;
    }

    if text.contains("回復")
        && (text.contains("HP") || text.contains("最大HP"))
        && !text.contains("ダメージ")
    {
        expected.requires_heal = true;
    }

    if text.contains("交代") || text.contains("入れ替わ") {
        let is_hazard = text.contains("交代する度に")
            || text.contains("交代するたびに")
            || text.contains("交代する毎に")
            || text.contains("交代するごとに");
        let is_non_switch_context = text.contains("入れ替わっても")
            || text.contains("交代しても")
            || text.contains("交代したとしても")
            || text.contains("交代した場合");
        if !is_hazard && !is_non_switch_context {
            expected.requires_switch = true;
        }
    }

    expected
}

fn has_stage_change(text: &str, stat: &str) -> bool {
    let rank_pattern = format!("{}』ランク", stat);
    let rank_pattern_plain = format!("{}ランク", stat);
    if text.contains(&rank_pattern) || text.contains(&rank_pattern_plain) {
        return true;
    }
    let stage_pattern = format!("{}』を", stat);
    let stage_pattern_plain = format!("{}を", stat);
    (text.contains(&stage_pattern) || text.contains(&stage_pattern_plain)) && text.contains("段階")
}

#[test]
fn sampled_description_matches_dsl_effects() {
    let move_db =
        MoveDatabase::load_from_yaml_file("data/moves.yaml".as_ref()).expect("load moves.yaml");
    let name_to_id = build_name_to_id_map(&move_db);

    let mut reader =
        csv::Reader::from_path("data/2期生男子種族値 - 技一覧.csv").expect("open move csv");
    let mut csv_records: Vec<(String, String)> = reader
        .records()
        .filter_map(|record| record.ok())
        .filter_map(|record| {
            let name = record.get(0)?.trim().to_string();
            let description = record.get(8)?.to_string();
            Some((name, description))
        })
        .collect();

    csv_records.retain(|(name, _)| name_to_id.contains_key(name));
    let mut csv_names: Vec<String> = csv_records.iter().map(|(name, _)| name.clone()).collect();
    let sample_size = ((csv_names.len() as f64) * 0.1).round() as usize;
    let sample_size = sample_size.max(1);
    shuffle_with_seed(&mut csv_names, 42);
    csv_names.truncate(sample_size);

    let descriptions_by_name: HashMap<String, String> = csv_records.into_iter().collect();

    for name in csv_names {
        let move_id = name_to_id
            .get(&name)
            .unwrap_or_else(|| panic!("missing moves.yaml id for {}", name));
        let move_data = move_db
            .get(move_id)
            .unwrap_or_else(|| panic!("missing move data for {}", move_id));
        let description = descriptions_by_name
            .get(&name)
            .unwrap_or_else(|| panic!("missing CSV description for {}", name));

        let expected = parse_expected_effects(description);
        if expected.status_ids.is_empty()
            && expected.stage_keys.is_empty()
            && !expected.requires_field_status
            && !expected.requires_item_change
            && !expected.requires_heal
            && !expected.requires_switch
        {
            continue;
        }

        let mut summary = EffectSummary::default();
        collect_effect_summary(&move_data.steps, &mut summary);

        for status_id in &expected.status_ids {
            assert!(
                summary.status_ids.contains(status_id),
                "expected status {} in DSL for move {} ({})",
                status_id,
                name,
                move_id
            );
        }

        for stage_key in &expected.stage_keys {
            assert!(
                summary.stage_keys.contains(stage_key),
                "expected stage {} in DSL for move {} ({})",
                stage_key,
                name,
                move_id
            );
        }

        if expected.requires_field_status {
            assert!(
                !summary.field_status_ids.is_empty(),
                "expected field status in DSL for move {} ({})",
                name,
                move_id
            );
        }

        if expected.requires_item_change {
            assert!(
                summary.has_item_change,
                "expected item change in DSL for move {} ({})",
                name, move_id
            );
        }

        if expected.requires_heal {
            assert!(
                summary.has_heal,
                "expected heal in DSL for move {} ({})",
                name, move_id
            );
        }

        if expected.requires_switch {
            assert!(
                summary.has_switch,
                "expected switch in DSL for move {} ({})",
                name, move_id
            );
        }
    }
}
