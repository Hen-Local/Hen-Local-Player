//! CPU recognizer adapted from Hen-Local-Translator/node-hub/dora-qwen3-asr/src/backend/windows.rs.
use anyhow::{ensure, Context, Result};
use sherpa_onnx::{OfflineQwen3ASRModelConfig, OfflineRecognizer, OfflineRecognizerConfig};
use std::{path::Path, thread};

pub const NAME: &str = "sherpa-qwen3";
pub const CAN_FORCE_LANGUAGE: bool = false;
pub const REPORTS_LANGUAGE: bool = false;

pub struct AsrEngine {
    recognizer: OfflineRecognizer,
}

impl AsrEngine {
    pub fn load(dir: &Path) -> Result<Self> {
        for file in [
            "conv_frontend.onnx",
            "encoder.int8.onnx",
            "decoder.int8.onnx",
            "tokenizer/vocab.json",
            "tokenizer/merges.txt",
            "tokenizer/tokenizer_config.json",
        ] {
            ensure!(
                dir.join(file).is_file(),
                "Missing {file} in {}. Run hlp-cli setup.",
                dir.display()
            );
        }
        let path = |name: &str| Some(dir.join(name).to_string_lossy().into_owned());
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.qwen3_asr = OfflineQwen3ASRModelConfig {
            conv_frontend: path("conv_frontend.onnx"),
            encoder: path("encoder.int8.onnx"),
            decoder: path("decoder.int8.onnx"),
            tokenizer: path("tokenizer"),
            max_new_tokens: 128,
            temperature: 0.0,
            ..Default::default()
        };
        config.model_config.provider = Some("cpu".into());
        config.model_config.tokens = Some(String::new());
        config.model_config.num_threads = thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(2).clamp(1, 8))
            as i32;
        let recognizer = OfflineRecognizer::create(&config).with_context(|| {
            format!(
                "Could not load the Qwen3-ASR ONNX models in {}",
                dir.display()
            )
        })?;
        Ok(Self { recognizer })
    }

    pub fn transcribe(
        &mut self,
        samples: &[f32],
        _language: Option<&str>,
        max_tokens: u32,
    ) -> Result<String> {
        let stream = self.recognizer.create_stream();
        // The pinned native decoder reads both options on each DecodeStream call.
        stream.set_option("max_new_tokens", &max_tokens.to_string());
        stream.set_option("temperature", "0");
        stream.accept_waveform(16_000, samples);
        self.recognizer.decode(&stream);
        let result = stream
            .get_result()
            .context("Sherpa returned no transcription")?;
        Ok(strip_rich_tags(&result.text))
    }
}

// Only remove <|...|> decorations. Some outputs retain the useful <asr_text> prefix.
fn strip_rich_tags(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some((before, tag)) = remaining.split_once("<|") {
        output.push_str(before);
        let Some((_, after)) = tag.split_once("|>") else {
            return output.trim().to_owned();
        };
        remaining = after;
    }
    output.push_str(remaining);
    output.trim().to_owned()
}

pub fn clear_cache() {} // Dropping OfflineRecognizer releases its native sessions.

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_language_prefix_and_plain_text() {
        assert_eq!(
            strip_rich_tags(" <|s|>language Chinese<asr_text>早上好。<|e|> "),
            "language Chinese<asr_text>早上好。"
        );
        assert_eq!(strip_rich_tags(" Good morning. "), "Good morning.");
    }
}
