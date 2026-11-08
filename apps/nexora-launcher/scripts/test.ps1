$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  & pnpm test
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & pnpm test:scripts
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & cargo test --manifest-path src-tauri/Cargo.toml
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
