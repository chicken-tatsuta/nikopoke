use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveData {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub move_type: Option<String>,
    pub category: Option<String>,
    pub pp: Option<i32>,
    pub power: Option<i32>,
    pub accuracy: Option<f32>,
    pub priority: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub steps: Vec<Effect>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "critRate")]
    pub crit_rate: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    #[serde(rename = "type")]
    pub effect_type: String,
    #[serde(flatten)]
    pub data: Map<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct MoveDatabase {
    moves: HashMap<String, MoveData>,
}

impl MoveDatabase {
    pub fn new() -> Self {
        Self {
            moves: HashMap::new(),
        }
    }

    pub fn minimal() -> Self {
        let mut db = Self::new();
        db.insert(MoveData {
            id: "tackle".to_string(),
            name: Some("Tackle".to_string()),
            move_type: Some("normal".to_string()),
            category: Some("physical".to_string()),
            pp: Some(35),
            power: Some(40),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            crit_rate: None,
        });
        db.insert(MoveData {
            id: "ember".to_string(),
            name: Some("Ember".to_string()),
            move_type: Some("fire".to_string()),
            category: Some("special".to_string()),
            pp: Some(25),
            power: Some(40),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            crit_rate: None,
        });
        db.insert(MoveData {
            id: "water_gun".to_string(),
            name: Some("Water Gun".to_string()),
            move_type: Some("water".to_string()),
            category: Some("special".to_string()),
            pp: Some(25),
            power: Some(40),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            crit_rate: None,
        });
        db.insert(MoveData {
            id: "vine_whip".to_string(),
            name: Some("Vine Whip".to_string()),
            move_type: Some("grass".to_string()),
            category: Some("physical".to_string()),
            pp: Some(25),
            power: Some(45),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            crit_rate: None,
        });
        db.insert(MoveData {
            id: "thunder_shock".to_string(),
            name: Some("Thunder Shock".to_string()),
            move_type: Some("electric".to_string()),
            category: Some("special".to_string()),
            pp: Some(30),
            power: Some(40),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            crit_rate: None,
        });
        db.insert(MoveData {
            id: "growl".to_string(),
            name: Some("Growl".to_string()),
            move_type: Some("normal".to_string()),
            category: Some("status".to_string()),
            pp: Some(40),
            power: Some(0),
            accuracy: Some(1.0),
            priority: Some(0),
            description: None,
            steps: Vec::new(),
            tags: Vec::new(),
            crit_rate: None,
        });
        db
    }

    pub fn load_default() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let dir = Path::new("data/moves");
            if dir.exists() {
                if let Ok(db) = Self::load_from_yaml_dir(dir) {
                    return Ok(db);
                }
            }
        }
        const DEFAULT_MOVES_YAML: &str = include_str!("../../data/moves.yaml");
        Self::load_from_yaml_str(DEFAULT_MOVES_YAML)
    }

    pub fn insert(&mut self, move_data: MoveData) {
        self.moves.insert(move_data.id.clone(), move_data);
    }

    pub fn get(&self, move_id: &str) -> Option<&MoveData> {
        self.moves.get(move_id)
    }

    pub fn as_map(&self) -> &HashMap<String, MoveData> {
        &self.moves
    }

    pub fn load_from_yaml_str(yaml: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Parse YAML, convert to JSON, then deserialize to maintain serde_json types
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml)?;
        let json_value = yaml_to_json(yaml_value);

        if let Ok(map) = serde_json::from_value::<HashMap<String, MoveData>>(json_value.clone()) {
            let mut db = Self::new();
            for (_, move_data) in map {
                db.insert(move_data);
            }
            return Ok(db);
        }

        if let Ok(vec_result) = serde_json::from_value::<Vec<MoveData>>(json_value.clone()) {
            let mut db = Self::new();
            for move_data in vec_result {
                db.insert(move_data);
            }
            return Ok(db);
        }

        let move_data: MoveData = serde_json::from_value(json_value)?;
        let mut db = Self::new();
        db.insert(move_data);
        Ok(db)
    }

    pub fn load_from_yaml_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let db = Self::load_from_yaml_str(&content)?;
        Ok(db)
    }

    pub fn load_from_yaml_dir(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        collect_yaml_files(dir, &mut files)?;
        files.sort();

        let mut db = Self::new();
        for path in files {
            let content = fs::read_to_string(&path)?;
            let file_db = Self::load_from_yaml_str(&content)?;
            for (_, move_data) in file_db.moves {
                db.insert(move_data);
            }
        }
        Ok(db)
    }
}

impl Default for MoveDatabase {
    fn default() -> Self {
        Self::load_default().unwrap_or_else(|_| Self::minimal())
    }
}

/// Convert serde_yaml::Value to serde_json::Value
fn yaml_to_json(yaml: serde_yaml::Value) -> serde_json::Value {
    match yaml {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.into_iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .into_iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        serde_yaml::Value::String(s) => s,
                        _ => return None,
                    };
                    Some((key, yaml_to_json(v)))
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

fn collect_yaml_files(
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
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
