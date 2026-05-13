use engine_rust::ai::get_best_move_minimax;
use engine_rust::core::battle::{is_battle_over, BattleEngine, BattleOptions};
use engine_rust::core::factory::{create_creature, CreateCreatureOptions};
use engine_rust::core::state::{create_battle_state, Action, ActionType, BattleState, PlayerState};
use engine_rust::core::utils::get_active_creature;
use engine_rust::data::learnsets::LearnsetDatabase;
use engine_rust::data::moves::MoveDatabase;
use engine_rust::data::species::SpeciesDatabase;
use inquire::Select;

use std::io::{self, Write};
use wana_kana::ConvertJapanese;

fn main() {
    println!("╔═══════════════════════════════════════╗");
    println!("║      ⚡ ニコポケ バトル CLI ⚡        ║");
    println!("╚═══════════════════════════════════════╝");
    println!();

    // データベース読み込み
    let species_db = SpeciesDatabase::load_default().expect("種族データの読み込みに失敗");
    let move_db = MoveDatabase::load_default().unwrap_or_else(|_| MoveDatabase::minimal());
    let learnset_db = LearnsetDatabase::load_default().unwrap_or_else(|_| LearnsetDatabase::new());
    let engine = BattleEngine::default();

    // チーム選択
    println!("📋 選べるポケモン:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let species_list: Vec<_> = species_db.as_map().values().collect();
    for (i, species) in species_list.iter().enumerate() {
        let total = species.base_stats.hp
            + species.base_stats.atk
            + species.base_stats.def
            + species.base_stats.spa
            + species.base_stats.spd
            + species.base_stats.spe;
        let types_str = species.types.join(" / ");
        let abilities_str = species.abilities.join(" / ");
        let romaji = species.name.to_romaji();
        println!("  {}. {} ({})", i + 1, species.name, romaji);
        println!("     タイプ: {}", types_str);
        println!("     特性: {}", abilities_str);
        println!(
            "     種族値: H{} A{} B{} C{} D{} S{} (計{})",
            species.base_stats.hp,
            species.base_stats.atk,
            species.base_stats.def,
            species.base_stats.spa,
            species.base_stats.spd,
            species.base_stats.spe,
            total
        );
        println!();
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // モード選択
    println!();
    println!("📝 設定を選択してください");
    println!("  1. 通常バトル");
    println!("  2. シミュレーションモード（自動対戦）");
    print!("> ");
    io::stdout().flush().ok();
    let mut main_mode_input = String::new();
    io::stdin().read_line(&mut main_mode_input).ok();
    let is_simulation = main_mode_input.trim() == "2";

    println!();
    println!("🤖 AIの強さを選択してください");
    println!("  1. Minimax (2手先読み)");
    println!("  2. ランダム (とにかく技を出す)");
    print!("> ");
    io::stdout().flush().ok();
    let mut ai_mode_input = String::new();
    io::stdin().read_line(&mut ai_mode_input).ok();
    let ai_is_random = ai_mode_input.trim() == "2";

    // 相手の技構成は常にランダムにする
    let randomize_ai_moves = true;

    // プレイヤーが3匹選択
    println!();
    println!("🎮 チームに入れる3匹を選んでください（番号をスペース区切りで入力）:");
    let player_indices = read_numbers(3, species_list.len());

    // プレイヤーの技選択モード
    let detailed_mode = if !is_simulation {
        println!();
        println!("📝 技の選択方法:");
        println!("  1. 通常モード（ランダム4つ）");
        println!("  2. 詳細モード（自分で1つずつ選択）");
        print!("> ");
        io::stdout().flush().ok();
        let mut mode_input = String::new();
        io::stdin().read_line(&mut mode_input).ok();
        mode_input.trim() == "2"
    } else {
        false
    };

    let mut player_team = Vec::new();
    for idx in &player_indices {
        let species = species_list[*idx];
        let learnable: Vec<String> = learnset_db
            .get(&species.id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|m_id| move_db.get(m_id).is_some())
            .collect();

        let moves: Vec<String> = if detailed_mode {
            // 詳細モード: 技を1つずつ選択させる
            let mut options = Vec::new();
            let mut move_ids = Vec::new();

            for move_id in &learnable {
                if let Some(move_data) = move_db.get(move_id) {
                    let name = move_data
                        .name
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or(move_id);
                    let move_type = move_data
                        .move_type
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or("???");
                    let power = move_data
                        .power
                        .map(|p| p.to_string())
                        .unwrap_or("-".to_string());
                    let category = match move_data.category.as_deref() {
                        Some("physical") => "物理",
                        Some("special") => "特殊",
                        Some("status") => "変化",
                        _ => "???",
                    };
                    let priority = move_data.priority.unwrap_or(0);
                    let priority_str = if priority != 0 {
                        format!(" 優先度:{:+}", priority)
                    } else {
                        String::new()
                    };

                    // Searchable metadata
                    let romaji = name.to_romaji();

                    let display = format!(
                        "{} [{}] {} 威力:{}{} | {} {}",
                        name, move_type, category, power, priority_str, move_id, romaji
                    );
                    options.push(display);
                    move_ids.push(move_id.clone());
                } else {
                    options.push(move_id.clone());
                    move_ids.push(move_id.clone());
                }
            }

            if options.is_empty() {
                Vec::new()
            } else {
                // 1つずつ選択（最大4つまで）
                let mut selected_moves = Vec::new();

                for i in 1..=4 {
                    if selected_moves.len() >= 4 {
                        break;
                    }

                    // 既に選択した技を除外
                    let available_options: Vec<String> = options
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

                    let prompt = if i == 1 {
                        format!(
                            "{}の技を選んでください [{}/4] (Enterで選択):",
                            species.name, i
                        )
                    } else {
                        format!("技を選んでください [{}/4] (Enterで選択):", i)
                    };

                    let ans = Select::new(&prompt, selection_options.clone())
                        .with_page_size(10)
                        .prompt();

                    match ans {
                        Ok(choice) => {
                            if choice == "✅ 選択完了（これ以上選ばない）" {
                                break;
                            }

                            // 選択された技のIDを取得
                            if let Some(original_idx) =
                                options.iter().position(|opt| opt == &choice)
                            {
                                selected_moves.push(move_ids[original_idx].clone());
                            }
                        }
                        Err(_) => {
                            println!("選択がキャンセルされました。");
                            if selected_moves.is_empty() {
                                println!("自動選択します。");
                                selected_moves = learnable.into_iter().take(4).collect();
                            }
                            break;
                        }
                    }
                }

                selected_moves
            }
        } else {
            // 通常モード: ランダム選択
            take_random_moves(learnable, 4)
        };

        if moves.len() < 4 {
            println!(
                "⚠️  警告: {} の技が不足しています（{}個のみロードされました）",
                species.name,
                moves.len()
            );
        }

        let creature = create_creature(
            species,
            CreateCreatureOptions {
                moves: Some(moves),
                ..Default::default()
            },
            &learnset_db,
            &move_db,
        )
        .expect("ポケモン作成に失敗");
        player_team.push(creature);
    }

    // AIが3匹選択
    let mut ai_team = Vec::new();
    let ai_indices: Vec<usize> = species_list
        .iter()
        .enumerate()
        .filter(|(i, _)| !player_indices.contains(i))
        .map(|(i, _)| i)
        .take(3)
        .collect();

    for idx in &ai_indices {
        let species = species_list[*idx];
        let learnable: Vec<String> = learnset_db
            .get(&species.id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|m_id| move_db.get(m_id).is_some())
            .collect();

        let moves: Vec<String> = if randomize_ai_moves {
            take_random_moves(learnable, 4)
        } else {
            learnable.into_iter().take(4).collect()
        };

        if moves.len() < 4 && moves.len() > 0 {
            // learnset_db.get 可能でも move_db にない場合があるので再度チェック
            println!(
                "⚠️  警告: 相手の {} の技が不足しています（{}個のみロードされました）",
                species.name,
                moves.len()
            );
        }

        let creature = create_creature(
            species,
            CreateCreatureOptions {
                moves: Some(moves),
                ..Default::default()
            },
            &learnset_db,
            &move_db,
        )
        .expect("AI ポケモン作成に失敗");
        ai_team.push(creature);
    }

    println!();
    println!(
        "✅ あなたのチーム: {}",
        player_team
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "🤖 相手のチーム: {}",
        ai_team
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    // バトル状態作成
    let player_state = PlayerState {
        id: "player".to_string(),
        name: "あなた".to_string(),
        team: player_team,
        active_slot: 0,
        last_fainted_ability: None,
    };
    let ai_state = PlayerState {
        id: "ai".to_string(),
        name: "相手".to_string(),
        team: ai_team,
        active_slot: 0,
        last_fainted_ability: None,
    };

    let mut state = create_battle_state(vec![player_state, ai_state]);
    let mut rng = || rand_f64();
    let mut last_log_idx = 0;

    println!("════════════════════════════════════════");
    println!("          ⚔️  バトル開始！ ⚔️          ");
    println!("════════════════════════════════════════");
    println!();

    // メインバトルループ
    while !is_battle_over(&state) {
        print_battle_status(&state, &move_db);

        // 交代が必要かチェック
        let player_needs_switch = needs_switch(&state, "player");
        let ai_needs_switch = needs_switch(&state, "ai");

        let mut actions = Vec::new();

        // プレイヤーのアクション
        if player_needs_switch {
            if is_simulation {
                if let Some(action) = ai_switch_for_player(&state, "player") {
                    actions.push(action);
                } else {
                    break;
                }
            } else {
                if let Some(active) = get_active_creature(&state, "player") {
                    if active.hp <= 0 {
                        println!("💀 ポケモンが倒れた！交代するポケモンを選んでください:");
                    } else {
                        println!("🔄 交代するポケモンを選んでください:");
                    }
                }
                if let Some(action) = prompt_switch(&state, "player") {
                    actions.push(action);
                } else {
                    break; // 残りポケモンなし
                }
            }
        } else {
            if is_simulation {
                // シミュレーション時はMinimaxを使用
                if let Some(action) = get_best_move_minimax(&state, "player", 2) {
                    actions.push(action);
                } else if let Some(action) = ai_choose_action_for_player(&state, &move_db, "player")
                {
                    actions.push(action);
                }
            } else {
                loop {
                    let input = prompt_action(&state, &move_db);
                    if let Some(action) = input {
                        actions.push(action);
                        break;
                    }
                }
            }
        }

        // AIのアクション
        if ai_needs_switch {
            if ai_is_random {
                if let Some(action) = ai_switch(&state) {
                    actions.push(action);
                }
            } else {
                if let Some(action) = get_best_move_minimax(&state, "ai", 2) {
                    actions.push(action);
                } else if let Some(action) = ai_switch(&state) {
                    actions.push(action);
                }
            }
        } else {
            if ai_is_random {
                if let Some(action) = ai_random_move(&state, &move_db, "ai") {
                    actions.push(action);
                }
            } else {
                if let Some(action) = get_best_move_minimax(&state, "ai", 2) {
                    actions.push(action);
                } else if let Some(action) = ai_choose_action(&state, &move_db) {
                    actions.push(action);
                }
            }
        }

        // ターン実行
        state = engine.step_battle(&state, &actions, &mut rng, BattleOptions::default());

        // ターンログ表示（詳細情報付き）
        println!();
        print_enriched_logs(&state, &move_db, &mut last_log_idx);
        println!();

        // ターン終了後に交代が必要なら即座に実行
        loop {
            if is_battle_over(&state) {
                break;
            }
            let player_switch_needed = needs_switch(&state, "player");
            let ai_switch_needed = needs_switch(&state, "ai");

            if !player_switch_needed && !ai_switch_needed {
                break;
            }

            let mut switch_actions = Vec::new();

            if player_switch_needed {
                if is_simulation {
                    if let Some(action) = ai_switch_for_player(&state, "player") {
                        switch_actions.push(action);
                    } else {
                        break;
                    }
                } else {
                    if let Some(active) = get_active_creature(&state, "player") {
                        if active.hp <= 0 {
                            println!("💀 ポケモンが倒れた！交代するポケモンを選んでください:");
                        } else {
                            println!("🔄 交代するポケモンを選んでください:");
                        }
                    }
                    if let Some(action) = prompt_switch(&state, "player") {
                        switch_actions.push(action);
                    } else {
                        break; // 残りポケモンなし
                    }
                }
            }

            if ai_switch_needed {
                if ai_is_random {
                    if let Some(action) = ai_switch(&state) {
                        switch_actions.push(action);
                    }
                } else {
                    if let Some(action) = get_best_move_minimax(&state, "ai", 2) {
                        switch_actions.push(action);
                    } else if let Some(action) = ai_switch(&state) {
                        switch_actions.push(action);
                    }
                }
            }

            if switch_actions.is_empty() {
                break;
            }

            state = engine.step_battle(&state, &switch_actions, &mut rng, BattleOptions::default());
            print_enriched_logs(&state, &move_db, &mut last_log_idx);
            println!();
        }
    }

    // 勝敗判定
    println!("════════════════════════════════════════");
    let player_alive = state.players[0].team.iter().any(|c| c.hp > 0);
    if player_alive {
        println!("      🎉 勝利！おめでとう！ 🎉      ");
    } else {
        println!("      💔 負けてしまった...次は頑張ろう!");
    }
    println!("════════════════════════════════════════");
}

fn print_battle_status(state: &BattleState, _move_db: &MoveDatabase) {
    let player = &state.players[0];
    let ai = &state.players[1];
    let player_active = player.team.get(player.active_slot);
    let ai_active = ai.team.get(ai.active_slot);

    println!("─────────────────────────────────────────");
    println!("  ターン {}", state.turn + 1);
    println!("─────────────────────────────────────────");

    if let Some(ai_mon) = ai_active {
        let types_str = ai_mon.types.join("/");
        let ability = ai_mon.ability.as_deref().unwrap_or("なし");
        let item = ai_mon.item.as_deref().unwrap_or("なし");
        let hp_bar = hp_bar_string(ai_mon.hp, ai_mon.max_hp);
        println!("  [相手] {} ({}) {}", ai_mon.name, types_str, hp_bar);
        println!("         特性: {} | 持ち物: {}", ability, item);
        print_stage_changes(&ai_mon.stages);
        print_status_effects(&ai_mon.statuses);
    }

    if let Some(player_mon) = player_active {
        let types_str = player_mon.types.join("/");
        let ability = player_mon.ability.as_deref().unwrap_or("なし");
        let item = player_mon.item.as_deref().unwrap_or("なし");
        let hp_bar = hp_bar_string(player_mon.hp, player_mon.max_hp);
        println!("  [自分] {} ({}) {}", player_mon.name, types_str, hp_bar);
        println!("         特性: {} | 持ち物: {}", ability, item);
        print_stage_changes(&player_mon.stages);
        print_status_effects(&player_mon.statuses);
    }
    println!();
}

fn hp_bar_string(hp: i32, max_hp: i32) -> String {
    let percentage = (hp as f64 / max_hp as f64 * 100.0) as i32;
    let bars = (hp as f64 / max_hp as f64 * 10.0) as usize;
    let filled = "█".repeat(bars);
    let empty = "░".repeat(10 - bars);
    format!("[{}{}] {}/{} ({}%)", filled, empty, hp, max_hp, percentage)
}

fn print_stage_changes(stages: &engine_rust::core::state::StatStages) {
    let mut changes = Vec::new();
    if stages.atk != 0 {
        changes.push(format!("攻撃 {:+}", stages.atk));
    }
    if stages.def != 0 {
        changes.push(format!("防御 {:+}", stages.def));
    }
    if stages.spa != 0 {
        changes.push(format!("特攻 {:+}", stages.spa));
    }
    if stages.spd != 0 {
        changes.push(format!("特防 {:+}", stages.spd));
    }
    if stages.spe != 0 {
        changes.push(format!("素早 {:+}", stages.spe));
    }
    if !changes.is_empty() {
        println!("         ランク変化: {}", changes.join(", "));
    }
}

fn print_status_effects(statuses: &[engine_rust::core::state::Status]) {
    let status_names: Vec<&str> = statuses
        .iter()
        .filter(|s| s.id != "pending_switch")
        .map(|s| match s.id.as_str() {
            "burn" => "やけど",
            "poison" => "どく",
            "toxic" => "もうどく",
            "paralysis" => "まひ",
            "sleep" => "ねむり",
            "freeze" => "こおり",
            "confusion" => "こんらん",
            "substitute" => "みがわり",
            "protect" => "まもる",
            "taunt" => "ちょうはつ",
            "encore" => "アンコール",
            other => other,
        })
        .collect();

    if !status_names.is_empty() {
        println!("         状態: {}", status_names.join(", "));
    }
}

fn print_enriched_logs(state: &BattleState, move_db: &MoveDatabase, last_idx: &mut usize) {
    for i in *last_idx..state.log.len() {
        let log = &state.log[i];
        print!("  📝 {}", log);

        // 技の使用ログであれば詳細を追記する
        // 形式: "ポケモン名の 技名！"
        if log.ends_with('！')
            && !log.contains("ダメージ")
            && !log.contains("回復")
            && !log.contains("たおれた")
            && !log.contains("守った")
        {
            if let Some(pos) = log.find("の ") {
                let move_part = &log[pos + 3..].trim_end_matches('！');
                // 技名部分にスペースが含まれている場合は、最初の部分を技名とする（一撃必殺などが続く場合のため）
                let move_name = move_part.split_whitespace().next().unwrap_or(move_part);

                // データベースから技を検索
                if let Some(move_data) = find_move_by_name(move_db, move_name) {
                    let move_type = move_data.move_type.as_deref().unwrap_or("???");
                    let category = match move_data.category.as_deref() {
                        Some("physical") => "物理",
                        Some("special") => "特殊",
                        Some("status") => "変化",
                        _ => "???",
                    };
                    let power = move_data
                        .power
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    print!(
                        " [タイプ: {}, 威力: {}, 分類: {}]",
                        move_type, power, category
                    );
                }
            }
        }
        println!();
    }
    *last_idx = state.log.len();
}

fn find_move_by_name<'a>(
    move_db: &'a MoveDatabase,
    name: &str,
) -> Option<&'a engine_rust::data::moves::MoveData> {
    for m in move_db.as_map().values() {
        if let Some(n) = &m.name {
            if n == name {
                return Some(m);
            }
        }
        if m.id == name {
            return Some(m);
        }
    }
    None
}

fn prompt_action(state: &BattleState, move_db: &MoveDatabase) -> Option<Action> {
    println!("どうする？");
    println!("  1. たたかう");
    println!("  2. ポケモン（交代）");
    println!("  /status - 詳細ステータス表示");
    println!("  /moves - 技の詳細表示");
    println!("  /team - チーム状態表示");
    println!("  /help - ヘルプ表示");
    print!("> ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;
    let input = input.trim();

    // スラッシュコマンド処理
    if input.starts_with('/') {
        handle_command(input, state, move_db);
        return None;
    }

    match input {
        "1" => prompt_move(state, move_db),
        "2" => prompt_switch(state, "player"),
        _ => {
            println!("無効な選択です。1か2を入力してください。");
            None
        }
    }
}

fn handle_command(cmd: &str, state: &BattleState, move_db: &MoveDatabase) {
    match cmd {
        "/status" => {
            println!();
            println!("══════ 詳細ステータス ══════");
            for player in &state.players {
                if let Some(active) = player.team.get(player.active_slot) {
                    println!("[{}] {} (場に出ている)", player.name, active.name);
                    println!("  HP: {}/{}", active.hp, active.max_hp);
                    println!("  攻撃: {} ({:+})", active.attack, active.stages.atk);
                    println!("  防御: {} ({:+})", active.defense, active.stages.def);
                    println!("  特攻: {} ({:+})", active.sp_attack, active.stages.spa);
                    println!("  特防: {} ({:+})", active.sp_defense, active.stages.spd);
                    println!("  素早さ: {} ({:+})", active.speed, active.stages.spe);
                    if !active.statuses.is_empty() {
                        let status_names: Vec<_> =
                            active.statuses.iter().map(|s| s.id.as_str()).collect();
                        println!("  状態異常: {}", status_names.join(", "));
                    }
                    println!();
                }
            }
        }
        "/moves" => {
            println!();
            println!("══════ 技の詳細 ══════");
            if let Some(active) = get_active_creature(state, "player") {
                for (i, move_id) in active.moves.iter().enumerate() {
                    if let Some(move_data) = move_db.get(move_id) {
                        let name = move_data
                            .name
                            .as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or(move_id);
                        let move_type = move_data
                            .move_type
                            .as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or("???");
                        let power = move_data
                            .power
                            .map(|p| p.to_string())
                            .unwrap_or("-".to_string());
                        let pp = move_data.pp.unwrap_or(0);
                        let current_pp = active.move_pp.get(move_id).copied().unwrap_or(pp);
                        let category = move_data
                            .category
                            .as_ref()
                            .map(|s| s.as_str())
                            .unwrap_or("???");
                        let priority = move_data.priority.unwrap_or(0);
                        println!(
                            "  {}. {} [{}] - {} | 威力: {} | PP: {}/{} | 優先度: {:+}",
                            i + 1,
                            name,
                            move_type,
                            category,
                            power,
                            current_pp,
                            pp,
                            priority
                        );
                    }
                }
            }
            println!();
        }
        "/team" => {
            println!();
            println!("══════ あなたのチーム ══════");
            let player = &state.players[0];
            for (i, mon) in player.team.iter().enumerate() {
                let active = if i == player.active_slot {
                    " (場に出ている)"
                } else {
                    ""
                };
                let status = if mon.hp <= 0 { " 💀" } else { "" };
                println!(
                    "  {}. {} HP: {}/{}{}{}",
                    i + 1,
                    mon.name,
                    mon.hp,
                    mon.max_hp,
                    active,
                    status
                );
            }
            println!();
        }
        "/help" => {
            println!();
            println!("══════ コマンド一覧 ══════");
            println!("  /status - 場のポケモンの詳細ステータスを表示");
            println!("  /moves  - 技の詳細を表示（タイプ、威力、PP、優先度）");
            println!("  /team   - 自分のチーム状態を表示");
            println!("  /log    - バトルログを表示");
            println!("  /help   - このヘルプを表示");
            println!();
        }
        "/log" => {
            println!();
            println!("══════ バトルログ ══════");
            for log in &state.log {
                println!("  {}", log);
            }
            println!();
        }
        _ => {
            println!("不明なコマンドです。/help で使えるコマンドを確認してください。");
        }
    }
}

fn prompt_move(state: &BattleState, move_db: &MoveDatabase) -> Option<Action> {
    let active = get_active_creature(state, "player")?;

    let mut options = Vec::new();
    let mut move_ids = Vec::new();

    for move_id in &active.moves {
        if let Some(move_data) = move_db.get(move_id) {
            let name = move_data
                .name
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(move_id);
            let move_type = move_data
                .move_type
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("???");
            let power = move_data
                .power
                .map(|p| p.to_string())
                .unwrap_or("-".to_string());
            let pp = move_data.pp.unwrap_or(0);
            let current_pp = active.move_pp.get(move_id).copied().unwrap_or(pp);

            // Searchable metadata: Romaji of the name + English ID
            let romaji = name.to_romaji();

            let display = if current_pp == 0 {
                format!(
                    "{} [{}] 威力:{} (PP切れ) | {} {}",
                    name, move_type, power, move_id, romaji
                )
            } else {
                format!(
                    "{} [{}] 威力:{} PP:{}/{} | {} {}",
                    name, move_type, power, current_pp, pp, move_id, romaji
                )
            };
            options.push(display);
            move_ids.push(move_id.clone());
        } else {
            options.push(move_id.clone());
            move_ids.push(move_id.clone());
        }
    }

    if options.is_empty() {
        println!("使える技がありません！");
        return None;
    }

    // 戻るオプション
    options.push("戻る".to_string());

    let ans = Select::new("技を選択(入力で絞り込み):", options.clone())
        .with_page_size(4)
        .prompt();

    match ans {
        Ok(choice) => {
            if choice == "戻る" {
                return None;
            }

            // 表示文字列のリストから選択されたもののインデックスを見つける
            let mut found_idx = None;

            for (i, opt) in options.iter().enumerate() {
                if opt == &choice {
                    found_idx = Some(i);
                    break;
                }
            }

            if let Some(idx) = found_idx {
                if idx >= move_ids.len() {
                    return None; // "戻る" was selected (double check)
                }

                let selected_move_id = &move_ids[idx];

                // PP check
                if let Some(move_data) = move_db.get(selected_move_id) {
                    let pp = move_data.pp.unwrap_or(0);
                    let current_pp = active.move_pp.get(selected_move_id).copied().unwrap_or(pp);
                    if current_pp == 0 {
                        println!("❌ その技はPPが切れています！");
                        // 再帰呼び出しで選び直させる
                        return prompt_move(state, move_db);
                    }
                }

                Some(Action {
                    player_id: "player".to_string(),
                    action_type: ActionType::Move,
                    move_id: Some(selected_move_id.clone()),
                    target_id: Some("ai".to_string()),
                    slot: None,
                    priority: None,
                })
            } else {
                None
            }
        }
        Err(_) => {
            println!("選択がキャンセルされました。");
            None
        }
    }
}

fn prompt_switch(state: &BattleState, player_id: &str) -> Option<Action> {
    let player_idx = state.players.iter().position(|p| p.id == player_id)?;
    let player = &state.players[player_idx];

    let available: Vec<(usize, &engine_rust::core::state::CreatureState)> = player
        .team
        .iter()
        .enumerate()
        .filter(|(i, c)| *i != player.active_slot && c.hp > 0)
        .collect();

    if available.is_empty() {
        println!("交代できるポケモンがいません！");
        return None;
    }

    println!();
    println!("交代するポケモンを選んでください:");
    for (display_idx, (_slot, mon)) in available.iter().enumerate() {
        println!(
            "  {}. {} HP: {}/{}",
            display_idx + 1,
            mon.name,
            mon.hp,
            mon.max_hp
        );
    }
    print!("> ");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;
    let choice: usize = input.trim().parse().ok()?;

    if choice == 0 || choice > available.len() {
        println!("無効な選択です。");
        return None;
    }

    let (slot, _) = available[choice - 1];
    Some(Action {
        player_id: player_id.to_string(),
        action_type: ActionType::Switch,
        move_id: None,
        target_id: None,
        slot: Some(slot),
        priority: None,
    })
}

fn needs_switch(state: &BattleState, player_id: &str) -> bool {
    if let Some(active) = get_active_creature(state, player_id) {
        active.hp <= 0 || active.statuses.iter().any(|s| s.id == "pending_switch")
    } else {
        false
    }
}

fn ai_switch(state: &BattleState) -> Option<Action> {
    let ai = state.players.iter().find(|p| p.id == "ai")?;
    let available: Vec<usize> = ai
        .team
        .iter()
        .enumerate()
        .filter(|(i, c)| *i != ai.active_slot && c.hp > 0)
        .map(|(i, _)| i)
        .collect();

    if available.is_empty() {
        return None;
    }

    Some(Action {
        player_id: "ai".to_string(),
        action_type: ActionType::Switch,
        move_id: None,
        target_id: None,
        slot: Some(available[0]),
        priority: None,
    })
}

fn ai_random_move(state: &BattleState, move_db: &MoveDatabase, player_id: &str) -> Option<Action> {
    let player = state.players.iter().find(|p| p.id == player_id)?;
    let active = player.team.get(player.active_slot)?;
    let opponent_id = if player_id == "player" {
        "ai"
    } else {
        "player"
    };

    if active.hp <= 0 {
        return None;
    }

    let usable_moves: Vec<&String> = active
        .moves
        .iter()
        .filter(|move_id| {
            if let Some(move_data) = move_db.get(*move_id) {
                let pp = move_data.pp.unwrap_or(10);
                let current_pp = active.move_pp.get(*move_id).copied().unwrap_or(pp);
                current_pp > 0
            } else {
                true
            }
        })
        .collect();

    if usable_moves.is_empty() {
        return Some(Action {
            player_id: player_id.to_string(),
            action_type: ActionType::Move,
            move_id: active.moves.first().cloned(),
            target_id: Some(opponent_id.to_string()),
            slot: None,
            priority: None,
        });
    }

    let idx = (rand_f64() * usable_moves.len() as f64) as usize;
    let selected_move = usable_moves[idx.min(usable_moves.len() - 1)];

    Some(Action {
        player_id: player_id.to_string(),
        action_type: ActionType::Move,
        move_id: Some(selected_move.clone()),
        target_id: Some(opponent_id.to_string()),
        slot: None,
        priority: None,
    })
}

fn ai_switch_for_player(state: &BattleState, player_id: &str) -> Option<Action> {
    let player = state.players.iter().find(|p| p.id == player_id)?;
    let available: Vec<usize> = player
        .team
        .iter()
        .enumerate()
        .filter(|(i, c)| *i != player.active_slot && c.hp > 0)
        .map(|(i, _)| i)
        .collect();

    if available.is_empty() {
        return None;
    }

    Some(Action {
        player_id: player_id.to_string(),
        action_type: ActionType::Switch,
        move_id: None,
        target_id: None,
        slot: Some(available[0]),
        priority: None,
    })
}

fn ai_choose_action_for_player(
    state: &BattleState,
    move_db: &MoveDatabase,
    player_id: &str,
) -> Option<Action> {
    let player = state.players.iter().find(|p| p.id == player_id)?;
    let active = player.team.get(player.active_slot)?;
    let opponent_id = if player_id == "player" {
        "ai"
    } else {
        "player"
    };

    if active.hp <= 0 {
        return ai_switch_for_player(state, player_id);
    }

    if active.moves.is_empty() {
        return None;
    }

    let usable_moves: Vec<&String> = active
        .moves
        .iter()
        .filter(|move_id| {
            if let Some(move_data) = move_db.get(*move_id) {
                let pp = move_data.pp.unwrap_or(10);
                let current_pp = active.move_pp.get(*move_id).copied().unwrap_or(pp);
                current_pp > 0
            } else {
                true
            }
        })
        .collect();

    if usable_moves.is_empty() {
        return Some(Action {
            player_id: player_id.to_string(),
            action_type: ActionType::Move,
            move_id: active.moves.first().cloned(),
            target_id: Some(opponent_id.to_string()),
            slot: None,
            priority: None,
        });
    }

    // シンプルAI: 威力が高い技を選ぶ（プレイヤー自動対戦用）
    let mut best_move = usable_moves.first().map(|s| (*s).clone()).unwrap();
    let mut best_power = 0;

    for move_id in &usable_moves {
        if let Some(move_data) = move_db.get(*move_id) {
            let power = move_data.power.unwrap_or(0);
            if power > best_power {
                best_power = power;
                best_move = (*move_id).clone();
            }
        }
    }

    Some(Action {
        player_id: player_id.to_string(),
        action_type: ActionType::Move,
        move_id: Some(best_move),
        target_id: Some(opponent_id.to_string()),
        slot: None,
        priority: None,
    })
}

fn ai_choose_action(state: &BattleState, move_db: &MoveDatabase) -> Option<Action> {
    let ai = state.players.iter().find(|p| p.id == "ai")?;
    let active = ai.team.get(ai.active_slot)?;

    if active.hp <= 0 {
        return ai_switch(state);
    }

    // 技がない場合はスキップ
    if active.moves.is_empty() {
        return None;
    }

    // PPが残っている技から選択
    let usable_moves: Vec<&String> = active
        .moves
        .iter()
        .filter(|move_id| {
            if let Some(move_data) = move_db.get(*move_id) {
                let pp = move_data.pp.unwrap_or(10);
                let current_pp = active.move_pp.get(*move_id).copied().unwrap_or(pp);
                current_pp > 0
            } else {
                true // データがない技はとりあえず使える扱い
            }
        })
        .collect();

    // 使える技がない場合はわるあがき（最初の技を使用）
    if usable_moves.is_empty() {
        return Some(Action {
            player_id: "ai".to_string(),
            action_type: ActionType::Move,
            move_id: active.moves.first().cloned(),
            target_id: Some("player".to_string()),
            slot: None,
            priority: None,
        });
    }

    // シンプルAI: 威力が高い技を選ぶ
    let mut best_move = usable_moves.first().map(|s| (*s).clone())?;
    let mut best_power = 0;

    for move_id in &usable_moves {
        if let Some(move_data) = move_db.get(*move_id) {
            let power = move_data.power.unwrap_or(0);
            if power > best_power {
                best_power = power;
                best_move = (*move_id).clone();
            }
        }
    }

    Some(Action {
        player_id: "ai".to_string(),
        action_type: ActionType::Move,
        move_id: Some(best_move),
        target_id: Some("player".to_string()),
        slot: None,
        priority: None,
    })
}

fn read_numbers(count: usize, max: usize) -> Vec<usize> {
    loop {
        print!("> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }

        let numbers: Vec<usize> = input
            .split_whitespace()
            .filter_map(|s| s.parse::<usize>().ok())
            .filter(|&n| n >= 1 && n <= max)
            .map(|n| n - 1)
            .take(count)
            .collect();

        if numbers.len() == count {
            return numbers;
        }

        println!("{}個の有効な番号を入力してください（1-{}）。", count, max);
    }
}

fn take_random_moves(mut moves: Vec<String>, count: usize) -> Vec<String> {
    if moves.len() <= count {
        return moves;
    }

    // 簡易的なシャッフル（rand_f64を使用）
    for i in (1..moves.len()).rev() {
        let j = (rand_f64() * (i + 1) as f64) as usize;
        if j <= i {
            moves.swap(i, j);
        }
    }

    moves.into_iter().take(count).collect()
}

fn rand_f64() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEED: AtomicU64 = AtomicU64::new(0);

    // Initialize seed from time if not yet initialized
    let mut seed = SEED.load(Ordering::Relaxed);
    if seed == 0 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        seed = now;
        SEED.store(seed, Ordering::Relaxed);
    }

    // LCG random number generator with time-based perturbation
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(now % 1000);
    SEED.store(seed, Ordering::Relaxed);

    (seed as f64) / (u64::MAX as f64)
}
