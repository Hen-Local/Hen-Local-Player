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
