//! Debug CLI for Pokémon Battle Engine
//!
//! A comprehensive debugging tool with full visualization of stats, damage calculations,
//! abilities, and battle mechanics.

use engine_rust::core::battle::{is_battle_over, BattleEngine, BattleOptions};
use engine_rust::core::factory::{calc_stat, create_creature, CreateCreatureOptions};
use engine_rust::core::state::{
    Action, ActionType, BattleState, CreatureState, FieldState, PlayerState,
};
use engine_rust::data::learnsets::LearnsetDatabase;
use engine_rust::data::moves::MoveDatabase;
use engine_rust::data::species::SpeciesDatabase;
use engine_rust::data::type_chart::TypeChart;
use inquire::{Select, Text};
use std::collections::HashMap;
use wana_kana::ConvertJapanese;

fn main() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║       🔧 ポケモンバトルエンジン デバッグCLI 🔧          ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Load databases
    let species_db = SpeciesDatabase::load_default().expect("Failed to load species.json");
    let move_db = MoveDatabase::load_default().expect("Failed to load moves.json");
    let learnset_db = LearnsetDatabase::load_default().expect("Failed to load learnsets.json");
    let type_chart = TypeChart::new();
    let engine = BattleEngine::new(move_db.clone(), type_chart);

    loop {
        let options = vec![
            "🎮 バトルシミュレーション開始",
            "📊 ポケモン情報を確認",
            "⚔️  技情報を確認",
            "🧮 ダメージ計算機",
            "🚪 終了",
        ];

        let selection = Select::new("メインメニュー", options)
            .prompt()
            .unwrap_or("🚪 終了");

        match selection {
            "🎮 バトルシミュレーション開始" => {
                run_battle_simulation(&species_db, &move_db, &learnset_db, &engine);
            }
            "📊 ポケモン情報を確認" => {
                inspect_pokemon(&species_db, &move_db, &learnset_db);
            }
            "⚔️  技情報を確認" => {
                inspect_move(&move_db);
            }
            "🧮 ダメージ計算機" => {
                damage_calculator(&species_db, &move_db, &learnset_db, &engine);
            }
            "🚪 終了" | _ => {
                println!("👋 デバッグCLIを終了します");
                break;
            }
        }
    }
}

// ============================================================================
// Pokemon Info Display
// ============================================================================

fn print_species_info(
    species_id: &str,
    species_db: &SpeciesDatabase,
    move_db: &MoveDatabase,
    learnset_db: &LearnsetDatabase,
) {
    let Some(species) = species_db.get(species_id) else {
        println!("❌ 種族 '{}' が見つかりません", species_id);
        return;
    };

    println!("\n┌─────────────────────────────────────────────────────────┐");
    println!(
        "│ 📖 {} (ID: {})                           ",
        species.name, species.id
    );
    println!("├─────────────────────────────────────────────────────────┤");

    // Types
    let types_str = species
        .types
        .iter()
        .map(|t| format_type(t))
        .collect::<Vec<_>>()
        .join(" ");
    println!(
        "│ タイプ: {}                                          ",
        types_str
    );

    // Base Stats
    let bs = &species.base_stats;
    let total = bs.hp + bs.atk + bs.def + bs.spa + bs.spd + bs.spe;
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│ 種族値                                                  │");
    println!(
        "│   HP:     {:>3} {}                          ",
        bs.hp,
        stat_bar(bs.hp, 255)
    );
    println!(
        "│   攻撃:   {:>3} {}                          ",
        bs.atk,
        stat_bar(bs.atk, 180)
    );
    println!(
        "│   防御:   {:>3} {}                          ",
        bs.def,
        stat_bar(bs.def, 180)
    );
    println!(
        "│   特攻:   {:>3} {}                          ",
        bs.spa,
        stat_bar(bs.spa, 180)
    );
    println!(
        "│   特防:   {:>3} {}                          ",
        bs.spd,
        stat_bar(bs.spd, 180)
    );
    println!(
        "│   素早さ: {:>3} {}                          ",
        bs.spe,
        stat_bar(bs.spe, 180)
    );
    println!(
        "│   合計:   {:>3}                                        ",
        total
    );

    // Abilities
    if !species.abilities.is_empty() {
        println!("├─────────────────────────────────────────────────────────┤");
        println!(
            "│ 特性: {}                                      ",
            species.abilities.join(", ")
        );
    }

    // Actual stats at level 50
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│ Lv.50 実数値 (V個体, 努力値0)                          │");
    let hp = calc_stat(bs.hp, true, 50, 31, 0);
    let atk = calc_stat(bs.atk, false, 50, 31, 0);
    let def = calc_stat(bs.def, false, 50, 31, 0);
    let spa = calc_stat(bs.spa, false, 50, 31, 0);
    let spd = calc_stat(bs.spd, false, 50, 31, 0);
    let spe = calc_stat(bs.spe, false, 50, 31, 0);
    println!(
        "│   HP: {} / 攻: {} / 防: {} / 特攻: {} / 特防: {} / 素早: {} ",
        hp, atk, def, spa, spd, spe
    );

    // Learnable moves
    if let Some(moves) = learnset_db.get(species_id) {
        println!("├─────────────────────────────────────────────────────────┤");
        println!(
            "│ 覚える技 ({} 種類)                                    ",
            moves.len()
        );
        for move_id in moves.iter().take(20) {
            if let Some(m) = move_db.get(move_id) {
                let name = m.name.as_deref().unwrap_or(move_id);
                let mtype = m.move_type.as_deref().unwrap_or("???");
                let cat = m.category.as_deref().unwrap_or("???");
                let power = m.power.map(|p| p.to_string()).unwrap_or("-".to_string());
                println!(
                    "│   {} {} {} 威力:{}          ",
                    format!("{:<12}", name),
                    format_type(mtype),
                    format_category(cat),
                    power
                );
            } else {
                println!("│   {} (未定義)                           ", move_id);
            }
        }
        if moves.len() > 20 {
            println!(
                "│   ... 他 {} 種類                                      ",
                moves.len() - 20
            );
        }
    }

    println!("└─────────────────────────────────────────────────────────┘\n");
}

fn stat_bar(value: i32, max: i32) -> String {
    let percentage = (value as f32 / max as f32).min(1.0);
    let bar_len = (percentage * 20.0) as usize;
    let bar = "█".repeat(bar_len);
    let empty = "░".repeat(20 - bar_len);
    format!("[{}{}]", bar, empty)
}

fn format_type(type_name: &str) -> String {
    match type_name.to_lowercase().as_str() {
        "normal" => "⚪ノーマル",
        "fire" => "🔥ほのお",
        "water" => "💧みず",
        "electric" => "⚡でんき",
        "grass" => "🌿くさ",
        "ice" => "❄️こおり",
        "fighting" => "🥊かくとう",
        "poison" => "☠️どく",
        "ground" => "🌍じめん",
        "flying" => "🪶ひこう",
        "psychic" => "🔮エスパー",
        "bug" => "🐛むし",
        "rock" => "🪨いわ",
        "ghost" => "👻ゴースト",
        "dragon" => "🐉ドラゴン",
        "dark" => "🌙あく",
        "steel" => "⚙️はがね",
        "fairy" => "🧚フェアリー",
        _ => type_name,
    }
    .to_string()
}

fn format_category(cat: &str) -> String {
    match cat.to_lowercase().as_str() {
        "physical" => "💪物理",
        "special" => "✨特殊",
        "status" => "📊変化",
        _ => cat,
    }
    .to_string()
}

// ============================================================================
// Interactive Pokemon Selection
// ============================================================================

fn select_pokemon(
    species_db: &SpeciesDatabase,
    move_db: &MoveDatabase,
    learnset_db: &LearnsetDatabase,
    prompt: &str,
) -> Option<CreatureState> {
    // Get species list
    let species_list: Vec<_> = species_db.as_map().values().collect();
    if species_list.is_empty() {
        println!("❌ 種族データが空です");
        return None;
    }

    // Create selection options with romaji for searchability
    let options: Vec<String> = species_list
        .iter()
        .map(|s| {
            let romaji = s.name.to_romaji();
            format!("{} ({}) | {}", s.name, s.id, romaji)
        })
        .collect();

    println!("\n📝 ひらがな/カタカナで検索可能 (例: 'ぴかちゅう' → 'pikachu')");

    let selection = Select::new(prompt, options.clone())
        .with_page_size(15)
        .prompt()
        .ok()?;

    // Find selected species
    let idx = options.iter().position(|o| o == &selection)?;
    let species = species_list[idx];

    // Show species info
    print_species_info(&species.id, species_db, move_db, learnset_db);

    // Select ability
    let ability = if species.abilities.is_empty() {
        None
    } else {
        let ability_options: Vec<&str> = species.abilities.iter().map(|s| s.as_str()).collect();
        let selected = Select::new("特性を選択", ability_options).prompt().ok();
        selected.map(|s| s.to_string())
    };

    // Select moves (one by one with Enter key)
    let moves = if let Some(learnable) = learnset_db.get(&species.id) {
        let move_options: Vec<String> = learnable
            .iter()
            .filter_map(|id| {
                let m = move_db.get(id)?;
                let name = m.name.as_deref().unwrap_or(id);
                let mtype = m.move_type.as_deref().unwrap_or("???");
                let power = m.power.map(|p| p.to_string()).unwrap_or("-".to_string());
                let romaji = name.to_romaji();
                Some(format!(
                    "{} | {} | 威力:{} | {} | ID:{}",
                    name, mtype, power, romaji, id
                ))
            })
            .collect();

        let move_ids: Vec<String> = learnable
            .iter()
            .filter(|id| move_db.get(id).is_some())
            .cloned()
            .collect();

        if move_options.is_empty() {
            Vec::new()
        } else {
            // 1つずつ選択（最大4つまで）
            let mut selected_moves = Vec::new();

            for i in 1..=4 {
                if selected_moves.len() >= 4 {
                    break;
                }

                // 既に選択した技を除外
                let available_options: Vec<String> = move_options
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| !selected_moves.contains(&move_ids[*idx]))
                    .map(|(_, opt)| opt.clone())
                    .collect();

                if available_options.is_empty() {
                    break;
                }

                // 「選択完了」オプションを追加
                let mut selection_options = available_options.clone();
                if i > 1 {
                    selection_options.push("✅ 選択完了（これ以上選ばない）".to_string());
                }

                let prompt = format!("技を選択 [{}/4] (Enterで選択):", i);

                let ans = Select::new(&prompt, selection_options)
                    .with_page_size(15)
                    .prompt();

                match ans {
                    Ok(choice) => {
                        if choice == "✅ 選択完了（これ以上選ばない）" {
                            break;
                        }

                        // 選択された技のIDを取得
                        if let Some(move_id) = choice.split(" | ID:").last() {
                            selected_moves.push(move_id.to_string());
                        }
                    }
                    Err(_) => {
                        println!("選択がキャンセルされました。");
                        if selected_moves.is_empty() {
                            println!("技なしで作成します。");
                        }
                        break;
                    }
                }
            }

            selected_moves
        }
    } else {
        Vec::new()
    };

    // Select item
    let item = Text::new("持ち物 (空欄でなし)")
        .prompt()
        .ok()
        .filter(|s| !s.is_empty());

    // Create creature
    let options = CreateCreatureOptions {
        moves: Some(moves),
        ability,
        name: None,
        level: Some(50),
        item,
        evs: None,
    };

    match create_creature(species, options, learnset_db, move_db) {
        Ok(creature) => {
            print_creature_details(&creature, move_db);
            Some(creature)
        }
        Err(e) => {
            println!("❌ ポケモン作成エラー: {}", e);
            None
        }
    }
}

fn print_creature_details(creature: &CreatureState, move_db: &MoveDatabase) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!(
        "║  {} (ID: {})                              ",
        creature.name, creature.id
    );
    println!("╠═══════════════════════════════════════════════════════════╣");

    // Types
    let types_str = creature
        .types
        .iter()
        .map(|t| format_type(t))
        .collect::<Vec<_>>()
        .join(" ");
    println!(
        "║  タイプ: {}                                           ",
        types_str
    );

    // Ability & Item
    if let Some(ref ability) = creature.ability {
        println!(
            "║  特性: {}                                              ",
            ability
        );
    }
    if let Some(ref item) = creature.item {
        println!(
            "║  持ち物: {}                                            ",
            item
        );
    }

    // Stats
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!(
        "║  実数値 (Lv.{})                                          ",
        creature.level
    );
    println!(
        "║    HP:     {}/{}                                 ",
        creature.hp, creature.max_hp
    );
    println!(
        "║    攻撃:   {}                                          ",
        creature.attack
    );
    println!(
        "║    防御:   {}                                          ",
        creature.defense
    );
    println!(
        "║    特攻:   {}                                          ",
        creature.sp_attack
    );
    println!(
        "║    特防:   {}                                          ",
        creature.sp_defense
    );
    println!(
        "║    素早さ: {}                                          ",
        creature.speed
    );

    // Moves
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  技                                                       ║");
    for move_id in &creature.moves {
        if let Some(m) = move_db.get(move_id) {
            let name = m.name.as_deref().unwrap_or(move_id);
            let mtype = m.move_type.as_deref().unwrap_or("???");
            let cat = m.category.as_deref().unwrap_or("???");
            let power = m.power.map(|p| p.to_string()).unwrap_or("-".to_string());
            let pp = m.pp.unwrap_or(0);
            println!(
                "║    {} | {} | {} | 威力:{} | PP:{}  ",
                name,
                format_type(mtype),
                format_category(cat),
                power,
                pp
            );
        }
    }

    println!("╚═══════════════════════════════════════════════════════════╝\n");
}

// ============================================================================
// Battle State Visualization
// ============================================================================

fn print_battle_state(state: &BattleState, move_db: &MoveDatabase) {
    println!(
        "\n┏━━━━━━━━━━━━━━━━━━━━━━━━━━ ターン {} ━━━━━━━━━━━━━━━━━━━━━━━━━━┓",
        state.turn
    );

    for (i, player) in state.players.iter().enumerate() {
        let active = &player.team[player.active_slot];
        let hp_bar = hp_bar_string(active.hp, active.max_hp);
        let hp_pct = (active.hp as f32 / active.max_hp as f32 * 100.0) as i32;

        println!("┃                                                              ┃");
        let side_label = if i == 0 {
            "【自分】"
        } else {
            "【相手】"
        };
        println!(
            "┃  {} {}                                   ",
            side_label, active.name
        );

        // Types
        let types_str = active
            .types
            .iter()
            .map(|t| format_type(t))
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            "┃    タイプ: {}                                          ",
            types_str
        );

        // Ability & Item
        if let Some(ref ability) = active.ability {
            println!(
                "┃    特性: {}                                             ",
                ability
            );
        }
        if let Some(ref item) = active.item {
            println!(
                "┃    持ち物: {}                                           ",
                item
            );
        }

        // HP
        println!(
            "┃    HP: {} ({}%)                           ",
            hp_bar, hp_pct
        );
        println!(
            "┃        {}/{}                                        ",
            active.hp, active.max_hp
        );

        // Stats
        println!(
            "┃    実数値: 攻:{} 防:{} 特攻:{} 特防:{} 素早:{}        ",
            active.attack, active.defense, active.sp_attack, active.sp_defense, active.speed
        );

        // Stage changes
        let stages = &active.stages;
        let mut stage_strs = Vec::new();
        if stages.atk != 0 {
            stage_strs.push(format!("攻撃{:+}", stages.atk));
        }
        if stages.def != 0 {
            stage_strs.push(format!("防御{:+}", stages.def));
        }
        if stages.spa != 0 {
            stage_strs.push(format!("特攻{:+}", stages.spa));
        }
        if stages.spd != 0 {
            stage_strs.push(format!("特防{:+}", stages.spd));
        }
        if stages.spe != 0 {
            stage_strs.push(format!("素早{:+}", stages.spe));
        }
        if stages.accuracy != 0 {
            stage_strs.push(format!("命中{:+}", stages.accuracy));
        }
        if stages.evasion != 0 {
            stage_strs.push(format!("回避{:+}", stages.evasion));
        }
        if stages.crit != 0 {
            stage_strs.push(format!("急所{:+}", stages.crit));
        }

        if !stage_strs.is_empty() {
            println!(
                "┃    ランク変化: {}                              ",
                stage_strs.join(" ")
            );
        }

        // Status conditions
        if !active.statuses.is_empty() {
            let status_strs: Vec<String> = active
                .statuses
                .iter()
                .map(|s| format_status(&s.id, s.remaining_turns))
                .collect();
            println!(
                "┃    状態: {}                                     ",
                status_strs.join(" ")
            );
        }

        // Moves with PP
        println!("┃    技:");
        for move_id in &active.moves {
            if let Some(m) = move_db.get(move_id) {
                let name = m.name.as_deref().unwrap_or(move_id);
                let max_pp = m.pp.unwrap_or(0);
                let current_pp = active.move_pp.get(move_id).copied().unwrap_or(max_pp);
                let power = m.power.map(|p| p.to_string()).unwrap_or("-".to_string());
                let mtype = m.move_type.as_deref().unwrap_or("???");
                println!(
                    "┃      {} ({}) 威力:{} PP:{}/{}          ",
                    name, mtype, power, current_pp, max_pp
                );
            }
        }

        println!("┃                                                              ┃");
    }

    // Field effects
    if !state.field.global.is_empty() {
        println!("┃  フィールド効果:                                            ┃");
        for effect in &state.field.global {
            let turns = effect
                .remaining_turns
                .map(|t| format!(" (残り{}ターン)", t))
                .unwrap_or_default();
            println!(
                "┃    {}{}                                          ",
                effect.id, turns
            );
        }
    }

    println!("┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛");
}

fn hp_bar_string(hp: i32, max_hp: i32) -> String {
    let pct = (hp as f32 / max_hp as f32).max(0.0).min(1.0);
    let bar_len = (pct * 20.0) as usize;
    let color = if pct > 0.5 {
        "🟩"
    } else if pct > 0.25 {
        "🟨"
    } else {
        "🟥"
    };
    let bar = color.repeat(bar_len);
    let empty = "⬜".repeat(20 - bar_len);
    format!("[{}{}]", bar, empty)
}

fn format_status(status_id: &str, remaining: Option<i32>) -> String {
    let turns = remaining.map(|t| format!("({}T)", t)).unwrap_or_default();
    match status_id {
        "burn" => format!("🔥やけど{}", turns),
        "poison" => format!("☠️どく{}", turns),
        "badly_poisoned" | "toxic" => format!("☠️もうどく{}", turns),
        "paralysis" => format!("⚡まひ{}", turns),
        "sleep" => format!("💤ねむり{}", turns),
        "freeze" | "frozen" => format!("❄️こおり{}", turns),
        "confusion" => format!("💫こんらん{}", turns),
        "flinch" => format!("😨ひるみ{}", turns),
        "trapped" => format!("🔒バインド{}", turns),
        "leech_seed" => format!("🌱やどりぎ{}", turns),
        _ => format!("{}{}", status_id, turns),
    }
}

// ============================================================================
// Battle Simulation
// ============================================================================

fn run_battle_simulation(
    species_db: &SpeciesDatabase,
    move_db: &MoveDatabase,
    learnset_db: &LearnsetDatabase,
    engine: &BattleEngine,
) {
    println!("\n🎮 バトルシミュレーション開始");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Select player's pokemon
    println!("【自分のポケモンを選択】");
    let player_pokemon = match select_pokemon(species_db, move_db, learnset_db, "自分のポケモン")
    {
        Some(p) => p,
        None => {
            println!("❌ ポケモン選択がキャンセルされました");
            return;
        }
    };

    // Select opponent's pokemon
    println!("\n【相手のポケモンを選択】");
    let opponent_pokemon = match select_pokemon(species_db, move_db, learnset_db, "相手のポケモン")
    {
        Some(p) => p,
        None => {
            println!("❌ ポケモン選択がキャンセルされました");
            return;
        }
    };

    // Create battle state
    let mut state = create_battle(vec![player_pokemon], vec![opponent_pokemon]);

    println!("\n⚔️ バトル開始！\n");

    // Battle loop
    loop {
        print_battle_state(&state, move_db);

        if is_battle_over(&state) {
            println!("\n🏆 バトル終了！");
            let p1_alive = state.players[0].team.iter().any(|c| c.hp > 0);
            if p1_alive {
                println!("   🎉 あなたの勝ち！");
            } else {
                println!("   😢 あなたの負け...");
            }
            break;
        }

        // Print recent logs
        if !state.log.is_empty() {
            println!("\n📜 ログ:");
            for log in state.log.iter().rev().take(10).rev() {
                println!("   {}", log);
            }
        }

        // Action menu
        let options = vec![
            "⚔️  技を使う",
            "🔄 ポケモン交代",
            "🧮 ダメージ予測",
            "📊 詳細ステータス",
            "🚪 バトル終了",
        ];

        let action = Select::new("アクションを選択", options)
            .prompt()
            .unwrap_or("🚪 バトル終了");

        match action {
            "⚔️  技を使う" => {
                if let Some(actions) = select_battle_actions(&state, move_db, engine) {
                    let mut rng = rand_f64;
                    state =
                        engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());
                }
            }
            "🔄 ポケモン交代" => {
                println!("   現在、1対1バトルのため交代できません");
            }
            "🧮 ダメージ予測" => {
                predict_damage(&state, move_db, engine);
            }
            "📊 詳細ステータス" => {
                let active = &state.players[0].team[state.players[0].active_slot];
                print_creature_details(active, move_db);
                let opp_active = &state.players[1].team[state.players[1].active_slot];
                print_creature_details(opp_active, move_db);
            }
            "🚪 バトル終了" | _ => {
                println!("👋 バトルを終了します");
                break;
            }
        }
    }
}

fn create_battle(p1_team: Vec<CreatureState>, p2_team: Vec<CreatureState>) -> BattleState {
    let p1 = PlayerState {
        id: "p1".to_string(),
        name: "あなた".to_string(),
        team: p1_team,
        active_slot: 0,
        last_fainted_ability: None,
    };
    let p2 = PlayerState {
        id: "p2".to_string(),
        name: "相手".to_string(),
        team: p2_team,
        active_slot: 0,
        last_fainted_ability: None,
    };
    BattleState {
        players: vec![p1, p2],
        turn: 1,
        field: FieldState {
            global: Vec::new(),
            sides: HashMap::new(),
        },
        log: Vec::new(),
        history: None,
    }
}

fn select_battle_actions(
    state: &BattleState,
    move_db: &MoveDatabase,
    _engine: &BattleEngine,
) -> Option<Vec<Action>> {
    let player = &state.players[0];
    let active = &player.team[player.active_slot];

    // Player selects move
    let move_options: Vec<String> = active
        .moves
        .iter()
        .filter_map(|id| {
            let m = move_db.get(id)?;
            let name = m.name.as_deref().unwrap_or(id);
            let mtype = m.move_type.as_deref().unwrap_or("???");
            let power = m.power.map(|p| p.to_string()).unwrap_or("-".to_string());
            let max_pp = m.pp.unwrap_or(0);
            let current_pp = active.move_pp.get(id).copied().unwrap_or(max_pp);
            let romaji = name.to_romaji();
            Some(format!(
                "{} | {} | 威力:{} | PP:{}/{} | {} | ID:{}",
                name, mtype, power, current_pp, max_pp, romaji, id
            ))
        })
        .collect();

    if move_options.is_empty() {
        println!("❌ 使える技がありません");
        return None;
    }

    let selected = Select::new("技を選択", move_options).prompt().ok()?;

    let move_id = selected.split(" | ID:").last()?.to_string();

    let p1_action = Action {
        player_id: "p1".to_string(),
        action_type: ActionType::Move,
        move_id: Some(move_id),
        target_id: Some("p2".to_string()),
        slot: None,
        priority: None,
    };

    // AI selects random move
    let opponent = &state.players[1];
    let opp_active = &opponent.team[opponent.active_slot];
    let opp_move_id = opp_active.moves.first().cloned();

    let p2_action = Action {
        player_id: "p2".to_string(),
        action_type: ActionType::Move,
        move_id: opp_move_id,
        target_id: Some("p1".to_string()),
        slot: None,
        priority: None,
    };

    Some(vec![p1_action, p2_action])
}

fn rand_f64() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    nanos as f64 / 4_294_967_295.0
}

// ============================================================================
// Damage Calculator
// ============================================================================

fn predict_damage(state: &BattleState, move_db: &MoveDatabase, _engine: &BattleEngine) {
    let player = &state.players[0];
    let active = &player.team[player.active_slot];
    let opponent = &state.players[1];
    let opp_active = &opponent.team[opponent.active_slot];

    println!("\n🧮 ダメージ予測");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for move_id in &active.moves {
        if let Some(m) = move_db.get(move_id) {
            let name = m.name.as_deref().unwrap_or(move_id);

            // Skip status moves
            if m.category.as_deref() == Some("status") || m.power.unwrap_or(0) == 0 {
                println!("  {} (変化技 - ダメージなし)", name);
                continue;
            }

            let damage_info = calc_damage_breakdown(active, opp_active, m);
            println!("  【{}】", name);
            println!(
                "    タイプ: {} | カテゴリ: {} | 威力: {}",
                format_type(m.move_type.as_deref().unwrap_or("???")),
                format_category(m.category.as_deref().unwrap_or("???")),
                m.power.unwrap_or(0)
            );
            println!("    タイプ相性: {}x", damage_info.type_effectiveness);
            println!(
                "    攻撃実数値: {} → 防御実数値: {}",
                damage_info.atk_stat, damage_info.def_stat
            );
            println!(
                "    ダメージ範囲: {} ~ {} (HP {}% ~ {}%)",
                damage_info.min_damage,
                damage_info.max_damage,
                (damage_info.min_damage as f32 / opp_active.max_hp as f32 * 100.0) as i32,
                (damage_info.max_damage as f32 / opp_active.max_hp as f32 * 100.0) as i32
            );

            // OHKO check
            if damage_info.min_damage >= opp_active.hp {
                println!("    ⚡ 確定1発！");
            } else if damage_info.max_damage >= opp_active.hp {
                println!("    💫 乱数1発 ({:.1}%)", damage_info.ohko_chance * 100.0);
            }
            println!();
        }
    }
}

struct DamageBreakdown {
    atk_stat: i32,
    def_stat: i32,
    type_effectiveness: f32,
    min_damage: i32,
    max_damage: i32,
    ohko_chance: f32,
}

fn calc_damage_breakdown(
    attacker: &CreatureState,
    defender: &CreatureState,
    move_data: &engine_rust::data::moves::MoveData,
) -> DamageBreakdown {
    let level = attacker.level as i32;
    let power = move_data.power.unwrap_or(0);

    let is_special = move_data.category.as_deref() == Some("special");
    let atk_stat = if is_special {
        attacker.sp_attack
    } else {
        attacker.attack
    };
    let def_stat = if is_special {
        defender.sp_defense
    } else {
        defender.defense
    };

    // Type effectiveness (simplified - would need full type chart)
    let move_type = move_data.move_type.as_deref().unwrap_or("normal");
    let type_effectiveness = calc_type_effectiveness(move_type, &defender.types);

    // STAB
    let stab = if attacker
        .types
        .iter()
        .any(|t| t.to_lowercase() == move_type.to_lowercase())
    {
        1.5
    } else {
        1.0
    };

    // Base damage formula
    let base_damage = ((2 * level / 5 + 2) * power * atk_stat / def_stat / 50 + 2) as f32;
    let modified_damage = base_damage * stab * type_effectiveness;

    let min_damage = (modified_damage * 0.85) as i32;
    let max_damage = modified_damage as i32;

    // OHKO chance (simplified)
    let ohko_chance = if min_damage >= defender.hp {
        1.0
    } else if max_damage < defender.hp {
        0.0
    } else {
        let range = max_damage - min_damage + 1;
        let ohko_rolls = max_damage - defender.hp + 1;
        (ohko_rolls as f32 / range as f32).max(0.0).min(1.0)
    };

    DamageBreakdown {
        atk_stat,
        def_stat,
        type_effectiveness,
        min_damage: min_damage.max(1),
        max_damage: max_damage.max(1),
        ohko_chance,
    }
}

fn calc_type_effectiveness(move_type: &str, defender_types: &[String]) -> f32 {
    // Simplified type chart - would use actual TypeChart in real implementation
    let mut effectiveness = 1.0;

    for def_type in defender_types {
        let mult = match (
            move_type.to_lowercase().as_str(),
            def_type.to_lowercase().as_str(),
        ) {
            // Super effective
            ("fire", "grass") | ("fire", "ice") | ("fire", "bug") | ("fire", "steel") => 2.0,
            ("water", "fire") | ("water", "ground") | ("water", "rock") => 2.0,
            ("grass", "water") | ("grass", "ground") | ("grass", "rock") => 2.0,
            ("electric", "water") | ("electric", "flying") => 2.0,
            ("ice", "grass") | ("ice", "ground") | ("ice", "flying") | ("ice", "dragon") => 2.0,
            ("fighting", "normal")
            | ("fighting", "ice")
            | ("fighting", "rock")
            | ("fighting", "dark")
            | ("fighting", "steel") => 2.0,
            ("ground", "fire")
            | ("ground", "electric")
            | ("ground", "poison")
            | ("ground", "rock")
            | ("ground", "steel") => 2.0,
            ("flying", "grass") | ("flying", "fighting") | ("flying", "bug") => 2.0,
            ("psychic", "fighting") | ("psychic", "poison") => 2.0,
            ("bug", "grass") | ("bug", "psychic") | ("bug", "dark") => 2.0,
            ("rock", "fire") | ("rock", "ice") | ("rock", "flying") | ("rock", "bug") => 2.0,
            ("ghost", "psychic") | ("ghost", "ghost") => 2.0,
            ("dragon", "dragon") => 2.0,
            ("dark", "psychic") | ("dark", "ghost") => 2.0,
            ("steel", "ice") | ("steel", "rock") | ("steel", "fairy") => 2.0,
            ("fairy", "fighting") | ("fairy", "dragon") | ("fairy", "dark") => 2.0,

            // Not very effective
            ("fire", "fire") | ("fire", "water") | ("fire", "rock") | ("fire", "dragon") => 0.5,
            ("water", "water") | ("water", "grass") | ("water", "dragon") => 0.5,
            ("grass", "fire")
            | ("grass", "grass")
            | ("grass", "poison")
            | ("grass", "flying")
            | ("grass", "bug")
            | ("grass", "dragon")
            | ("grass", "steel") => 0.5,
            ("electric", "electric") | ("electric", "grass") | ("electric", "dragon") => 0.5,
            ("fighting", "poison")
            | ("fighting", "flying")
            | ("fighting", "psychic")
            | ("fighting", "bug")
            | ("fighting", "fairy") => 0.5,
            ("poison", "poison")
            | ("poison", "ground")
            | ("poison", "rock")
            | ("poison", "ghost") => 0.5,
            ("ground", "grass") | ("ground", "bug") => 0.5,
            ("flying", "electric") | ("flying", "rock") | ("flying", "steel") => 0.5,
            ("psychic", "psychic") | ("psychic", "steel") => 0.5,
            ("bug", "fire")
            | ("bug", "fighting")
            | ("bug", "poison")
            | ("bug", "flying")
            | ("bug", "ghost")
            | ("bug", "steel")
            | ("bug", "fairy") => 0.5,
            ("rock", "fighting") | ("rock", "ground") | ("rock", "steel") => 0.5,
            ("ghost", "dark") => 0.5,
            ("dragon", "steel") => 0.5,
            ("dark", "fighting") | ("dark", "dark") | ("dark", "fairy") => 0.5,
            ("steel", "fire") | ("steel", "water") | ("steel", "electric") | ("steel", "steel") => {
                0.5
            }
            ("fairy", "fire") | ("fairy", "poison") | ("fairy", "steel") => 0.5,

            // Immunities
            ("normal", "ghost") | ("fighting", "ghost") | ("ghost", "normal") => 0.0,
            ("electric", "ground") => 0.0,
            ("poison", "steel") => 0.0,
            ("ground", "flying") => 0.0,
            ("psychic", "dark") => 0.0,
            ("dragon", "fairy") => 0.0,

            _ => 1.0,
        };
        effectiveness *= mult;
    }

    effectiveness
}

fn damage_calculator(
    species_db: &SpeciesDatabase,
    move_db: &MoveDatabase,
    learnset_db: &LearnsetDatabase,
    engine: &BattleEngine,
) {
    println!("\n🧮 ダメージ計算機");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("【攻撃側のポケモンを選択】");
    let attacker = match select_pokemon(species_db, move_db, learnset_db, "攻撃側") {
        Some(p) => p,
        None => return,
    };

    println!("\n【防御側のポケモンを選択】");
    let defender = match select_pokemon(species_db, move_db, learnset_db, "防御側") {
        Some(p) => p,
        None => return,
    };

    // Create temporary battle state
    let state = create_battle(vec![attacker], vec![defender]);
    predict_damage(&state, move_db, engine);
}

// ============================================================================
// Inspection Tools
// ============================================================================

fn inspect_pokemon(
    species_db: &SpeciesDatabase,
    move_db: &MoveDatabase,
    learnset_db: &LearnsetDatabase,
) {
    let species_list: Vec<_> = species_db.as_map().values().collect();
    let options: Vec<String> = species_list
        .iter()
        .map(|s| {
            let romaji = s.name.to_romaji();
            format!("{} ({}) | {}", s.name, s.id, romaji)
        })
        .collect();

    let selection = Select::new("ポケモンを選択", options.clone())
        .with_page_size(15)
        .prompt();

    if let Ok(sel) = selection {
        if let Some(idx) = options.iter().position(|o| o == &sel) {
            let species = species_list[idx];
            print_species_info(&species.id, species_db, move_db, learnset_db);
        }
    }
}

fn inspect_move(move_db: &MoveDatabase) {
    let moves: Vec<_> = move_db.as_map().values().collect();
    let options: Vec<String> = moves
        .iter()
        .map(|m| {
            let name = m.name.as_deref().unwrap_or(&m.id);
            let romaji = name.to_romaji();
            format!("{} ({}) | {}", name, m.id, romaji)
        })
        .collect();

    let selection = Select::new("技を選択", options.clone())
        .with_page_size(15)
        .prompt();

    if let Ok(sel) = selection {
        if let Some(idx) = options.iter().position(|o| o == &sel) {
            let m = moves[idx];
            println!("\n┌─────────────────────────────────────────────────────────┐");
            let name = m.name.as_deref().unwrap_or(&m.id);
            println!("│ 📖 {} (ID: {})                           ", name, m.id);
            println!("├─────────────────────────────────────────────────────────┤");
            println!(
                "│ タイプ: {}                                          ",
                format_type(m.move_type.as_deref().unwrap_or("???"))
            );
            println!(
                "│ 分類: {}                                            ",
                format_category(m.category.as_deref().unwrap_or("???"))
            );
            println!(
                "│ 威力: {}                                             ",
                m.power.map(|p| p.to_string()).unwrap_or("-".to_string())
            );
            println!(
                "│ 命中: {}                                             ",
                m.accuracy
                    .map(|a| format!("{:.0}%", a * 100.0))
                    .unwrap_or("-".to_string())
            );
            println!(
                "│ PP: {}                                               ",
                m.pp.map(|p| p.to_string()).unwrap_or("-".to_string())
            );
            println!(
                "│ 優先度: {}                                           ",
                m.priority.unwrap_or(0)
            );
            if m.crit_rate.is_some() {
                println!(
                    "│ 急所+: {}                                          ",
                    m.crit_rate.unwrap()
                );
            }
            if !m.tags.is_empty() {
                println!(
                    "│ タグ: {}                                          ",
                    m.tags.join(", ")
                );
            }
            if !m.steps.is_empty() {
                println!("├─────────────────────────────────────────────────────────┤");
                println!("│ 効果:                                                   ");
                for effect in &m.steps {
                    println!(
                        "│   {}: {:?}                             ",
                        effect.effect_type, effect.data
                    );
                }
            }
            println!("└─────────────────────────────────────────────────────────┘\n");
        }
    }
}
