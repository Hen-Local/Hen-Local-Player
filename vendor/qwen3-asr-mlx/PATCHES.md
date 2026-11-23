# Source

Copied from OminiX-ai/OminiX-MLX, revision 6aac996db8b71fb7dae7a2409c46b4f2ade93092, directory qwen3-asr-mlx. The Cargo manifest points the three workspace-local dependencies at that exact revision and removes the standalone example. Upstream license texts are included.

## Automatic language detection

The existing forced-language APIs delegate to `transcribe_raw`. Passing `None` ends the prompt immediately after the assistant header. This leaves the language prefix and transcript delimiter in the model's generation. The worker returns the unparsed generation over JSON Lines.

Some existing installs have a generated tokenizer.json with every added token marked special and no byte-level pre-tokenizer. Its decoder drops the transcript delimiter. When that happens, reconstruct the tokenizer in memory from vocab.json, merges.txt and tokenizer_config.json, respecting the numeric IDs and special flags. Leave shared model files untouched. The model-backed regression test checks that ID 151704 survives decoding and that the stop IDs match generation_config.json.

## Native build on macOS 14+

The OminiX prebuilt v0.1.0 archive contains macOS 15 objects. Linking a worker that calls the model with the Command Line Tools 14.4 SDK fails on newer Metal, Foundation and C++ symbols. Its metallib also contains a macOS 15 target. `tools/prepare_mlx.py` builds the same MLX 0.30.1 source and the lockfile's C wrapper with a macOS 14.0 deployment target. It obtains the matching macOS 14 Metal kernels from the official MLX release wheel. CMake receives pinned, verified source trees for each native dependency.

Standalone Command Line Tools have no offline Metal compiler. Upstream's JIT preamble builder uses it to enumerate header dependencies and silently generates incomplete snippets without it. The build helper expands only project includes, preserving system includes and conditionals for the Metal runtime. It installs the native libraries atomically into a no-space Cargo target directory. Full Xcode and an installed Command Line Tools SDK are both supported as build toolchains.

```sh
export CARGO_TARGET_DIR="${TMPDIR:-/tmp}/hen-local-player-target"
export MLX_PREBUILT_PATH="$(python3 vendor/qwen3-asr-mlx/tools/prepare_mlx.py)"
cargo build --locked -p hlp-asr-worker
```
