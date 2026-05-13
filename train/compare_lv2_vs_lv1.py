#!/usr/bin/env python3
"""MLP AI (LV2) vs LV1 Minimax 対戦比較スクリプト"""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

TRAIN_DIR = Path(__file__).resolve().parent
BINARY_PATH = TRAIN_DIR.parent / "engine-rust/target/release/self-play-export"
WEIGHTS_PATH = TRAIN_DIR.parent / "frontend/public/ai_weights.json"
TEAM_PATH = TRAIN_DIR.parent / "frontend/public/ai_team.json"

GAMES = 100
SEED_BASE = 12345


def load_weights(path):
    with path.open(encoding="utf-8") as f:
        return json.load(f)


def load_team(path):
    if path.exists():
        with path.open(encoding="utf-8") as f:
            return json.load(f)
    return None


def run_batch(matches):
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as tmp:
        json.dump(matches, tmp, separators=(",", ":"))
        tmp_path = tmp.name

    result = subprocess.run(
        [str(BINARY_PATH), "--batch", tmp_path],
        check=True,
        capture_output=True,
        text=True,
        timeout=600,
    )
    return json.loads(result.stdout)


def main():
    if not BINARY_PATH.exists():
        print(f"ERROR: バイナリが見つかりません: {BINARY_PATH}", file=sys.stderr)
        sys.exit(1)

    if not WEIGHTS_PATH.exists():
        print(f"ERROR: 重みファイルが見つかりません: {WEIGHTS_PATH}", file=sys.stderr)
        sys.exit(1)

    weights = load_weights(WEIGHTS_PATH)
    team = load_team(TEAM_PATH)
    print(f"重みファイル: {WEIGHTS_PATH} ({WEIGHTS_PATH.stat().st_size // 1024}KB)")
    print(f"チーム: {TEAM_PATH} ({'読み込み済み' if team else 'なし → ランダムチーム使用'})")

    # 100試合を10バッチ(各10試合)に分割して実行
    batch_size = 10
    matches = [
        {
            "weights_a": weights,
            "weights_b": weights,  # bはbaseline使用時は無視される
            "team_a": team,
            "team_b": team,
            "games": batch_size,
            "seed": SEED_BASE + i * batch_size,
            "baseline_a": False,  # LV2 MLP
            "baseline_b": True,   # LV1 Minimax
        }
        for i in range(GAMES // batch_size)
    ]

    print(f"\n{GAMES}試合実行中 (MLP LV2 vs Minimax LV1)...")
    results = run_batch(matches)

    total_wins_mlp = sum(r["wins_a"] for r in results)
    total_wins_lv1 = sum(r["wins_b"] for r in results)
    total_draws = sum(r["draws"] for r in results)
    total = total_wins_mlp + total_wins_lv1 + total_draws

    print("\n" + "=" * 50)
    print("        対戦結果サマリー")
    print("=" * 50)
    print(f"  MLP LV2 (学習済み) 勝利: {total_wins_mlp:3d} / {total}  ({100*total_wins_mlp/total:.1f}%)")
    print(f"  Minimax LV1        勝利: {total_wins_lv1:3d} / {total}  ({100*total_wins_lv1/total:.1f}%)")
    print(f"  引き分け:               {total_draws:3d} / {total}  ({100*total_draws/total:.1f}%)")
    print("=" * 50)

    if total_wins_mlp > total_wins_lv1:
        margin = total_wins_mlp - total_wins_lv1
        print(f"\nMLP LV2 が LV1 に勝ち越し (+{margin}勝)")
    elif total_wins_lv1 > total_wins_mlp:
        margin = total_wins_lv1 - total_wins_mlp
        print(f"\nMinimax LV1 が MLP LV2 に勝ち越し (+{margin}勝)")
    else:
        print("\n五分五分")


if __name__ == "__main__":
    main()
