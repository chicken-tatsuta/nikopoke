use engine_rust::ai::vega::{
    get_best_move_vega_with_options_and_db_ref_and_stats, VegaStats, DEFAULT_PARAMS,
};
use engine_rust::core::factory::{create_creature, CreateCreatureOptions};
use engine_rust::core::state::{create_battle_state, BattleState, PlayerState};
use engine_rust::data::learnsets::LearnsetDatabase;
use engine_rust::data::moves::MoveDatabase;
use engine_rust::data::species::SpeciesDatabase;
use serde::Serialize;
use std::env;

#[derive(Clone, Copy)]
struct BenchOptions {
    depth: usize,
    branch_limit: usize,
    iterations: usize,
}

#[derive(Serialize)]
struct BenchReport {
    depth: usize,
    branch_limit: usize,
    iterations: usize,
    avg_ms: f64,
    last_action: Option<String>,
    stats: VegaStats,
}

struct Databases {
    species: SpeciesDatabase,
    learnsets: LearnsetDatabase,
    moves: MoveDatabase,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_options();
    let db = Databases {
        species: SpeciesDatabase::load_default()?,
        learnsets: LearnsetDatabase::load_default()?,
        moves: MoveDatabase::load_default()?,
    };
    let state = create_fixed_state(&db)?;

    let mut stats = VegaStats::default();
    let mut last_action = None;
    for _ in 0..options.iterations {
        last_action = get_best_move_vega_with_options_and_db_ref_and_stats(
            &state,
            "a",
            options.depth,
            DEFAULT_PARAMS,
            options.branch_limit,
            &db.moves,
            &mut stats,
        );
    }

    let avg_ms = if stats.searches > 0 {
        stats.elapsed_ns as f64 / stats.searches as f64 / 1_000_000.0
    } else {
        0.0
    };
    let report = BenchReport {
        depth: options.depth,
        branch_limit: options.branch_limit,
        iterations: options.iterations,
        avg_ms,
        last_action: last_action.map(|action| format!("{:?}", action)),
        stats,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn parse_options() -> BenchOptions {
    let mut options = BenchOptions {
        depth: 2,
        branch_limit: 4,
        iterations: 10,
    };
    for arg in env::args().skip(1) {
        let Some((key, value)) = arg.strip_prefix("--").and_then(|arg| arg.split_once('=')) else {
            continue;
        };
        match key {
            "depth" => options.depth = value.parse().unwrap_or(options.depth),
            "branch-limit" => options.branch_limit = value.parse().unwrap_or(options.branch_limit),
            "iterations" => options.iterations = value.parse().unwrap_or(options.iterations),
            _ => {}
        }
    }
    options
}

fn create_fixed_state(db: &Databases) -> Result<BattleState, Box<dyn std::error::Error>> {
    let player_a = create_player(
        "a",
        &[
            (
                "ayuma",
                &["torment", "fake_tears", "memento", "parting_shot"],
            ),
            ("eiraku", &["taunt", "nasty_plot", "stone_edge", "agility"]),
            (
                "futo",
                &["nasty_plot", "amnesia", "low_sweep", "petal_dance"],
            ),
        ],
        db,
    )?;
    let player_b = create_player(
        "b",
        &[
            (
                "haruta",
                &["nasty_plot", "lash_out", "rollout", "healing_wish"],
            ),
            (
                "ikkun",
                &["torment", "nasty_plot", "ruination", "stealth_rock"],
            ),
            (
                "machida",
                &["nasty_plot", "agility", "rest", "petal_blizzard"],
            ),
        ],
        db,
    )?;
    Ok(create_battle_state(vec![player_a, player_b]))
}

fn create_player(
    id: &str,
    team: &[(&str, &[&str])],
    db: &Databases,
) -> Result<PlayerState, Box<dyn std::error::Error>> {
    let mut creatures = Vec::new();
    for (species_id, moves) in team {
        let species = db
            .species
            .get(species_id)
            .ok_or_else(|| format!("unknown species: {}", species_id))?;
        let creature = create_creature(
            species,
            CreateCreatureOptions {
                moves: Some(moves.iter().map(|move_id| move_id.to_string()).collect()),
                ..CreateCreatureOptions::default()
            },
            &db.learnsets,
            &db.moves,
        )?;
        creatures.push(creature);
    }
    Ok(PlayerState {
        id: id.to_string(),
        name: id.to_string(),
        team: creatures,
        active_slot: 0,
        last_fainted_ability: None,
    })
}
