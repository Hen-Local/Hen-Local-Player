# Lane F · desktop and URL playback

This lane starts at `slice-v1`. The tests run the real Tauri WKWebView on the M4 Pro with an isolated model/tool root and cache. A temporary Vite harness observes the same bridge events as the Svelte player and operates its controls. The shipped app has no test commands. An external AppKit drag source provides an actual NSURL pasteboard drop.

## M6

- [x] Empty drop target and Direct playback. During a cold Japanese session the operating system delivered the dropped file, the player published `media://opened` at 199 ms and WKWebView fired `playing` at 287 ms.
- [x] Japanese language badge, real bilingual overlay and timing. The badge reached 100% in 1.194 s. The Japanese and Chinese overlay appeared at 1.668 s. The CLI fixture test measured a maximum 37 ms onset error on the same file.
- [x] Original, Translation, Both and Off. All four switched the displayed Japanese and Chinese text as expected. Switching the target to English produced pending coverage and the translated sentence “It’s very nice weather today.”
- [x] 20-minute priority seek. At 15:00 the uncached cue appeared in 791 ms and its visible overlay was on screen at 1.32 s.
- [x] Format handling. Opening French MKV delivered three preparation events, then played through the remuxed HTTP URL. Spanish AVI transcoded and German WebM played directly. Each completed with six translated cues. A separate cold French launch began playback at 215 ms and received its first cue at 1.747 s. After evicting only the temporary playable derivative, opening the same file restored all six cues before preparation finished. WKWebView started at 188 ms with the six cues already available.
- [x] Window fullscreen, overlay, controls, Escape and scaling. The fullscreen screenshot includes the native video, bilingual overlay and controls after the scale was changed to 1.1.
- [x] Transcript click and keyboard navigation. The third row sought to 7.650 s. The period key advanced to 15.462 s and comma moved back to 11.846 s. On the long file the panel scrolled 20,338 pixels and the active row was inside its viewport.
- [x] Native Export All wrote nine subtitle files beside the temporary video. VLC 3.0.21 opened bilingual SRT, VTT and ASS using its subsdec, webvtt and libass subtitle decoders. The test invoked VLC with `--ignore-config --sub-file=<export> --no-sub-autodetect-file`, the Japanese video and a short playback limit.
- [x] Wrong-language override followed by Auto. Seven stale cue updates were observed. Fresh English-forced rows appeared before automatic Japanese replaced them.
- [x] After a separate app launch the recent list contained six items. Opening Japanese restored all six cached cues in 146 ms.
- [x] An isolated installation with only Silero missing opened the setup screen and listed that one dependency. The Japanese MP4 played while the stage reported Needs setup. The native Download all action fetched Silero and the open session resumed to six Japanese and Chinese cues.
- [x] Worker crash recovery. The active worker was sent SIGKILL while processing the long file. The app stayed open and the completed utterance count advanced from 11 to 14.

## M7

- [x] Internet Archive confirmed download, progress, playback and subtitles. The native URL dialog previewed `ed_1024_512kb`, then emitted 36 download updates. WKWebView played the downloaded 426-pixel-wide video directly. The completed English transcript produced 32 display cues from 28 translated utterances, including “We can see that at the right.” with “我们在右边可以看到。” [Direct video](https://archive.org/download/ElephantsDream/ed_1024_512kb.mp4)
- [x] The supplied 15:30 YouTube video “No, I wouldn’t say I have a passion for HR” previewed, downloaded with 322 progress events, played directly at 1920 pixels wide and displayed its English opening line with a Chinese translation. The check ended after the opening bilingual cues appeared. At the start of this test both URL tools were Missing. The preview installed yt-dlp and Deno under the isolated root. [User-supplied video](https://www.youtube.com/watch?v=EpJj0qAE0Hg)
- [x] The native Cancel button stopped a throttled loopback MP4 download after 3,072 received bytes. The temporary destination was empty after cancellation.
- [x] The native Settings update control checked yt-dlp and reported that it was up to date.

The initial Wikimedia Apollo clip returned HTTP 429 from the upstream service during the standard yt-dlp preview. The Internet Archive source satisfied the alternate direct-video criterion.


## Verification

`cargo check --locked -p hlp-core -p hlp-media -p hlp-models -p hlp-pipeline -p hlp-cli -p henlocal-player`, `cargo clippy -p henlocal-player --no-deps`, `npm --prefix app/ui run check` and `npm --prefix app/ui run build` passed. Svelte reported zero errors and warnings. The tests used `CARGO_TARGET_DIR="$TMPDIR/hlp-target-f"`, `HLP_OMINIX_ROOT` and `HLP_DATA_DIR` overrides. Launch the player with `bash scripts/dev.sh` to repeat the interactive checks in PLAN §8.4.

The preparation screenshot uses a 180-second repeat of the French fixture to keep the progress state visible during capture. The cached-remux screenshot shows restored transcript rows before the first video frame. The URL preview screenshot shows the native dialog. The playback screenshot of third-party footage is retained privately. [Native screenshots](../screenshots/native/)

## Regenerating saved subtitles

On November 23, 2026, `cargo test --offline --locked -p hlp-pipeline -- --include-ignored --test-threads=1 --nocapture` passed all nine tests with the installed Mac speech and translation models. The Japanese clip completed six speech requests and six English translations. Two consecutive regeneration requests took those totals to twelve each. The decoded PCM kept its original modification time and the media session emitted a single open event. Reopening the clip restored all six translated utterances with zero model calls.

Requesting regeneration with missing components preserved all six translations. A request naming a different audio track returned an error. A separate cache test reopened the track immediately after the regeneration checkpoint and recovered the existing speech regions as pending work. It checked both English and French caches so switching the target language also uses the new transcript. The desktop command passed `cargo check --offline --locked -p henlocal-player`.

## Model memory and completed playback

The native player stores recognition results in the media cache's `track-N/transcript.json` and translations in `track-N/translations/`. Reopening a completed track constructs timed cues from these records and sends them in the initial media snapshot. Export writes an SRT, VTT or ASS to the viewer's chosen destination.

On November 23, 2026, the previously installed player still owned an idle `llama-server` with 2,994,576 KiB RSS eleven minutes after generation. The new pipeline keeps the model workers warm for thirty seconds after the last processing session releases them. A new generation within that interval reuses the engines. After the interval both workers are shut down and reaped. A clean, completed session also suspends its 100 ms scheduling timer. Playback and export continue using the in-memory timeline.

- `python3 /tmp/hen-sample-backend-playback.py` exercised the desktop's `Session::open_for_playback` on a private clone of the 19-minute H.264 film's cache. It restored 474 cues and 180 completed translations using direct playback. Over 21 advancing playhead reports there were zero ASR or MT requests, zero cue patches, one initial completion status and zero model children.
- A second run removed just the final translation in that clone. The player issued one MT request. Its server peaked at 1,508,192 KiB RSS and exited approximately thirty seconds after completion while the session stayed open and playhead updates continued. An independent FFprobe pass found evenly spaced H.264 packet presentation timestamps across this 29.97 fps film and the 24 fps Mandarin film.
- With the installed models and the real ASR worker, `cargo test --locked --offline -p hlp-pipeline completed_track_schedules_a_new_translation_and_respects_lookahead -- --ignored` passed in 12.18 seconds. The test exercised a second user of the same loaded translation process across an idle deadline, expiry after the last user released it, regeneration from unloaded engines, and reopening with zero inference. Ordinary pipeline tests, `cargo clippy --locked --offline -p hlp-pipeline --all-targets --no-deps -- -D warnings` and formatting passed.

## Local delivery and decode capability for the 4K HomeView movie

On November 23, 2026, the active desktop process held the cache lock for the 192.586-second, 260,280,132-byte HomeView video. Its saved playback plan was `direct` with no prepared file. Playback served the original progressive 3840 × 2160 AVC Main 8-bit video and stereo AAC. The combined file bitrate averaged 10.81 Mbit/s.

- Linked an isolated loopback test host against the same `hlp_media::MediaServer` as the player. `python3 /tmp/hen-homeview-range-test.py` verified the MIME type, advertised length, byte-range support, the initial two-byte request, and seeks spread through the movie. Each of 22 ranges returned HTTP 206 with the correct content range and matched the source bytes. A past-end request returned HTTP 416. For local cached reads, the median 1 MiB request took 0.60 ms and the 95th percentile took 0.91 ms. These times measure file delivery separately from video presentation.
- Loaded the original video track's format description with AVFoundation. The default VideoToolbox decompression session and one requiring hardware acceleration both reported `UsingHardwareAcceleratedVideoDecoder = 1`. This checks eligibility on the test Mac. WebKit does not expose that property for its active media element.

## Private software update delivery and restart

On November 24, 2026, `cargo test --locked --offline -p henlocal-player` passed all 11 active tests (two opt-in subtitle-provider tests ignored), including stable-version filtering and platform eligibility. The Windows cross-toolchain from `docs/progress/g-windows.md` passed `cargo check --locked --offline --target x86_64-pc-windows-msvc -p henlocal-player`. A Windows installer launch was not exercised on this Mac.

`python3 /tmp/hen-build-update-fixture.py` built two real Apple Silicon player executables with package versions 0.1.0 and 0.1.1, an isolated updater key, a test HTTPS certificate authority and a process-local test credential store. `python3 /tmp/hen-test-version-install.py` opened a disposable copy of 0.1.0 and exercised the native WebView commands against a loopback HTTPS store. The unconnected check sent zero requests, a denied purchase returned the access error, and a redirect from the credential-bearing feed was not followed. The eligible stable version and the older history entry appeared with the correct install eligibility. The release was resolved again before download. The artifact redirect carried no Referer or Authorization header. The signed bytes matched the declared SHA-256 and size, and the staged metadata contained neither the purchaser link nor the temporary file URL.

The isolated app refused installation after one byte of the staged package was changed, discarded the corrupted package and accepted a clean download of the same release. With the test server then stopped, Install and Restart replaced the .app and Launch Services opened the new executable under a new process ID. A subsequent diagnostic launch of that installed executable returned native version 0.1.1 and cleared the consumed staging record. The executable matched the expected SHA-256, strict Apple code-signature validation passed, and the workspace's dist executable retained its original hash. This exercise used test credentials and unsigned-by-Apple, ad hoc app identities. It did not contact or publish to a merchant account or GitHub.
