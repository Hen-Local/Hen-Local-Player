# Hen Local Player

A local video player with on-device speech recognition, translation and bilingual subtitles. The desktop app and the headless CLI share the same processing pipeline.

## Open the player

On Apple Silicon, open the DMG and drag **Hen Local Player** to Applications. In Finder, choose **Open With → Hen Local Player** on a movie, or launch the player and choose **Open file** (⌘O), **Open folder** (⇧⌘O), or **Play sample**. FFmpeg and FFprobe are included in the app. Ordinary playback and authored text subtitles work offline from the first launch.

- Select multiple files to play them in order. **Playlist** lets you add, remove and skip files. Playback resumes when you reopen an unfinished movie.
- Click the video to play or pause. Double-click for fullscreen. The transport includes volume, speed, ten-second skips, repeat and picture-in-picture on supported systems.
- **Playback** includes time and chapter navigation, A–B repeat, video fit/fill, audio track selection and saving a PNG frame.
- Open **Subtitles** to use an embedded text track or an SRT/WebVTT file, adjust subtitle timing, or choose locally generated bilingual captions. Install the speech and translation components from **Models & tools** when you want generated subtitles. **Transcript** lets you jump to a line and **Export** saves generated subtitles as SRT, WebVTT or ASS.
- **Open link** previews a video before downloading it. URL tools install on first use. General settings include the four Hen Local Translator accent colors.

### Browser preview

From the repository root, run the following commands and open the local address printed by Vite. **Play sample** includes a Japanese clip and authored demo captions. You can also open browser-compatible files from your computer. The desktop app runs the speech models and handles additional formats.

```sh
npm --prefix app/ui ci
npm --prefix app/ui run dev
```

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

On an Apple Silicon Mac, run `bash scripts/build_macos_app.sh`. It stages the pinned playback tools and produces `dist/Hen Local Player.app` and a versioned arm64 DMG. Set `HLP_BUNDLE_TOOL_CACHE` to reuse an existing component cache during packaging. The internal build uses an ad-hoc signature. Public distribution requires a Developer ID signature and notarization.

On Windows x64, run `.\scripts\build_windows.ps1` from a checkout with the MSVC tools installed. The script builds the worker, includes it beside the app executable, and writes a versioned NSIS installer to `dist`. The Windows fixture scripts also require Python 3 and an English desktop SAPI voice. The manual **Windows installer and fixtures** GitHub Actions workflow can build the installer and run the speech tests on a hosted Windows runner when the workflow is published and dispatched. [Windows test commands and verification status](docs/progress/g-windows.md).

See [implementation evidence](docs/PROGRESS.md) for tested milestones.
