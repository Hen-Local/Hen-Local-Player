# Models, tools and assets

Audit structure adapted from Hen-Local-Translator/docs/MODEL_AND_ASSET_LICENSE_AUDIT.md. The installer contains the player, its speech worker, native libraries and interface assets. Setup downloads the selected models and external executables to the shared installation directory.

| Component | Delivery | Upstream license and provenance |
| --- | --- | --- |
| Qwen3-ASR 1.7B and 0.6B | Model download | Apache-2.0. [Upstream speech models](https://github.com/QwenLM/Qwen3-ASR) and [MLX conversion](https://huggingface.co/mlx-community/Qwen3-ASR-1.7B-8bit). |
| OminiX Qwen3-ASR and MLX Rust bindings | Worker | MIT or Apache-2.0. Upstream notice texts and patch notes accompany the vendored source. [Pinned revision](https://github.com/OminiX-ai/OminiX-MLX/tree/6aac996db8b71fb7dae7a2409c46b4f2ade93092). |
| MLX | Worker and Metal kernels | MIT. [Apple MLX](https://github.com/ml-explore/mlx/blob/main/LICENSE). |
| Hy-MT2-1.8B Q4_K_M | GGUF model download | Tencent's license text for the exact distributed GGUF revision specifies Apache-2.0. [Pinned LICENSE.txt](https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/blob/a0c709d9fac510f2c807aa3af52872340dc37a4a/LICENSE.txt). |
| llama.cpp | Separate downloaded process | MIT. The runtime includes its upstream license. [Release b11429](https://github.com/ggml-org/llama.cpp/releases/tag/b11429). |
| sherpa-onnx and ONNX Runtime | Native VAD and Windows recognition | Apache-2.0 and MIT, respectively. [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx/blob/master/LICENSE), [ONNX Runtime](https://github.com/microsoft/onnxruntime/blob/main/LICENSE). |
| Silero VAD | ONNX model download | MIT. [Silero](https://github.com/snakers4/silero-vad/blob/master/LICENSE). |
| yt-dlp | Optional standalone executable | Source code is Unlicense. Upstream identifies the PyInstaller binaries for macOS and Windows as GPL-3.0-or-later combined works. [Distribution licenses](https://github.com/yt-dlp/yt-dlp#licensing). |
| Deno | Optional separate process | MIT plus notices for its components. [License](https://github.com/denoland/deno/blob/main/LICENSE.md). |
| Tauri and Svelte | Desktop shell and interface | Tauri uses MIT or Apache-2.0 and Svelte uses MIT. [Tauri](https://github.com/tauri-apps/tauri#license), [Svelte](https://github.com/sveltejs/svelte/blob/main/LICENSE.md). |
| Hen Local icon | Desktop shell | Copied from Hen-Local-Translator/hen-local-translator-shell/icons. Company artwork. |
| Test speech | Locally generated fixtures | The fixture scripts use installed system voices. Generated media are excluded from Git. |

## FFmpeg builds

The media layer invokes FFmpeg and FFprobe as separate executables. The macOS fixture generator was exercised with OSXExperts FFmpeg 9.1 arm64. Its Mach-O deployment target is macOS 13.0. The binary reports VideoToolbox, GPL, version 3, x264, x265 and libvpx support. [Build publisher and corresponding source links](https://www.osxexperts.net/).
