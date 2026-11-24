use super::{display_cols, lexical_mark, AlignedPhrase, RETRANSLATE_COLS};
use crate::word_boundaries::word_ends;

const MAX_WORDS: usize = 256;
const MAX_BYTES: usize = 16384;
const MAX_SPANS: usize = 24;

struct Word {
    text: String,
    start: usize,
    end: usize,
}

fn words(text: &str) -> Vec<Word> {
    let mut previous = 0;
    word_ends(text)
        .into_iter()
        .filter_map(|end| {
            let slice = &text[previous..end];
            let start = previous;
            previous = end;
            let first = slice.find(char::is_alphanumeric)?;
            let token = &slice[first..];
            Some(Word {
                text: token
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect(),
                start: start + first,
                end,
            })
        })
        .collect()
}

// Length of the longest common subsequence for every pair of prefixes. Two
// tables (forward and backward) reveal all equally good target word cuts.
fn prefix_matches(hints: &[&str], target: &[&str]) -> Vec<Vec<u16>> {
    let mut rows = vec![vec![0; target.len() + 1]; hints.len() + 1];
    for (i, hint) in hints.iter().enumerate() {
        for (j, t) in target.iter().enumerate() {
            rows[i + 1][j + 1] = if hint == t {
                rows[i][j] + 1
            } else {
                rows[i][j + 1].max(rows[i + 1][j])
            };
        }
    }
    rows
}

fn target_cut(whole: &str, words: &[Word], at: usize) -> Option<usize> {
    let prev = &words[at - 1];
    let next = &words[at];
    let gap = &whole[prev.end..next.start];
    // Dictionary segmentation may separate tokens in a compound or number.
    // Never expose a slash, decimal point or internal hyphen as a cue break.
    let before = whole[..prev.end].chars().next_back();
    let after = whole[next.start..].chars().next();
    if !gap.is_empty() && gap.chars().all(|c| lexical_mark(c, before, after)) {
        return None;
    }
    Some(prev.end + gap.trim_end().len())
}

/// Find boundaries in a complete translation using independent translations
/// of source phrases as hints. Only UTF-8 offsets are returned. Every visible
/// character and punctuation mark comes from the original whole translation.
pub fn align_translated_phrases(
    source: &str,
    whole: &str,
    spans: &[(&str, &str)],
) -> Option<Vec<AlignedPhrase>> {
    if !(2..=MAX_SPANS).contains(&spans.len())
        || source.len() > MAX_BYTES
        || whole.len() > MAX_BYTES
        || spans
            .iter()
            .map(|(_, t)| t.len())
            .try_fold(0usize, usize::checked_add)?
            > MAX_BYTES
    {
        return None;
    }
    let target_words = words(whole);
    if target_words.is_empty() || target_words.len() > MAX_WORDS {
        return None;
    }
    let target: Vec<_> = target_words.iter().map(|w| w.text.as_str()).collect();
    let mut source_end = 0;
    let mut hint_words = Vec::new();
    let mut ends = Vec::with_capacity(spans.len());
    for &(source_span, hint) in spans {
        if source_span.trim().is_empty() || !source[source_end..].starts_with(source_span) {
            return None;
        }
        source_end += source_span.len();
        let start = hint_words.len();
        hint_words.extend(words(hint).into_iter().map(|w| w.text));
        if hint_words.len() == start || hint_words.len() > MAX_WORDS {
            return None;
        }
        ends.push((source_end, hint_words.len()));
    }
    if source_end != source.len() {
        return None;
    }

    let hints: Vec<_> = hint_words.iter().map(String::as_str).collect();
    let forward = prefix_matches(&hints, &target);
    let backward = prefix_matches(
        &hints.iter().rev().copied().collect::<Vec<_>>(),
        &target.iter().rev().copied().collect::<Vec<_>>(),
    );
    let best = forward[hints.len()][target.len()];
    let mut aligned = Vec::with_capacity(spans.len());
    let (mut hint_start, mut target_start, mut byte_start) = (0, 0, 0);
    for (source_end, hint_end) in ends {
        let (target_end, byte_end) = if hint_end == hints.len() {
            (target.len(), whole.len())
        } else {
            // Model wording may leave one extra word unaccounted for beside
            // a comma. Resolve that one-word tie only when exactly one of the
            // two cuts is a punctuation break in the original translation.
            let mut cuts = (1..target.len()).filter(|&j| {
                forward[hint_end][j] + backward[hints.len() - hint_end][target.len() - j] == best
            });
            let mut j = cuts.next()?;
            if let Some(next) = cuts.next() {
                if next != j + 1 || cuts.next().is_some() {
                    return None;
                }
                let punctuated = |i: usize| {
                    whole[target_words[i - 1].end..target_words[i].start]
                        .chars()
                        .any(|ch| super::END.contains(ch) || super::CLAUSE.contains(ch))
                };
                j = match (punctuated(j), punctuated(next)) {
                    (true, false) => j,
                    (false, true) => next,
                    _ => return None,
                };
            }
            if j <= target_start {
                return None;
            }
            (j, target_cut(whole, &target_words, j)?)
        };
        let inferred = &hints[hint_start..hint_end];
        let original = &target[target_start..target_end];
        let matched =
            usize::from(forward[hint_end][target_end] - forward[hint_start][target_start]);
        // Both sides must be explained, including words the model added or
        // echoed. A very short clause must match in full.
        let longer = inferred.len().max(original.len());
        if matched * 10 < 7 * longer
            || (matched < 2 && inferred != original)
            || display_cols(whole[byte_start..byte_end].trim()) > RETRANSLATE_COLS
        {
            return None;
        }
        aligned.push(AlignedPhrase {
            source_end,
            target_end: byte_end,
        });
        (hint_start, target_start, byte_start) = (hint_end, target_end, byte_end);
    }
    Some(aligned)
}
