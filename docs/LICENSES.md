# Models, tools and assets

Hen Local Player’s original code is licensed under the Apache License, Version 2.0. The root `LICENSE` contains the terms and `NOTICE` credits Hen Local Translator code and artwork adapted for this project. Source files record the original paths. The interface uses fonts installed on the user’s system. Project names and logos are subject to the trademark provision in section 6 of the Apache License.

The installer contains the player, the speech worker, native libraries and the bundled demo. Setup downloads the selected model weights and optional tools separately. Each third-party component retains its upstream terms.

## Notices included with the installer

On macOS, open the application package and look in `Contents/Resources/licenses`. The Windows installer includes the corresponding `licenses` resource directory.

- `Hen-Local-Player-LICENSE.txt` and `NOTICE.txt` contain the project license and credits.
- `Dependency-Notices.txt` identifies the locked Rust and frontend packages, their source URLs and the license texts available with the build. The versioned source links also provide the source for the MPL-2.0 components. Both platform packaging scripts generate this file using `scripts/stage_licenses.py`. Supplementary texts for packages that omit a license file from their registry archive are kept in `docs/licenses/registry`.
- `Native-Notices.txt` includes the MLX and Metal dependencies, the native libraries linked by sherpa-onnx 2.0.3, and ONNX Runtime 1.28.2’s third-party notices. The upstream revisions are listed alongside their license texts. [sherpa-onnx build inputs](https://github.com/k2-fsa/sherpa-onnx/tree/38016c6e0622e90bf417c35e8b47e0eb805af417/cmake) · [ONNX Runtime notices](https://github.com/microsoft/onnxruntime/blob/v1.28.2/ThirdPartyNotices.txt)
- `Mei-Voice-LICENSE.txt` credits the voice used for the Japanese narration. The six lines were synthesized, trimmed and timed to the project’s train animation. [MMDAgent](https://www.mmdagent.jp/) · [Creative Commons Attribution 3.0](https://creativecommons.org/licenses/by/3.0/)

## Components

| Component | Delivery | Upstream license and provenance |
| --- | --- | --- |
| Qwen3-ASR 1.7B and 0.6B | Model download | Apache-2.0. [Upstream speech models](https://github.com/QwenLM/Qwen3-ASR) and [MLX conversion](https://huggingface.co/mlx-community/Qwen3-ASR-1.7B-8bit). |
| OminiX Qwen3-ASR and MLX Rust bindings | Worker | MIT or Apache-2.0. Upstream notice texts and patch notes accompany the vendored source. [Pinned revision](https://github.com/OminiX-ai/OminiX-MLX/tree/6aac996db8b71fb7dae7a2409c46b4f2ade93092). |
| MLX | Worker and Metal kernels | MIT. [Apple MLX](https://github.com/ml-explore/mlx/blob/main/LICENSE). |
| Hy-MT2-1.8B Q4_K_M | GGUF model download | Tencent's license text for the exact distributed GGUF revision specifies Apache-2.0. [Pinned LICENSE.txt](https://huggingface.co/tencent/Hy-MT2-1.8B-GGUF/blob/a0c709d9fac510f2c807aa3af52872340dc37a4a/LICENSE.txt). |
| llama.cpp | Separate downloaded process | MIT. The runtime includes its upstream license. [Release b11429](https://github.com/ggml-org/llama.cpp/releases/tag/b11429). |
| sherpa-onnx and ONNX Runtime | Native VAD and Windows recognition | Apache-2.0 and MIT, respectively. [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx/blob/master/LICENSE), [ONNX Runtime](https://github.com/microsoft/onnxruntime/blob/main/LICENSE). |
| Silero VAD | Speech detector downloaded during model setup | MIT. The desktop package includes the upstream notice. [Silero](https://github.com/snakers4/silero-vad/blob/master/LICENSE). |
| ICU4X word segmentation and dictionaries | Compiled into the player and CLI | Unicode-3.0. For redistribution, the desktop package includes `licenses/ICU4X-LICENSE.txt` and `licenses/ICU-Dictionary-LICENSE.txt`. The latter includes the individual Chinese/Japanese, Khmer, Thai, Lao and Burmese dictionary notices. [ICU4X 2.4.0 license](https://github.com/unicode-org/icu4x/blob/031d9e89f6a3fd26a14491e090729d46094328fd/LICENSE), [ICU dictionary sources](https://github.com/unicode-org/icu/tree/release-79.1rc/icu4c/source/data/brkitr/dictionaries). |
| yt-dlp | Optional standalone executable | Source code is Unlicense. Upstream identifies the PyInstaller binaries for macOS and Windows as GPL-3.0-or-later combined works. [Distribution licenses](https://github.com/yt-dlp/yt-dlp#licensing). |
| Deno | Optional separate process | MIT plus notices for its components. [License](https://github.com/denoland/deno/blob/main/LICENSE.md). |
| Tauri and Svelte | Desktop shell and interface | Tauri uses MIT or Apache-2.0 and Svelte uses MIT. [Tauri](https://github.com/tauri-apps/tauri#license), [Svelte](https://github.com/sveltejs/svelte/blob/main/LICENSE.md). |
| Hen Local icon and interface artwork | Desktop shell | Adapted from Hen Local Translator, including `hen-local-translator-shell/icons`. See the root `NOTICE`. |
| Japanese demonstration | Bundled animation, subtitle track and narration | Project animation and subtitles. Japanese speech synthesized with HTS Voice “Mei” from the MMDAgent Project Team, © 2009–2013 Nagoya Institute of Technology, Department of Computer Science. [Voice license](https://creativecommons.org/licenses/by/3.0/). |
| Test speech | Locally generated development fixtures | The fixture scripts can use installed system voices. These fixtures are excluded from the distribution. |

## FFmpeg builds

The open-source Apple Silicon distribution includes FFmpeg 9.1 and FFprobe as stand-alone executables under LGPL-2.1-or-later. Their software AV1 decoder is dav1d 1.5.5 under BSD-2-Clause. They use Apple's system frameworks for hardware video and audio codecs. The build uses the unmodified upstream releases. This software is based in part on the work of the Independent JPEG Group. [FFmpeg licensing](https://ffmpeg.org/legal.html) · [dav1d release](https://downloads.videolan.org/pub/videolan/dav1d/1.5.5/)

For complete corresponding source, open the Mac application's package and navigate to `Contents/Resources/licenses/FFmpeg-sources`. This directory contains the original FFmpeg and dav1d source archives, the build script, and upstream notices. The FFmpeg license is in `Contents/Resources/licenses/FFmpeg-LICENSE.txt`. With Apple Command Line Tools, Meson, Ninja and pkg-config installed on an Apple Silicon Mac, run `bash build_macos_media.sh /tmp/hen-media-rebuild` from the source directory to rebuild both tools offline. [FFmpeg 9.1 source](https://ffmpeg.org/releases/ffmpeg-9.1.tar.xz) · [dav1d 1.5.5 source](https://downloads.videolan.org/pub/videolan/dav1d/1.5.5/dav1d-1.5.5.tar.xz)

The separate macOS command-line setup still offers the pinned OSXExperts binaries. The open-source Mac installer uses its own source build. [Command-line tool distributor](https://www.osxexperts.net/)

The Windows build is BtbN FFmpeg 8.1.3-27-g8dca1586f4 win64-lgpl with version3 enabled. Setup preserves the archive’s LICENSE.txt. The audit records the complete configure strings for both builds. [Pinned Windows release](https://github.com/FFmpeg-Builds/FFmpeg-Builds/releases/tag/autobuild-2026-10-31-12-59), [source revision](https://github.com/FFmpeg/FFmpeg/tree/8dca1586f4), [configure flags](progress/c-models.md#ffmpeg-audit-evidence).
