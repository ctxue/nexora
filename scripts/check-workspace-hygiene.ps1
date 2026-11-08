[CmdletBinding()]
param([string]$Root)

$ErrorActionPreference = "Stop"
if (-not $Root) {
  $Root = Split-Path -Parent $PSScriptRoot
}
$root = [IO.Path]::GetFullPath($Root).TrimEnd("\", "/")
if (-not (Test-Path -LiteralPath $root)) {
  throw "Workspace root does not exist: $root"
}

$legacyPattern = "(?i)nexora[-_. ]?halo"
$pending = [Collections.Generic.Stack[IO.DirectoryInfo]]::new()
$pending.Push([IO.DirectoryInfo]::new($root))
$legacyMatches = [Collections.Generic.List[string]]::new()

while ($pending.Count -gt 0) {
  $directory = $pending.Pop()
  if ($directory.Name -eq ".git") {
    continue
  }
  foreach ($item in Get-ChildItem -LiteralPath $directory.FullName -Force) {
    if ($item.Name -match $legacyPattern) {
      $legacyMatches.Add($item.FullName)
    }
    if (
      $item.PSIsContainer -and
      -not ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
    ) {
      $pending.Push([IO.DirectoryInfo]$item)
    }
  }
}

if ($legacyMatches.Count -gt 0) {
  Write-Output "Stale product artifacts found:"
  foreach ($match in $legacyMatches) {
    Write-Output $match
  }
  exit 1
}

Write-Output "Workspace hygiene check passed."
