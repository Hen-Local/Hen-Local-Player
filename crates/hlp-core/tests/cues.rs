use hlp_core::{cues::*, types::*};
use proptest::prelude::*;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
fn utterance(id: usize, start: Ms, end: Ms, text: &str) -> Utterance {
    Utterance {
        id: format!("u{id:06}"),
        start_ms: start,
        end_ms: end,
        regions: vec![SpeechRegion {
            start_ms: start,
            end_ms: end,
        }],
        status: UttStatus::Done,
        text: Some(text.into()),
        language: None,
        forced_language: None,
        asr_model: None,
    }
}
#[test]
fn split_over_pause_and_linger() {
    let sentence = "Here is a longer sentence.";
    let mut u = utterance(1, 1000, 7000, &format!("{sentence} {sentence}"));
    u.regions = vec![
        SpeechRegion {
            start_ms: 1000,
            end_ms: 3000,
        },
        SpeechRegion {
            start_ms: 5000,
            end_ms: 7000,
        },
    ];
    let cues = build_cues(&[u], &HashMap::new());
    assert_eq!(cues.len(), 2);
    assert_eq!((cues[0].start_ms, cues[0].end_ms), (1000, 3500));
    assert_eq!((cues[1].start_ms, cues[1].end_ms), (5000, 7500));
}
#[test]
fn preserve_words_graphemes_and_punctuation() {
    for (s, separate_words) in [
        ("เด็ก ๆ เดินไปเที่ยวที่แม่น้ำ", false),
        ("आज मौसम बहुत अच्छा है", true),
        ("مرحبا بكم في الحديقة", true),
        ("A family 👩‍👩‍👧‍👦 met us in the park", true),
        ("今日も桜の花が咲いています。", false),
    ] {
        let text = [s; 8].join(if separate_words { " " } else { "" });
        let utter = utterance(1, 0, 12000, &text);
        let cues = build_cues(&[utter], &HashMap::new());
        assert!(cues.len() > 1);
        let graphemes = |text: &str| {
            text.graphemes(true)
                .filter(|g| !g.trim().is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            cues.iter()
                .flat_map(|c| graphemes(&c.original))
                .collect::<Vec<_>>(),
            graphemes(&text)
        );
        for c in cues {
            assert!(display_cols(&c.original) <= 84);
            let wrapped = wrap_lines(&c.original);
            assert!(wrapped.lines().count() <= 2);
            assert_eq!(graphemes(&wrapped), graphemes(&c.original));
            assert!(wrapped.lines().all(|line| display_cols(line) <= 43));
        }
    }
    let c = build_cues(&[utterance(1, 0, 1500, "好！再见。")], &HashMap::new());
    assert_eq!(c[0].original, "好！再见。");
}
#[test]
fn length_bounds_pending_text_and_tiny_neighbor() {
    let mut old = utterance(1, 0, 200, "Yes.");
    old.status = UttStatus::Pending;
    let cues = build_cues(
        &[old, utterance(2, 5000, 17000, "Long sound.")],
        &HashMap::new(),
    );
    assert!(cues[0].stale);
    assert_eq!(cues[0].end_ms, 1500);
    assert_eq!(cues[1].end_ms, 12000);
    let tiny = utterance(
        1,
        0,
        600,
        "This opening statement is long. This following statement is long.",
    );
    let next = utterance(2, 640, 2000, "Next.");
    let cues = build_cues(&[tiny, next], &HashMap::new());
    assert_eq!(cues.len(), 2);
    assert!(cues[0].original.contains("long. This"));
}
#[test]
fn translations_follow_sentences_then_weights() {
    let originals = vec!["a".into(), "b".into()];
    assert_eq!(distribute(&originals, "自然。简单。"), ["自然。", "简单。"]);
    assert_eq!(distribute(&originals, "天蓝草绿"), ["天蓝", "草绿"]);
    assert_eq!(
        distribute(&["aaa".into(), "bbbbbb".into()], "one two three"),
        ["one", "two three"]
    );
    assert_eq!(distribute(&[], "text"), Vec::<String>::new());
    assert_eq!(distribute(&["one".into()], "甲。乙。"), ["甲。乙。"]);
}
proptest! {
    #[test] fn no_overlap(spans in prop::collection::vec((0u64..4000,1u64..12000),0..45)){
        let utts:Vec<_>=spans.into_iter().enumerate().map(|(i,(start,duration))|utterance(i,start,start+duration,"The first part is spoken here. The second part comes later.")).collect();
        let cues=build_cues(&utts,&HashMap::new());
        prop_assert!(cues.iter().all(|c|c.start_ms<c.end_ms&&c.end_ms-c.start_ms<=7000));
        prop_assert!(cues.windows(2).all(|w|w[0].end_ms<=w[1].start_ms));
    }
}

#[test]
fn expanding_translation_pages_at_matching_clauses() {
    let text = "我们先打开窗户，然后一起看看外面的天气。";
    let target =
        "First, let us open the windows, then we can take a look at the weather outside together.";
    let u = utterance(1, 900, 5600, text);
    let originals = build_cues(std::slice::from_ref(&u), &HashMap::new());
    assert_eq!(originals.len(), 1);
    let targets = HashMap::from([(u.id.clone(), target.to_owned())]);
    let pages = build_cues(&[u], &targets);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].original, "我们先打开窗户，");
    assert_eq!(
        pages[0].translation.as_deref(),
        Some("First, let us open the windows,")
    );
    assert_eq!(
        pages
            .iter()
            .map(|c| c.original.as_str())
            .collect::<String>(),
        text
    );
    assert_eq!(
        pages
            .iter()
            .map(|c| c.translation.as_deref().unwrap())
            .collect::<Vec<_>>()
            .join(" "),
        target
    );
    assert!(pages
        .iter()
        .all(|c| display_cols(c.translation.as_deref().unwrap()) <= 56));
    assert_eq!(pages[0].start_ms, 900);
    assert!(pages[0].end_ms <= pages[1].start_ms);
    assert_eq!(pages[0].id, originals[0].id);
}

#[test]
fn multilingual_paging_preserves_whole_words_and_indivisible_pairs() {
    let words = ["今天", "我们", "学习", "中文", "一起", "阅读"];
    let source = words.repeat(6).join("");
    let pages = build_cues(&[utterance(1, 0, 16000, &source)], &HashMap::new());
    assert!(pages.len() >= 3);
    assert_eq!(
        pages
            .iter()
            .map(|c| c.original.as_str())
            .collect::<String>(),
        source
    );
    assert!(pages
        .iter()
        .all(|c| words.iter().any(|w| c.original.ends_with(w))));
    let target =
        "A winter migration can take several months across this region, depending on the weather. "
            .repeat(3);
    let u = utterance(2, 0, 7000, "Migration");
    let cues = build_cues(&[u.clone()], &HashMap::from([(u.id, target.trim().into())]));
    // One source word has no timed alignment points. Preserve it and wrap the
    // corresponding translation; never alternate blank captions or half a word.
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].original, "Migration");
    assert_eq!(cues[0].translation.as_deref(), Some(target.trim()));
    let wrapped = wrap_lines(cues[0].translation.as_deref().unwrap());
    assert_eq!(wrapped.lines().collect::<Vec<_>>().join(" "), target.trim());
    assert!(wrapped.lines().all(|line| display_cols(line) <= 42));

    let punctuation = "！".repeat(50);
    assert!(wrap_lines(&punctuation)
        .lines()
        .all(|line| display_cols(line) <= 42));
}

#[test]
fn decimal_stays_whole_and_short_neighbors_cannot_recreate_oversized_cues() {
    let text = "The reading is 3.14 today.";
    let cues = build_cues(&[utterance(1, 0, 2000, text)], &HashMap::new());
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].original, text);

    let sentence = "Use the first full sentence here. ";
    let source = sentence.repeat(12);
    let translation = "这是完整的中文句子，需要保留全部字词。".repeat(12);
    let fast = utterance(1, 0, 2100, source.trim_end());
    let id = fast.id.clone();
    let cues = build_cues(&[fast], &HashMap::from([(id, translation.clone())]));
    assert!(cues.len() > 1);
    assert!(cues.iter().all(|c| display_cols(&c.original) <= 84
        && display_cols(c.translation.as_deref().unwrap()) <= 84));
    assert!(cues.windows(2).all(|w| w[0].end_ms <= w[1].start_ms));
    assert_eq!(
        cues.iter()
            .map(|c| c.original.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        source.trim()
    );
    assert_eq!(
        cues.iter()
            .map(|c| c.translation.as_deref().unwrap())
            .collect::<String>(),
        translation
    );
}
