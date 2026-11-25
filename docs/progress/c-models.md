# Lane C · Models and setup

- [x] Existing ASR, GGUF and llama runtime are ready. On the development Mac, `hlp-cli status` reported both MLX variants, Hy-MT2 and b11429 as Ready before any install. No changes were made to the translator's installation.
- [x] Verify managed Silero and FFmpeg installs under a temporary root. With only the existing model and runtime paths symlinked under `HLP_OMINIX_ROOT=$(mktemp -d)`, `hlp-cli setup --yes --with-urls` downloaded and verified Silero, both FFmpeg executables, yt-dlp and Deno. A subsequent `hlp-cli status` reported all eight entries Ready.
- [x] Interrupted downloads resume and manifest pins pass validation. `cargo test --locked -p hlp-models` passed six offline tests. The loopback origin recorded a cancelled response after three bytes and a new request with `Range: bytes=3-`. Other tests cover servers ignoring the range, same-size bad digests, concurrent installs, tag validation and immutable pins for both platforms.

The live `cargo test --locked -p hlp-models -- --ignored` passed in 48.25 s. It installed fresh macOS FFmpeg, Silero, llama.cpp, yt-dlp and Deno in an empty temporary directory, exercised the explicit yt-dlp update, and validated the resulting executables. After rebasing on the lane A merge, the six-package check in §9.1.1 passed and `npm --prefix app/ui run check` reported zero errors and warnings.

## Verified component details

The repository manifest records exact URLs, byte counts and SHA-256 digests. The default and Light MLX repositories have the same ten runtime/model-card files, including one weight file each. The tokenizer can be constructed from vocab.json and merges.txt. The Git transport metadata file is excluded. Verified each local file against its Hugging Face blob ID or LFS digest and the matching ModelScope digest. Each mirror uses its own commit revision. The downloader writes the translator-compatible repo_id and bootstrap_version marker.

The Windows Sherpa archive measured 878,702,423 bytes, SHA-256 `393f8a14e2f5fb96746aaab342997a40641001fbd5bf9592a080a8329178ee96`. Its file list includes conv_frontend.onnx, encoder.int8.onnx, decoder.int8.onnx and the three tokenizer files. The Silero model measured 643,854 bytes, SHA-256 `9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6`.

The yt-dlp 2026.10.26 standalone pins match its SHA2-256SUMS. The Deno v2.9.8 pins match the platform-specific .sha256sum files. The yt-dlp source is Unlicense. The release README identifies the combined PyInstaller executables as GPLv3+. Existing installs and ordinary playback require no release metadata request. The explicit yt-dlp update command checks the release's checksum file before activating its version directory. [yt-dlp release](https://github.com/yt-dlp/yt-dlp/releases/tag/2026.10.26) · [Deno release](https://github.com/denoland/deno/releases/tag/v2.9.8)

## Media tools in the open-source Mac release

The Apple Silicon installer builds two independent executables from the unmodified FFmpeg 9.1 and dav1d 1.5.5 release archives. Both archives, notices and the complete build recipe are included in the app and the update archive. FFmpeg's release signature verified against its published key `FCF986EA15E6E293A5644F10B4322F04D67658D8`. The recipe checks both archive sizes and SHA-256 hashes before extraction. Configure reported LGPL 2.1 or later. The only non-system external library is dav1d (BSD-2-Clause). [FFmpeg release and signing key](https://ffmpeg.org/download.html) · [Official dav1d archive](https://downloads.videolan.org/pub/videolan/dav1d/1.5.5/)

Verified on Apple Silicon:

- `bash scripts/build_macos_media.sh "$media" "$archives"` completed from local source archives using Apple Command Line Tools, Meson 1.9.1, Ninja 1.13.0 and pkgconf 2.5.1. Repeating the command without the archive argument reused the checked output. `shasum -a 256 -c SHA256SUMS` in the output directory passed for all eight files.
- `otool -L` reported only system library and framework paths for both programs. `vtool -show-build` reported minimum macOS 14.0. `ffmpeg -L` reported LGPL 2.1 or later.
- FFprobe recognized the H.264 and AC-3 streams in `fixture_fr.mkv`. FFmpeg converted two seconds of VP9/Opus from `fixture_de.webm` with `-vf yadif -c:v h264_videotoolbox -c:a aac`. FFprobe verified H.264/AAC and a duration of 2.000 seconds.
- `ffmpeg -c:v libdav1d -i av1.ivf -f framecrc -` decoded all 12 AV1 test frames. A one-second audio extraction from `fixture_en.m4a` produced 32,000 bytes at 16 kHz mono signed PCM. `ffmpeg -i app/ui/public/demo/demo.srt -map 0:s:0 -c:s webvtt -f webvtt -` preserved Japanese and English cues. The protocol and demuxer lists include HTTPS and DASH.
- `python3 scripts/e2e.py --cli "$target/debug/hlp-cli" --ffmpeg "$media/ffmpeg" --report "$audit/e2e-ja-verified.json" ja` passed all nine checks for 0.1.1 in a separate component directory using these same FFmpeg/FFprobe executables, Silero, Qwen3-ASR 1.7B and Hy-MT2 1.8B. All six utterances were detected and translated. Character error was 2.86% and maximum onset error was 37 ms. The first run took 8.662 seconds and the cached run took 0.208 seconds with zero model requests. FFmpeg decoded all nine exported SRT, WebVTT and ASS files.

## FFmpeg audit evidence

The following details refer to the stand-alone CLI setup downloads and earlier packaging.

The macOS downloads are OSXExperts FFmpeg and ffprobe 9.1 for Apple Silicon. The archive hashes and sizes are in components.json. Both unpacked binaries target macOS 13.0 according to `otool -l`, compatible with the player's macOS 14 minimum. The binaries report VideoToolbox and GPL with version3 enabled. They are invoked as separate programs. The distributor describes its binaries as educational builds. The source link on its page still points to the 6.1 branch. The upstream 9.1 source tag matches the reported version. [Distributor](https://www.osxexperts.net/) · [FFmpeg n9.1](https://github.com/FFmpeg/FFmpeg/tree/n9.1)

The Windows pin uses the month-end BtbN build of FFmpeg 8.1.3-27-g8dca1586f4, win64-lgpl. The ZIP was downloaded and hashed locally. It includes LICENSE.txt, which the installer preserves. The build enables version3, AMF, ffnvcodec, libvpl and libopenh264, with libx264 disabled. BtbN retains month-end releases for two years. [Pinned release](https://github.com/FFmpeg-Builds/FFmpeg-Builds/releases/tag/autobuild-2026-10-31-12-59) · [FFmpeg source revision](https://github.com/FFmpeg/FFmpeg/tree/8dca1586f4)

Exact configure flags read from each build follow.

**macOS**

```text
configuration: --prefix=/Volumes/tempdisk/prefix --disable-shared --enable-static --disable-autodetect --enable-gpl --arch=arm64 --cc=clang --cxx=clang++ --pkg-config-flags=--static --extra-cflags=-fno-stack-check --enable-videotoolbox --enable-audiotoolbox --enable-runtime-cpudetect --disable-doc --enable-vulkan --enable-vulkan-static --enable-lto=thin --enable-neon --enable-version3 --enable-lzma --enable-zlib --enable-iconv --enable-libxml2 --enable-libmp3lame --enable-libvidstab --enable-libx264 --enable-libx265 --enable-libvpx --enable-libfribidi --enable-libfreetype --enable-libfontconfig --enable-libharfbuzz --enable-libass --enable-libopus --enable-libvorbis --enable-libtheora --enable-libsnappy --enable-libopenjpeg --enable-libaom --enable-libwebp --enable-libzimg --enable-libsvtav1 --enable-libkvazaar --enable-libbluray --enable-libvmaf --enable-libvvenc --enable-liboapv --enable-libdav1d --enable-librubberband --enable-libjxl --enable-libplacebo --enable-libsoxr --enable-ffplay --enable-sdl2 --enable-ffprobe
```

**Windows**

```text
--prefix=/ffbuild/prefix --pkg-config-flags=--static --pkg-config=pkg-config --cross-prefix=x86_64-w64-mingw32- --arch=x86_64 --target-os=mingw32 --enable-version3 --disable-debug --disable-w32threads --enable-pthreads --enable-iconv --enable-zlib --enable-libxml2 --enable-libvmaf --enable-fontconfig --enable-libharfbuzz --enable-libfreetype --enable-libfribidi --enable-lcms2 --enable-vulkan --enable-libshaderc --enable-libdav1d --enable-libvorbis --enable-librav1e --enable-librsvg --disable-libxcb --disable-xlib --disable-libpulse --enable-gmp --enable-lzma --enable-liblcevc-dec --enable-opencl --enable-amf --enable-libaom --disable-avisynth --enable-chromaprint --disable-libdavs2 --disable-libdvdread --disable-libdvdnav --disable-libfdk-aac --enable-ffnvcodec --enable-cuda-llvm --disable-frei0r --enable-libgme --enable-libkvazaar --enable-libaribb24 --enable-libaribcaption --enable-libass --enable-libbluray --enable-libjxl --enable-libmp3lame --enable-libopus --enable-libplacebo --enable-librist --enable-libssh --enable-libtheora --enable-libvpx --enable-libwebp --enable-libzmq --enable-lv2 --enable-libvpl --enable-openal --enable-liboapv --enable-libopencore-amrnb --enable-libopencore-amrwb --enable-libopenh264 --enable-libopenjpeg --enable-libopenmpt --disable-librubberband --enable-schannel --enable-sdl2 --enable-libsnappy --enable-libsoxr --enable-libsrt --enable-libsvtav1 --enable-libtwolame --enable-libuavs3d --disable-libdrm --enable-vaapi --enable-vapoursynth --disable-libvidstab --enable-libvvenc --disable-whisper --disable-libx264 --disable-libx265 --disable-libxavs2 --disable-libxvid --enable-libzimg --enable-libzvbi --extra-cflags=-DLIBTWOLAME_STATIC --extra-cxxflags= --extra-libs=-lgomp --extra-ldflags=-pthread --extra-ldexeflags= --cc=x86_64-w64-mingw32-gcc --cxx=x86_64-w64-mingw32-g++ --ar=x86_64-w64-mingw32-gcc-ar --ranlib=x86_64-w64-mingw32-gcc-ranlib --nm=x86_64-w64-mingw32-gcc-nm --extra-version=20261031
```
