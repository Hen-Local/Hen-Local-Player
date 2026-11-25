#!/usr/bin/env bash
# Build the stand-alone FFmpeg and FFprobe shipped in the Apple Silicon app.
# Usage: bash build_macos_media.sh OUTPUT [DIRECTORY_WITH_SOURCE_ARCHIVES]
set -euo pipefail
[[ $# -ge 1 && $# -le 2 ]] || { echo "Expected an output directory and optionally a source archive directory" >&2; exit 2; }
[[ "$(uname -sm)" == 'Darwin arm64' ]] || { echo "Build the media tools on Apple Silicon" >&2; exit 1; }
mkdir -p "$1"
out=$(cd "$1" && pwd -P)
recipe_dir=$(cd "$(dirname "$0")" && pwd -P)
recipe="$recipe_dir/$(basename "$0")"
supplied=$(cd "${2:-$recipe_dir}" && pwd -P)
if cmp -s "$recipe" "$out/sources/build_macos_media.sh" && (cd "$out" && shasum -a 256 --check --status SHA256SUMS); then
  exit 0
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/hen-media.XXXXXXXX")
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/result/sources" "$work/prefix"
sources="$work/result/sources"
get_archive() {
  local url=$1 size=$2 sha=$3 name
  name=${url##*/}
  if [[ -f "$supplied/$name" ]]; then
    cp "$supplied/$name" "$sources/$name"
  elif [[ -f "$out/sources/$name" ]]; then
    cp "$out/sources/$name" "$sources/$name"
  else
    curl --fail --location --silent --show-error --retry 2 --output "$sources/$name" "$url"
  fi
  [[ "$(stat -f '%z' "$sources/$name")" == "$size" ]]
  (cd "$sources" && printf '%s  %s\n' "$sha" "$name" | shasum -a 256 --check --status)
  tar -xf "$sources/$name" -C "$work"
}
get_archive https://ffmpeg.org/releases/ffmpeg-9.1.tar.xz 12557532 \
  1a4d394ae63d190a526cd052eb4415c156b5c5e2df37efab41a7de0b8fde2433
get_archive https://downloads.videolan.org/pub/videolan/dav1d/1.5.5/dav1d-1.5.5.tar.xz 1051484 \
  ca96b1d0ba3f01f55de7231d775892b64994ba061aaeeaaa4f662a390b3abf9e

if ! xcrun clang --version >/dev/null 2>&1 && [[ -x /Library/Developer/CommandLineTools/usr/bin/clang ]]; then
  export DEVELOPER_DIR=/Library/Developer/CommandLineTools
fi
export MACOSX_DEPLOYMENT_TARGET=14.0
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
export CC="$(xcrun --find clang)"
export CXX="$(xcrun --find clang++)"
unset CPATH LIBRARY_PATH CFLAGS CPPFLAGS LDFLAGS PKG_CONFIG_PATH
export PKG_CONFIG_LIBDIR="$work/prefix/lib/pkgconfig"
jobs=$(sysctl -n hw.activecpu)
cd "$work"
meson setup dav1d-build dav1d-1.5.5 --prefix "$work/prefix" --libdir lib \
  --buildtype release --default-library static --wrap-mode nodownload \
  -Denable_tools=false -Denable_tests=false -Denable_examples=false -Denable_docs=false
meson compile -C dav1d-build -j "$jobs"
meson install -C dav1d-build

# Apple ships libxml2 for the DASH demuxer, but its SDK has no pkg-config file.
xml_header="$SDKROOT/usr/include/libxml2/libxml/xmlversion.h"
xml_version=$(awk '/^#define LIBXML_DOTTED_VERSION / { gsub(/"/, "", $3); print $3 }' "$xml_header")
[[ -n "$xml_version" ]]
cat > "$PKG_CONFIG_LIBDIR/libxml-2.0.pc" <<PC
Name: libxml2
Description: libxml2 in the macOS SDK
Version: $xml_version
Cflags: -I"$SDKROOT/usr/include/libxml2"
Libs: -lxml2
PC
cd ffmpeg-9.1
./configure --arch=arm64 --cc="$CC" --cxx="$CXX" --extra-libs=-liconv \
  --disable-autodetect --disable-shared --enable-static --disable-debug --disable-doc --disable-ffplay \
  --enable-videotoolbox --enable-audiotoolbox --enable-securetransport --enable-iconv \
  --enable-zlib --enable-bzlib --enable-libxml2 --enable-libdav1d
make -j"$jobs" ffmpeg ffprobe
for name in ffmpeg ffprobe; do
  if otool -L "$name" | tail -n +2 | grep -Evq '^[[:space:]]+(/System/Library/|/usr/lib/)'; then
    echo "Unexpected non-system library in $name" >&2
    exit 1
  fi
  install -m 755 "$name" "$work/result/$name"
done
install -m 644 COPYING.LGPLv2.1 "$work/result/FFmpeg-LICENSE.txt"
install -m 644 LICENSE.md "$sources/FFmpeg-license-summary.md"
install -m 644 "$work/dav1d-1.5.5/COPYING" "$sources/dav1d-LICENSE.txt"
install -m 644 "$recipe" "$sources/build_macos_media.sh"
cd "$work/result"
shasum -a 256 ffmpeg ffprobe FFmpeg-LICENSE.txt sources/* > SHA256SUMS
mkdir -p "$out/sources"
cp -f sources/* "$out/sources/"
cp -f ffmpeg ffprobe FFmpeg-LICENSE.txt "$out/"
mv SHA256SUMS "$out/"
