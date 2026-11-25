#!/usr/bin/env bash
# Bundle layout adapted from Hen-Local-Translator/scripts/build_macos_app.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
root="$(pwd -P)"
[[ "$(uname -sm)" == "Darwin arm64" ]] || { echo "Build the macOS package on an Apple Silicon Mac." >&2; exit 1; }
source scripts/build_env.sh
triple=aarch64-apple-darwin
version="$(node -p "require('./app/tauri.conf.json').version")"
case "${1:-}" in
  ''|--updater) ;;
  *) echo "Usage: bash scripts/build_macos_app.sh [--updater]" >&2; exit 2 ;;
esac
out="$root/dist"
mkdir -p "$out" app/binaries
stage="$(mktemp -d "$out/.macos-package.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
worker="$CARGO_TARGET_DIR/release/hlp-asr-worker"
cargo build --locked --release -p hlp-asr-worker
install -m 755 "$worker" "app/binaries/hlp-asr-worker-$triple"
HLP_OMINIX_ROOT="${HLP_BUNDLE_TOOL_CACHE:-$CARGO_TARGET_DIR/packaging-tools}" \
  cargo run --quiet --locked -p hlp-models --example stage_playback_tools -- \
    app/binaries "$triple" docs/licenses/FFmpeg-COPYING.GPLv3
[[ -x app/ui/node_modules/.bin/tauri ]] || npm --prefix app/ui ci
(
  cd app
  ./ui/node_modules/.bin/tauri build --bundles app --config tauri.bundle.conf.json
)
src="$CARGO_TARGET_DIR/release/bundle/macos/Hen Local Player.app"
mkdir -p "$stage/volume"
bundle="$stage/volume/Hen Local Player.app"
ditto "$src" "$bundle"
resources="$bundle/Contents/Resources"
mkdir -p "$resources/licenses"
install -m 644 "$CARGO_TARGET_DIR/release/mlx.metallib" "$resources/mlx.metallib"
install -m 644 docs/LICENSES.md "$resources/licenses/components.md"
install -m 644 vendor/qwen3-asr-mlx/LICENSE-* "$resources/licenses"
ln -s ../Resources "$bundle/Contents/MacOS/Resources"
test -x "$bundle/Contents/MacOS/hlp-asr-worker"
test -s "$bundle/Contents/MacOS/Resources/mlx.metallib"
# Developer ID signing order adapted from Hen-Local-Translator/scripts/macos_sign_and_notarize.sh.
if [[ -n "${HLP_NOTARY_PROFILE:-}" && -z "${HLP_DEVELOPER_ID:-}" ]]; then
  echo "Set HLP_DEVELOPER_ID to the Developer ID Application identity for notarization." >&2
  exit 1
fi
if [[ -n "${HLP_DEVELOPER_ID:-}" ]]; then
  while IFS= read -r -d '' code; do
    if file -b "$code" | grep -q 'Mach-O'; then
      codesign --force --options runtime --timestamp --sign "$HLP_DEVELOPER_ID" "$code"
    fi
  done < <(find "$bundle/Contents" -type f -print0)
  codesign --force --options runtime --timestamp --sign "$HLP_DEVELOPER_ID" "$bundle"
else
  codesign --force --deep --sign - "$bundle"
fi
codesign --verify --deep --strict "$bundle"
if [[ -n "${HLP_NOTARY_PROFILE:-}" ]]; then
  notary_args=(--keychain-profile "$HLP_NOTARY_PROFILE" --wait)
  if [[ -n "${HLP_NOTARY_KEYCHAIN:-}" ]]; then
    notary_args+=(--keychain "$HLP_NOTARY_KEYCHAIN")
  fi
  ditto -c -k --keepParent "$bundle" "$stage/notarize-app.zip"
  xcrun notarytool submit "$stage/notarize-app.zip" "${notary_args[@]}"
  xcrun stapler staple "$bundle"
  xcrun stapler validate "$bundle"
  spctl --assess --type execute "$bundle"
fi
if [[ "${1:-}" == --updater ]]; then
  # Archive the signed, fully staged app. The initial Tauri bundle lacks the Metal library.
  archive_name="Hen-Local-Player-${version}-macos-arm64.app.tar.gz"
  COPYFILE_DISABLE=1 tar --no-xattrs -czf "$stage/$archive_name" -C "$stage/volume" "Hen Local Player.app"
  if [[ -n "${HLP_NOTARY_PROFILE:-}" ]]; then
    mkdir "$stage/check-updater"
    tar -xzf "$stage/$archive_name" -C "$stage/check-updater"
    archived_bundle="$stage/check-updater/Hen Local Player.app"
    codesign --verify --deep --strict "$archived_bundle"
    xcrun stapler validate "$archived_bundle"
  fi
fi
ln -s /Applications "$stage/volume/Applications"
image_name="Hen-Local-Player-${version}-macos-arm64.dmg"
hdiutil create -quiet -fs HFS+ -volname "Hen Local Player" -format UDZO -srcfolder "$stage/volume" "$stage/$image_name"
if [[ -n "${HLP_DEVELOPER_ID:-}" ]]; then
  codesign --force --timestamp --sign "$HLP_DEVELOPER_ID" "$stage/$image_name"
fi
if [[ -n "${HLP_NOTARY_PROFILE:-}" ]]; then
  xcrun notarytool submit "$stage/$image_name" "${notary_args[@]}"
  xcrun stapler staple "$stage/$image_name"
  xcrun stapler validate "$stage/$image_name"
fi
hdiutil verify -quiet "$stage/$image_name"
# Move a prior generated application aside until its replacement is ready.
if [[ -e "$out/Hen Local Player.app" ]]; then
  mv "$out/Hen Local Player.app" "$stage/previous.app"
fi
if ! mv "$bundle" "$out/Hen Local Player.app"; then
  [[ ! -e "$stage/previous.app" ]] || mv "$stage/previous.app" "$out/Hen Local Player.app"
  exit 1
fi
mv -f "$stage/$image_name" "$out/$image_name"
if [[ "${1:-}" == --updater ]]; then
  mv -f "$stage/$archive_name" "$out/$archive_name"
  printf '%s\n' "$out/$archive_name"
fi
printf '%s\n' "$out/Hen Local Player.app" "$out/$image_name"
