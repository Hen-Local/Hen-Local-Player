# Hen Local Player

A local video player with on-device speech recognition, translation and bilingual subtitles. The desktop app and the headless CLI share the same processing pipeline.

## Open the player

On Apple Silicon, open the DMG and drag **Hen Local Player** to Applications. Launch the app and choose **Download missing** in Model setup. The default Mac setup downloads approximately 3.67 GB. Drop a video into the player or use **Open file**. Choose the spoken language or leave it on Auto, then choose the subtitle language. **Open link** previews a video before downloading it. The URL tools install on first use.

## Development

Use an Apple Silicon Mac running macOS 14 or later with Xcode Command Line Tools, CMake 3.25+, Python 3, Rust 1.92 and Node 20.19 or later. Windows development requires the MSVC Rust toolchain, VS 2022 Build Tools and WebView2.

```sh
npm --prefix app/ui ci
bash scripts/dev.sh
```

On Windows run `scripts/dev.ps1`. The macOS developer script builds the pinned MLX 0.30.1 libraries for macOS 14 and obtains matching Metal kernels on its first run. The scripts keep Rust build products outside paths containing spaces. For manual builds on macOS, run `source scripts/build_env.sh` to configure the target directory and the pinned MLX libraries. On Windows set it to `C:\hlp-target`.

## Headless workflow

```sh
source scripts/build_env.sh
cargo build --locked -p hlp-cli -p hlp-asr-worker
"$CARGO_TARGET_DIR/debug/hlp-cli" status
"$CARGO_TARGET_DIR/debug/hlp-cli" setup --yes
"$CARGO_TARGET_DIR/debug/hlp-cli" subtitle movie.mp4 --to zh --formats srt,vtt,ass
```

Shared models and managed tools live in `~/.OminiX`. Set `HLP_OMINIX_ROOT` to use an independent installation. Generate local speech fixtures on macOS with `bash scripts/make_test_media.sh`. The fixture generator needs FFmpeg with VideoToolbox, VP9, Opus and MP3 encoding. Set `HLP_FFMPEG` to a compatible binary if needed. Generated media and model weights stay out of Git.

Run `bash scripts/e2e.sh` after generating the fixtures and installing the components. It tests all nine media files against their speech timing references and writes a machine-readable report under `tests/results/`. Pass fixture names such as `ja fr` for a shorter run. Set `HLP_DATA_DIR` to isolate settings, recent files and subtitle caches. The CLI’s `--no-cache` option processes a file in a fresh temporary directory.

## Build an installer

On an Apple Silicon Mac, run `bash scripts/build_macos_app.sh`. It produces `dist/Hen Local Player.app` and a versioned arm64 DMG. The internal build uses an ad-hoc signature. Public distribution requires a Developer ID signature and notarization.

On Windows x64, run `.\scripts\build_windows.ps1` from a checkout with the MSVC tools installed. The script builds the worker, includes it beside the app executable, and writes a versioned NSIS installer to `dist`. The Windows fixture scripts also require Python 3 and an English desktop SAPI voice. The manual **Windows installer and fixtures** GitHub Actions workflow can build the installer and run the speech tests on a hosted Windows runner when the workflow is published and dispatched. [Windows test commands and verification status](docs/progress/g-windows.md).

See [implementation evidence](docs/PROGRESS.md) for tested milestones.
