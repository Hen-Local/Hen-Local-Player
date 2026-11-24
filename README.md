# Hen Local Player

A local video player with on-device speech recognition, translation and bilingual subtitles. The desktop app and the headless CLI share the same processing pipeline.

## Open the player

On Apple Silicon, open the DMG and drag **Hen Local Player** to Applications. The compact opening window offers **Open File** (⌘O) and **Open URL** (⌘L). After you have watched a movie, you can reopen it from the short recent-files list. Press **⌘W** to close the window. Click the player’s Dock icon to return to the opening window, or press **⌘Q** to quit. In Finder, **Open With → Hen Local Player** starts playback directly. The **•••** menu includes **Open Folder** and, on the opening screen, **Play Sample**.

- The floating controls appear when the pointer enters the player and hide when it leaves, including while paused. They also fade during playback when the pointer rests over the picture. The translucent bar shows volume, playback speed, ten-second skips and a timeline. Click the speed button to choose 0.5×, 0.75×, 1×, 1.25×, 1.5×, 2× or 4×. The slider covers 0.25× to 5×. On supported videos, **Picture in Picture** opens a floating video window. Click it again to return. Drag the picture or the background of the opening window to move the player. Double-click the picture or press **F** to enter fullscreen. On Mac, you can also click the green window button to open the native fullscreen Space. Press **Esc** to return.
- **Playlist** opens on the left and **Subtitles** on the right. Both can stay open together. Video subtitles stay centered on the picture when either panel opens. If the playback controls cover the subtitle position, the subtitles rise above them and return to the chosen height when the controls hide. When space is tight, either sidebar covers the part of the playback controls beneath it. Click outside to dismiss the active sidebar. Opening a single local file lists the playable files in its folder, sorted by name, with the playing item highlighted. Click a row to switch files. Repeat, Previous File and Next File are grouped at the bottom of the playlist. Selecting multiple files or using **Add Files** creates a custom playlist. Removing an item changes only the playlist. Reopening an unfinished movie resumes playback.
- The **Subtitles** sidebar finds embedded text tracks and nearby SRT/WebVTT files. If a movie has no captions, choose **Transcribe** at the bottom of the sidebar. Progress appears there during generation. Once finished, **Regenerate Subtitles** appears at the center of the right sidebar’s footer. It transcribes the selected audio track again and updates its generated subtitles. Expand **Subtitle List** to read the timestamped lines or click one to seek. Under **Languages**, choose the spoken and translation languages, which ones to show and their order. **Appearance & Timing** adjusts text size, the distance from the bottom of the picture (5% by default), background opacity and subtitle delay. Click a sidebar’s button again to close it. Escape closes the focused sidebar.
- Choose **Find Online Subtitles…** from the Subtitles menu (⇧⌘D) or **Find Online…** in the right sidebar, enter the movie title, choose a language, then choose **Use Subtitle** beside the release you want. Guest search uses Subtitle Search Engine and requires no account or API key. The provider applies a daily guest allowance. [Subtitle Search Engine](https://subtitles.website/docs/).
- The player saves generated subtitles in its local cache for each movie and audio track. Reopening a completed track plays the saved text. To save an SRT beside the movie or use it in another player, choose **Subtitles → Export Generated Subtitles…** (⌘E on Mac). To load your own SRT or WebVTT, choose **Subtitles → Import Subtitles…**. Imported and downloaded tracks can also be reopened offline.
- Generated subtitles use short phrases with spaces in place of sentence punctuation. Each language keeps its font size across cues. On a narrow picture, a phrase that exceeds the available width advances in timed parts. In bilingual mode the two rows are stacked. Reopening a movie applies this formatting to its saved transcription and translation. Under **Settings → Processing**, **Use Previous Dialogue for Translation** lets the local translation model read recently spoken lines in the same language. New settings enable it by default.
- When an audio file has subtitles, the player highlights the current phrase and scrolls the surrounding lines in time with playback. Click a line to hear it again. Display Language, Language Order, Text Size and Vertical Position also apply in this view.
- FFmpeg, FFprobe and the speech detector are included. Ordinary playback and authored text subtitles work offline from first launch. The player uses installed speech and translation models and offers to install individual missing components when you request captions. **Open URL** previews a video before downloading it and offers to install URL tools on first use.

### Browser preview

From the repository root, run the following commands and open the local address printed by Vite. Choose **••• → Play Sample** for the Japanese clip or **Open File** for a browser-compatible movie. The Mac app runs local speech models and handles additional formats.

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

When testing a rebuilt Mac app, quit the running copy with **⌘Q** and open the new bundle. While the old process is running, both the Dock icon and the terminal’s `open` command bring its existing window forward.

On an Apple Silicon Mac, run `bash scripts/build_macos_app.sh`. It stages the pinned playback tools and speech detector and produces `dist/Hen Local Player.app` and a versioned arm64 DMG. Set `HLP_BUNDLE_TOOL_CACHE` to reuse an existing component cache during packaging. The internal build uses an ad-hoc signature. Public distribution requires a Developer ID signature and notarization.

On Windows x64, run `.\scripts\build_windows.ps1` from a checkout with the MSVC tools installed. The script builds the worker, includes it beside the app executable, and writes a versioned NSIS installer to `dist`. The Windows fixture scripts also require Python 3 and an English desktop SAPI voice. The manual **Windows installer and fixtures** GitHub Actions workflow can build the installer and run the speech tests on a hosted Windows runner when the workflow is published and dispatched. [Windows test commands and verification status](docs/progress/g-windows.md).

See [implementation evidence](docs/PROGRESS.md) for tested milestones.
