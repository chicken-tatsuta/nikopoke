use engine_rust::data::moves::MoveDatabase;
use engine_rust::{
    apply_initial_switch_in_effects, create_battle_state, create_creature, get_best_move_minimax,
    CreateCreatureOptions, EVStats, LearnsetDatabase, PlayerState, SpeciesDatabase,
};

fn deck_pokemon(
    species_id: &str,
    ability: &str,
    moves: &[&str],
    evs: EVStats,
) -> engine_rust::CreatureState {
    let species_db = SpeciesDatabase::load_default().expect("species data");
    let learnsets = LearnsetDatabase::load_default().expect("learnsets");
    let move_db = MoveDatabase::load_default().expect("moves");
    let species = species_db.get(species_id).expect("species");
    create_creature(
        species,
        CreateCreatureOptions {
            moves: Some(moves.iter().map(|move_id| move_id.to_string()).collect()),
            ability: Some(ability.to_string()),
            evs: Some(evs),
            ..CreateCreatureOptions::default()
        },
        &learnsets,
        &move_db,
    )
    .expect("creature")
}

#[test]
fn buchii_sub_deck_allows_ai_move_selection() {
    let player_team = vec![
        deck_pokemon(
            "nisiki",
            "moody",
            &["protect", "substitute", "minimize", "freeze_dry"],
            EVStats {
                hp: 32,
                atk: 0,
                def: 0,
                spa: 0,
                spd: 2,
                spe: 32,
            },
        ),
        deck_pokemon(
            "michii",
            "unnerve",
            &["reflect", "light_screen", "toxic_spikes", "destiny_bond"],
            EVStats {
                hp: 0,
                atk: 0,
                def: 1,
                spa: 32,
                spd: 0,
                spe: 32,
            },
        ),
        deck_pokemon(
            "buchii",
            "fur_coat",
            &["quiver_dance", "moonblast", "recover", "disarming_voice"],
            EVStats {
                hp: 32,
                atk: 0,
                def: 32,
                spa: 0,
                spd: 1,
                spe: 0,
            },
        ),
    ];
    let ai_team = vec![
        deck_pokemon(
            "ayuma",
            "berserk",
            &["nasty_plot", "dark_pulse", "flash_cannon", "aura_sphere"],
            EVStats::default(),
        ),
        deck_pokemon(
            "ikkun",
            "thick_fat",
            &["accelerock", "substitute", "toxic", "protect"],
            EVStats::default(),
        ),
        deck_pokemon(
            "macchan",
            "simple",
            &[
                "close_combat",
                "bullet_punch",
                "meteor_mash",
                "dragon_dance",
            ],
            EVStats::default(),
        ),
    ];
    let state = create_battle_state(vec![
        PlayerState {
            id: "player".to_string(),
            name: "player".to_string(),
            team: player_team,
            active_slot: 0,
            last_fainted_ability: None,
        },
        PlayerState {
            id: "ai".to_string(),
            name: "ai".to_string(),
            team: ai_team,
            active_slot: 0,
            last_fainted_ability: None,
        },
    ]);
    let mut rng = || 0.0;
    let state = apply_initial_switch_in_effects(&state, &mut rng);

    let action = get_best_move_minimax(&state, "ai", 1);

    assert!(action.is_some());
}
