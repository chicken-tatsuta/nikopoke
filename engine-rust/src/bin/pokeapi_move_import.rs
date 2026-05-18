//! PokeAPI Move Importer
//!
//! Fetches move data from PokeAPI, matches Japanese move names from CSV,
//! generates Nikopoke move DSL (steps), and writes moves.yaml.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
struct Config {
    csv_path: PathBuf,
    output_yaml: PathBuf,
    cache_dir: PathBuf,
    report_path: PathBuf,
    migrate_report_path: PathBuf,
    delay_ms: u64,
    dry_run: bool,
    skip_migrate: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            csv_path: PathBuf::from("data/2期生男子種族値 - 技一覧.csv"),
            output_yaml: PathBuf::from("data/moves.yaml"),
            cache_dir: PathBuf::from("data/cache/pokeapi/move"),
            report_path: PathBuf::from("data/move_import_report.json"),
            migrate_report_path: PathBuf::from("data/move_id_migration_report.json"),
            delay_ms: 120,
            dry_run: false,
            skip_migrate: false,
        }
    }
}

fn parse_args() -> Config {
    let mut cfg = Config::default();
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--csv" => {
                if i + 1 < args.len() {
                    cfg.csv_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    cfg.output_yaml = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--cache-dir" => {
                if i + 1 < args.len() {
                    cfg.cache_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--report" => {
                if i + 1 < args.len() {
                    cfg.report_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--migrate-report" => {
                if i + 1 < args.len() {
                    cfg.migrate_report_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--delay-ms" => {
                if i + 1 < args.len() {
                    cfg.delay_ms = args[i + 1].parse().unwrap_or(cfg.delay_ms);
                    i += 1;
                }
            }
            "--dry-run" => cfg.dry_run = true,
            "--skip-migrate" => cfg.skip_migrate = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    cfg
}

fn print_help() {
    println!(
        r#"pokeapi-move-import - Generate moves.yaml from PokeAPI

USAGE:
  cargo run --bin pokeapi-move-import -- [OPTIONS]

OPTIONS:
  --csv <PATH>            CSV with move names (default: data/2期生男子種族値 - 技一覧.csv)
  --output <PATH>         Output moves.yaml (default: data/moves.yaml)
  --cache-dir <DIR>       Cache dir for PokeAPI responses (default: data/cache/pokeapi/move)
  --report <PATH>         Output report JSON (default: data/move_import_report.json)
  --migrate-report <PATH> Migration report JSON (default: data/move_id_migration_report.json)
  --delay-ms <N>          Delay between PokeAPI calls (default: 120)
  --dry-run               Do not write files
  --skip-migrate          Skip learnsets/frontend ID migration
  --help, -h              Show help
"#
    );
}

#[derive(Debug, Deserialize)]
struct NamedResource {
    name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct MoveListResponse {
    next: Option<String>,
    results: Vec<NamedResource>,
}

#[derive(Debug, Deserialize)]
struct LocalizedName {
    name: String,
    language: NamedResource,
}

#[derive(Debug, Deserialize)]
struct VerboseEffect {
    effect: Option<String>,
    short_effect: Option<String>,
    language: NamedResource,
}

#[derive(Debug, Deserialize)]
struct FlavorText {
    flavor_text: String,
    language: NamedResource,
}

#[derive(Debug, Deserialize)]
struct StatChange {
    change: i32,
    stat: NamedResource,
}

#[derive(Debug, Deserialize)]
struct MoveMeta {
    ailment: NamedResource,
    category: NamedResource,
    min_hits: Option<i32>,
    max_hits: Option<i32>,
    min_turns: Option<i32>,
    max_turns: Option<i32>,
    drain: Option<i32>,
    healing: Option<i32>,
    crit_rate: Option<i32>,
    ailment_chance: Option<i32>,
    flinch_chance: Option<i32>,
    stat_chance: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct MoveDetail {
    name: String,
    accuracy: Option<i32>,
    power: Option<i32>,
    pp: Option<i32>,
    priority: i32,
    damage_class: NamedResource,
    #[serde(rename = "type")]
    move_type: NamedResource,
    names: Vec<LocalizedName>,
    effect_entries: Vec<VerboseEffect>,
    flavor_text_entries: Vec<FlavorText>,
    meta: Option<MoveMeta>,
    stat_changes: Vec<StatChange>,
    target: NamedResource,
}

#[derive(Debug)]
struct CsvMoveRecord {
    name: String,
    contact: String,
}

#[derive(Debug, Serialize)]
struct OutputMove {
    id: String,
    name: String,
    #[serde(rename = "type")]
    move_type: String,
    category: String,
    pp: Option<i32>,
    power: Option<i32>,
    accuracy: Option<f64>,
    priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default)]
    steps: Vec<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(rename = "critRate", skip_serializing_if = "Option::is_none")]
    crit_rate: Option<i32>,
}

#[derive(Debug, Default, Serialize)]
struct ImportReport {
    not_found_in_pokeapi: Vec<String>,
    ambiguous_matches: Vec<AmbiguousMatch>,
    manual_effects: Vec<ManualEffect>,
    id_changes: Vec<IdChange>,
}

#[derive(Debug, Serialize)]
struct AmbiguousMatch {
    csv_name: String,
    candidates: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ManualEffect {
    move_id: String,
    move_name: String,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct IdChange {
    name: String,
    old_id: String,
    new_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut cfg = parse_args();
    let (repo_root, engine_root) = resolve_roots()?;
    cfg.csv_path = resolve_path(&engine_root, &cfg.csv_path);
    cfg.output_yaml = resolve_path(&engine_root, &cfg.output_yaml);
    cfg.cache_dir = resolve_path(&engine_root, &cfg.cache_dir);
    cfg.report_path = resolve_path(&engine_root, &cfg.report_path);
    cfg.migrate_report_path = resolve_path(&engine_root, &cfg.migrate_report_path);

    fs::create_dir_all(&cfg.cache_dir)?;

    println!("📖 Reading CSV: {}", cfg.csv_path.display());
    let csv_records = read_csv(&cfg.csv_path)?;
    println!("   CSV moves: {}", csv_records.len());

    let client = reqwest::Client::builder().no_proxy().build()?;
    println!("🌐 Fetching PokeAPI move list...");
    let move_list = fetch_all_move_list(&client, cfg.delay_ms).await?;
    println!("   PokeAPI moves: {}", move_list.len());

    let mut detail_by_id: HashMap<String, MoveDetail> = HashMap::new();
    let mut name_to_ids: HashMap<String, Vec<String>> = HashMap::new();

    println!("🔎 Building Japanese name map...");
    for (idx, item) in move_list.iter().enumerate() {
        let id = extract_id_from_url(&item.url).unwrap_or_else(|| item.name.clone());
        let cache_path = cfg.cache_dir.join(format!("{id}.json"));
        let detail = fetch_move_detail(&client, &item.url, &cache_path).await?;

        let jp_names = collect_japanese_names(&detail);
        for jp in jp_names {
            let key = normalize_name(&jp);
            if !key.is_empty() {
                name_to_ids.entry(key).or_default().push(id.clone());
            }
        }

        detail_by_id.insert(id.clone(), detail);

        if (idx + 1) % 100 == 0 {
            println!("   Processed {} moves...", idx + 1);
        }
        if cfg.delay_ms > 0 {
            sleep(Duration::from_millis(cfg.delay_ms)).await;
        }
    }

    let mut report = ImportReport::default();
    let mut output_moves: BTreeMap<String, OutputMove> = BTreeMap::new();
    let mut matched_ids: HashSet<String> = HashSet::new();

    for record in &csv_records {
        let key = normalize_name(&record.name);
        let Some(candidates) = name_to_ids.get(&key) else {
            report.not_found_in_pokeapi.push(record.name.clone());
            continue;
        };

        let chosen_id = if candidates.len() == 1 {
            candidates[0].clone()
        } else {
            report.ambiguous_matches.push(AmbiguousMatch {
                csv_name: record.name.clone(),
                candidates: candidates.clone(),
            });
            candidates[0].clone()
        };

        let Some(detail) = detail_by_id.get(&chosen_id) else {
            report.not_found_in_pokeapi.push(record.name.clone());
            continue;
        };

        matched_ids.insert(chosen_id.clone());

        let (steps, manual_reasons) = build_steps(detail);
        let output_id = to_snake_id(&detail.name);
        if !manual_reasons.is_empty() {
            report.manual_effects.push(ManualEffect {
                move_id: output_id.clone(),
                move_name: choose_display_name(detail),
                reasons: manual_reasons,
            });
        }

        let mut tags = Vec::new();
        if record.contact == "接触" {
            tags.push("contact".to_string());
        }

        let description = choose_description(detail);

        let output = OutputMove {
            id: output_id,
            name: choose_display_name(detail),
            move_type: detail.move_type.name.clone(),
            category: detail.damage_class.name.clone(),
            pp: detail.pp,
            power: detail.power,
            accuracy: detail.accuracy.map(|a| a as f64 / 100.0),
            priority: detail.priority,
            description,
            steps,
            tags,
            crit_rate: detail
                .meta
                .as_ref()
                .and_then(|m| m.crit_rate)
                .filter(|v| *v > 0),
        };

        output_moves.insert(output.id.clone(), output);
    }

    if cfg.dry_run {
        println!("🏃 Dry run: skipping file writes.");
        return Ok(());
    }

    println!("📝 Writing moves.yaml: {}", cfg.output_yaml.display());
    write_moves_yaml(&cfg.output_yaml, &output_moves)?;

    println!("📝 Writing report: {}", cfg.report_path.display());
    write_json(&cfg.report_path, &report)?;

    if !cfg.skip_migrate {
        println!("🔁 Migrating move IDs in learnsets/frontend...");
        let changes = migrate_ids(
            &output_moves,
            &cfg.migrate_report_path,
            &engine_root,
            &repo_root,
        )?;
        if !changes.is_empty() {
            report.id_changes = changes;
            write_json(&cfg.report_path, &report)?;
        }
    }

    println!("✅ Done.");
    Ok(())
}

fn read_csv(path: &Path) -> Result<Vec<CsvMoveRecord>, Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;
    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let name = record.get(0).unwrap_or("").trim().to_string();
        if name.is_empty() {
            continue;
        }
        let contact = record.get(7).unwrap_or("").trim().to_string();
        records.push(CsvMoveRecord { name, contact });
    }
    Ok(records)
}

fn resolve_roots() -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let cwd = env::current_dir()?;
    if cwd.join("engine-rust").exists() && cwd.join("frontend").exists() {
        return Ok((cwd.clone(), cwd.join("engine-rust")));
    }
    if cwd
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "engine-rust")
        .unwrap_or(false)
    {
        if let Some(parent) = cwd.parent() {
            return Ok((parent.to_path_buf(), cwd));
        }
    }
    Ok((cwd.clone(), cwd))
}

fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

async fn fetch_all_move_list(
    client: &reqwest::Client,
    delay_ms: u64,
) -> Result<Vec<NamedResource>, Box<dyn Error>> {
    let mut url = "https://pokeapi.co/api/v2/move?limit=200&offset=0".to_string();
    let mut all = Vec::new();
    loop {
        let res = client.get(&url).send().await?;
        let body = res.text().await?;
        let list: MoveListResponse = serde_json::from_str(&body)?;
        all.extend(list.results.into_iter());
        if let Some(next) = list.next {
            url = next;
            if delay_ms > 0 {
                sleep(Duration::from_millis(delay_ms)).await;
            }
        } else {
            break;
        }
    }
    Ok(all)
}

async fn fetch_move_detail(
    client: &reqwest::Client,
    url: &str,
    cache_path: &Path,
) -> Result<MoveDetail, Box<dyn Error>> {
    if cache_path.exists() {
        let content = fs::read_to_string(cache_path)?;
        let value: Value = serde_json::from_str(&content)?;
        return Ok(serde_json::from_value(value)?);
    }

    let res = client.get(url).send().await?;
    let body = res.text().await?;
    fs::write(cache_path, &body)?;
    let value: Value = serde_json::from_str(&body)?;
    Ok(serde_json::from_value(value)?)
}

fn extract_id_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim_end_matches('/');
    trimmed.split('/').last().map(|s| s.to_string())
}

fn collect_japanese_names(detail: &MoveDetail) -> Vec<String> {
    detail
        .names
        .iter()
        .filter(|n| is_japanese_lang(&n.language.name))
        .map(|n| n.name.clone())
        .collect()
}

fn choose_display_name(detail: &MoveDetail) -> String {
    detail
        .names
        .iter()
        .find(|n| n.language.name == "ja-Hrkt")
        .or_else(|| detail.names.iter().find(|n| n.language.name == "ja"))
        .or_else(|| detail.names.iter().find(|n| n.language.name == "ja-kanji"))
        .map(|n| n.name.clone())
        .unwrap_or_else(|| detail.name.clone())
}

fn choose_description(detail: &MoveDetail) -> Option<String> {
    let mut candidates: Vec<String> = detail
        .effect_entries
        .iter()
        .filter(|e| is_japanese_lang(&e.language.name))
        .filter_map(|e| e.short_effect.clone().or(e.effect.clone()))
        .collect();
    if candidates.is_empty() {
        candidates = detail
            .flavor_text_entries
            .iter()
            .filter(|e| is_japanese_lang(&e.language.name))
            .map(|e| e.flavor_text.replace('\n', " "))
            .collect();
    }
    if candidates.is_empty() {
        candidates = detail
            .effect_entries
            .iter()
            .filter(|e| e.language.name == "en")
            .filter_map(|e| e.short_effect.clone().or(e.effect.clone()))
            .collect();
    }
    candidates.into_iter().next()
}

fn is_japanese_lang(lang: &str) -> bool {
    matches!(lang, "ja-Hrkt" | "ja" | "ja-kanji")
}

fn normalize_name(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_whitespace() {
            continue;
        }
        if "・･・/／-−ー―–—~〜()（）[]【】".contains(ch) {
            continue;
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn to_snake_id(name: &str) -> String {
    name.to_lowercase()
        .replace('-', "_")
        .replace(' ', "_")
        .replace('’', "")
        .replace('\'', "")
}

fn build_steps(detail: &MoveDetail) -> (Vec<Value>, Vec<String>) {
    let mut steps = Vec::new();
    let mut manual_reasons = Vec::new();

    let accuracy = detail.accuracy.map(|a| a as f64 / 100.0);
    let power = detail.power;
    let meta = detail.meta.as_ref();

    let is_ohko = meta
        .and_then(|m| Some(m.category.name.as_str() == "ohko"))
        .unwrap_or(false);

    if is_ohko {
        let mut effect = Map::new();
        effect.insert("type".to_string(), Value::String("ohko".to_string()));
        if let Some(acc) = accuracy {
            effect.insert(
                "baseAccuracy".to_string(),
                Value::Number(serde_json::Number::from_f64(acc).unwrap()),
            );
        }
        steps.push(Value::Object(effect));
        return (steps, manual_reasons);
    }

    let multi_hit =
        meta.and_then(|m| m.min_hits).is_some() || meta.and_then(|m| m.max_hits).is_some();
    if let Some(p) = power {
        let damage_effect = damage_step(p, accuracy);
        if multi_hit {
            let mut repeat = Map::new();
            repeat.insert("type".to_string(), Value::String("repeat".to_string()));
            let min_hits = meta.and_then(|m| m.min_hits).unwrap_or(2);
            let max_hits = meta.and_then(|m| m.max_hits).unwrap_or(min_hits);
            repeat.insert(
                "times".to_string(),
                json!({ "min": min_hits, "max": max_hits }),
            );
            repeat.insert("steps".to_string(), Value::Array(vec![damage_effect]));
            steps.push(Value::Object(repeat));
        } else {
            steps.push(damage_effect);
        }
    }

    if let Some(meta) = meta {
        if meta.drain.unwrap_or(0) != 0 {
            manual_reasons.push("Drain/recoil effects are not supported".to_string());
        }
        if meta.healing.unwrap_or(0) > 0 && power.is_some() {
            manual_reasons.push("Damage+healing effects are not supported".to_string());
        }
        if meta.min_turns.unwrap_or(0) > 0 || meta.max_turns.unwrap_or(0) > 0 {
            manual_reasons.push("Turn-based trapping/binding is not supported".to_string());
        }
    }

    if power.is_none() {
        if let Some(meta) = meta {
            if meta.healing.unwrap_or(0) > 0 {
                let ratio = -(meta.healing.unwrap_or(0) as f64 / 100.0);
                let mut effect = Map::new();
                effect.insert(
                    "type".to_string(),
                    Value::String("damage_ratio".to_string()),
                );
                effect.insert(
                    "ratioMaxHp".to_string(),
                    Value::Number(serde_json::Number::from_f64(ratio).unwrap()),
                );
                effect.insert("target".to_string(), Value::String("self".to_string()));
                steps.push(Value::Object(effect));
            }
        }
    }

    if let Some(meta) = meta {
        if meta.flinch_chance.unwrap_or(0) > 0 {
            let chance = meta.flinch_chance.unwrap_or(0) as f64 / 100.0;
            let apply = json!({
                "type": "apply_status",
                "statusId": "flinch",
                "target": "target"
            });
            steps.push(chance_wrap(chance, apply));
        }

        if meta.ailment.name != "none" && meta.ailment_chance.unwrap_or(0) > 0 {
            if let Some(status_id) = map_ailment(&meta.ailment.name) {
                let chance = meta.ailment_chance.unwrap_or(0) as f64 / 100.0;
                let target = status_target(detail);
                let apply = json!({
                    "type": "apply_status",
                    "statusId": status_id,
                    "target": target
                });
                steps.push(chance_wrap(chance, apply));
            } else {
                manual_reasons.push(format!("Unsupported ailment: {}", meta.ailment.name));
            }
        } else if meta.ailment.name != "none" && meta.ailment_chance.unwrap_or(0) == 0 {
            if let Some(status_id) = map_ailment(&meta.ailment.name) {
                let target = status_target(detail);
                let apply = json!({
                    "type": "apply_status",
                    "statusId": status_id,
                    "target": target
                });
                steps.push(apply);
            } else {
                manual_reasons.push(format!("Unsupported ailment: {}", meta.ailment.name));
            }
        }
    }

    if !detail.stat_changes.is_empty() {
        let target = decide_stat_target(detail);
        let mut stages = Map::new();
        for change in &detail.stat_changes {
            let key = map_stat_key(&change.stat.name);
            if let Some(k) = key {
                stages.insert(k.to_string(), Value::Number(change.change.into()));
            }
        }
        if !stages.is_empty() {
            let mut effect = Map::new();
            effect.insert(
                "type".to_string(),
                Value::String("modify_stage".to_string()),
            );
            effect.insert("target".to_string(), Value::String(target));
            effect.insert("stages".to_string(), Value::Object(stages));
            if let Some(meta) = detail.meta.as_ref() {
                let chance = meta.stat_chance.unwrap_or(0);
                if chance > 0 && chance < 100 {
                    steps.push(chance_wrap(chance as f64 / 100.0, Value::Object(effect)));
                } else {
                    steps.push(Value::Object(effect));
                }
            } else {
                steps.push(Value::Object(effect));
            }
        }
    }

    if let Some(meta) = meta {
        if meta.category.name.contains("force-switch") {
            steps.push(json!({ "type": "force_switch" }));
        }
        if meta.category.name.contains("field-effect")
            || meta.category.name.contains("whole-field-effect")
        {
            manual_reasons.push("Field effects are not supported".to_string());
        }
    }

    if steps.is_empty() {
        manual_reasons.push("No supported effects inferred".to_string());
        let reason = manual_reasons.join("; ");
        steps.push(json!({ "type": "manual", "manualReason": reason }));
    } else if !manual_reasons.is_empty() {
        let reason = manual_reasons.join("; ");
        steps.push(json!({ "type": "manual", "manualReason": reason }));
    }

    (steps, manual_reasons)
}

fn damage_step(power: i32, accuracy: Option<f64>) -> Value {
    let mut effect = Map::new();
    effect.insert("type".to_string(), Value::String("damage".to_string()));
    effect.insert("power".to_string(), Value::Number(power.into()));
    if let Some(acc) = accuracy {
        if let Some(num) = serde_json::Number::from_f64(acc) {
            effect.insert("accuracy".to_string(), Value::Number(num));
        }
    }
    Value::Object(effect)
}

fn chance_wrap(p: f64, then_step: Value) -> Value {
    json!({
        "type": "chance",
        "p": p,
        "then": [then_step]
    })
}

fn map_ailment(name: &str) -> Option<&'static str> {
    match name {
        "paralysis" => Some("paralysis"),
        "burn" => Some("burn"),
        "poison" => Some("poison"),
        "bad-poison" => Some("bad_poison"),
        "sleep" => Some("sleep"),
        "freeze" => Some("freeze"),
        "confusion" => Some("confusion"),
        "flinch" => Some("flinch"),
        _ => None,
    }
}

fn map_stat_key(name: &str) -> Option<&'static str> {
    match name {
        "attack" => Some("atk"),
        "defense" => Some("def"),
        "special-attack" => Some("spa"),
        "special-defense" => Some("spd"),
        "speed" => Some("spe"),
        "accuracy" => Some("accuracy"),
        "evasion" => Some("evasion"),
        _ => None,
    }
}

fn decide_stat_target(detail: &MoveDetail) -> String {
    let target = detail.target.name.as_str();
    if target.starts_with("user") {
        return "self".to_string();
    }

    let hint = choose_effect_text_hint(detail);
    if let Some(text) = hint {
        let text_lower = text.to_lowercase();
        if text_lower.contains("user's") || text_lower.contains("user’s") {
            return "self".to_string();
        }
        if text_lower.contains("target's") || text_lower.contains("opponent's") {
            return "target".to_string();
        }
    }

    if let Some(meta) = detail.meta.as_ref() {
        if meta.category.name.contains("raise") || meta.category.name.contains("net-good-stats") {
            return "self".to_string();
        }
        if meta.category.name.contains("lower") || meta.category.name.contains("net-bad-stats") {
            return "target".to_string();
        }
    }

    "target".to_string()
}

fn choose_effect_text_hint(detail: &MoveDetail) -> Option<String> {
    detail
        .effect_entries
        .iter()
        .find(|e| e.language.name == "en")
        .and_then(|e| e.short_effect.clone().or(e.effect.clone()))
}

fn status_target(detail: &MoveDetail) -> String {
    if detail.target.name.starts_with("user") {
        "self".to_string()
    } else {
        "target".to_string()
    }
}

fn write_moves_yaml(
    path: &Path,
    moves: &BTreeMap<String, OutputMove>,
) -> Result<(), Box<dyn Error>> {
    let yaml = serde_yaml::to_string(moves)?;
    fs::write(path, yaml)?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    let json_text = serde_json::to_string_pretty(value)?;
    fs::write(path, json_text)?;
    Ok(())
}

fn migrate_ids(
    output_moves: &BTreeMap<String, OutputMove>,
    report_path: &Path,
    engine_root: &Path,
    repo_root: &Path,
) -> Result<Vec<IdChange>, Box<dyn Error>> {
    let old_id_map = load_existing_name_to_id(&engine_root.join("data/moves"))?;
    let mut changes = Vec::new();
    for (new_id, m) in output_moves {
        if let Some(old_id) = old_id_map.get(&m.name) {
            if old_id != new_id {
                changes.push(IdChange {
                    name: m.name.clone(),
                    old_id: old_id.clone(),
                    new_id: new_id.clone(),
                });
            }
        }
    }

    if changes.is_empty() {
        return Ok(changes);
    }

    let change_map: HashMap<String, String> = changes
        .iter()
        .map(|c| (c.old_id.clone(), c.new_id.clone()))
        .collect();

    update_json_ids(&engine_root.join("data/learnsets.json"), &change_map)?;
    update_yaml_ids(&engine_root.join("data/learnsets.yaml"), &change_map)?;

    let frontend_data_dir = repo_root.join("frontend/public/data");
    if frontend_data_dir.exists() {
        for entry in fs::read_dir(frontend_data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                update_json_ids(&path, &change_map)?;
            }
        }
    }

    write_json(report_path, &changes)?;
    Ok(changes)
}

fn load_existing_name_to_id(dir: &Path) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut map = HashMap::new();
    let mut files = Vec::new();
    collect_yaml_files(dir, &mut files)?;
    for path in files {
        let content = fs::read_to_string(&path)?;
        let yaml: Value = serde_yaml::from_str(&content)?;
        if let Some(obj) = yaml.as_object() {
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let id = obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !name.is_empty() && !id.is_empty() {
                map.insert(name, id);
            }
        }
    }
    Ok(map)
}

fn collect_yaml_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(&path, files)?;
            continue;
        }
        let is_yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "yaml" | "yml"))
            .unwrap_or(false);
        if is_yaml {
            files.push(path);
        }
    }
    Ok(())
}

fn update_json_ids(
    path: &Path,
    change_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let mut value: Value = serde_json::from_str(&content)?;
    update_value_ids(&mut value, change_map);
    let output = serde_json::to_string_pretty(&value)?;
    fs::write(path, output)?;
    Ok(())
}

fn update_yaml_ids(
    path: &Path,
    change_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let mut value: Value = serde_yaml::from_str(&content)?;
    update_value_ids(&mut value, change_map);
    let output = serde_yaml::to_string(&value)?;
    fs::write(path, output)?;
    Ok(())
}

fn update_value_ids(value: &mut Value, change_map: &HashMap<String, String>) {
    match value {
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                update_value_ids(v, change_map);
            }
        }
        Value::Object(obj) => {
            let keys: Vec<String> = obj.keys().cloned().collect();
            for key in keys {
                if let Some(new_key) = change_map.get(&key) {
                    if let Some(val) = obj.remove(&key) {
                        obj.insert(new_key.clone(), val);
                    }
                }
            }
            for (_, v) in obj.iter_mut() {
                update_value_ids(v, change_map);
            }
        }
        Value::String(s) => {
            if let Some(new) = change_map.get(s) {
                *s = new.clone();
            }
        }
        _ => {}
    }
}
