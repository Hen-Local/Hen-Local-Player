// Adapted from Hen-Local-Translator/node-hub/dora-qwen3-asr/src/backend/macos.rs.
use anyhow::Result;
use qwen3_asr_mlx::{Qwen3ASR, SamplingConfig};
use std::path::Path;

pub const NAME: &str = "qwen3-mlx";
pub const CAN_FORCE_LANGUAGE: bool = true;
pub const REPORTS_LANGUAGE: bool = true;

pub struct AsrEngine(Qwen3ASR);

impl AsrEngine {
    pub fn load(dir: &Path) -> Result<Self> {
        Ok(Self(Qwen3ASR::load(dir)?))
    }

    pub fn transcribe(
        &mut self,
        samples: &[f32],
        language: Option<&str>,
        max_tokens: u32,
    ) -> Result<String> {
        let config = SamplingConfig {
            temperature: 0.0,
            max_tokens: max_tokens as usize,
        };
        Ok(self.0.transcribe_raw(samples, language, &config)?.raw)
    }
}

pub fn clear_cache() {
    // MLX's process-wide allocator has no other users in the single-threaded worker.
    unsafe {
        mlx_sys::mlx_clear_cache();
    }
}
