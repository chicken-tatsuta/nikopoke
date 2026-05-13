//! Move Generator CLI
//!
//! This tool reads move data from a CSV file and uses Gemini API
//! to generate moves.json DSL entries.
//!
//! Usage:
//!   cargo run --bin move-generator -- \
//!     --csv data/2期生男子種族値\ -\ 技一覧.csv \
//!     --output data/moves_generated.json \
//!     --batch-size 10 \
//!     --delay-ms 200

use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use engine_rust::tools::{
    gemini::{build_move_prompt, find_similar_moves, GeminiClient},
    spell_checker::SpellChecker,
};
// use serde::{Deserialize, Serialize}; // Removed unused
use serde_json::{Map, Value};
use tokio::time::sleep;

#[derive(Debug, Clone)]
struct MoveCSVRecord {
    name: String,
    move_type: String,
    power: String,
    accuracy: String,
    pp: String,
    category: String,
    contact: String,
    effect: String,
}

#[derive(Debug)]
struct Config {
    csv_path: PathBuf,
    output_path: PathBuf,
    existing_moves_path: PathBuf,
    batch_size: usize,
    delay_ms: u64,
    dry_run: bool,
    single_move: Option<String>,
    model: String,
    process_all: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            csv_path: PathBuf::from("data/2期生男子種族値 - 技一覧.csv"),
            output_path: PathBuf::from("data/moves_generated.json"),
            existing_moves_path: PathBuf::from("data/moves.json"),
            batch_size: 10,
            delay_ms: 200,
            dry_run: false,
            single_move: None,
            model: "gemini-2.0-flash".to_string(),
            process_all: false,
        }
    }
}

fn parse_args() -> Config {
    let mut config = Config::default();
    let args: Vec<String> = env::args().collect();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--csv" => {
                if i + 1 < args.len() {
                    config.csv_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    config.output_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--existing" => {
                if i + 1 < args.len() {
                    config.existing_moves_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--batch-size" => {
                if i + 1 < args.len() {
                    config.batch_size = args[i + 1].parse().unwrap_or(10);
                    i += 1;
                }
            }
            "--delay-ms" => {
                if i + 1 < args.len() {
                    config.delay_ms = args[i + 1].parse().unwrap_or(200);
                    i += 1;
                }
            }
            "--dry-run" => {
                config.dry_run = true;
            }
            "--single" => {
                if i + 1 < args.len() {
                    config.single_move = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--model" => {
                if i + 1 < args.len() {
                    config.model = args[i + 1].clone();
                    i += 1;
                }
            }
            "--all" => {
                config.process_all = true;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    config
}

fn print_help() {
    println!(
        r#"Move Generator - Generate moves.json DSL using Gemini API

USAGE:
    cargo run --bin move-generator -- [OPTIONS]

OPTIONS:
    --csv <PATH>        Path to CSV file with move data
                        Default: data/2期生男子種族値 - 技一覧.csv
    --output <PATH>     Path to output JSON file
                        Default: data/moves_generated.json
    --existing <PATH>   Path to existing moves.json for reference
                        Default: data/moves.json
    --all               Process all moves including existing ones
    --batch-size <N>    Number of moves to process in parallel
                        Default: 10
    --delay-ms <MS>     Delay between batches in milliseconds
                        Default: 200
    --dry-run           Don't actually call the API, just print what would be done
    --single <NAME>     Only process a single move by name
    --model <NAME>      Gemini model to use
                        Default: gemini-2.0-flash
    --help, -h          Print this help message

ENVIRONMENT VARIABLES:
    GEMINI_API_KEY      Required. Your Gemini API key.

EXAMPLES:
    # Process all moves
    cargo run --bin move-generator

    # Process a single move (dry run)
    cargo run --bin move-generator -- --single "たいあたり" --dry-run

    # Process with custom settings
    cargo run --bin move-generator -- --batch-size 5 --delay-ms 500
"#
    );
}

// Use read_csv_with_csv_crate instead

fn read_csv_with_csv_crate(path: &PathBuf) -> Result<Vec<MoveCSVRecord>, Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let mut records = Vec::new();

    for result in rdr.records() {
        let record = result?;
        if record.len() >= 8 {
            let move_record = MoveCSVRecord {
                name: record.get(0).unwrap_or("").trim().to_string(),
                move_type: record.get(2).unwrap_or("").trim().to_string(),
                power: record.get(3).unwrap_or("-").trim().to_string(),
                accuracy: record.get(4).unwrap_or("-").trim().to_string(),
                pp: record.get(5).unwrap_or("-").trim().to_string(),
                category: record.get(6).unwrap_or("").trim().to_string(),
                contact: record.get(7).unwrap_or("").trim().to_string(),
                effect: record.get(8).unwrap_or("").trim().to_string(),
            };

            if !move_record.name.is_empty() {
                records.push(move_record);
            }
        }
    }

    Ok(records)
}

fn load_existing_moves(path: &PathBuf) -> Result<Value, Box<dyn Error>> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(Value::Object(Map::new()))
    }
}

fn get_existing_move_names(moves: &Value) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(obj) = moves.as_object() {
        for (_, move_data) in obj {
            if let Some(name) = move_data.get("name").and_then(|n| n.as_str()) {
                names.insert(name.to_string());
            }
        }
    }
    names
}

async fn generate_move(
    client: &GeminiClient,
    record: &MoveCSVRecord,
    existing_moves: &Value,
) -> Result<Value, Box<dyn Error>> {
    let example_moves = find_similar_moves(&record.effect, existing_moves, 2);

    let prompt = build_move_prompt(
        &record.name,
        &record.move_type,
        &record.power,
        &record.accuracy,
        &record.pp,
        &record.category,
        &record.effect,
        &example_moves,
    );

    let mut prompt = prompt;
    let mut retries = 0;
    const MAX_RETRIES: usize = 3;

    let mut move_data = loop {
        let response = client.generate(&prompt).await?;

        // Parse the JSON response
        let parsed: Result<Value, _> = serde_json::from_str(&response);

        match parsed {
            Ok(mut data) => {
                // If it's an array with one element, take that element
                if let Some(arr) = data.as_array() {
                    if let Some(first) = arr.first() {
                        data = first.clone();
                    }
                }

                // Validate with SpellChecker
                match SpellChecker::validate(&data) {
                    Ok(_) => {
                        break data;
                    }
                    Err(e) => {
                        println!("      ⚠️ Validation failed: {}", e);
                        if retries >= MAX_RETRIES {
                            return Err(format!(
                                "Validation failed after {} retries: {}",
                                MAX_RETRIES, e
                            )
                            .into());
                        }
                        // Append error to prompt and retry
                        prompt = format!(
                            "{}\n\n前回の出力は以下のエラーがありました。修正してください:\n{}",
                            prompt, e
                        );
                        retries += 1;
                        println!("      🔄 Retrying ({}/{})...", retries, MAX_RETRIES);
                        sleep(Duration::from_millis(1000)).await;
                    }
                }
            }
            Err(e) => {
                println!("      ⚠️ JSON parsing failed: {}", e);
                if retries >= MAX_RETRIES {
                    return Err(format!(
                        "JSON parsing failed after {} retries: {}",
                        MAX_RETRIES, e
                    )
                    .into());
                }
                prompt = format!(
                    "{}\n\n前回の出力は有効なJSONではありませんでした。修正してください:\n{}",
                    prompt, e
                );
                retries += 1;
                println!("      🔄 Retrying ({}/{})...", retries, MAX_RETRIES);
                sleep(Duration::from_millis(1000)).await;
            }
        }
    };

    // Add contact tag if needed
    if record.contact == "接触" {
        if let Some(obj) = move_data.as_object_mut() {
            let tags = obj.entry("tags").or_insert(Value::Array(Vec::new()));
            if let Some(arr) = tags.as_array_mut() {
                if !arr.iter().any(|t| t.as_str() == Some("contact")) {
                    arr.push(Value::String("contact".to_string()));
                }
            }
        }
    }

    Ok(move_data)
}

async fn process_moves(
    client: &GeminiClient,
    records: Vec<MoveCSVRecord>,
    existing_moves: &Value,
    config: &Config,
) -> Result<Map<String, Value>, Box<dyn Error>> {
    let mut generated: Map<String, Value> = Map::new();
    let total = records.len();

    for (batch_idx, chunk) in records.chunks(config.batch_size).enumerate() {
        println!(
            "\n📦 Processing batch {} ({}-{} of {})",
            batch_idx + 1,
            batch_idx * config.batch_size + 1,
            (batch_idx + 1) * config.batch_size.min(total - batch_idx * config.batch_size),
            total
        );

        for record in chunk {
            print!("  🔄 Generating DSL for '{}'... ", record.name);

            if config.dry_run {
                println!("(dry run - skipped)");
                continue;
            }

            match generate_move(client, record, existing_moves).await {
                Ok(move_data) => {
                    if let Some(id) = move_data.get("id").and_then(|i| i.as_str()) {
                        generated.insert(id.to_string(), move_data);
                        println!("✅");
                    } else {
                        println!("⚠️ No ID in response");
                    }
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }
        }

        // Add delay between batches
        if batch_idx < (total / config.batch_size) {
            sleep(Duration::from_millis(config.delay_ms)).await;
        }
    }

    Ok(generated)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load .env file if present
    dotenv::dotenv().ok();

    let config = parse_args();

    // Get API key
    let api_key = env::var("GEMINI_API_KEY").map_err(|_| {
        "GEMINI_API_KEY environment variable not set. Set it with:\n  export GEMINI_API_KEY=your_key"
    })?;

    println!("🚀 Move Generator - Gemini DSL Generator");
    println!("=========================================");
    println!("📁 CSV: {:?}", config.csv_path);
    println!("📁 Output: {:?}", config.output_path);
    println!("🤖 Model: {}", config.model);
    println!("📦 Batch size: {}", config.batch_size);
    println!("⏱️  Delay: {}ms", config.delay_ms);
    if config.dry_run {
        println!("🏃 Mode: DRY RUN (no API calls)");
    }
    println!();

    // Read CSV
    println!("📖 Reading CSV file...");
    let records = read_csv_with_csv_crate(&config.csv_path)?;
    println!("   Found {} moves in CSV", records.len());

    // Load existing moves
    println!("📖 Loading existing moves...");
    let existing_moves = load_existing_moves(&config.existing_moves_path)?;
    let existing_names = get_existing_move_names(&existing_moves);
    println!("   Found {} existing moves", existing_names.len());

    // Filter records
    let records_to_process: Vec<MoveCSVRecord> = if let Some(ref single) = config.single_move {
        records.into_iter().filter(|r| r.name == *single).collect()
    } else if config.process_all {
        records
    } else {
        records
            .into_iter()
            .filter(|r| !existing_names.contains(&r.name))
            .collect()
    };

    if records_to_process.is_empty() {
        println!("\n✨ No new moves to process!");
        return Ok(());
    }

    println!("\n📝 Will process {} new moves", records_to_process.len());

    // Create client
    let client = GeminiClient::new(api_key, config.model.clone());

    // Process moves
    let generated = process_moves(&client, records_to_process, &existing_moves, &config).await?;

    // Save results
    if !config.dry_run && !generated.is_empty() {
        println!(
            "\n💾 Saving {} generated moves to {:?}",
            generated.len(),
            config.output_path
        );

        // Merge with existing generated moves if file exists
        let mut output_moves: Map<String, Value> = if config.output_path.exists() {
            let content = fs::read_to_string(&config.output_path)?;
            serde_json::from_str(&content).unwrap_or(Map::new())
        } else {
            Map::new()
        };

        for (id, move_data) in generated {
            output_moves.insert(id, move_data);
        }

        let output = serde_json::to_string_pretty(&output_moves)?;
        fs::write(&config.output_path, output)?;

        println!("✅ Done! Generated moves saved to {:?}", config.output_path);
        println!("\n📋 Next steps:");
        println!("   1. Review the generated DSL in {:?}", config.output_path);
        println!("   2. Merge valid entries into data/moves.json");
        println!("   3. Test with: cargo run --bin battle-cli");
    }

    Ok(())
}
