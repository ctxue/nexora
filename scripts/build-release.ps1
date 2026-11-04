[CmdletBinding()]
param(
  [string]$ProjectRoot = "",
  [string]$ExpectedVersion,
  [switch]$ValidateOnly,
  [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
trap {
  Write-Error $_
  exit 1
}
if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
  $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$root = [System.IO.Path]::GetFullPath($ProjectRoot).TrimEnd("\", "/")
$srcTauri = Join-Path $root "src-tauri"

function Read-PackageVersion {
  $package = Get-Content -Raw -LiteralPath (Join-Path $root "package.json") | ConvertFrom-Json
  return [string]($package.version)
}

function Read-TauriVersion {
  $config = Get-Content -Raw -LiteralPath (Join-Path $srcTauri "tauri.conf.json") | ConvertFrom-Json
  return [string]($config.version)
}

function Read-CargoVersion {
  $cargo = Get-Content -Raw -LiteralPath (Join-Path $srcTauri "Cargo.toml")
  $packageSection = [regex]::Match(
    $cargo,
    "(?ms)^\s*\[package\]\s*(.*?)(?=^\s*\[|\z)"
  )
  if (-not $packageSection.Success) {
    throw "Cargo.toml does not contain a [package] section"
  }

  $version = [regex]::Match($packageSection.Groups[1].Value, '(?m)^\s*version\s*=\s*"([^"]+)"')
  if (-not $version.Success) {
    throw "Cargo.toml [package] section does not contain version"
  }
  return $version.Groups[1].Value
}

function Get-Sha256 {
  param([string]$Path)

  $sha = [System.Security.Cryptography.SHA256]::Create()
  $stream = [System.IO.File]::OpenRead($Path)
  try {
    return ([System.BitConverter]::ToString($sha.ComputeHash($stream))).Replace("-", "")
  } finally {
    $stream.Dispose()
    $sha.Dispose()
  }
}

if (-not (Test-Path -LiteralPath (Join-Path $root "package.json"))) {
  throw "ProjectRoot does not contain package.json: $root"
}
if (-not (Test-Path -LiteralPath (Join-Path $srcTauri "Cargo.toml"))) {
  throw "ProjectRoot does not contain src-tauri\Cargo.toml: $root"
}

$packageVersion = Read-PackageVersion
$cargoVersion = Read-CargoVersion
$tauriVersion = Read-TauriVersion
$versions = @(@($packageVersion, $cargoVersion, $tauriVersion) | Select-Object -Unique)

if ($versions.Count -ne 1) {
  throw "Version mismatch: package.json=$packageVersion Cargo.toml=$cargoVersion tauri.conf.json=$tauriVersion"
}
if ($ExpectedVersion -and $versions[0] -ne $ExpectedVersion) {
  throw "Version mismatch: project=$($versions[0]) expected=$ExpectedVersion"
}

$alternates = @(
  Get-ChildItem -LiteralPath $srcTauri -Directory -Filter "target-*" -ErrorAction SilentlyContinue
)
if ($alternates.Count -gt 0) {
  $names = ($alternates | Select-Object -ExpandProperty Name) -join ", "
  throw "Alternate target directories found: $names. Run pnpm clean:release or use pnpm release:build."
}

Write-Output "Validated NEXORA version $($versions[0])."
if ($ValidateOnly) {
  return
}

if (-not $SkipBuild) {
  $cleanScript = Join-Path $PSScriptRoot "clean.ps1"
  & $cleanScript -ProjectRoot $root -ReleaseOnly

  $canonicalTarget = Join-Path $srcTauri "target"
  $env:CARGO_TARGET_DIR = $canonicalTarget
  Write-Host "Building with CARGO_TARGET_DIR=$canonicalTarget"

  Push-Location $root
  try {
    & pnpm tauri build
    if ($LASTEXITCODE -ne 0) {
      throw "pnpm tauri build failed with exit code $LASTEXITCODE"
    }
  } finally {
    Pop-Location
  }
}

$version = $versions[0]
$releaseDir = Join-Path $root "release"
$executable = Join-Path $srcTauri "target\release\nexora.exe"
$installer = Join-Path $srcTauri "target\release\bundle\nsis\NEXORA_${version}_x64-setup.exe"
$portable = Join-Path $releaseDir "NEXORA_${version}_portable.zip"
$installerCopy = Join-Path $releaseDir "NEXORA_${version}_x64-setup.exe"

foreach ($artifact in @($executable, $installer)) {
  if (-not (Test-Path -LiteralPath $artifact)) {
    throw "Release artifact not found: $artifact"
  }
}

if (Test-Path -LiteralPath $releaseDir) {
  Remove-Item -LiteralPath $releaseDir -Recurse -Force
}
New-Item -ItemType Directory -Path $releaseDir | Out-Null

Compress-Archive -LiteralPath $executable -DestinationPath $portable -CompressionLevel Optimal
Copy-Item -LiteralPath $installer -Destination $installerCopy

$checksums = @($installerCopy, $portable) | ForEach-Object {
  "$(Get-Sha256 -Path $_)  $([System.IO.Path]::GetFileName($_))"
}
$checksums | Set-Content -LiteralPath (Join-Path $releaseDir "SHA256SUMS.txt") -Encoding ASCII

Write-Host "Release $version packaged in $releaseDir"
