# Signed desktop releases

Version 0.1.0 ships the locally notarized Apple Silicon build. The two-platform workflow described below was prepared separately for later releases.

## Package and manifest checks

On November 24, 2026, `actionlint .github/workflows/release.yml` and `bash -n scripts/build_macos_app.sh` passed. Stable source tags trigger the release workflow in the private Hen-Local/Hen-Local-Player repository. The two build jobs run on macOS 15 arm64 and Windows Server 2022 x64. The Mac build takes its precompiled macOS 14 Metal kernels from the pinned MLX wheel through `prepare_mlx.py`. [Hosted runner architectures](https://docs.github.com/en/actions/reference/runners/github-hosted-runners#standard-runners-for-private-repositories)

The Mac build with `--updater` archives the final, signed app after the Metal library and worker resources have been staged. Locally, extracting an archive made with the release tar command preserved the app signature under `codesign --verify --deep --strict`. Windows reuses the NSIS installer as its updater payload. [Supported Tauri updater formats](https://v2.tauri.app/plugin/updater/#building)

In an isolated directory, `python3 scripts/prepare_app_release.py v0.1.0 darwin-aarch64` signed a copy of the Mac payload with a disposable key and generated a one-platform manifest. A second run with `darwin-aarch64 windows-x86_64` used a synthetic Windows payload to exercise the two-platform manifest. The Tauri CLI signed each payload with its app version in the trusted comment. Minisign verified the content and the comment with the matching verification key. Independent reads matched the byte length and SHA-256 recorded in each platform entry. The script rejected a mismatched release tag and a nonmatching signing key. A failed signing attempt preserved the previous manifest and signature sidecars. The Windows packaging job and an upgrade on a real Windows installation require a Windows runner.

The signing job requires `TAURI_SIGNING_PRIVATE_KEY` and its `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if encrypted. Each tag's `latest.json` contains only the signed platforms selected for that run. The workflow uploads the DMG, updater payloads, signatures and manifest together in a private draft, then finalizes the release. An interrupted draft upload can be retried. Source repository access controls cover both the packages and the manifest. The payment service can use this private release as its upstream and issue an expiring download URL after checking the purchase.

## Apple distribution credentials

For a Developer ID release, configure these five Actions secrets in the private source repository.

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE_P12_BASE64` | Base64 of the exported Developer ID Application certificate and private key in PKCS#12 format. |
| `APPLE_CERTIFICATE_PASSWORD` | Export password for the PKCS#12 file. |
| `APPLE_NOTARY_APPLE_ID` | Apple Account email authorized for notarization. |
| `APPLE_NOTARY_TEAM_ID` | Developer Team ID of the imported certificate. |
| `APPLE_NOTARY_PASSWORD` | App-specific password for the Apple Account. |

The workflow imports the certificate into a temporary runner keychain, checks for one valid Developer ID Application identity for the configured team, and stores the notarization profile in that keychain. The bundle is signed, notarized and stapled before the updater archive is created. The build extracts that archive to verify its code signature and staple, then signs, notarizes and staples the DMG. The job removes the keychain after packaging. Leaving all five secrets empty produces an ad-hoc private test build. An incomplete set fails before the build begins. [GitHub certificate setup](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications) · [Apple notarization](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)

Local checks covered empty credentials, five incomplete combinations, team mismatch and simulated full credentials. The updater tar round trip preserved the valid signature and staple of an existing notarized app. GitHub Actions lint and Bash syntax checks passed.

On November 24, 2026, Apple accepted the locally submitted 0.1.0 Apple Silicon DMG under submission `e8f1b58d-d4c8-46e1-a014-486c56cfb576`. The disk image and standalone app received their tickets. The updater archive was rebuilt from the stapled app, extracted for ticket verification, and signed with the publisher’s update key. The final DMG is 126,948,782 bytes with SHA-256 `ee8cd42af06fd404208cea8fce9655b09c15a433f6d0ecf1e0def7145e67a38d`. Gatekeeper accepted the quarantined disk image and the app copied from it. Paid distribution and the production buyer update feed have not been enabled.

## Windows speech model downloads

The Windows Qwen3-ASR 0.6B model installs six files, including the tokenizer subfolder. The global source is pinned to Hugging Face revision `68818b2313fe77bd06f6a7c5068ff3ef59d02b8a` and the mainland source to ModelScope revision `de9e449eb376dcb472c1ce8141fbae8d524fabbd`. Both retain the existing installation layout. All twelve file URLs answered download requests, and ModelScope’s revision metadata reported all six expected sizes and hashes. [Hugging Face source](https://huggingface.co/csukuangfj2/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/tree/68818b2313fe77bd06f6a7c5068ff3ef59d02b8a) · [ModelScope source](https://modelscope.cn/models/zengshuishui/Qwen3-ASR-onnx)

Downloaded the original 878,702,423-byte release archive and verified its SHA-256 as `393f8a14e2f5fb96746aaab342997a40641001fbd5bf9592a080a8329178ee96`. Streaming every runtime member from that archive reproduced the catalog size and SHA-256 for all six files. `cargo test --locked --offline -p hlp-models --lib` passed nine tests with one opt-in test ignored, including recognition of legacy extracted models and concurrent installation into a nested directory. [Original Sherpa model release](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models)
