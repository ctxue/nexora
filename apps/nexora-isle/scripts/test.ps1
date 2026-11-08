$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  & cargo +stable fmt --all -- --check
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & (Join-Path $root 'tests\installer_layout.tests.ps1')
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & cargo +stable test --locked --target x86_64-pc-windows-msvc -- --test-threads=1
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
