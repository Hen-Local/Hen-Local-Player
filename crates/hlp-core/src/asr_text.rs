use crate::{cues::display_cols, lang::languages, types::LangCode};
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, PartialEq)]
pub enum ParsedAsr {
    Text {
        text: String,
        language: Option<LangCode>,
    },
    NoSpeech,
    Failed(String),
}

static TAGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<\|[^|]*\|>").expect("literal tag pattern"));

static NON_TEXT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{P}\s]*$").expect("literal punctuation pattern"));

/// Collapse a contiguous run only after four equal periods have been observed.
fn collapse<T: Eq + Clone>(items: &[T], max_period: usize) -> Vec<T> {
    let mut out = Vec::with_capacity(items.len());
    let mut i = 0;
    while i < items.len() {
        let max_period = max_period.min((items.len() - i) / 4);
        let mut next = i + 1;
        let mut keep = 1;
        for period in 1..=max_period {
            let pattern = &items[i..i + period];
            if (1..4).all(|j| &items[i + j * period..i + (j + 1) * period] == pattern) {
                keep = period;
                next = i + 4 * period;
                while items.get(next..next + period) == Some(pattern) {
                    next += period
                }
                break;
            }
        }
        out.extend_from_slice(&items[i..i + keep]);
        i = next;
    }
    out
}

pub fn parse(raw: &str, speech_seconds: f64) -> ParsedAsr {
    let (meta, body) = raw.split_once("<asr_text>").unwrap_or(("", raw));
    let none = meta.to_ascii_lowercase().contains("language none");
    let mut language = None;
    if !none {
        if let Some(name) = meta.lines().map(str::trim).find_map(|line| {
            let (prefix, value) = line.split_once(char::is_whitespace)?;
            prefix
                .eq_ignore_ascii_case("language")
                .then_some(value.trim())
        }) {
            language = languages()
                .iter()
                .find(|lang| {
                    lang.qwen3_asr_name
                        .is_some_and(|n| n.eq_ignore_ascii_case(name))
                })
                .map(|lang| LangCode(lang.code.to_owned()));
        }
    }
    let stripped = TAGS.replace_all(body, "");
    if none && stripped.trim().chars().count() <= 8 {
        return ParsedAsr::NoSpeech;
    }
    let words = stripped.split_whitespace().collect::<Vec<_>>();
    let word_clean = collapse(&words, 6).join(" ");
    let chars = word_clean.chars().collect::<Vec<_>>();
    let cleaned = collapse(&chars, 20).into_iter().collect::<String>();
    let text = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if NON_TEXT.is_match(&text) {
        return ParsedAsr::NoSpeech;
    }
    if !speech_seconds.is_finite()
        || speech_seconds <= 0.
        || display_cols(&text) as f64 > 45. * speech_seconds
    {
        return ParsedAsr::Failed("implausible length".into());
    }
    ParsedAsr::Text { text, language }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn assert_text(raw: &str, text: &str, lang: Option<&str>) {
        assert_eq!(
            parse(raw, 10.),
            ParsedAsr::Text {
                text: text.into(),
                language: lang.map(|c| LangCode(c.into()))
            }
        );
    }
    #[test]
    fn header_and_forced_text() {
        assert_text(
            "other\n LANGUAGE Japanese \n<asr_text>こんにちは。",
            "こんにちは。",
            Some("ja"),
        );
        assert_text("<|en|>Hello  \nworld", "Hello world", None);
        assert_text("language Unknown<asr_text>Hello", "Hello", None);
        assert_text(
            "language None<asr_text>A longer sentence",
            "A longer sentence",
            None,
        );
        assert_eq!(
            parse("language None<asr_text>こんにちは", 1.),
            ParsedAsr::NoSpeech
        );
        assert_eq!(parse("…！", 4.), ParsedAsr::NoSpeech);
    }
    #[test]
    fn repeated_characters_and_word_sequences() {
        assert_text("開門開門開門開門開門吧", "開門吧", None);
        assert_text(
            &"sufficiently lengthy multiword utterance ".repeat(5),
            "sufficiently lengthy multiword utterance",
            None,
        );
        assert_text("go go go", "go go go", None);
        assert_eq!(
            parse("世界你好", 0.1),
            ParsedAsr::Failed("implausible length".into())
        );
    }
}
