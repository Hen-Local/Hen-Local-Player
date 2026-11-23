# Media player window

The window uses the Hen Local Translator mark, type stack, four accent colors, square buttons, black rules and white panels. The reference was the sibling application's `hen-local-translator-shell/ui/src/App.svelte`, `hen-local-translator-shell/ui/src/styles.css` and `icons/logo-mark.png`. The stage remains black during playback. Users can open files, folders, links, recent items or a sample. Transcript, playlist and subtitle controls open on demand, including while watching fullscreen.

## Browser checks

On 2026-11-23, `npm --prefix app/ui run check` reported zero errors and zero warnings. A local Chrome session verified all of the following against the running player.

- Opened the sample in one click and advanced its 1280 × 720 video. Six Japanese and English transcript rows appeared. Play, pause, settings, subtitle tools, playlist and recent-file navigation worked.
- Closed the current media from Library and returned to the start screen. The sample and local media inputs remained available.
- Opened subtitle tools in fullscreen with C. The first Escape dismissed the tools and the second returned to the window. T opened the transcript and the playlist highlighted the current file.
- At widths of 390 and 320 pixels, the toolbar remained reachable and the inspector fit the viewport. The run recorded zero missing assets or browser script errors.

[Library](../screenshots/player/library.webp) · [Playback](../screenshots/player/playback.webp) · [Subtitle controls](../screenshots/player/subtitle-controls.webp)

## Built-in film

The 22.5-second landscape, train animation and cover image were drawn locally for this player. The clip retains the macOS-generated Japanese narration from the existing speech fixture. FFprobe identified H.264 video at 1280 × 720, 24 fps, and AAC audio in the 1,662,373-byte MP4. Packet extraction produced the same SHA-256 for the sample and the fixture's AAC stream, `113247a8e4527de13af608ea0d446376163ab91c211199124526fda5e5b66309`. The bilingual SRT ships alongside the film. Generating new test fixtures leaves the shipped sample in place.

## Fullscreen transitions during playback

On 2026-11-23, I compared the prior Apple Silicon disk image (SHA-256 `2c20d7073c56650cc031b173bad87dee1c37368d9201b756ba68f3c1261d456d`) with a signed local release built using `cargo build --locked --release -p henlocal-player --features custom-protocol`. Both played the same bundled 1280 × 720, 22.5-second Japanese sample. Each run used a separate player profile. A local timing probe sent AppKit key and mouse events, sampled the window and WKWebView every 36 ms, and recorded the system fullscreen notifications and movie clock.

| Check | Result |
| --- | --- |
| Prior macOS Space transition | The green button completed entry in 678 ms and exit in 610 ms. On a separate F attempt AppKit announced entry and then returned to the original frame without completing the transition. |
| Player fullscreen | F, Escape, the player control, and a double-click filled the 2580 × 1080 display or restored the previous 720 × 430 frame within 35–38 ms in the timing run. The media clock advanced continuously. |
| Mixed shortcuts | F entered player fullscreen. Control-Command-F exited that mode and the next Control-Command-F entered a native Space in 631 ms. Escape completed a single system exit in 593 ms. |
| Native Spaces | The green button and Control-Command-F retained the system transition. Green followed by Escape emitted one exit notification and restored the window. Closing the window from a native Space exited the application cleanly. |
| Space unavailable | In a separate test process I set the individual NSWindow to disallow fullscreen Spaces. Real F, Escape, and double-click still entered and exited player fullscreen. The user’s system preferences were untouched. |
| Live resize | Sixty native window resizes in 480 ms sent 60 fullscreen-state queries in the prior release and one in the revised release. Playback advanced approximately 1.26 seconds over the same 1.25-second observation window. |
| Browser and Windows paths | Chrome opened a local movie, entered genuine document fullscreen and exited with Escape. When the host denied the request, Escape restored the embedded player. A simulated Windows Tauri bridge confirmed ordinary native fullscreen requests, OS state synchronization, and one state query after 160 resize events. A Windows PC was unavailable. |

The native window measurements include the operating system transition where the green button or system menu starts a separate Space. Player fullscreen fills the current display without a Space animation. The 36 ms observation interval limits the precision of the faster measurements.

## Window dragging during playback

The desktop starts a native window drag after the primary mouse moves five CSS pixels on the picture or a non-interactive part of the window. Welcome controls, the transport, side-panel controls, selectable sidebar text and fullscreen keep their usual input behavior. The original title bar keeps its native drag handling.

The local Chrome pointer test used the real player UI, a sample movie and a recording callback for the native drag request. It exercised dragging from the welcome background, the picture and the playlist header, including immediately after a button lost focus. A normal click and a two-pixel movement toggled playback, a double-click toggled fullscreen, and releasing a drag kept the movie paused. Scrubbing changed the playhead, dragging the volume slider changed the volume, and fullscreen drags, playlist buttons and right-button drags made zero window-drag requests. The production build and `npm --prefix app/ui run check` passed with zero Svelte errors or warnings.
