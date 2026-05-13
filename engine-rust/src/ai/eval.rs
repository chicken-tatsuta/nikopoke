use crate::core::state::{BattleState, CreatureState, PlayerState};

const WIN_SCORE: f32 = 1_000_000.0;
const ALIVE_BONUS: f32 = 350.0;
const ACTIVE_HP_BONUS: f32 = 0.6;

pub fn evaluate_state(state: &BattleState, player_id: &str) -> f32 {
    let Some(player) = state.players.iter().find(|p| p.id == player_id) else {
        return 0.0;
    };

    let Some(opponent) = state.players.iter().find(|p| p.id != player_id) else {
        return 0.0;
    };

    let player_alive = alive_count(player);
    let opponent_alive = alive_count(opponent);

    if player_alive == 0 {
        return -WIN_SCORE;
    }

    if opponent_alive == 0 {
        return WIN_SCORE;
    }

    score_player(player) - score_player(opponent)
}

fn score_player(player: &PlayerState) -> f32 {
    let mut score = 0.0;

    for (slot, creature) in player.team.iter().enumerate() {
        score += score_creature(creature);

        if slot == player.active_slot && creature.hp > 0 {
            score += score_active_creature(creature);
        }
    }

    score
}

fn score_creature(creature: &CreatureState) -> f32 {
    if creature.hp <= 0 {
        return -120.0;
    }

    let hp = creature.hp.max(0) as f32;
    let max_hp = creature.max_hp.max(1) as f32;
    let hp_ratio = hp / max_hp;

    let mut score = 0.0;

    // 生きていること自体をかなり重く見る。
    score += ALIVE_BONUS;

    // 残りHPも評価する。
    score += hp;
    score += hp_ratio * 180.0;

    // 能力ランク。
    score += score_stages(creature);

    // 状態異常・継続状態。
    score += score_statuses(creature);

    score
}

fn score_active_creature(creature: &CreatureState) -> f32 {
    let hp = creature.hp.max(0) as f32;
    let max_hp = creature.max_hp.max(1) as f32;
    let hp_ratio = hp / max_hp;

    let mut score = 0.0;

    // 今場に出ているポケモンのHPを少し重視。
    score += hp_ratio * 120.0 * ACTIVE_HP_BONUS;

    // 瀕死寸前のアクティブは危険。
    if hp_ratio <= 0.2 {
        score -= 120.0;
    } else if hp_ratio <= 0.4 {
        score -= 50.0;
    }

    score
}

fn alive_count(player: &PlayerState) -> usize {
    player
        .team
        .iter()
        .filter(|creature| creature.hp > 0)
        .count()
}

fn score_stages(creature: &CreatureState) -> f32 {
    let stages = &creature.stages;

    score_stage(stages.atk, 28.0)
        + score_stage(stages.def, 22.0)
        + score_stage(stages.spa, 28.0)
        + score_stage(stages.spd, 22.0)
        + score_stage(stages.spe, 18.0)
        + score_stage(stages.accuracy, 14.0)
        + score_stage(stages.evasion, 16.0)
}

fn score_stage(stage: i32, weight: f32) -> f32 {
    stage.clamp(-6, 6) as f32 * weight
}

fn score_statuses(creature: &CreatureState) -> f32 {
    let mut score = 0.0;

    for status in &creature.statuses {
        score += score_status_id(status.id.as_str());
    }

    score
}

fn score_status_id(status_id: &str) -> f32 {
    match status_id {
        // 主要状態異常。自分についているとマイナス。
        "sleep" => -170.0,
        "freeze" | "frozen" => -190.0,
        "paralysis" | "paralyze" | "paralyzed" => -95.0,
        "burn" | "burned" => -100.0,
        "poison" | "poisoned" => -75.0,
        "toxic" | "badly_poison" | "badly_poisoned" => -120.0,

        // 行動阻害。
        "confusion" | "confused" => -45.0,
        "flinch" => -35.0,
        "taunt" => -35.0,
        "encore" => -30.0,
        "disable" => -30.0,
        "torment" => -20.0,

        // 継続ダメージ・拘束系。
        "leech_seed" => -80.0,
        "bind" | "wrap" | "fire_spin" | "whirlpool" | "infestation" => -45.0,

        // 良い状態。
        "substitute" => 85.0,
        "protect" => 20.0,
        "endure" => 20.0,
        "aqua_ring" => 45.0,
        "ingrain" => 35.0,
        "focus_energy" => 35.0,

        // よく分からない状態は軽く評価。
        _ => 0.0,
    }
}
