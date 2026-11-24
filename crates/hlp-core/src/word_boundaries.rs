use icu_segmenter::{WordSegmenter, WordSegmenterBorrowed};
use std::sync::LazyLock;

static WORDS: LazyLock<WordSegmenterBorrowed<'static>> =
    LazyLock::new(|| WordSegmenter::new_dictionary(Default::default()));

/// UTF-8 end offsets of complete words, excluding spaces and punctuation.
pub(crate) fn word_ends(text: &str) -> Vec<usize> {
    let mut start = 0;
    WORDS
        .segment_str(text)
        .filter(|&end| {
            let word = &text[start..end];
            start = end;
            word.chars().any(char::is_alphanumeric)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_finds_words_inside_unspaced_text() {
        assert_eq!(word_ends("今天，我们学习中文。"), [6, 15, 21, 27]);
        assert_eq!(word_ends("こんにちは世界"), [15, 21]);
        assert_eq!(word_ends("ทุกสองสัปดาห์"), [9, 18, 39]);
    }

    #[test]
    fn offsets_preserve_combining_characters_and_ignore_nonwords() {
        let mixed = "Cafe\u{301} 世界！";
        assert_eq!(
            word_ends(mixed),
            ["Cafe\u{301}".len(), mixed.len() - '！'.len_utf8()]
        );
        assert!(word_ends("，… ！").is_empty());
        assert!(word_ends("").is_empty());
    }
}
