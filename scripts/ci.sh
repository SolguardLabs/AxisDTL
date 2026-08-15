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

BUN_BIN="$(command -v bun || command -v bun.exe || true)"
if [[ -z "$BUN_BIN" ]]; then
  echo "Bun is required but was not found in PATH" >&2
  exit 1
fi

cargo fmt --all -- --check
cargo build --all-targets --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
"$BUN_BIN" install --frozen-lockfile
"$BUN_BIN" run fmt:check
"$BUN_BIN" run build
"$BUN_BIN" test --timeout 30000 ./tests/node ./sdk ./scripts
