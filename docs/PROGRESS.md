# Prototype progress

A checked item has been exercised. Worktree branches and tags are local until publication is requested.

## Subtitle language, formatting and fullscreen animation · November 23, 2026

- [x] Opened a saved Chinese recording in a separate installation from the rebuilt Apple Silicon disk image. **Player Settings → Subtitles → Subtitle Settings → Display Language** switched among Chinese, English and both. In bilingual mode the English and Chinese text each occupied one row. The 56 subtitle pairs restored with zero speech recognition or translation requests. The longest English caption was 39 characters. The English and Chinese SRT exports contained one row per cue and the bilingual export contained two.
- [x] Used the same packaged app to transcribe a fresh Japanese recording. The progress bar appeared on starting transcription and reached 100% while **Subtitle List** stayed collapsed. Expanding it revealed seven timed pairs. Switching the target to French produced different on-screen text and saved a French translation. Live size and vertical position changes moved and resized the captions. Chrome and WebKit also exercised the localized controls, incomplete caches, retry and existing subtitle files. [Sidebar checks](progress/player-subtitles.md#subtitle-settings-and-generation-progress).
- [x] A long bilingual cue fit inside the picture at 320, 450, 720 and 1920 pixels wide, including tests with text size set to 200%. A late translation filled its reserved row while the original text stayed at the same height. Core and export tests cover punctuation, lexical marks, timing, and physical line counts. All 81 release checks across nine media fixtures passed. [Core checks](progress/a-core.md#generated-subtitles-on-one-line-per-language) · [Fixture results](../tests/results/2026-11-23-darwin-single-line.json).
- [x] Native tests measured 216–224 ms of visible window movement on Mac fullscreen entry and exit. The playing video rendered at 13–14 intermediate widths. Rapid reversals restored the window and testing with Reduce Motion enabled showed only the initial and final sizes. [Fullscreen checks](progress/player-shell.md#animated-player-fullscreen).
- [x] `cargo fmt --all --check` and all 49 regular tests in `cargo test --locked --workspace` passed. The production UI built and `npm --prefix app/ui run check` reported zero errors and warnings. The Windows x64 desktop app, CLI and worker passed the local cross-check. Strict signature and disk-image verification passed for the Apple Silicon build. Installer size 126,293,878 bytes, SHA-256 `513354e855f76aa76cbf4edd67e4f86245a54d264e8aa09c040e1fe8e7b4ce7b`.

## Mac window closing, overlay panels and pointer controls · November 23, 2026

- [x] In the previous Mac release, pressing ⌘W on the last video exited the process. With the new behavior, the window count dropped to zero, the movie's cache lock was released, and pressing the actual Dock icon reopened **Open file** and **Open URL** in the same process. Finder reopened a previously closed movie at seven seconds. Separate checks covered closing in player fullscreen and in a macOS fullscreen Space, opening files or settings from the menu with no window, and ⌘Q. [Mac window checks](progress/player-shell.md#closing-and-reopening-the-mac-window).
- [x] At a native viewport of 721 × 430, opening both panels in the previous release reduced the video width to 342 pixels. In the updated app, toggling either or both panels kept the video, subtitle area and ten transport controls at their original bounds, including in 2580 × 1080 fullscreen. Browser checks at six sizes exercised the 180 ms slide, rapid reversal, reduced motion and the A–B loop while settings were closed. [Overlay panel checks](progress/player-subtitles.md#sidebars-over-the-picture).
- [x] Sent native mouse movements between the Mac window and desktop. The transport disappeared on exit and reappeared on entry in all four combinations of paused/playing and settings open/closed. Chrome and WebKit also exercised keyboard focus, an open menu, dragging a slider across the edge and waking the controls by touch. [Pointer checks](progress/player-transport.md#pointer-leaving-the-player).
- [x] The final read-only disk image supplied the independently installed app for the window, pointer and panel tests. Disk image and strict app signature verification passed. The Rust workspace passed 47 tests, Svelte reported zero errors or warnings, and the Windows x64 desktop, CLI and worker passed the local cross-check. Apple Silicon installer size 126,282,725 bytes, SHA-256 `f26043c8f72c91f253d4acaadf3258573ab7bfe9de1f444d9f418aeb26ffaa70`.

## Readable bilingual subtitles · November 23, 2026

- [x] Opened the same 8-minute 12-second movie in the previous and updated Mac releases using separate copies of its saved Chinese transcription and English translation. Both runs used zero recognition and translation requests. The 68 utterances produced 104 timed subtitle pairs before and 204 after. The longest English cue in the player fell from 179 to 56 characters. The Chinese and English exports each retained the same non-whitespace text in the same order. The longest English SRT line fell from 90 to 42 display columns.
- [x] Mounted the updated disk image, copied out the app, opened the movie and selected its longest English cue from the subtitle list. At 720 × 430, its widest rendered English line measured 368 pixels, down from 638 for the previous release's longest cue. At 2580 × 1080 fullscreen, the corresponding measurements were 982 and 1700 pixels. The updated English cue occupied two lines and its Chinese counterpart one line in both windows. All glyphs stayed inside the picture. [Additional window sizes and imported subtitles](progress/player-subtitles.md#subtitle-line-length-on-screen).
- [x] `cargo test --locked --workspace` passed 47 tests. Two additional fixture-backed tests passed for embedded subtitle tracks and on-demand transcription and cache reuse. New regressions cover English translation expansion, unspaced word boundaries, decimal numbers, very short speech and an indivisible source word. The release CLI passed all 81 checks across nine fixtures, including export decoding and cache reuse. [Fixture results](../tests/results/2026-11-23-darwin-readable-captions.json).
- [x] Svelte checks passed with zero diagnostics, the production UI built, and the local Windows x64 cross-check passed for the desktop app, CLI and speech worker. The Apple Silicon disk image and app passed disk-image and strict signature verification. Both bundled ICU notices matched their source files. Installer size 126,251,704 bytes, SHA-256 `43de8a9017d922557e8e42b9de5b6b741d00034fe1f277fd0bb1cd24278ad619`.

## Scrolling and window dragging · November 23, 2026

- [x] A sideways trackpad gesture previously displaced the whole Mac player by up to 38 pixels. An unbroken subtitle also allowed the list to scroll sideways by 528 pixels. In the fixed player, horizontal movement stayed at zero while vertical scrolling still moved the list. Focused seek and volume sliders retained their values during horizontal input. [Wheel and trackpad checks](progress/player-transport.md#horizontal-scrolling).
- [x] In an independently installed copy of the Mac app, a left-button drag on the welcome background moved the window 70 pixels right and 28 down. Dragging a paused movie moved the window 70 pixels left and 28 down while the playhead stayed at five seconds. A playing movie kept advancing during a drag. Dragging the background also worked immediately after using the seek slider. A click played the movie, a double-click entered fullscreen, and dragging the seek slider changed the playback position with the window stationary. [Window drag checks](progress/player-shell.md).
- [x] The production UI build and Svelte checks passed with zero errors or warnings. The arm64 disk image and its app signature passed verification. Installer size 123,949,149 bytes, SHA-256 `e895b87793396cf8554377318cd6ec6bdf7bb4af44b4b242c5d907d8e01ffb2e`.

## Playback and subtitle fixes · November 23, 2026

- [x] The two sidebars open and close independently. In native WKWebView, all ten transport targets kept the same window-relative bounds and passed hit tests with neither, either and both sidebars open, at 720 × 430 and 2580 × 1080. Touch checks covered 320- and 390-pixel widths. [Transport results](progress/player-transport.md).
- [x] The fullscreen button, F and double-click fill the current Mac display in place. Native mouse and keyboard events restored the original window and keyboard focus. Sampled transition times were 35–38 ms. The green button uses the macOS Spaces transition. On the same machine it took about 0.6 seconds and the sample kept playing. [Fullscreen checks](progress/player-shell.md).
- [x] Reproduced the reported setup condition with the existing speech and translation models and no user-installed Silero detector. A mounted installer copy supplied its own 643,854-byte detector. Its Subtitles panel opened the reported 110-second clip from the old empty cache, offered **Transcribe and create subtitles**, and generated eight Chinese lines and eight English translations. Clicking a timed line showed its caption over the video. Playback kept the same video element and media URL. Reopening restored all eight lines. A separate run reopened an interrupted request with no saved lines and finished automatically.
- [x] With an empty model directory the same app named the selected speech model, translation model and translation runtime, each with an enabled download button. Browser checks covered individual install failures, retry, automatic resume, original-language-only generation, a silent file and switching movies during a pending request. The bundled sample automatically selected its six-line bilingual SRT. Explicitly choosing the generated track offered transcription in the same panel.
- [x] A backend test with real speech confirmed idle playback used zero inference calls and created no PCM, two consecutive start requests produced six lines with exactly six speech requests, and reopening used zero additional requests. The release CLI passed all 81 checks on nine fixtures, including the 359-cue long movie. [Fixture results](../tests/results/2026-11-23-darwin-subtitles.json).
- [x] All 42 regular Rust tests, the additional real embedded-track and on-demand generation tests, Svelte checks, production UI build and local Windows x64 cross-check passed. Native Windows playback remains untested. Opening a file from Finder at cold launch populated the playlist on the first event, including when macOS delivered it before the player window was ready.

The locally signed Apple Silicon installer passed disk image and strict code-signature verification. It is 123,947,712 bytes with SHA-256 `adfc7b607215b003e93a89a56993cd526471eff4f954f8b0c67011a33f33af74`.

## Compact player window · November 23, 2026

The first compact Apple Silicon package opened in a centered 720 × 430 window with the Hen mark and **Open file** and **Open URL**. A returning viewer sees up to four recent movies. **More → Play sample** opens the bundled Japanese film. The playback surface uses the full window with a floating controller. Playlist opens on the left and Player settings on the right. In that release, a sidebar made room beside the movie on wider windows and overlaid it on narrower windows. [Mac opening window](screenshots/player/compact-window.webp) · [Floating controls](screenshots/player/compact-playback-native.webp) · [Subtitle sidebar](screenshots/player/compact-sidebar.webp).

- [x] Native macOS inspection found a non-opaque, resizable window, standard traffic lights and an AppKit HUD material. Changing the solid background behind the window changed the same 80 × 80 pixel patch from RGB(84, 29, 31) to RGB(25, 50, 90). A real title-bar drag moved the window. It clamped a 300 × 200 resize request to its 450 × 300 minimum. In a copy installed from the DMG, **Open file** presented the native picker and **Open URL** presented the URL form. Both cancelled back to the opening window.
- [x] The installed Mac app decoded the packaged sample at 1280 × 720 with Japanese/English captions. Native button presses opened Playlist on the left, Player settings on the right and the Playback tab with speed, repeat, chapter/time navigation where available, audio tracks and frame capture.
- [x] Chrome exercised actual media playback at six widths between 320 and 1280 pixels. At 720 × 430, the floating controls measured 540 × 66, the play button remained centered and visible captions cleared the controls by eight pixels. Mouse, touch and keyboard checks covered automatic hiding, waking without pausing, scrubbing, mute, fullscreen and toggling each sidebar. At 720 pixels wide the left playlist occupied x=0–248 and the right settings occupied x=472–720. At 320 pixels wide the open sidebar stayed onscreen and scrolled independently.
- [x] The right sidebar selected subtitle tracks, adjusted live captions from an imported SRT, changed playback speed to 1.5×, switched repeat modes, preserved an A–B loop across panel closure, and entered and exited picture-in-picture. The sample transcript had six timed rows. English and Chinese keyboard tests covered tab navigation, Escape and focus restoration. All four Hen accents and the reduced-transparency preference were exercised.
- [x] `npm --prefix app/ui run check` returned zero errors and warnings. The production UI build, Rust formatting, `cargo check --locked -p henlocal-player` and all four desktop subtitle-track tests (including real embedded streams) passed locally. The Apple Silicon package passed disk image and strict code-signature verification. Native Windows visual verification remains pending.

Previous compact-window installer: `dist/Hen-Local-Player-0.1.0-macos-arm64.dmg`, 123,440,981 bytes. SHA-256 `2c20d7073c56650cc031b173bad87dee1c37368d9201b756ba68f3c1261d456d`. The local package is ad-hoc signed.

## Standalone player revision · November 23, 2026

- [x] The desktop opens to a media library in the Hen Local Translator style. It includes the original hen mark, four persistent accent colors and a 22.5-second animated Japanese sample with six bilingual lines. A movie takes over the main window. Subtitles, the transcript and the playlist open on demand. Chrome checks covered the full-size window, fullscreen keyboard navigation and 320-pixel layouts. [Screenshots and interaction checks](progress/player-shell.md).
- [x] Finder can open multiple movies directly in the player. The file and folder pickers, playlist, resume, repeat, fullscreen, timed picture-in-picture subtitles and system media keys are connected. Chapter selection, jump to time, A–B repeat, fit/fill, audio tracks and PNG frame capture are available during playback. A native test switched audio while paused at 7.1 seconds and sought to a chapter at 8 seconds. Opening a bad file preserved playback. [Playback checks](progress/player-transport.md).
- [x] Embedded text tracks, adjacent or imported SRT/WebVTT, subtitle delay and a clickable transcript work without model installation. Native WKWebView selected a forced Japanese track, displayed the sample’s six bilingual lines and sought to 7.767 seconds from its third transcript row. Per-movie preferences survived switching files and restarting the browser. [Subtitle checks](progress/player-subtitles.md).
- [x] The Apple Silicon disk image was mounted read-only, copied into an independent directory and verified with strict code signing checks. The bundled sample, poster and SRT matched the source assets. The installed app used its packaged tools with external networking and access to the developer models and build products denied. It opened the sample directly and remuxed a silent, chaptered MKV to a 22.5-second H.264 MP4 with zero audio streams. Both model directories remained empty. Three successive Finder opens on the installed app left the newest file first in the library. The sample button was exercised in the production browser preview. The installed-app test used an open-file event path because its accessibility probe could not locate that button.
- [x] Rust formatting, 42 regular Rust tests, the additional real embedded-track test, both fixture-backed media tests, the production UI build and the Svelte check passed. The release CLI passed all 81 checks on nine speech and media fixtures. The long fixture produced 359 bilingual cues in 65.72 seconds and reopened in 45 ms. [Fixture results](../tests/results/2026-11-23-darwin-player.json).
- [x] Windows x64 code passed the local Rust cross-check, and PowerShell parsed the updated packaging script. Native Windows installation, WebView2 playback and speech inference still need a Windows host. All development and packaging stayed local.

Previous standalone-player installer, 123,442,074 bytes. SHA-256 `e8cedb95e98ba6e787afb9dcfd1ff5bbdc16a8278d25f886f7ca4ce7b400fa2c`. The package uses an ad-hoc signature for local testing.

## Original prototype

The lead merged phase 2 in order (A, C, B, D, E), verified the CLI slice on macOS, then merged F followed by G. The original tree passed Rust formatting, 38 regular Rust tests, the Svelte check with zero warnings, the production UI build and the seven-package Windows x64 cross-check.

The M0–M9 evidence below records the original prototype at `d3768fe`.

## Foundation and lane evidence

### M0 · Workspace

- [x] Core contracts and settings tests pass.
- [x] Svelte checks pass.
- [x] Development script opens the titled Tauri window.
- [x] Shared Cargo check and MLX worker build pass.
- [x] All nine media fixtures and demo generated.

Evidence on Apple Silicon, 2026-11-23. `cargo test -p hlp-core` passed 3 foundation tests. The six-package check from §9.1.1 succeeded, `cargo build -p hlp-asr-worker` compiled the pinned MLX dependencies, and `npm --prefix app/ui run check` reported 0 errors and 0 warnings. `bash scripts/dev.sh` launched Tauri and the macOS window list returned “henlocal-player Hen Local Player”. `HLP_FFMPEG=/tmp/hlp-fixture-tools/ffmpeg bash scripts/make_test_media.sh` generated nine media files, nine timing references and the browser demo (141 MiB of fixtures).

### M1 · Core logic

- [x] Core unit and property tests from §8.1 pass. Lead reran the shared six-package Rust check, UI check and 18 core tests on the merge result. Golden exports and 256 randomized timeline cases passed. [Lane A evidence](progress/a-core.md).

### M2 · Media

- [x] Probe and playback decisions match the fixtures.
- [x] Ten-minute extraction meets the sample-count tolerance.
- [x] Byte ranges, download progress and cancellation tests pass.

Lead reran the shared checks, seven regular media tests and both fixture-backed tests after rebasing B on C. The 600-second extraction produced exactly 9,600,000 samples. Both prepared videos parsed and the loopback yt-dlp download matched the original. [Lane B evidence](progress/b-media.md).

### M3 · Models and setup

- [x] Existing ASR, GGUF and llama runtime are ready.
- [x] Verify managed Silero and FFmpeg installs under a temporary root.
- [x] Interrupted downloads resume and manifest pins pass validation.

Lead verified the merged result with six offline installer tests, the shared Cargo and UI checks, and a separate temporary installation. `hlp-cli status` recognized both ASR models, the GGUF and llama.cpp. `hlp-cli setup --yes` fetched only Silero (643,854 bytes) and FFmpeg/FFprobe (57,152,216 bytes). A second status reported both Ready with URL tools still optional. [Lane C evidence](progress/c-models.md).

### M4 · ASR worker

- [x] Japanese auto and forced inference pass on real speech.
- [x] Tokenizer and protocol checks pass.
- [x] A killed worker is restarted.

Lead built the macOS 14 native libraries from verified sources, reran worker unit tests, the shared Rust/UI checks and the real kill/reload regression (4.59 s). A separate five-request JSONL run returned the expected first fixture sentence in both automatic and forced Japanese modes. [Lane D evidence](progress/d-asr.md).

### M5 · Pipeline and CLI

On 2026-11-23, `bash scripts/e2e.sh` passed all nine fixtures on the M4 Pro using installed local models. Normalized character error ranged from 0% to 2.86%. The largest onset error was 216 ms. The 20-minute file produced 359 cues in 109.8 seconds and reopened in 0.529 seconds. Every second run reported zero ASR and translation requests. [Machine-readable results](../tests/results/2026-11-23-darwin.json).

On the long fixture, ASR processed speech at 6.0× real time in the developer build, below the §7 goal of 15×. Mean translation time was 68 ms per utterance. The six-package Rust check, Svelte check, core, media, installer and pipeline tests passed. Dedicated regressions cover stale source overrides, a split language vote and safe cache eviction. The CLI returned exit 2 for missing setup, 3 for a missing file, and 4 for an unavailable worker. All 45 languages appeared in `hlp-cli langs`.

- [x] Detected languages match truth.
- [x] Normalized character error rate ≤ 15%.
- [x] Every truth onset has a cue within 600 ms.
- [x] Cue intervals are positive and non-overlapping.
- [x] Translations present and same-language translation skipped.
- [x] SRT, VTT and ASS parse.
- [x] Music yields zero cues.
- [x] Cached runs finish within two seconds.
- [x] Direct, Remux and Transcode plans match fixture expectations.

### M6 · Desktop player

Phase 4 started from `slice-v1` (`3e7522d`). Lane F owns the native desktop and URL work in `codex/lane-f`. Lane G owns the Windows port in `codex/lane-g`. Both use independent worktrees and build directories.

Mock UI reviewed and merged after D. The six-package check, Svelte check and production build passed on the merged tree. Eight screenshots and three browser smoke runs cover the mock flows. Native acceptance is tracked below. [Lane E evidence](progress/e-ui.md).

Lead reviewed the native implementation and reran the shared Rust check, Svelte check, production build and media tests. A real external file drop started playback in 287 ms. The Japanese badge reached 100% in 1.194 s. At 15:00 the long fixture displayed a new subtitle in 1.32 s. A separate app process restored six cached cues in 146 ms. On a cold French remux, playback started at 215 ms and the first cue arrived at 1.747 s. With the playable evicted and the transcript retained, the player restored all six cues before playback at 188 ms. [Native checks](progress/f-app.md) · [Screenshots](screenshots/native/).

- [x] Empty state shows the drop zone and the recent list. Dropping a file plays it within 1 s (Direct).
- [x] The language badge shows "Japanese 9x %" within 15 s, and subtitles appear and stay in sync during playback.
- [x] Original / Translation / Both / Off switch instantly. Changing the target to English shows "translating…" coverage, then English text.
- [x] Seeking to 15:00 in `fixture_long_en.mp4` shows subtitles there within 5 s, and the coverage bar shows processing has moved there.
- [x] `fixture_fr.mkv` shows "Preparing video…" progress, then plays with subtitles already available.
- [x] Fullscreen keeps the subtitles and controls; `Esc` exits; subtitle size scales.
- [x] Clicking a transcript row seeks; `,` and `.` jump between lines; auto-scroll follows.
- [x] Export writes SRT, VTT and ASS next to the video, and they load in QuickTime/IINA/VLC (any player available).
- [x] Overriding the spoken language to the wrong one and back re-transcribes (stale text is replaced).
- [x] Quitting and reopening the same file restores the subtitles instantly.
- [x] Setup screen: rename `~/.OminiX/models/silero-vad`, restart, and the app asks for that one download only.
- [x] Killing `hlp-asr-worker` mid-session: the app restarts it and processing continues.
- [x] Main-state screenshots recorded.

### M7 · URL downloads

- [x] Wikimedia or Internet Archive download plays with subtitles.
- [x] User-supplied YouTube URL downloads, plays and receives subtitles.
- [x] Cancel removes partial downloads. The real UI cancelled a throttled transfer after 3,072 bytes and the destination contained no partial files.

In native WKWebView, the Internet Archive direct video produced 32 bilingual cues. The user-supplied YouTube video `EpJj0qAE0Hg` delivered 322 progress events and played at 1920 pixels wide with detected English and a visible Chinese translation. The Commons test host returned HTTP 429, so the accepted direct-file check used Internet Archive.

### M8 · Windows

All seven packages, including the real desktop and worker, passed `cargo check --target x86_64-pc-windows-msvc` using the production Sherpa archive and portable LLVM tools. The same Sherpa implementation transcribed 2.547 s of English on the Mac CPU in 0.339 s. The PowerShell media generator produced all nine ffprobe-verified files with recorded English standing in for SAPI. A final worker link on macOS identified missing Microsoft SDK and static runtime libraries. The user confirmed no Windows PC is available. The native checks below remain open. [Windows evidence and handoff](progress/g-windows.md).

- [ ] SAPI fixture E2E passes on Windows.
- [ ] Desktop smoke test passes.
- [ ] CPU throughput measured.

### M9 · Packaging

- [x] Internal macOS app and DMG include the worker and Metal library. `bash scripts/build_macos_app.sh` built both arm64 executables with a 14.0 minimum deployment target. `codesign --verify --deep --strict` and `hdiutil verify` passed. Both executables link only to system libraries. The 62,844,795-byte DMG has SHA-256 `a9c379d0d1cfdf5b224040a7dcbdfe8c83794216f33f3b29db949882639cfec5`.
- [x] The installed application completed its 3.67 GB setup in a fresh macOS app profile and played media with generated bilingual subtitles.
- [ ] Repeat under a separate macOS login. The fresh-profile test used isolated preferences, data and model directories under the existing login.
- [ ] NSIS installer passes on Windows. The x64 packaging script passed PowerShell parsing. A manual Windows-hosted workflow builds and uploads the installer and can run the SAPI fixture matrix. No native Windows run has been observed.

An independent release CLI run used the exact sidecar in the installed app while macOS denied off-machine connections and access to the developer build directory. On the 20-minute fixture, all nine checks passed with 359 cues in 68.4 s, a 94 ms maximum onset error and a 47 ms cached reopen. ASR reached 9.56× real time, below the 15× performance goal. Mean translation latency was 113 ms per utterance. [Release offline measurements](../tests/results/2026-11-23-darwin-release-long_en.json).

The lead mounted the DMG read-only, copied the app to a separate Applications folder, detached it, and launched the installed executable with empty preferences, cache and component directories. The first-launch dialog offered 3.67 GB. During setup the app quit with 267,043,606 bytes of the translation model downloaded. The installed update resumed that file and verified all five required components. It started its bundled ASR worker and the newly downloaded llama runtime, then rendered six Japanese lines with six English translations. [First-launch setup](screenshots/installed/setup.png) · [Installed bilingual player](screenshots/installed/playback.png).

The installed app also opened a new Japanese/English dual-track MKV with off-machine network, developer build and pre-existing model access blocked by macOS. Each track produced six correctly identified utterances in its own cache. Switching back displayed all six Japanese cues in 492 ms without rewriting the prepared video. The track-cache regression test checks capability changes and eviction.

The final release CLI repeated the full fixture matrix under the same network and filesystem restrictions, using the worker copied from the DMG and the models downloaded during setup. All 81 checks across nine fixtures passed. The 20-minute file produced 359 bilingual cues in 62.38 seconds and reopened in 46 ms. The highest character error across fixtures was 2.86%, and the largest onset error was 216 ms. [Full offline release results](../tests/results/2026-11-23-darwin-release.json).

The production frontend rendered three distinct subtitle overlays and streamed six transcript rows during a ten-second playback sample in headless Chrome. The animation clock averaged 60.0 fps across 601 intervals, with a 16.8 ms maximum interval. Both the native app and a separate WKWebView timing host reported their documents hidden and suspended animation callbacks during the frame-rate test. The native player passed the playback and overlay checks in M6, but a native frame-rate measurement remains open. [Browser frame timings](../tests/results/2026-11-23-darwin-chrome-frames.json).

An offline release run on a two-hour file made by looping the English fixture produced 2,154 English/Chinese cues in 8 minutes 56 seconds, inside the 15-minute target. Reopening took 216 ms and made zero inference requests. ASR averaged 7.32× with translation running. In a separate warm worker run, 54.05 seconds of speech took 4.27 seconds with translation idle (12.65×), below the 15× ASR target. [Two-hour results](../tests/results/2026-11-23-darwin-release-two-hours.json) · [Warm ASR timings](../tests/results/2026-11-23-darwin-release-warm-asr.json).

The Mac player now defers background translation while playback has available, unrecognized speech in the lookahead and measured ASR throughput is below 8×. The current subtitle and the next two seconds retain translation priority. The scheduler regression covers cold startup, seeking, recovery, extraction and lookahead boundaries. After this change the lead mounted the rebuilt DMG, verified that both installed executables and the Metal library matched the distribution byte for byte, and opened the Japanese fixture in the installed app with an empty project cache and external network access blocked. It generated all six original lines and English translations. A fresh native WebView session on the final source advanced 3.92 seconds in the 1280-pixel video and rendered six Japanese/English transcript rows.

## Lane reports

- [a-core](progress/a-core.md)
- [b-media](progress/b-media.md)
- [c-models](progress/c-models.md)
- [d-asr](progress/d-asr.md)
- [e-ui](progress/e-ui.md)
- [f-app](progress/f-app.md)
- [g-windows](progress/g-windows.md)
