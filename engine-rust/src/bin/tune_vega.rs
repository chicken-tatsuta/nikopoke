//! Vega パラメーターチューニング（2段階SPRT版）
//!
//! Stage 1: candidate vs weak_baseline（DEFAULT_PARAMS固定）
//!   → DEFAULT より明らかに弱い候補を素早く棄却
//!
//! Stage 2: Stage 1通過 → candidate vs best_params
//!   → 現在のベストを本当に上回るか精密判定

use engine_rust::ai::vega::{
    get_best_move_vega_with_options_and_db_ref, VegaParams, DEFAULT_PARAMS,
};
use engine_rust::core::battle::{
    is_battle_over, replace_fainted_pokemon, step_battle, BattleOptions,
};
use engine_rust::core::factory::{create_creature, CreateCreatureOptions};
use engine_rust::core::state::{create_battle_state, BattleState, PlayerState};
use engine_rust::data::learnsets::LearnsetDatabase;
use engine_rust::data::moves::MoveDatabase;
use engine_rust::data::species::SpeciesDatabase;
use rayon::prelude::*;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::time::Instant;

// ──────────────────────────────────────────
// データ構造
// ──────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
struct TeamEntry {
    team: Vec<TeamPokemon>,
}

#[derive(Clone, Debug, Deserialize)]
struct TeamPokemon {
    species_id: String,
    moves: Vec<String>,
}

struct Databases {
    species: SpeciesDatabase,
    learnsets: LearnsetDatabase,
    moves: MoveDatabase,
}

#[derive(Clone, Debug)]
struct Options {
    iterations: usize,
    /// Stage 1 の最大ゲーム数（weak_baseline 戦）
    stage1_max_games: usize,
    /// Stage 1 の SPRT H1 勝率差（DEFAULT に対して何%強ければ通過）
    stage1_elo: f32,
    /// Stage 2 の最大ゲーム数（best_params 戦）
    max_games: usize,
    /// Stage 2 の SPRT H1 勝率差
    sprt_elo: f32,
    batch_size: usize,
    depth: usize,
    branch_limit: usize,
    max_turns: usize,
    seed: u64,
    progress_file: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            iterations: 500,
            stage1_max_games: 80,
            stage1_elo: 0.05,
            max_games: 200,
            sprt_elo: 0.05,
            batch_size: 20,
            depth: 2,
            branch_limit: 2, // ← 3 から 2 に変更（約5倍速）
            max_turns: 40,
            seed: 20260509,
            progress_file: "tune_vega_progress.json".to_string(),
        }
    }
}

// ──────────────────────────────────────────
// LCG 乱数
// ──────────────────────────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }
    fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
    fn usize(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u32() as usize) % upper
        }
    }
}

// ──────────────────────────────────────────
// SPRT
// ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum SprtDecision {
    Better,
    NotBetter,
    Uncertain,
}

/// SPRT で candidate vs baseline を直接対戦。
/// `max_games` `h1_elo` を外から渡せる汎用版。
fn sprt_duel(
    candidate: VegaParams,
    baseline: VegaParams,
    teams: &[Vec<TeamPokemon>],
    db: &Databases,
    options: &Options,
    seed: u64,
    max_games: usize,
    h1_elo: f32,
) -> (SprtDecision, usize, usize, usize) {
    let p0 = 0.5_f32;
    let p1 = 0.5 + h1_elo;
    let alpha = 0.05_f32;
    let beta = 0.20_f32;
    let log_a = (beta / (1.0 - alpha)).ln();
    let log_b = ((1.0 - beta) / alpha).ln();

    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut draws = 0usize;
    let mut rng = Lcg::new(seed);
    let mut game = 0;
    let mut decision = SprtDecision::Uncertain;

    while game < max_games {
        let batch = options.batch_size.min(max_games - game);
        let setups: Vec<(usize, usize, bool)> = (0..batch)
            .map(|g| {
                (
                    rng.usize(teams.len()),
                    rng.usize(teams.len()),
                    (game + g) % 2 == 0,
                )
            })
            .collect();

        let (bw, bl, bd) = setups
            .par_iter()
            .map(|&(ai, bi, cand_as_a)| {
                let winner = play_duel(
                    &teams[ai], &teams[bi], cand_as_a, candidate, baseline, db, options,
                );
                match winner {
                    Some("a") if cand_as_a => (1usize, 0usize, 0usize),
                    Some("b") if !cand_as_a => (1, 0, 0),
                    Some(_) => (0, 1, 0),
                    None => (0, 0, 1),
                }
            })
            .reduce(
                || (0, 0, 0),
                |(w1, l1, d1), (w2, l2, d2)| (w1 + w2, l1 + l2, d1 + d2),
            );

        wins += bw;
        losses += bl;
        draws += bd;
        game += batch;

        let eff_w = wins as f32 + draws as f32 * 0.5;
        let eff_l = losses as f32 + draws as f32 * 0.5;
        if eff_w + eff_l > 0.0 {
            let log_lr = eff_w * (p1 / p0).ln() + eff_l * ((1.0 - p1) / (1.0 - p0)).ln();
            if log_lr >= log_b {
                decision = SprtDecision::Better;
                break;
            } else if log_lr <= log_a {
                decision = SprtDecision::NotBetter;
                break;
            }
        }
    }

    (decision, wins, losses, draws)
}

fn play_duel(
    team_a: &[TeamPokemon],
    team_b: &[TeamPokemon],
    candidate_as_a: bool,
    candidate: VegaParams,
    baseline: VegaParams,
    db: &Databases,
    options: &Options,
) -> Option<&'static str> {
    let player_a = create_player("a", team_a, db)?;
    let player_b = create_player("b", team_b, db)?;
    let mut state = create_battle_state(vec![player_a, player_b]);
    state = settle_forced_switches(&state);

    for _ in 0..options.max_turns {
        if is_battle_over(&state) {
            break;
        }
        let (params_a, params_b) = if candidate_as_a {
            (candidate, baseline)
        } else {
            (baseline, candidate)
        };
        let action_a = get_best_move_vega_with_options_and_db_ref(
            &state,
            "a",
            options.depth,
            params_a,
            options.branch_limit,
            &db.moves,
        );
        let action_b = get_best_move_vega_with_options_and_db_ref(
            &state,
            "b",
            options.depth,
            params_b,
            options.branch_limit,
            &db.moves,
        );
        let (Some(action_a), Some(action_b)) = (action_a, action_b) else {
            break;
        };
        let mut rng = || 0.42;
        state = step_battle(
            &state,
            &[action_a, action_b],
            &mut rng,
            BattleOptions {
                record_history: false,
            },
        );
        state = settle_forced_switches(&state);
    }

    winner_by_state(&state)
}

// ──────────────────────────────────────────
// main
// ──────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_options();
    let db = Databases {
        species: SpeciesDatabase::load_default()?,
        learnsets: LearnsetDatabase::load_default()?,
        moves: MoveDatabase::load_default()?,
    };
    let teams = load_teams()?;
    let mut rng = Lcg::new(options.seed);
    let start = Instant::now();

    println!(
        "Vega 2-stage SPRT: iterations={} branch={} depth={} \
         s1(max={} elo={:.2}) s2(max={} elo={:.2}) batch={}",
        options.iterations,
        options.branch_limit,
        options.depth,
        options.stage1_max_games,
        options.stage1_elo,
        options.max_games,
        options.sprt_elo,
        options.batch_size,
    );
    flush();

    // weak_baseline は DEFAULT_PARAMS 固定
    // （"defaultより弱い・bestより2回り弱い"基準点）
    let weak_baseline = DEFAULT_PARAMS;
    let mut best_params = DEFAULT_PARAMS;
    let mut new_count = 0usize;
    let mut total_games = 0usize;

    for iter in 1..=options.iterations {
        let scale = 0.22 * (1.0 - iter as f32 / options.iterations.max(1) as f32) + 0.035;
        let candidate = mutate_params(best_params, &mut rng, scale);
        let seed = options.seed.wrapping_add(iter as u64 * 7919);
        let elapsed = || {
            let s = start.elapsed().as_secs();
            format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
        };

        // ── Stage 1: candidate vs DEFAULT_PARAMS ──────────────────────
        let (s1, w1, l1, d1) = sprt_duel(
            candidate,
            weak_baseline,
            &teams,
            &db,
            &options,
            seed,
            options.stage1_max_games,
            options.stage1_elo,
        );
        let g1 = w1 + l1 + d1;
        total_games += g1;
        let sc1 = score(w1, l1, d1);

        // Better: SPRT確定通過 / Uncertain + 勝ち越し: max_games到達でも通過
        let s1_pass = matches!(s1, SprtDecision::Better)
            || (matches!(s1, SprtDecision::Uncertain) && w1 > l1);

        if !s1_pass {
            println!("iter={iter:03} S1✗ sc={sc1:.3} W-L-D={w1}-{l1}-{d1} games={g1}");
            flush();
            continue;
        }

        // ── Stage 2: candidate vs best_params ─────────────────────────
        let (s2, w2, l2, d2) = sprt_duel(
            candidate,
            best_params,
            &teams,
            &db,
            &options,
            seed.wrapping_add(99991),
            options.max_games,
            options.sprt_elo,
        );
        let g2 = w2 + l2 + d2;
        total_games += g2;
        let sc2 = score(w2, l2, d2);

        let is_new = matches!(s2, SprtDecision::Better)
            || (matches!(s2, SprtDecision::Uncertain) && w2 >= l2);

        if is_new {
            best_params = candidate;
            new_count += 1;
            println!(
                "iter={iter:03} NEW  s1={sc1:.2}({g1}g) s2={sc2:.3} W-L-D={w2}-{l2}-{d2} ({g2}g) \
                 total={total_games} t={}",
                elapsed()
            );
            println!("{best_params:#?}");
            flush();
            write_progress(
                &options,
                iter,
                new_count,
                total_games,
                start.elapsed().as_secs(),
                sc2,
                &best_params,
            );
        } else {
            println!(
                "iter={iter:03} S2✗  s1={sc1:.2}({g1}g) s2={sc2:.3} W-L-D={w2}-{l2}-{d2} ({g2}g)"
            );
            flush();
        }
    }

    let elapsed_s = start.elapsed().as_secs();
    println!("\nBEST_PARAMS={best_params:#?}");
    println!(
        "DONE: new={new_count} total_games={total_games} t={}h{:02}m",
        elapsed_s / 3600,
        (elapsed_s % 3600) / 60,
    );
    flush();
    write_progress(
        &options,
        options.iterations,
        new_count,
        total_games,
        elapsed_s,
        0.0,
        &best_params,
    );
    Ok(())
}

// ──────────────────────────────────────────
// 進捗 JSON 書き出し（手元から確認用）
// ──────────────────────────────────────────

fn write_progress(
    options: &Options,
    iter: usize,
    new_count: usize,
    total_games: usize,
    elapsed_secs: u64,
    latest_score: f32,
    best: &VegaParams,
) {
    if options.progress_file.is_empty() {
        return;
    }
    let pct = iter * 100 / options.iterations.max(1);
    let eta_secs = if iter > 0 {
        elapsed_secs * (options.iterations - iter) as u64 / iter as u64
    } else {
        0
    };
    let json = format!(
        r#"{{
  "iter": {iter},
  "iterations": {},
  "pct": {pct},
  "new_count": {new_count},
  "total_games": {total_games},
  "elapsed_secs": {elapsed_secs},
  "eta_secs": {eta_secs},
  "latest_score": {latest_score:.4},
  "best_params": {{
    "alive": {}, "hp": {}, "hp_ratio": {}, "active_hp": {},
    "outgoing": {}, "incoming": {},
    "ko_fast": {}, "ko_slow": {}, "risk_fast": {}, "risk_slow": {},
    "speed": {}, "bench": {}, "stage": {}, "status": {},
    "switch_pressure": {}, "switch_danger": {}, "switch_hp": {},
    "action_damage": {}, "action_priority_ko": {}, "action_accuracy": {}
  }}
}}"#,
        options.iterations,
        best.alive,
        best.hp,
        best.hp_ratio,
        best.active_hp,
        best.outgoing,
        best.incoming,
        best.ko_fast,
        best.ko_slow,
        best.risk_fast,
        best.risk_slow,
        best.speed,
        best.bench,
        best.stage,
        best.status,
        best.switch_pressure,
        best.switch_danger,
        best.switch_hp,
        best.action_damage,
        best.action_priority_ko,
        best.action_accuracy,
    );
    let _ = fs::write(&options.progress_file, json);
}

fn score(w: usize, l: usize, d: usize) -> f32 {
    (w as f32 + d as f32 * 0.5) / (w + l + d).max(1) as f32
}

fn flush() {
    let _ = std::io::stdout().flush();
}

// ──────────────────────────────────────────
// オプション解析
// ──────────────────────────────────────────

fn parse_options() -> Options {
    let mut o = Options::default();
    for arg in env::args().skip(1) {
        let Some((k, v)) = arg.strip_prefix("--").and_then(|a| a.split_once('=')) else {
            continue;
        };
        match k {
            "iterations" => o.iterations = v.parse().unwrap_or(o.iterations),
            "stage1-max-games" => o.stage1_max_games = v.parse().unwrap_or(o.stage1_max_games),
            "stage1-elo" => o.stage1_elo = v.parse().unwrap_or(o.stage1_elo),
            "max-games" => o.max_games = v.parse().unwrap_or(o.max_games),
            "sprt-elo" => o.sprt_elo = v.parse().unwrap_or(o.sprt_elo),
            "batch-size" => o.batch_size = v.parse().unwrap_or(o.batch_size),
            "depth" => o.depth = v.parse().unwrap_or(o.depth),
            "branch-limit" => o.branch_limit = v.parse().unwrap_or(o.branch_limit),
            "max-turns" => o.max_turns = v.parse().unwrap_or(o.max_turns),
            "seed" => o.seed = v.parse().unwrap_or(o.seed),
            "progress-file" => o.progress_file = v.to_string(),
            _ => {}
        }
    }
    o
}

// ──────────────────────────────────────────
// ゲーム・プレイヤー作成
// ──────────────────────────────────────────

fn create_player(id: &str, team: &[TeamPokemon], db: &Databases) -> Option<PlayerState> {
    let mut creatures = Vec::new();
    for pokemon in team.iter().take(3) {
        let species = db.species.get(pokemon.species_id.as_str())?;
        let creature = create_creature(
            species,
            CreateCreatureOptions {
                moves: Some(pokemon.moves.clone()),
                ..CreateCreatureOptions::default()
            },
            &db.learnsets,
            &db.moves,
        )
        .ok()?;
        creatures.push(creature);
    }
    Some(PlayerState {
        id: id.to_string(),
        name: id.to_string(),
        team: creatures,
        active_slot: 0,
        last_fainted_ability: None,
    })
}

fn settle_forced_switches(state: &BattleState) -> BattleState {
    let mut next = state.clone();
    for _ in 0..8 {
        let mut changed = false;
        let ids: Vec<String> = next.players.iter().map(|p| p.id.clone()).collect();
        for pid in ids {
            let Some(player) = next.players.iter().find(|p| p.id == pid) else {
                continue;
            };
            let Some(active) = player.team.get(player.active_slot) else {
                continue;
            };
            if active.hp > 0 && !active.statuses.iter().any(|s| s.id == "pending_switch") {
                continue;
            }
            if let Some(slot) = first_switch_slot(&next, pid.as_str()) {
                let mut rng = || 0.42;
                next = replace_fainted_pokemon(&next, pid.as_str(), slot, &mut rng);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    next
}

fn first_switch_slot(state: &BattleState, pid: &str) -> Option<usize> {
    let p = state.players.iter().find(|p| p.id == pid)?;
    p.team
        .iter()
        .enumerate()
        .find(|(slot, c)| *slot != p.active_slot && c.hp > 0)
        .map(|(slot, _)| slot)
}

fn winner_by_state(state: &BattleState) -> Option<&'static str> {
    let a = state.players.iter().find(|p| p.id == "a")?;
    let b = state.players.iter().find(|p| p.id == "b")?;
    let alive_a = a.team.iter().filter(|c| c.hp > 0).count();
    let alive_b = b.team.iter().filter(|c| c.hp > 0).count();
    if alive_a > 0 && alive_b == 0 {
        return Some("a");
    }
    if alive_b > 0 && alive_a == 0 {
        return Some("b");
    }
    let hp_a: i32 = a.team.iter().map(|c| c.hp.max(0)).sum();
    let hp_b: i32 = b.team.iter().map(|c| c.hp.max(0)).sum();
    if hp_a > hp_b {
        Some("a")
    } else if hp_b > hp_a {
        Some("b")
    } else {
        None
    }
}

// ──────────────────────────────────────────
// パラメーター突然変異
// ──────────────────────────────────────────

fn mutate_params(base: VegaParams, rng: &mut Lcg, scale: f32) -> VegaParams {
    VegaParams {
        alive: mutate(base.alive, 220.0, 520.0, scale, rng),
        hp: base.hp,
        hp_ratio: mutate(base.hp_ratio, 80.0, 320.0, scale, rng),
        active_hp: mutate(base.active_hp, 20.0, 150.0, scale, rng),
        outgoing: mutate(base.outgoing, 80.0, 340.0, scale, rng),
        incoming: mutate(base.incoming, 80.0, 340.0, scale, rng),
        ko_fast: mutate(base.ko_fast, 120.0, 420.0, scale, rng),
        ko_slow: mutate(base.ko_slow, 40.0, 240.0, scale, rng),
        risk_fast: mutate(base.risk_fast, 120.0, 460.0, scale, rng),
        risk_slow: mutate(base.risk_slow, 40.0, 260.0, scale, rng),
        speed: mutate(base.speed, 0.0, 90.0, scale, rng),
        bench: mutate(base.bench, 0.0, 160.0, scale, rng),
        stage: mutate(base.stage, 0.4, 1.8, scale, rng),
        status: mutate(base.status, 0.5, 1.8, scale, rng),
        switch_pressure: mutate(base.switch_pressure, 40.0, 260.0, scale, rng),
        switch_danger: mutate(base.switch_danger, 40.0, 240.0, scale, rng),
        switch_hp: mutate(base.switch_hp, 0.0, 120.0, scale, rng),
        action_damage: mutate(base.action_damage, 80.0, 360.0, scale, rng),
        action_priority_ko: mutate(base.action_priority_ko, 20.0, 240.0, scale, rng),
        action_accuracy: base.action_accuracy,
    }
}

fn mutate(value: f32, min: f32, max: f32, scale: f32, rng: &mut Lcg) -> f32 {
    let span = max - min;
    (value + (rng.f32() * 2.0 - 1.0) * span * scale).clamp(min, max)
}

// ──────────────────────────────────────────
// チーム読み込み
// ──────────────────────────────────────────

fn load_teams() -> Result<Vec<Vec<TeamPokemon>>, Box<dyn std::error::Error>> {
    let path = repo_root()?.join("frontend/public/ai_teams.json");
    let entries: Vec<TeamEntry> = serde_json::from_str(&fs::read_to_string(path)?)?;
    let teams: Vec<_> = entries
        .into_iter()
        .map(|e| e.team)
        .filter(|t| t.len() >= 3)
        .collect();
    if teams.is_empty() {
        return Err("no teams in ai_teams.json".into());
    }
    Ok(teams)
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = env::current_dir()?;
    loop {
        if path.join("frontend").exists() && path.join("engine-rust").exists() {
            return Ok(path);
        }
        if !path.pop() {
            return Err("repo root not found".into());
        }
    }
}
