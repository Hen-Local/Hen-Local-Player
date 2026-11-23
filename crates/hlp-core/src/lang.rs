use crate::types::LangCode;
pub struct Language {
    pub code: &'static str,
    pub english_name: &'static str,
    pub chinese_name: &'static str,
    pub qwen3_asr_name: Option<&'static str>,
    pub hy_mt2_target: bool,
    pub whatlang: Option<whatlang::Lang>,
}
pub fn languages() -> &'static [Language] {
    todo!()
}
pub fn lookup(code: &str) -> Option<&'static Language> {
    todo!()
}
pub fn default_target(locale: &str) -> LangCode {
    let locale = locale.replace('_', "-").to_ascii_lowercase();
    let base = locale.split('-').next().unwrap_or("en");
    let code=if locale.starts_with("zh-hant") || locale.starts_with("zh-tw") || locale.starts_with("zh-hk") {"zh-Hant"} else if "zh en fr pt es ja tr ru ar ko th it de vi ms id tl hi pl cs nl km my fa gu ur te mr he bn ta uk bo kk mn ug yue".split_whitespace().any(|c|c==base){base}else{"en"};
    LangCode(code.into())
}
