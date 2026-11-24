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
