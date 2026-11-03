$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$cleanScript = Join-Path $repoRoot "scripts\clean.ps1"
$releaseScript = Join-Path $repoRoot "scripts\build-release.ps1"
$powerShell = Join-Path $PSHOME "powershell.exe"
$failures = 0
$tests = 0

function Assert-True {
  param(
    [bool]$Condition,
    [string]$Message
  )

  if (-not $Condition) {
    throw $Message
  }
}

function New-ProjectFixture {
  $root = Join-Path ([System.IO.Path]::GetTempPath()) ("nexora-workflow-" + [guid]::NewGuid().ToString("N"))
  $paths = @(
    "dist",
    "logs",
    "release",
    "node_modules\fixture",
    "src-tauri\target\debug",
    "src-tauri\target\release",
    "src-tauri\target\test-tmp",
    "src-tauri\target-release-old\release"
  )
  foreach ($path in $paths) {
    New-Item -ItemType Directory -Force -Path (Join-Path $root $path) | Out-Null
  }

  Set-Content -LiteralPath (Join-Path $root "dist\index.html") -Value "dist"
  Set-Content -LiteralPath (Join-Path $root "logs\old.log") -Value "old"
  Set-Content -LiteralPath (Join-Path $root "release\installer.exe") -Value "release"
  Set-Content -LiteralPath (Join-Path $root "node_modules\fixture\index.js") -Value "dependency"
  Set-Content -LiteralPath (Join-Path $root "src-tauri\target\debug\nexora.exe") -Value "debug"
  Set-Content -LiteralPath (Join-Path $root "src-tauri\target\release\nexora.exe") -Value "release"
  Set-Content -LiteralPath (Join-Path $root "src-tauri\target\test-tmp\leftover.bin") -Value "test"
  Set-Content -LiteralPath (Join-Path $root "src-tauri\target-release-old\release\nexora.exe") -Value "old"
  Set-Content -LiteralPath (Join-Path $root "tsconfig.tsbuildinfo") -Value "{}"

  Set-Content -LiteralPath (Join-Path $root "package.json") -Value '{"version":"1.2.3"}'
  Set-Content -LiteralPath (Join-Path $root "src-tauri\tauri.conf.json") -Value '{"version":"1.2.3"}'
  Set-Content -LiteralPath (Join-Path $root "src-tauri\Cargo.toml") -Value @'
[package]
name = "fixture"
version = "1.2.3"
'@

  return $root
}

function Invoke-ProjectScript {
  param(
    [string]$Script,
    [string]$ProjectRoot,
    [string[]]$Arguments = @()
  )

  $stdout = [System.IO.Path]::GetTempFileName()
  $stderr = [System.IO.Path]::GetTempFileName()
  try {
    $process = Start-Process `
      -FilePath $powerShell `
      -ArgumentList (@("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $Script, "-ProjectRoot", $ProjectRoot) + $Arguments) `
      -NoNewWindow `
      -Wait `
      -PassThru `
      -RedirectStandardOutput $stdout `
      -RedirectStandardError $stderr
    $output = @(
      Get-Content -Raw -LiteralPath $stdout -ErrorAction SilentlyContinue
      Get-Content -Raw -LiteralPath $stderr -ErrorAction SilentlyContinue
    ) | Out-String
  } finally {
    Remove-Item -LiteralPath $stdout, $stderr -Force -ErrorAction SilentlyContinue
  }

  return [PSCustomObject]@{
    ExitCode = $process.ExitCode
    Output = $output
  }
}

function Test-Case {
  param(
    [string]$Name,
    [scriptblock]$Body
  )

  $script:tests += 1
  try {
    & $Body
    Write-Host "  ok - $Name"
  } catch {
    $script:failures += 1
    Write-Host "  not ok - $Name"
    Write-Host "    $($_.Exception.Message)"
  }
}

Test-Case "release cleanup removes old builds but keeps the debug cache" {
  $fixture = New-ProjectFixture
  try {
    $result = Invoke-ProjectScript -Script $cleanScript -ProjectRoot $fixture -Arguments @("-ReleaseOnly")
    Assert-True ($result.ExitCode -eq 0) "clean.ps1 failed: $($result.Output)"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "release"))) "release staging was not removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "logs"))) "stale logs were not removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target\release"))) "release target was not removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target-release-old"))) "alternate target was not removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target\test-tmp"))) "test cache was not removed"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target\debug\nexora.exe")) "debug cache should be preserved"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "dist\index.html")) "dist should be preserved"
  } finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "debug cleanup removes only the development build" {
  $fixture = New-ProjectFixture
  try {
    $result = Invoke-ProjectScript -Script $cleanScript -ProjectRoot $fixture -Arguments @("-DebugOnly")
    Assert-True ($result.ExitCode -eq 0) "clean.ps1 failed: $($result.Output)"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target\debug"))) "debug target was not removed"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target\release\nexora.exe")) "release target should be preserved"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "dist\index.html")) "dist should be preserved"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target-release-old\release\nexora.exe")) "alternate targets should be preserved"
  } finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "full cleanup removes generated frontend and Rust outputs without deleting dependencies" {
  $fixture = New-ProjectFixture
  try {
    $result = Invoke-ProjectScript -Script $cleanScript -ProjectRoot $fixture
    Assert-True ($result.ExitCode -eq 0) "clean.ps1 failed: $($result.Output)"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "dist"))) "dist was not removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "src-tauri\target"))) "target was not removed"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture "tsconfig.tsbuildinfo"))) "tsbuildinfo was not removed"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "node_modules\fixture\index.js")) "dependencies should be preserved"
  } finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "release validation accepts matching project versions" {
  $fixture = New-ProjectFixture
  try {
    Remove-Item -LiteralPath (Join-Path $fixture "src-tauri\target-release-old") -Recurse -Force
    $result = Invoke-ProjectScript -Script $releaseScript -ProjectRoot $fixture -Arguments @("-ValidateOnly")
    Assert-True ($result.ExitCode -eq 0) "release validation failed: $($result.Output)"
    Assert-True ($result.Output.Contains("1.2.3")) "validated version was not reported"
  } finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "release validation rejects mismatched project versions" {
  $fixture = New-ProjectFixture
  try {
    Remove-Item -LiteralPath (Join-Path $fixture "src-tauri\target-release-old") -Recurse -Force
    Set-Content -LiteralPath (Join-Path $fixture "package.json") -Value '{"version":"1.2.4"}'
    $result = Invoke-ProjectScript -Script $releaseScript -ProjectRoot $fixture -Arguments @("-ValidateOnly")
    Assert-True ($result.ExitCode -ne 0) "mismatched versions were accepted"
    Assert-True ($result.Output.Contains("Version mismatch")) "version mismatch was not explained"
  } finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "release validation rejects an alternate target cache" {
  $fixture = New-ProjectFixture
  try {
    Remove-Item -LiteralPath (Join-Path $fixture "src-tauri\target-release-old") -Recurse -Force
    New-Item -ItemType Directory -Force -Path (Join-Path $fixture "src-tauri\target-release-9.9.9") | Out-Null
    $result = Invoke-ProjectScript -Script $releaseScript -ProjectRoot $fixture -Arguments @("-ValidateOnly")
    Assert-True ($result.ExitCode -ne 0) "alternate target cache was accepted"
    Assert-True ($result.Output.Contains("Alternate target")) "alternate target failure was not explained"
  } finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "release packaging uses canonical artifacts and writes checksums" {
  $fixture = New-ProjectFixture
  try {
    Remove-Item -LiteralPath (Join-Path $fixture "src-tauri\target-release-old") -Recurse -Force
    $nsis = Join-Path $fixture "src-tauri\target\release\bundle\nsis"
    New-Item -ItemType Directory -Force -Path $nsis | Out-Null
    Set-Content -LiteralPath (Join-Path $nsis "NEXORA_1.2.3_x64-setup.exe") -Value "installer"
    $result = Invoke-ProjectScript -Script $releaseScript -ProjectRoot $fixture -Arguments @("-SkipBuild")

    Assert-True ($result.ExitCode -eq 0) "release packaging failed: $($result.Output)"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "release\NEXORA_1.2.3_x64-setup.exe")) "installer was not staged"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "release\NEXORA_1.2.3_portable.zip")) "portable archive was not created"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture "release\SHA256SUMS.txt")) "checksums were not created"
  } finally {
    Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction SilentlyContinue
  }
}

if ($failures -gt 0) {
  Write-Error "$failures of $tests workflow tests failed"
  exit 1
}

Write-Host "$tests workflow tests passed"
