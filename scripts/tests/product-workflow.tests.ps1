$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$runner = Join-Path $repoRoot "scripts\products.ps1"
$hygiene = Join-Path $repoRoot "scripts\check-workspace-hygiene.ps1"
$powerShell = Join-Path $PSHOME "powershell.exe"
$tests = 0
$failures = 0

function Assert-True {
  param([bool]$Condition, [string]$Message)
  if (-not $Condition) { throw $Message }
}

function New-ProductFixture {
  $root = Join-Path ([IO.Path]::GetTempPath()) ("nexora-products-" + [guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Path $root | Out-Null

  $entries = foreach ($id in @("alpha", "beta", "gamma")) {
    $directory = Join-Path $root "apps\$id"
    $scripts = Join-Path $directory "scripts"
    New-Item -ItemType Directory -Path $scripts -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $scripts "dev.ps1") -Value "Set-Content -LiteralPath ""`$PSScriptRoot\dev-ran.txt"" -Value dev"
    Set-Content -LiteralPath (Join-Path $scripts "debug.ps1") -Value "Set-Content -LiteralPath ""`$PSScriptRoot\debug-ran.txt"" -Value debug"
    Set-Content -LiteralPath (Join-Path $scripts "test.ps1") -Value "Set-Content -LiteralPath ""`$PSScriptRoot\test-ran.txt"" -Value test"
    Set-Content -LiteralPath (Join-Path $scripts "package.ps1") -Value @"
param([string]`$OutputDirectory)
New-Item -ItemType Directory -Path `$OutputDirectory -Force | Out-Null
Set-Content -LiteralPath (Join-Path `$OutputDirectory '$id`_setup.exe') -Value setup
Set-Content -LiteralPath (Join-Path `$OutputDirectory '$id`_portable.zip') -Value zip
"@
    @"
@{
  Id = '$id'
  Directory = 'apps/$id'
  Scripts = @{
    Dev = 'scripts/dev.ps1'
    Debug = 'scripts/debug.ps1'
    Test = 'scripts/test.ps1'
    Package = 'scripts/package.ps1'
  }
}
"@
  }

  $manifest = Join-Path $root "products.psd1"
  "@{ Products = @(" + ($entries -join ",") + ") }" |
    Set-Content -LiteralPath $manifest
  return [pscustomobject]@{ Root = $root; Manifest = $manifest }
}

function Invoke-Runner {
  param([string[]]$Arguments)
  $stdout = [IO.Path]::GetTempFileName()
  $stderr = [IO.Path]::GetTempFileName()
  try {
    $process = Start-Process -FilePath $powerShell `
      -ArgumentList (@("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner) + $Arguments) `
      -NoNewWindow -Wait -PassThru `
      -RedirectStandardOutput $stdout `
      -RedirectStandardError $stderr
    return [pscustomobject]@{
      ExitCode = $process.ExitCode
      Output = ((Get-Content -Raw -LiteralPath $stdout -ErrorAction SilentlyContinue) + (Get-Content -Raw -LiteralPath $stderr -ErrorAction SilentlyContinue))
    }
  } finally {
    Remove-Item -LiteralPath $stdout, $stderr -Force -ErrorAction SilentlyContinue
  }
}

function Invoke-Script {
  param([string]$Script, [string[]]$Arguments)
  $stdout = [IO.Path]::GetTempFileName()
  $stderr = [IO.Path]::GetTempFileName()
  try {
    $process = Start-Process -FilePath $powerShell `
      -ArgumentList (@("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $Script) + $Arguments) `
      -NoNewWindow -Wait -PassThru `
      -RedirectStandardOutput $stdout `
      -RedirectStandardError $stderr
    return [pscustomobject]@{
      ExitCode = $process.ExitCode
      Output = ((Get-Content -Raw -LiteralPath $stdout -ErrorAction SilentlyContinue) + (Get-Content -Raw -LiteralPath $stderr -ErrorAction SilentlyContinue))
    }
  } finally {
    Remove-Item -LiteralPath $stdout, $stderr -Force -ErrorAction SilentlyContinue
  }
}

function Test-Case {
  param([string]$Name, [scriptblock]$Body)
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

Test-Case "unknown product reports the manifest entries" {
  $fixture = New-ProductFixture
  try {
    $result = Invoke-Runner @("-Action", "debug", "-Product", "missing", "-RepoRoot", $fixture.Root, "-ManifestPath", $fixture.Manifest)
    Assert-True ($result.ExitCode -ne 0) "unknown product was accepted"
    Assert-True ($result.Output.Contains("alpha")) "available products were not listed"
    Assert-True ($result.Output.Contains("gamma")) "later product entries were not listed"
  } finally {
    Remove-Item -LiteralPath $fixture.Root -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "debug invokes every product-selected script independently" {
  $fixture = New-ProductFixture
  try {
    $result = Invoke-Runner @("-Action", "debug", "-Product", "beta", "-RepoRoot", $fixture.Root, "-ManifestPath", $fixture.Manifest)
    Assert-True ($result.ExitCode -eq 0) "debug runner failed: $($result.Output)"
    Assert-True (Test-Path -LiteralPath (Join-Path $fixture.Root "apps\beta\scripts\debug-ran.txt")) "selected debug script did not run"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $fixture.Root "apps\alpha\scripts\debug-ran.txt"))) "unselected product ran"
  } finally {
    Remove-Item -LiteralPath $fixture.Root -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "package all is manifest-driven and cleans stale release files" {
  $fixture = New-ProductFixture
  try {
    $release = Join-Path $fixture.Root "release"
    New-Item -ItemType Directory -Path $release | Out-Null
    Set-Content -LiteralPath (Join-Path $release "stale.exe") -Value stale
    $result = Invoke-Runner @("-Action", "package", "-All", "-RepoRoot", $fixture.Root, "-ManifestPath", $fixture.Manifest, "-ReleaseDirectory", $release)
    Assert-True ($result.ExitCode -eq 0) "package all failed: $($result.Output)"
    $files = @(Get-ChildItem -LiteralPath $release -File)
    Assert-True ($files.Count -eq 6) "expected two artifacts for each of three products, got $($files.Count)"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $release "stale.exe"))) "stale release file was not removed"
  } finally {
    Remove-Item -LiteralPath $fixture.Root -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "packaging the same product repeatedly replaces its two artifacts" {
  $fixture = New-ProductFixture
  try {
    $release = Join-Path $fixture.Root "release"
    $first = Invoke-Runner @("-Action", "package", "-Product", "alpha", "-RepoRoot", $fixture.Root, "-ManifestPath", $fixture.Manifest, "-ReleaseDirectory", $release)
    Assert-True ($first.ExitCode -eq 0) "first package failed: $($first.Output)"
    $second = Invoke-Runner @("-Action", "package", "-Product", "alpha", "-RepoRoot", $fixture.Root, "-ManifestPath", $fixture.Manifest, "-ReleaseDirectory", $release)
    Assert-True ($second.ExitCode -eq 0) "repeat package failed: $($second.Output)"
    Assert-True (@(Get-ChildItem -LiteralPath $release -File).Count -eq 2) "repeat package changed the artifact count"
  } finally {
    Remove-Item -LiteralPath $fixture.Root -Recurse -Force -ErrorAction SilentlyContinue
  }
}

Test-Case "workspace hygiene rejects stale product artifact names" {
  $root = Join-Path ([IO.Path]::GetTempPath()) ("nexora-hygiene-" + [guid]::NewGuid().ToString("N"))
  $debug = Join-Path $root "target\debug"
  New-Item -ItemType Directory -Path $debug -Force | Out-Null
  try {
    $stale = Join-Path $debug "nexora_halo.pdb"
    Set-Content -LiteralPath $stale -Value stale
    $rejected = Invoke-Script $hygiene @("-Root", $root)
    Assert-True ($rejected.ExitCode -ne 0) "stale product artifact was accepted"
    Assert-True ($rejected.Output.Contains("nexora_halo.pdb")) "stale artifact path was not reported: $($rejected.Output)"

    Remove-Item -LiteralPath $stale
    $clean = Invoke-Script $hygiene @("-Root", $root)
    Assert-True ($clean.ExitCode -eq 0) "clean workspace was rejected: $($clean.Output)"
  } finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
  }
}

if ($failures -gt 0) {
  Write-Error "$failures of $tests product workflow tests failed"
  exit 1
}

Write-Host "$tests product workflow tests passed"
