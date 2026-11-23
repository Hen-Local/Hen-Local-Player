# Prototype progress

A checked item has been exercised. Worktree branches and tags are local until publication is requested.

The lead merged phase 2 in order (A, C, B, D, E), verified the CLI slice on macOS, then merged F followed by G. The final tree passed Rust formatting, 38 regular Rust tests, the Svelte check with zero warnings, the production UI build and the seven-package Windows x64 cross-check.

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
