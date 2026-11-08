param([switch]$SkipBuild, [string]$Executable, [string]$NsisCompiler, [string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Push-Location $projectRoot
try {
    $version = [regex]::Match((Get-Content Cargo.toml -Raw), '(?m)^version = "([^"]+)"').Groups[1].Value
    # 发布版不嵌入构建机器的用户目录和项目路径：依赖源码路径换成中性前缀，PDB 只记文件名。
    # 用 CARGO_ENCODED_RUSTFLAGS（\x1f 分隔）避免路径里的空格被拆开；它会覆盖 .cargo/config.toml 的 rustflags，所以带上 crt-static。
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $env:CARGO_ENCODED_RUSTFLAGS = @('-C', 'target-feature=+crt-static', "--remap-path-prefix=$cargoHome=cargo", "--remap-path-prefix=$projectRoot=nexora-isle", '-C', 'link-arg=/PDBALTPATH:%_PDB%') -join [char]0x1f
    if (-not $SkipBuild) {
        $env:CARGO_BUILD_JOBS = '1'
        cargo +stable-x86_64-pc-windows-msvc build --locked --release --bin nexora-isle --target x86_64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    }
    if (-not $Executable) {
        $targetDirectory = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $projectRoot 'target' }
        $Executable = Join-Path $targetDirectory 'x86_64-pc-windows-msvc\release\nexora-isle.exe'
    }
    $binary = Get-Item -LiteralPath $Executable
    & (Join-Path $PSScriptRoot 'check-runtime.ps1') -Executable $binary.FullName
    if ($binary.VersionInfo.FileVersion -ne $version -and $binary.VersionInfo.FileVersion -ne "$version.0") { throw 'Executable version mismatch' }
    $content = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($binary.FullName))
    foreach ($private in @($env:USERPROFILE, $projectRoot)) {
        if ($private -and $content.Contains($private)) { throw "Executable embeds a build-machine path: $private" }
    }
    if (-not $NsisCompiler) {
        $localNsis = Join-Path $projectRoot '.tools\nsis\nsis-3.13\Bin\makensis.exe'
        $compiler = Get-Command makensis.exe -ErrorAction SilentlyContinue
        if (Test-Path -LiteralPath $localNsis) { $NsisCompiler = $localNsis }
        elseif ($compiler) { $NsisCompiler = $compiler.Source }
        elseif ($env:NSIS_HOME) { $NsisCompiler = Join-Path $env:NSIS_HOME 'makensis.exe' }
        else { $NsisCompiler = Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe' }
    }
    if (-not (Test-Path -LiteralPath $NsisCompiler)) {
        $NsisCompiler = & (Join-Path $PSScriptRoot 'get-nsis.ps1') -Directory ([IO.Path]::Combine([IO.Path]::GetTempPath(), 'nexora-nsis')) | Select-Object -Last 1
    }
    if (-not (Test-Path -LiteralPath $NsisCompiler)) { throw 'NSIS compiler not found; install NSIS or pass -NsisCompiler' }
    $packageName = "NEXORA_ISLE_${version}_windows_x64"
    if (-not $OutputDirectory) { $OutputDirectory = Join-Path $projectRoot 'dist' }
    $outputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
    $stagingRoot = Join-Path ([IO.Path]::GetTempPath()) ('nexora-isle-package-' + [guid]::NewGuid().ToString('N'))
    $stagingDirectory = Join-Path $stagingRoot $packageName
    New-Item -ItemType Directory -Path $stagingDirectory -Force | Out-Null
    Copy-Item -LiteralPath $binary.FullName -Destination (Join-Path $stagingDirectory 'nexora-isle.exe')
    # ZIP 保留给免安装使用和应用内更新；普通用户只需下载安装 EXE。
    @('LICENSE','THIRD_PARTY_NOTICES.txt') | ForEach-Object {
        Copy-Item -LiteralPath (Join-Path $projectRoot $_) -Destination $stagingDirectory -Recurse
    }
    $setup = Join-Path $outputDirectory "NEXORA_ISLE_${version}_x64_setup.exe"
    $archive = Join-Path $outputDirectory "NEXORA_ISLE_${version}_portable.zip"
    foreach ($existing in @($setup, $archive)) {
        if (Test-Path -LiteralPath $existing) { Remove-Item -LiteralPath $existing -Force }
    }
    & $NsisCompiler /V2 /WX "/DAPP_VERSION=$version" "/DPROJECT_DIR=$projectRoot" "/DSOURCE_EXE=$($binary.FullName)" "/DOUTPUT_FILE=$setup" (Join-Path $projectRoot 'installer\setup.nsi')
    if ($LASTEXITCODE -ne 0) { throw 'Installer build failed' }
    Compress-Archive -LiteralPath $stagingDirectory -DestinationPath $archive
    Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    Write-Output $setup
    Write-Output $archive
} finally { Pop-Location }
