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

## Offline playback cadence on the HomeView 2160p clip

- Matched the open player's project lock to the prepared media path. The player served the original 3840×2160 H.264 Main / AAC MP4 directly. The 192.53-second video contains 4,616 progressive frames at 24000/1001 fps.
- `ffprobe -v error -show_packets -show_streams -show_entries 'stream=index,codec_type,time_base:packet=stream_index,pts,dts,duration,flags' -of json "$MOVIE"` found 1,001 ticks at a 24,000 Hz clock for every video packet and every adjacent video PTS and DTS difference. All adjacent audio packets also meet exactly. Both tracks have a single rate-1 edit starting at media time zero. The dominant keyframe interval is 250 frames (10.427 s).
- `ffmpeg -v error -i "$MOVIE" -map 0:v:0 -vf 'scale=160:90:flags=area,format=gray' -fps_mode passthrough -f rawvideo -` decoded every picture. Comparing adjacent reduced-luma frames found 230 exact repeats, all after 181.5 s during the ending. A threshold of mean absolute luma difference below 0.2 and at most 2% of pixels differing by more than 2 included a brief near-still sequence at 47.5–48.7 s. No regularly recurring duplicate appeared across the moving portion or near its keyframes. Downsampling can hide detail and this comparison does not measure physical presentation.
- An eight-second Core Video display-link sample measured 120.00005 Hz on the Dell AW3418DW and 60.00003 Hz on the Lenovo P32p-20. At exactly 120 Hz and normal speed this clip takes 5.005 refresh intervals per frame. Ideal scheduling therefore holds one frame for an extra 8.33 ms about every 8.34 s. That arithmetic describes display cadence independently of decoder deadline misses.
- Compared standalone AppKit / AVPlayerLayer playback against Hen on the same Dell. Played the original local file muted in a visible 894×1050-point window at 2× backing scale, sought to 12 s and observed the next 53 s. The native test used an AVPlayerItemVideoOutput with ordinary player rendering enabled. A Core Video callback requested the available source picture for each upcoming display refresh. The harness sampled layer playback metrics separately at 0.5 s intervals. These diagnostic probes were confined to temporary test executables.

| Standalone playback | Source picture transitions (13.1–65.1 s) | Layer drops / total frames (about 52 s) | Damaged frames | Waiting polls |
| --- | --- | --- | --- | --- |
| Original 2160p, 1× | 1,239 intervals of five refreshes and six intervals of six refreshes | 19 / 1,238 | 0 | 0 / 533 |
| Original 2160p, 1.001× | 1,245 intervals of five refreshes | 17 / 1,242 | 0 | 0 / 533 |
| 1080p control, 1× | 1,239 intervals of five refreshes and six intervals of six refreshes | 17 / 1,246 | 0 | 0 / 533 |

Both rates delivered consecutive source picture timestamps and uninterrupted display-link callbacks throughout the comparison. At 1× the six longer source dwells recurred every 8.30–8.34 s. The layer's drop counter rose in clusters about ten seconds apart, with additional increments between clusters. Changing the rate aligned source pictures to exactly five refreshes but those layer drops persisted. The layer also counted drops during straightforward local-file playback without a webview or subtitle renderer. The layer statistics cannot identify which physical scanouts the viewer perceived.

For the resolution control, encoded a temporary 70-second excerpt with `ffmpeg -i "$MOVIE" -t 70 -map 0:v:0 -map 0:a:0 -vf scale=1920:1080:flags=area -fps_mode passthrough -c:v h264_videotoolbox -profile:v main -level:v 5.1 -bf 0 -b:v 5000k -g 250 -force_key_frames 'expr:gte(n,n_forced*250)' -video_track_timescale 24000 -c:a copy -movflags +faststart "$TEMP_MP4"`. FFprobe verified all 1,679 video PTS, all seven keyframe positions, and all 3,015 AAC packet PTS and SHA-256 hashes against the source excerpt. Reducing resolution to 1080p and video bitrate to 4.96 Mb/s preserved the recurring layer drop increments in this test.
