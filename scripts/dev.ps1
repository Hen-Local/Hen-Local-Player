$ErrorActionPreference = "Stop"
Set-Location "$PSScriptRoot/.."
if (!$env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = "C:\hlp-target" }
if ($env:CARGO_TARGET_DIR.Contains(" ")) { throw "Choose a CARGO_TARGET_DIR without spaces." }
cargo build --locked -p hlp-asr-worker -p hlp-cli
if ($LASTEXITCODE) { exit $LASTEXITCODE }
Set-Location app
if (!(Test-Path ui/node_modules/.bin/tauri.cmd)) {
  npm --prefix ui ci
  if ($LASTEXITCODE) { exit $LASTEXITCODE }
}
& ui/node_modules/.bin/tauri.cmd dev
exit $LASTEXITCODE
