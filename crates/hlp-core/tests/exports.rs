use hlp_core::{
    subs::{filename, render},
    types::*,
};
fn cues() -> Vec<Cue> {
    let first = Cue {
        id: "u000001-0".into(),
        utterance_id: "u000001".into(),
        original: "你好｛朋友｝！".into(),
        translation: Some("مرحبا & <أهلا>".into()),
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
    assert_eq!(tr, "1\n01:02:03,456 --> 01:02:05,995\nمرحبا & <أهلا>\n\n");
    let orig = render(&c, &SubtitleFormat::Srt, &SubtitleVariant::Original);
    assert!(!orig.contains("مرحبا"));
    let both = render(
        &c,
        &SubtitleFormat::Ass,
        &SubtitleVariant::Bilingual(BilingualOrder::OriginalFirst),
    );
    assert!(both.contains(r"{\rOrig}你好｛朋友｝！\N{\rTrans}"));
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
