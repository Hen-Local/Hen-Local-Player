# Playback controls and playlists

The desktop player accepts multiple startup paths, open-document events, a folder, or files selected together. A playlist keeps playback order and supports adding, removing, advancing, and repeating files. It remembers each file’s viewing position locally and clears the position when the file finishes. A browser session can reopen its sample across reloads. Browsers require the user to select their local files again after reloading.

## Native macOS checks

On November 23, 2026, the debug Tauri app ran with a separate data directory and preferences. An external test script observed its normal WebView events and DOM. AppKit accessibility invoked real menu items. An Apple open-documents event addressed the already running player by process ID.

- Two MP4 arguments opened the first file and populated the playlist in argument order.
- **Playback → Next File** played the second item. **Previous File** returned to the first at 6.300 seconds.
- The open-documents event delivered two additional paths together and opened the first of the new playlist.
- A Japanese/English two-track MKV switched from the original audio to the second track while paused at 7.100 seconds. The test waited for the second track’s opening event, a different playable URL, and a newly loaded video element before checking its position and paused state.
- Missing and corrupt replacements produced errors and retained the same session ID, playable URL and playlist. The original MP4 accepted a session command and sought to two seconds after each error. Playback advanced to 2.602 and 2.274 seconds respectively.
- On a silent MKV, the native **Playback Options** menu displayed the two authored chapters. Selecting **Homeward** sought to 8.000 seconds.
- **File → Close File** closed the active media and cleared the playlist.

## Browser checks

Headless Chrome played real MP4s selected through its file dialog. The smoke tests exercised click to pause, double-click fullscreen, ten-second seek, fit/fill, playlist navigation, and viewing positions. Adding a file preserved the playing file. Removing the playing item selected its successor. End-of-file, repeat-one and repeat-all selected the expected item and playback position.

A folder containing `2.mp4`, `10.mp4`, and a text file yielded the two playable entries in natural order. Dropping two files produced a two-entry playlist. Local browser files used their own bytes and carried no demo transcript. The bundled demo restored its position after a full browser reload. The system media session reflected the playing title and state.

The browser entered picture-in-picture with six real timed caption entries on its native text track. Leaving picture-in-picture disabled that track and returned caption rendering to the window.

## Independent sidebars and fixed playback controls

On November 23, 2026, a native macOS window opened the Japanese sample and its adjacent bilingual SRT with an isolated data directory. At 720 × 430, in fullscreen at 2580 × 1080, and back at 720 × 430, the four combinations of Playlist and Player settings produced identical window-relative rectangles for all eight transport buttons and both sliders. Center-point hit tests reached each control. The sidecar cue stayed visible throughout. Both a debug app bundle and the bare executable from `cargo build --locked -p henlocal-player` passed.

Chrome repeated the rectangle and hit tests at 720 × 430, 1280 × 760, and touch viewports of 390 × 680 and 320 × 235. With both panels open, the picture and authored caption stayed clear of both sidebars and the transport. Escape closed the focused panel; the other stayed available. The T shortcut, native transcript event and General preference opened the unified Subtitles tab, and both panels survived a fullscreen round trip. Holding F left fullscreen unchanged. `npm --prefix app/ui run check` returned zero errors and warnings.

## Horizontal scrolling

On November 23, 2026, a 720 × 430 Mac player received horizontal AppKit scroll gestures while the sample was paused at three seconds. The previous build briefly moved the entire WebView by +38 and −37 CSS pixels. With the viewport locked, all ten gesture sequences kept the page and subtitle list at horizontal offset zero. Each sequence delivered eleven trusted wheel events. Vertical input still moved the subtitle list from 0 to 13 pixels, and diagonal input moved it from 13 to 31. Horizontal gestures over the focused seek and volume sliders preserved the three-second position and 85% volume.

A 250-character unbroken subtitle previously widened the native list from 202 to 1,981 pixels and allowed 528 pixels of sideways movement. The text now wraps within the original 202 pixels. Chrome also verified that the subtitle and playback panels scroll vertically and remain within their bounds with both sidebars open at widths of 320, 450, 720 and 1,024 pixels. The production UI build and `npm --prefix app/ui run check` passed with zero Svelte errors or warnings.

## Pointer leaving the player

On November 23, 2026, Chrome and WebKit played the bundled Japanese sample in an 860 × 480 player frame. Playwright moved a real browser pointer across its viewport boundary. During pause and playback the transport began hiding on exit and reappeared on entry. It reached zero opacity after the 160 ms fade. With both sidebars open, moving from a panel to the surrounding page hid the transport and returning over either panel restored it. The More menu stayed open across an exit, and Escape restored keyboard focus and made the controls visible.

Both engines kept the transport visible while the seek slider was dragged outside the viewport, then hid it on release. Ordinary playback still hid the controls after 2.5 seconds of inactivity inside the frame. On touch input, tapping the movie woke the hidden transport and playback continued. The Svelte check completed with zero errors and warnings.

## Glass transport, speed and long recordings

On November 23, 2026, the speed button, slider, presets, Playback Settings and bracket shortcuts controlled the same media element from 0.25× to 5×. Chrome exercised a mouse at 720 × 430, 1280 × 760 and 450 × 300, and touch at 320 × 235. WebKit repeated the checks at 720 × 430. Both engines advanced the playing sample at a measured 5.00× after startup. With both side panels open, all transport controls stayed at their original window coordinates and passed pointer hit tests. Escape closed the speed selector and restored focus to its button. The gap below the control bar measured 20 pixels at 720 × 430 and 35 at 1280 × 760.

A Chrome rendering check placed two different colors behind the same point on the glass surface. The control picked up the warm and cool colors. Enabling the system Reduce Transparency preference gave the same opaque RGB (36, 38, 45) over either color. The blur is confined to the control surface and its speed selector.

A separate seek-bar check supplied a 65-minute timeline with 1,807 pending recognition intervals and 1,807 translation intervals. With the scrubber hovered, nine successive status snapshots were applied and measured through the next synchronous layout. After the last snapshot, commit `9cd0ad5` used 3,606 descendants under the scrubber and a median 27.8 ms per update after the first one. The new pixel strip used seven descendants and a median 0.3 ms on the same Mac in Chrome. Updating the playback clock 100 times without changing the transcription status caused zero strip redraws. `npm --prefix app/ui run check` passed with zero errors or warnings.

## Larger playback controls and speed presets

On November 23, 2026, Chrome and WebKit opened the bundled Japanese sample through the player menu. Each engine exercised the controls at 1280 × 760, 720 × 430, 510 × 360, 450 × 300, 390 × 350 and 320 × 235. The desktop control bar grew from 520 × 70 to 600 × 87 CSS pixels, and the Play target from 39 × 34 to 48 × 42. At the smallest size the bar fit within 304 × 78 pixels. Every button and slider passed center-point hit testing with the panels closed. A separate Chrome sweep checked 156 window widths between 320 and 630 pixels and found no overlap between adjacent targets or controls outside the glass surface.

In both engines, clicking each of the seven presets set the media element to 0.5×, 0.75×, 1×, 1.25×, 1.5×, 2× and 4× in turn. Home and End on the speed slider selected 0.25× and 5×. Arrow Left then selected 4.75×, which Playback Settings displayed correctly. Selecting a preset in Settings updated the movie again. Escape returned keyboard focus to the speed button.

The Chrome glass check sampled the same portion of the transport over a warm and a cool background. The samples changed from RGB (122, 68, 75) and (57, 100, 111) in the previous build to (164, 68, 76) and (51, 122, 137). With Reduce Transparency enabled, both measured (36, 38, 45). The local checks used `node /tmp/hlp-transport-sizing.cjs` and `node /tmp/hlp-transport-glass.cjs` against Vite. `npm --prefix app/ui run check` reported zero errors and warnings.

## Picture in Picture in the floating controls

Checked the native close and restore actions on November 23, 2026. `picture-in-picture.ts` requests the browser’s video presentation mode. The mode-change listeners in `Player.svelte` update the toolbar and captions. Apple documents Close (×) as pausing playback and Restore as returning a playing video to its original window while playback continues. [Apple’s Safari User Guide](https://support.apple.com/guide/safari/play-web-videos-ibrw27291639/mac).

On November 23, 2026, Chrome opened and closed Picture in Picture using its standard video API through the new transport button. Playwright WebKit exercised the corresponding presentation-mode event path with a simulated PiP window. With a supported video loaded, both engines checked 159 viewport widths from 320 through 1440 CSS pixels. The play button stayed centered, every visible button or slider passed center-point hit testing, and adjacent hit targets did not overlap.

## Playback clock with completed subtitles

On November 23, 2026, `node /tmp/hen-play-clock.cjs PORT ENGINE` compared the original UI at commit e6dcd33 with the changed UI. Playwright WebKit and Chrome played the bundled video with 1,807 completed bilingual cues in a 1112 × 586 window. During four seconds with the transport hidden, both previously ran 240 animation-frame callbacks and sent 15 playhead reports. The seek bar made 408 style mutations in WebKit and 450 in Chrome. The updated UI recorded zero for all three counts in both engines. With the short video’s controls visible, the seek bar made 154 and 164 style mutations respectively, down from 480 each. No recurring backend reports were sent during completed playback. Active generation still received four to five reports in 1.2 seconds. The measurements cover UI updates and scheduler traffic. The 1× sample reported zero dropped video frames in both builds.

The native media clock updates the transport and resume history. A timer follows the next subtitle start, end or page change between native events. Short clips and higher playback speeds receive additional visible-only progress samples when the thumb moves more than one pixel per quarter second. Returning the pointer or pressing a key refreshes the transport from the video clock.

`node /tmp/hen-clock-seek-check.cjs` checked six overlapping SRT transitions in both engines at 2× with 250 ms of subtitle delay and native timeupdate suppressed. Both engines displayed every transition. The largest observed delay was 27 ms of media time. Forward/backward seeks inside the overlap also selected the correct lines. Source, target, subtitle mode, translation context and regeneration each reported the current 9,000-ms paused position before issuing the configuration or start command. Svelte validation returned zero errors and warnings.

A separate, visible macOS 27.2.1 WKWebView played the production frontend with the same 474-cue bilingual track loaded as generated subtitles and as imported SRT. The test backend restored the completed state before measurement. The 1920 × 1080 H.264 clip ran at 29.97 fps. Two passes played the same 6.2-second interval at 1× in opposite order. Generated subtitles dropped 4/186 and 1/185 frames, imported SRT dropped 1/185 and 3/186, and captions disabled dropped 3/185 and 2/190. The media clock advanced within 1 ms of elapsed time in every run. Each caption track displayed four distinct captions. Each run recorded zero hidden transport mutations and zero recurring playhead messages. These short runs showed no consistent frame-drop advantage for either subtitle format.

The identical native test against a production frontend built with `git archive e6dcd33 app/ui app/media-formats.json` recorded 750 hidden transport mutations per mode and 24, 24 and 25 recurring playhead messages for generated, disabled and SRT respectively. Baseline dropped frames were 3/185, 2/185 and 6/188 in that order. Native runs used a test backend to count calls and did not exercise Tauri IPC transport or live inference.

## 4K playback without captions on the Dell display

On November 23, 2026, a visible WKWebView played the original 2160p HomeView recording at 1× on the Dell AW3418DW. The display reported 120 Hz and a 2× backing scale. Each test used an 894 × 1050-point content area, the same transparent window with HUD material, the same local Rust MediaServer and the same media interval. The test sought to 12 seconds, played for 1.1 seconds to settle, then measured the next 52 seconds. The production-compiled frontend used a test backend with inference stopped, captions off and playback controls hidden. The comparison page contained a single video element. Both muted playback paths used identical instrumentation.

| Playback surface in run order | WebKit dropped / total frames | Missed video frame callbacks | Waiting or stalled events |
| --- | ---: | ---: | ---: |
| Player | 17 / 1,257 | 0 | 0 |
| Video element | 18 / 1,257 | 1 | 0 |
| Video element | 14 / 1,257 | 1 | 0 |
| Player | 18 / 1,257 | 3 | 0 |

The playback clock advanced within 2 ms of the elapsed time in every trial. The player sent zero playhead reports while its controls were hidden. All four trials buffered beyond 83 seconds while the measured segment ended near 65 seconds. The layer drop counter increased in approximately two-second batches on both pages. Missed callbacks were identified by jumps in the callback's frame sequence number. A missed callback does not identify the individual frame dropped by the layer. The runs showed similar layer losses with and without the player interface. They did not establish the timing of an individual visible hitch or a link to the file's 10.427-second keyframe interval.

The temporary acceptance harness ran with `python3 /tmp/hen-frame-run.py`. `python3 /tmp/hen-analyze-frames.py` compared media timestamps, callback timing, sequence numbers, layer counters and display animation callbacks. WebKit's AVFoundation implementation retrieves quality metrics from the video layer and obtains video frame callback metadata from a separate output. Its expected display time is estimated from the media clock. [WebKit AVFoundation implementation](https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/graphics/avfoundation/objc/MediaPlayerPrivateAVFoundationObjC.mm).

A final run of `python3 /tmp/hen-frame-low.py bare quiet-bare 52` removed both animation and video frame callbacks, and polled the two layer counters every 500 ms. The bare video element recorded 15 dropped frames out of 1,257 across 52.000 seconds, with the media clock advancing 52.0003 seconds and zero waiting or stalled events. The cumulative dropped count rose by two near media times 22, 32, 42, 52 and 62 seconds, and by one near 16, 26, 36, 46 and 56 seconds. These are observation times for batched layer statistics. The pattern occurred without subtitle rendering, any player controls or an auxiliary output for per-frame JavaScript callbacks.

## Settings, sidebar and transport glass

On November 24, 2026, Playwright WebKit exercised the production frontend at 1000 × 564 in English and Chinese. Settings, the subtitle drawer and playback controls were captured over bright calendar and dim workshop frames from the HomeView clip. The glass has a light-catching rim, a dark translucent tint and a local 11 px blur. The Settings scrim dims the movie without filtering the entire window. Pointer exit and re-entry checks confirmed that the transport's optical layer becomes hidden after the controls fade and returns when the pointer comes back.

`node /tmp/hen-review-glass-final.cjs webkit after en` and the corresponding `zh` run completed without page errors. Chrome repeated the English run and used `chromium after zh reduce` to exercise Reduce Transparency. Pixel samples over a white viewport gave secondary-text contrast ratios of 7.22 to 1 in Settings, 5.75 to 1 in the drawer and 4.66 to 1 on the transport in WebKit. The same Settings sample changed from RGB (66, 36, 25) over warm footage colors to (19, 43, 68) over cool colors. With Reduce Transparency enabled, all four tested backgrounds produced identical samples of (32, 34, 40) for the dialog and drawer and (36, 38, 45) for the transport. These are surface samples from rendered screenshots, not measurements of video frame timing.

## Playback control reveal and hidden idle state

On November 24, 2026, `node /tmp/hen-transport-reveal.cjs webkit` and the corresponding `chromium` run exercised pointer exit, entry and a fast reversal while the sample video played. For each animation frame, the optical surface and playback buttons had matching opacity and their container stayed opaque. The bar stopped receiving pointer hits on exit. After fade-out its glass was hidden, the video kept advancing, and a mutation observer recorded zero transport DOM writes over 420 ms. The speed slider accepted an arrow key, Escape restored focus to its button, and Enter reopened the slider with a visible focus indicator. The `webkit reduce` run verified that Reduce Motion made visibility changes immediate. `npm --prefix app/ui run check` reported zero errors and warnings.

## Playback glass during pointer entry and exit

On November 24, 2026, a 960 × 540 system WKWebView played a local 30 fps H.264 test clip while pointer entry and exit events toggled the player’s controls. The clip included moving shapes and a stationary striped band beneath the bar. A ScreenCaptureKit stream captured only the test window. The moving region produced 98 distinct pixel samples in each run. The unchanged band let us measure when the backdrop actually blurred in the composited image.

Ran `/tmp/hen-reveal-measure` against the prior packaged frontend and the updated production frontend, then ran `python3 /tmp/hen-reveal-summary.py`. On two full reveals in the prior build, the tint had settled when the stripe contrast (standard deviation in an 8-bit green channel) dropped from 34.9 to 0.5 in one captured frame, at 182 ms and 177 ms. In the updated build, contrast during the last part of one reveal went from 12.8 to 7.6, 4.0, 1.7 and 0.5 over 35 ms. The second reveal and a reversal after a 65 ms entry and 50 ms exit also reached 0.5 smoothly. Once hidden, the bar’s optical layer reported hidden visibility and its play button had pointer events disabled.

`node /tmp/hen-reveal-borders.cjs` checked the integrated frontend in WebKit. The transport material, selected speed preset, selected Settings tab and active accent button each had a zero-width border. Selecting a Settings tab by mouse produced no focus outline. Tabbing to the next control produced a 2 px neutral focus ring. The selection highlight moved 296 px between General and Storage. Chromium with Reduce Motion and Reduce Transparency enabled repeated those interaction checks with a solid transport and a zero-duration fade.
