use serde::{Deserialize, Serialize};
/// Milliseconds from the beginning of a media file.
pub type Ms = u64;
/// Browser canPlayType results indexed by the exact MIME query.
pub type Caps = std::collections::BTreeMap<String, String>;
/// Canonical language code used in persisted and wire data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LangCode(pub String);
/// Half-open speech interval on the media clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechRegion {
    pub start_ms: Ms,
    pub end_ms: Ms,
}
/// A stable ASR and translation work unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Utterance {
    pub id: String,
    pub start_ms: Ms,
    pub end_ms: Ms,
    pub regions: Vec<SpeechRegion>,
    pub status: UttStatus,
    pub text: Option<String>,
    pub language: Option<LangCode>,
    pub forced_language: Option<LangCode>,
    pub asr_model: Option<String>,
}
/// Display-ready subtitle with a stable, utterance-relative ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: String,
    pub utterance_id: String,
    pub start_ms: Ms,
    pub end_ms: Ms,
    pub original: String,
    pub translation: Option<String>,
    pub lang: Option<LangCode>,
    pub stale: bool,
}
/// Video codec metadata from ffprobe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStream {
    pub codec_name: String,
    pub codec_tag_string: String,
    pub profile: String,
    pub pix_fmt: String,
    pub width: u32,
    pub height: u32,
    pub field_order: String,
}
/// Audio stream and its audio-relative order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStream {
    pub index: usize,
    pub order: usize,
    pub codec_name: String,
    pub channels: u32,
    pub channel_layout: String,
    pub sample_rate: u32,
    pub language_tag: Option<String>,
    pub default: bool,
}
/// Container and stream information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub container: String,
    pub duration_ms: Ms,
    pub video: Option<VideoStream>,
    pub audio: Vec<AudioStream>,
    pub has_subtitle_streams: bool,
}
/// Metadata presented before confirming a download. Duration is seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub extractor: String,
    pub thumbnail: Option<String>,
    pub is_live: bool,
}
/// Readiness and verified byte progress for one installable component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentStatus {
    pub id: String,
    pub ready: bool,
    pub bytes_total: u64,
    pub bytes_done: u64,
    pub state: ComponentState,
}
/// Progress of the current audio or video download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub part: u32,
    pub parts: u32,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub eta: Option<u64>,
}
/// Recently opened media with last known completion fraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentItem {
    pub fingerprint: String,
    pub path: String,
    pub title: String,
    pub duration_ms: Ms,
    pub last_opened: u64,
    pub progress: f32,
}
/// UttStatus in the player protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UttStatus {
    Pending,
    Done,
    NoSpeech,
    Failed { attempts: u8, error: String },
}
/// PlaybackPlan in the player protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackPlan {
    Direct,
    Remux,
    Transcode,
}
/// ComponentState in the player protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentState {
    Missing,
    Downloading,
    Verifying,
    Ready,
    Failed(String),
}
/// SessionCommand in the player protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionCommand {
    Playhead {
        ms: Ms,
        playing: bool,
    },
    SetTarget(String),
    SetSource(String),
    SelectAudioTrack(usize),
    PlaybackError(u16),
    StartTranscription {
        components_ready: bool,
        #[serde(default)]
        regenerate: bool,
        #[serde(default)]
        audio_track_order: Option<usize>,
    },
    RetryFailed,
    UpdateSettings(crate::settings::Settings),
    Export {
        format: SubtitleFormat,
        variant: SubtitleVariant,
        path: std::path::PathBuf,
    },
    Shutdown,
}
/// SubtitleFormat in the player protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubtitleFormat {
    Srt,
    Vtt,
    Ass,
}
/// SubtitleVariant in the player protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SubtitleVariant {
    Original,
    Translation,
    Bilingual(BilingualOrder),
}
/// BilingualOrder in the player protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BilingualOrder {
    TranslationFirst,
    OriginalFirst,
}
