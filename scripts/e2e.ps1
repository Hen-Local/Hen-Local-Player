# The same timing, text, cache and export oracle runs on both platforms.
param([string[]]$Fixtures = @())
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build_env.ps1"
Push-Location $repoDirectory
try {
    $ffmpeg = Get-HlpFfmpeg
    cargo build --locked -p hlp-asr-worker -p hlp-cli
    if ($LASTEXITCODE) { throw "Cargo build failed ($LASTEXITCODE)." }
    $cli = Join-Path $env:CARGO_TARGET_DIR 'debug/hlp-cli.exe'
    # Make Python's JSON and subprocess I/O independent of the Windows ANSI code page.
    $env:PYTHONUTF8 = '1'
    if (Get-Command py.exe -ErrorAction SilentlyContinue) {
        & py.exe -3 scripts/e2e.py --cli $cli --ffmpeg $ffmpeg @Fixtures
    } else {
        & python.exe scripts/e2e.py --cli $cli --ffmpeg $ffmpeg @Fixtures
    }
    if ($LASTEXITCODE) { throw "Fixture checks failed ($LASTEXITCODE). See tests/results/." }
} finally {
    Pop-Location
}
