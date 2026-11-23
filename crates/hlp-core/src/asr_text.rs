use crate::types::LangCode;
#[derive(Debug, PartialEq)]
pub enum ParsedAsr {
    Text {
        text: String,
        language: Option<LangCode>,
    },
    NoSpeech,
    Failed(String),
}
pub fn parse(raw: &str, speech_seconds: f64) -> ParsedAsr {
    todo!()
}
