$ErrorActionPreference = "Stop"
Set-Location (Resolve-Path "$PSScriptRoot\..")

cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --all-targets --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo clippy --all-targets --all-features --locked -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
bun install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
bun run fmt:check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
bun run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
bun test --timeout 30000 ./tests/node ./sdk ./scripts
exit $LASTEXITCODE
