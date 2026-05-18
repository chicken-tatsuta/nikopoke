use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseStats {
    pub hp: i32,
    pub atk: i32,
    pub def: i32,
    pub spa: i32,
    pub spd: i32,
    pub spe: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpeciesData {
    pub id: String,
    pub name: String,
    #[serde(default, alias = "type")]
    pub types: Vec<String>,
    #[serde(rename = "baseStats")]
    pub base_stats: BaseStats,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default, rename = "weight_kg")]
    pub weight_kg: f64,
}

#[derive(Clone, Debug, Default)]
pub struct SpeciesDatabase {
    species: HashMap<String, SpeciesData>,
}

impl SpeciesDatabase {
    pub fn new() -> Self {
        Self {
            species: HashMap::new(),
        }
    }

    pub fn insert(&mut self, data: SpeciesData) {
        self.species.insert(data.id.clone(), data);
    }

    pub fn get(&self, species_id: &str) -> Option<&SpeciesData> {
        self.species.get(species_id)
    }

    pub fn as_map(&self) -> &HashMap<String, SpeciesData> {
        &self.species
    }

    pub fn load_from_yaml_str(yaml: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Direct parse - species.yaml is a simple map of id -> SpeciesData
        let map: HashMap<String, SpeciesData> = serde_yaml::from_str(yaml)?;
        let mut db = Self::new();
        for (_, data) in map {
            db.insert(data);
        }
        Ok(db)
    }

    pub fn load_default() -> Result<Self, Box<dyn std::error::Error>> {
        const DEFAULT_SPECIES_YAML: &str = include_str!("../../data/species.yaml");
        Self::load_from_yaml_str(DEFAULT_SPECIES_YAML)
    }
}
