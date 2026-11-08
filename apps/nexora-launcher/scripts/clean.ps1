[CmdletBinding()]
param(
  [string]$ProjectRoot = "",
  [switch]$DebugOnly,
  [switch]$ReleaseOnly,
  [switch]$IncludeNodeModules
)

$ErrorActionPreference = "Stop"
trap {
  Write-Error $_
  exit 1
}
if ([string]::IsNullOrWhiteSpace($ProjectRoot)) {
  $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
if ($DebugOnly -and $ReleaseOnly) {
  throw "DebugOnly and ReleaseOnly cannot be used together."
}
$root = [System.IO.Path]::GetFullPath($ProjectRoot).TrimEnd("\", "/")
$rootPrefix = $root + [System.IO.Path]::DirectorySeparatorChar

if (-not (Test-Path -LiteralPath (Join-Path $root "package.json"))) {
  throw "ProjectRoot does not contain package.json: $root"
}
if (-not (Test-Path -LiteralPath (Join-Path $root "src-tauri\Cargo.toml"))) {
  throw "ProjectRoot does not contain src-tauri\Cargo.toml: $root"
}

function Remove-GeneratedPath {
  param([string]$Path)

  $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd("\", "/")
  if (-not $fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove a path outside the project: $fullPath"
  }
  if (-not (Test-Path -LiteralPath $fullPath)) {
    return
  }

  Write-Host "Removing $fullPath"
  Remove-Item -LiteralPath $fullPath -Recurse -Force
}

$srcTauri = Join-Path $root "src-tauri"
$targets = @()

if ($DebugOnly) {
  $targets += Join-Path $srcTauri "target\debug"
} elseif ($ReleaseOnly) {
  $targets += Join-Path $root "logs"
  $targets += Join-Path $root "release"
  $targets += Join-Path $srcTauri "target\release"
  $targets += Join-Path $srcTauri "target\test-tmp"
} else {
  $targets += Join-Path $root "dist"
  $targets += Join-Path $root "logs"
  $targets += Join-Path $root "release"
  $targets += Join-Path $root "tsconfig.tsbuildinfo"
  $targets += Join-Path $srcTauri "target"
}

if (-not $DebugOnly) {
  $targets += Get-ChildItem -LiteralPath $srcTauri -Directory -Filter "target-*" -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty FullName
}

if (-not $DebugOnly -and $IncludeNodeModules) {
  $targets += Join-Path $root "node_modules"
}

$targets | Select-Object -Unique | ForEach-Object {
  Remove-GeneratedPath -Path $_
}

Write-Host "Generated artifacts cleaned."
