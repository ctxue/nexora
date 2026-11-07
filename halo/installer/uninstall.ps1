$ErrorActionPreference='Stop'
$installDir=[System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\NexoraHalo'))
if([System.IO.Path]::GetFullPath($PSScriptRoot) -ne $installDir){throw '卸载脚本只能在 NEXORA HALO 安装目录中运行。'}
$exePath=Join-Path $installDir 'nexora-halo.exe'
Get-Process nexora-halo -ErrorAction SilentlyContinue | Where-Object {$_.Path -eq $exePath} | ForEach-Object {
    $null=$_.CloseMainWindow();if(-not $_.WaitForExit(3000)){Stop-Process -Id $_.Id}
}
$shell=New-Object -ComObject WScript.Shell
$links=@((Join-Path $shell.SpecialFolders.Item('Desktop') 'NEXORA HALO.lnk'),(Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\NEXORA HALO.lnk'),(Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\NEXORA HALO 设置.lnk'))
foreach($path in $links){if((Test-Path -LiteralPath $path) -and $shell.CreateShortcut($path).TargetPath -eq $exePath){Remove-Item -LiteralPath $path}}
$runKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
Remove-ItemProperty -Path $runKey -Name NexoraHalo -ErrorAction SilentlyContinue
Remove-Item -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\NexoraHalo' -ErrorAction SilentlyContinue
foreach($name in @('nexora-halo.exe','nexora-halo-previous.exe','THIRD_PARTY_NOTICES.txt','LICENSE','uninstall.ps1')){
    $path=Join-Path $installDir $name;if(Test-Path -LiteralPath $path){Remove-Item -LiteralPath $path}
}
if(-not(Get-ChildItem -LiteralPath $installDir -Force)){Remove-Item -LiteralPath $installDir}
Write-Output 'NEXORA HALO 已卸载。邮件连接、文件口袋和设置仍保存在本机用户数据目录中。'
