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

- [ ] Probe and playback decisions match the fixtures.
- [ ] Ten-minute extraction meets the sample-count tolerance.
- [ ] Byte ranges, download progress and cancellation tests pass.

### M3 · Models and setup

- [ ] Existing ASR, GGUF and llama runtime are ready.
- [ ] Verify managed Silero and FFmpeg installs under a temporary root.
- [ ] Interrupted downloads resume and manifest pins pass validation.

### M4 · ASR worker

- [ ] Japanese auto and forced inference pass on real speech.
- [ ] Tokenizer and protocol checks pass.
- [ ] A killed worker is restarted.

### M5 · Pipeline and CLI

- [ ] Detected languages match truth.
- [ ] Normalized character error rate ≤ 15%.
- [ ] Every truth onset has a cue within 600 ms.
- [ ] Cue intervals are positive and non-overlapping.
- [ ] Translations present and same-language translation skipped.
- [ ] SRT, VTT and ASS parse.
- [ ] Music yields zero cues.
- [ ] Cached runs finish within two seconds.
- [ ] Direct, Remux and Transcode plans match fixture expectations.

### M6 · Desktop player

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
