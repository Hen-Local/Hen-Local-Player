# Adapted from Hen-Local-Translator/scripts/run_translator_desktop.sh.
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build_env.ps1"
Push-Location $repoDirectory
try {
    cargo build --locked -p hlp-asr-worker -p hlp-cli
    if ($LASTEXITCODE) { throw "Cargo build failed ($LASTEXITCODE)." }
    Set-Location app
    if (!(Test-Path -LiteralPath 'ui/node_modules/.bin/tauri.cmd')) {
        npm.cmd --prefix ui ci
        if ($LASTEXITCODE) { throw "npm ci failed ($LASTEXITCODE)." }
    }
    & './ui/node_modules/.bin/tauri.cmd' dev
    if ($LASTEXITCODE) { throw "Tauri dev failed ($LASTEXITCODE)." }
} finally {
    Pop-Location
}
