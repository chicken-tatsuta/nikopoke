# 対戦処理の6項目の改善

レシーバー・かがくのちからはユーザー指定に従い、登場時に直前の味方の特性を引き継ぐ独自仕様を維持した。それ以外の6項目を修正した。

## 変更内容

| 項目 | 改善後の動作 |
| --- | --- |
| 両者の交代順 | 交代元の有効素早さを既存の能力ランク・持ち物・麻痺・特性補正込みで取得して比較。同速時のみ乱数を使い、トリックルーム中は順序を反転する |
| 特性変更 | 旧特性の専用状態を終了し、交代時の復元用特性を保持。SetAbility/SwapAbilitiesで変更をまとめて反映した後、共通の開始フックを呼ぶ。進化・ガス解除も開始フックを使用。登場時専用の独自引き継ぎを開始フックから再実行しない |
| 特性無視技 | 無視するのは防御側の補正。攻撃者自身の威力補正・特攻補正・てきおうりょくは有効。イベント経由の防御特性にも防御側無視フラグを渡す |
| 睡眠 | 通常の付与時にカウンターを決め、技を出そうとしたときだけ減算。ターン終了・控えへの交代では減算しない。はやおきは2ずつ減算。ねむるは2回行動不能の後、3回目に起床。旧elapsed形式は読み込み時の互換処理を用意 |
| ダメージ | 共通の整数計算モジュールを追加。基礎式の整数除算、天候→急所→乱数→一致→相性→やけど→最終補正の段階を分離。補正は4096分率と五捨五超入（ちょうど0.5は切り捨て）を使用。あついしぼうは威力でなく攻撃側実数値への補正に移動 |
| どくしゅ | 追加効果防止の共通判定で、有効なりんぷん・おんみつマント所持時は毒付与を阻止。ガスで無効化されたりんぷんは阻止しない |

**意図的な結果変更**：交代順、起床するターン、特性変更直後の発動、整数の端数があるダメージは、修正前と結果が変わり得る。独自EV・カミシンカ時のHP維持・1人1回制限は維持。

既存のフック/技DSLが扱う補正を段階化したものであり、未実装の技・特性・全世代の特殊例まで追加したわけではない。どくしゅの防止判定は既存の特性ID/持ち物IDを受け付ける層への対応であり、現在データに無い通常種族・持ち物の新規登録は含めていない。

## 主な変更ファイル

- `engine-rust/src/core/battle.rs`：交代順、特性変更後の開始処理
- `engine-rust/src/core/abilities.rs`：特性状態の終了・変更・開始、どくしゅの共通防止判定
- `engine-rust/src/core/events.rs`：特性変更記録、元の特性が無かった場合の復元、睡眠カウンター
- `engine-rust/src/core/statuses.rs`：睡眠の減算・起床・旧形式互換
- `engine-rust/src/core/effects.rs`：睡眠付与、攻防の特性無視の分離、段階的ダメージ計算
- `engine-rust/src/core/damage.rs`：新規の共通整数計算
- `engine-rust/src/core/mod.rs`：共通整数計算の公開
- `engine-rust/src/ai/vega.rs`：新しい睡眠・特性タイマーを盤面キャッシュの識別に反映
- `engine-rust/tests/kamishinka.rs`：回帰テスト追加
- エンジン/フロントの配信用WASM 3ファイル：再生成

## 検証

- Rust全体：234件成功、2件失敗。今回の変更による新規失敗なし。
- カミシンカおよび今回の回帰テスト：25件すべて成功。
- 従来失敗していた睡眠カウンターのテストは成功へ改善。
- 残る2件は以前からある技データの検査：`p1_spec_effect_status_ids_use_supported_canonical_ids` / `p1_spec_effect_targets_use_supported_literals`。
- WASM再生成、実WASMでのJSON往復・使用権・リセット検査、456技/24種族のデータ読み込み検査に成功。
- TypeScript/Vite本番ビルド成功。
- ブラウザーでカミダマ複数所持、予約/解除、演出中の入力ロック、形態変更、交代・再登場、390px幅の表示を確認。

## 参照

比較には [Pokémon Showdownの状態異常](https://github.com/smogon/pokemon-showdown/blob/master/data/conditions.ts)、[ダメージ処理](https://github.com/smogon/pokemon-showdown/blob/master/sim/battle-actions.ts)、[特性](https://github.com/smogon/pokemon-showdown/blob/master/data/abilities.ts)、[有効素早さ](https://github.com/smogon/pokemon-showdown/blob/master/sim/pokemon.ts) の一次実装を使用した。
