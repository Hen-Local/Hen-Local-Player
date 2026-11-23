# Lane D · ASR worker and client

- [x] Japanese automatic detection and forced recognition on real speech.
- [x] Tokenizer round-trip, generation stop IDs, PCM padding and JSONL error recovery.
- [x] Automatic restart after a deliberately killed worker.
- [x] The native MLX libraries compile and link with the macOS 14.4 Command Line Tools SDK.
- [ ] Post-rebase lane tests and the shared Rust/UI checks.

## Evidence on the development Mac

Use the no-space lane target, `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, and the Mac's installed Qwen3-ASR 1.7B model. The real model directory was read-only throughout.

- `ffmpeg -nostdin -i tests/fixtures/fixture_ja.mp4 -map 0:a:0 -ac 1 -ar 16000 -f s16le /tmp/hlp-d-ja.pcm` followed by six JSONL requests to `hlp-asr-worker`. Operations were hello, load, transcribe in automatic mode, transcribe with Japanese forced, unload and shutdown. Every stdout line was a successful protocol response. Samples 8000 through 42000 yielded `language Japanese<asr_text>今日はとてもいい天気です。` automatically and `今日はとてもいい天気です。` with Japanese forced. Model load took 1862 ms, first recognition took 1419 ms, and the next recognition took 248 ms for 2125 ms of audio.
- `cargo test --locked -p qwen3-asr-mlx tokenizer_round_trip_and_eos -- --ignored` passed. The existing cached tokenizer had incorrectly classified ID 151704 as special. The corrected in-memory tokenizer preserved the marker with skip-special decoding, and the stop IDs matched the local generation config.
- `HLP_ASR_WORKER="$CARGO_TARGET_DIR/debug/hlp-asr-worker" HLP_FFMPEG=/tmp/hlp-fixture-tools/ffmpeg cargo test --locked -p hlp-pipeline real_worker_restarts_after_kill -- --ignored` passed. The test kills the loaded worker with SIGKILL, checks for a new PID and one restart, and requires a Japanese auto-mode generation from the replacement process.

## Native dependency finding

The pinned OminiX prebuilt C++ archive targets macOS 15 and references symbols missing from the development Mac's standalone SDK 14.4. A minimal executable with a dead-stripped engine linked during M0. A worker that actually loaded the model failed to link. Rebuilding the pinned MLX 0.30.1 source at deployment target 14.0 and using the same version's official macOS 14 Metal kernels resolved it. Standalone Command Line Tools also lack the offline Metal compiler used by upstream to expand headers for runtime compilation. The helper expands those headers from the same verified source tree. The native build helper, archive pins and patch notes are under `vendor/qwen3-asr-mlx`. The helper installs once per target directory and accepts a normal Command Line Tools SDK.

Lane G owns the Windows backend implementation. Its platform interface is in `crates/hlp-asr-worker/src/backend/windows.rs`.
