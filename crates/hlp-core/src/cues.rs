mod alignment;
pub use alignment::align_translated_phrases;

use crate::types::*;
use icu_properties::{
    props::{GeneralCategory, GeneralCategoryGroup},
    CodePointMapData,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Aim for a single line in each language. Fast speech can share a longer
// timed cue when an individual phrase would otherwise flash for under 500 ms.
const PAGE_COLS: usize = 42;
const RETRANSLATE_COLS: usize = 120;
const MAX_CUE_COLS: usize = 2 * PAGE_COLS;
const MIN_CUE_MS: Ms = 1000;
const MAX_CUE_MS: Ms = 7000;
const MIN_GAP_MS: Ms = 80;
const LINGER_MS: Ms = 500;
const READ_COLS_PER_SEC: Ms = 20;
const END: &str = ".!?。！？…؟।";
const CLAUSE: &str = ",;:，；：、—–؛،";
// A prefix such as “Today,” is too short to read as a page on its own.
const MIN_PHRASE_COLS: usize = 8;
const CLOSE: &str = "”’'\"）)]}】》」』»";

pub fn display_cols(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Break {
    Clause,
    Word,
    Grapheme,
}
struct Boundary {
    byte: usize,
    cols: usize,
    kind: Break,
}
fn unspaced(ch: char) -> bool {
    matches!(ch as u32,0x0e00..=0x0eff|0x0f00..=0x109f|0x1780..=0x17ff|0x3040..=0x30ff|0x3400..=0x9fff|0xf900..=0xfaff|0x20000..=0x323af)
}
fn boundaries(text: &str) -> Vec<Boundary> {
    let words = if text.chars().any(unspaced) {
        crate::word_boundaries::word_ends(text)
    } else {
        Vec::new()
    };
    let mut result = Vec::new();
    let mut cols = 0;
    let mut graphemes = text.grapheme_indices(true).peekable();
    while let Some((_, g)) = graphemes.next() {
        cols += display_cols(g);
        let Some(&(byte, next)) = graphemes.peek() else {
            break;
        };
        let next_ch = next.chars().next().unwrap();
        if END.contains(next_ch) || CLAUSE.contains(next_ch) || CLOSE.contains(next_ch) {
            continue;
        }
        let prev = g.chars().last().unwrap();
        if prev.is_whitespace() {
            continue;
        }
        // Never treat a numeric separator as a clause, e.g. 10:30 or 1,000.
        let before = text[..byte - prev.len_utf8()].chars().next_back();
        let internal = lexical_mark(prev, before, Some(next_ch))
            || (prev == '.' && next_ch.is_ascii_alphanumeric());
        let kind = if !internal && (END.contains(prev) || CLAUSE.contains(prev)) {
            Break::Clause
        } else if next_ch.is_whitespace() || words.binary_search(&byte).is_ok() {
            Break::Word
        } else {
            Break::Grapheme
        };
        result.push(Boundary { byte, cols, kind });
    }
    result
}

// Common clause introducers also work when one recognizer or translator
// omits the comma. Match whole English words, and only test Chinese prefixes
// at dictionary word boundaries.
fn connector(right: &str) -> Option<usize> {
    const GROUPS: &[(&[&str], &[&str])] = &[
        (
            &["but", "however", "instead", "whereas"],
            &["但是", "可是", "不过", "然而", "而是", "但"],
        ),
        (&["then", "next", "afterwards"], &["然后", "接着", "随后"]),
        (&["because", "since"], &["因为", "由于"]),
        (&["therefore", "so", "thus"], &["因此", "所以"]),
        (&["if", "unless"], &["如果", "假如", "除非"]),
        (&["although", "though"], &["虽然", "尽管"]),
        (&["finally", "ultimately"], &["最后", "最终"]),
    ];
    let right = right.trim_start();
    let first = right
        .split(|c: char| !c.is_ascii_alphabetic())
        .next()
        .unwrap_or("");
    GROUPS.iter().position(|(en, zh)| {
        en.iter().any(|w| first.eq_ignore_ascii_case(w)) || zh.iter().any(|w| right.starts_with(w))
    })
}

fn dangling_word(left: &str, right: &str) -> bool {
    let last = left.split_whitespace().next_back().unwrap_or("");
    [
        "a", "an", "the", "that", "my", "your", "our", "their", "to", "of", "for", "with", "and",
        "or", "in", "at", "from", "into", "is", "are", "can", "but", "will",
    ]
    .iter()
    .any(|w| last.eq_ignore_ascii_case(w))
        || [
            "的", "就", "把", "被", "将", "个", "很", "高", "这", "那", "到", "但", "和", "从",
            "与",
        ]
        .iter()
        .any(|w| left.ends_with(w))
        || ["的", "地", "得", "了", "吗", "呢"]
            .iter()
            .any(|w| right.trim_start().starts_with(w))
}

// Without a translation, prefer clauses and complete word groups. A modest
// overrun is more readable than separating a determiner from its noun.
fn source_cut(text: &str, for_translation: bool) -> Option<usize> {
    let text_width = display_cols(text);
    let count = text_width.div_ceil(PAGE_COLS).max(2);
    let target = text_width * (count / 2) / count;
    let candidates = boundaries(text);
    let words = candidates.iter().any(|b| b.kind != Break::Grapheme);
    if !words && (for_translation || text_width <= MAX_CUE_COLS) {
        return None;
    }
    candidates
        .iter()
        .filter(|b| !words || b.kind != Break::Grapheme)
        .filter(|b| b.cols.min(text_width.saturating_sub(b.cols)) >= MIN_PHRASE_COLS)
        .filter(|b| {
            let enough = |part: &str| {
                part.split_whitespace()
                    .filter(|w| w.chars().any(char::is_alphanumeric))
                    .take(2)
                    .count()
                    == 2
                    || part.chars().filter(|&c| unspaced(c)).take(4).count() == 4
            };
            !for_translation || (enough(&text[..b.byte]) && enough(&text[b.byte..]))
        })
        .min_by_key(|b| {
            let (left, right) = text.split_at(b.byte);
            let distance = b.cols.abs_diff(target);
            let first = right.split_whitespace().next().unwrap_or("");
            let word_group = [
                "that",
                "which",
                "when",
                "where",
                "into",
                "with",
                "without",
                "before",
                "after",
                "until",
                "including",
                "about",
            ]
            .iter()
            .any(|w| first.eq_ignore_ascii_case(w));
            let clause = b.kind == Break::Clause || connector(right).is_some();
            (
                distance.saturating_sub(PAGE_COLS / 2),
                dangling_word(left, right),
                !clause,
                !word_group,
                distance,
            )
        })
        .map(|b| b.byte)
}

// Keep the exact slices (including inter-word spaces) until the final cue
// boundaries are fixed. That lets short neighbors join without changing text.
fn aligned_translation<'a>(originals: &[&str], translation: &'a str) -> Vec<&'a str> {
    if originals.len() <= 1 {
        return originals.iter().map(|_| translation).collect();
    }
    let sentences: Vec<_> = translation.split_sentence_bounds().collect();
    if sentences.len() == originals.len() {
        return sentences;
    }
    let mut atoms = boundaries(translation);
    atoms.retain(|b| b.kind != Break::Grapheme);
    let total_cols = display_cols(translation);
    atoms.push(Boundary {
        byte: translation.len(),
        cols: total_cols,
        kind: Break::Word,
    });
    let weights: Vec<_> = originals
        .iter()
        .map(|o| display_cols(o.trim()).max(1))
        .collect();
    let total_weight: usize = weights.iter().sum();
    let mut result = Vec::new();
    let (mut accum, mut first, mut start) = (0, 0, 0);
    for (i, weight) in weights.iter().enumerate().take(weights.len() - 1) {
        accum += weight;
        let available = atoms.len() - first;
        let remaining = weights.len() - i;
        if available == 0 {
            result.push("");
            continue;
        }
        let last = if available >= remaining {
            atoms.len() - (remaining - 1)
        } else {
            atoms.len()
        };
        let target = (total_cols * accum + total_weight / 2) / total_weight;
        let tolerance = total_cols / originals.len() * 15 / 100;
        let (relative, b) = atoms[first..last]
            .iter()
            .enumerate()
            .min_by_key(|(_, b)| {
                let dist = b.cols.abs_diff(target);
                (dist.saturating_sub(tolerance), b.kind, dist)
            })
            .unwrap();
        result.push(&translation[start..b.byte]);
        start = b.byte;
        first += relative + 1;
    }
    result.push(&translation[start..]);
    result
}

/// Fixed-count text allocation for clients with a pre-existing list of source
/// lines. The player builds both sides together using sentence and clause pairs.
pub fn distribute(originals: &[String], translation: &str) -> Vec<String> {
    let originals: Vec<_> = originals.iter().map(String::as_str).collect();
    aligned_translation(&originals, translation.trim())
        .into_iter()
        .map(|s| s.trim().to_owned())
        .collect()
}

struct PhraseBreak {
    byte: usize,
    cols: usize,
    punctuation: bool,
    sentence: bool,
    connector: Option<usize>,
}

fn phrase_breaks(text: &str) -> Vec<PhraseBreak> {
    boundaries(text)
        .into_iter()
        .filter(|b| b.kind != Break::Grapheme)
        .filter_map(|b| {
            let punctuation = b.kind == Break::Clause;
            let connector = connector(&text[b.byte..]);
            if !punctuation && dangling_word(&text[..b.byte], &text[b.byte..]) {
                return None;
            }
            (punctuation || connector.is_some()).then_some(PhraseBreak {
                byte: b.byte,
                cols: b.cols,
                punctuation,
                sentence: punctuation && text[..b.byte].ends_with(|ch| END.contains(ch)),
                connector,
            })
        })
        .collect()
}

/// A cached translation belongs to its entire utterance. Its words can be in
/// a different order, so only cut at corresponding clauses. An unmatched
/// pair stays together, even when the longer language exceeds the width goal.
fn clause_pair(source: &str, target: &str) -> Option<(usize, usize)> {
    let sources = phrase_breaks(source);
    let targets = phrase_breaks(target);
    let sw = display_cols(source);
    let tw = display_cols(target);
    let mut best = None;
    let mut consider = |s: &PhraseBreak, t: &PhraseBreak, matches_connector: bool| {
        // Avoid orphaning an introductory word or pairing grossly different
        // positions merely because the two sentences have the same comma count.
        let sp = 100 * s.cols / sw;
        let tp = 100 * t.cols / tw;
        if s.cols
            .min(sw.saturating_sub(s.cols))
            .min(t.cols)
            .min(tw.saturating_sub(t.cols))
            < MIN_PHRASE_COLS
            || sp.abs_diff(tp) > 30
            || matches!((s.connector, t.connector), (Some(a), Some(b)) if a != b)
        {
            return;
        }
        let quality = (!matches_connector, sp.abs_diff(50).max(tp.abs_diff(50)));
        if best.is_none_or(|(score, _)| quality < score) {
            best = Some((quality, (s.byte, t.byte)));
        }
    };

    let sc: Vec<_> = sources.iter().filter(|b| b.punctuation).collect();
    let tc: Vec<_> = targets.iter().filter(|b| b.punctuation).collect();
    if sc.len() == tc.len() {
        for (s, t) in sc.into_iter().zip(tc) {
            consider(s, t, false);
        }
    }
    // A translator may join two sentences or promote a clause to a sentence.
    // Recover unambiguous nearby sentence endings in that case. Cuts must
    // be actual punctuation on both sides and each must be the other's sole
    // candidate within this narrow window, not an arbitrary nearby word.
    let near = |s: &PhraseBreak, t: &PhraseBreak| {
        s.punctuation
            && t.punctuation
            && (s.sentence || t.sentence)
            && (100 * s.cols / sw).abs_diff(100 * t.cols / tw) <= 8
    };
    for s in sources.iter().filter(|b| b.punctuation) {
        let mut matching = targets.iter().filter(|t| near(s, t));
        if let (Some(t), None) = (matching.next(), matching.next()) {
            if sources.iter().filter(|b| near(b, t)).count() == 1 {
                consider(s, t, false);
            }
        }
    }
    for s in sources.iter().filter(|s| s.connector.is_some()) {
        let mut matching = targets.iter().filter(|t| t.connector == s.connector);
        if let (Some(t), None) = (matching.next(), matching.next()) {
            if sources
                .iter()
                .filter(|b| b.connector == s.connector)
                .count()
                == 1
            {
                consider(s, t, true);
            }
        }
    }
    best.map(|(_, pair)| pair)
}

fn paired_pages<'a>(original: &'a str, translation: &'a str) -> Vec<(&'a str, &'a str)> {
    let sources: Vec<_> = original.split_sentence_bounds().collect();
    let targets: Vec<_> = translation.split_sentence_bounds().collect();
    let mut pending: Vec<_> = if translation.is_empty() {
        sources.into_iter().map(|s| (s, "")).rev().collect()
    } else if sources.len() == targets.len() {
        sources.into_iter().zip(targets).rev().collect()
    } else {
        vec![(original, translation)]
    };
    let mut pages = Vec::new();
    while let Some((source, target)) = pending.pop() {
        if display_cols(source.trim()).max(display_cols(target.trim())) > PAGE_COLS {
            let cut = if target.is_empty() {
                source_cut(source, false).map(|i| (i, 0))
            } else {
                clause_pair(source, target)
            };
            if let Some((s, t)) = cut {
                pending.push((&source[s..], &target[t..]));
                pending.push((&source[..s], &target[..t]));
                continue;
            }
        }
        pages.push((source, target));
    }
    pages
}

/// Cumulative UTF-8 offsets into the ASR text and its stored translation.
/// The last span includes the end of both strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlignedPhrase {
    pub source_end: usize,
    pub target_end: usize,
}

/// Reuse the translation of each aligned clause that already fits. Only the
/// source slices marked None need another translation request. Slices retain
/// interword spaces so the caller can record exact offsets in the cache.
pub fn translation_plan<'a>(
    source: &'a str,
    translation: &'a str,
) -> Option<Vec<(&'a str, Option<&'a str>)>> {
    if display_cols(source).max(display_cols(translation)) <= RETRANSLATE_COLS
        || translation.trim().is_empty()
    {
        return None;
    }
    let mut plan = Vec::new();
    let mut changed = false;
    for (s, t) in paired_pages(source, translation) {
        let width = display_cols(s.trim()).max(display_cols(t.trim()));
        if width > RETRANSLATE_COLS {
            // A single word (including an identifier or proper name) cannot
            // support multiple independent translations. Keep that pair intact.
            if let Some(i) = source_cut(s, true) {
                let mut remaining = vec![&s[i..], &s[..i]];
                while let Some(piece) = remaining.pop() {
                    // Only split further when the source phrase itself is long.
                    // Each translation request must contain a meaningful group.
                    if display_cols(piece.trim()) > MAX_CUE_COLS {
                        if let Some(i) = source_cut(piece, true) {
                            remaining.push(&piece[i..]);
                            remaining.push(&piece[..i]);
                            continue;
                        }
                    }
                    plan.push((piece, None));
                }
                changed = true;
                continue;
            }
        }
        plan.push((s, Some(t)));
    }
    changed.then_some(plan)
}

fn stored_pages<'a>(
    source: &'a str,
    target: &'a str,
    alignment: &[AlignedPhrase],
) -> Option<Vec<(&'a str, &'a str)>> {
    let last = alignment.last()?;
    if last.source_end != source.len() || last.target_end != target.len() {
        return None;
    }
    let mut pages = Vec::with_capacity(alignment.len());
    let (mut start_s, mut start_t) = (0, 0);
    for p in alignment {
        let s = source.get(start_s..p.source_end)?;
        let t = target.get(start_t..p.target_end)?;
        if s.trim().is_empty() || t.trim().is_empty() {
            return None;
        }
        pages.push((s, t));
        (start_s, start_t) = (p.source_end, p.target_end);
    }
    Some(pages)
}

// These marks carry information inside a word, identifier or number.
fn lexical_mark(mark: char, before: Option<char>, after: Option<char>) -> bool {
    let word_before = before.is_some_and(char::is_alphanumeric);
    let word_after = after.is_some_and(char::is_alphanumeric);
    let number_before = before.is_some_and(char::is_numeric);
    let number_after = after.is_some_and(char::is_numeric);
    match mark {
        '\'' | '’' | '_' | '&' | '@' => word_before && word_after,
        '-' | '‐' | '‑' => (word_before && word_after) || number_after,
        '.' | ',' | ':' | '/' | '٫' | '٬' => number_before && number_after,
        '%' | '％' | '‰' | '‱' => number_before,
        '#' => word_before || word_after,
        _ => false,
    }
}

/// Format a single language after the original punctuation has guided timing.
/// Never write this presentation text back into the transcription or MT cache.
pub fn generated_line(text: &str) -> String {
    let categories = CodePointMapData::<GeneralCategory>::new();
    let mut line = String::with_capacity(text.len());
    let mut space = false;
    let mut before = None;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        let punctuation = GeneralCategoryGroup::Punctuation.contains(categories.get(ch));
        if ch.is_whitespace() || (punctuation && !lexical_mark(ch, before, chars.peek().copied())) {
            space = !line.is_empty();
        } else {
            if space {
                line.push(' ');
                space = false;
            }
            line.push(ch);
        }
        before = Some(ch);
    }
    line
}

fn speech_time(regions: &[SpeechRegion], mut offset: Ms, is_start: bool) -> Ms {
    for region in regions {
        let length = region.end_ms - region.start_ms;
        if offset < length || (offset == length && !is_start) {
            return region.start_ms + offset;
        }
        offset -= length;
    }
    regions.last().map_or(0, |r| r.end_ms)
}

fn utterance_cues(utt: &Utterance, translation: &str, alignment: &[AlignedPhrase]) -> Vec<Cue> {
    let Some(text) = utt.text.as_deref().filter(|s| !s.trim().is_empty()) else {
        return Vec::new();
    };
    // Pending lines retain their previous transcript while a language override runs.
    if !matches!(utt.status, UttStatus::Done | UttStatus::Pending) {
        return Vec::new();
    }
    let mut regions: Vec<_> = utt
        .regions
        .iter()
        .filter(|r| r.start_ms < r.end_ms)
        .cloned()
        .collect();
    if regions.is_empty() {
        if utt.start_ms >= utt.end_ms {
            return Vec::new();
        }
        regions.push(SpeechRegion {
            start_ms: utt.start_ms,
            end_ms: utt.end_ms,
        });
    }
    let total: Ms = regions.iter().map(|r| r.end_ms - r.start_ms).sum();
    let pages = stored_pages(text, translation, alignment)
        .unwrap_or_else(|| paired_pages(text.trim(), translation.trim()));
    let weights: Vec<_> = pages
        .iter()
        .map(|(s, _)| display_cols(s.trim()).max(1))
        .collect();
    let cols: usize = weights.iter().sum();
    let mut cum = 0;
    let mut cues: Vec<Cue> = Vec::new();
    for ((source, target), weight) in pages.into_iter().zip(weights) {
        let offset = |w: usize| (total as u128 * w as u128 / cols as u128) as Ms;
        let start = speech_time(&regions, offset(cum), true);
        cum += weight;
        let end = speech_time(&regions, offset(cum), false);
        if let Some(last) = cues.last_mut() {
            if cue_width(last) < 20
                && display_cols(source.trim()).max(display_cols(target.trim())) < 20
            {
                last.original.push_str(source);
                if let Some(tr) = &mut last.translation {
                    tr.push_str(target);
                }
                last.end_ms = end;
                continue;
            }
        }
        cues.push(Cue {
            id: format!("{}-{}", utt.id, cues.len()),
            utterance_id: utt.id.clone(),
            start_ms: start,
            end_ms: end,
            original: source.to_owned(),
            translation: (!target.is_empty()).then(|| target.to_owned()),
            lang: utt.language.clone(),
            stale: utt.status == UttStatus::Pending,
        });
    }
    cues
}

fn cue_width(cue: &Cue) -> usize {
    display_cols(cue.original.trim()).max(display_cols(
        cue.translation.as_deref().unwrap_or("").trim(),
    ))
}

fn finish_timing(cues: &mut [Cue]) {
    // Stable order also resolves coincident starts deterministically (1 ms each
    // in the pathological case of multiple utterances with identical onset).
    let mut first_free = 0;
    for cue in cues.iter_mut() {
        cue.start_ms = cue.start_ms.max(first_free);
        first_free = cue.start_ms.saturating_add(1);
    }
    for i in 0..cues.len() {
        let next = cues.get(i + 1).map(|c| c.start_ms);
        let cue = &mut cues[i];
        let reading = 1000 * cue_width(cue) as Ms / READ_COLS_PER_SEC;
        let needed = cue.start_ms.saturating_add(MIN_CUE_MS.max(reading));
        let limit = next
            .unwrap_or(Ms::MAX)
            .saturating_sub(MIN_GAP_MS)
            .max(cue.start_ms.saturating_add(1));
        // An indivisible bilingual clause can cover more than seven seconds
        // of actual speech. Cap any added linger, but keep its spoken end.
        let end_cap = cue.start_ms.saturating_add(MAX_CUE_MS);
        let end_cap = if cue_width(cue) > PAGE_COLS {
            end_cap.max(cue.end_ms)
        } else {
            end_cap
        };
        cue.end_ms = cue
            .end_ms
            .max(needed)
            .saturating_add(LINGER_MS)
            .min(end_cap)
            .min(limit);
    }
}

pub fn build_cues(utterances: &[Utterance], translations: &HashMap<String, String>) -> Vec<Cue> {
    finish_cues(
        utterances
            .iter()
            .flat_map(|u| {
                utterance_cues(u, translations.get(&u.id).map_or("", String::as_str), &[])
            })
            .collect(),
    )
}

#[derive(Default)]
pub struct CueBuilder {
    pages: HashMap<String, CachedPages>,
}
struct CachedPages {
    utterance: Utterance,
    translation: String,
    alignment: Vec<AlignedPhrase>,
    cues: Vec<Cue>,
}
impl CueBuilder {
    /// Reuse phrase segmentation between successive recognition and translation batches.
    /// Timing and merges still see every neighbor, including overlapping or out-of-order speech.
    pub fn update<'a>(
        &mut self,
        rows: impl IntoIterator<Item = (&'a Utterance, &'a str)>,
    ) -> Vec<Cue> {
        self.update_aligned(rows.into_iter().map(|(u, t)| (u, t, &[][..])))
    }

    pub fn update_aligned<'a>(
        &mut self,
        rows: impl IntoIterator<Item = (&'a Utterance, &'a str, &'a [AlignedPhrase])>,
    ) -> Vec<Cue> {
        let mut previous = std::mem::take(&mut self.pages);
        let mut cues = Vec::new();
        for (row, translation, alignment) in rows {
            if row.text.is_none() {
                continue;
            }
            let (id, pages) = previous
                .remove_entry(&row.id)
                .filter(|(_, p)| {
                    &p.utterance == row && p.translation == translation && p.alignment == alignment
                })
                .unwrap_or_else(|| {
                    (
                        row.id.clone(),
                        CachedPages {
                            utterance: row.clone(),
                            translation: translation.to_owned(),
                            alignment: alignment.to_vec(),
                            cues: utterance_cues(row, translation, alignment),
                        },
                    )
                });
            cues.extend_from_slice(&pages.cues);
            self.pages.insert(id, pages);
        }
        finish_cues(cues)
    }
}

fn finish_cues(mut cues: Vec<Cue>) -> Vec<Cue> {
    cues.sort_by_key(|cue| cue.start_ms);
    finish_timing(&mut cues);
    let mut i = 0;
    while i < cues.len() {
        if cues[i].end_ms.saturating_sub(cues[i].start_ms) < 500 {
            let same = |j: usize| {
                cues.get(j).is_some_and(|cue| {
                    cue.utterance_id == cues[i].utterance_id
                        && cue_width(cue) + cue_width(&cues[i]) <= MAX_CUE_COLS
                })
            };
            let neighbor = if same(i + 1) {
                Some(i)
            } else {
                i.checked_sub(1).filter(|&j| same(j))
            };
            if let Some(left) = neighbor {
                let right = cues.remove(left + 1);
                cues[left].original.push_str(&right.original);
                if let (Some(left), Some(right)) = (&mut cues[left].translation, right.translation)
                {
                    left.push_str(&right);
                }
                cues[left].end_ms = if cue_width(&cues[left]) > PAGE_COLS {
                    right.end_ms
                } else {
                    right
                        .end_ms
                        .min(cues[left].start_ms.saturating_add(MAX_CUE_MS))
                };
                i = left;
                continue;
            }
        }
        i += 1;
    }
    cues.retain_mut(|cue| {
        cue.original = generated_line(&cue.original);
        cue.translation = cue
            .translation
            .as_deref()
            .map(generated_line)
            .filter(|s| !s.is_empty());
        !cue.original.is_empty() || cue.translation.is_some()
    });
    cues
}
