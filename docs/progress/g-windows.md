# Lane G · Windows x64

## Windows PC handoff

- [ ] Build and link the worker, CLI and desktop shell with the Windows x64 MSVC toolchain.
- [ ] Generate the SAPI media matrix, pass every §8.3 check, and record the CPU model, thread count, ASR speed and translation latency.
- [ ] After lane F merges, rebase this lane on main and run the native WebView2 smoke test.

The project owner confirmed a Windows PC is currently unavailable. When one becomes available, use a Windows 10/11 x64 PC with Rust 1.92 MSVC, VS 2022 C++ Build Tools, Node 20.19+, Python 3, WebView2 and an enabled English desktop SAPI voice. In the lane G worktree, run the following in PowerShell. Keep the branch and report the results to the lead before merging.

```powershell
$env:CARGO_TARGET_DIR = 'C:\hlp-target-g'
cargo build --locked -p hlp-asr-worker -p hlp-cli -p henlocal-player
cargo test --locked -p hlp-core -p hlp-media -p hlp-models -p hlp-asr-worker
& "$env:CARGO_TARGET_DIR\debug\hlp-cli.exe" setup --yes --with-urls
& "$env:CARGO_TARGET_DIR\debug\hlp-cli.exe" status
.\scripts\make_test_media.ps1
.\scripts\e2e.ps1
Get-CimInstance Win32_Processor | Select-Object Name,NumberOfLogicalProcessors
npm.cmd --prefix app/ui ci
npm.cmd --prefix app/ui run check
cargo check --locked -p hlp-core -p hlp-media -p hlp-models -p hlp-pipeline -p hlp-cli -p henlocal-player
.\scripts\dev.ps1
```

The fixture generator identifies the actual voice used for each of the seven language slots. On an English-only PC, it uses English for every slot and retains the nine-container coverage. It derives sentence onsets from the trimmed 16 kHz PCM sample counts. The oracle reads each file’s actual spoken language from its truth file. The full report goes to `tests/results/<date>-windows.json`. Record `summary.asrRtf` (audio seconds processed per wall second), `msPerTranslation` and the cached-run time. The Windows ASR goal is 2× real time.

In WebView2, open and play the generated MP4. Check spoken language and bilingual subtitles, then export original SRT, translated VTT and bilingual ASS. Open the MKV and AVI to exercise preparation and seek on the long file. Paste a public spoken-video URL, confirm the metadata, download, play and verify subtitles. Cancel a second download and confirm its temporary files disappear. Save native captures and record the media URL, observed outcome, GPU/encoder and Windows build here. Packaging and installer validation are owned by the lead.

## Work completed on the Apple Silicon host

- [x] Implemented CPU Qwen3-ASR with all three ONNX models and the tokenizer directory. One recognizer serves successive streams. Each stream gets its own generation limit and greedy decoding. The worker advertises automatic recognition and passes the result to the pipeline’s language detection.
- [x] Confirmed the exact Rust configuration field is `model_config.qwen3_asr`. The pinned native implementation reads `max_new_tokens` and `temperature` per stream. It also recognizes a stream-level language hint and generally strips the language scaffold from the returned text. The player intentionally uses automatic recognition on Windows, as specified. [Rust API](https://docs.rs/sherpa-onnx/2.0.3/sherpa_onnx/struct.OfflineQwen3ASRModelConfig.html), [versioned native decoder](https://github.com/k2-fsa/sherpa-onnx/blob/v2.0.3/sherpa-onnx/csrc/offline-recognizer-qwen3-asr-impl.cc), [upstream example](https://github.com/k2-fsa/sherpa-onnx/blob/v2.0.3/rust-api-examples/examples/qwen3_asr.rs).
- [x] Confirmed the actual Windows x64 Sherpa static archive contains `/FAILIFMISMATCH:"RuntimeLibrary=MT_StaticRelease"` and `/DEFAULTLIB:"libcpmt"`. The repository config selects `+crt-static` for this target. Actual Windows link and execution are outstanding.
- [x] Inspected the pinned model archive and Windows FFmpeg configuration. The Windows media encoder chain tries NVENC, Quick Sync, AMF, Media Foundation and libx264 if available. The managed LGPL build disables libx264 and includes OpenH264, which now supplies the final CPU fallback. The fixture generator also supports its software HEVC encoder, Kvazaar. [Pin and build flags](c-models.md#ffmpeg-audit-evidence).
- [x] Ran `rustup target add x86_64-pc-windows-msvc` and `cargo check --locked --target x86_64-pc-windows-msvc -p hlp-core -p hlp-asr-worker` with the real Windows Sherpa archive. Both packages passed. The initial cross-check of the remaining packages reached the native `ring` build, where Apple Clang lacked Windows headers. The portable toolchain check below subsequently passed for the entire workspace. The shared six-package check and Svelte check passed on macOS. The regular core, media and installer suites passed.
- [x] PowerShell 7.5.4 parsed all four scripts. Exercised tool discovery and rejection of a target directory containing spaces. Ran the PowerShell fixture generator with a recorded English sentence provider standing in for Windows SAPI on this Mac. FFmpeg produced all nine media/truth pairs. FFprobe verified H.264/AAC, H.264/AC-3, VP9/Opus, MPEG-4/MP3, HEVC/AAC, six-channel H.264/AAC, AAC audio, the tone-only file, and a 1,200.088 s loop with 360 sentence onsets. This exercised the PowerShell media and truth generation. Native SAPI, MSVC linkage, Windows latency, WebView2 and playback remain the PC checks above.
- [x] Loaded the exact pinned Windows-model assets into the lane's unmodified Sherpa backend compiled for the Apple Silicon host as a standalone CPU smoke. A 2.547 s fixture returned “Today, the sky is clear and blue.” in 0.339 s of decoding. This checks the native model/decoder interface. The Windows CPU measurement remains outstanding.


## Complete Windows-target compile check on macOS

The portable LLVM-MinGW UCRT macOS universal toolchain, release `20261117`, supplied real Windows headers, Clang, the COFF librarian, the resource compiler and LLD. The 125,197,636-byte upstream archive matched SHA-256 `98d340bbb8d46b09ad6886ffba88bd9d5415e131c316de48e6bd7647377446aa`. All tools were unpacked privately. [Upstream release](https://github.com/mstorsjo/llvm-mingw/releases/tag/20261117).

From lane G, with `tool` pointing to the extracted toolchain, the following cross-check passed for all seven explicit packages in 17.01 s. Rust checked the real `x86_64-pc-windows-msvc` code paths, including WebView2. The native C dependencies compiled as x64 Windows COFF with MinGW-w64 headers, and Tauri built the 37,504-byte application resource library.

```sh
export CARGO_TARGET_DIR="${TMPDIR%/}/hlp-target-g"
source scripts/build_env.sh
export CC_x86_64_pc_windows_msvc="$tool/bin/x86_64-w64-mingw32-clang"
export CXX_x86_64_pc_windows_msvc="$tool/bin/x86_64-w64-mingw32-clang++"
export CFLAGS_x86_64_pc_windows_msvc="--target=x86_64-w64-windows-gnu"
export CXXFLAGS_x86_64_pc_windows_msvc="$CFLAGS_x86_64_pc_windows_msvc"
export AR_x86_64_pc_windows_msvc="$tool/bin/llvm-lib"
export RC_x86_64_pc_windows_msvc="$tool/bin/llvm-rc"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$tool/bin/lld-link"
cargo check --locked --target x86_64-pc-windows-msvc -p hlp-core -p hlp-media -p hlp-models -p hlp-pipeline -p hlp-cli -p henlocal-player -p hlp-asr-worker
```

A separate `cargo build --locked --target x86_64-pc-windows-msvc -p hlp-asr-worker` reached the final linker. LLD reported missing Microsoft SDK import libraries including `kernel32.lib` and missing static Microsoft runtime libraries `libcpmt.lib` and `LIBCMT.lib`. The Sherpa Windows archive explicitly requests that runtime. The cross-check verifies types, dependencies and resources. A runnable MSVC executable and installer still require those Microsoft libraries, followed by native execution.

## Checks after the macOS desktop lane merged

Rebased on the merged desktop player at `5efcd74`. The seven-package Windows x64 compile check above included the real Tauri commands, pre-warm, URL and player integration and passed in 2.18 s. The shared six-package macOS Cargo check passed in 1.83 s. Regular core, media, installer, pipeline and CLI tests reported 34 passed, 0 failed, 5 explicitly ignored. `cargo fmt --all --check` and `git diff --check` passed. After `npm --prefix app/ui ci`, Svelte reported 0 errors and 0 warnings and Vite built the production UI. Windows runtime, throughput and installed-app checks remain pending in the handoff.
