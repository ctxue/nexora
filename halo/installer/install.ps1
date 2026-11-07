param([string]$SourceExecutable)
if(-not $SourceExecutable){
    $packaged=Join-Path $PSScriptRoot '..\nexora-halo.exe'
    $SourceExecutable=if(Test-Path -LiteralPath $packaged){$packaged}else{Join-Path $PSScriptRoot '..\target\release\nexora-halo.exe'}
}
$ErrorActionPreference='Stop'
$sourcePath=[System.IO.Path]::GetFullPath($SourceExecutable)
if(-not(Test-Path -LiteralPath $sourcePath)){throw '找不到 NEXORA HALO 程序，请先完成构建。'}
$installDir=Join-Path $env:LOCALAPPDATA 'Programs\NexoraHalo'
$exePath=Join-Path $installDir 'nexora-halo.exe'
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
Get-Process nexora-halo -ErrorAction SilentlyContinue | Where-Object {$_.Path -eq $exePath} | ForEach-Object {
    $null=$_.CloseMainWindow()
    if(-not $_.WaitForExit(3000)){Stop-Process -Id $_.Id; $_.WaitForExit()}
}
if(Test-Path -LiteralPath $exePath){Copy-Item -LiteralPath $exePath -Destination (Join-Path $installDir 'nexora-halo-previous.exe') -Force}
Copy-Item -LiteralPath $sourcePath -Destination $exePath -Force
if(Test-Path -LiteralPath (Join-Path $PSScriptRoot '..\THIRD_PARTY_NOTICES.txt')){Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\THIRD_PARTY_NOTICES.txt') -Destination (Join-Path $installDir 'THIRD_PARTY_NOTICES.txt') -Force}
if(Test-Path -LiteralPath (Join-Path $PSScriptRoot '..\LICENSE')){Copy-Item -LiteralPath (Join-Path $PSScriptRoot '..\LICENSE') -Destination (Join-Path $installDir 'LICENSE') -Force}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'uninstall.ps1') -Destination (Join-Path $installDir 'uninstall.ps1') -Force
$shell=New-Object -ComObject WScript.Shell
$entries=@(
    @((Join-Path $shell.SpecialFolders.Item('Desktop') 'NEXORA HALO.lnk'),''),
    @((Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\NEXORA HALO.lnk'),''),
    @((Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\NEXORA HALO 设置.lnk'),'--settings')
)
foreach($entry in $entries){
    $link=$shell.CreateShortcut($entry[0]);$link.TargetPath=$exePath;$link.Arguments=$entry[1]
    $link.WorkingDirectory=$installDir;$link.IconLocation=$exePath+',0';$link.Description='NEXORA HALO';$link.Save()
}
$key='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\NexoraHalo'
New-Item -Path $key -Force | Out-Null
$version=(Get-Item -LiteralPath $exePath).VersionInfo.FileVersion
$psExe=Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
$values=@{DisplayName='NEXORA HALO';DisplayVersion=$version;Publisher='NEXORA';InstallLocation=$installDir;DisplayIcon=$exePath+',0';UninstallString=('"'+$psExe+'" -NoProfile -ExecutionPolicy Bypass -File "'+(Join-Path $installDir 'uninstall.ps1')+'"')}
foreach($name in $values.Keys){New-ItemProperty -Path $key -Name $name -Value $values[$name] -PropertyType String -Force | Out-Null}
foreach($name in @('NoModify','NoRepair')){New-ItemProperty -Path $key -Name $name -Value 1 -PropertyType DWord -Force | Out-Null}
Start-Process -FilePath $exePath -WorkingDirectory $installDir -WindowStyle Hidden
Write-Output ('NEXORA HALO '+$version+' 已安装：'+$exePath)
