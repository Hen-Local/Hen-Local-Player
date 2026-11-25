# Lane E · Browser player

- [x] The mock player implements the player, transcript, language and subtitle controls, setup, settings, recents, export and URL dialogs in §5.9. The desktop backend contract remains at its foundation revision.
- [x] On Apple Silicon macOS, `npm --prefix app/ui ci`, `npm --prefix app/ui run check` and `npm --prefix app/ui run build` passed after rebasing on `main` at `ca09b00`. Svelte reported zero errors and zero warnings.
- [x] With a separate no-space target, `cargo check --locked -p hlp-core -p hlp-media -p hlp-models -p hlp-pipeline -p hlp-cli -p henlocal-player` passed on the rebased branch.
- [x] `npm --prefix app/ui run dev -- --port 1475` served the app to Chrome. Browser checks used the generated local demo, real HTML video playback, a file chooser, a file drop, keyboard input and actual browser download events. The mock provides canned speech and translation events.
- [x] Dropping the Japanese MP4 reached playing in 349 ms. The sample streamed six cues and showed “Auto · Japanese 96%”. Selecting Chinese produced both subtitle lines with a clear gap above the visible controls. Off removed the overlay. Clicking a row and pressing period sought to the next cue. Fullscreen kept subtitles and controls, and Escape restored the layout.
- [x] Opening the French MKV in the mock displayed preparation progress before playback. Changing and restoring its spoken language marked visible lines stale and replaced them. “Finish first” deferred export until the completion event. Reopening the file immediately restored all six cues.
- [x] Opening the real 20-minute English fixture in Chrome and seeking to 15:00 produced a **mock** cue there in 583 ms. This measures seek prioritization and rendering with canned results. Desktop engine timing is measured in M5/M6.
- [x] Export produced a browser SRT download. Chinese interface selection, optional URL-tool setup, URL preview, progress and cancellation worked. The mobile layout fit a 375 px viewport. The file, language, seek and dialog sequence produced zero page or resource errors.
- [x] Browser keyboard checks exercised Space, ArrowRight, the speed brackets, T, F, Escape, S, Control+L and Control+E. The browser runs used temporary Playwright Core scripts outside the repository (`smoke.cjs`, `more.cjs`, `shortcuts.cjs`) against the dev URL. No browser testing package was added to the app.
- [x] Eight screenshots at 1280×760 capture [empty](../screenshots/mock/empty.png), [player](../screenshots/mock/player.png), [fullscreen](../screenshots/mock/fullscreen.png), [preparation](../screenshots/mock/preparing.png), [export](../screenshots/mock/export.png), [setup](../screenshots/mock/setup.png), [Chinese settings](../screenshots/mock/settings-zh.png) and [URL preview](../screenshots/mock/url.png).

The browser sample has prerecorded Japanese text and canned English and Chinese translations. The desktop build selects TauriBackend through the same interface. Lane F owns the native commands, window fullscreen, setup downloads and real URL flow.

## General settings and model setup

- `npm --prefix app/ui run check` passed with zero errors and warnings. The Settings dialog now contains General, Processing and Storage. The player’s Subtitles panel contains language, appearance and timing.
- `node /tmp/hen-pref-flow.cjs` exercised the settings menu event and actual controls in English Chromium at 820 × 600 and Chinese WebKit at 450 × 300. Back and Escape returned from Model Setup to General and focused the Model Setup button. Close dismissed the preferences entirely. Setup opened independently had no Back button and dismissed on Escape.
- The same browser tests simulated missing components. The default setup request included Qwen3-ASR 1.7B and Hy-MT2 1.8B, excluding the alternate 0.6B model and optional URL tools. Selecting the lighter speech model in Processing replaced only the ASR component in the request. The numeric subtitle lookahead field exposed a description and persisted an edit to 90 seconds.
- Checked the default model preference in `hlp-core/src/settings.rs` and component selection and translation model path in `hlp-models/src/lib.rs`. macOS uses Qwen3-ASR 1.7B and Hy-MT2 1.8B. Windows selects the Sherpa 0.6B speech model and the same translation model.

## Glass reflections and keyboard focus

- Ran `node /tmp/hen-border-review.cjs webkit standard` and the matching Chromium command against the local Vite preview. Inspected Settings over bright and dark frames and the open Subtitles sidebar over the bright frame. The sidebar, track source buttons, section headings, active Settings tab and selected theme button all reported a resting border width of zero. The active theme used a translucent fill. Tabbing through the inspector displayed a 2 px neutral focus outline.
- Repeated the Chromium check with `reduced`, `contrast` and `forced`. Reduced Transparency used an opaque sidebar with the backdrop filter and decorative reflection disabled. Increased Contrast used a solid sidebar and stronger form fields. In Forced Colors mode, keyboard focus followed the system highlight color.

## Recent history during player startup

- Ran `node /tmp/hen-recent-init.cjs` against the Vite preview. With both the settings reply and the media-open listener deliberately held, WebKit and Chromium displayed the recent item. Clicking it queued the movie and opened it after playback capabilities and listeners were ready. The saved Chinese interface preference applied independently.
- Neither `get_setup_status` nor `app_info` appeared in the launch or ordinary file-open command trace. When transcription reported a missing model, the app fetched the catalog and displayed its download size.
- In a separate WebKit run, a delayed initial history reply arrived after a movie opened and closed. The start screen retained the updated list. `npm --prefix app/ui run check` and `git diff --check` passed.

## Stable start screen during history loading

- On November 24, 2026, `node /tmp/hen-welcome-layout.cjs before` exercised the previous frontend through a local Vite server. Populating the first four recent files at the default 720 × 430 window moved the heading and actions up by 70.5 CSS pixels in WebKit and Chromium.
- `node /tmp/hen-welcome-layout.cjs after` measured the mark, title, Open File and Open URL in both engines at 720 × 430, 450 × 300, 530 × 270, 532 × 270, 375 × 198 and 1180 × 780. All four bounding boxes held the same positions and dimensions when the recent count went from zero to one, four, zero and four again. Switching between English, Chinese and the system language also preserved these bounds.
- Long media titles ellipsized within the list. Open File kept focus through history updates, and focusing the last file at 375 × 198 scrolled it fully into view while the heading stayed stationary.

## Version display, available releases and staged updates

- On November 24, 2026, `node /tmp/hen-paid-app-update-ui.cjs` exercised Settings in Chinese WebKit at 450 × 300 and English Chromium at 720 × 430 with a controlled local release service. Settings showed 0.1.0 before checking for releases. A fresh profile made zero release requests by default. The tests covered an empty list, an unavailable release feed, a rate limit, retry, selection of an intermediate update, and release notes containing literal HTML.
- Without a purchase link, an automatic check with the preference enabled and a direct manual check both issued zero release requests. The link field was masked and cleared after submission. The mock credential provider stored only a Boolean in the test harness. The submitted synthetic secret never appeared in page text or browser storage. An expired link cleared the release list. Replacing or removing a link during a deliberately delayed lookup discarded the previous account’s result; removing the credential hid download and install actions after reload.
- The download button showed byte progress and left restart to an explicit action. Opting in persisted the preference and downloaded the latest available release while the player was idle. Opening a movie before enabling the preference initiated zero update commands during playback. Another run restored a staged update offline after a page reload, simulated native rejection of a corrupted package and verified the download action became available again.
- `npm --prefix app/ui run check` reported zero diagnostics and `npm --prefix app/ui run build` produced the production assets.

## Model download source in Settings

- `node /tmp/hen-model-source-ui.cjs` tested Chinese WebKit and English Chromium at 480 × 360. Model Setup defaulted to Automatic. A delayed preference write disabled both the individual and bulk install buttons. The China and international preferences each survived a reload. Clicking the missing 0.6B speech model issued a single install request with that model ID and the selected source in the mock backend’s saved settings. The selector became available again when the install finished.
