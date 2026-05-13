use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use serde_yaml::{Mapping, Value};

#[derive(Debug)]
struct Config {
    input_path: PathBuf,
    output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_path: PathBuf::from("data/moves.yaml"),
            output_dir: PathBuf::from("data/moves"),
        }
    }
}

fn parse_args() -> Config {
    let mut config = Config::default();
    let args: Vec<String> = env::args().collect();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                if i + 1 < args.len() {
                    config.input_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    config.output_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
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
        r#"split-moves-yaml - Split moves.yaml into per-move YAML files

USAGE:
    cargo run --bin split-moves-yaml -- [OPTIONS]

OPTIONS:
    --input <PATH>     Input moves YAML file (default: data/moves.yaml)
    --output <DIR>     Output directory (default: data/moves)
    --help, -h         Print this help message
"#
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_args();
    let content = fs::read_to_string(&config.input_path)?;
    let yaml: Value = serde_yaml::from_str(&content)?;
    let map = yaml
        .as_mapping()
        .ok_or("moves.yaml should be a map of move_id -> move data")?;

    let mut grouped: BTreeMap<String, Vec<(String, Value)>> = BTreeMap::new();
    for (key, value) in map {
        let move_id = key.as_str().ok_or("move id must be string")?.to_string();
        let move_type = extract_move_type(value).unwrap_or_else(|| "unknown".to_string());
        grouped
            .entry(move_type)
            .or_default()
            .push((move_id, value.clone()));
    }

    for (move_type, moves) in grouped {
        let dir = config.output_dir.join(move_type);
        fs::create_dir_all(&dir)?;
        for (move_id, move_value) in moves {
            let file_path = dir.join(format!("{move_id}.yaml"));
            let ordered = ordered_move_value(&move_value);
            let yaml_text = serde_yaml::to_string(&ordered)?;
            fs::write(file_path, yaml_text)?;
        }
    }

    println!("Split moves into {}", config.output_dir.display());
    Ok(())
}

fn extract_move_type(value: &Value) -> Option<String> {
    let Value::Mapping(map) = value else {
        return None;
    };
    map.get(&Value::String("type".to_string()))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| {
            map.get(&Value::String("move_type".to_string()))
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        })
}

fn ordered_move_value(move_value: &Value) -> Value {
    let Value::Mapping(map) = move_value else {
        return move_value.clone();
    };

    let ordered_keys = [
        "id",
        "name",
        "type",
        "category",
        "pp",
        "power",
        "accuracy",
        "priority",
        "description",
        "steps",
        "tags",
        "critRate",
    ];

    let mut ordered = Mapping::new();
    for key in ordered_keys {
        let value_key = Value::String(key.to_string());
        if let Some(value) = map.get(&value_key) {
            ordered.insert(value_key, value.clone());
        }
    }

    for (key, value) in map {
        if !ordered.contains_key(key) {
            ordered.insert(key.clone(), value.clone());
        }
    }

    Value::Mapping(ordered)
}
