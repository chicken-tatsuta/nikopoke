use serde_json::Value;

pub struct SpellChecker;

impl SpellChecker {
    pub fn validate(json: &Value) -> Result<(), String> {
        // 1. Check if it's an object
        let obj = json.as_object().ok_or("Output is not a JSON object")?;

        // 2. Check required fields
        let required_fields = [
            "id",
            "name",
            "type",
            "category",
            "pp",
            "power",
            "accuracy",
            "priority",
            "steps",
            "description",
        ];

        for field in required_fields {
            if !obj.contains_key(field) {
                return Err(format!("Missing required field: '{}'", field));
            }
        }

        // 3. Check types of specific fields
        if !obj["steps"].is_array() {
            return Err("'steps' must be an array".to_string());
        }

        if let Some(category) = obj["category"].as_str() {
            if !["physical", "special", "status"].contains(&category) {
                return Err(format!(
                    "Invalid category: '{}'. Must be physical, special, or status",
                    category
                ));
            }
        } else {
            return Err("'category' must be a string".to_string());
        }

        if let Some(type_str) = obj["type"].as_str() {
            let valid_types = [
                "normal", "fire", "water", "electric", "grass", "ice", "fighting", "poison",
                "ground", "flying", "psychic", "bug", "rock", "ghost", "dragon", "dark", "steel",
                "fairy",
            ];
            if !valid_types.contains(&type_str) {
                return Err(format!("Invalid type: '{}'.", type_str));
            }
        } else {
            return Err("'type' must be a string".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_valid_move() {
        let valid_json = json!({
            "id": "tackle",
            "name": "たいあたり",
            "type": "normal",
            "category": "physical",
            "pp": 35,
            "power": 40,
            "accuracy": 1.0,
            "priority": 0,
            "steps": [
                {
                    "type": "damage",
                    "power": 40,
                    "accuracy": 1.0
                }
            ],
            "description": "通常攻撃。"
        });

        assert!(SpellChecker::validate(&valid_json).is_ok());
    }

    #[test]
    fn test_validate_missing_field() {
        let invalid_json = json!({
            "id": "tackle",
            "name": "たいあたり",
            // Missing type
            "category": "physical",
            "pp": 35,
            "power": 40,
            "accuracy": 1.0,
            "priority": 0,
            "steps": [],
            "description": "test"
        });

        assert!(SpellChecker::validate(&invalid_json).is_err());
    }

    #[test]
    fn test_validate_invalid_category() {
        let invalid_json = json!({
            "id": "tackle",
            "name": "たいあたり",
            "type": "normal",
            "category": "invalid",
            "pp": 35,
            "power": 40,
            "accuracy": 1.0,
            "priority": 0,
            "steps": [],
            "description": "test"
        });

        assert!(SpellChecker::validate(&invalid_json).is_err());
    }
}
