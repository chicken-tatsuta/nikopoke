use crate::core::battle::{step_battle, BattleOptions};
use crate::core::state::{BattleHistory, BattleState};

pub fn replay_battle(initial_state: &BattleState, history: &BattleHistory) -> BattleState {
    let mut next = initial_state.clone();
    for turn in &history.turns {
        let mut idx = 0usize;
        let mut rng = || {
            let v = turn.rng.get(idx).copied().unwrap_or(0.5);
            idx += 1;
            v
        };
        next = step_battle(
            &next,
            &turn.actions,
            &mut rng,
            BattleOptions {
                record_history: false,
            },
        );
    }
    next
}
