# Hen Local Player

A local video player with on-device speech recognition, translation and bilingual subtitles. The desktop app and the headless CLI share the same processing pipeline.

## Development

Use an Apple Silicon Mac running macOS 14 or later with Xcode Command Line Tools, CMake 3.25+, Python 3, Rust 1.92 and Node 20.19 or later. Windows development requires the MSVC Rust toolchain, VS 2022 Build Tools and WebView2.

```sh
npm --prefix app/ui ci
bash scripts/dev.sh
```

On Windows run `scripts/dev.ps1`. The macOS developer script builds the pinned MLX 0.30.1 libraries for macOS 14 and obtains matching Metal kernels on its first run. The scripts keep Rust build products outside paths containing spaces. For manual builds on macOS, export `CARGO_TARGET_DIR="${TMPDIR:-/tmp}/hen-local-player-target"` before running Cargo. On Windows set it to `C:\hlp-target`.

## Headless workflow

```sh
source scripts/build_env.sh
cargo build --locked -p hlp-cli -p hlp-asr-worker
"$CARGO_TARGET_DIR/debug/hlp-cli" status
"$CARGO_TARGET_DIR/debug/hlp-cli" setup --yes
"$CARGO_TARGET_DIR/debug/hlp-cli" subtitle movie.mp4 --to zh --formats srt,vtt,ass
```

Shared models and managed tools live in `~/.OminiX`. Set `HLP_OMINIX_ROOT` to use an independent installation. Generate local speech fixtures on macOS with `bash scripts/make_test_media.sh`. The fixture generator needs FFmpeg with VideoToolbox, VP9, Opus and MP3 encoding. Set `HLP_FFMPEG` to a compatible binary if needed. Generated media and model weights stay out of Git.

See [implementation evidence](docs/PROGRESS.md) for tested milestones.
