# Prototype progress

A checked item has been exercised. Worktree branches and tags are local until publication is requested.

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

The six-package Rust check, Svelte check, core, media, installer and pipeline tests passed. Dedicated regressions cover stale source overrides, a split language vote and safe cache eviction. The CLI returned exit 2 for missing setup, 3 for a missing file, and 4 for an unavailable worker. All 45 languages appeared in `hlp-cli langs`.

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

Mock UI reviewed and merged after D. The six-package check, Svelte check and production build passed on the merged tree. Eight screenshots and three browser smoke runs cover the mock flows. Native acceptance remains below. [Lane E evidence](progress/e-ui.md).

- [ ] Empty state shows the drop zone and the recent list. Dropping a file plays it within 1 s (Direct).
- [ ] The language badge shows "Japanese 9x %" within 15 s, and subtitles appear and stay in sync during playback.
- [ ] Original / Translation / Both / Off switch instantly. Changing the target to English shows "translating…" coverage, then English text.
- [ ] Seeking to 15:00 in `fixture_long_en.mp4` shows subtitles there within 5 s, and the coverage bar shows processing has moved there.
- [ ] `fixture_fr.mkv` shows "Preparing video…" progress, then plays with subtitles already available.
- [ ] Fullscreen keeps the subtitles and controls; `Esc` exits; subtitle size scales.
- [ ] Clicking a transcript row seeks; `,` and `.` jump between lines; auto-scroll follows.
- [ ] Export writes SRT, VTT and ASS next to the video, and they load in QuickTime/IINA/VLC (any player available).
- [ ] Overriding the spoken language to the wrong one and back re-transcribes (stale text is replaced).
- [ ] Quitting and reopening the same file restores the subtitles instantly.
- [ ] Setup screen: rename `~/.OminiX/models/silero-vad`, restart, and the app asks for that one download only.
- [ ] Killing `hlp-asr-worker` mid-session: the app restarts it and processing continues.
- [ ] Main-state screenshots recorded.

### M7 · URL downloads

- [ ] Wikimedia or Internet Archive download plays with subtitles.
- [ ] User-supplied YouTube URL downloads, plays and receives subtitles.
- [ ] Cancel removes partial downloads.

### M8 · Windows

- [ ] SAPI fixture E2E passes on Windows.
- [ ] Desktop smoke test passes.
- [ ] CPU throughput measured.

### M9 · Packaging

- [ ] Unsigned macOS app and DMG include the worker and Metal library.
- [ ] Installed app completes setup and playback on a clean account.
- [ ] NSIS installer passes on Windows.

## Lane reports

- [a-core](progress/a-core.md)
- [b-media](progress/b-media.md)
- [c-models](progress/c-models.md)
- [d-asr](progress/d-asr.md)
- [e-ui](progress/e-ui.md)
