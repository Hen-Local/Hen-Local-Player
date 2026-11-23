use crate::types::*;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const MAX_LINE_COLS: usize = 42;
const MAX_CUE_COLS: usize = 2 * MAX_LINE_COLS;
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

/// Keep whitespace on the right-hand piece until final display trimming. Adjacent
/// slices can then be merged exactly for both spaced and unspaced scripts.
fn sentences(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut ended = false;
    for (index, ch) in text.char_indices() {
        if ended && !END.contains(ch) && !CLOSE.contains(ch) {
            result.push(&text[start..index]);
            start = index;
            ended = false;
        }
        ended |= END.contains(ch);
    }
    if !text[start..].trim().is_empty() {
        result.push(&text[start..])
    }
    result
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
    let mut result = Vec::new();
    let mut cols = 0;
    let mut graphemes = text.grapheme_indices(true).peekable();
    while let Some((_, g)) = graphemes.next() {
        cols += display_cols(g);
        let Some(&(byte, next)) = graphemes.peek() else {
            break;
        };
        let next_ch = next.chars().next().unwrap();
        if (END.contains(next_ch) || CLAUSE.contains(next_ch) || CLOSE.contains(next_ch))
            && !g.trim().is_empty()
        {
            continue;
        }
        let prev = g.chars().last().unwrap();
        let kind = if CLAUSE.contains(prev) || END.contains(prev) {
            Break::Clause
        } else if next_ch.is_whitespace() || unspaced(prev) || unspaced(next_ch) {
            Break::Word
        } else {
            Break::Grapheme
        };
        result.push(Boundary { byte, cols, kind });
    }
    result
}

fn cut(text: &str, wrap: bool) -> Option<usize> {
    let cols = display_cols(text.trim());
    let leading = display_cols(text) - display_cols(text.trim_start());
    boundaries(text)
        .into_iter()
        .filter(|b| !text[..b.byte].trim().is_empty() && !text[b.byte..].trim().is_empty())
        .min_by_key(|b| {
            let left = b.cols.saturating_sub(leading);
            let right = cols.saturating_sub(left);
            // Keep both export lines within their column budget when possible.
            let overflow =
                if wrap {
                    left.max(right.saturating_sub(usize::from(
                        text[b.byte..].starts_with(char::is_whitespace),
                    )))
                    .saturating_sub(MAX_LINE_COLS)
                } else {
                    0
                };
            (overflow, b.kind, left.abs_diff(right))
        })
        .map(|b| b.byte)
}

fn short_pieces(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut pending = sentences(text);
    pending.reverse();
    while let Some(piece) = pending.pop() {
        if display_cols(piece.trim()) > MAX_CUE_COLS {
            if let Some(at) = cut(piece, false) {
                let (left, right) = piece.split_at(at);
                pending.push(right);
                pending.push(left);
                continue;
            }
        }
        pieces.push(piece);
    }
    pieces
}
fn merge_short_pieces(text: &str) -> Vec<String> {
    let mut pieces: Vec<String> = Vec::new();
    for s in short_pieces(text) {
        if let Some(last) = pieces.last_mut() {
            if display_cols(last.trim()) < 20 && display_cols(s.trim()) < 20 {
                last.push_str(s);
                continue;
            }
        }
        pieces.push(s.to_owned());
    }
    pieces
}

pub fn wrap_lines(text: &str) -> String {
    let mut text = text.trim();
    let flattened;
    if text.contains(['\r', '\n']) {
        let lines: Vec<_> = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if lines.len() <= 2 && lines.iter().all(|s| display_cols(s) <= MAX_LINE_COLS) {
            return lines.join("\n");
        }
        flattened = lines.into_iter().fold(String::new(), |mut joined, line| {
            let unspaced_boundary = joined.ends_with(unspaced) && line.starts_with(unspaced);
            if !joined.is_empty() && !unspaced_boundary {
                joined.push(' ');
            }
            joined.push_str(line);
            joined
        });
        text = &flattened;
    }
    if display_cols(text) <= MAX_LINE_COLS {
        return text.to_owned();
    }
    if let Some(at) = cut(text, true) {
        format!("{}\n{}", text[..at].trim_end(), text[at..].trim_start())
    } else {
        text.to_owned()
    }
}

pub fn distribute(originals: &[String], translation: &str) -> Vec<String> {
    if originals.is_empty() {
        return Vec::new();
    }
    let text = translation.trim();
    if originals.len() == 1 {
        return vec![text.to_owned()];
    }
    let s = sentences(text);
    if s.len() == originals.len() {
        return s.into_iter().map(|p| p.trim().to_owned()).collect();
    }
    let mut atoms = boundaries(text);
    atoms.retain(|b| b.kind != Break::Grapheme);
    let total_cols = display_cols(text);
    atoms.push(Boundary {
        byte: text.len(),
        cols: total_cols,
        kind: Break::Word,
    });
    let weights: Vec<_> = originals.iter().map(|o| display_cols(o).max(1)).collect();
    let total_weight: usize = weights.iter().sum();
    let mut result = Vec::new();
    let (mut accum, mut first, mut start) = (0, 0, 0);
    for (i, weight) in weights.iter().enumerate().take(weights.len() - 1) {
        accum += weight;
        // Reserve one atom for each remaining cue if the translation contains enough.
        let available = atoms.len() - first;
        let remaining = weights.len() - i;
        if available == 0 {
            result.push(String::new());
            continue;
        }
        let last = if available >= remaining {
            atoms.len() - (remaining - 1)
        } else {
            atoms.len()
        };
        let target = total_cols as f64 * accum as f64 / total_weight as f64;
        let tolerance = 0.15 * target;
        let (relative, b) = atoms[first..last]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let score = |p: &Boundary| {
                    let distance = (p.cols as f64 - target).abs();
                    (p.kind != Break::Clause || distance > tolerance, distance)
                };
                score(a).partial_cmp(&score(b)).unwrap()
            })
            .unwrap();
        result.push(text[start..b.byte].trim().to_string());
        start = b.byte;
        first += relative + 1;
    }
    result.push(text[start..].trim().to_string());
    result
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

fn utterance_cues(utt: &Utterance) -> Vec<Cue> {
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
    let pieces = merge_short_pieces(text);
    let cols: usize = pieces.iter().map(|s| display_cols(s.trim()).max(1)).sum();
    let mut cum = 0;
    let mut cues = Vec::new();
    for piece in pieces {
        let offset = |w: usize| (total as u128 * w as u128 / cols as u128) as Ms;
        let start = speech_time(&regions, offset(cum), true);
        cum += display_cols(piece.trim()).max(1);
        let end = speech_time(&regions, offset(cum), false);
        let id = format!("{}-{}", utt.id, cues.len());
        cues.push(Cue {
            id,
            utterance_id: utt.id.clone(),
            start_ms: start,
            end_ms: end,
            original: piece,
            translation: None,
            lang: utt.language.clone(),
            stale: utt.status == UttStatus::Pending,
        });
    }
    cues
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
        let reading = 1000 * display_cols(cue.original.trim()) as Ms / READ_COLS_PER_SEC;
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
    let mut cues: Vec<_> = utterances.iter().flat_map(utterance_cues).collect();
    cues.sort_by_key(|cue| cue.start_ms);
    finish_timing(&mut cues);
    let mut i = 0;
    while i < cues.len() {
        if cues[i].end_ms.saturating_sub(cues[i].start_ms) < 500 {
            let same = |j: usize| {
                cues.get(j)
                    .is_some_and(|cue| cue.utterance_id == cues[i].utterance_id)
            };
            let neighbor = if same(i + 1) {
                Some(i)
            } else {
                i.checked_sub(1).filter(|&j| same(j))
            };
            if let Some(left) = neighbor {
                let right = cues.remove(left + 1);
                cues[left].original.push_str(&right.original);
                cues[left].end_ms = right
                    .end_ms
                    .min(cues[left].start_ms.saturating_add(MAX_CUE_MS));
                i = left;
                continue;
            }
        }
        i += 1;
    }
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, cue) in cues.iter_mut().enumerate() {
        cue.original = cue.original.trim().to_string();
        groups.entry(cue.utterance_id.clone()).or_default().push(i);
    }
    for (id, indices) in groups {
        if let Some(text) = translations.get(&id) {
            let originals = indices
                .iter()
                .map(|&i| cues[i].original.clone())
                .collect::<Vec<_>>();
            for (i, t) in indices.into_iter().zip(distribute(&originals, text)) {
                cues[i].translation = (!t.is_empty()).then_some(t);
            }
        }
    }
    cues
}
