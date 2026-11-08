[CmdletBinding()]
param([switch]$IncludeDependencies)

$ErrorActionPreference = "Stop"
$root = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot)).TrimEnd("\", "/")
$prefix = $root + [IO.Path]::DirectorySeparatorChar

function Remove-GeneratedPath {
  param([string]$Path)
  $full = [IO.Path]::GetFullPath($Path).TrimEnd("\", "/")
  if (-not $full.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove path outside the workspace: $full"
  }
  if (Test-Path -LiteralPath $full) {
    Remove-Item -LiteralPath $full -Recurse -Force
  }
}

Remove-GeneratedPath (Join-Path $root "release")
Remove-GeneratedPath (Join-Path $root "logs")
foreach ($target in @(Get-ChildItem -LiteralPath (Join-Path $root "apps") -Directory | ForEach-Object {
  @(
    (Join-Path $_.FullName "dist"),
    (Join-Path $_.FullName "target"),
    (Join-Path $_.FullName "src-tauri\target"),
    (Join-Path $_.FullName "tsconfig.tsbuildinfo")
  )
})) {
  Remove-GeneratedPath $target
}
if ($IncludeDependencies) {
  Remove-GeneratedPath (Join-Path $root "node_modules")
  Get-ChildItem -LiteralPath (Join-Path $root "apps") -Directory | ForEach-Object {
    Remove-GeneratedPath (Join-Path $_.FullName "node_modules")
  }
}
