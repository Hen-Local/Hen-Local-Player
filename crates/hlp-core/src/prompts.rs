use crate::lang::lookup;

pub const PROMPT_VERSION: u32 = 1;
pub const TEMPERATURE: f32 = 0.;
pub const REPEAT_PENALTY: f32 = 1.05;
pub const MAX_TOKENS: u32 = 384;

/// The single user message expected by Hy-MT2. Preserve the source literally.
pub fn translation_prompt(
    source: Option<&str>,
    target: &str,
    text: &str,
    background: Option<&str>,
    force_english: bool,
) -> String {
    let chinese = !force_english
        && [source, Some(target)]
            .into_iter()
            .flatten()
            .any(|code| matches!(code, "zh" | "zh-Hant" | "yue"));
    let name = lookup(target).map_or(target, |lang| {
        if chinese {
            lang.chinese_name
        } else {
            lang.english_name
        }
    });
    match (chinese, background.filter(|bg|!bg.trim().is_empty())) {
        (true,None) => format!("将以下文本翻译为{name}，注意只需要输出翻译后的结果，不要额外解释：\n\n{text}"),
        (false,None) => format!("Translate the following text into {name}. Note that you should only output the translated result without any additional explanation:\n\n{text}"),
        (true,Some(bg)) => format!("【背景信息】\n{bg}\n\n请结合背景信息将以下文本翻译为{name}。\n\n【待翻译文本】\n{text}"),
        (false,Some(bg)) => format!("[Background Information]\n{bg}\n\nPlease translate the following text into {name}, taking the provided background information into consideration.\n\n[Source Text]\n{text}"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_prompts_and_source_preservation() {
        let text = "<字幕> \"é\"\nline 2";
        assert_eq!(
            translation_prompt(Some("yue"), "en", text, None, false),
            format!("将以下文本翻译为英语，注意只需要输出翻译后的结果，不要额外解释：\n\n{text}")
        );
        assert_eq!(
            translation_prompt(Some("en"), "zh-Hant", "hi", None, false),
            "将以下文本翻译为繁体中文，注意只需要输出翻译后的结果，不要额外解释：\n\nhi"
        );
        assert_eq!(translation_prompt(Some("fr"),"de",text,None,false),format!("Translate the following text into German. Note that you should only output the translated result without any additional explanation:\n\n{text}"));
        assert!(translation_prompt(None, "zh", "a", None, false).starts_with("将"));
        assert!(translation_prompt(Some("zh"), "en", "a", None, true).starts_with("Translate"));
    }
    #[test]
    fn structured_background() {
        assert_eq!(translation_prompt(Some("de"),"fr","now",Some("earlier"),false),"[Background Information]\nearlier\n\nPlease translate the following text into French, taking the provided background information into consideration.\n\n[Source Text]\nnow");
        assert_eq!(
            translation_prompt(None, "zh", "now", Some("earlier"), false),
            "【背景信息】\nearlier\n\n请结合背景信息将以下文本翻译为中文。\n\n【待翻译文本】\nnow"
        );
    }
}
