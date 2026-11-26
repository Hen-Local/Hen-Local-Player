# Development guide

Run commands from the repository root. On Apple Silicon, use macOS 14 or later, Xcode Command Line Tools, CMake 3.25+, Python 3, Rust 1.92 and Node.js 20.19+ or 22.12+.

## Run the desktop app

```sh
npm --prefix app/ui ci
bash scripts/dev.sh
```

The first run prepares the pinned MLX runtime and matching Metal kernels. For manual builds, source `scripts/build_env.sh` in Bash. Each worktree needs its own `CARGO_TARGET_DIR` with a path free of spaces.

On Windows x64, install the MSVC Rust toolchain, Visual Studio 2022 Build Tools, Node.js and WebView2. Run `.\scripts\dev.ps1` in PowerShell. The default Cargo target directory is `C:\hlp-target`.

## Preview the interface in a browser

```sh
npm --prefix app/ui ci
npm --prefix app/ui run dev
```

Open the address printed by Vite and choose **Open File** to play a browser-compatible video or audio file. Use the desktop app for on-device transcription and translation.

## Generate subtitles from the command line

The CLI shares its processing pipeline with the desktop app. On macOS, run the following in Bash, replacing `movie.mp4` with a media file. This example translates to Chinese and exports three subtitle formats.

```sh
source scripts/build_env.sh
cargo build --locked -p hlp-cli -p hlp-asr-worker
"$CARGO_TARGET_DIR/debug/hlp-cli" status
"$CARGO_TARGET_DIR/debug/hlp-cli" setup --yes
"$CARGO_TARGET_DIR/debug/hlp-cli" subtitle movie.mp4 --to zh --formats srt,vtt,ass
```

Models and managed tools live in `~/.OminiX`. Set `HLP_OMINIX_ROOT` for a separate installation. Set `HLP_DATA_DIR` to isolate preferences, recent files and subtitle caches. The CLI option `--no-cache` processes a file in a temporary directory.

## Test the player

Run `npm --prefix app/ui run check` for frontend checks. After installing the models, run `bash scripts/make_test_media.sh` to generate local speech fixtures, then `bash scripts/e2e.sh` to test all nine files. Use `bash scripts/e2e.sh ja fr` to test just Japanese and French. Reports go to `tests/results/`.

The macOS fixture generator needs FFmpeg with VideoToolbox, VP9, Opus and MP3 encoding. Use `HLP_FFMPEG` to select a compatible executable. Windows speech fixtures use Python 3 and an English desktop SAPI voice. [Windows testing](progress/g-windows.md) · [Project test results](PROGRESS.md)

## Package and release

For a local UI change, run `bash scripts/build_macos_app.sh --app-only` to rebuild `dist/Hen Local Player.app` at the current version. Existing installers and update archives are preserved.

On Apple Silicon, install the packaging tools and build the app and DMG.

```sh
brew install meson ninja pkgconf
bash scripts/build_macos_app.sh
open "dist/Hen Local Player.app"
```

Quit any running copy with **⌘Q** before opening a rebuilt app. Local builds use an ad-hoc signature. To sign and notarize a distribution build, set `HLP_DEVELOPER_ID` to the Developer ID Application identity and `HLP_NOTARY_PROFILE` to the notarization Keychain profile. The app includes the FFmpeg and dav1d source archives and build recipe. On Windows, `.\scripts\build_windows.ps1` creates an x64 NSIS installer in `dist`.

Use `python3 scripts/version.py set <version>` and `python3 scripts/version.py check` to prepare the next release. Tag the matching commit and run the **Publish signed macOS release** workflow on that tag. [Signed release guide](progress/release-updates.md)

## License and optional support

The source uses the Apache License 2.0. Bundled tools, assets and downloaded models have their own terms. [License](../LICENSE) · [Third-party notices](LICENSES.md)

To enable a Buy Me a Coffee link in Settings, set `VITE_BUY_ME_A_COFFEE_USERNAME` when building the frontend. An empty value keeps the link hidden. To enable GitHub sponsorship, copy the funding example to `.github/FUNDING.yml` and enter the same username. [Funding example](../.github/FUNDING.yml.example)
