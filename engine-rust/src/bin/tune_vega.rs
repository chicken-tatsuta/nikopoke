use engine_rust::ai::minimax::get_best_move_minimax;
use engine_rust::ai::vega::{
    get_best_move_vega_with_options_and_db, VegaParams, DEFAULT_PARAMS,
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
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
struct TeamEntry {
    team: Vec<TeamPokemon>,
}

#[derive(Clone, Debug, Deserialize)]
struct TeamPokemon {
    species_id: String,
    moves: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct EvalResult {
    wins: usize,
    losses: usize,
    draws: usize,
    score: f32,
}

#[derive(Clone, Debug)]
struct Options {
    iterations: usize,
    games: usize,
    eval_games: usize,
    depth: usize,
    baseline_depth: usize,
    branch_limit: usize,
    baseline_policy: String,
    max_turns: usize,
    seed: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            iterations: 80,
            games: 24,
            eval_games: 80,
            depth: 2,
            baseline_depth: 1,
            branch_limit: 2,
            baseline_policy: "default-vega".to_string(),
            max_turns: 40,
            seed: 20260509,
        }
    }
}

struct Databases {
    species: SpeciesDatabase,
    learnsets: LearnsetDatabase,
    moves: MoveDatabase,
}

const OLD_DEFAULT_PARAMS: VegaParams = VegaParams {
    alive: 360.0,
    hp: 1.0,
    hp_ratio: 190.0,
    active_hp: 75.0,
    outgoing: 180.0,
    incoming: 170.0,
    ko_fast: 230.0,
    ko_slow: 120.0,
    risk_fast: 240.0,
    risk_slow: 130.0,
    speed: 28.0,
    bench: 65.0,
    stage: 1.0,
    status: 1.0,
    switch_pressure: 140.0,
    switch_danger: 110.0,
    switch_hp: 40.0,
    action_damage: 220.0,
    action_priority_ko: 120.0,
    action_accuracy: 20.0,
};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_options();
    let db = Databases {
        species: SpeciesDatabase::load_default()?,
        learnsets: LearnsetDatabase::load_default()?,
        moves: MoveDatabase::load_default()?,
    };
    let teams = load_teams()?;
    let mut rng = Lcg::new(options.seed);

    let train_seed = options.seed + 11;
    let holdout_seed = options.seed + 100_011;
    let mut best_params = DEFAULT_PARAMS;
    let mut best_train = evaluate_params(best_params, &teams, &db, &options, options.eval_games, train_seed);
    let mut best_holdout =
        evaluate_params(best_params, &teams, &db, &options, options.eval_games, holdout_seed);
    let mut best_metric = best_train.score * 0.7 + best_holdout.score * 0.3;
    println!(
        "initial metric={:.3} train={} holdout={}",
        best_metric,
        format_result(best_train),
        format_result(best_holdout)
    );

    for iter in 1..=options.iterations {
        let scale = 0.22 * (1.0 - iter as f32 / options.iterations.max(1) as f32) + 0.035;
        let candidate = mutate_params(best_params, &mut rng, scale);
        let quick = evaluate_params(candidate, &teams, &db, &options, options.games, train_seed);
        if quick.score + 0.02 < best_train.score && quick.wins <= quick.losses {
            println!("iter={iter:03} quick {}", format_result(quick));
            continue;
        }

        let train = evaluate_params(candidate, &teams, &db, &options, options.eval_games, train_seed);
        let holdout = evaluate_params(candidate, &teams, &db, &options, options.eval_games, holdout_seed);
        let metric = train.score * 0.7 + holdout.score * 0.3;
        if metric > best_metric {
            best_metric = metric;
            best_params = candidate;
            best_train = train;
            best_holdout = holdout;
            println!(
                "iter={iter:03} NEW metric={:.3} train={} holdout={}",
                best_metric,
                format_result(best_train),
                format_result(best_holdout)
            );
        } else {
            println!(
                "iter={iter:03} metric={:.3} train={} holdout={}",
                metric,
                format_result(train),
                format_result(holdout)
            );
        }
    }

    let final_result = evaluate_params(
        best_params,
        &teams,
        &db,
        &options,
        options.eval_games,
        options.seed + 900_001,
    );
    println!("\nBEST_PARAMS={best_params:#?}");
    println!("FINAL {}", format_result(final_result));
    Ok(())
}

fn parse_options() -> Options {
    let mut options = Options::default();
    for arg in env::args().skip(1) {
        let Some((key, value)) = arg.strip_prefix("--").and_then(|arg| arg.split_once('=')) else {
            continue;
        };
        match key {
            "iterations" => options.iterations = value.parse().unwrap_or(options.iterations),
            "games" => options.games = value.parse().unwrap_or(options.games),
            "eval-games" => options.eval_games = value.parse().unwrap_or(options.eval_games),
            "depth" => options.depth = value.parse().unwrap_or(options.depth),
            "baseline-depth" => {
                options.baseline_depth = value.parse().unwrap_or(options.baseline_depth)
            }
            "branch-limit" => options.branch_limit = value.parse().unwrap_or(options.branch_limit),
            "baseline-policy" => options.baseline_policy = value.to_string(),
            "max-turns" => options.max_turns = value.parse().unwrap_or(options.max_turns),
            "seed" => options.seed = value.parse().unwrap_or(options.seed),
            _ => {}
        }
    }
    options
}

fn load_teams() -> Result<Vec<Vec<TeamPokemon>>, Box<dyn std::error::Error>> {
    let path = repo_root()?.join("frontend/public/ai_teams.json");
    let entries: Vec<TeamEntry> = serde_json::from_str(&fs::read_to_string(path)?)?;
    let teams = entries
        .into_iter()
        .map(|entry| entry.team)
        .filter(|team| team.len() >= 3)
        .collect::<Vec<_>>();
    if teams.is_empty() {
        return Err("no teams found in frontend/public/ai_teams.json".into());
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

fn evaluate_params(
    params: VegaParams,
    teams: &[Vec<TeamPokemon>],
    db: &Databases,
    options: &Options,
    games: usize,
    seed: u64,
) -> EvalResult {
    // LCGで決定論的にチームペアを生成してから並列実行
    let setups: Vec<(usize, usize, bool)> = {
        let mut rng = Lcg::new(seed);
        (0..games)
            .map(|game| (rng.usize(teams.len()), rng.usize(teams.len()), game % 2 == 0))
            .collect()
    };

    let (wins, losses, draws) = setups
        .par_iter()
        .map(|&(a_idx, b_idx, vega_as_a)| {
            let team_a = &teams[a_idx];
            let team_b = &teams[b_idx];
            let winner = play_game(
                if vega_as_a { team_a } else { team_b },
                if vega_as_a { team_b } else { team_a },
                vega_as_a,
                params,
                db,
                options,
            );
            match winner {
                Some("a") if vega_as_a => (1usize, 0usize, 0usize),
                Some("b") if !vega_as_a => (1, 0, 0),
                Some(_) => (0, 1, 0),
                None => (0, 0, 1),
            }
        })
        .reduce(|| (0, 0, 0), |(w1, l1, d1), (w2, l2, d2)| (w1 + w2, l1 + l2, d1 + d2));

    EvalResult {
        wins,
        losses,
        draws,
        score: (wins as f32 + draws as f32 * 0.5) / games.max(1) as f32,
    }
}

fn play_game(
    team_a: &[TeamPokemon],
    team_b: &[TeamPokemon],
    vega_as_a: bool,
    params: VegaParams,
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
        let action_a = if vega_as_a {
            get_best_move_vega_with_options_and_db(
                &state,
                "a",
                options.depth,
                params,
                options.branch_limit,
                db.moves.clone(),
            )
        } else {
            select_baseline_action(&state, "a", options, &db.moves)
        };
        let action_b = if vega_as_a {
            select_baseline_action(&state, "b", options, &db.moves)
        } else {
            get_best_move_vega_with_options_and_db(
                &state,
                "b",
                options.depth,
                params,
                options.branch_limit,
                db.moves.clone(),
            )
        };
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

fn select_baseline_action(
    state: &BattleState,
    player_id: &str,
    options: &Options,
    move_db: &MoveDatabase,
) -> Option<engine_rust::core::state::Action> {
    if options.baseline_policy == "minimax" {
        return get_best_move_minimax(state, player_id, options.baseline_depth);
    }

    let baseline_params = if options.baseline_policy == "old-vega" {
        OLD_DEFAULT_PARAMS
    } else {
        DEFAULT_PARAMS
    };

    get_best_move_vega_with_options_and_db(
        state,
        player_id,
        1,
        baseline_params,
        options.branch_limit,
        move_db.clone(),
    )
}

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
        let player_ids = next
            .players
            .iter()
            .map(|player| player.id.clone())
            .collect::<Vec<_>>();
        for player_id in player_ids {
            let Some(player) = next.players.iter().find(|player| player.id == player_id) else {
                continue;
            };
            let Some(active) = player.team.get(player.active_slot) else {
                continue;
            };
            if active.hp > 0 && !active.statuses.iter().any(|status| status.id == "pending_switch") {
                continue;
            }
            if let Some(slot) = first_switch_slot(&next, player_id.as_str()) {
                let mut rng = || 0.42;
                next = replace_fainted_pokemon(&next, player_id.as_str(), slot, &mut rng);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    next
}

fn first_switch_slot(state: &BattleState, player_id: &str) -> Option<usize> {
    let player = state.players.iter().find(|player| player.id == player_id)?;
    player
        .team
        .iter()
        .enumerate()
        .find(|(slot, creature)| *slot != player.active_slot && creature.hp > 0)
        .map(|(slot, _)| slot)
}

fn winner_by_state(state: &BattleState) -> Option<&'static str> {
    let player_a = state.players.iter().find(|player| player.id == "a")?;
    let player_b = state.players.iter().find(|player| player.id == "b")?;
    let alive_a = player_a.team.iter().filter(|creature| creature.hp > 0).count();
    let alive_b = player_b.team.iter().filter(|creature| creature.hp > 0).count();
    if alive_a > 0 && alive_b == 0 {
        return Some("a");
    }
    if alive_b > 0 && alive_a == 0 {
        return Some("b");
    }
    let hp_a: i32 = player_a
        .team
        .iter()
        .map(|creature| creature.hp.max(0))
        .sum();
    let hp_b: i32 = player_b
        .team
        .iter()
        .map(|creature| creature.hp.max(0))
        .sum();
    if hp_a > hp_b {
        Some("a")
    } else if hp_b > hp_a {
        Some("b")
    } else {
        None
    }
}

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

fn format_result(result: EvalResult) -> String {
    format!(
        "score={:.3} W-L-D={}-{}-{}",
        result.score, result.wins, result.losses, result.draws
    )
}
