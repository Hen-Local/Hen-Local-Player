use crate::types::*;
use icu_properties::{
    props::{GeneralCategory, GeneralCategoryGroup},
    CodePointMapData,
};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Aim for a single line in each language. Fast speech can share a longer
// timed cue when an individual phrase would otherwise flash for under 500 ms.
const PAGE_COLS: usize = 42;
const MAX_CUE_COLS: usize = 2 * PAGE_COLS;
const MIN_CUE_MS: Ms = 1000;
const MAX_CUE_MS: Ms = 7000;
const MIN_GAP_MS: Ms = 80;
const LINGER_MS: Ms = 500;
const READ_COLS_PER_SEC: Ms = 20;
const END: &str = ".!?。！？…؟।";
const CLAUSE: &str = ",;:，；：、—؛،";
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

/// Favor a nearby phrase/word boundary; a distant comma must not cause a
/// one-word flash followed by another oversized page.
fn page_cut(text: &str, fraction: f64) -> Option<usize> {
    let end = text.trim_end().len();
    let leading = text.len() - text.trim_start().len();
    let leading_cols = display_cols(&text[..leading]);
    let width = display_cols(text.trim());
    let target = (width as f64 * fraction).round() as usize;
    let tolerance = (target.min(width.saturating_sub(target)) / 3).max(2);
    let choices = boundaries(text);
    let usable = |b: &&Boundary| b.byte > leading && b.byte < end;
    let candidates = choices.iter().filter(usable);
    let has_words = candidates.clone().any(|b| b.kind != Break::Grapheme);
    if !has_words && width <= PAGE_COLS {
        return None; // Keep a short indivisible word whole on its companion page.
    }
    candidates
        .filter(|b| !has_words || b.kind != Break::Grapheme)
        .min_by_key(|b| {
            let dist = b.cols.saturating_sub(leading_cols).abs_diff(target);
            (dist.saturating_sub(tolerance), b.kind, dist)
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

pub fn distribute(originals: &[String], translation: &str) -> Vec<String> {
    let originals: Vec<_> = originals.iter().map(String::as_str).collect();
    aligned_translation(&originals, translation.trim())
        .into_iter()
        .map(|s| s.trim().to_owned())
        .collect()
}

fn paired_pages<'a>(original: &'a str, translation: &'a str) -> Vec<(&'a str, &'a str)> {
    let originals: Vec<_> = original.split_sentence_bounds().collect();
    let targets = aligned_translation(&originals, translation);
    let mut pending = if !translation.is_empty() && targets.iter().any(|s| s.trim().is_empty()) {
        // More source sentences than target words. Split the pair jointly.
        vec![(original, translation)]
    } else {
        originals.into_iter().zip(targets).rev().collect()
    };
    let mut pages = Vec::new();
    while let Some((source, target)) = pending.pop() {
        let sw = display_cols(source.trim());
        let tw = display_cols(target.trim());
        let width = sw.max(tw);
        if width > PAGE_COLS {
            let count = width.div_ceil(PAGE_COLS);
            let (long, companion) = if sw >= tw {
                (source, target)
            } else {
                (target, source)
            };
            if let Some(i) = page_cut(long, (count / 2) as f64 / count as f64) {
                let fraction = display_cols(long[..i].trim()) as f64 / width as f64;
                let j = if companion.is_empty() {
                    Some(0)
                } else {
                    page_cut(companion, fraction)
                };
                if let Some(j) = j {
                    let (s, t) = if sw >= tw { (i, j) } else { (j, i) };
                    pending.push((&source[s..], &target[t..]));
                    pending.push((&source[..s], &target[..t]));
                    continue;
                }
            }
        }
        pages.push((source, target));
    }
    pages
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

fn utterance_cues(utt: &Utterance, translation: &str) -> Vec<Cue> {
    let Some(text) = utt.text.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
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
    let pages = paired_pages(text, translation.trim());
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
        cue.end_ms = cue
            .end_ms
            .max(needed)
            .saturating_add(LINGER_MS)
            .min(cue.start_ms.saturating_add(MAX_CUE_MS))
            .min(limit);
    }
}

pub fn build_cues(utterances: &[Utterance], translations: &HashMap<String, String>) -> Vec<Cue> {
    finish_cues(
        utterances
            .iter()
            .flat_map(|u| utterance_cues(u, translations.get(&u.id).map_or("", String::as_str)))
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
    cues: Vec<Cue>,
}
impl CueBuilder {
    /// Reuse phrase segmentation between successive recognition and translation batches.
    /// Timing and merges still see every neighbor, including overlapping or out-of-order speech.
    pub fn update<'a>(
        &mut self,
        rows: impl IntoIterator<Item = (&'a Utterance, &'a str)>,
    ) -> Vec<Cue> {
        let mut previous = std::mem::take(&mut self.pages);
        let mut cues = Vec::new();
        for (row, translation) in rows {
            if row.text.is_none() {
                continue;
            }
            let (id, pages) = previous
                .remove_entry(&row.id)
                .filter(|(_, p)| &p.utterance == row && p.translation == translation)
                .unwrap_or_else(|| {
                    (
                        row.id.clone(),
                        CachedPages {
                            utterance: row.clone(),
                            translation: translation.to_owned(),
                            cues: utterance_cues(row, translation),
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
                cues[left].end_ms = right
                    .end_ms
                    .min(cues[left].start_ms.saturating_add(MAX_CUE_MS));
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
