# Hen Local Player: Prototype Implementation Plan

| | |
| --- | --- |
| Repo | `Hen-Local-Player` (new, sibling of `Hen-Local-Translator`) |
| Product name | **Hen Local Player** (display name), binary `henlocal-player`, bundle id `com.henlocal.player` |
| Platforms | Apple Silicon macOS 14+ (primary), Windows 10/11 x64 |
| Plan date | 2026-11-23 |
| Audience | The coding agent that will implement the prototype in one pass |

---

## 0. Read this first (rules for the implementing agent)

1. **Build in milestone order (section 9)**, or, when several agents are available, in the parallel phases of section 9.1. Either way, the headless CLI vertical slice (M5) must pass before the UI is connected to the real backend (M6). Building the UI against the mock backend earlier is fine. The CLI and the app share one pipeline library, so a passing CLI shows the subtitle engine works.
2. **Copy, don't depend.** Reuse code from the existing translator at
   `/Users/zenghaochen/WORKING/Hen Local/Hen-Local-Translator` by copying the relevant files into this repo with a header comment naming the source file. Do not add path or git dependencies on that repo.
3. **Keep the Cargo target directory out of any path that contains a space.** This repo lives under `.../Hen Local/...`. MLX 0.30.1 generates invalid Metal JIT sources when the target path contains spaces. Every build script must set `CARGO_TARGET_DIR` (default `${TMPDIR}/hen-local-player-target` on macOS, `C:\hlp-target` on Windows) and refuse to run if it contains a space. The translator's `scripts/run_translator_desktop.sh` shows the pattern.
4. **Pin every download** with exact URL, byte size and SHA-256. Never ship a "latest" URL in the manifest; the only exception is the explicit "Update yt-dlp" action (section 5.2.8). Where this plan says `TBD-SHA`, download the file once, compute the hash, and commit it.
5. **Work offline after setup.** Network access is allowed only for (a) first-run model and tool downloads, and (b) user-requested URL downloads.
6. **Items marked `VERIFY`** are facts this plan could not confirm. Check them against the real crate or docs before building on them, and record what you found in `docs/ARCHITECTURE.md`.
7. Keep `docs/PROGRESS.md` updated, with one checkbox per milestone acceptance item and the evidence (command plus output summary). A later agent resumes from that file. In parallel mode, each lane writes only its own `docs/progress/<lane>.md` (section 9.1.1).
8. Never print anything other than protocol JSON to the ASR worker's **stdout**. Logs go to stderr.

---

## 1. Product

### 1.1 One sentence
A local video player that plays a video or movie in a language you don't understand. While it plays, it detects the spoken language, transcribes the speech with timestamps, translates it into your language, and shows subtitles (original, translation or both). All inference runs on your own machine.

### 1.2 Core user flows
- **F1 Open a local file.** Drag and drop, File → Open, or a recent item. The video starts playing at once. Within seconds, subtitles appear at the playhead and keep being produced ahead of it, and the whole file keeps processing in the background.
- **F2 Paste a URL** (YouTube or any site yt-dlp supports). The app shows the title and duration, the user confirms, the file downloads with progress, and then F1 runs.
- **F3 Switch the subtitle display** at any time: Original / Translation / Both / Off. Change the target language; cached translations show instantly, and new ones are generated starting at the playhead.
- **F4 Seek anywhere.** Processing re-prioritizes around the new playhead.
- **F5 Override the detected spoken language** if detection is wrong. Affected lines are transcribed again.
- **F6 Export** SRT, WebVTT or ASS (original, translation, or bilingual) with player-friendly names, for example `movie.ja.srt`, `movie.zh.srt`, `movie.ja-zh.ass`.
- **F7 Re-open** a processed video. Subtitles come back instantly from the cache, and interrupted processing resumes.
- **F8 First-run setup.** The app lists the missing models and tools, then downloads them with resume, progress and checksum verification. Playback works before setup finishes; only subtitles wait.
- **F9 Transcript panel.** A scrolling list of lines with the current line highlighted. Clicking a line seeks to it, and shortcuts jump to the previous or next line.

### 1.3 Decisions already made (by the product owner)
| Topic | Decision |
| --- | --- |
| Playback engine | Tauri 2 WebView `<video>` plus FFmpeg. FFmpeg extracts audio and remuxes or transcodes formats the WebView can't play. Subtitles are a custom HTML overlay. |
| Repo | New repo `Hen-Local-Player`. Copy the shared pieces from the translator. |
| Prototype scope | Local files, **online URLs (yt-dlp)** and a **Windows build**, as well as macOS |
| Translation targets | All Hy-MT2 languages (38 entries, section 5.1.3). Default follows the system language. |
| Explicitly NOT in prototype | TTS voice-over or dubbing; Whisper fallback ASR |

### 1.4 Out of scope for the prototype
TTS dubbing; speaker diarization; word-level (karaoke) timing; forced alignment; importing or translating subtitle tracks already embedded in the file; live streams; playlists; streaming playback of a URL before its download finishes; Linux; accounts, licensing and payments; auto-update; code signing and notarization (document it, don't block on it). See the Phase 2 backlog (section 12).

### 1.5 Definition of done (whole prototype)
1. On an Apple Silicon Mac with no models, `scripts/dev.sh` builds and launches the app. The setup screen downloads everything, and the following then works:
   - Open `fixture_ja.mp4` → the language is detected as Japanese, and Japanese plus Chinese subtitles appear in the overlay while it plays, with timing within ±0.6 s of speech onset.
   - Open `fixture_fr.mkv` (needs remux), `fixture_es.avi` (needs transcode) and `fixture_de.webm`: each plays and gets subtitles.
   - Seek to the middle of a 20-minute file → subtitles appear there within 5 s.
   - Export SRT, VTT and ASS; `ffmpeg -i out.srt -f null -` parses each without errors.
   - Paste a URL → download → play → subtitles.
   - Quit and reopen the same file → the subtitles are restored instantly.
2. `hlp-cli subtitle <file> --to zh` produces the same subtitles headlessly (this is what the E2E tests use).
3. The Windows x64 build runs the same flows with the Windows backends (section 10). Lower speed is acceptable.
4. `cargo test` (pure crates) and `npm run check` pass, and `docs/PROGRESS.md` lists evidence for every milestone.

---

## 2. Existing assets to reuse

### 2.1 Code to copy from `Hen-Local-Translator`
All paths are relative to `/Users/zenghaochen/WORKING/Hen Local/Hen-Local-Translator/`.

| Source | What to take | Goes to |
| --- | --- | --- |
| `hen-local-models/src/lib.rs` | `model_root()` = `~/.OminiX/models` (**shared with the translator, so existing downloads are reused**); `LLAMA_RELEASE = "b11429"`; `llama_runtime_dir()`, `llama_server_path()`; the Hy-MT2 `GgufSource` (repo `tencent/Hy-MT2-1.8B-GGUF`, revision `a0c709d9fac510f2c807aa3af52872340dc37a4a`, file `Hy-MT2-1.8B-Q4_K_M.gguf`, bytes `1133080448`, sha256 `dc5f44fcf1fa496ee7ad725982c0c8c553a4de00259b53af84c4b89fb0c06699`); the GGUF `ready()` checks (size, `GGUF` magic, `.gguf.sha256` sidecar) | `crates/hlp-models` |
| `hen-local-init/src/gguf.rs` | `verify_sha256`, llama.cpp runtime download, extraction and installation; asset names and hashes: `llama-b11429-bin-macos-arm64.tar.gz` / `740288ec6887be94280a5dfa25b5e23a78285cab104519e6c7e218904ee82459`, `llama-b11429-bin-win-cpu-x64.zip` / `1283323272b04cd07905816a597a0da810918102de958f4ff6f7bbaa70ed2efe`; `.part` resume; atomic rename | `crates/hlp-models` |
| `hen-local-init/src/main.rs` | HF to ModelScope provider fallback; `ASR_MODEL_FILES` list for `mlx-community/Qwen3-ASR-1.7B-8bit` (≈2,473,308,160 bytes); completion marker `.moxin-model-complete.json` (**keep this format** so the translator's downloads count as complete); bootstrap lock; retry policy | `crates/hlp-models` |
| `node-hub/dora-qwen35-translator/src/backend_llamacpp.rs` | `LlamaServer::spawn` (free port, `-ngl 99` on macOS / `0` on Windows, `-c 4096`, `--jinja`, stderr drain thread, health polling, `Drop` kill); `sanitize_translation_output`, `strip_think_block`; reasoning-leak retry | `crates/hlp-pipeline/src/translate.rs` |
| `node-hub/dora-qwen3-asr/src/backend/macos.rs`, `main.rs` | `AsrEngine` shape, `sanitize_audio_samples` (NaN/Inf to 0, clamp to ±1), `ASR_MIN_SAMPLES_16K = 1600`, `mlx_clear_cache()` | `crates/hlp-asr-worker` |
| `node-hub/dora-qwen3-asr/src/backend/windows.rs` | sherpa-onnx `OfflineRecognizer` setup pattern and `strip_rich_tags` | `crates/hlp-asr-worker` (Windows) |
| `node-hub/dora-qwen3-tts-mlx/patches/qwen3-tts-mlx/Cargo.toml` | How to vendor an OminiX crate with git deps on `mlx-rs`, `mlx-sys` and `mlx-rs-core` pinned to rev `6aac996db8b71fb7dae7a2409c46b4f2ade93092` | `vendor/qwen3-asr-mlx/Cargo.toml` |
| `scripts/run_translator_desktop.sh` | No-spaces `CARGO_TARGET_DIR` guard; build workers before `tauri dev` | `scripts/dev.sh` |
| `scripts/build_macos_app.sh`, `.github/workflows/release.yml` | `MLX_PREBUILT_PATH` (prebuilt MLX archive `mlx-prebuilt-v0.1.0-macos-arm64`); `mlx.metallib` must sit at `Contents/Resources/mlx.metallib` with a `Contents/MacOS/Resources` link, because MLX looks for `Resources/mlx.metallib` beside each executable | `scripts/build_macos_app.sh` (M9) |
| `scripts/make_e2e_wavs.ps1` | Windows SAPI speech fixture generation | `scripts/make_test_media.ps1` |
| `hen-local-translator-shell/ui/package.json`, `tauri.conf.json` | Toolchain versions: Tauri 2, `@tauri-apps/api ^2.8`, Svelte `^5.39`, Vite `^7.1`, `@sveltejs/vite-plugin-svelte ^6.2`, TypeScript `^5.9`, svelte-check `^4.3` | `app/` |
| `docs/MODEL_AND_ASSET_LICENSE_AUDIT.md` | License-audit format | `docs/LICENSES.md` |

### 2.2 Upstream crate used directly
- `qwen3-asr-mlx` from `https://github.com/OminiX-ai/OminiX-MLX.git` at rev **`6aac996db8b71fb7dae7a2409c46b4f2ade93092`** (MLX 0.30.1, the same rev as the translator), **vendored with a small patch** (section 5.4). Do **not** move to OminiX-MLX `main`: its `qwen3-asr-mlx` adds `transcribe_batch` but requires MLX 0.32, which is untested in this toolchain. Batching is a Phase 2 optimization.
- The local cargo checkout already exists at `~/.cargo/git/checkouts/ominix-mlx-4c1b6923576390a4/6aac996/qwen3-asr-mlx/`.

### 2.3 State of the development Mac (as of 2026-11-23)
- Apple M4 Pro, 48 GB, macOS 27.2.1, rustc 1.92.0, Node 20.20.0.
- Already present: `~/.OminiX/models/qwen3-asr-1.7b`, `~/.OminiX/models/qwen3-asr-0.6b`, `~/.OminiX/models/Hy-MT2-1.8B-Q4_K_M.gguf` (+ `.sha256`), and `~/.OminiX/llama.cpp/b11429/` (llama-server). The setup screen should report these as **Ready** without downloading them again.
- `ffmpeg`/`ffprobe` are on PATH from `/opt/anaconda3/bin`, an unknown build that may lack VideoToolbox. Use the managed pinned ffmpeg even in development (section 5.3). `mpv` is not installed (and not needed).

---

## 3. Architecture

### 3.1 Process and data flow

```
┌──────────────────────────── henlocal-player (Tauri 2, Rust) ─────────────────────────────┐
│                                                                                           │
│  WebView UI (Svelte 5)                         Rust backend                               │
│  ┌──────────────────────────┐   commands/      ┌──────────────────────────────────────┐   │
│  │ <video src=http://127.0. │◀── events ──────▶│ app (Tauri commands, event bridge)   │   │
│  │   0.1:P/m/<token>>       │                  │  └ hlp-pipeline::Session             │   │
│  │ subtitle overlay (HTML)  │                  │      ├ hlp-media::Prober  (ffprobe)  │   │
│  │ transcript panel         │                  │      ├ hlp-media::PlaybackPrep       │──▶ ffmpeg (remux/transcode → cache/playable-N.mp4)
│  │ seek bar + coverage      │                  │      ├ hlp-media::AudioExtractor     │──▶ ffmpeg (→ 16 kHz mono s16le PCM in cache)
│  └──────────────────────────┘                  │      ├ VAD (sherpa-onnx Silero)      │
│            ▲  HTTP Range                       │      ├ Scheduler (playhead priority) │
│            └──────────── hlp-media::MediaServer│ (axum, 127.0.0.1, tokenized)          │
│                                                │      ├ AsrClient ──stdio JSONL──────▶│──▶ hlp-asr-worker (sidecar)
│                                                │      │                               │      macOS: Qwen3-ASR-1.7B-8bit (MLX)
│                                                │      │                               │      Windows: Qwen3-ASR-0.6B-int8 (sherpa-onnx)
│                                                │      ├ Translator ──HTTP─────────────│──▶ llama-server b11429 + Hy-MT2-1.8B Q4_K_M
│                                                │      ├ CueBuilder / Exporters        │
│                                                │      └ Store (cache dir JSON + PCM)  │
│                                                │  hlp-media::Downloader ───────────────│──▶ yt-dlp (+ deno, + ffmpeg)
│                                                │  hlp-models::Setup (downloads)        │
│                                                └──────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────────────────────────────┘
hlp-cli uses the same hlp-pipeline::Session with a "process everything from 0" playhead policy.
```

### 3.2 Why these choices
- **WebView `<video>` with an HTML subtitle overlay.** This matches the team's Tauri and Svelte stack, gives full control of bilingual styling and live cue updates, and is the most likely design to work in one pass. Format coverage comes from FFmpeg remux or transcode when the WebView can't play a file.
- **ASR in a sidecar process (`hlp-asr-worker`).** It isolates MLX/Metal or ONNX crashes and memory from the UI process and keeps MLX out of the Tauri crate's build. The worker can be killed to free memory. It mirrors the translator's process separation, but **without Dora**: offline file processing doesn't need a dataflow runtime, and Dora would add a CLI dependency.
- **Translation via the pinned `llama-server`.** It is already proven in the translator on both macOS (Metal) and Windows (CPU), and the runtime is already installed on the dev Mac.
- **Timestamps from VAD (Silero) plus proportional allocation.** Qwen3-ASR outputs text only, with no timestamps (checked in the `qwen3-asr-mlx` source and the sherpa-onnx docs). Speech regions give accurate cue boundaries (about ±0.3 s). Inside a multi-sentence utterance, time is split in proportion to text length across the speech portions only. A forced aligner is Phase 2.
- **Cache keyed by content fingerprint.** Re-opening is instant, and renaming or moving a file doesn't lose work.

### 3.3 Concurrency model
- One Tokio runtime (Tauri's). The CLI creates its own.
- `Session` is a Tokio task that owns all state for the open media and receives `SessionCommand`s over an mpsc channel (`Playhead{ms, playing}`, `SetTarget`, `SetSource`, `SelectAudioTrack`, `Export`, `Shutdown`).
- Audio extraction runs on a blocking thread that reads ffmpeg stdout, appends to the PCM file, and pushes 512-sample frames into a crossbeam channel to the VAD thread. The VAD thread sends `SpeechRegion`s to the session.
- `AsrClient` is a dedicated thread that owns the worker's stdin and stdout, with one request in flight. It talks to the session through channels.
- `Translator` uses an async `reqwest` client with one request in flight (raise to 2 later if measurements allow).
- Only one Session is active. Opening new media shuts the old Session down, but **the ASR worker and llama-server stay alive** so models stay loaded. Both are pre-warmed at app start when their models are ready.

---

## 4. Repository layout

```
Hen-Local-Player/
├─ PLAN.md                       # this file
├─ README.md                     # build/run instructions (write in M0, keep current)
├─ AGENTS.md                     # short rules: sections 0.1–0.8 of this plan
├─ Cargo.toml                    # [workspace] resolver = "2"; members = crates/*, app; exclude = ["vendor/qwen3-asr-mlx"]
├─ rust-toolchain.toml           # channel = "1.92.0"
├─ crates/
│  ├─ hlp-core/                  # pure logic + types; no process spawning, no network (unit-testable everywhere)
│  ├─ hlp-media/                 # ffprobe/ffmpeg/yt-dlp wrappers, playback decision, PCM extraction, media HTTP server
│  ├─ hlp-models/                # component manifest, paths, readiness, downloads (copied/adapted from translator)
│  ├─ hlp-pipeline/              # Session orchestrator: VAD, scheduler, AsrClient, Translator, Store, events
│  ├─ hlp-asr-worker/            # bin: stdio JSONL ASR server; cfg(macos)=MLX, cfg(windows)=sherpa-onnx
│  └─ hlp-cli/                   # bin: setup / probe / subtitle / langs
├─ vendor/
│  └─ qwen3-asr-mlx/             # patched copy of OminiX-MLX@6aac996 qwen3-asr-mlx (macOS only)
├─ app/                          # Tauri app (mirrors hen-local-translator-shell layout)
│  ├─ Cargo.toml                 # package henlocal-player
│  ├─ tauri.conf.json
│  ├─ build.rs
│  ├─ capabilities/default.json
│  ├─ icons/                     # copy Hen Local icons from translator for now
│  ├─ src/{main.rs, commands.rs, state.rs, events.rs, prewarm.rs}
│  └─ ui/                        # Svelte 5 + Vite + TS
│     ├─ package.json, vite.config.ts, tsconfig.json, index.html
│     ├─ public/demo/            # generated demo clip for mock mode (gitignored)
│     └─ src/{main.ts, App.svelte, styles.css, lib/...}
├─ scripts/
│  ├─ dev.sh / dev.ps1           # set no-space CARGO_TARGET_DIR, build worker + cli, run tauri dev
│  ├─ make_test_media.sh / .ps1  # generate fixtures (section 8.2)
│  ├─ e2e.sh / e2e.ps1           # run hlp-cli on fixtures and assert (section 8.3)
│  └─ build_macos_app.sh         # M9
├─ tests/fixtures/               # generated, gitignored
└─ docs/{ARCHITECTURE.md, PROGRESS.md, progress/<lane>.md, LICENSES.md}
```

Workspace notes:
- `vendor/qwen3-asr-mlx` is **excluded** from the workspace and pulled in only through `[target.'cfg(target_os = "macos")'.dependencies]` in `hlp-asr-worker`. On Windows, build with explicit `-p` flags, never `--workspace`.
- `[workspace.package] version = "0.1.0"`, `edition = "2021"`, `license = "Apache-2.0"`.
- Main shared dependencies: `tokio 1 (full)`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber`, `tracing-appender`, `parking_lot`, `crossbeam-channel`, `dirs 5`, `sha2`, `hex`, `rand`, `reqwest 0.12 (rustls-tls, json, stream, blocking)`, `axum 0.8`, `tower-http 0.6 (fs)`, `unicode-width`, `unicode-segmentation`, `whatlang`, `clap 4 (derive)`, `indicatif` (CLI), `flate2` + `tar` + `bzip2` + `zip` (archives), `sherpa-onnx = "2.0.3"` (static libs auto-downloaded by its build script for `osx-arm64` and `win-x64`; `SHERPA_ONNX_ARCHIVE_DIR` can point at pre-downloaded archives), and `proptest` (dev).

---

## 5. Component specifications

### 5.1 `hlp-core`: types and pure logic

#### 5.1.1 Core types (Rust; serde, camelCase for UI-facing JSON)

```rust
pub type Ms = u64;

pub struct SpeechRegion { pub start_ms: Ms, pub end_ms: Ms }            // from VAD

pub enum UttStatus { Pending, Done, NoSpeech, Failed { attempts: u8, error: String } }

pub struct Utterance {                     // unit of ASR and of translation
    pub id: String,                        // "u000042" = index in VAD discovery order (deterministic)
    pub start_ms: Ms, pub end_ms: Ms,      // span incl. internal pauses (no padding)
    pub regions: Vec<SpeechRegion>,        // speech parts inside the span (≥1)
    pub status: UttStatus,
    pub text: Option<String>,              // ASR text (cleaned)
    pub language: Option<LangCode>,        // detected or forced
    pub forced_language: Option<LangCode>, // language passed to ASR, if any
    pub asr_model: Option<String>,
}

pub struct Cue {                           // what the UI and exporters consume (derived, not persisted)
    pub id: String,                        // "<utt_id>-<k>"
    pub utterance_id: String,
    pub start_ms: Ms, pub end_ms: Ms,
    pub original: String,
    pub translation: Option<String>,
    pub lang: Option<LangCode>,
    pub stale: bool,                       // shown but being re-processed
}

pub struct LangCode(pub String);           // BCP-47-ish: "zh", "zh-Hant", "yue", "ja", "en", ...
```

#### 5.1.2 Language table (`hlp_core::lang`)
One static table with columns `code`, `english_name` (for prompts and UI), `chinese_name` (for Chinese prompts and UI), `qwen3_asr_name: Option<&str>`, `hy_mt2_target: bool`, `whatlang: Option<whatlang::Lang>`.

**Qwen3-ASR spoken languages (30; exact names used in the ASR prompt):**
Chinese `zh`, English `en`, Cantonese `yue`, Arabic `ar`, German `de`, French `fr`, Spanish `es`, Portuguese `pt`, Indonesian `id`, Italian `it`, Korean `ko`, Russian `ru`, Thai `th`, Vietnamese `vi`, Japanese `ja`, Turkish `tr`, Hindi `hi`, Malay `ms`, Dutch `nl`, Swedish `sv`, Danish `da`, Finnish `fi`, Polish `pl`, Czech `cs`, Filipino `tl`, Persian `fa`, Greek `el`, Romanian `ro`, Hungarian `hu`, Macedonian `mk`.

**Hy-MT2 targets (38 rows, from the official README; English name / code / Chinese name):**
Chinese zh 中文 · English en 英语 · French fr 法语 · Portuguese pt 葡萄牙语 · Spanish es 西班牙语 · Japanese ja 日语 · Turkish tr 土耳其语 · Russian ru 俄语 · Arabic ar 阿拉伯语 · Korean ko 韩语 · Thai th 泰语 · Italian it 意大利语 · German de 德语 · Vietnamese vi 越南语 · Malay ms 马来语 · Indonesian id 印尼语 · Filipino tl 菲律宾语 · Hindi hi 印地语 · Traditional Chinese zh-Hant 繁体中文 · Polish pl 波兰语 · Czech cs 捷克语 · Dutch nl 荷兰语 · Khmer km 高棉语 · Burmese my 缅甸语 · Persian fa 波斯语 · Gujarati gu 古吉拉特语 · Urdu ur 乌尔都语 · Telugu te 泰卢固语 · Marathi mr 马拉地语 · Hebrew he 希伯来语 · Bengali bn 孟加拉语 · Tamil ta 泰米尔语 · Ukrainian uk 乌克兰语 · Tibetan bo 藏语 · Kazakh kk 哈萨克语 · Mongolian mn 蒙古语 · Uyghur ug 维吾尔语 · Cantonese yue 粤语.

Rules:
- ASR languages **not** in Hy-MT2's list (sv, da, fi, el, ro, hu, mk) are still translated, but the UI shows a "translation quality unverified for this language" badge.
- The target dropdown lists all 38 entries plus **"None (transcribe only)"**.
- The default target comes from the system locale: `zh-TW`/`zh-HK`/`zh-Hant*` → `zh-Hant`; other `zh*` → `zh`; otherwise the locale's language if it is in the list; otherwise `en`.
- **Skip translation** when the locked source language code equals the target code exactly. (`yue` → `zh` is still translated.)

#### 5.1.3 Translation prompts (`hlp_core::prompts`, `PROMPT_VERSION = 1`)
Hy-MT2 takes **no system prompt**; send a single user message. Copy the templates verbatim from the official README (`https://raw.githubusercontent.com/Tencent-Hunyuan/Hy-MT2/HEAD/README.md`, "Translation Task Instruction Examples"), **removing the markdown bold `**` markers** (`VERIFY` the exact punctuation and newlines):

- **Chinese template.** Used when the source *or* the target is `zh`, `zh-Hant` or `yue`; insert the Chinese target name (for example 英语):
  `将以下文本翻译为{target_zh}，注意只需要输出翻译后的结果，不要额外解释：\n\n{source}`
- **English template.** Used for all other pairs; insert the English target name:
  `Translate the following text into {target_en}.\nNote that you should only output the translated result without any additional explanation:\n\n{source}`
- **Background/context template** ("Structured Data 2" in the README: 【背景信息】… / [Background Information]…). Used only when the `translation_context` setting is on (default **off**, experimental). The background is the previous utterance's original text.
- If the source language is unknown, choose the template from the target alone.
- Unit tests: template selection for zh↔x and x↔y pairs, verbatim strings, and source text with `<`, quotes and newlines kept unchanged at the end of the prompt.

Sampling (llama-server `/v1/chat/completions`, non-streaming): `temperature 0`, `repeat_penalty 1.05`, `max_tokens 384`, `cache_prompt true`. The README recommends `temperature 0.7 / top_p 0.6 / top_k 20`. We use greedy decoding so results are deterministic and cacheable, matching the measured translator setup. Keep these values as constants in one place.

#### 5.1.4 ASR output parsing (`hlp_core::asr_text`)
This mirrors the official `qwen_asr` `parse_asr_output`:
- Raw auto-mode output looks like `language Japanese<asr_text>こんにちは…`. `<asr_text>` is a **non-special added token** (id 151704), so it survives decoding with `skip_special_tokens=true`.
- Split on the first `<asr_text>`. If the meta part (lowercased) contains `language none`, the result is **no speech** (return `NoSpeech` even if the text is non-empty and very short; if the text is longer than 8 characters, keep it as text with an unknown language). Otherwise take the first meta line starting with `language ` and map the name to a `LangCode` through the table.
- No `<asr_text>` tag: the whole string is text and the language is unknown.
- **Cleaning:** trim; collapse whitespace; strip `<|…|>` tags (Windows/sherpa); **repetition collapse** (any substring of 1–20 characters, or 1–6 words, repeated ≥4 times in a row is reduced to a single occurrence); drop results that are empty or punctuation-only.
- **Hallucination guard:** if `display_cols(text) / speech_seconds > 45`, mark the result `Failed("implausible length")` and retry once with a forced language (when known) and `max_tokens` halved. A second failure marks `NoSpeech`.

#### 5.1.5 Cue builder (`hlp_core::cues`): deterministic, pure
Constants: `MAX_LINE_COLS = 42` (display columns via `unicode-width`; CJK counts 2), `MAX_LINES = 2`, `MAX_CUE_COLS = 84`, `MIN_CUE_MS = 1000`, `MAX_CUE_MS = 7000`, `MIN_GAP_MS = 80`, `LINGER_MS = 500`, `READ_COLS_PER_SEC = 20`.

For each `Done` utterance:
1. **Atoms.** For scripts with spaces, split into words (trailing punctuation stays attached). For scripts without spaces (zh, ja, yue, th, km, my, lo, bo), split into grapheme clusters (`unicode-segmentation`, **never split inside a grapheme**, which matters for Thai and Hindi). Punctuation attaches to the previous atom.
2. **Sentences.** Split at sentence enders (`. ! ? 。 ！ ？ … ؟ ।` plus closing quotes or brackets that follow them).
3. **Long sentences** (more than `MAX_CUE_COLS`): recursively split at the clause punctuation (`, ; : ， ； ： 、 —`) nearest the middle; otherwise at the word boundary nearest the middle; otherwise at the grapheme nearest the middle.
4. **Merge short pieces.** Merge adjacent pieces of the same utterance when each is under 20 columns and together they fit in `MAX_CUE_COLS`.
5. **Timing.** Build a piecewise map from the utterance's *speech-only* time (the sum of its `regions`, gaps excluded) to absolute ms. Each piece's weight is `max(1, display_cols)`. For piece i, `start = map(total_speech * cum_before/total_cols)` and `end = map(total_speech * cum_after/total_cols)`. A boundary that lands inside a gap snaps to the gap's start (end of piece) or end (start of next piece).
6. **Global pass** over all cues sorted by start time, after building every utterance:
   - Enforce `end ≥ start + max(MIN_CUE_MS, cols/READ_COLS_PER_SEC*1000)` by extending into the following silence, never past `next.start - MIN_GAP_MS`.
   - Add a linger of up to `LINGER_MS` into the silence.
   - If a cue is still shorter than 500 ms and has a neighbor in the same utterance, merge them.
   - **Invariant:** no overlaps and `start < end` (covered by a property test with `proptest`).
7. **Line wrapping** (exports only; the UI uses CSS): wrap into at most 2 lines balanced near the middle, preferring breaks at clause punctuation and then at spaces. Insert no spaces in CJK text.

**Translation distribution** (utterance translation → its cues):
- One cue: it gets the whole translation.
- If the number of translated sentences equals the number of cues, assign one sentence per cue.
- Otherwise split proportionally. The target fraction for cue k is `cols(original_k)/cols(original_total)`. Walk the translation's atoms and cut at the atom boundary closest to each cumulative fraction, preferring a punctuation boundary within ±15 % of the target.

#### 5.1.6 Subtitle writers (`hlp_core::subs`)
- **Variants:** `Original`, `Translation`, `Bilingual { order: TranslationFirst | OriginalFirst }`.
- **SRT:** 1-based index; `HH:MM:SS,mmm --> HH:MM:SS,mmm` (hours may exceed 99 for very long media; zero-pad to at least 2 digits); wrapped lines; blank line; UTF-8 **without** a BOM; `\n` line endings (`\r\n` on Windows is optional).
- **WebVTT:** `WEBVTT` header and blank line; `HH:MM:SS.mmm`; escape `&`, `<`, `>`.
- **ASS:** `[Script Info]` with `ScriptType: v4.00+`, `PlayResX: 1920`, `PlayResY: 1080`, `ScaledBorderAndShadow: yes`, `WrapStyle: 0`. Two styles: `Trans` (Fontsize 64, white, outline 3, shadow 0, MarginV 60) and `Orig` (Fontsize 46, `&H00E0E0E0`, outline 2). Font name `PingFang SC` on macOS and `Microsoft YaHei` on Windows (players fall back to other fonts). The bilingual Dialogue text is `{\rTrans}<trans>\N{\rOrig}<orig>` (or the reverse order). Times use `H:MM:SS.cc` (centiseconds, rounded). Escape `{`→`｛`, `}`→`｝`, and turn newlines into `\N`.
- **Default filenames:** `<stem>.<src>.srt`, `<stem>.<tgt>.srt`, `<stem>.<src>-<tgt>.ass`, `.vtt` equivalents.
- Golden-file unit tests for every format, including a cue at 1:02:03.456, CJK, RTL (Arabic) and an empty translation (omitted in Translation mode; Bilingual falls back to the original only).

#### 5.1.7 Scheduler priority (`hlp_core::schedule`)
`fn priority(utt_start: Ms, playhead: Ms, process_whole: bool) -> Option<u64>`:
- Ahead, or current within a 2 s back-window (`utt_start + 2000 ≥ playhead`): `Some(utt_start.saturating_sub(playhead))`, so nearest first.
- Behind: `Some(1_000_000_000 + (playhead - utt_start))` when `process_whole`, otherwise `None` (skip).
- The ASR picker chooses the `Pending` utterance with the lowest priority. The translation picker chooses the lowest-priority `Done` utterance with no translation for the current target. Both evaluate in O(n) per pick, which is fine for 3,000 or fewer utterances.
- Pause rule: when `process_whole` is false, stop picking work beyond `playhead + lookahead_s`.

#### 5.1.8 Fingerprint
`sha256(len_u64_le ‖ first 4 MiB ‖ last 4 MiB)`, hex, first 32 characters. Unit test: same content at different paths gives the same fingerprint; one changed byte in the head changes it.

#### 5.1.9 ASR worker protocol types (`hlp_core::asr_protocol`)
These are shared by the worker and `AsrClient`; see section 5.5.

---

### 5.2 `hlp-media`: FFmpeg, playback preparation, media server, downloader

#### 5.2.1 Tool resolution (`Tools`)
Resolve the paths of `ffmpeg`, `ffprobe`, `yt-dlp` and `deno` in this order: env override (`HLP_FFMPEG`, `HLP_FFPROBE`, `HLP_YTDLP`, `HLP_DENO`) → managed install under `~/.OminiX/tools/<name>/<version>/` (from `hlp-models`) → PATH, **only** when `HLP_ALLOW_SYSTEM_TOOLS=1`. Every command runs with `-nostdin -hide_banner` and, on Windows, the `CREATE_NO_WINDOW` creation flag so no console window flashes.

#### 5.2.2 Probe
`ffprobe -v error -print_format json -show_format -show_streams <input>` → `MediaInfo { container: format_name, duration_ms, video: Option<VideoStream{codec_name, codec_tag_string, profile, pix_fmt, width, height, field_order}>, audio: Vec<AudioStream{index, order, codec_name, channels, channel_layout, sample_rate, language_tag, default}>, has_subtitle_streams }`.
- Select the default audio track (disposition default, otherwise the first). Error out if there are no audio streams ("This file has no audio").
- Audio-only files are supported: the stage shows a black frame and an audio icon.

#### 5.2.3 Playback capability probe (from the UI)
At startup the UI calls `video.canPlayType()` for the strings below and sends a `{mime: "probably"|"maybe"|""}` map to the backend (`set_playback_capabilities`). Treat `"maybe"` as playable.
```
video/mp4; codecs="avc1.640028"      video/mp4; codecs="hvc1.1.6.L120.90"   video/mp4; codecs="av01.0.08M.08"
video/mp4; codecs="vp09.00.40.08"    video/webm; codecs="vp9"               video/webm; codecs="vp8"
video/webm; codecs="av01.0.08M.08"   audio/mp4; codecs="mp4a.40.2"          audio/mp4; codecs="opus"
audio/mp4; codecs="flac"             audio/mpeg                             audio/mp4; codecs="ac-3"
audio/mp4; codecs="ec-3"             audio/webm; codecs="opus"              audio/webm; codecs="vorbis"
```
The CLI `probe` command takes `--webview wkwebview|webview2` and uses a built-in default map for each.

#### 5.2.4 Playback decision (`decide(&MediaInfo, &Caps, selected_audio) -> PlaybackPlan`)
| Case | Plan |
| --- | --- |
| Container mp4/mov/m4v, video codec playable in MP4, audio playable in MP4, selected track is the default, HEVC tagged `hvc1` | **Direct** (serve the original file) |
| Container webm, VP8/VP9/AV1 and Opus/Vorbis playable | **Direct** |
| Video codec playable in MP4 but the container isn't (mkv, avi, ts, flv…), or the audio needs conversion, or HEVC is tagged `hev1`, or a non-default audio track is selected | **Remux** to `media/<fp>/playable-<order>.mp4` |
| Video codec not playable (mpeg2video, vc1, mpeg4/xvid, wmv*, h263, prores, HEVC/AV1 where caps say no, interlaced content) | **Transcode** video |
| Audio-only | Direct if the audio codec is playable, otherwise transcode to AAC in `.m4a` |

Commands (all with `-progress pipe:2 -nostats -v error -y`; parse `out_time_us=` and `progress=end`):
- **Remux:** `ffmpeg -i IN -map 0:v:0 -map 0:a:<order> -sn -dn -c:v copy [-tag:v hvc1 if hevc] -c:a <copy|aac -b:a 192k -ac 2> -movflags +faststart OUT.mp4`. Copy audio only if its codec is playable in MP4 (aac, mp3, plus opus/flac when caps allow); AC-3, E-AC-3, DTS, TrueHD, PCM, Vorbis and others are converted to AAC.
- **Transcode:** the same mapping with a video encoder.
  - macOS: `-c:v h264_videotoolbox -b:v {1080p: 8M, 720p: 5M, ≤480p: 2.5M, 4K: 20M} -pix_fmt yuv420p` (10-bit sources are converted to 8-bit). Add `-vf yadif` when `field_order` is not progressive.
  - Windows: try `h264_nvenc` → `h264_qsv` → `h264_amf` → `h264_mf` (pick the first encoder listed by `ffmpeg -hide_banner -encoders`; if the encoder fails to start, try the next), and `libx264 -preset veryfast -crf 20` if the build has it.
- Write to `playable-<order>.mp4.part`, then rename on success. Cache the result: if `playable-<order>.mp4` exists and `project.json` records the same plan and audio track, reuse it.
- **Runtime fallback:** if the UI reports a `<video>` `error` event (`MEDIA_ERR_SRC_NOT_SUPPORTED` or `MEDIA_ERR_DECODE`) for a Direct plan, escalate to Remux, then to Transcode (`report_playback_error` command).
- While a remux or transcode runs, the UI shows "Preparing video for playback… 37 %". **Audio extraction and ASR run in parallel on the original file**, so subtitles are already waiting when playback becomes possible.

#### 5.2.5 Audio extraction (for ASR)
`ffmpeg -i IN -map 0:a:<order> -vn -sn -dn [AF] -ac 1 -ar 16000 -f s16le -acodec pcm_s16le pipe:1` with `-progress pipe:2`.
- **Dialogue focus** (setting, default on): if `channel_layout` contains a front-center (FC) channel (5.1, 7.1…), use `-af "pan=mono|c0=FC"` instead of `-ac 1`. Movie dialogue lives in the center channel, so this greatly improves ASR in films.
- A reader thread writes stdout to `track-<order>/audio.pcm` (the final name is used from the start; `"complete": true` in `audio.json` marks completion, which avoids Windows rename-while-open problems) and forwards samples to the VAD in 512-sample frames (as f32, divided by 32768).
- Progress: `extracted_until_ms` = samples written / 16.
- Expected speed: hundreds of times real time for typical AAC/AC-3 audio. A 2-hour film takes roughly 10–40 s.

#### 5.2.6 PCM range reads
`read_pcm(path, start_sample, end_sample) -> Vec<f32>` (s16le → f32/32768). The worker uses this too, so put it in `hlp-core` and share it.

#### 5.2.7 Media server
- `axum` on `127.0.0.1:0`, started once per app run.
- `register(path) -> url` creates a random 128-bit token, maps it to the canonical path, and returns `http://127.0.0.1:<port>/m/<token>`.
- `GET`/`HEAD /m/{token}` uses `tower_http::services::ServeFile`, which supports Range and If-Range. Set the content type from the extension: mp4/m4v `video/mp4`, mov `video/quicktime`, webm `video/webm`, m4a `audio/mp4`, mp3 `audio/mpeg`.
- Unknown token: 404. No directory listing. Tokens are never persisted.
- `localhost` HTTP is a potentially trustworthy origin, so neither WKWebView nor WebView2 blocks it as mixed content.

#### 5.2.8 Downloader (yt-dlp)
- **Tools:** the yt-dlp standalone binary (`yt-dlp_macos` / `yt-dlp.exe`, Unlicense) and **Deno**. Since yt-dlp 2025.11.12, YouTube needs an external JS runtime: Deno is enabled by default, and the official yt-dlp executables already bundle the `yt-dlp-ejs` component. Install both lazily the first time the user opens a URL (pinned in the manifest, verified against the release's `SHA2-256SUMS` and Deno's `.sha256sum`).
- **Probe:** `yt-dlp --js-runtimes deno:<deno> --ffmpeg-location <ffmpeg_dir> --no-playlist --dump-single-json --no-warnings <url>` → `UrlInfo{id, title, duration, extractor, thumbnail, is_live}`. Reject `is_live`.
- **Download:**
  ```
  yt-dlp --js-runtimes deno:<deno> --ffmpeg-location <ffmpeg_dir> --no-playlist --newline --no-colors
         -f "bv*[vcodec^=avc1][height<=1080]+ba[acodec^=mp4a]/b[ext=mp4][height<=1080]/bv*[height<=1080]+ba/b"
         --merge-output-format mp4
         -o "<download_dir>/%(title).80B [%(id)s].%(ext)s"
         --progress-template "download:HLPPROG %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s %(progress.eta)s"
         --print after_move:filepath  <url>
  ```
  - Parse `HLPPROG` lines (values may be `NA`). Video and audio download as separate parts, so show "part 1/2".
  - The last stdout line that is an existing path is the result. Then open it with F1 and record `source_url` in `project.json`.
  - The format string prefers H.264 + AAC in MP4, so downloads usually play directly.
- **Cancel:** kill the process tree and delete `*.part` / `*.ytdl` files for that id.
- **Settings → "Update yt-dlp":** fetch the GitHub `releases/latest` metadata, download the binary for the platform, verify it against that release's `SHA2-256SUMS`, and install it as a new version directory. This is the only "latest" download allowed.
- **Notice:** show a one-time notice: "Only download videos you have the right to download."

---

### 5.3 `hlp-models`: components, paths, setup

Component manifest (`manifest.rs`). Each entry has: `id`, `kind` (Model/Tool), platforms, `required_for` (Subtitles/Urls), install path, readiness check, and source (URL + size + sha256, or an HF repo + file list).

| id | Platform | Required for | Install path | Source |
| --- | --- | --- | --- | --- |
| `asr-qwen3-1.7b-mlx` | macOS | Subtitles (default) | `~/.OminiX/models/qwen3-asr-1.7b` | HF `mlx-community/Qwen3-ASR-1.7B-8bit` (file list and completion marker from the translator; ModelScope fallback) |
| `asr-qwen3-0.6b-mlx` | macOS | optional ("Light" ASR setting, for 8 GB Macs) | `~/.OminiX/models/qwen3-asr-0.6b` | HF `mlx-community/Qwen3-ASR-0.6B-8bit` (≈1.01 GB; `VERIFY` the file list) |
| `asr-qwen3-0.6b-sherpa` | Windows | Subtitles | `%USERPROFILE%\.OminiX\models\sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25` | `https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25.tar.bz2` (≈1.9 GB unpacked: `conv_frontend.onnx`, `encoder.int8.onnx`, `decoder.int8.onnx`, `tokenizer/`), `TBD-SHA` |
| `vad-silero` | both | Subtitles | `~/.OminiX/models/silero-vad/silero_vad.onnx` | `https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx` (≈629 KB), `TBD-SHA` |
| `mt-hy-mt2-1.8b` | both | Subtitles (translation) | `~/.OminiX/models/Hy-MT2-1.8B-Q4_K_M.gguf` | copied `GgufSource` (section 2.1) |
| `llama-runtime-b11429` | both | Subtitles (translation) | `~/.OminiX/llama.cpp/b11429/` | copied asset names and hashes (section 2.1) |
| `ffmpeg` | both | Everything | `~/.OminiX/tools/ffmpeg/<ver>/{ffmpeg,ffprobe}` | macOS arm64: a pinned static build **with VideoToolbox**, version ≥ 7.1 (candidates: osxexperts.net arm64 builds, ffmpeg.martin-riedl.de release builds; record the exact URL and `TBD-SHA`; note the GPL/LGPL status in `docs/LICENSES.md`). Windows x64: BtbN FFmpeg-Builds **dated autobuild** tag, `win64-lgpl` variant zip, `TBD-SHA`. |
| `yt-dlp` | both | URLs | `~/.OminiX/tools/yt-dlp/<ver>/` | yt-dlp GitHub release (latest stable at implementation time), `yt-dlp_macos` / `yt-dlp.exe`, sha from `SHA2-256SUMS` |
| `deno` | both | URLs | `~/.OminiX/tools/deno/<ver>/` | denoland/deno release `deno-aarch64-apple-darwin.zip` / `deno-x86_64-pc-windows-msvc.zip` (≥ 2.3), `TBD-SHA` |

Behavior:
- `status() -> Vec<ComponentStatus{id, ready, bytes_total, bytes_done, state: Missing|Downloading|Verifying|Ready|Failed(msg)}>`. Readiness checks:
  - MLX ASR: `config.json`, `tokenizer_config.json`, `vocab.json`, `merges.txt` and the safetensors are present; **accept** the translator's completion marker or verify file sizes.
  - GGUF: size, magic and sha sidecar (copied logic).
  - Tools: the binary exists and `--version` exits with 0.
- `download(ids, progress_cb, cancel_token)`: resumable `.part` with HTTP `Range`, SHA-256 verification, atomic rename, ≤4 attempts with backoff, HF → ModelScope fallback for HF repos (copied logic), a lock file so two processes don't download the same component. Extract archives: `.tar.gz` (flate2 + tar), `.tar.bz2` (bzip2 + tar), `.zip` (zip; preserve the executable bit on macOS).
- On macOS, remove `com.apple.quarantine` from extracted tool binaries if present (`xattr -d`). Ignore failures.
- `HLP_OMINIX_ROOT` replaces `~/.OminiX` in every model and tool path. Tests use it to download into a temporary root without touching the real models.
- Exposed to the UI by `get_setup_status`, `start_setup` and `cancel_setup`, and to the CLI by `hlp-cli setup`.

---

### 5.4 `vendor/qwen3-asr-mlx`: patch for automatic language detection
1. Copy `~/.cargo/git/checkouts/ominix-mlx-4c1b6923576390a4/6aac996/qwen3-asr-mlx/{Cargo.toml,README.md,src/}` (or fetch the same rev from GitHub) to `vendor/qwen3-asr-mlx/`. Keep the MIT/Apache license notice and add `PATCHES.md` describing the changes.
2. In `Cargo.toml`, replace the path deps with git deps pinned to rev `6aac996db8b71fb7dae7a2409c46b4f2ade93092`, the way the translator's TTS patch does it. **Keep** `mlx-rs` features `["metal", "accelerate"]`. Drop the example.
3. In `src/model.rs`:
   - Change `fn build_prompt(&self, num_audio_tokens: i32, language: &str)` to take `language: Option<&str>`:
     - `Some(l)`: unchanged, `…<|im_start|>assistant\nlanguage {l}<asr_text>`.
     - `None`: the prompt **ends at** `…<|im_start|>assistant\n`, and the model generates `language X<asr_text>text` itself (the official `qwen_asr` behaviour for `language=None`).
   - Add the public API:
     ```rust
     pub struct AsrOutput { pub raw: String }   // parsing happens in hlp-core::asr_text
     pub fn transcribe_raw(&mut self, samples: &[f32], language: Option<&str>, config: &SamplingConfig) -> Result<AsrOutput>
     ```
     `transcribe_samples_with_config` keeps its current behavior (it calls `build_prompt(.., Some(lang))`).
   - Confirm that `generate()` stops on the EOS ids from `generation_config.json` (`<|im_end|>`, `<|endoftext|>`).
4. Tests (run on macOS, `#[ignore]` unless the model is present):
   - The tokenizer round-trip `"language English<asr_text>Hi"` → encode → decode(skip_special=true) gives back the same string.
   - Auto mode on the Japanese fixture utterance yields `language Japanese<asr_text>…`.
5. The library contains no `println!` (verified; only a doc comment). Its `eprintln!` load logs go to stderr, which the worker drains.

---

### 5.5 `hlp-asr-worker`: sidecar ASR server

**Launch:** `hlp-asr-worker --backend <qwen3-mlx|sherpa-qwen3> --model-dir <dir>` (or the `HLP_ASR_*` env vars). Resolve the binary through env `HLP_ASR_WORKER`, otherwise as a sibling of `current_exe()` (this covers `tauri dev`, the CLI and the bundled app, since Tauri places `externalBin` sidecars next to the main executable without the triple suffix).

**Protocol:** JSON Lines; one request per line on stdin, one response per line on stdout. Logs go to stderr only.
```jsonc
// requests
{"id":1,"op":"hello"}
{"id":2,"op":"load"}                                     // loads model (idempotent); may take seconds
{"id":3,"op":"transcribe","pcmPath":"/…/audio.pcm","startSample":123456,"endSample":234567,
 "language":null,                                        // null = auto-detect; or Qwen3-ASR name e.g. "Japanese"
 "maxTokens":400}
{"id":4,"op":"unload"}                                   // free memory (mlx_clear_cache), keep process
{"id":5,"op":"shutdown"}
// responses
{"id":1,"ok":true,"result":{"protocol":1,"backend":"qwen3-mlx","platform":"macos","canForceLanguage":true,"reportsLanguage":true}}
{"id":2,"ok":true,"result":{"loadMs":4120}}
{"id":3,"ok":true,"result":{"raw":"language Japanese<asr_text>…","elapsedMs":182,"audioMs":5230}}
{"id":3,"ok":false,"error":"…"}
```
Worker rules:
- Single-threaded and processes requests in order.
- Read the PCM range with 100 ms padding before and 150 ms after (clamped to the file); `sanitize_audio_samples`; skip anything under 1,600 samples (return `raw:""`).
- `maxTokens` is computed by the client as `min(1024, 48 + 20 * ceil(audio_s))`.
- After every 200 requests, call `mlx_clear_cache()` (macOS).
- Panic safety: wrap each request in `catch_unwind`, so a panic is reported as an error and the worker keeps running.
- If stdin reaches EOF, exit.

**macOS backend:** the vendored `qwen3_asr_mlx::Qwen3ASR`, `transcribe_raw`, temperature 0.

**Windows backend:** sherpa-onnx 2.0.3 `OfflineRecognizer`.
- `OfflineRecognizerConfig::default()`, then `config.model_config.<qwen3 field> = OfflineQwen3ASRModelConfig { conv_frontend, encoder, decoder, tokenizer: Some(<dir>/tokenizer), max_new_tokens, temperature: 0.0, ..Default::default() }`, with `num_threads = min(8, available_parallelism − 2)` and `provider "cpu"`.
- **`VERIFY`** the exact `OfflineModelConfig` field name (likely `qwen3_asr`) against `docs.rs/sherpa-onnx/2.0.3` and `github.com/k2-fsa/sherpa-onnx/blob/master/rust-api-examples/examples/qwen3_asr.rs`.
- This config has **no language field and no timestamps** (auto only), so `hello` reports `canForceLanguage:false`.
- If the output text contains `language X<asr_text>`, pass it through `raw` unchanged; otherwise return plain text. The pipeline then runs LID with `whatlang` (section 5.6.3).

**`AsrClient`** (in `hlp-pipeline`):
- Spawns the worker, sends `hello` and then `load`, and keeps one request in flight with a 120 s timeout.
- On crash or timeout, restarts with backoff, at most 3 times per 10 minutes, then surfaces `AsrUnavailable` to the UI.
- Forwards the worker's stderr to the log at debug level.

---

### 5.6 `hlp-pipeline`: Session orchestrator

#### 5.6.1 Session lifecycle
`Session::open(input_path, settings, caps, events: Arc<dyn PipelineEvents>, shared: SharedEngines) -> SessionHandle`:
1. Fingerprint, then load or create `cache/<fp>/project.json`. Emit `media_opened`, including any cached cues (built from `transcript.json` and `translations/<tgt>.json`).
2. `ffprobe`, then `decide()` the playback plan. If the plan is Direct, register the original file with the media server right away. Otherwise start PlaybackPrep and register `playable-<order>.mp4` when it's ready.
3. If `audio.json` is missing or not `complete`, start audio extraction and streaming VAD. Otherwise run VAD from `audio.pcm` when `segments.json` is missing or its `vadParamsHash` differs; otherwise load `segments.json`.
4. Run the scheduling loop until shutdown: pick ASR work, pick translation work, apply results, rebuild the affected cues, emit cue upserts (batched every 250 ms), and persist (debounced 2 s, atomic writes).
5. On shutdown, flush state and cancel child processes (ffmpeg); the shared engines stay alive.

`SharedEngines { asr: AsrClient, translator: Translator }` is created once by the app or CLI and pre-warmed.

#### 5.6.2 VAD and utterance grouping
- sherpa-onnx `VoiceActivityDetector::create(&VadModelConfig{ silero_vad: SileroVadModelConfig{ model: Some(path), threshold: 0.5, min_silence_duration: 0.25, min_speech_duration: 0.25, max_speech_duration: 15.0, window_size: 512 }, sample_rate: 16000, num_threads: 1, provider: Some("cpu"), debug: false, ..Default::default() }, 60.0)`.
- Feed frames of 512 samples with `accept_waveform`. Pop segments with `front()` / `pop()` while `!is_empty()`, and call `flush()` at the end of input.
- Region = `[seg.start(), seg.start() + seg.n())` in samples, converted to ms. (`start()` is relative to all input seen so far, so create one detector per extraction run and never call `reset()`.)
- **Grouping into utterances:** merge consecutive regions while `gap ≤ 500 ms` and the merged span is `≤ 12 s`. Drop an utterance whose speech totals under 300 ms if it is isolated (gap > 1 s on both sides); otherwise merge it into the nearest neighbor.
- Emit an utterance only when it is closed: the next region starts more than 500 ms later, the span limit is reached, or the input ends. Ids are `u{:06}` in emission order.
- `vadParamsHash` = hash of all VAD and grouping constants plus the model sha. If it changes, transcripts are invalidated.

#### 5.6.3 Language identification (LID)
Settings `source_language`: `"auto"` (default; detect, then lock), `"auto-per-line"` (multilingual content), or a specific code.

- **macOS (`canForceLanguage && reportsLanguage`):**
  1. **Probe phase:** transcribe in auto mode (`language: null`) in normal priority order until 6 utterances with at least 1.5 s of speech each have a language.
  2. **Vote**, weighted by speech duration. If the leader has at least 60 % of the weight, **lock** it and emit `language {detected, confidence, locked:true}`. Otherwise extend the probe to 12 utterances; if there is still no majority, switch to per-line mode and show "Mixed".
  3. **After lock:** all new ASR requests force the locked language. Probe utterances whose detected language differs from the lock are re-queued (marked `stale`) with the forced language.
- **Windows (sherpa):** ASR always runs in auto mode. Detect the language from text with `whatlang` (`detect(text)` with confidence of at least 0.5 and text of at least 10 columns), using the same weighted vote over the first 6 utterances. The lock affects only translation-template choice, the skip-translation rule and the UI badge.
- **Cross-check (both platforms):** if `whatlang` strongly disagrees with the ASR-reported language for an utterance (confidence ≥ 0.9 and a different language in the table), log it at debug level only. No action is taken in the prototype.
- **User override** (specific code): set the lock to that language, then mark every `Done` utterance whose `forced_language` differs as `stale` + `Pending`. Old text stays visible until it is replaced. On Windows, the override changes only the template, skip and badge.

#### 5.6.4 Translation
- `Translator` is a copy of `LlamaServer` from the translator: spawn lazily or pre-warm, health-check `/health`, and restart up to 3 times.
- Work unit = one `Done` utterance (its full text, which keeps sentence context). Request body: `{"messages":[{"role":"user","content":PROMPT}],"temperature":0,"repeat_penalty":1.05,"max_tokens":384,"stream":false,"cache_prompt":true}`.
- Post-process with the copied `strip_think_block` and `sanitize_translation_output`; also strip wrapping quotes the source didn't have and leading labels (`Translation:`, `译文：`, `翻译：`). An empty result, or one identical to the source when source ≠ target, gets one retry with the English template; after that the utterance is marked failed and the UI shows the original only.
- Store results in `translations/<tgt>.json` as `items[utt_id] = {sourceHash, text}`, where `sourceHash = sha256(original text)[..16]`, so a re-transcribed utterance gets re-translated automatically. The header holds `translatorId: "hy-mt2-1.8b-q4km"` and `promptVersion`.
- Target `none`, or source == target: no translation work; cues have `translation: null`.

#### 5.6.5 Cache layout and persistence
Root: `dirs::data_dir()/com.henlocal.player/` (macOS `~/Library/Application Support/com.henlocal.player`, Windows `%APPDATA%\com.henlocal.player`). The CLI uses the same root.
```
settings.json
recent.json                         # [{fingerprint, path, title, durationMs, lastOpened, progress}] max 20
logs/henlocal-player.log            # tracing-appender, daily rotation, keep 7
media/<fp>/project.json             # {schemaVersion:1, fingerprint, sourcePath, sourceUrl?, title, durationMs,
                                    #  probe summary, audioTrackOrder, playback:{plan, playablePath?},
                                    #  language:{mode, detected?, confidence?, locked?}, createdAt, updatedAt}
media/<fp>/playable-<order>.mp4      # only for Remux/Transcode plans (per selected audio track)
media/<fp>/track-<order>/audio.pcm  # 16 kHz mono s16le (≈115 MB per hour)
media/<fp>/track-<order>/audio.json # {sampleRate:16000, totalSamples, complete, dialogueFocus}
media/<fp>/track-<order>/segments.json            # {vadParamsHash, complete, utterances:[{id,startMs,endMs,regions}]}
media/<fp>/track-<order>/transcript.json          # {asrModel, items:{utt_id:{status,text,language,forcedLanguage,attempts}}}
media/<fp>/track-<order>/translations/<tgt>.json  # {translatorId, promptVersion, items:{utt_id:{sourceHash,text}}}
```
- `<order>` is the audio-relative stream index (`0:a:<order>`). Everything derived from audio lives under its track folder, so switching tracks never mixes data and switching back is instant.
- Atomic JSON writes (`*.tmp`, then rename). If any JSON is corrupt, treat it as missing and log a warning.
- Cache limit (setting, default 30 GB): when exceeded on open, evict the least recently opened media folders other than the current one, deleting `playable-*.mp4` before whole folders.
- Privacy: transcripts never go to logs above debug level; log lengths and timings only.

#### 5.6.6 Events (trait `PipelineEvents`, implemented by the Tauri bridge and the CLI printer)
```rust
fn media_opened(&self, e: MediaOpened);      // {sessionId, title, durationMs, mediaUrl: Option<String>, plan, audioTracks, language, target, cues: Vec<Cue>}
fn playback_prep(&self, e: PlaybackPrep);    // {sessionId, plan, progress: f32, mediaUrl: Option<String>, error: Option<String>}
fn cues(&self, e: CueBatch);                 // {sessionId, upserts: Vec<Cue>, removedIds: Vec<String>}   (≤4 Hz)
fn status(&self, e: PipelineStatus);         // {sessionId, extractedUntilMs, vadComplete, uttTotal, uttDone, uttNoSpeech, uttFailed,
                                             //  translatedCount, asrPendingSpans: Vec<[Ms;2]>, trPendingSpans: Vec<[Ms;2]>,
                                             //  asrRtf: f32, msPerTranslation: f32, etaSeconds: Option<u32>, stage}   (≤2 Hz)
fn language(&self, e: LanguageEvent);        // {sessionId, mode, detected, confidence, locked, unverifiedTranslation: bool}
fn error(&self, e: ErrorEvent);              // {sessionId?, code, message, recoverable}
```
Pending spans are merged ranges, which lets the UI draw coverage cheaply.

---

### 5.7 `hlp-cli`
```
hlp-cli setup [--yes] [--with-urls]          # download missing components with progress bars
hlp-cli status                               # component readiness table
hlp-cli probe <input> [--webview wkwebview|webview2]   # MediaInfo + PlaybackPlan as JSON
hlp-cli subtitle <input|url> [--source auto|auto-per-line|<code>] [--to <code>|none]
                 [--formats srt,vtt,ass] [--out-dir DIR] [--no-cache] [--json-progress]
hlp-cli langs                                # ASR languages and translation targets
```
- `subtitle` runs a Session with the playhead fixed at 0 and `process_whole=true`, and waits until every utterance is final and all translations are done. It writes `<stem>.<src>.srt`, `<stem>.<tgt>.srt` and `<stem>.<src>-<tgt>.ass` (plus VTT if requested), then prints a summary: detected language and confidence, utterance counts, ASR real-time factor, mean translation ms, wall time, and output paths.
- Exit codes: 0 OK, 2 setup missing, 3 media error, 4 engine error.
- One file per subcommand under `src/cmd/` (`probe.rs`, `setup.rs`, `status.rs`, `subtitle.rs`, `langs.rs`); `main.rs` only parses arguments and dispatches.

---

### 5.8 Tauri app backend (`app/`)
- `tauri.conf.json`:
  - `productName "Hen Local Player"`, `identifier "com.henlocal.player"`, window `main` 1280×760 (min 900×560), `title "Hen Local Player"`, `backgroundColor "#000000"`, `dragDropEnabled: true`.
  - Put `bundle.externalBin: ["binaries/hlp-asr-worker"]` in a separate `tauri.bundle.conf.json` used only for packaging (`tauri build --config tauri.bundle.conf.json`, as the translator does). `tauri-build` fails at compile time when a listed sidecar file is missing, so keeping it out of the main config lets `cargo check -p henlocal-player` work without building the worker. The packaging script copies the built worker to `app/binaries/hlp-asr-worker-<target-triple>` (Tauri's sidecar naming). In development, the worker is found next to the app binary in the same `CARGO_TARGET_DIR`.
  - macOS `minimumSystemVersion "14.0"`.
  - `bundle.fileAssociations` for mp4, mkv, mov, m4v, webm, avi, ts, flv, wmv, mp3, m4a, wav, flac, ogg (stretch goal; also handle `RunEvent::Opened { urls }` on macOS and argv on Windows).
  - CSP: `default-src 'self'; media-src 'self' http://127.0.0.1:*; img-src 'self' data: blob: http://127.0.0.1:* https:; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost`.
- Plugins: `tauri-plugin-dialog` (open and save dialogs) and `tauri-plugin-opener` (reveal in Finder or Explorer).
- **Commands**, all `async` and returning `Result<T, String>`:
  `set_playback_capabilities(caps)`, `open_media(path)`, `probe_url(url)`, `download_url(url)`, `cancel_download()`, `set_playhead(sessionId, ms, playing)`, `set_target_language(code)`, `set_source_language(mode_or_code)`, `select_audio_track(order)`, `report_playback_error(sessionId, code)`, `retry_failed()`, `export_subtitles({variant, format, path})`, `suggest_export_paths()`, `get_settings()`, `update_settings(patch)`, `get_setup_status()`, `start_setup({includeUrlTools})`, `cancel_setup()`, `list_recent()`, `clear_cache()`, `reveal_in_folder(path)`, `app_info()`.
- **Events:** `media://opened`, `media://prep`, `pipeline://cues`, `pipeline://status`, `pipeline://language`, `setup://progress`, `download://progress`, `app://error`.
- **Pre-warm** (`prewarm.rs`): on launch, if the models are ready, start the ASR worker (`hello` + `load`) and llama-server in the background, and send one tiny warm-up translation.
- Logging: `tracing` to the file and stderr; panics are logged.

---

### 5.9 UI (`app/ui`, Svelte 5 runes + TypeScript, plain CSS)

#### 5.9.1 Layout (dark theme)
```
┌ Top bar ───────────────────────────────────────────────────────────────────────────────┐
│ [Open] [Link]  Title of the video            Spoken: Auto · Japanese 94% ▾   Subtitles in: 中文 ▾ │
│                                              [Orig|Trans|Both|Off]  [Export ▾]  [⚙]           │
├───────────────────────────────────────────────────────────┬────────────────────────────┤
│                                                           │ Transcript         [×]     │
│                    VIDEO STAGE                            │ Audio 100% · ASR 42% · …   │
│            (drop zone when empty)                         │ 00:12  こんにちは…           │
│                                                           │        你好……               │
│         subtitle overlay (inside video content box)       │ 00:15 ▶ 現在の行 (highlight) │
│ ┌ controls (auto-hide) ─────────────────────────────────┐ │ 00:19  …transcribing        │
│ │ ▶ 01:02:03 / 01:45:00 ═══coverage+seek═══ 🔊 1.0x A- A+ ⛶│ │                            │
│ └───────────────────────────────────────────────────────┘ │                            │
└───────────────────────────────────────────────────────────┴────────────────────────────┘
```

#### 5.9.2 Components (`src/lib/`)
- `backend.ts`: the `Backend` interface and `TauriBackend` (invoke/listen). `mock.ts`: **`MockBackend`**, which plays `public/demo/demo.mp4` and streams canned cues over time to simulate processing. It is selected when `!('__TAURI_INTERNALS__' in window)`, so `npm run dev` works in a normal browser for UI development and screenshots.
- `stores.svelte.ts`: session state, the sorted `cues` array plus an `id → index` map, status, settings, setup status.
- `Player.svelte`: `<video preload="auto" playsinline>` with no native controls and `object-fit: contain`. It reports the playhead to the backend at ≤4 Hz while playing and immediately on `seeked`, `pause` and `play`, plus `error` events through `report_playback_error`.
- `SubtitleOverlay.svelte`:
  - On `requestAnimationFrame` while playing (and on `seeked`/`timeupdate` when paused), binary-search for the last cue with `start ≤ t` and show it if `t < end`.
  - Position the overlay inside the **video content rectangle**, computed from `videoWidth/videoHeight` and the element box (letterbox-aware). The bottom offset is a setting (default 6 % of the content height).
  - Font size is 4.2 % of the content height for the primary line and 3.4 % for the secondary line, times the user's scale. White text with a black outline (`paint-order: stroke` + `-webkit-text-stroke`, or a multi-`text-shadow`) and an optional `rgba(0,0,0,α)` box. `max-width: 90 %`, `text-wrap: balance`.
  - Bilingual order follows the setting (default: translation on top, then the original in smaller light grey).
  - If the playhead is inside an `asrPendingSpan`, show a small "Transcribing…" chip in the top right of the video (not in the subtitle area).
- `Controls.svelte`: play/pause, time display, a seek bar with layers (track, extracted range, ASR-pending holes, translation-pending holes, buffered, played) and hover-time tooltip, volume, speed (0.5–2.0), subtitle size −/+, and fullscreen. Auto-hide after 2.5 s idle while playing; hide the cursor too in fullscreen.
- `TranscriptPanel.svelte`:
  - A virtualized list if there are more than 500 rows (simple windowing).
  - Each row shows the time, original text and translation; failed or pending rows get a status chip.
  - The current row is highlighted and auto-scrolled; auto-scroll pauses for 5 s after the user scrolls manually. Clicking a row seeks to `cue.start - 100 ms`.
  - The header shows the pipeline status line, for example "Audio 100% · Transcribed 42% · Translated 38% · ASR 24× realtime".
- `LanguageControls.svelte`:
  - The spoken-language dropdown offers Auto (with the detected name and %), Auto per line, then the 30 ASR languages.
  - The target dropdown offers None plus the 38 targets, each labeled in the UI language with the native name.
  - Show the "quality unverified" badge when it applies.
- `OpenUrlDialog.svelte`: paste URL → `probe_url` → a card with title, duration and thumbnail → Download → progress (part 1/2, %, ETA) → opens automatically. Errors show yt-dlp's last error line plus an "Update yt-dlp" hint.
- `ExportMenu.svelte`:
  - Variants: Original / Translation / Bilingual; formats: SRT / VTT / ASS.
  - The save dialog defaults to the video's folder with the default filename. "Export all next to video" is a one-click action.
  - If processing is incomplete, ask "Only 42 % processed. Export now, or finish first?" ("Finish first" sets `process_whole` and exports when done).
- `SetupScreen.svelte`: the component table with sizes and states, "Download all (X GB)", per-component retry, and cancel. Playback stays usable; subtitles show "Needs setup".
- `SettingsDialog.svelte`: the section 6 settings plus model status, "Update yt-dlp", cache size, Clear cache, and the log folder.
- `RecentList.svelte` on the empty stage: thumbnails are optional; show the title, duration and processed %.
- **Drag and drop:** `getCurrentWebview().onDragDropEvent` → take the first path → `open_media`.

#### 5.9.3 Fullscreen
**Do not use** the HTML Element Fullscreen API: WKWebView has element fullscreen disabled by default. Use `getCurrentWindow().setFullscreen(true)` and a `.fullscreen` CSS class that hides the top bar and transcript panel and makes the stage fill the viewport. `Esc` exits.

#### 5.9.4 Keyboard shortcuts
`Space` play/pause · `←/→` −/+5 s · `Shift+←/→` −/+30 s · `↑/↓` volume · `,` / `.` previous/next cue (seek to its start) · `F` fullscreen · `S` cycle subtitle mode · `T` toggle the transcript panel · `[` / `]` speed −/+ · `Cmd/Ctrl+O` open file · `Cmd/Ctrl+L` open link · `Cmd/Ctrl+E` export · `Esc` exits fullscreen or closes a dialog.

#### 5.9.5 i18n
Strings live in `src/lib/i18n/{en.ts, zh.ts}`. The UI language follows `navigator.language` (zh* → zh, otherwise en) and can be overridden in Settings. Language names come from the core table (English and Chinese names).

---

## 6. Settings (`settings.json`, with defaults)
```jsonc
{
  "uiLanguage": "system",                 // system | en | zh
  "targetLanguage": "<from locale>",      // code | "none"
  "sourceLanguage": "auto",               // auto | auto-per-line | <code>
  "subtitleMode": "both",                 // original | translation | both | off
  "bilingualOrder": "translation-first",
  "subtitleScale": 1.0,                   // 0.6–2.0
  "subtitleBackground": 0.0,              // box opacity 0–0.8 (outline always on)
  "subtitleBottomOffset": 0.06,
  "showTranscriptPanel": true,
  "lookaheadSeconds": 180,
  "processWholeFile": true,
  "dialogueChannelFocus": true,
  "asrModel": "qwen3-asr-1.7b",           // macOS: qwen3-asr-1.7b | qwen3-asr-0.6b; Windows: fixed sherpa-qwen3-asr-0.6b
  "translationContext": false,            // experimental background-context prompt
  "downloadDir": "<Videos>/Hen Local Player",
  "cacheLimitGb": 30
}
```
Changing `asrModel` restarts the worker and invalidates transcripts made with the other model (kept on disk; re-queued only for the open media).

---

## 7. Performance targets (M4 Pro, warm engines)
| Metric | Target |
| --- | --- |
| Open local Direct-play MP4 → first subtitle shown | ≤ 5 s warm, ≤ 15 s cold (model load) |
| Seek to an unprocessed position → subtitle there | ≤ 5 s (after audio extraction has passed that point) |
| ASR speed (1.7B, utterance-sized inputs) | ≥ 15× real time (README: ~30× on 30 s chunks, M4 Max) |
| Translation per utterance (Hy-MT2 Q4_K_M, Metal) | ≤ 400 ms median (translator measured 182 ms on short sentences) |
| 2-hour movie, fully processed | ≤ 15 min |
| Overlay | 60 fps, no jank during cue updates |
| Windows (CPU only) | ASR ≥ 2× real time with the 0.6B int8 model (**measure**; if lower, document it and reduce lookahead expectations) |

Log the real-time factor and timings for every session in `status` and in the CLI summary.

---

## 8. Testing and verification

### 8.1 Unit tests (no models; run in CI on macOS and Windows)
- `hlp-core`:
  - Language table integrity (30 ASR, 38 targets, unique codes) and default target from locale.
  - Prompt selection and verbatim templates.
  - ASR output parsing (auto, forced, `language None`, no tag, multi-line meta) and repetition collapse.
  - Cue builder: Latin, CJK, Thai graphemes, Arabic; timing over pauses; min/max duration; linger; **proptest no-overlap invariant**.
  - Translation distribution.
  - SRT, VTT and ASS golden files and timestamp formatting (≥1 h, rounding).
  - Scheduler priority ordering.
  - Fingerprint.
  - PCM read conversion.
- `hlp-media`:
  - `decide()` table tests from stored ffprobe JSON fixtures (mp4 h264/aac, mkv h264/ac3, mkv hevc hev1, webm vp9/opus, avi mpeg4/mp3, mov prores, audio-only flac).
  - Progress-line parser and yt-dlp `HLPPROG` parser.
  - Media server Range responses (start a server on a temp file and request `bytes=10-19`).
- `hlp-models`: manifest sanity (no "latest" URLs, every sha 64 hex characters except the documented update flow), and resume logic against a local test HTTP server.

### 8.2 Fixtures (`scripts/make_test_media.sh`, macOS)
- **Voices:** generate speech with the built-in `say` voices. List them with `say -v '?'` and pick an available voice per language: ja (Kyoko/Otoya), fr (Thomas/Amélie), de (Anna), es (Mónica/Paulina), zh (Tingting), ko (Yuna), en (Samantha).
- **Script:** each script has 6 known sentences. Concatenate them with ffmpeg using known silences (1.5 s gaps via `anullsrc`), and write the expected onset times to `tests/fixtures/<name>.truth.json` (sentence text, onset ms, language).
- **Video:** `-f lavfi -i testsrc2=size=1280x720:rate=30`.

Outputs:

| File | Video / audio | Exercises |
| --- | --- | --- |
| `fixture_ja.mp4` | h264 + aac | Direct play |
| `fixture_fr.mkv` | h264 + ac3 | Remux plus audio conversion |
| `fixture_de.webm` | vp9 + opus | Direct play (WebM) |
| `fixture_es.avi` | mpeg4 + mp3 | Transcode |
| `fixture_zh.mov` | hevc (`hvc1`, via `hevc_videotoolbox`) + aac | Direct play |
| `fixture_ko_51.mkv` | h264 + 6-channel AAC, speech only in FC, noise in the other channels | Dialogue focus |
| `fixture_music.mp4` | Tone and noise only | Expect zero cues |
| `fixture_en.m4a` | Audio only | Audio-only path |
| `fixture_long_en.mp4` | 20-minute English speech loop | Seek and priority |

- `ui/public/demo/demo.mp4` is a copy of `fixture_ja.mp4` for mock mode.
- Windows: `scripts/make_test_media.ps1` uses SAPI `System.Speech` with the installed voices (at least English) to build the same container matrix. Copy the WAV generation from the translator's `scripts/make_e2e_wavs.ps1`.

### 8.3 E2E (`scripts/e2e.sh` / `.ps1`; needs setup; also available as `cargo test -p hlp-pipeline -- --ignored`)
For each fixture, run `hlp-cli subtitle <f> --to zh --formats srt,vtt,ass --json-progress` and assert:
1. The detected language equals the truth (macOS: every fixture; Windows: those with installed voices).
2. Normalized character error rate against the truth text is ≤ 15 % (`say` speech is clean).
3. Every truth sentence's onset has a cue starting within ±600 ms.
4. No overlapping cues; every cue's `start < end`.
5. Translations are non-empty and differ from the source (except zh→zh, which is skipped).
6. `ffmpeg -v error -i out.srt -f null -` (and the same for `.vtt` and `.ass`) prints no errors.
7. `fixture_music.mp4` produces 0 cues.
8. A second run hits the cache and finishes in under 2 s.
9. `hlp-cli probe` plans match: ja → Direct, fr → Remux, es → Transcode, de → Direct (WKWebView), zh → Direct.

Print the real-time factor and timings, and save the summary to `tests/results/<date>-<platform>.json`.

### 8.4 Manual UI checklist (record the results in `docs/PROGRESS.md`)
- [ ] Empty state shows the drop zone and the recent list. Dropping a file plays it within 1 s (Direct).
- [ ] The language badge shows "Japanese 9x %" within 15 s, and subtitles appear and stay in sync during playback.
- [ ] Original / Translation / Both / Off switch instantly. Changing the target to English shows "translating…" coverage, then English text.
- [ ] Seeking to 15:00 in `fixture_long_en.mp4` shows subtitles there within 5 s, and the coverage bar shows processing has moved there.
- [ ] `fixture_fr.mkv` shows "Preparing video…" progress, then plays with subtitles already available.
- [ ] Fullscreen keeps the subtitles and controls; `Esc` exits; subtitle size scales.
- [ ] Clicking a transcript row seeks; `,` and `.` jump between lines; auto-scroll follows.
- [ ] Export writes SRT, VTT and ASS next to the video, and they load in QuickTime/IINA/VLC (any player available).
- [ ] Overriding the spoken language to the wrong one and back re-transcribes (stale text is replaced).
- [ ] URL flow: paste a URL → card → download progress → plays → subtitles. Cancel mid-download cleans up the parts.
- [ ] Quitting and reopening the same file restores the subtitles instantly.
- [ ] Setup screen: rename `~/.OminiX/models/silero-vad`, restart, and the app asks for that one download only.
- [ ] Killing `hlp-asr-worker` mid-session: the app restarts it and processing continues.

---

## 9. Milestones (implement in this order)

| # | Milestone | Deliverables | Acceptance |
| --- | --- | --- | --- |
| **M0** | Scaffold | Repo tree (section 4), workspace, toolchain, `README.md`, `AGENTS.md`, `scripts/dev.sh`/`dev.ps1` with the no-space target-dir guard, empty Tauri + Svelte app, minimal CI (fmt, clippy and tests for pure crates on macOS and Windows; `npm run check`) | `cargo test -p hlp-core` and `npm --prefix app/ui run check` pass; `scripts/dev.sh` opens an empty "Hen Local Player" window |
| **M1** | Core logic | Everything in section 5.1 with unit tests | Section 8.1 `hlp-core` tests pass |
| **M2** | Media layer | Section 5.2 (except the downloader), `hlp-cli probe` | Decision table tests pass; probe output is correct for the fixtures; extracting a 10-minute file yields `duration × 16000 ± 0.1 %` samples; the Range test passes |
| **M3** | Models and setup | Section 5.3, `hlp-cli setup/status` | On the dev Mac, the existing ASR, Hy-MT2 and llama runtime show **Ready** without downloading; silero and ffmpeg download and verify; an interrupted download resumes |
| **M4** | ASR worker | Section 5.4 patch, section 5.5 worker (macOS), protocol tests | The worker transcribes fixture utterances; auto mode returns `language Japanese<asr_text>…` for ja; a forced language works; a deliberately killed worker is restarted by `AsrClient` |
| **M5** | Pipeline and CLI vertical slice | Section 5.6, section 5.7, exports | **All of section 8.3 passes on macOS** for the core fixtures (ja, fr, de, es, zh, music, long_en) |
| **M6** | App and UI | Sections 5.8 and 5.9, pre-warm, settings, setup screen, export, transcript, fullscreen, shortcuts, mock mode | Section 8.4 checklist (except the URL items) on macOS; screenshots of the main states saved to `docs/screenshots/` |
| **M7** | URL downloads | Section 5.2.8 plus the UI dialog, lazy yt-dlp and Deno setup, "Update yt-dlp" | A Wikimedia Commons or Internet Archive direct video URL and one YouTube URL (supplied by the user at test time) download, play and get subtitles |
| **M8** | Windows | Sherpa backend (section 5.5), Windows manifest entries, the `dev.ps1` flow, encoder fallback, WebView2 caps | Section 8.3 on Windows with SAPI fixtures; section 8.4 smoke test (open, play, subtitles, export, URL); speeds recorded |
| **M9** | Packaging (should-have) | macOS `.app`/`.dmg` (unsigned internal build) with the worker sidecar and the `mlx.metallib` layout from the translator's `build_macos_app.sh`; Windows NSIS installer via `tauri build` | The installed app runs the flows on a clean user account (models are downloaded through setup) |

Stop rule: if a milestone is blocked by something outside the repo (for example, a VERIFY item proves false), record it in `docs/PROGRESS.md` (in parallel mode, your lane's progress file) with the evidence and the chosen workaround, then continue.

### 9.1 Running several agents in parallel (optional)

The default is one agent working through M0–M9 in order, and it needs nothing from this section. If several coding agents are available, split the work into the five phases below. The time saved comes from phase 2; phases 1, 3 and 5 stay single-agent because they set or join the pieces everyone else builds on.

```
Phase 1  foundation + frozen contracts (1 agent)                → human merges, tag contracts-v1
Phase 2  lanes A | B | C | D | E in parallel (up to 5 agents)    → human merges in order A, C, B, D, E
Phase 3  integration = M5 (1 agent)                             → human merges, tag slice-v1
Phase 4  lane F (Mac app + URLs) | lane G (Windows PC)           → human merges F, then G
Phase 5  packaging = M9 (1 agent)
```

#### 9.1.1 Ground rules for every lane
- **One git worktree and one branch per lane**, created from the phase's starting tag, for example:
  `git worktree add "../Hen-Local-Player-lane-a" -b lane-a contracts-v1`
- **Use a separate `CARGO_TARGET_DIR` per worktree**, with no spaces: `$TMPDIR/hlp-target-<lane>` on macOS, `C:\hlp-target-<lane>` on Windows. A shared target directory makes parallel builds wait on Cargo's lock and rebuild each other's changes.
- **Edit only the paths your lane owns** (tables below). Everything else is read-only for you.
- **Contract files are frozen during phase 2** (9.1.2 lists them). If you need a contract change, don't make it. Write a "Contract change request" (what, why, which lanes it affects) at the top of your progress file, and continue with a local workaround such as a private adapter. The human or the phase 3 agent applies approved changes on `main`, and other lanes rebase.
- **No new dependencies in phase 2.** Phase 1 declares all of them. If one is truly missing, file a change request instead of editing `Cargo.toml` or `Cargo.lock`.
- **Build only your own crates** with `-p`, never `--workspace`. Only lane D builds `hlp-asr-worker`, and therefore MLX.
- **Don't modify `~/.OminiX`.** Other lanes read models from it. Lane C runs download tests under a temporary root via `HLP_OMINIX_ROOT`.
- **Only lane D runs model inference in phase 2**, so agents don't compete for the GPU. In phases 3 and 4, run model-heavy tests one at a time.
- **Progress files:** each lane writes only `docs/progress/<lane>.md` (acceptance checkboxes, evidence, change requests, blockers). The phase 1 and phase 3 agents own `docs/PROGRESS.md`, which links to the lane files.
- **Before handing back:** rebase on `main`, run your lane's tests, run `cargo check -p hlp-core -p hlp-media -p hlp-models -p hlp-pipeline -p hlp-cli -p henlocal-player` and `npm --prefix app/ui run check`, then stop for review. The human merges branches in the order shown above.

#### 9.1.2 Phase 1: foundation and contract freeze (1 agent, branch `foundation`)
Everything in M0, plus:
1. **Every dependency declared up front.** `[workspace.dependencies]` lists every external crate from section 4 with versions, and each crate's `Cargo.toml` already lists the dependencies it will need (`workspace = true`). Copy `vendor/qwen3-asr-mlx` **unpatched** (section 5.4, steps 1–2), wire it into `hlp-asr-worker`, and add `sherpa-onnx` to the crates that use it, so `Cargo.lock` is complete. Commit `Cargo.lock`.
2. **Contract files, complete and compiling.** Types, serde derives and doc comments must be final; logic bodies may be `todo!()` unless marked "implemented".
   - `crates/hlp-core/src/types.rs`: the section 5.1.1 types plus `MediaInfo`, `AudioStream`, `VideoStream`, `PlaybackPlan`, `Caps`, `UrlInfo`, `ComponentStatus`, `SessionCommand`.
   - `crates/hlp-core/src/asr_protocol.rs`: the section 5.5 requests and responses, **implemented**, with JSON round-trip tests (lane D and phase 3 both rely on it).
   - `crates/hlp-core/src/events.rs`: the `PipelineEvents` trait and every payload in section 5.6.6.
   - `crates/hlp-core/src/settings.rs`: the section 6 struct with defaults, **implemented**.
   - `crates/hlp-core/src/pcm.rs`: `read_pcm` (section 5.2.6), **implemented** and tested, because several lanes need it.
   - Public signatures with `todo!()` bodies in `hlp-core` (lang, prompts, asr_text, cues, subs, schedule, fingerprint), `hlp-media` (`Tools`, `probe`, `decide`, `extract_audio`, `prepare_playback`, `MediaServer`, `Downloader`), `hlp-models` (`status`, `download`, path functions) and `hlp-pipeline` (`Session::open`, `SessionHandle`, `SharedEngines`, `AsrClient`, `Translator`).
   - `app/ui/src/lib/types.ts`: TypeScript mirrors of every UI-facing payload, using the same camelCase field names as the Rust serde output. `app/ui/src/lib/backend.ts`: the `Backend` interface and a `TauriBackend` stub. `app/ui/src/lib/mock.ts`: a `MockBackend` skeleton.
   - `crates/hlp-cli/src/main.rs` with clap dispatch to one stub file per subcommand (`cmd/probe.rs`, `cmd/setup.rs`, `cmd/status.rs`, `cmd/subtitle.rs`, `cmd/langs.rs`), so no two lanes edit the same CLI file.
3. **Test media:** a working `scripts/make_test_media.sh` (section 8.2), including `app/ui/public/demo/demo.mp4`, so every lane has fixtures from the start.
4. **Docs:** `docs/ARCHITECTURE.md` summarizing the contracts, and `docs/progress/{a-core,b-media,c-models,d-asr,e-ui}.md`, each starting with its lane's "done when" checklist from 9.1.3.

Exit criteria: M0 acceptance passes; the `cargo check` command from 9.1.1 passes; `cargo build -p hlp-asr-worker` succeeds on macOS (this builds MLX once); the protocol, settings and PCM tests pass. Then merge to `main` and tag `contracts-v1`.

#### 9.1.3 Phase 2: parallel lanes (each branches from `contracts-v1`)

| Lane | Work | Owns (may edit) | Done when |
| --- | --- | --- | --- |
| **A: core** | M1: fill in every `hlp-core` body from section 5.1 | `crates/hlp-core/**`: may fill bodies and add private items, but may not change public signatures or the contract files | Section 8.1 `hlp-core` tests pass |
| **B: media** | M2, plus the downloader backend from section 5.2.8 (no UI) | `crates/hlp-media/**`, `crates/hlp-cli/src/cmd/probe.rs` | M2 acceptance passes, and the downloader unit tests (progress parser, cancel cleanup) pass. Until lane C's managed installs exist, use the `HLP_FFMPEG`, `HLP_FFPROBE`, `HLP_YTDLP` and `HLP_DENO` overrides |
| **C: models** | M3 | `crates/hlp-models/**`, `crates/hlp-cli/src/cmd/{setup,status}.rs` | M3 acceptance passes, with download tests under `HLP_OMINIX_ROOT=<temp dir>`; checks against the real `~/.OminiX` are read-only |
| **D: ASR** | M4: patch the vendored crate (section 5.4, steps 3–5), build the macOS worker and `AsrClient` | `vendor/qwen3-asr-mlx/**`, `crates/hlp-asr-worker/**`, `crates/hlp-pipeline/src/asr_client.rs` | M4 acceptance passes, using the models already in `~/.OminiX/models` |
| **E: UI** (optional) | The UI half of M6 against `MockBackend` only: every component in section 5.9, shortcuts, the fullscreen class, i18n | `app/ui/**` except `types.ts` and `backend.ts` | `npm run dev` in an ordinary browser shows the section 8.4 states with mock data; `npm run check` passes; screenshots are in `docs/screenshots/mock/` |

Merge order at the end of phase 2 is **A, C, B, D, E**, each rebased on the previous merge. A goes first because the other lanes' code starts calling real `hlp-core` logic once it lands.

#### 9.1.4 Phase 3: integration (1 agent, branch `integration`)
M5: the rest of `hlp-pipeline`, plus `cmd/subtitle.rs` and `cmd/langs.rs`. This agent may edit any crate to fix integration problems, applies any approved contract change requests, and owns `docs/PROGRESS.md`. Exit: section 8.3 passes on macOS. Then merge and tag `slice-v1`.

#### 9.1.5 Phase 4: two lanes (each branches from `slice-v1`)

| Lane | Work | Owns (may edit) | Done when |
| --- | --- | --- | --- |
| **F: app** (Mac) | The rest of M6: the Tauri backend (section 5.8), the real `TauriBackend`, pre-warm, setup screen and export wiring. Then M7: the URL dialog, lazy yt-dlp and Deno setup, and "Update yt-dlp" | `app/**`, `crates/hlp-media/src/download*.rs` | M6 and M7 acceptance pass |
| **G: Windows** (Windows PC) | The M8 engine work: the sherpa backend in the worker, Windows manifest entries, `dev.ps1`, `make_test_media.ps1`, `e2e.ps1`, the encoder fallback chain | `crates/hlp-asr-worker/src/backend/windows.rs`, Windows-only (`#[cfg(windows)]`) code in other crates, `scripts/*.ps1` | Section 8.3 passes on Windows. After F merges, rebase and run the section 8.4 smoke test on Windows |

Merge F first, then G. Keep G's fixes to shared code (for example `CREATE_NO_WINDOW` in `hlp-media`) inside `#[cfg(windows)]` blocks so they don't conflict with F.

#### 9.1.6 Phase 5: packaging (1 agent)
M9 on `main`, after F and G are merged.

#### 9.1.7 Kickoff message for each lane agent
```text
Read PLAN.md, especially sections 0 and 9.1. You are lane <X> (<name>) in phase <N>.
Work in the worktree <path> on branch <branch>, with CARGO_TARGET_DIR=<dir>.
Edit only the paths your lane owns. Don't change contract files or dependencies; if you need to, write a contract change request in docs/progress/<lane>.md instead.
Record acceptance evidence in docs/progress/<lane>.md. When your "done when" checks pass, rebase on main, run the pre-merge checks in 9.1.1, and stop for review.
```

---

## 10. Windows specifics (summary)
- **Toolchain:** Rust 1.92 MSVC, VS 2022 Build Tools, Node 20, WebView2 runtime (preinstalled on Windows 11). `CARGO_TARGET_DIR=C:\hlp-target`. Build with `cargo build -p hlp-asr-worker -p hlp-cli -p henlocal-player`, never `--workspace`.
- **ASR:** sherpa-onnx 2.0.3 with Qwen3-ASR-0.6B int8 (30 languages, auto only, no forced language, no timestamps). `whatlang` handles LID for the badge, template choice and skip rule. It is CPU-only; measure the real-time factor.
- **sherpa-onnx linking:** the crate links its static `-static-MT-Release-lib` by default. If you hit CRT mismatch link errors, switch to `default-features = false, features = ["shared"]` (copy the DLLs next to the executables and into the bundle), or add `-C target-feature=+crt-static` for the MSVC target in `.cargo/config.toml`. Record which one worked. `VERIFY`.
- **Translation:** llama.cpp b11429 `win-cpu-x64`, `-ngl 0`. **Do not enable Vulkan offload by default**: the translator found corrupted output on Intel Arc 130T with Vulkan (`WINDOWS.md`). Hy-MT2-1.8B on CPU should run at about 30 tokens/s (the translator measured Qwen3-1.7B at about 32 tokens/s).
- **ffmpeg:** BtbN LGPL win64 build; transcode encoder fallback chain in section 5.2.4.
- **WebView2 playback:** H.264/AAC/MP3/VP8/VP9/AV1/Opus play directly. HEVC plays only with hardware support and the HEVC extension (caps decide). MKV goes through remux.
- **Processes:** spawn every child with `CREATE_NO_WINDOW`; kill process trees on cancel (`taskkill /T /F /PID` or a Job Object).
- **Paths:** `%USERPROFILE%\.OminiX\...` for models and tools, `%APPDATA%\com.henlocal.player` for data. Long paths: use `\\?\` only if needed. Filenames from yt-dlp are already sanitized by yt-dlp.

---

## 11. Risks and mitigations
| Risk | Mitigation |
| --- | --- |
| MLX build breaks because the path has spaces | The scripts enforce a no-space `CARGO_TARGET_DIR` (rule 0.3) |
| MLX version drift | Pin OminiX-MLX rev `6aac996` (MLX 0.30.1), the same as the translator; use `MLX_PREBUILT_PATH` as in the translator's CI |
| Auto-LID prompt behaves differently from expected | Tokenizer round-trip and fixture tests in M4. Fallback: run `whatlang` on the transcript, lock that language and force it |
| VAD fires on music or singing, and ASR hallucinates | Drop on `language None`, length-plausibility guard, repetition collapse, minimum speech length |
| Timing imprecision inside long utterances | Speech-only proportional allocation with pause snapping (±0.6 s target). Phase 2: Qwen3-ForcedAligner-0.6B (11 languages, word or character timestamps, ≤5 min per call) ported to MLX |
| WKWebView or WebView2 can't play a file | Caps probe → Remux/Transcode, plus runtime escalation on `<video>` errors |
| Remuxing a huge MKV takes a while before playback | Progress UI; subtitles are produced in parallel. Phase 2: on-the-fly fragmented MP4/HLS |
| Element fullscreen is unavailable in WKWebView | Window fullscreen plus CSS (section 5.9.3) |
| GPU contention between MLX ASR and Metal llama-server | Both have one request in flight; measure. If ASR real-time factor drops below 8×, pause translation while ASR is behind the lookahead |
| Memory on 8–16 GB Macs | Light ASR model option (0.6B); `unload` op; ASR ≈2.5 GB + MT ≈1.2 GB total |
| yt-dlp breaks because sites change | Deno runtime, "Update yt-dlp" button, clear error surfacing |
| Windows CPU ASR too slow | Measure; document it; Phase 2 options: DirectML provider, SenseVoice fast path (zh/en/ja/ko/yue) |
| Checksums unknown at planning time | `TBD-SHA` entries must be filled in during M3/M7/M8. CI test fails if any remain |
| Licensing | `docs/LICENSES.md` (section 13); start from the translator's audit doc |

---

## 12. Phase 2 backlog (do not build now)
1. TTS voice-over (Apple voices first, then Qwen3-TTS MLX from the translator's node), with ducking of the original audio.
2. Word-level timing with Qwen3-ForcedAligner-0.6B (MLX port).
3. Batched ASR decode (upstream `transcribe_batch`, requires MLX 0.32).
4. Whisper (whisper.cpp) fallback for languages outside Qwen3-ASR's 30.
5. Import embedded or external subtitle tracks and translate them directly (skipping ASR).
6. Instant playback of any format through on-the-fly fMP4/HLS remux; audio-first processing for URLs.
7. Subtitle editing in the transcript panel, with re-translation of the edited line; terminology lists (Hy-MT2 terminology template).
8. Choice of translation model (Index-Translate-2B through the same llama-server path; Qwen3.5-2B MLX).
9. Speaker labels; video summary and chapters through a local LLM.
10. Signing, notarization and auto-update (reuse the translator's release pipeline).

---

## 13. Licenses to record in `docs/LICENSES.md`
Qwen3-ASR models (Apache-2.0) · OminiX-MLX (MIT/Apache-2.0) · MLX (MIT) · Hy-MT2 (Tencent Hy license; follow the translator's audit) · llama.cpp (MIT) · sherpa-onnx (Apache-2.0) and ONNX Runtime (MIT) · Silero VAD (MIT) · FFmpeg (LGPL-2.1+ builds preferred; GPL if the chosen macOS build includes GPL parts. It is invoked as a separate executable, never linked. Record the exact build, its configure flags and where its source is available) · yt-dlp (Unlicense) · Deno (MIT) · Tauri (MIT/Apache-2.0) · Svelte (MIT).

---

## Appendix A: Reference commands (macOS dev)

```bash
# one-time
npm --prefix app/ui install
# dev loop (builds hlp-asr-worker + hlp-cli into $TMPDIR/hen-local-player-target, copies sidecar, runs tauri dev)
bash scripts/dev.sh
# headless
cargo run -p hlp-cli -- setup --yes
cargo run -p hlp-cli -- subtitle tests/fixtures/fixture_ja.mp4 --to zh --formats srt,vtt,ass
# fixtures + e2e
bash scripts/make_test_media.sh && bash scripts/e2e.sh
```
(`cargo run` must also use the no-space `CARGO_TARGET_DIR`; `scripts/dev.sh` exports it. Document `export CARGO_TARGET_DIR=$TMPDIR/hen-local-player-target` in the README.)

## Appendix B: Sources consulted for this plan
- Hen-Local-Translator source (local): model catalog, downloader, llama-server backend, ASR nodes, build scripts, `WINDOWS.md`, `docs/TRANSLATION_MODEL_COMPARISON.md`.
- OminiX-MLX `qwen3-asr-mlx` at rev 6aac996 (local checkout) and upstream `main` (it has `transcribe_batch` and requires MLX 0.32).
- Qwen3-ASR official repo (`github.com/QwenLM/Qwen3-ASR`): auto-LID prompt and output format, the 30-language list, and that `<asr_text>` is a non-special token (`Qwen/Qwen3-ASR-0.6B` tokenizer_config); ForcedAligner capabilities.
- Hy-MT2 official README (`github.com/Tencent-Hunyuan/Hy-MT2`) and the `tencent/Hy-MT2-1.8B` model card: 38-row language table, prompt templates, no system prompt, recommended sampling.
- sherpa-onnx docs (`k2-fsa.github.io/sherpa/onnx/qwen3-asr/`, `.../vad/silero-vad.html`) and docs.rs `sherpa-onnx` 2.0.3: Qwen3-ASR 0.6B int8 package, VAD API, prebuilt static libs.
- yt-dlp announcement (yt-dlp issue #15012, EJS wiki): an external JS runtime (Deno by default) is required for YouTube since 2025.11.12.
