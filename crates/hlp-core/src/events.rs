use crate::types::*;
use serde::{Deserialize, Serialize};
/// Language vote and lock state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageEvent {
    pub session_id: String,
    pub mode: String,
    pub detected: Option<LangCode>,
    pub confidence: f32,
    pub locked: bool,
    pub unverified_translation: bool,
}
/// Initial snapshot, including any restored cues.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaOpened {
    pub session_id: String,
    pub title: String,
    pub duration_ms: Ms,
    pub media_url: Option<String>,
    pub plan: PlaybackPlan,
    pub audio_tracks: Vec<AudioStream>,
    pub language: LanguageEvent,
    pub target: String,
    pub cues: Vec<Cue>,
}
/// Progress fraction or URL of the prepared media.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPrep {
    pub session_id: String,
    pub plan: PlaybackPlan,
    pub progress: f32,
    pub media_url: Option<String>,
    pub error: Option<String>,
}
/// Incremental cue changes for one session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CueBatch {
    pub session_id: String,
    pub upserts: Vec<Cue>,
    pub removed_ids: Vec<String>,
}
/// Coverage, measured throughput and stage of processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStatus {
    pub session_id: String,
    pub extracted_until_ms: Ms,
    pub vad_complete: bool,
    pub utt_total: usize,
    pub utt_done: usize,
    pub utt_no_speech: usize,
    pub utt_failed: usize,
    pub translated_count: usize,
    pub asr_pending_spans: Vec<[Ms; 2]>,
    pub tr_pending_spans: Vec<[Ms; 2]>,
    pub asr_rtf: f32,
    pub ms_per_translation: f32,
    pub eta_seconds: Option<u32>,
    pub stage: String,
}
/// Actionable pipeline failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEvent {
    pub session_id: Option<String>,
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}
/// Session-scoped sinks. Receivers discard events for superseded sessions.
pub trait PipelineEvents: Send + Sync {
    fn media_opened(&self, e: MediaOpened);
    fn playback_prep(&self, e: PlaybackPrep);
    fn cues(&self, e: CueBatch);
    fn status(&self, e: PipelineStatus);
    fn language(&self, e: LanguageEvent);
    fn error(&self, e: ErrorEvent);
}
