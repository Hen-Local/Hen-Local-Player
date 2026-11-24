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

## Bilingual phrase alignment

The player now uses matching sentences, punctuation on both sides, and shared clause introducers to page an utterance with a saved translation. If a sentence has no reliable place to divide both languages, the complete pair stays on screen through its speech interval. On fresh translations, the core plans extra work only for separable pairs over 120 display columns. It can reuse shorter translated clauses from the same utterance. The resulting UTF-8 source and target end offsets restore the exact phrase pairs on reopen. Missing or malformed offsets fall back to the stored full text.

Read-only replays of the two reported video caches on Apple Silicon covered 64 Mandarin utterances translated into English and 180 English utterances translated into Mandarin. Across all 244 utterances, the 523 generated cues preserved both complete texts after presentation formatting. Every utterance’s last cue reached within 100 ms of its recorded speech end or later.

| Reproduction | Before | After |
| --- | --- | --- |
| 00:41, `三个高技能人才的亲身经历` | Three pairs, including `来讲三个高` against `personal experiences` | One full pair, retaining the English noun phrase and its Chinese meaning |
| 00:45, `的故事啊，可以说是把这个问题讲透了` | Two pairs, the first English line ending in `to have` | One full pair |
| 01:42, Windows API and SteamOS | Six pairs, including `into something that your` against `原生处理的` | Two pairs, breaking after `natively` and `兼容层`. The second pair contains the virtualization clause in both languages. |

The Mandarin recognition result at 00:45 itself starts with `的故事啊`. The preceding utterance ends with `亲身经历。`, followed by an 864 ms pause. The only saved English translation for this file used context-off mode and starts the second utterance with “The story”. Adjusting the cue boundaries preserves that wording. Correcting the recognized Chinese would require revisiting the audio.

Eighteen previously cached pairs exceeded 120 display columns after this conservative reflow. The widest was 200. Running the fresh-translation planner on the same raw utterances selected 19 of 244 for additional work and requested 42 translated phrases. The planner measures the original punctuated text before the display formatter removes sentence punctuation.

`cargo test --locked -p hlp-core` passed all 31 tests, including the screenshot sentences, cache offset round trips, corrupt offsets, single-line exports and exact source-byte reconstruction. `PROPTEST_CASES=4096 cargo test --locked -p hlp-core --test cues no_overlap`, `cargo clippy --locked -p hlp-core --all-targets -- -D warnings` and `cargo fmt -p hlp-core --check` also passed.

### Preserving the whole translation during phrase alignment

Independent translations sometimes repeated a neighboring phrase. The aligner now uses them as hints to locate word boundaries in the original whole translation. Both sides of each proposed phrase must match at least 70% of the longer word sequence in order. The ICU dictionary supplies legal word cuts for unspaced text. When one unmatched word could be assigned to either side, a unique punctuation break in the whole translation can resolve it. Larger ambiguities leave the clause intact. Work is bounded to 24 source spans and 256 words per sequence.

Local replay compared all 22 attempted refinements in the two context-enabled video translations against the whole translations from the original model run. Three had enough evidence for eight aligned spans. In the Mandarin-to-English video, the project, startup funding, jobs, economic development and innovation clauses each lined up with their corresponding translation. In English-to-Mandarin, the November offer, gaming bundle and non-gamers clauses lined up separately. Each set of target slices reconstructed the original translation byte for byte. The other 19 retained the whole translation, including both observed model echoes and the differently ordered SteamOS sentence. An existing double-negative translation error in the non-gamers clause remains in the whole translation.

The final core run passed all 34 tests. The three additional regression tests cover verbatim English and Chinese target slices, overlapping or reordered hints, an internal hyphen, ambiguous dictionary cuts, oversized output, and the one-word punctuation tie. Clippy with warnings denied and the core format check passed.
