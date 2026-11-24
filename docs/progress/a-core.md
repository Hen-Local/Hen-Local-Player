# Lane A · Core logic

- [x] All section 8.1 core checks pass. `cargo test --locked -p hlp-core` passed 11 module tests, 5 cue integration tests and 2 export integration tests on Apple Silicon. The cue suite exercised 256 randomized overlapping-input cases along with speech-only timing, minimum spacing, stale lines, grapheme boundaries, mixed scripts and translation allocation. Export goldens covered SRT, VTT and ASS, CJK, RTL text, missing translations, escaping, hour overflow and centisecond rounding.
- [x] `cargo clippy --locked -p hlp-core --all-targets -- -D warnings` and `cargo fmt -p hlp-core --check` passed.
- [x] After rebasing on main, `cargo check --locked -p hlp-core -p hlp-media -p hlp-models -p hlp-pipeline -p hlp-cli -p henlocal-player` succeeded. Foundation stubs in the other crates still emitted unused-variable warnings.
- [x] `npm --prefix app/ui ci` and `npm --prefix app/ui run check` succeeded, with 0 Svelte errors and 0 warnings.

The upstream English default prompt has a space before “Note that”. Both default prompts separate the source text by a blank line. The implementation uses those verified separators and the “Structured Data 2” template for contextual translation. [Hy-MT2 translation instructions](https://github.com/Tencent-Hunyuan/Hy-MT2/blob/ff1903ecaa724e10951a23c16817a2413c752b35/README.md#hy-mt2-translation-task-instruction-examples-chinese-english-comparison)

## Generated subtitles on one line per language

Generated cues use the original recognition and translation text to align phrases, aiming for 42 display columns per language. After timing is assigned, punctuation between phrases and line breaks collapse to a single space. Marks inside contractions, compound words and numbers retain their meaning, including “I’ll”, “real-time”, “3.14”, “10:30” and “25%”. The stored utterance and translation text retain the punctuation used for alignment when the movie is reopened.

On Apple Silicon, `cargo test --locked -p hlp-core` passed 25 tests. Regression cases cover existing punctuated transcription and translation, repeated punctuation, empty and translation-only results, numeric separators, and mixed-language text longer than the former line limit. SRT and WebVTT each carry one physical text row per selected language. ASS disables automatic line wrapping and inserts one explicit break between the two languages in a bilingual cue. Separate goldens verify ordering, VTT escaping and time formatting in each export.

The release CLI passed all 81 fixture checks with the installed speech and translation models. The 20-minute English fixture produced 359 cues in 90.6 seconds and reopened from cache in 94 ms. All nine fixture exports passed their FFmpeg check and the largest onset error was 216 ms. The core Clippy check with warnings denied and the Svelte check also passed. [Fixture report](../../tests/results/2026-11-23-darwin-single-line.json).
