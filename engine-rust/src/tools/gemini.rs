//! Gemini API client for generating move DSL
//!
//! This module provides a client for interacting with Google's Gemini API
//! to generate moves.json DSL from natural language descriptions.

use serde::{Deserialize, Serialize};
use std::error::Error;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Gemini API client
pub struct GeminiClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "topK")]
    top_k: i32,
    #[serde(rename = "topP")]
    top_p: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: i32,
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
    #[serde(rename = "code")]
    _code: Option<i32>,
}

impl GeminiClient {
    /// Create a new Gemini client
    ///
    /// # Arguments
    /// * `api_key` - Gemini API key
    /// * `model` - Model name (e.g., "gemini-2.0-flash", "gemini-2.5-flash")
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    /// Generate move DSL from a prompt
    pub async fn generate(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            GEMINI_API_URL, self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.1, // Low temperature for consistent DSL generation
                top_k: 40,
                top_p: 0.95,
                max_output_tokens: 8192,
                response_mime_type: "application/json".to_string(),
            },
        };

        let response = self.client.post(&url).json(&request).send().await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(format!("API request failed with status {}: {}", status, body).into());
        }

        let gemini_response: GeminiResponse = serde_json::from_str(&body)?;

        if let Some(error) = gemini_response.error {
            return Err(format!("Gemini API error: {}", error.message).into());
        }

        let text = gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or("No response from Gemini API")?;

        // Clean markdown code blocks if present
        let clean_text = if text.contains("```json") {
            text.split("```json")
                .nth(1)
                .unwrap_or(&text)
                .split("```")
                .next()
                .unwrap_or(&text)
                .trim()
                .to_string()
        } else if text.contains("```") {
            text.split("```")
                .nth(1)
                .unwrap_or(&text)
                .split("```")
                .next()
                .unwrap_or(&text)
                .trim()
                .to_string()
        } else {
            text.trim().to_string()
        };

        Ok(clean_text)
    }
}

/// Build a prompt for generating move DSL
pub fn build_move_prompt(
    name: &str,
    move_type: &str,
    power: &str,
    accuracy: &str,
    pp: &str,
    category: &str,
    effect: &str,
    example_moves: &str,
) -> String {
    format!(
        r#"あなたはポケモンバトルエンジンのDSLエキスパートです。
以下の技の説明文を、moves.json形式のDSLに変換してください。

## 入力
- わざ名: {name}
- タイプ: {move_type}
- いりょく: {power}
- めいちゅう: {accuracy}
- PP: {pp}
- ぶんるい: {category}
- 効果: {effect}

## 出力形式
以下のJSON形式で出力してください。idはわざ名をローマ字（スネークケース）に変換してください。

```json
{{
  "id": "move_id",
  "name": "わざ名（日本語）",
  "type": "タイプ（小文字英語）",
  "category": "physical/special/status",
  "pp": PP数値,
  "power": 威力数値またはnull,
  "accuracy": 命中率（0.0-1.0）またはnull,
  "priority": 優先度（通常は0）,
  "description": "技の説明文",
  "steps": [
    {{
      "type": "damage",
      "power": 威力,
      "accuracy": 1.0
    }},
    // その他の効果
  ],
  "tags": ["sound", "contact", etc.] // 該当する場合のみ
}}
```

## 利用可能な Effect Types

### ダメージ系
- `"type": "damage"` - ダメージを与える
  - `power`: 威力, `accuracy`: 命中率
- `"type": "damage_ratio"` - HP割合ダメージ
  - `ratioMaxHp`: 割合（-0.5 = 回復, 0.5 = ダメージ）, `target`: "self"/"target"
- `"type": "ohko"` - 一撃必殺
  - `baseAccuracy`: 基本命中率

### ステータス変化系
- `"type": "modify_stage"` - 能力ランク変更
  - `target`: "self"/"target", `stages`: {{"atk": 2, "def": -1, etc.}}
- `"type": "apply_status"` - 状態異常付与
  - `statusId`: "burn"/"paralysis"/"sleep"/"poison"/"bad_poison"/"freeze"/"confusion"/"flinch"など
  - `chance`: 確率, `target`: "self"/"target"
- `"type": "remove_status"` - 状態異常解除

### 条件分岐系
- `"type": "chance"` - 確率で効果発動
  - `p`: 確率(0.0-1.0), `then`: [手順配列], `else`: [手順配列]（省略可）
- `"type": "conditional"` - 条件分岐
  - `if`: {{ "type": "condition_type", ... }}, `then`: [手順配列], `else`: [手順配列]
- `"type": "repeat"` - 連続攻撃
  - `times`: {{ "min": 2, "max": 5 }} または固定数, `steps`: [手順配列]

### その他
- `"type": "protect"` - まもる系
- `"type": "wait"` - 遅延効果
  - `turns`: ターン数, `steps`: [手順配列]
- `"type": "over_time"` - 継続効果
  - `duration`: ターン数, `steps`: [手順配列]
- `"type": "force_switch"` - 強制交代
- `"type": "apply_field_status"` - フィールド状態付与
- `"type": "self_switch"` - 自分交代

## タイプ対応表
- ノーマル: normal
- ほのお: fire
- みず: water
- でんき: electric
- くさ: grass
- こおり: ice
- かくとう: fighting
- どく: poison
- じめん: ground
- ひこう: flying
- エスパー: psychic
- むし: bug
- いわ: rock
- ゴースト: ghost
- ドラゴン: dragon
- あく: dark
- はがね: steel
- フェアリー: fairy

## ぶんるい対応表
- 物理: physical
- 特殊: special
- 変化: status

## 既存の類似技の例（参考にしてください）
{example_moves}

## 重要なルール
1. JSONのみを出力してください。説明文は不要です。
2. "-" は null として扱ってください
3. 命中率は百分率から小数に変換してください（100 → 1.0, 85 → 0.85）
4. 基本項目（id/name/type/category/pp/power/accuracy/priority/description/steps/tags/critRate）はこの順序で並べてください。
5. 攻撃技（威力がある技）には必ず `steps` 配列の最初に `"type": "damage"` を含めてください。
6. `conditional` や `if` に使用する条件は、必ず以下の形式のオブジェクトにしてください：
   - {{ "type": "target_has_status", "statusId": "..." }}
   - {{ "type": "user_has_status", "statusId": "..." }}
   - {{ "type": "field_has_status", "statusId": "..." }}
   - {{ "type": "target_hp_lt", "value": 0.5 }}
   - {{ "type": "weather_is_sunny" }} / `weather_is_raining` / `weather_is_hail` / `weather_is_sandstorm`
   - {{ "type": "user_type", "typeId": "..." }}
7. まだエンジンでサポートされていない `first_turn` や `weight` などの複雑な条件は、可能な限り `"type": "log"` などで説明を記述するか、将来の拡張のために独自の `type` 名を持つオブジェクトにしてください（文字列は不可）。
8. 接触技には tags に "contact" を追加してください
9. 音系の技には tags に "sound" を追加してください
10. critRate（急所ランク）が高い技は "critRate": 1 などを追加してください

出力:"#,
        name = name,
        move_type = move_type,
        power = power,
        accuracy = accuracy,
        pp = pp,
        category = category,
        effect = effect,
        example_moves = example_moves
    )
}

/// Find similar moves from existing moves.json for few-shot examples
pub fn find_similar_moves(
    effect_description: &str,
    existing_moves: &serde_json::Value,
    max_examples: usize,
) -> String {
    let keywords = extract_keywords(effect_description);
    let mut examples = Vec::new();

    if let Some(moves) = existing_moves.as_object() {
        for (id, move_data) in moves {
            if examples.len() >= max_examples {
                break;
            }

            // Check if the move has steps that match keywords
            if let Some(steps) = move_data.get("steps").and_then(|e| e.as_array()) {
                for effect in steps {
                    if let Some(effect_type) = effect.get("type").and_then(|t| t.as_str()) {
                        if keywords.iter().any(|k| effect_type.contains(k)) {
                            examples.push(format!(
                                "// {} の例:\n{}",
                                id,
                                serde_json::to_string_pretty(move_data).unwrap_or_default()
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }

    if examples.is_empty() {
        "（類似技の例なし）".to_string()
    } else {
        examples.join("\n\n")
    }
}

fn extract_keywords(effect: &str) -> Vec<&str> {
    let mut keywords = Vec::new();

    if effect.contains("ダメージ") || effect.contains("攻撃") {
        keywords.push("damage");
    }
    if effect.contains("状態")
        || effect.contains("どく")
        || effect.contains("まひ")
        || effect.contains("やけど")
        || effect.contains("ねむり")
        || effect.contains("こおり")
    {
        keywords.push("status");
    }
    if effect.contains("確率") || effect.contains("%") {
        keywords.push("chance");
    }
    if effect.contains("ランク") || effect.contains("上げる") || effect.contains("下げる")
    {
        keywords.push("stage");
    }
    if effect.contains("連続") {
        keywords.push("repeat");
    }
    if effect.contains("まもる") || effect.contains("みきり") {
        keywords.push("protect");
    }
    if effect.contains("回復") {
        keywords.push("damage_ratio");
    }
    if effect.contains("交代") {
        keywords.push("switch");
    }

    keywords
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let keywords = extract_keywords("30%の確率で相手をまひ状態にする");
        assert!(keywords.contains(&"chance"));
        assert!(keywords.contains(&"status"));
    }

    #[test]
    fn test_build_prompt() {
        let prompt = build_move_prompt(
            "たいあたり",
            "ノーマル",
            "40",
            "100",
            "35",
            "物理",
            "通常攻撃。",
            "",
        );
        assert!(prompt.contains("たいあたり"));
        assert!(prompt.contains("ノーマル"));
    }
}
