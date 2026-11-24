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
fn assert_readable(cues: &[Cue], source: &str, target: &str) {
    let joined = |text: &str| generated_line(text).replace(' ', "");
    assert_eq!(
        cues.iter()
            .map(|c| c.original.replace(' ', ""))
            .collect::<String>(),
        joined(source)
    );
    assert_eq!(
        cues.iter()
            .filter_map(|c| c.translation.as_deref())
            .map(|s| s.replace(' ', ""))
            .collect::<String>(),
        joined(target)
    );
    assert!(cues.iter().all(|c| display_cols(&c.original) <= 72
        && display_cols(c.translation.as_deref().unwrap_or("")) <= 72));
    assert!(cues.windows(2).all(|w| w[0].end_ms <= w[1].start_ms));
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
fn preserve_words_and_graphemes_in_single_line_cues() {
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
            graphemes(&generated_line(&text))
        );
        for c in cues {
            assert!(display_cols(&c.original) <= 84);
            assert_eq!(c.original.lines().count(), 1);
        }
    }
    let c = build_cues(&[utterance(1, 0, 1500, "好！再见。")], &HashMap::new());
    assert_eq!(c[0].original, "好 再见");
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
        "The first statement is long. The next one is long.",
    );
    let next = utterance(2, 640, 2000, "Next.");
    let cues = build_cues(&[tiny, next], &HashMap::new());
    assert_eq!(cues.len(), 2);
    assert!(cues[0].original.contains("long The"));
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
    let target = "First, let us open the windows, then let's look at the weather outside.";
    let u = utterance(1, 900, 5600, text);
    let originals = build_cues(std::slice::from_ref(&u), &HashMap::new());
    assert_eq!(originals.len(), 1);
    let targets = HashMap::from([(u.id.clone(), target.to_owned())]);
    let pages = build_cues(&[u], &targets);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].original, "我们先打开窗户");
    assert_eq!(
        pages[0].translation.as_deref(),
        Some("First let us open the windows")
    );
    assert_eq!(
        pages
            .iter()
            .map(|c| c.original.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        generated_line(text)
    );
    assert_eq!(
        pages
            .iter()
            .map(|c| c.translation.as_deref().unwrap())
            .collect::<Vec<_>>()
            .join(" "),
        generated_line(target)
    );
    assert!(pages
        .iter()
        .all(|c| display_cols(c.translation.as_deref().unwrap()) <= 42));
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
    let translations = HashMap::from([(u.id.clone(), target.trim().into())]);
    let cues = build_cues(&[u], &translations);
    assert!(cues.len() >= 4);
    assert_eq!(cues[0].original, "Migration");
    assert!(cues[1..].iter().all(|c| c.original.is_empty()));
    assert_readable(&cues, "Migration", &target);
    assert_eq!(
        cues.iter()
            .filter_map(|c| c.translation.as_deref())
            .collect::<Vec<_>>()
            .join(" "),
        generated_line(&target)
    );
}

#[test]
fn decimal_stays_whole_and_short_neighbors_cannot_recreate_oversized_cues() {
    let text = "The reading is 3.14 today.";
    let cues = build_cues(&[utterance(1, 0, 2000, text)], &HashMap::new());
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].original, "The reading is 3.14 today");

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
        generated_line(&source)
    );
    assert_eq!(
        cues.iter()
            .map(|c| c.translation.as_deref().unwrap())
            .collect::<String>()
            .replace(' ', ""),
        generated_line(&translation).replace(' ', "")
    );
}

#[test]
fn generated_punctuation_becomes_spacing_after_alignment() {
    let source = " “你好”，\n世界！！  ";
    let target = "Well, yes! I’ll pay 1,000.25 at 10:30; 25% off.";
    let spoken = utterance(1, 0, 4000, source);
    let translations = HashMap::from([(spoken.id.clone(), target.into())]);
    let cues = build_cues(std::slice::from_ref(&spoken), &translations);
    assert_eq!(
        cues.iter()
            .map(|c| c.original.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        "你好 世界"
    );
    assert_eq!(
        cues.iter()
            .map(|c| c.translation.as_deref().unwrap())
            .collect::<Vec<_>>()
            .join(" "),
        "Well yes I’ll pay 1,000.25 at 10:30 25% off"
    );
    assert_eq!(spoken.text.as_deref(), Some(source));
    assert_eq!(translations[&spoken.id], target);
    assert_eq!(
        generated_line("  “Hybrid”  ——  real-time\r\ncaption… "),
        "Hybrid real-time caption"
    );
    assert!(generated_line(" ？！—\n，").is_empty());

    let punctuation = utterance(2, 0, 1300, "……，。");
    assert!(build_cues(std::slice::from_ref(&punctuation), &HashMap::new()).is_empty());
    let target_only = build_cues(
        std::slice::from_ref(&punctuation),
        &HashMap::from([(punctuation.id.clone(), "Hello!".to_owned())]),
    );
    assert_eq!(target_only.len(), 1);
    assert_eq!(target_only[0].translation.as_deref(), Some("Hello"));
}

#[test]
fn streaming_updates_match_full_layout_when_neighbors_and_languages_change() {
    let mut builder = hlp_core::cues::CueBuilder::default();
    let mut rows = vec![
        utterance(1, 0, 2600, "The first line"),
        utterance(3, 4000, 5100, "The final line"),
    ];
    let mut translations = HashMap::from([(rows[0].id.clone(), "第一句话".to_owned())]);
    let mut check = |rows: &[Utterance], translations: &HashMap<String, String>| {
        let actual = builder.update(
            rows.iter()
                .map(|row| (row, translations.get(&row.id).map_or("", String::as_str))),
        );
        assert_eq!(actual, build_cues(rows, translations));
    };
    check(&rows, &translations);
    rows.push(utterance(
        2,
        1500,
        4700,
        "An overlapping caption that was recognized later",
    ));
    check(&rows, &translations);
    translations.insert(
        rows[0].id.clone(),
        "Une première ligne beaucoup plus longue que la traduction précédente".into(),
    );
    rows[1].status = UttStatus::Pending;
    check(&rows, &translations);
    rows.remove(2);
    rows[0].text = None;
    check(&rows, &translations);
    rows[0].text = Some("A corrected line".into());
    rows[1].language = Some(LangCode("es".into()));
    check(&rows, &HashMap::new());
    check(&[], &HashMap::new());
}

#[test]
fn cached_reordered_english_and_chinese_keep_complete_meanings() {
    let cases = [
        (
            41158,
            44678,
            "今天，我们就来讲三个高技能人才的亲身经历。",
            "Today, we will share the personal experiences of three highly skilled professionals.",
        ),
        (
            45542,
            48422,
            "的故事啊，可以说是把这个问题讲透了。",
            "The story can be said to have explained this issue thoroughly.",
        ),
    ];
    for (start, end, s, t) in cases {
        let u = utterance(1, start, end, s);
        let cues = build_cues(
            std::slice::from_ref(&u),
            &HashMap::from([(u.id.clone(), t.into())]),
        );
        assert_readable(&cues, s, t);
        if start == 41158 {
            assert!(cues.iter().any(|cue| cue.original.contains("高技能人才")
                && cue
                    .translation
                    .as_deref()
                    .unwrap()
                    .contains("highly skilled professionals")));
            assert!(cues
                .iter()
                .all(|c| !c.translation.as_deref().unwrap().ends_with(" the")));
        } else {
            assert_eq!(cues.len(), 1);
        }
        assert_eq!(cues[0].start_ms, start);
        assert!(cues.last().unwrap().end_ms >= end);
        assert!(translation_plan(s, t).is_none());
    }
}

#[test]
fn cached_mac_and_steamos_clauses_cover_their_speech() {
    let first_s = "Instead of being a compatibility layer that translates Windows API calls into something that your Mac can handle natively,";
    let last_s = " it uses virtualization to run a native ARM version of SteamOS.";
    let first_t = "它并非那种将Windows API调用转换为mac电脑能够原生处理的格式的兼容层，";
    let last_t = "而是通过虚拟化技术来运行原生ARM版本的SteamOS。";
    let mut u = utterance(11, 102694, 114438, &[first_s, last_s].concat());
    u.regions = vec![
        SpeechRegion {
            start_ms: 102694,
            end_ms: 109350,
        },
        SpeechRegion {
            start_ms: 109702,
            end_ms: 114438,
        },
    ];
    let cues = build_cues(
        std::slice::from_ref(&u),
        &HashMap::from([(u.id.clone(), [first_t, last_t].concat())]),
    );
    assert_readable(
        &cues,
        &[first_s, last_s].concat(),
        &[first_t, last_t].concat(),
    );
    for name in ["windows api", "mac", "steamos"] {
        assert!(cues.iter().any(|c| c.original.to_lowercase().contains(name)
            && c.translation
                .as_deref()
                .unwrap()
                .to_lowercase()
                .contains(name)));
    }
    assert!(cues.iter().all(|c| !c.original.ends_with("that your")));
    assert_eq!(cues[0].start_ms, 102694);
    assert!(cues.last().unwrap().end_ms >= 114438);

    let u = utterance(1, 2000, 15000, first_s);
    let fixed = build_cues(
        std::slice::from_ref(&u),
        &HashMap::from([(u.id.clone(), first_t.into())]),
    );
    assert_readable(&fixed, first_s, first_t);
    assert!(fixed.last().unwrap().end_ms >= 15000);
}

#[test]
fn joined_sentences_and_missing_commas_use_existing_clause_boundaries() {
    let s = "我们把文件准备好然后再发给本地的客户。";
    let t = "We prepare the documents first, then send them to the local customers.";
    let u = utterance(1, 0, 5500, s);
    let cues = build_cues(
        std::slice::from_ref(&u),
        &HashMap::from([(u.id.clone(), t.into())]),
    );
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[1].original, "然后再发给本地的客户");
    assert_eq!(
        cues[1].translation.as_deref(),
        Some("then send them to the local customers")
    );

    let s = "从一身才华无处施展，到最后找到机会，中间到底遇到了多少制度上的坎儿？";
    let t = "From having great talent that has no place to be utilized, to finally finding an opportunity – how many institutional obstacles did he have to overcome along the way?";
    let u = utterance(1, 49158, 55430, s);
    let cues = build_cues(
        std::slice::from_ref(&u),
        &HashMap::from([(u.id.clone(), t.into())]),
    );
    assert_readable(&cues, s, t);
    assert_eq!(cues[1].original, "到最后找到机会");
    assert_eq!(
        cues[1].translation.as_deref(),
        Some("to finally finding an opportunity")
    );
}

#[test]
fn fresh_translation_plan_preserves_bytes_and_reuses_short_pairs() {
    let s = "  She went home. Farmers prepare the fields for planting every year before the rains arrive and their children help them with heavy equipment from the nearby town during the summer.  ";
    let t = "她回家了。每年雨季来临之前农民都会用从邻近城镇运来的重型设备准备耕作而他们的孩子也会来帮忙。  ";
    let plan = translation_plan(s, t).unwrap();
    assert_eq!(plan.iter().map(|(part, _)| *part).collect::<String>(), s);
    assert_eq!(plan[0].0.trim(), "She went home.");
    assert_eq!(plan[0].1.unwrap().trim(), "她回家了。");
    assert!(plan.len() >= 3);
    for &(source, existing) in &plan[1..] {
        assert!(existing.is_none());
        assert!(source.split_whitespace().count() >= 2);
        assert_eq!(source.trim().lines().count(), 1);
    }
    assert!(translation_plan("Migration", &"A very long translation. ".repeat(10)).is_none());
    assert!(translation_plan("Brief source", "简短的译文").is_none());
    assert!(translation_plan(s, "").is_none());
}

#[test]
fn stored_alignment_round_trip_and_invalid_offsets_use_safe_fallback() {
    let left = "  Farmers prepare the fields";
    let right = " before the summer rains arrive.  ";
    let t1 = "农民们整地";
    let t2 = "，赶在夏季雨水来临之前。 ";
    let s = [left, right].concat();
    let t = [t1, t2].concat();
    let u = utterance(1, 0, 6000, &s);
    let first = AlignedPhrase {
        source_end: left.len(),
        target_end: t1.len(),
    };
    let end = AlignedPhrase {
        source_end: s.len(),
        target_end: t.len(),
    };
    let alignment = vec![first.clone(), end.clone()];
    let json = serde_json::to_value(&alignment).unwrap();
    assert_eq!(json[0]["sourceEnd"], left.len());
    let reopen: Vec<AlignedPhrase> = serde_json::from_value(json).unwrap();
    let mut builder = CueBuilder::default();
    let valid = builder.update_aligned([(&u, t.as_str(), reopen.as_slice())]);
    assert_eq!(valid.len(), 2);
    assert_eq!(valid[0].translation.as_deref(), Some(t1));
    assert_eq!(valid[1].original, generated_line(right));
    assert_eq!(
        valid[1].translation.as_deref(),
        Some(generated_line(t2).as_str())
    );
    let fallback = build_cues(
        std::slice::from_ref(&u),
        &HashMap::from([(u.id.clone(), t.clone())]),
    );
    assert_eq!(fallback.len(), 1);
    for bad in [
        vec![],
        vec![first.clone()],
        vec![first.clone(), first, end.clone()],
        vec![
            AlignedPhrase {
                source_end: 1,
                target_end: 1,
            },
            end,
        ],
    ] {
        let actual = builder.update_aligned([(&u, t.as_str(), bad.as_slice())]);
        assert_eq!(actual, fallback);
    }
    // An alignment-only cache update must also invalidate in-memory pages.
    assert_eq!(
        builder.update_aligned([(&u, t.as_str(), reopen.as_slice())]),
        valid
    );
}

#[test]
fn phrase_hints_map_to_the_verbatim_english_and_chinese_translations() {
    let source = "先打开窗户再走进花园";
    let whole = "  Open the bright window, then walk into the quiet garden!  ";
    let spans = [
        ("先打开窗户", "Open a bright window"),
        ("再走进花园", "Then walk into the quiet green garden"),
    ];
    let alignment = align_translated_phrases(source, whole, &spans).unwrap();
    assert_eq!(alignment[0].source_end, spans[0].0.len());
    let boundary = alignment[0].target_end;
    assert_eq!(&whole[..boundary], "  Open the bright window,");
    assert_eq!(
        &whole[boundary..alignment[1].target_end],
        " then walk into the quiet garden!  "
    );
    assert_eq!(alignment[1].source_end, source.len());

    let source = "Open the window then walk into the garden";
    let whole = "他们先打开窗户，然后走进花园。";
    let spans = [
        ("Open the window", "他们先打开窗户。"),
        (" then walk into the garden", "然后一起走进花园"),
    ];
    let alignment = align_translated_phrases(source, whole, &spans).unwrap();
    assert_eq!(&whole[..alignment[0].target_end], "他们先打开窗户，");
    assert_eq!(
        &whole[alignment[0].target_end..alignment[1].target_end],
        "然后走进花园。"
    );
}

#[test]
fn overlapping_echoes_reordering_and_ambiguous_seams_fall_back_to_whole() {
    let source = "First part then second part";
    for (whole, a, b) in [
        (
            "Close the green window. Open the large red door.",
            "Close the green window. Open the large red door.",
            "Open the large red door.",
        ),
        (
            "Close the green window. Open the large red door.",
            "Open the large red door.",
            "Close the green window.",
        ),
        (
            "Close the green window. Open the large red door.",
            "Close this farm gate.",
            "Go out into the garden.",
        ),
        (
            "First read this report and take a break.",
            "First read this report",
            "take a break",
        ),
        ("打开窗户以后就出发。", "打开窗户以后就出发", "以后就出发"),
        ("今天我们学习中文语法。", "今天我们学", "习中文语法"),
        ("We can re-enter the room.", "We can re", "enter the room"),
    ] {
        let spans = [("First part", a), (" then second part", b)];
        assert!(
            align_translated_phrases(source, whole, &spans).is_none(),
            "{whole}"
        );
    }
    let oversized = "Walk forward ".repeat(15);
    let whole = format!("{oversized}then turn left.");
    assert!(align_translated_phrases(
        source,
        &whole,
        &[
            ("First part", &oversized),
            (" then second part", "then turn left")
        ]
    )
    .is_none());
    assert!(
        align_translated_phrases("wrong source", "One two", &[("a", "One"), ("b", "two")])
            .is_none()
    );
}

#[test]
fn only_original_punctuation_resolves_a_one_word_alignment_tie() {
    let source = "先拆开设备然后抬起盖子";
    let spans = [
        ("先拆开设备", "We unpack the heavy equipment."),
        ("然后抬起盖子", "Carefully lift the cover."),
    ];
    let whole = "We unpack the heavy equipment, then carefully lift the cover.";
    let cut = align_translated_phrases(source, whole, &spans).unwrap()[0].target_end;
    assert_eq!(&whole[..cut], "We unpack the heavy equipment,");
    assert_eq!(&whole[cut..], " then carefully lift the cover.");
    assert!(align_translated_phrases(source, &whole.replace(',', ""), &spans).is_none());
    assert!(align_translated_phrases(source, &whole.replace("then", "and then"), &spans).is_none());
}

#[test]
fn long_cached_question_keeps_names_and_reordered_clauses_together() {
    let s = "那问题就来了，为什么像纳蒂亚、雷扎和娜塔莉亚这样的人才，在一个明明急需他们技能的国家，反而会遇到这么多坎儿呢？";
    let t = "So the question arises: why do talents like Natiya, Reza, and Natalia face so many obstacles in a country that desperately needs their skills?";
    let u = utterance(21, 164070, 173640, s);
    let full = [AlignedPhrase {
        source_end: s.len(),
        target_end: t.len(),
    }];
    for alignment in [&full[..], &[]] {
        let mut builder = CueBuilder::default();
        let cues = builder.update_aligned([(&u, t, alignment)]);
        assert_readable(&cues, s, t);
        assert_eq!(cues.len(), 3);
        assert_eq!(
            cues[0].translation.as_deref(),
            Some("So the question arises")
        );
        assert_eq!(
            cues[1].translation.as_deref(),
            Some("why do talents like Natiya Reza and Natalia")
        );
        assert!(cues[1].original.contains("纳蒂亚 雷扎和娜塔莉亚"));
        assert!(cues[2].original.starts_with("在一个"));
        assert!(cues[2]
            .translation
            .as_deref()
            .unwrap()
            .ends_with("needs their skills"));
        assert_eq!(cues[0].start_ms, 164070);
        assert!(cues.last().unwrap().end_ms >= 173640);
    }
}
