# Authored subtitle tracks

The subtitle drawer offers local SRT and WebVTT files, matching files beside the movie, embedded text tracks, generated captions and Off. FFprobe reads subtitle language, title, default and forced flags from the original movie. FFmpeg converts the selected text stream to WebVTT, including when the video is being remuxed. The player draws timed text as plain text and supports simultaneous cues. Picture in Picture uses the same adjusted cue times.

Explicit track choices and timing offsets are kept per movie. Imported subtitle text is saved locally in IndexedDB. On first open, a forced text track takes priority over a default track. When neither exists and required subtitle models are missing, the player tries a same-basename SRT or VTT. Opening the subtitle drawer later never overrides a viewer's choice. The authored transcript shows the selected track's text and adjusted timestamps. Subtitle export continues to use the generated transcript.

## Checks on macOS

- Four Rust tests passed with the installed FFmpeg and FFprobe. They cover UTF-16 SRT, sibling filename matching, access limited to the open movie's neighboring files, and real English/Japanese Matroska stream extraction. The stream test generates its own media and checks an invalid stream selection as well. Run `cargo test --locked -p henlocal-player subtitle_tracks::tests -- --include-ignored` after setting up the playback tools.
- A real native WKWebView played a 1280-pixel-wide Matroska movie with the AI model directory empty. At 1.5 seconds its two embedded streams and two adjacent files rendered “Station”, “駅です”, “Station”, and “駅です”, respectively. Choosing Off cleared the overlay. Adding 1.1 seconds of delay hid the English cue until seeking to 2.2 seconds.
- Chrome's file picker imported a multiline SRT and a UTF-16 Japanese WebVTT. The browser test verified entity decoding, overlapping cues, exclusive end times, a 1-second timing shift, generated/Off switching, both active transcript highlights, and an authored-row click that sought to 2.5 seconds.
- Reloading the browser restored explicit Off. The subtitle shortcut restored the last imported Japanese track and its timing offset. Another reload restored the same captions before the subtitle drawer opened.
- A second native run used fresh media and an empty model directory. Forced Japanese was selected automatically over the English default. Opening a separate MP4 immediately displayed its adjacent bilingual SRT, with six authored transcript rows. Clicking the third row sought to 7.767 seconds. The two movies subsequently reopened with their separate explicit Off and Generated choices.

## Subtitle line length on screen

On November 23, 2026, headless Chrome played the bundled sample with a Chinese cue and an English translation. In a 720 × 430 window the previous layout displayed the two languages on single lines 468 and 487 pixels wide. With each language limited to 22 times its own font size, the same text wrapped into two lines per language, with longest lines of 234 and 262 pixels. A 450 × 300 window and a 1920 × 1080 fullscreen session also displayed two lines per language. DOM range measurements placed every glyph inside the picture.

An imported SRT retained its explicit line break. A separate 132-letter string without spaces wrapped into four lines in all three viewports and retained every character. The production Vite build and `npm --prefix app/ui run check` passed with zero Svelte errors or warnings.

## Sidebars over the picture

The playlist and settings now slide from the edges over the playing movie. With the sample video and its bilingual SRT open in Chrome, the movie element, subtitle area and all ten transport targets kept identical window coordinates with either sidebar open, both open and both closed. All ten controls remained clickable at 720 × 430, 1280 × 760, 450 × 300, touch sizes 390 × 680 and 320 × 235, and the player's fullscreen layout at 1920 × 1080.

Frame sampling at 720 × 430 recorded the playlist sliding 180 pixels and settings sliding 250 pixels. Both retained their contents during opening and closing. Enabling reduced motion produced no intermediate movement. Closing and reopening either sidebar 42 ms apart left exactly one interactive sidebar with keyboard focus. Escape restored the toggle, and a 1.3–2.1 second A–B playback loop continued with settings closed.

## Subtitle settings and generation progress

On November 23, 2026, the browser player was exercised in English in Chrome at 960 × 640 and WebKit at 720 × 430, and in Chinese in Chrome at 450 × 300. The local preview used the bundled Japanese sample and the preview speech backend. The subtitle panel displayed generation progress from 17% to 100% without expanding a section. Reopening the completed sample restored all six cues at 100% without starting transcription. The subtitle list stayed folded for the generated track, the neighboring SRT and a newly imported SRT. Expanding the neighboring track and selecting its third cue sought to 7.767 seconds.

Selecting Japanese, English and Both in Subtitle Settings produced one original row, one translation row and two rows on the picture. Changing the translation target updated the choices. Matching the spoken language or disabling translation left one original-language choice. Per-line detection showed a generic original-language choice and a forced source showed its actual name. In WebKit, increasing Text Size from 100% to 150% changed the main text from 17.01 to 25.515 pixels. A 38% vertical position placed the captions 153.9 pixels above the bottom of the picture. The settings and file picker remained reachable in the smallest window without horizontal scrolling.

The preview was also driven with a partially cached status and a recoverable ASR error. The first showed 17% and offered resume. The second exposed the error and retry action, which returned the sample to 100%. The Svelte checker reported zero errors and warnings with `npm --prefix app/ui run check`. The production bundle built with `npm --prefix app/ui run build`.

## Long transcripts and bottom-aligned transcription controls

On November 23, 2026, Chrome at 960 × 640 and 450 × 300 and WebKit at 720 × 430 exercised the English **Transcribe** and Chinese **转录** actions. The action and the live progress bar occupied the footer of the Subtitles panel. Completion removed both and kept **Subtitle Settings** at the same vertical coordinate. Reopening a finished sample restored its six captions without a footer. Incomplete cache, model error and retry paths remained reachable. An imported SRT opened with its list collapsed.

At 760 × 460, the previous browser UI mounted 480 buttons for 480 timed captions. The new list mounted 10 for both 480 and 1,807 captions. In a 65-minute simulated timeline, the test changed the playhead and scrolled the list on 100 consecutive animation frames, including 34 incoming subtitle updates. Both Chrome and WebKit reached cue 1,807, sought from it, and closed and reopened the list with visible content. The 95th-percentile frame interval was 17.2 ms in Chrome and 23 ms in WebKit. Moving, deleting and inserting cues out of chronological order preserved sort order and cue identity. The settings header did not move when progress finished.

A release-mode timing test used 600 Chinese/English utterances spanning 65 minutes and producing 2,400 timed captions. Over 30 single-translation edits, median time to rebuild the timed captions fell from 19.56 ms to 0.89 ms. Unchanged utterances reuse their phrase segmentation. Neighbor timing is recomputed across the track, and regression tests compare the incremental output with a full layout after overlaps, removals, changed languages and shorter or longer translations.

`cargo test --locked -p hlp-core -p hlp-pipeline` passed 31 regular tests. Two additional native fixture tests passed with the installed speech and translation models. In `explicit_generation_starts_once_and_restores_the_result`, reopening a completed Japanese clip used zero engine requests. In `completed_track_schedules_a_new_translation_and_respects_lookahead`, changing a completed track to English at the end of the file paused while whole-file processing was off. Enabling whole-file processing completed all six English translations with the original six speech requests. `npm --prefix app/ui run check` reported zero errors and warnings.
