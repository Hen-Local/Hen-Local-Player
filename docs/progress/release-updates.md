# Signed desktop releases

## Publishing an Apple Silicon update

1. Set the version with `python3 scripts/version.py set <version>`. Verify it with `python3 scripts/version.py check`, then commit and tag the same revision as `v<version>`.
2. In Actions, select **Publish signed macOS release**, choose **Run workflow**, and select that tag. The workflow checks the version and repository before building on a macOS arm64 runner. The runner imports the Developer ID certificate into a temporary keychain and removes it after the job.
3. The build runs `bash scripts/build_macos_app.sh --updater`. It signs and notarizes the app, staples its ticket, verifies the updater archive, and then signs, notarizes and staples the DMG. `scripts/prepare_app_release.py` signs the update and verifies it against the public key embedded in the app.
4. GitHub Releases hosts the DMG, the update archive, its signature and `latest.json`. Public downloads work without an account. In the app, **Settings → General → Software Update** lists published stable releases. An installation of the original private 0.1.0 build can switch to this channel by installing the public DMG once.

## Release credentials

Store the following values as GitHub Actions secrets when using the hosted release workflow. Local releases can use the existing notarization Keychain profile and local update key as described in the README.

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE_P12_BASE64` | Base64 of the exported Developer ID Application certificate and private key in PKCS#12 format. |
| `APPLE_CERTIFICATE_PASSWORD` | Export password for the PKCS#12 file. |
| `APPLE_NOTARY_APPLE_ID` | Apple Account email authorized for notarization. |
| `APPLE_NOTARY_TEAM_ID` | Developer Team ID of the imported certificate. |
| `APPLE_NOTARY_PASSWORD` | App-specific password for the Apple Account. |
| `TAURI_SIGNING_PRIVATE_KEY` | Publisher key matching `app/updater.pub`. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password when the publisher key is encrypted. |

[GitHub certificate setup](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications) · [Apple notarization](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow) · [Tauri updater formats](https://v2.tauri.app/plugin/updater/#building)

## Windows speech model downloads

The Windows Qwen3-ASR 0.6B model installs six files, including the tokenizer subfolder. The global source is pinned to Hugging Face revision `68818b2313fe77bd06f6a7c5068ff3ef59d02b8a` and the mainland source to ModelScope revision `de9e449eb376dcb472c1ce8141fbae8d524fabbd`. Both retain the existing installation layout. All twelve file URLs answered download requests, and ModelScope’s revision metadata reported all six expected sizes and hashes. [Hugging Face source](https://huggingface.co/csukuangfj2/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tree/68818b2313fe77bd06f6a7c5068ff3ef59d02b8a) · [ModelScope source](https://modelscope.cn/models/zengshuishui/Qwen3-ASR-onnx)

Downloaded the original 878,702,423-byte release archive and verified its SHA-256 as `393f8a14e2f5fb96746aaab342997a40641001fbd5bf9592a080a8329178ee96`. Streaming every runtime member from that archive reproduced the catalog size and SHA-256 for all six files. `cargo test --locked --offline -p hlp-models --lib` passed nine tests with one opt-in test ignored, including recognition of legacy extracted models and concurrent installation into a nested directory. [Original Sherpa model release](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models)
