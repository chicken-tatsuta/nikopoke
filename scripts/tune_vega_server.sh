#!/usr/bin/env bash
# sakura-v100 でVegaパラメーターチューニングを長時間実行するスクリプト
# 使い方: bash scripts/tune_vega_server.sh

set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Vega Tuning on Server ==="
echo "Repo: $REPO_ROOT"

# リリースビルド
echo "Building tune-vega (release)..."
cd engine-rust
cargo build --release --bin tune-vega
cd "$REPO_ROOT"

BINARY="$REPO_ROOT/engine-rust/target/release/tune-vega"
LOG_FILE="$REPO_ROOT/tune_vega_$(date +%Y%m%d_%H%M%S).log"

echo "Binary: $BINARY"
echo "Log:    $LOG_FILE"
echo ""

# パラメーター設定（長時間版）
# iterations: 500   ← デフォルト80の約6倍
# eval-games: 300   ← デフォルト80の約4倍
# games: 50         ← クイック評価もやや増量
# branch-limit: 3   ← 探索幅をやや拡大
ARGS="--iterations=500 --eval-games=300 --games=50 --depth=2 --branch-limit=3 --baseline-policy=default-vega"

if command -v tmux &>/dev/null; then
    SESSION="tune_vega"
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    tmux new-session -d -s "$SESSION" "$BINARY $ARGS 2>&1 | tee $LOG_FILE"
    echo "Started in tmux session '$SESSION'"
    echo "  Attach:  tmux attach -t $SESSION"
    echo "  Detach:  Ctrl-b d"
    echo "  Kill:    tmux kill-session -t $SESSION"
else
    echo "tmux not found. Running with nohup..."
    nohup "$BINARY" $ARGS >"$LOG_FILE" 2>&1 &
    echo "PID: $! (log: $LOG_FILE)"
fi

echo ""
echo "Tail log with: tail -f $LOG_FILE"
