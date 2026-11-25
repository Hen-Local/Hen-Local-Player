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

## Regenerating completed subtitles

On November 23, 2026, the local preview ran the **Regenerate** and **重新生成** actions with the Japanese sample in Chrome at 960 × 640, 450 × 300 and 320 × 235, and in WebKit at 720 × 430. The button remained clickable with playback controls hidden and at the foot of either settings tab. A click opened the Subtitles tab, displayed the new run’s progress and restored the button after all six captions returned. The video URL, session and paused playback time stayed the same throughout the run.

In each viewport, reopening the completed sample restored the button without sending a transcription request. Selecting the accompanying SRT and starting regeneration switched to the generated track. Reading the SRT again returned the original text byte for byte. A simulated missing speech model kept the six cached captions available, then installation retried with the same audio track and regeneration flag. Two rapid regeneration requests during a delayed command sent one backend call. A separate page reload restored the sample cache, and a completed zero-segment audio run kept its action available.

Acceptance used `node /tmp/hen-regeneration-ui.cjs`, `node /tmp/hen-regeneration-tiny.cjs` and `node /tmp/hen-regenerate-cache.cjs` against the local Vite preview. `npm --prefix app/ui run check` reported zero errors and warnings.

## Importing and finding subtitle tracks

On November 23, 2026, `node /tmp/hen-opensubtitles-ui-check.cjs` exercised the subtitle sidebar in Chrome at 450 × 300 in English and WebKit at 720 × 430 in Chinese. With Subtitle Settings collapsed, **Import Subtitles…** opened the file chooser, selected a local SRT and displayed its cue at the two-second playhead. The OpenSubtitles button opened the title, language and key form. Online responses in this browser test came from a local fixture. Tests covered an invalid key, the API registration action, no results, a quota error, download retry and switching between two available files.

Selecting **Use Subtitle** displayed the chosen cue. Selecting the same result a second time reused the saved track. After reloading the player, both the imported SRT and downloaded subtitles were available with zero provider calls. The API key was absent from local storage and the key input was empty after reload. Deleting the locally cached subtitle text while retaining its track preference allowed the next explicit download to repair that track without creating a duplicate. Closing the search dialog or switching media during a delayed download left the existing subtitle selection intact. The browser-only preview directs online searches to the desktop app. [Desktop service verification](opensubtitles.md).

## Finding online subtitles without an API key

On November 23, 2026, `node /tmp/hen-guest-ui.cjs` exercised the **Find Online Subtitles…** button, the native menu event and the browser preview shortcut in Chrome (450 × 300, English) and WebKit (720 × 430, Chinese). The dialog focused the editable movie title. The search button was enabled immediately and no provider calls occurred on opening. A Traditional Chinese translation preference selected the provider’s Chinese search filter. Both browser engines exercised twelve example filenames. The initial search for `The.Matrix.1999.1080p.BluRay.x264.mkv` read **The Matrix**. Numeric titles, decimals, dotted initials, episode numbers, Chinese and Japanese text survived cleanup. Editing the title and pressing Enter submitted only the title and selected language.

A local provider fixture exercised no matches, a network error, a download quota error and successful retry. Choosing **Use Subtitle** displayed the downloaded cue and used the readable release filename in the track picker. Selecting the same result again reused the local file. Closing the dialog during a different download preserved the original track. After reload, guest and previously formatted OpenSubtitles tracks both played from the local cache with zero provider calls. Both browser runs reported zero page errors. `npm --prefix app/ui run check` reported zero errors and warnings.

## Search shortcut and personal recordings

On November 23, 2026, `node /tmp/hen-subtitle-shortcut-fallback.cjs` exercised the native menu event and the shortcut forwarded to the desktop web view in Chrome at 450 × 300 in English and WebKit at 720 × 430 in Chinese. The launcher displayed an open-media hint. With media open, both routes opened the same search dialog. Repeating the shortcut and menu action preserved the edited title. Opening the dialog sent zero provider requests.

For a recording with no catalogue match, the empty results offered another title or language and a local subtitle import. Selecting an SRT with the file chooser displayed the imported cue during playback. On a recording awaiting transcription, the local transcription action opened the subtitle panel and dispatched the existing transcription command once. With generated cues already present, the empty results offered local import. These browser tests used fixture provider responses and a transcription dispatch spy. `npm --prefix app/ui run check` reported zero errors and warnings.

## English and Chinese interface labels

On November 23, 2026, `node /tmp/hen-ui-labels.cjs` played the sample and checked the six visible More Options entries in order in Chrome (English) and WebKit (Chinese). The English menu read **Open File**, **Open Folder**, **Open URL**, **Recently Opened**, **Settings** and **Close File**. Both runs opened Settings, found the interface language field, cache limit and log folder action by their accessible names, then opened subtitle search and located its movie title field. Both catalogs contained the same 244 keys. `npm --prefix app/ui run check` reported zero errors and warnings.

## Subtitle search field alignment

On November 23, 2026, `node /tmp/hen-subtitle-field-layout.cjs` opened the search dialog in Chrome and WebKit at 450 × 300 and 720 × 430 in both English and Chinese. Across all eight screenshots the movie title field and language picker measured 36 CSS pixels tall. Their painted top and bottom borders aligned to the same pixels and the chevron sat at the vertical midpoint. Tab reached the language picker, English keyboard selection chose Japanese, and Enter submitted the chosen language and edited title in both locales. Each run made one search request to the local fixture.

## Subtitle inspector and playlist controls

On November 23, 2026, the subtitle drawer opened directly to the track picker, with separate disclosures for Languages, Appearance & Timing and the subtitle list. Chrome and WebKit exercised the English and Chinese interfaces at 320 × 235, 450 × 300, 720 × 430 and 1440 × 900. Language fields measured 32 CSS pixels high at desktop width. The current bilingual selection showed the full names for Japanese and English, Chinese and English in both directions, and Macedonian and Traditional Chinese. At the minimum split-panel width the longer names wrapped within the picker.

`node /tmp/hen-subtitle-playlist-inspector.cjs` passed four browser-and-locale runs against the Vite mock. Each run imported a local SRT, opened online search and export, changed the display language, vertical position and subtitle delay, and expanded a virtualized list of 1,807 generated cues. Regeneration displayed progress and restored the footer action when complete. The playlist cycled through all repeat modes, navigated two files, sought to a chapter and reopened without issuing another chapter query. A file with no chapters displayed only the playlist controls.

`npm --prefix app/ui run check` reported zero diagnostics, and `npm --prefix app/ui run build` produced the production frontend.

## Delete generated subtitles for one audio track

`node /tmp/hen-delete-ui.cjs` exercised the right inspector in Chromium (English) and WebKit (Chinese) using a local Vite server and the sample movie. **Regenerate** and **Delete** stayed on one row inside the inspector at widths of 320, 480, 720 and 1050 CSS pixels. Escape and Cancel preserved all six generated cues. Keyboard focus opened on a safe control and returned to Delete after cancellation. A simulated disk error kept the captions available for another attempt.

Confirming deletion sent one request with the open session ID and audio-track order. With a sidecar SRT selected, its text, selection, playback URL and paused playhead stayed intact. The generated track offered **Transcribe** again, and the deleted cues stayed absent after reopening the file. A new transcription completed successfully. Opening another video dismissed the previous confirmation. Separate checks covered saved partial translations and a completed track with no speech. `npm --prefix app/ui run check` reported zero diagnostics and `npm --prefix app/ui run build` succeeded.
