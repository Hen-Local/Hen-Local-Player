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

## Closing and reopening the Mac window

On macOS, the window close handler pauses the movie and saves its resume position before the WebView is destroyed. The player releases the movie and its cache lock while the application remains available in the Dock. Reopening builds a fresh window from the Mac configuration. A Finder file or File menu action received during startup waits until the new window can handle it. Command-Q and the Dock’s Quit action retain their normal application-wide behavior.

On November 23, 2026, a signed Apple Silicon release built with `npm --prefix app/ui run build` and `cargo build --locked --release -p henlocal-player --features custom-protocol` played the bundled Japanese film in an isolated profile. An AppKit probe exercised the application’s actual menu key equivalents and queried its visible WebView windows.

- Command-W at seven seconds left zero visible player windows, released the file’s cache lock and retained the same regular Dock application process. AppKit’s reopen action produced a welcome screen with recent movies. Opening the same file again through the system’s file-open callback resumed playback at 7.065 seconds.
- From zero windows, Command-O and Shift-Command-O displayed the native file and folder pickers. Command-L opened the URL dialog and Command-comma opened Settings. Command-Q from zero windows terminated the process with exit code zero.
- Command-W closed the movie in both the player’s in-place fullscreen mode and a native macOS fullscreen Space. Reopening in each case produced a 720-pixel-wide welcome screen. After closing in-place fullscreen, the application's Dock and menu bar presentation flags matched their original values.

## Animated player fullscreen

The Mac fullscreen button, F, double-click and Escape animate the live player window on its current display. AppKit changes its frame over 220 ms with an ease-in/ease-out curve. Changing direction mid-animation starts from the current frame. The window restores its original frame, title bar, keyboard focus and Dock/menu presentation on exit. The system's Reduce Motion preference makes the size change immediate.

An isolated, signed Apple Silicon build from `cargo build --locked --release -p henlocal-player --features custom-protocol` played the bundled sample while a native probe sampled its window frame every 8 ms and WebKit sampled the video bounds on animation frames. Across entry and exit using F, Escape, the fullscreen button and a double-click, the window traversed 24–26 measured intermediate widths and the video rendered at 13–14 intermediate widths. First movement to the final frame took 216–224 ms on a 2580 × 1080 display. The movie clock advanced on all four transitions. Four toggle requests 50–65 ms apart restored the original window; ⌘W during entry followed by Dock reopen returned to the welcome screen. A separate process with Reduce Motion enabled recorded only the initial and final widths for each control.

## macOS system fullscreen

The fullscreen button, F, and a double-click use the same macOS fullscreen Space as the green window button and the system View menu. AppKit controls the animation. The webview follows the native window state, including reversals requested before a transition finishes.

After `npm --prefix app/ui run build` and `cargo build --locked --offline --release -p henlocal-player --features custom-protocol`, a signed copy of the Mac app played the bundled video with its own data directory. A probe recorded AppKit's will-enter/did-enter and will-exit/did-exit notifications for F, Escape, the transport button, a double-click, the green window button and the system menu. The six transitions took 552–600 ms each on this Mac. The playback clock advanced during each transition and each exit restored the exact starting position and size. Groups of two, three and four rapid presses produced the expected final state in both AppKit and the controls. Closing with ⌘W in fullscreen and during entry both allowed Dock reopen to show the welcome window in the same process. ⌘Q then quit normally.

## Compact playlist and settings overlays

At 720 × 430, opening Playlist in `cca556d` narrowed the movie to 540 × 315 CSS pixels and reserved the bottom strip for the transport. In the updated player the movie stayed at 720 × 430, the left drawer spanned the full height, and the transport held its 600 × 87 bounds while the drawer covered its left end.

`node /tmp/hen-symmetric-drawers.cjs` exercised Chrome and WebKit at nine sizes from 320 × 235 to 1440 × 900. The checks opened the playlist, opened settings alongside it, and compared the video, play button, sidebar toggles and transport geometry through both operations. In compact windows, clicking over the exposed More button dismissed the playlist and returned focus to its toggle. A second click opened the menu. With both sidebars open, an outside click closed the most recently used panel and kept the other one open. Escape, the playlist shortcut and the close buttons remained functional. Portrait and short landscape windows kept the movie edge to edge. The Svelte check passed with zero errors and warnings.

## Drawer height at intermediate window sizes

At 1112 × 586 CSS pixels in `d318c3d`, both sidebars ended at y=464, leaving a 122-pixel gap underneath them. At 1209 pixels wide, the drawers reached the bottom but the transport intercepted clicks over the right drawer. The updated layout gives the drawers the full window height. When the centered 600-pixel transport and a 332-pixel side panel would overlap, the drawer takes precedence in their shared area. At 1112 × 586 the playlist covered the leftmost 22 pixels of the transport and Settings covered the rightmost 76. The picture retained its full 1112 × 586 bounds and the transport stayed at (256, 472) with its 600 × 87 size.

`node /tmp/hen-drawer-boundaries.cjs` tested Chrome and WebKit at 19 viewport sizes from 320 × 235 through 1440 × 900. The test opened each drawer alone and both together, compared video and transport bounds, checked the footer edge and hit-tested the overlapping regions. With both drawers open, 358 further resize samples per engine swept from 1440 to 906 pixels and back. Every sample kept both panels full-height and retained the video's bounds. An outside click at 1112 × 586 closed the last-opened drawer and kept the other visible.

## Floating playback controls

The transport's right group now contains Playlist, Player Settings and More. Removing the fullscreen button freed enough space to center Play even at the minimum window width.

`node /tmp/hen-player-transport-actions.cjs` played the sample in Chrome and WebKit and checked eight viewport sizes from 320 × 235 through 1440 × 900. Every visible control fit within the window, accepted pointer hit tests, and had a separate click target. Play remained centered to the pixel at all eight sizes. In each browser, F entered fullscreen, Escape exited, and double-clicking the video entered and exited fullscreen.
