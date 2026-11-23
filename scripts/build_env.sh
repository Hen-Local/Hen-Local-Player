# Adapted from Hen-Local-Translator/scripts/run_translator_desktop.sh.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/hen-local-player-target}"
if [[ "$CARGO_TARGET_DIR" == *" "* ]]; then
  echo "Choose a CARGO_TARGET_DIR without spaces (MLX embeds the path in Metal sources)." >&2
  exit 1
fi
if [[ "$(uname -s)" == Darwin ]]; then
  if ! xcrun clang --version >/dev/null 2>&1 && [[ -x /Library/Developer/CommandLineTools/usr/bin/clang ]]; then
    export DEVELOPER_DIR=/Library/Developer/CommandLineTools
  fi
fi
if [[ "$(uname -s)" == Darwin ]]; then
  export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"
  if [[ -z "${MLX_PREBUILT_PATH:-}" ]]; then
    mlx_helper="$(cd "$(dirname "${BASH_SOURCE[0]:-scripts/build_env.sh}")/.." && pwd)/vendor/qwen3-asr-mlx/tools/prepare_mlx.py"
    MLX_PREBUILT_PATH="$(python3 "$mlx_helper")" || exit 1
    export MLX_PREBUILT_PATH
  fi
fi
