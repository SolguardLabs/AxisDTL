#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1 && command -v cygpath >/dev/null 2>&1 && [[ -n "${USERPROFILE:-}" ]]; then
  export PATH="$(cygpath -u "$USERPROFILE")/.cargo/bin:$PATH"
fi
if ! command -v cargo >/dev/null 2>&1 && [[ -d "${HOME:-}/.cargo/bin" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

cargo test --locked
if command -v node >/dev/null 2>&1 && node --version >/dev/null 2>&1; then
  NODE_BIN="node"
elif command -v node.exe >/dev/null 2>&1 && node.exe --version >/dev/null 2>&1; then
  NODE_BIN="node.exe"
else
  echo "node executable not found" >&2
  exit 127
fi

"$NODE_BIN" --test tests/node/*.test.js
