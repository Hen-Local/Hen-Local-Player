# Working agreements

1. Follow PLAN.md section 9.1. Verify the CLI vertical slice before connecting the desktop backend.
2. Copy reusable translator code with its source path recorded. Keep dependencies self-contained.
3. Use a separate Cargo target directory without spaces for each worktree.
4. Pin all setup downloads by URL, size and SHA-256. Only the explicit yt-dlp update action queries the latest release.
5. Keep playback and inference offline after setup. Network access is for setup and user-requested downloads.
6. Verify uncertain upstream details and record findings in docs/ARCHITECTURE.md.
7. Record acceptance commands and results in docs/PROGRESS.md. Lane agents own their individual docs/progress file.
8. The ASR worker writes JSON Lines only to stdout. Send diagnostics to stderr.
9. Keep the current version for local builds. Change the version when the user agrees to a release.
