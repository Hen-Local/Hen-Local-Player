use crate::types::LangCode;
use whatlang::Lang::{self, *};
pub struct Language {
    pub code: &'static str,
    pub english_name: &'static str,
    pub chinese_name: &'static str,
    pub qwen3_asr_name: Option<&'static str>,
    pub hy_mt2_target: bool,
    pub whatlang: Option<Lang>,
}
const fn entry(
    code: &'static str,
    english_name: &'static str,
    chinese_name: &'static str,
    asr: bool,
    hy_mt2_target: bool,
    whatlang: Option<Lang>,
) -> Language {
    Language {
        code,
        english_name,
        chinese_name,
        qwen3_asr_name: if asr { Some(english_name) } else { None },
        hy_mt2_target,
        whatlang,
    }
}
const LANGUAGES: &[Language] = &[
    entry("zh", "Chinese", "中文", true, true, Some(Cmn)),
    entry("en", "English", "英语", true, true, Some(Eng)),
    entry("fr", "French", "法语", true, true, Some(Fra)),
    entry("pt", "Portuguese", "葡萄牙语", true, true, Some(Por)),
    entry("es", "Spanish", "西班牙语", true, true, Some(Spa)),
    entry("ja", "Japanese", "日语", true, true, Some(Jpn)),
    entry("tr", "Turkish", "土耳其语", true, true, Some(Tur)),
    entry("ru", "Russian", "俄语", true, true, Some(Rus)),
    entry("ar", "Arabic", "阿拉伯语", true, true, Some(Ara)),
    entry("ko", "Korean", "韩语", true, true, Some(Kor)),
    entry("th", "Thai", "泰语", true, true, Some(Tha)),
    entry("it", "Italian", "意大利语", true, true, Some(Ita)),
    entry("de", "German", "德语", true, true, Some(Deu)),
    entry("vi", "Vietnamese", "越南语", true, true, Some(Vie)),
    entry("ms", "Malay", "马来语", true, true, None),
    entry("id", "Indonesian", "印尼语", true, true, Some(Ind)),
    entry("tl", "Filipino", "菲律宾语", true, true, Some(Tgl)),
    entry("hi", "Hindi", "印地语", true, true, Some(Hin)),
    entry(
        "zh-Hant",
        "Traditional Chinese",
        "繁体中文",
        false,
        true,
        None,
    ),
    entry("pl", "Polish", "波兰语", true, true, Some(Pol)),
    entry("cs", "Czech", "捷克语", true, true, Some(Ces)),
    entry("nl", "Dutch", "荷兰语", true, true, Some(Nld)),
    entry("km", "Khmer", "高棉语", false, true, Some(Khm)),
    entry("my", "Burmese", "缅甸语", false, true, Some(Mya)),
    entry("fa", "Persian", "波斯语", true, true, Some(Pes)),
    entry("gu", "Gujarati", "古吉拉特语", false, true, Some(Guj)),
    entry("ur", "Urdu", "乌尔都语", false, true, Some(Urd)),
    entry("te", "Telugu", "泰卢固语", false, true, Some(Tel)),
    entry("mr", "Marathi", "马拉地语", false, true, Some(Mar)),
    entry("he", "Hebrew", "希伯来语", false, true, Some(Heb)),
    entry("bn", "Bengali", "孟加拉语", false, true, Some(Ben)),
    entry("ta", "Tamil", "泰米尔语", false, true, Some(Tam)),
    entry("uk", "Ukrainian", "乌克兰语", false, true, Some(Ukr)),
    entry("bo", "Tibetan", "藏语", false, true, None),
    entry("kk", "Kazakh", "哈萨克语", false, true, None),
    entry("mn", "Mongolian", "蒙古语", false, true, None),
    entry("ug", "Uyghur", "维吾尔语", false, true, None),
    entry("yue", "Cantonese", "粤语", true, true, None),
    entry("sv", "Swedish", "瑞典语", true, false, Some(Swe)),
    entry("da", "Danish", "丹麦语", true, false, Some(Dan)),
    entry("fi", "Finnish", "芬兰语", true, false, Some(Fin)),
    entry("el", "Greek", "希腊语", true, false, Some(Ell)),
    entry("ro", "Romanian", "罗马尼亚语", true, false, Some(Ron)),
    entry("hu", "Hungarian", "匈牙利语", true, false, Some(Hun)),
    entry("mk", "Macedonian", "马其顿语", true, false, Some(Mkd)),
];
pub fn languages() -> &'static [Language] {
    LANGUAGES
}
pub fn lookup(code: &str) -> Option<&'static Language> {
    LANGUAGES
        .iter()
        .find(|lang| lang.code.eq_ignore_ascii_case(code))
}
fn locale_language(locale: &str) -> Option<&'static Language> {
    let normalized = locale
        .split(['.', '@'])
        .next()
        .unwrap_or(locale)
        .replace('_', "-")
        .to_ascii_lowercase();
    let mut subtags = normalized.split('-');
    let base = subtags.next().unwrap_or_default();
    let code = match base {
        "zh" => subtags
            .find_map(|tag| match tag {
                "hans" => Some("zh"),
                "hant" | "tw" | "hk" | "mo" => Some("zh-Hant"),
                _ => None,
            })
            .unwrap_or("zh"),
        "fil" => "tl",
        "iw" => "he",
        "in" => "id",
        other => other,
    };
    lookup(code)
}

pub fn default_target(locale: &str) -> LangCode {
    LangCode(default_subtitle_languages([locale]).1.into())
}

/// Spoken input and translation output for a fresh installation, in OS preference order.
pub fn default_subtitle_languages(
    locales: impl IntoIterator<Item = impl AsRef<str>>,
) -> (&'static str, &'static str) {
    let mut target = None;
    let mut alternate_spoken = None;
    for language in locales
        .into_iter()
        .filter_map(|l| locale_language(l.as_ref()))
    {
        if language.hy_mt2_target {
            target.get_or_insert(language.code);
        }
        if language.code == "zh-Hant" {
            alternate_spoken.get_or_insert("zh");
        } else if language.code != "en" && language.qwen3_asr_name.is_some() {
            alternate_spoken.get_or_insert(language.code);
        }
    }
    let target = target.unwrap_or("en");
    let source = if target == "en" {
        alternate_spoken.unwrap_or("auto")
    } else {
        "en"
    };
    (source, target)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn inventories_are_complete_and_unique() {
        assert_eq!(
            languages()
                .iter()
                .filter(|l| l.qwen3_asr_name.is_some())
                .count(),
            30
        );
        assert_eq!(languages().iter().filter(|l| l.hy_mt2_target).count(), 38);
        for keys in [
            languages().iter().map(|l| l.code).collect::<Vec<_>>(),
            languages()
                .iter()
                .filter_map(|l| l.qwen3_asr_name)
                .collect(),
        ] {
            assert_eq!(keys.len(), keys.iter().collect::<HashSet<_>>().len());
        }
        let detector: Vec<_> = languages().iter().filter_map(|l| l.whatlang).collect();
        assert_eq!(
            detector.len(),
            detector.iter().collect::<HashSet<_>>().len()
        );
        assert!(!lookup("sv").unwrap().hy_mt2_target);
        assert!(lookup("zh-Hant").unwrap().qwen3_asr_name.is_none());
    }
    #[test]
    fn locale_defaults() {
        for (locale, code) in [
            ("zh-TW", "zh-Hant"),
            ("zh_Hant_HK", "zh-Hant"),
            ("ZH-hk", "zh-Hant"),
            ("zh-Hans-CN", "zh"),
            ("zh_CN.UTF-8", "zh"),
            ("de-DE", "de"),
            ("fi", "en"),
            ("", "en"),
            ("yue-HK", "yue"),
        ] {
            assert_eq!(default_target(locale).0, code)
        }
    }

    #[test]
    fn fresh_subtitle_languages_follow_preference_order() {
        for (locales, expected) in [
            (vec!["zh-Hans-CN", "en-US"], ("en", "zh")),
            (vec!["zh-TW"], ("en", "zh-Hant")),
            (vec!["zh-Hans-TW"], ("en", "zh")),
            (vec!["fil-PH"], ("en", "tl")),
            (vec!["sv-SE", "de-DE"], ("en", "de")),
            (vec!["en-US", "en-GB", "fr-CA"], ("fr", "en")),
            (vec!["en-US", "zh-Hant-TW"], ("zh", "en")),
            (vec!["en-US", "gu-IN"], ("auto", "en")),
            (vec!["fi-FI"], ("fi", "en")),
            (vec!["en-US"], ("auto", "en")),
            (vec![], ("auto", "en")),
        ] {
            assert_eq!(default_subtitle_languages(locales), expected);
        }
    }
}
