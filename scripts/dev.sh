#!/usr/bin/env bash
# Adapted from Hen-Local-Translator/scripts/run_translator_desktop.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/build_env.sh
cargo build --locked -p hlp-asr-worker -p hlp-cli
cd app
[[ -x ui/node_modules/.bin/tauri ]] || npm --prefix ui ci
exec ./ui/node_modules/.bin/tauri dev
