//! Generation 7+ integer stages. Fixed-point modifiers round half down.
pub fn modify(value: i32, multiplier: f32) -> i32 {
    let modifier = (multiplier as f64 * 4096.0).round() as i64;
    ((i64::from(value) * modifier + 2047) / 4096).clamp(0, i32::MAX as i64) as i32
}
pub fn base_damage(level: u32, power: i32, attack: i32, defense: i32) -> i32 {
    let level_term = 2 * i64::from(level) / 5 + 2;
    ((level_term * i64::from(power) * i64::from(attack) / i64::from(defense.max(1))) / 50 + 2)
        .clamp(0, i32::MAX as i64) as i32
}
pub fn apply_roll(value: i32, roll: i32) -> i32 {
    ((i64::from(value) * i64::from(roll)) / 100) as i32
}
