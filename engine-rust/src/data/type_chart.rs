use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TypeEntry {
    pub super_effective: Vec<String>,
    pub resists: Vec<String>,
    pub weak_to: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TypeChart {
    chart: HashMap<String, TypeEntry>,
    immunities: HashMap<String, Vec<String>>,
}

impl TypeChart {
    pub fn new() -> Self {
        let mut chart = HashMap::new();
        let mut add_entry =
            |type_name: &str, super_effective: &[&str], resists: &[&str], weak_to: &[&str]| {
                chart.insert(
                    type_name.to_string(),
                    TypeEntry {
                        super_effective: super_effective.iter().map(|v| v.to_string()).collect(),
                        resists: resists.iter().map(|v| v.to_string()).collect(),
                        weak_to: weak_to.iter().map(|v| v.to_string()).collect(),
                    },
                );
            };

        add_entry("normal", &[], &[], &["fighting"]);
        add_entry(
            "fire",
            &["grass", "ice", "bug", "steel"],
            &["fire", "grass", "ice", "bug", "steel", "fairy"],
            &["water", "ground", "rock"],
        );
        add_entry(
            "water",
            &["fire", "ground", "rock"],
            &["steel", "fire", "water", "ice"],
            &["electric", "grass"],
        );
        add_entry(
            "electric",
            &["water", "flying"],
            &["flying", "steel", "electric"],
            &["ground"],
        );
        add_entry(
            "grass",
            &["water", "ground", "rock"],
            &["ground", "water", "grass", "electric"],
            &["fire", "ice", "poison", "flying", "bug"],
        );
        add_entry(
            "ice",
            &["flying", "ground", "grass", "dragon"],
            &["ice"],
            &["fire", "fighting", "rock", "steel"],
        );
        add_entry(
            "fighting",
            &["normal", "ice", "rock", "dark", "steel"],
            &["rock", "bug", "dark"],
            &["flying", "psychic", "fairy"],
        );
        add_entry(
            "poison",
            &["grass", "fairy"],
            &["grass", "fighting", "poison", "bug", "fairy"],
            &["ground", "psychic"],
        );
        add_entry(
            "ground",
            &["fire", "electric", "poison", "rock", "steel"],
            &["poison", "rock"],
            &["water", "grass", "ice"],
        );
        add_entry(
            "flying",
            &["fighting", "bug", "grass"],
            &["fighting", "bug", "grass"],
            &["electric", "ice", "rock"],
        );
        add_entry(
            "psychic",
            &["fighting", "poison"],
            &["fighting", "psychic"],
            &["bug", "ghost", "dark"],
        );
        add_entry(
            "bug",
            &["grass", "psychic", "dark"],
            &["grass", "fighting", "ground"],
            &["fire", "flying", "rock"],
        );
        add_entry(
            "rock",
            &["flying", "bug", "fire", "ice"],
            &["normal", "flying", "poison", "fire"],
            &["water", "grass", "fighting", "ground", "steel"],
        );
        add_entry(
            "ghost",
            &["ghost", "psychic"],
            &["poison", "bug"],
            &["ghost", "dark"],
        );
        add_entry(
            "dragon",
            &["dragon"],
            &["grass", "fire", "water", "electric"],
            &["ice", "dragon", "fairy"],
        );
        add_entry(
            "dark",
            &["ghost", "psychic"],
            &["ghost", "dark"],
            &["fighting", "bug", "fairy"],
        );
        add_entry(
            "steel",
            &["ice", "rock", "fairy"],
            &[
                "normal", "flying", "rock", "bug", "steel", "grass", "psychic", "ice", "dragon",
                "fairy",
            ],
            &["fire", "fighting", "ground"],
        );
        add_entry(
            "fairy",
            &["fighting", "dragon", "dark"],
            &["fighting", "bug", "dark"],
            &["poison", "steel"],
        );

        let mut immunities = HashMap::new();
        immunities.insert("normal".to_string(), vec!["ghost".to_string()]);
        immunities.insert(
            "ghost".to_string(),
            vec!["normal".to_string(), "fighting".to_string()],
        );
        immunities.insert("steel".to_string(), vec!["poison".to_string()]);
        immunities.insert("flying".to_string(), vec!["ground".to_string()]);
        immunities.insert("dark".to_string(), vec!["psychic".to_string()]);
        immunities.insert("ground".to_string(), vec!["electric".to_string()]);
        immunities.insert("fairy".to_string(), vec!["dragon".to_string()]);

        Self { chart, immunities }
    }

    pub fn effectiveness(&self, move_type: &str, target_types: &[String]) -> f32 {
        if move_type.is_empty() {
            return 1.0;
        }
        let move_key = move_type.to_lowercase();
        let mut multiplier = 1.0;
        for target_type in target_types {
            let target_key = target_type.to_lowercase();
            if let Some(immunities) = self.immunities.get(&target_key) {
                if immunities.iter().any(|t| t == &move_key) {
                    return 0.0;
                }
            }
            if let Some(chart) = self.chart.get(&target_key) {
                if chart.weak_to.iter().any(|t| t == &move_key) {
                    multiplier *= 2.0;
                }
                if chart.resists.iter().any(|t| t == &move_key) {
                    multiplier *= 0.5;
                }
            }
        }
        multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::TypeChart;

    fn types(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn type_chart_matches_attack_to_defense_table_for_regression_edges() {
        let chart = TypeChart::new();

        assert_eq!(chart.effectiveness("fire", &types(&["fire"])), 0.5);
        assert_eq!(chart.effectiveness("ice", &types(&["water"])), 0.5);
        assert_eq!(chart.effectiveness("electric", &types(&["grass"])), 0.5);
        assert_eq!(chart.effectiveness("fighting", &types(&["steel"])), 2.0);
        assert_eq!(chart.effectiveness("water", &types(&["steel"])), 1.0);
        assert_eq!(chart.effectiveness("electric", &types(&["steel"])), 1.0);
        assert_eq!(chart.effectiveness("fire", &types(&["steel"])), 2.0);
        assert_eq!(chart.effectiveness("ground", &types(&["steel"])), 2.0);
        assert_eq!(chart.effectiveness("poison", &types(&["steel"])), 0.0);
    }
}
