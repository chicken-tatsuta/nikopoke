use std::collections::HashMap;
use std::fs;

fn normalize_description(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn load_csv_descriptions() -> HashMap<String, String> {
    let mut descriptions = HashMap::new();
    let mut reader =
        csv::Reader::from_path("data/2期生男子種族値 - 技一覧.csv").expect("open move csv");
    for result in reader.records() {
        let record = result.expect("read move record");
        let name = record.get(0).expect("move name").trim().to_string();
        let Some(effect) = record.get(8) else {
            continue;
        };
        let effect = effect.to_string();
        descriptions.insert(name, effect);
    }
    descriptions
}

fn load_move_descriptions() -> HashMap<String, String> {
    let content = fs::read_to_string("data/moves.json").expect("read moves.json");
    let json: serde_json::Value = serde_json::from_str(&content).expect("parse moves.json");
    let obj = json.as_object().expect("moves.json should be object");
    let mut descriptions = HashMap::new();
    for value in obj.values() {
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .expect("move name in moves.json");
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        descriptions.insert(name.to_string(), description.to_string());
    }
    descriptions
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

#[test]
fn sampled_move_descriptions_match_csv() {
    let csv_descriptions = load_csv_descriptions();
    let json_descriptions = load_move_descriptions();

    let mut move_names: Vec<String> = csv_descriptions
        .keys()
        .filter(|name| json_descriptions.contains_key(*name))
        .cloned()
        .collect();
    if move_names.is_empty() {
        return;
    }
    let sample_size = ((move_names.len() as f64) * 0.3).round() as usize;
    let sample_size = sample_size.max(1);
    shuffle_with_seed(&mut move_names, 42);
    let sample = &move_names[..sample_size];

    for name in sample {
        let csv_description = csv_descriptions
            .get(name)
            .unwrap_or_else(|| panic!("missing CSV description for {}", name));
        let json_description = json_descriptions
            .get(name)
            .unwrap_or_else(|| panic!("missing moves.json description for {}", name));
        assert_eq!(
            normalize_description(csv_description),
            normalize_description(json_description),
            "description mismatch for {}",
            name
        );
    }
}
