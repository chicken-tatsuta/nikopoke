#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE_DIR="$REPO_ROOT/engine-rust"
WASM_IN="$ENGINE_DIR/target/wasm32-unknown-unknown/wasm-speed/engine_rust.wasm"
PKG_DIR="$ENGINE_DIR/pkg"
FRONTEND_ENGINE_DIR="$REPO_ROOT/frontend/src/lib/engine-rust"

cd "$ENGINE_DIR"
cargo build --lib --target wasm32-unknown-unknown --profile wasm-speed

wasm-bindgen "$WASM_IN" \
  --out-dir "$PKG_DIR" \
  --typescript \
  --target web

if command -v wasm-opt >/dev/null 2>&1; then
  WASM_OPT_OUT="$PKG_DIR/engine_rust_bg.opt.wasm"
  wasm-opt \
    -O3 \
    --enable-bulk-memory \
    --enable-bulk-memory-opt \
    --enable-nontrapping-float-to-int \
    --quiet \
    "$PKG_DIR/engine_rust_bg.wasm" \
    -o "$WASM_OPT_OUT"
  mv "$WASM_OPT_OUT" "$PKG_DIR/engine_rust_bg.wasm"
else
  echo "wasm-opt not found; skipped binaryen optimization." >&2
fi

cat > "$PKG_DIR/package.json" <<'JSON'
{
  "name": "engine-rust",
  "type": "module",
  "version": "0.1.0",
  "files": [
    "engine_rust_bg.wasm",
    "engine_rust.js",
    "engine_rust.d.ts"
  ],
  "main": "engine_rust.js",
  "types": "engine_rust.d.ts",
  "sideEffects": [
    "./snippets/*"
  ]
}
JSON

cp "$PKG_DIR/engine_rust.js" "$FRONTEND_ENGINE_DIR/engine_rust.js"
cp "$PKG_DIR/engine_rust.d.ts" "$FRONTEND_ENGINE_DIR/engine_rust.d.ts"
cp "$PKG_DIR/engine_rust_bg.wasm" "$FRONTEND_ENGINE_DIR/engine_rust_bg.wasm"
cp "$PKG_DIR/engine_rust_bg.wasm.d.ts" "$FRONTEND_ENGINE_DIR/engine_rust_bg.wasm.d.ts"
cp "$PKG_DIR/package.json" "$FRONTEND_ENGINE_DIR/package.json"
cp "$PKG_DIR/engine_rust_bg.wasm" "$REPO_ROOT/frontend/public/engine_rust_bg.wasm"
