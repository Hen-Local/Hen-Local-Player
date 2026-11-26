use crate::types::BilingualOrder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelDownloadSource {
    #[default]
    Auto,
    China,
    Global,
}

/// Player preferences. Missing fields receive platform defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub ui_language: String,
    pub auto_download_updates: bool,
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
    pub model_download_source: ModelDownloadSource,
    pub translation_context: bool,
    pub download_dir: PathBuf,
    pub cache_limit_gb: u32,
}
impl Default for Settings {
    fn default() -> Self {
        let (source, target) = crate::lang::default_subtitle_languages(sys_locale::get_locales());
        Self {
            ui_language: "system".into(),
            auto_download_updates: false,
            accent_theme: if cfg!(target_os = "macos") {
                "system"
            } else {
                "neon-green"
            }
            .into(),
            target_language: target.into(),
            source_language: source.into(),
            subtitle_mode: "translation".into(),
            bilingual_order: BilingualOrder::TranslationFirst,
            subtitle_scale: 1.,
            subtitle_background: 0.7,
            subtitle_bottom_offset: 0.05,
            show_transcript_panel: false,
            lookahead_seconds: 180,
            process_whole_file: true,
            dialogue_channel_focus: true,
            model_download_source: ModelDownloadSource::Auto,
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
        #[cfg(target_os = "macos")]
        assert_eq!(s.accent_theme, "system");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(s.accent_theme, "neon-green");
        assert_eq!(s.subtitle_scale, 1.6);
        assert_eq!(s.subtitle_bottom_offset, 0.05);
        assert_eq!(s.subtitle_background, 0.7);
        assert_eq!(s.lookahead_seconds, 180);
        let defaults = Settings::default();
        assert_eq!(s.target_language, defaults.target_language);
        assert_eq!(s.source_language, defaults.source_language);
        assert_ne!(s.source_language, s.target_language);
        assert_eq!(s.subtitle_mode, "translation");
        assert!(!s.auto_download_updates);
        assert_eq!(
            serde_json::to_value(&s).unwrap()["bilingualOrder"],
            "translation-first"
        );
    }

    #[test]
    fn saved_subtitle_preferences_survive_new_defaults() {
        let settings: Settings = serde_json::from_str(
            r#"{"sourceLanguage":"auto","targetLanguage":"ja","subtitleMode":"both","bilingualOrder":"original-first"}"#,
        ).unwrap();
        assert_eq!(settings.source_language, "auto");
        assert_eq!(settings.target_language, "ja");
        assert_eq!(settings.subtitle_mode, "both");
        assert_eq!(settings.bilingual_order, BilingualOrder::OriginalFirst);
    }

    #[test]
    fn explicit_accent_survives_default_change() {
        for theme in [
            "neon-blue",
            "neon-orange",
            "neon-pink",
            "neon-green",
            "system",
        ] {
            let s: Settings =
                serde_json::from_value(serde_json::json!({ "accentTheme": theme })).unwrap();
            assert_eq!(s.accent_theme, theme);
        }
    }
}
