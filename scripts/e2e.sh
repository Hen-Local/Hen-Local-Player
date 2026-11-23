#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/build_env.sh
cargo build -p hlp-cli -p hlp-asr-worker
ffmpeg_bin="${HLP_FFMPEG:-$(find "${HLP_OMINIX_ROOT:-$HOME/.OminiX}/tools/ffmpeg" -type f -name ffmpeg | head -1)}"
python3 scripts/e2e.py --cli "$CARGO_TARGET_DIR/debug/hlp-cli" --ffmpeg "$ffmpeg_bin" "$@"
