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
