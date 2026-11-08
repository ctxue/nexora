$ErrorActionPreference = "Stop"

$project = Split-Path $PSScriptRoot -Parent
$source = Get-Content -Raw -LiteralPath (Join-Path $project "installer\setup.nsi")
$config = Get-Content -Raw -LiteralPath (Join-Path $project "installer\config.nsh")

function Assert-True {
  param([bool]$Condition, [string]$Message)
  if (-not $Condition) { throw $Message }
}

Assert-True ($source.Contains('!insertmacro MUI_PAGE_WELCOME')) "standard welcome page is missing"
Assert-True ($source.Contains('!insertmacro MUI_PAGE_DIRECTORY')) "standard install directory page is missing"
Assert-True ($source.Contains('!insertmacro MUI_PAGE_INSTFILES')) "standard install page is missing"
Assert-True ($source.Contains('!include "config.nsh"')) "installer configuration is not included"
Assert-True (-not $source.Contains("Page custom")) "custom installer page is still present"
Assert-True (-not $source.Contains("nsDialogs")) "custom dialog code is still present"
foreach ($function in @("InstallPage", "BrowseDirectory", "ValidateDirectory", "InstallPageLeave")) {
  Assert-True (-not $source.Contains("Function $function")) "unused installer function remains: $function"
}
Assert-True ($config.Contains('!define INSTALL_DESKTOP_SHORTCUT 1')) "desktop shortcut default is not configurable"
Assert-True ($config.Contains('!define ENABLE_AUTOSTART 0')) "autostart default is not configurable"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $project "installer\install.ps1"))) "legacy install.ps1 remains"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $project "installer\uninstall.ps1"))) "legacy uninstall.ps1 remains"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $project "安装-NEXORA-ISLE.cmd"))) "legacy manual install launcher remains"

Write-Host "Installer layout checks passed"
