[CmdletBinding()]
param(
  [Parameter(Mandatory = $true, Position = 0)]
  [ValidateSet("dev", "test", "debug", "package")]
  [string]$Action,
  [string[]]$Product = @(),
  [switch]$All,
  [string]$RepoRoot = "",
  [string]$ManifestPath = "",
  [string]$ReleaseDirectory = ""
)

$ErrorActionPreference = "Stop"
trap {
  Write-Error $_
  exit 1
}

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
  $RepoRoot = Split-Path -Parent $PSScriptRoot
}
$root = [IO.Path]::GetFullPath($RepoRoot).TrimEnd("\", "/")
if ([string]::IsNullOrWhiteSpace($ManifestPath)) {
  $ManifestPath = Join-Path $root "products.psd1"
}
$ManifestPath = [IO.Path]::GetFullPath($ManifestPath)
if (-not (Test-Path -LiteralPath $ManifestPath)) {
  throw "Product manifest not found: $ManifestPath"
}

$manifest = & ([ScriptBlock]::Create((Get-Content -Raw -LiteralPath $ManifestPath)))
$products = @($manifest.Products)
if ($products.Count -eq 0) {
  throw "Product manifest contains no products: $ManifestPath"
}

$seen = @{}
foreach ($entry in $products) {
  if ([string]::IsNullOrWhiteSpace($entry.Id) -or [string]::IsNullOrWhiteSpace($entry.Directory)) {
    throw "Every product requires Id and Directory."
  }
  if ($seen.ContainsKey($entry.Id)) {
    throw "Duplicate product Id: $($entry.Id)"
  }
  if (-not $entry.Scripts.ContainsKey($Action)) {
    throw "Product '$($entry.Id)' has no '$Action' script."
  }
  $seen[$entry.Id] = $entry
}

$available = @($products | ForEach-Object { $_.Id })
if ($All -and $Product.Count -gt 0) {
  throw "-All and -Product cannot be used together."
}
if ($All -or $Product.Count -eq 0) {
  $selected = $products
} else {
  $unknown = @($Product | Where-Object { -not $seen.ContainsKey($_) })
  if ($unknown.Count -gt 0) {
    throw "Unknown product: $($unknown -join ', '). Available products: $($available -join ', ')."
  }
  $selected = @($Product | ForEach-Object { $seen[$_] })
}

if ([string]::IsNullOrWhiteSpace($ReleaseDirectory)) {
  $ReleaseDirectory = Join-Path $root "release"
}
$release = [IO.Path]::GetFullPath($ReleaseDirectory).TrimEnd("\", "/")
$rootPrefix = $root + [IO.Path]::DirectorySeparatorChar
if ($Action -eq "package" -and $selected.Count -gt 1) {
  if (-not $release.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean a release directory outside the repository: $release"
  }
  if (Test-Path -LiteralPath $release) {
    Remove-Item -LiteralPath $release -Recurse -Force
  }
}

foreach ($entry in $selected) {
  $productRoot = [IO.Path]::GetFullPath((Join-Path $root $entry.Directory))
  if (-not $productRoot.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Product directory escaped the repository: $($entry.Directory)"
  }
  if (-not (Test-Path -LiteralPath $productRoot -PathType Container)) {
    throw "Product directory not found: $productRoot"
  }

  $scriptPath = [IO.Path]::GetFullPath((Join-Path $productRoot $entry.Scripts[$Action]))
  if (-not (Test-Path -LiteralPath $scriptPath -PathType Leaf)) {
    throw "Product '$($entry.Id)' $Action script not found: $scriptPath"
  }

  $before = @{}
  if ($Action -eq "package") {
    New-Item -ItemType Directory -Path $release -Force | Out-Null
    foreach ($file in @(Get-ChildItem -LiteralPath $release -File -ErrorAction SilentlyContinue)) {
      $before[$file.Name] = $file.LastWriteTimeUtc
    }
  }

  Write-Host "==> [$($entry.Id)] $Action ($productRoot)"
  $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $scriptPath)
  if ($Action -eq "package") {
    $arguments += @("-OutputDirectory", $release)
  }
  & (Join-Path $PSHOME "powershell.exe") @arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Product '$($entry.Id)' $Action failed with exit code $LASTEXITCODE."
  }

  if ($Action -eq "package") {
    $after = @(Get-ChildItem -LiteralPath $release -File)
    $newArtifacts = @($after | Where-Object {
      -not $before.ContainsKey($_.Name) -or $_.LastWriteTimeUtc -gt $before[$_.Name]
    })
    if ($newArtifacts.Count -ne 2) {
      throw "Product '$($entry.Id)' must produce exactly two release files; found $($newArtifacts.Count): $(($newArtifacts | Select-Object -ExpandProperty Name) -join ', ')"
    }
  }
}

if ($Action -eq "package") {
  $files = @(Get-ChildItem -LiteralPath $release -File | Sort-Object Name)
  if ($selected.Count -gt 1 -and $files.Count -ne $selected.Count * 2) {
    throw "Combined release must contain exactly $($selected.Count * 2) files; found $($files.Count)."
  }
  Write-Host "Release files in $release"
  $files | ForEach-Object { Write-Host "  $($_.Name)" }
}
