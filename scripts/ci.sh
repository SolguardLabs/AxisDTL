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

cargo fmt --all -- --check
cargo check --locked
bash scripts/tests.sh
