# Lane A · Core logic

- [x] All section 8.1 core checks pass. `cargo test --locked -p hlp-core` passed 11 module tests, 5 cue integration tests and 2 export integration tests on Apple Silicon. The cue suite exercised 256 randomized overlapping-input cases along with speech-only timing, minimum spacing, stale lines, grapheme boundaries, mixed scripts and translation allocation. Export goldens covered SRT, VTT and ASS, CJK, RTL text, missing translations, escaping, hour overflow and centisecond rounding.
- [x] `cargo clippy --locked -p hlp-core --all-targets -- -D warnings` and `cargo fmt -p hlp-core --check` passed.
- [x] After rebasing on main, `cargo check --locked -p hlp-core -p hlp-media -p hlp-models -p hlp-pipeline -p hlp-cli -p henlocal-player` succeeded. Foundation stubs in the other crates still emitted unused-variable warnings.
- [x] `npm --prefix app/ui ci` and `npm --prefix app/ui run check` succeeded, with 0 Svelte errors and 0 warnings.

The upstream English default prompt has a space before “Note that”. Both default prompts separate the source text by a blank line. The implementation uses those verified separators and the “Structured Data 2” template for contextual translation. [Hy-MT2 translation instructions](https://github.com/Tencent-Hunyuan/Hy-MT2/blob/ff1903ecaa724e10951a23c16817a2413c752b35/README.md#hy-mt2-translation-task-instruction-examples-chinese-english-comparison)
