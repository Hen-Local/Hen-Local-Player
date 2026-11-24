# Same-folder playlist

Opening one local media file now shows the playable audio and video files immediately beside it in the left playlist. The panel displays the folder name, sorts numbered filenames in natural order, and brings the playing row into view. Next, Previous and row selection use the displayed order. Choosing multiple files, adding files, or removing a row leaves the user in control of the queue. Removing a row only edits the playlist.

The native directory scan runs off the playback thread. The player can open the selected file even when listing its parent fails. A late scan cannot replace a newer selection or an edited queue. Opening a downloaded URL does not start a scan of its cache directory.

## Acceptance evidence

- `source scripts/build_env.sh && cargo test --locked --offline -p henlocal-player sibling_playlist_lists_playable_neighbors_and_keeps_the_open_file` passed on macOS. The filesystem fixture includes MP4, uppercase MOV, a Chinese FLAC filename, subtitle/text files, hidden metadata, a directory named with a media extension, a nested file, and a directly opened extensionless file. Opening the MOV with alternative filename casing on the case-insensitive volume retained one selected row.
- `npm --prefix app/ui run check` returned zero errors and warnings. `git diff --check` passed.
- `node /tmp/hen-folder-playlist-test.cjs` exercised the running Svelte player in Chromium and WebKit with controlled native-command responses. Both engines passed single-file picker and Finder-event opens, folder replacement, numeric filename order, clicking a sibling, next/previous, explicit multi-select, add/remove, and single-entry explicit folder opens. The test also verified that item 67 was visible upon opening an 80-entry folder, and that unreadable parents, failed opens, delayed scans, queue edits, downloads, and closing media preserved the expected selection.
