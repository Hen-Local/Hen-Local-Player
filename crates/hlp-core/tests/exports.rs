use hlp_core::{
    subs::{filename, render},
    types::*,
};
fn cues() -> Vec<Cue> {
    let first = Cue {
        id: "u000001-0".into(),
        utterance_id: "u000001".into(),
        original: "你好｛朋友｝！".into(),
        translation: Some("مرحبا R&D <أهلا>".into()),
        start_ms: 3723456,
        end_ms: 3725995,
        lang: Some(LangCode("zh".into())),
        stale: false,
    };
    let second = Cue {
        start_ms: 360000000,
        end_ms: 360002000,
        translation: Some(" ".into()),
        original: "未译 {原文}".into(),
        ..first.clone()
    };
    vec![first, second]
}
#[test]
fn golden_files() {
    for (format, expected) in [
        (SubtitleFormat::Srt, include_str!("golden/sample.srt")),
        (SubtitleFormat::Vtt, include_str!("golden/sample.vtt")),
        (SubtitleFormat::Ass, include_str!("golden/sample.ass")),
    ] {
        let expected = if cfg!(windows) {
            expected.replace("PingFang SC", "Microsoft YaHei")
        } else {
            expected.to_owned()
        };
        assert_eq!(
            render(
                &cues(),
                &format,
                &SubtitleVariant::Bilingual(BilingualOrder::TranslationFirst)
            ),
            expected
        );
    }
}
#[test]
fn variants_and_filenames() {
    let c = cues();
    let tr = render(&c, &SubtitleFormat::Srt, &SubtitleVariant::Translation);
    assert_eq!(tr, "1\n01:02:03,456 --> 01:02:05,995\nمرحبا R&D <أهلا>\n\n");
    let orig = render(&c, &SubtitleFormat::Srt, &SubtitleVariant::Original);
    assert!(!orig.contains("مرحبا"));
    let both = render(
        &c,
        &SubtitleFormat::Ass,
        &SubtitleVariant::Bilingual(BilingualOrder::OriginalFirst),
    );
    assert!(both.contains(r"{\rOrig}你好 朋友\N{\rTrans}"));
    for (variant, format, name) in [
        (
            SubtitleVariant::Original,
            SubtitleFormat::Srt,
            "movie.ja.srt",
        ),
        (
            SubtitleVariant::Translation,
            SubtitleFormat::Vtt,
            "movie.zh.vtt",
        ),
        (
            SubtitleVariant::Bilingual(BilingualOrder::TranslationFirst),
            SubtitleFormat::Ass,
            "movie.ja-zh.ass",
        ),
    ] {
        assert_eq!(filename("movie", "ja", "zh", &format, &variant), name);
    }
}

#[test]
fn one_row_per_language_in_every_generated_export() {
    let original = "每一行很短，\n并保留原来的词语。";
    let translation =
        "I’ll read 3.14, followed by a sentence with enough words to exceed the old line limit.";
    let cue = Cue {
        original: original.into(),
        translation: Some(translation.into()),
        ..cues().remove(0)
    };
    let rows = [
        "每一行很短 并保留原来的词语",
        "I’ll read 3.14 followed by a sentence with enough words to exceed the old line limit",
    ];
    for format in [
        SubtitleFormat::Srt,
        SubtitleFormat::Vtt,
        SubtitleFormat::Ass,
    ] {
        let rendered = render(
            std::slice::from_ref(&cue),
            &format,
            &SubtitleVariant::Bilingual(BilingualOrder::OriginalFirst),
        );
        if matches!(format, SubtitleFormat::Ass) {
            assert!(rendered.contains("WrapStyle: 2"));
            assert_eq!(rendered.matches(r"\N").count(), 1);
            assert!(rows.iter().all(|row| rendered.contains(row)));
        } else {
            assert!(rendered.ends_with(&format!("{}\n{}\n\n", rows[0], rows[1])));
        }
    }
}
