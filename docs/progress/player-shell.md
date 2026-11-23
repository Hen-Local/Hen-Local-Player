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
