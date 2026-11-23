# Implementation contracts

Rust owns the canonical camelCase wire and disk types in `hlp-core`. The UI mirrors them in `app/ui/src/lib/types.ts`. Every pipeline event carries its session ID. Times are milliseconds from the start of the media. Sample offsets always refer to mono signed little-endian 16-bit PCM at 16 kHz.

One shared ASR client and one llama-server serve successive sessions. The worker protocol is line-delimited JSON with monotonically assigned request IDs. A session owns the selected file, track, playhead and cache lifetime. The desktop and CLI use the same session. The local media server issues per-registration opaque URLs and serves byte ranges.

Setup is independent of player sessions. Model and tool paths use `HLP_OMINIX_ROOT` when set. Caches, settings and recently opened items live in the platform application data directory. Lane-local fixture tests read generated speech and its onset truth from `tests/fixtures`.

## Build observations

The pinned upstream MLX release includes libmlx.a, libmlxc.a, libgguflib.a and mlx.metallib. The translator release workflow supplies the SHA-256 for the build archive. A Command Line Tools installation can supply clang on hosts where full Xcode is unavailable. The installed Homebrew FFmpeg on the development Mac omits libvpx. The fixture generator accepts an encoder-capable FFmpeg via HLP_FFMPEG.

## Verification notes

- sherpa-onnx 2.0.3 exposes `OfflineModelConfig::qwen3_asr` with convolutional frontend, encoder, decoder and tokenizer paths. The Rust VAD API gives the global segment start in samples and sample count through `start()` and `n()`.
- The local Qwen3-ASR generation config lists EOS IDs 151643 and 151645. The tokenizer config includes the added token `<asr_text>`.
- The Section 8.3 example using FFmpeg's null muxer fails even for a valid, single-cue SRT with “Output file does not contain any stream.” Explicitly mapping the subtitle stream and decoding to WebVTT verifies parsing and produces the expected cue. The E2E oracle uses `-map 0:s:0 -c:s webvtt -f webvtt -` for nonempty exports.
- The GGUF's pinned LICENSE.txt declares Apache-2.0, and yt-dlp's packaged binary includes GPL components. See [the license inventory](LICENSES.md).
- Hy-MT2's English default prompt has a space between its two instruction sentences and a blank line before the source. The implementation uses the verified default and Structured Data 2 templates. [Upstream instructions at the reviewed revision](https://github.com/Tencent-Hunyuan/Hy-MT2/blob/ff1903ecaa724e10951a23c16817a2413c752b35/README.md).
- Both MLX variants use the same ten pinned runtime and model-card filenames. The two providers use separate immutable revisions with matching contents. The Windows ASR tarball contains three ONNX graphs and the three tokenizer files. Exact archive sizes and digests are in the component catalog.
- FFprobe shares one container name for WebM and Matroska. The EBML DocType distinguishes them. Timestamp-aware 16 kHz resampling keeps PCM aligned across discontinuous AAC packets. The Range server supplements ServeFile with If-Range validation.
- Some existing tokenizer.json files discard the ASR delimiter during skip-special decoding. The worker rebuilds the tokenizer in memory from the original pinned vocabulary, merges and token configuration. Real automatic inference returns the expected language header.
- Native objects and Metal kernels in the upstream prebuilt archive target macOS 15. The developer and packaging scripts build MLX 0.30.1 for macOS 14, with SHA-verified kernels from the corresponding official wheel. The build and fixture recognition passed on the development Mac using the standalone macOS 14.4 SDK. A Sonoma hardware run is outstanding. [Native patch notes](../vendor/qwen3-asr-mlx/PATCHES.md).
