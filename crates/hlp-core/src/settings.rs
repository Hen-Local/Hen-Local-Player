use crate::types::BilingualOrder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
/// Player preferences. Missing fields receive platform defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub ui_language: String,
    pub accent_theme: String,
    pub target_language: String,
    pub source_language: String,
    pub subtitle_mode: String,
    pub bilingual_order: BilingualOrder,
    pub subtitle_scale: f32,
    pub subtitle_background: f32,
    pub subtitle_bottom_offset: f32,
    pub show_transcript_panel: bool,
    pub lookahead_seconds: u32,
    pub process_whole_file: bool,
    pub dialogue_channel_focus: bool,
    pub asr_model: String,
    pub translation_context: bool,
    pub download_dir: PathBuf,
    pub cache_limit_gb: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_language: "system".into(),
            accent_theme: "neon-blue".into(),
            target_language: crate::lang::default_target(
                &sys_locale::get_locale().unwrap_or_default(),
            )
            .0,
            source_language: "auto".into(),
            subtitle_mode: "both".into(),
            bilingual_order: BilingualOrder::TranslationFirst,
            subtitle_scale: 1.,
            subtitle_background: 0.,
            subtitle_bottom_offset: 0.05,
            show_transcript_panel: false,
            lookahead_seconds: 180,
            process_whole_file: true,
            dialogue_channel_focus: true,
            asr_model: if cfg!(windows) {
                "sherpa-qwen3-asr-0.6b"
            } else {
                "qwen3-asr-1.7b"
            }
            .into(),
            translation_context: true,
            download_dir: dirs::video_dir()
                .or_else(|| dirs::home_dir().map(|p| p.join("Movies")))
                .unwrap_or_default()
                .join("Hen Local Player"),
            cache_limit_gb: 30,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_preferences_get_defaults() {
        let s: Settings = serde_json::from_str(r#"{"subtitleScale":1.6}"#).unwrap();
        assert_eq!(s.subtitle_scale, 1.6);
        assert_eq!(s.subtitle_bottom_offset, 0.05);
        assert_eq!(s.lookahead_seconds, 180);
        assert_eq!(s.source_language, "auto");
        assert_eq!(
            serde_json::to_value(&s).unwrap()["bilingualOrder"],
            "translation-first"
        );
    }
}
