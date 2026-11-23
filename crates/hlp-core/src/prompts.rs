pub const PROMPT_VERSION: u32 = 1;
pub const TEMPERATURE: f32 = 0.;
pub const REPEAT_PENALTY: f32 = 1.05;
pub const MAX_TOKENS: u32 = 384;
pub fn translation_prompt(
    source: Option<&str>,
    target: &str,
    text: &str,
    background: Option<&str>,
    force_english: bool,
) -> String {
    todo!()
}
