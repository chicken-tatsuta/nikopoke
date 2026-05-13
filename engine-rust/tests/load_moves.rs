use engine_rust::data::moves::MoveDatabase;
use std::path::Path;

#[test]
fn load_full_move_database() {
    let path = Path::new("data/moves.yaml");
    let db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");
    assert!(!db.as_map().is_empty(), "move database should not be empty");
    assert!(
        db.get("tackle").is_some(),
        "expected tackle in full database"
    );
}

#[test]
fn shell_smash_and_withdraw_are_not_mixed_up() {
    let path = Path::new("data/moves.yaml");
    let db = MoveDatabase::load_from_yaml_file(path).expect("load moves.yaml");

    let shell_smash = db.get("shell_smash").expect("shell_smash exists");
    assert_eq!(shell_smash.name.as_deref(), Some("からをやぶる"));
    assert_eq!(shell_smash.pp, Some(15));
    assert_eq!(shell_smash.move_type.as_deref(), Some("normal"));
    let shell_smash_stages = shell_smash.steps[0]
        .data
        .get("stages")
        .and_then(|value| value.as_object())
        .expect("shell_smash stage changes");
    assert_eq!(
        shell_smash_stages.get("atk").and_then(|v| v.as_i64()),
        Some(2)
    );
    assert_eq!(
        shell_smash_stages.get("spa").and_then(|v| v.as_i64()),
        Some(2)
    );
    assert_eq!(
        shell_smash_stages.get("spe").and_then(|v| v.as_i64()),
        Some(2)
    );
    assert_eq!(
        shell_smash_stages.get("def").and_then(|v| v.as_i64()),
        Some(-1)
    );
    assert_eq!(
        shell_smash_stages.get("spd").and_then(|v| v.as_i64()),
        Some(-1)
    );

    let withdraw = db.get("withdraw").expect("withdraw exists");
    assert_eq!(withdraw.name.as_deref(), Some("からにこもる"));
    let withdraw_stages = withdraw.steps[0]
        .data
        .get("stages")
        .and_then(|value| value.as_object())
        .expect("withdraw stage changes");
    assert_eq!(withdraw_stages.get("def").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(withdraw_stages.len(), 1);
}
