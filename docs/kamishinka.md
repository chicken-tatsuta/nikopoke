# カミシンカ実装・検証メモ

## 実装方針

既存の Rust `BattleEngine`、WASM、React の対戦再生、PeerJS のホスト側ターン解決を拡張した。既存の通常種族データ、ダメージ計算、持ち物保存、状態異常・特性イベントを再利用する。

- `kamidama` を共通の非消費進化アイテムとして登録。編成の持ち物重複制限から、このアイテムだけを除外。
- プレイヤーに `kamishinka_used`、個体に通常形態の復元情報を含む `kamishinka`、技アクションに `kamishinka_form` を追加。WASM/PeerJS 上は camelCase。旧保存形式の新フィールド省略は許容。
- 通常の交代を処理した後、最初の技の前に両者の予約を検証・適用。進化後の素早さ、能力ランク、麻痺、特性、トリックルーム、技優先度を使って技の順序を再計算。
- 持ち物・通常種族との対応・使用権・生存・強制交代待ち・技の存在/所持/PPをエンジンで検証。1プレイヤーの重複アクションは既存処理で除外。睡眠などによる行動不能判定より先に進化する。
- 名前・タイプ・特性・HP以外の能力値を変更。レベル、固定IV31、既存のEV計算式を再利用。現在/最大HP、PP、能力ランク、状態異常、継続する技の状態を維持。旧特性の専用記録のみ初期化。
- `species_id` は通常種族のIDとして保持し、形態IDは別フィールドで管理。図鑑・覚える技・パーティ情報を破壊しない。対戦内の画像・種族値表示は形態データを参照。
- 通常交代・瀕死で形態と使用権は消えない。結果画面への遷移時にローカル・オンラインの一時状態を復元し、新規バトル生成時にもリセットする。元の編成データとカミダマは消費しない。
- ボタンで予約/解除し、イックンはX/Yを明示選択。技とセットで送信。暗転→カミダマ発光→光で包む→形態表示→メッセージの演出中は技・交代をロック。
- 対人戦はホストが進化と使用権を含む状態を配信。ゲストの送信元役割/プレイヤーIDを照合し、解決中の重複受付を防止。
- AIの候補に進化付きの技を追加。Vegaの盤面キャッシュに形態・使用権・特性・なまけの状態・そうだいしょうの記録を含める。

## 形態と画像

正本は `engine-rust/data/kamishinka.json`。Rustはコンパイル時、フロントはJSON importで同じ登録データを読む。

| 通常ID | 形態ID | 名前 | 特性 |
| --- | --- | --- | --- |
| makocchan | kami_makocchan | カミマコッチャン | かがくへんかガス |
| nisiki | kami_nisiki | カミニシキ | いろめがね |
| ikkun | kami_ikkun_x | カミイックンX | なまけ |
| ikkun | kami_ikkun_y | カミイックンY | あついしぼう |
| morimitu | kami_morimitu | カミモリミツ | どくしゅ |
| ayuma | kami_ayuma | カミアユマ | そうだいしょう |
| morimori | kami_morimori | カミモリモリ | ねつぼうそう |
| haruta | kami_haruta | カミハルタ | サンパワー |

種族値とタイプは依頼の表の数値を登録した。HP種族値もデータとして保持するが、進化時の現在/最大HPには適用しない。

画像は通常形態と共通とし、進化中は虹色のエフェクトを重ねる。`image: null` は通常形態の画像を再利用する指定。未設定の案内は表示しない。必要なら将来 `image` に別の画像URLを登録できる。

新形態の追加は、既存通常ID、固有の形態ID、名前、6種族値、タイプ、既存特性ID、画像URLをJSONへ追加し、WASMとフロントを再ビルドする。専用の持ち物・分岐コードの追加は不要。新しい未実装特性自体を追加する場合はその効果実装が別途必要。

## 特性

ユーザー確認済みの「本家と同じ仕様」に従い、あついしぼうの既存処理を利用し、残る7特性を既存フックへ追加した。

- かがくへんかガス：有効特性の取得を共通化。既存特性フック、ダメージ計算中の直接参照、浮遊判定などから参照。抑制対象外の特性は維持。退出・瀕死・特性変更で抑制が解除された際に登場時フックを再実行。既に得た能力ランクは消去しない。スロースタートも再開時から数える。
- いろめがね：半減以下の相性によるダメージを2倍。無効相性は無効のまま。
- なまけ：行動判定を交互にスキップ。交代や再発動時はリセット。
- どくしゅ：接触ダメージから30%で既存の毒付与処理へ接続。みがわりは除外し、毒・鋼タイプなどの既存無効判定を再利用。
- そうだいしょう：登場/進化時点の味方の瀕死回数を記録し、1回につき威力10%、最大50%。プレイヤーの瀕死回数を記録するため、復活によって過去の瀕死回数が消えない。
- ねつぼうそう：やけど中の特殊技威力1.5倍。
- サンパワー：晴れで特攻1.5倍、ターン終了時に最大HPの1/8ダメージ。

挙動の照合には [Pokémon Showdownの特性実装](https://github.com/smogon/pokemon-showdown/blob/master/data/abilities.ts) を参照。独自仕様が必要な特性は今回残っていない。ただし既存エンジン全体を本家全世代のすべての相互作用へ拡張する変更ではない。

## 検証結果

- カミシンカ専用Rustテスト：16件成功。
- Rust全体：224件成功、3件失敗。3件は変更前コードでも再現する既存不具合（下記参照）。
- ブラウザー用WASM：再生成済み。実WASMのJSON送受信相当、複数所持、両者の進化、再要求拒否、リセット検証に成功。
- 既存バトルデータ検査：456技・24種族で成功。
- ブラウザーUI：2匹へのカミダマ設定、予約/解除、演出の全画面表示・操作ロック、進化後名称、使用済み表示、別個体への交代/戻り後の維持、390px幅の表示を確認。
- TypeScriptチェック・Vite本番ビルド成功。新規TS/TSXモジュールのESLint成功。
- 既存の大きなチャンクに関するVite警告とRustの未使用コード警告は残る。

## 検証コマンド

```sh
cargo test --manifest-path engine-rust/Cargo.toml --test kamishinka
cargo test --manifest-path engine-rust/Cargo.toml --lib --tests --no-fail-fast
bash scripts/build-wasm-speed.sh
node scripts/check-kamishinka.cjs
node scripts/check-battle-data.cjs
cd frontend
npm run build
```

ブラウザー検証はViteを起動し、Playwrightを利用する。

```sh
node frontend/scripts/check-kamishinka-ui.cjs
```

必要に応じて `PLAYWRIGHT_MODULE` に既存のPlaywrightモジュールパス、`CHROME_PATH` にChrome実行ファイル、`TEST_ORIGIN` に開発サーバーURLを指定できる。認証は隔離したブラウザー内でモックし、実アカウントやクラウドの編成情報は更新しない。

## 残っている項目

- 実機2台を使うPeerJSの接続・再接続を含む通し検証。今回は実WASMのJSON往復で形態・使用権・重複要求の検証を実施。
- 既存の `spec_priority_cases` で3件が失敗。今回の変更前のHEADコードに同じ作業中データを組み合わせた別ディレクトリでも同じ3件を再現した。対象は以下。
  - `p0_spec_sleep_wakes_and_allows_action_when_counter_reaches_zero`
  - `p1_spec_effect_status_ids_use_supported_canonical_ids`
  - `p1_spec_effect_targets_use_supported_literals`

通常種族データや既存の作業中変更は保持している。デプロイ・コミットは行っていない。

## 変更ファイル

既存のCLI・テストの一部は、新しい状態/アクションフィールドの初期値追加のみ。

- `docs/kamishinka.md`
- `engine-rust/data/items.json`
- `engine-rust/data/kamishinka.json`
- `engine-rust/pkg/engine_rust_bg.wasm`
- `engine-rust/src/ai/mcts.rs`
- `engine-rust/src/ai/minimax.rs`
- `engine-rust/src/ai/simple.rs`
- `engine-rust/src/ai/vega.rs`
- `engine-rust/src/bin/cli.rs`
- `engine-rust/src/bin/cli_test.rs`
- `engine-rust/src/bin/self_play_export.rs`
- `engine-rust/src/bin/tune_vega.rs`
- `engine-rust/src/bin/vega_bench.rs`
- `engine-rust/src/core/abilities.rs`
- `engine-rust/src/core/battle.rs`
- `engine-rust/src/core/effects.rs`
- `engine-rust/src/core/events.rs`
- `engine-rust/src/core/factory.rs`
- `engine-rust/src/core/kamishinka.rs`
- `engine-rust/src/core/mod.rs`
- `engine-rust/src/core/state.rs`
- `engine-rust/src/core/statuses.rs`
- `engine-rust/src/wasm.rs`
- `engine-rust/tests/abilities.rs`
- `engine-rust/tests/bypass.rs`
- `engine-rust/tests/choice.rs`
- `engine-rust/tests/effects.rs`
- `engine-rust/tests/experiment.rs`
- `engine-rust/tests/experiment_abilities.rs`
- `engine-rust/tests/issue_fixes.rs`
- `engine-rust/tests/kamishinka.rs`
- `engine-rust/tests/mechanics.rs`
- `engine-rust/tests/move_behavior_samples.rs`
- `engine-rust/tests/random_move.rs`
- `engine-rust/tests/spec_priority_cases.rs`
- `engine-rust/tests/substitute.rs`
- `engine-rust/tests/support/harness.rs`
- `engine-rust/tests/user_deck_regressions.rs`
- `frontend/public/data/items.json`
- `frontend/public/engine_rust_bg.wasm`
- `frontend/scripts/check-kamishinka-ui.cjs`
- `frontend/src/components/KamishinkaAnimation.tsx`
- `frontend/src/index.css`
- `frontend/src/lib/engine-rust/engine_rust_bg.wasm`
- `frontend/src/lib/engine.ts`
- `frontend/src/lib/kamishinka.ts`
- `frontend/src/lib/p2p.ts`
- `frontend/src/pages/BattlePage.tsx`
- `frontend/src/pages/DeckBuilderPage.tsx`
- `frontend/src/pages/PokemonDetailPage.tsx`
- `scripts/check-kamishinka.cjs`
- `scripts/sync-battle-data.cjs`

追加レビューと本家との差は [レビュー報告](kamishinka-review.md) を参照。

睡眠・交代順・特性・整数ダメージの追加改善は [改善報告](battle-alignment.md) を参照。最新の検証結果は234件成功・既存の技データ検査2件失敗。

カミモリミツのタイプは「みず・ひこう」。もりみつはフェザーダンスを除く登録済みのひこう技を習得可能。配布元CSVとエンジン/フロントの習得技データに反映済み。
