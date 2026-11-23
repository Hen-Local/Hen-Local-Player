# Build an internal x64 NSIS installer with the matching ASR sidecar.
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Build the NSIS package on Windows with the MSVC x64 tools.' }
. "$PSScriptRoot/build_env.ps1"
$triple = 'x86_64-pc-windows-msvc'
Push-Location $repoDirectory
try {
    cargo build --locked --release --target $triple -p hlp-asr-worker
    if ($LASTEXITCODE) { throw "Worker build failed ($LASTEXITCODE)." }

    $releaseDir = Join-Path $env:CARGO_TARGET_DIR "$triple/release"
    New-Item -ItemType Directory -Force 'app/binaries' | Out-Null
    Copy-Item -LiteralPath (Join-Path $releaseDir 'hlp-asr-worker.exe') -Destination "app/binaries/hlp-asr-worker-$triple.exe" -Force
    if (!(Test-Path 'app/ui/node_modules/.bin/tauri.cmd')) {
        npm.cmd --prefix app/ui ci
        if ($LASTEXITCODE) { throw "Frontend dependencies failed ($LASTEXITCODE)." }
    }
    Push-Location app
    try {
        & ./ui/node_modules/.bin/tauri.cmd build --target $triple --bundles nsis --config tauri.bundle.conf.json
        if ($LASTEXITCODE) { throw "NSIS build failed ($LASTEXITCODE)." }
    } finally {
        Pop-Location
    }

    $config = Get-Content -Raw -Encoding UTF8 'app/tauri.conf.json' | ConvertFrom-Json
    $nsisDir = Join-Path $releaseDir 'bundle/nsis'
    $installer = Get-ChildItem -LiteralPath $nsisDir -Filter "*_$($config.version)_x64-setup.exe" |
        Select-Object -First 1
    if (!$installer) { throw "The Tauri bundler did not produce the expected x64 installer in $nsisDir." }
    New-Item -ItemType Directory -Force dist | Out-Null
    $destination = Join-Path $repoDirectory "dist/Hen-Local-Player-$($config.version)-windows-x64-setup.exe"
    Copy-Item -LiteralPath $installer.FullName -Destination $destination -Force
    Get-FileHash -Algorithm SHA256 -LiteralPath $destination
} finally {
    Pop-Location
}
