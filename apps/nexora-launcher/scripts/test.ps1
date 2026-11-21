$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  & (Join-Path $root 'node_modules\.bin\vitest.cmd') run
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & (Join-Path $PSHOME 'powershell.exe') -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\tests\release-workflow.tests.ps1')
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & cargo test --manifest-path src-tauri/Cargo.toml
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
