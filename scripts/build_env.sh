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
# Build dependency pinned by Hen-Local-Translator/.github/workflows/release.yml.
if [[ "$(uname -s)" == Darwin && -z "${MLX_PREBUILT_PATH:-}" ]]; then
  mlx_dir="$CARGO_TARGET_DIR/mlx-prebuilt-v0.1.0-macos-arm64"
  if [[ ! -s "$mlx_dir/libmlx.a" || ! -s "$mlx_dir/mlx.metallib" ]]; then
    mkdir -p "$CARGO_TARGET_DIR"
    mlx_tmp="$(mktemp -d "$CARGO_TARGET_DIR/.mlx-XXXXXX")"
    (
      set -e
      trap 'rm -rf "$mlx_tmp"' EXIT
      curl -fsSL --retry 3 -o "$mlx_tmp/mlx.tgz" https://github.com/OminiX-ai/OminiX-MLX/releases/download/mlx-prebuilt-v0.1.0/mlx-prebuilt-v0.1.0-macos-arm64.tar.gz
      [[ "$(wc -c < "$mlx_tmp/mlx.tgz" | tr -d ' ')" == 38962928 ]]
      (cd "$mlx_tmp" && echo '8be50f294fee2ee55400ec802bfb7bcd1d1d95c74bc2c84a45d94e685c77aed5  mlx.tgz' | shasum -a 256 -c -)
      tar -xzf "$mlx_tmp/mlx.tgz" -C "$mlx_tmp"
      mv "$mlx_tmp/mlx-prebuilt-v0.1.0-macos-arm64" "$mlx_dir"
    ) || exit 1
  fi
  export MLX_PREBUILT_PATH="$mlx_dir"
fi
