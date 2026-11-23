use serde::{Deserialize, Serialize};
use std::path::PathBuf;
/// Every request has one response bearing the same ID.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    #[serde(flatten)]
    pub op: Operation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Operation {
    Hello,
    Load,
    Unload,
    Shutdown,
    Transcribe {
        pcm_path: PathBuf,
        start_sample: u64,
        end_sample: u64,
        language: Option<String>,
        max_tokens: u32,
    },
}
/// A success carries typed result data. Failures carry displayable errors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
/// Worker capabilities negotiated at startup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub protocol: u32,
    pub backend: String,
    pub platform: String,
    pub can_force_language: bool,
    pub reports_language: bool,
}
/// Raw model output and timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub raw: String,
    pub elapsed_ms: u64,
    pub audio_ms: u64,
}
/// Model load duration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loaded {
    pub load_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    #[test]
    fn round_trip() {
        for v in [
            json!({"id":1,"op":"load"}),
            json!({"id":3,"op":"transcribe","pcmPath":"/tmp/é","startSample":2,"endSample":30,"language":null,"maxTokens":68}),
        ] {
            let req: Request = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(serde_json::to_value(req).unwrap(), v);
        }
        for v in [
            json!({"id":7,"ok":true,"result":{"raw":"日本語"}}),
            json!({"id":7,"ok":false,"error":"missing PCM"}),
        ] {
            let res: Response = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(serde_json::to_value(res).unwrap(), v);
        }
        assert!(serde_json::from_value::<Request>(Value::Null).is_err());
    }
}
