// Platform interface for the sherpa-onnx implementation in lane G.
use anyhow::{bail, Result};
use std::path::Path;

pub const NAME: &str = "sherpa-qwen3";
pub const CAN_FORCE_LANGUAGE: bool = false;
pub const REPORTS_LANGUAGE: bool = false;
pub struct AsrEngine;
impl AsrEngine {
    pub fn load(_dir: &Path) -> Result<Self> {
        bail!("Windows ASR backend is not installed")
    }
    pub fn transcribe(
        &mut self,
        _samples: &[f32],
        _language: Option<&str>,
        _max_tokens: u32,
    ) -> Result<String> {
        bail!("Windows ASR backend is not installed")
    }
}
pub fn clear_cache() {}
