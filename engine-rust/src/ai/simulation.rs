use crate::core::battle::{BattleEngine, BattleOptions};
use crate::core::state::{Action, BattleState};
use crate::data::moves::MoveDatabase;
use crate::data::type_chart::TypeChart;

/// Reusable battle runtime for AI search.
///
/// A search can simulate hundreds of turns. Constructing the default battle
/// engine for every node reloads the move database, so the engine is owned by
/// the search and shared by every simulated turn instead.
pub(crate) struct SearchSimulator {
    engine: BattleEngine,
}

impl SearchSimulator {
    pub(crate) fn new(move_db: MoveDatabase) -> Self {
        Self {
            engine: BattleEngine::new(move_db, TypeChart::new()),
        }
    }

    pub(crate) fn from_move_db(move_db: &MoveDatabase) -> Self {
        Self::new(move_db.clone())
    }

    pub(crate) fn move_db(&self) -> &MoveDatabase {
        &self.engine.move_db
    }

    pub(crate) fn step(
        &self,
        state: &BattleState,
        actions: &[Action],
        rng: &mut dyn FnMut() -> f64,
    ) -> BattleState {
        let mut next = self.engine.step_battle(
            state,
            actions,
            rng,
            BattleOptions {
                record_history: false,
            },
        );
        strip_observation_data(&mut next);
        next
    }
}

pub(crate) fn prepare_search_state(state: &BattleState) -> BattleState {
    let mut search_state = state.clone();
    strip_observation_data(&mut search_state);
    search_state
}

fn strip_observation_data(state: &mut BattleState) {
    state.log.clear();
    state.history = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::{create_battle_state, BattleHistory, BattleTurn};

    #[test]
    fn prepares_search_state_without_mutating_visible_battle_data() {
        let mut state = create_battle_state(Vec::new());
        state.log.push("visible log".to_string());
        state.history = Some(BattleHistory {
            turns: vec![BattleTurn {
                turn: 1,
                actions: Vec::new(),
                log: vec!["visible history".to_string()],
                rng: Vec::new(),
            }],
        });

        let search_state = prepare_search_state(&state);

        assert!(search_state.log.is_empty());
        assert!(search_state.history.is_none());
        assert_eq!(state.log, vec!["visible log"]);
        assert_eq!(
            state.history.as_ref().map(|history| history.turns.len()),
            Some(1)
        );
    }

    #[test]
    fn simulated_turn_does_not_retain_logs_or_history() {
        let simulator = SearchSimulator::new(MoveDatabase::minimal());
        let state = create_battle_state(Vec::new());
        let mut rng = || 0.42;

        let next = simulator.step(&state, &[], &mut rng);

        assert_eq!(next.turn, 1);
        assert!(next.log.is_empty());
        assert!(next.history.is_none());
    }
}
