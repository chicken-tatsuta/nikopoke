pub mod ai;
pub mod core;
pub mod data;

#[cfg(not(target_arch = "wasm32"))]
pub mod tools;

pub use ai::{
    choose_highest_power, get_best_move_mcts, get_best_move_minimax, get_best_move_vega,
    run_auto_battle,
};
pub use core::{
    battle::{
        apply_initial_switch_in_effects, is_battle_over, step_battle, BattleEngine, BattleOptions,
    },
    factory::{calc_stat, create_creature, CreateCreatureOptions},
    replay::replay_battle,
    state::{
        create_battle_state, Action, BattleHistory, BattleState, BattleTurn, CreatureState,
        EVStats, FieldState, PlayerState,
    },
};
pub use data::{
    learnsets::LearnsetDatabase,
    species::{BaseStats, SpeciesData, SpeciesDatabase},
};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
