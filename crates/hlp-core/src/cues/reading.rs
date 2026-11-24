use super::{boundaries, connector, dangling_word, display_cols, Break};

// A normal subtitle can stay intact just beyond the 42-column target. A
// slightly longer pair is preferable when an adjunct changes position in MT.
pub(super) const LINE_LIMIT: usize = 62;
const PAGE_TARGET: usize = 58;
const REORDERED_LIMIT: usize = 72;

#[derive(Clone, Copy)]
struct Cut {
    byte: usize,
    width: usize,
    penalty: usize,
}

fn is_name(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase)
        && word.chars().skip(1).any(char::is_lowercase)
}

fn first_word(text: &str) -> &str {
    text.trim_start()
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
}

fn among(word: &str, choices: &[&str]) -> bool {
    choices.iter().any(|s| word.eq_ignore_ascii_case(s))
}

// Rank ends of readable word groups in each language before comparing their
// relative positions. Keep determiners, compounds, lists of names and numeric
// expressions together even when punctuation in one language differs.
fn penalty(left: &str, right: &str, kind: Break) -> usize {
    let right = right.trim_start();
    let last = left.split_whitespace().next_back().unwrap_or("");
    let next = first_word(right);
    let numeric = |c: char| "一二三四五六七八九十".contains(c);
    let last_name = is_name(last.trim_end_matches(','));
    let next_name = is_name(next);
    let after_and = right
        .strip_prefix("and ")
        .and_then(|r| r.split_whitespace().next());
    let in_list = left.ends_with('、')
        || right.starts_with('和')
        || (last_name && (next_name || after_and.is_some_and(is_name)))
        || (last.ends_with(',') && right.starts_with("and "));
    if in_list
        || dangling_word(left, right)
        || among(
            last,
            &[
                "even", "than", "very", "many", "as", "was", "were", "like", "let’s", "let's",
            ],
        )
        || left.ends_with(['像', '于', '在', '和'])
        || left.ends_with(numeric)
        || last.ends_with("ly")
        || last.ends_with("al")
        || among(
            last,
            &[
                "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
            ],
        )
    {
        return 30;
    }
    if kind == Break::Clause || connector(right).is_some() {
        return 0;
    }

    // A coordinating conjunction followed by a capitalized name often ends
    // the subject of the clause. A cut after the list keeps every name on it.
    if last_name
        && left
            .split_whitespace()
            .rev()
            .nth(1)
            .is_some_and(|w| w == "and")
    {
        return 0;
    }
    if among(
        next,
        &[
            "that",
            "which",
            "when",
            "where",
            "before",
            "after",
            "until",
            "while",
            "into",
            "in",
            "with",
            "without",
            "including",
            "about",
            "for",
            "from",
            "to",
            "a",
            "an",
            "the",
            "their",
            "our",
            "my",
            "your",
            "three",
            "two",
            "one",
        ],
    ) || right.starts_with(['在', '从', '让', '把', '给', '向', '将'])
        || right.starts_with("为了")
        || right.starts_with("可以")
        || right
            .chars()
            .next()
            .is_some_and(|c| "一二三四五六七八九十".contains(c))
    {
        return 4;
    }
    10
}

fn candidates(text: &str, offset: usize, width: usize) -> Vec<Cut> {
    let bounds = boundaries(text);
    let has_words = bounds.iter().any(|b| b.kind != Break::Grapheme);
    let mut cuts: Vec<_> = bounds
        .into_iter()
        .filter(|b| b.kind != Break::Grapheme || (!has_words && width > LINE_LIMIT))
        .map(|b| {
            let (left, right) = text.split_at(b.byte);
            Cut {
                byte: b.byte,
                // Some shaped scripts have a smaller whole-string width than the
                // sum of the individual graphemes (for example Arabic lam-alef).
                width: b.cols.min(width),
                penalty: penalty(left, right, b.kind),
            }
        })
        .collect();
    cuts.sort_unstable_by_key(|c| {
        let edge = c.width.min(width - c.width);
        (
            c.width.abs_diff(offset) + c.penalty + 3 * 8usize.saturating_sub(edge),
            c.byte,
        )
    });
    cuts.truncate(16);
    cuts
}

fn reordered_adjunct(source: &str, target: &str) -> bool {
    fn moved(zh: &str, en: &str) -> bool {
        let text = zh.trim_start_matches(|c: char| c.is_whitespace() || super::CLAUSE.contains(c));
        text.starts_with('在')
            && text
                .trim_end_matches(|c: char| !c.is_alphanumeric())
                .contains('，')
            && [" in ", " at ", " on "]
                .iter()
                .any(|prep| en.to_ascii_lowercase().contains(prep))
    }
    moved(source, target) || moved(target, source)
}

// Numbers, brands and technical terms can survive a change of language.
// A boundary crossing one of these anchors in just one language would put
// the name on screen at the wrong time.
fn shared_tokens(s: &str, t: &str) -> Vec<((usize, usize), (usize, usize))> {
    use std::collections::HashMap;
    fn words(text: &str) -> HashMap<String, Option<(usize, usize)>> {
        let mut unique = HashMap::new();
        let mut record = |word: String, start, end| {
            unique
                .entry(word)
                .and_modify(|v| *v = None)
                .or_insert(Some((start, end)));
        };
        let mut chars = text.char_indices().peekable();
        while let Some((start, c)) = chars.next() {
            if c.is_ascii_alphanumeric() {
                while chars.next_if(|(_, c)| c.is_ascii_alphanumeric()).is_some() {}
                let end = chars.peek().map_or(text.len(), |(i, _)| *i);
                if end - start > 1 || c.is_ascii_digit() {
                    record(text[start..end].to_ascii_lowercase(), start, end);
                }
            } else if let Some(word) = match c {
                '二' | '两' => Some("two"),
                '三' => Some("three"),
                '四' => Some("four"),
                '五' => Some("five"),
                '六' => Some("six"),
                '七' => Some("seven"),
                '八' => Some("eight"),
                '九' => Some("nine"),
                '十' => Some("ten"),
                _ => None,
            } {
                record(word.into(), start, start + c.len_utf8());
            }
        }
        unique
    }
    let targets = words(t);
    words(s)
        .into_iter()
        .filter_map(|(token, end)| Some((end?, (*targets.get(&token)?)?)))
        .collect()
}

fn split(s: &str, t: &str, sw: usize, tw: usize) -> Option<(usize, usize)> {
    let width = sw.max(tw);
    let visible_width = display_cols(s.trim()).max(display_cols(t.trim()));
    if visible_width <= LINE_LIMIT || (visible_width <= REORDERED_LIMIT && reordered_adjunct(s, t))
    {
        return None;
    }
    let n = width.div_ceil(PAGE_TARGET);
    let left = n / 2;
    let right = n - left;
    let sc = candidates(s, sw * left / n, sw);
    let tc = candidates(t, tw * left / n, tw);
    // A single source word cannot align with several complete target clauses.
    // Keep its one occurrence on the first page, then time the remaining text
    // against the same utterance. No text is repeated or discarded.
    let single = |cuts: &[Cut], w: usize| {
        cuts.iter()
            .min_by_key(|c| c.width.abs_diff(w * left / n) + c.penalty)
            .map(|c| c.byte)
    };
    if sc.is_empty() {
        return single(&tc, tw).map(|j| (s.len(), j));
    }
    if tc.is_empty() {
        return single(&sc, sw).map(|i| (i, t.len()));
    }
    let anchors = shared_tokens(s, t);
    let mut best = None;
    for a in &sc {
        for b in &tc {
            let lhs = a.width.max(b.width);
            let rhs = (sw - a.width).max(tw - b.width);
            let overflow =
                lhs.saturating_sub(left * PAGE_TARGET) + rhs.saturating_sub(right * PAGE_TARGET);
            let drift = (a.width * 100 / sw.max(1)).abs_diff(b.width * 100 / tw.max(1));
            let aim = lhs.abs_diff(width * left / n) + rhs.abs_diff(width * right / n);
            let short_edge = |edge: usize, total: usize| {
                (total / 3).min(8).saturating_sub(edge.min(total - edge))
            };
            let tiny = 6 * (short_edge(a.width, sw) + short_edge(b.width, tw));
            let mismatches = anchors
                .iter()
                .filter(|((_, i), (_, j))| (a.byte >= *i) != (b.byte >= *j))
                .count();
            let at_anchor = anchors.iter().any(|((i, _), (j, _))| {
                s.get(a.byte..*i).is_some_and(|lead| lead.trim().is_empty())
                    && t.get(b.byte..*j).is_some_and(|lead| lead.trim().is_empty())
            });
            let score = 40 * usize::from(!at_anchor)
                + 80 * mismatches
                + 8 * overflow
                + drift
                + aim / 2
                + 3 * (a.penalty + b.penalty)
                + tiny;
            if best.is_none_or(|(v, _)| score < v) {
                best = Some((score, (a.byte, b.byte)));
            }
        }
    }
    best.map(|(_, pair)| pair)
}

/// Refine already aligned spans for display and export. The same boundaries
/// apply when opening previously saved translations without either model.
pub(super) fn pages<'a>(paired: Vec<(&'a str, &'a str)>) -> Vec<(&'a str, &'a str)> {
    let mut pending: Vec<_> = paired.into_iter().rev().collect();
    let mut pages = Vec::new();
    while let Some((s, t)) = pending.pop() {
        let (sw, tw) = (display_cols(s), display_cols(t));
        if let Some((i, j)) = split(s, t, sw, tw) {
            pending.push((&s[i..], &t[j..]));
            pending.push((&s[..i], &t[..j]));
        } else {
            pages.push((s, t));
        }
    }
    pages
}
