# B · Media and URL transport

- [x] Probe and playback decisions match the fixtures. `hlp-cli probe tests/fixtures/<file> --webview wkwebview` returned Direct for ja, de, zh, en audio-only, music and long_en. It returned Remux for fr and Korean 5.1, and Transcode for es. The fixture test prepared real fr and es MP4s and verified their H.264 and AAC/MP3 streams with ffprobe.
- [x] Ten-minute extraction meets the sample-count tolerance. The ignored fixture test decoded a 600-second AAC cut from long_en to 9,600,000 samples (19,200,000 bytes). It checked the on-disk position of every published frame. Initial plain decoding produced 9,647,357 samples. Timestamp-aware FFmpeg resampling with `async=1:first_pts=0` keeps PCM aligned with the media timeline across packet discontinuities.
- [x] `cargo test --locked -p hlp-media` covers seven stored ffprobe reports, HEVC tags, interlacing, multiple tracks, capability-dependent decisions, absent audio, FFmpeg and yt-dlp progress, Range 206, conditional range 200/206, HEAD, token revocation, process-tree cancellation and isolated partial-file cleanup.
- [x] `cargo test --locked -p hlp-media -- --ignored --nocapture` exercises generated fixtures and a separate loopback download with yt-dlp 2026.10.26 and Deno 2.9.8. Probe produced a title, real download progress arrived, the final bytes matched the original file and all temporary files were removed.
- [x] The six-package Cargo check in §9.1.1 and `npm --prefix app/ui ci && npm --prefix app/ui run check` succeeded on Apple Silicon. The UI checker reported zero errors and zero warnings.

Foundation used Rust 1.92 and a separate no-space `CARGO_TARGET_DIR`. Media tests set `HLP_FFMPEG`, `HLP_FFPROBE`, `HLP_YTDLP` and `HLP_DENO` to the test tools. The worker and models were not used.

## Integration observations

- FFprobe reports the same `matroska,webm` format name for both containers. Probe reads the EBML DocType to distinguish them independently of the filename.
- FFprobe accepts `-hide_banner` and receives null stdin. Its CLI lacks FFmpeg's `-nostdin` option.
- The pinned tower-http ServeFile honors Range and Last-Modified. The wrapper checks If-Range explicitly and suppresses Range when the validator is stale.
- The PCM callback's final frame can be shorter than 512 samples. The exact length belongs to the media timeline. The return value is the total sample count.
- The session owns project metadata. Preparation accepts an output path for the selected track and checks the project record before reusing an existing file. The caller handles runtime escalation and media URL registration.
- The downloader uses a temporary directory per attempt and only removes that attempt's files. External site acceptance, the app's URL dialog and Windows execution belong to phases 4 and 5.
