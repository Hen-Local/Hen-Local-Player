use hlp_core::events::*;
use parking_lot::RwLock;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub struct Bridge {
    pub app: AppHandle,
    pub language: RwLock<Option<String>>,
}
pub fn emit<T: Serialize + Clone>(app: &AppHandle, name: &str, payload: T) {
    if let Err(e) = app.emit_to("main", name, payload) {
        tracing::warn!(event=name,error=%e,"Could not deliver player event");
    }
}
impl PipelineEvents for Bridge {
    fn media_opened(&self, e: MediaOpened) {
        *self.language.write() = e.language.detected.as_ref().map(|c| c.0.clone());
        emit(&self.app, "media://opened", e)
    }
    fn language(&self, e: LanguageEvent) {
        *self.language.write() = e.detected.as_ref().map(|c| c.0.clone());
        emit(&self.app, "pipeline://language", e)
    }
    fn cues(&self, e: CueBatch) {
        emit(&self.app, "pipeline://cues", e)
    }
    fn status(&self, e: PipelineStatus) {
        if e.stage == "complete" {
            tracing::info!(session=%e.session_id, utterances=e.utt_total, asr_speed=e.asr_rtf, translation_ms=e.ms_per_translation, "Subtitle processing completed");
        }
        emit(&self.app, "pipeline://status", e)
    }
    fn playback_prep(&self, e: PlaybackPrep) {
        emit(&self.app, "media://prep", e)
    }
    fn error(&self, e: ErrorEvent) {
        tracing::warn!(code=%e.code,recoverable=e.recoverable,"Pipeline error");
        emit(&self.app, "app://error", e)
    }
}
