# Guest subtitle search

On November 23, 2026, a client identifying itself as Hen Local Player searched for the public film title *Metropolis* in English. The request supplied no API key, account, or cookies. Results included separate 1927 and 2001 releases. Selecting the 1927 release requested a ticket for its document ID, then fetched the ticket's relative file under the provider's uploads directory. The server returned HTTP 200 and a 27,449-byte UTF-8 SRT with a byte-order mark and 314 timing arrows. The search and download sent no local video path, media hash or audio.

The provider documents guest access and a quota which resets at midnight UTC. Its API documentation and live ticket reported different daily allowances, so the player does not promise a fixed number. The Chinese filter uses the provider's generic `zh` code and displays the language actually returned. Searching *Avatar* with `zh` yielded 20 Chinese results. [Subtitle Search Engine API](https://subtitles.website/docs/).

The offline HTTP fixture in `cargo test --locked --offline -p henlocal-player subtitle_search_engine` checks UTF-8 titles, IMDb lookup, Chinese language codes, duplicate and unsupported formats, the selected file's ticket, SRT decoding and the quota response. A second test verifies that ticket paths stay inside the upload directory. The opt-in Rust command `cargo test --locked -p henlocal-player live_metropolis_guest_download -- --ignored` repeats the guest request to the real service and consumes one download.

Both provider regression tests passed in the integrated `cargo test --locked --offline --workspace` run, with 57 regular tests passing overall. The live Rust test is deliberately opt-in to preserve the guest allowance.
