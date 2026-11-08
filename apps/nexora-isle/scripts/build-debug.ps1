$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  & cargo build --bin nexora-isle
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  Write-Output (Join-Path $root "target\debug\nexora-isle.exe")
} finally {
  Pop-Location
}
