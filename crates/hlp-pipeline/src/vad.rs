use anyhow::{Context, Result};
use hlp_core::types::*;
use sha2::{Digest, Sha256};
use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};
use std::path::Path;

pub const WINDOW: usize = 512;
const GROUP_GAP: Ms = 500;
const GROUP_SPAN: Ms = 12000;
pub fn params_hash(path: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(b"silero:16000:512:0.5:0.25:0.25:15;group:500:12000:isolated:300:1000:v1");
    hash.update(std::fs::read(path).context("Silero VAD needs setup")?);
    Ok(hex::encode(hash.finalize()))
}

pub struct Detector {
    vad: VoiceActivityDetector,
    group: Vec<SpeechRegion>,
    previous: Option<Vec<SpeechRegion>>,
    next_id: usize,
}
impl Detector {
    pub fn new(path: &Path) -> Result<Self> {
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(path.to_string_lossy().into_owned()),
                threshold: 0.5,
                min_silence_duration: 0.25,
                min_speech_duration: 0.25,
                max_speech_duration: 15.,
                window_size: WINDOW as i32,
            },
            sample_rate: 16000,
            num_threads: 1,
            provider: Some("cpu".into()),
            ..Default::default()
        };
        Ok(Self {
            vad: VoiceActivityDetector::create(&config, 60.).context("Load Silero VAD")?,
            group: vec![],
            previous: None,
            next_id: 0,
        })
    }
    fn emit(&mut self, regions: Vec<SpeechRegion>, emit: &mut impl FnMut(Utterance)) {
        if regions.is_empty() {
            return;
        }
        let id = format!("u{:06}", self.next_id);
        self.next_id += 1;
        emit(Utterance {
            id,
            start_ms: regions[0].start_ms,
            end_ms: regions.last().unwrap().end_ms,
            regions,
            status: UttStatus::Pending,
            text: None,
            language: None,
            forced_language: None,
            asr_model: None,
        });
    }
    fn close(&mut self, next: Option<Ms>, emit: &mut impl FnMut(Utterance)) {
        let mut group = std::mem::take(&mut self.group);
        if group.is_empty() {
            return;
        }
        let short = group.iter().map(|r| r.end_ms - r.start_ms).sum::<u64>() < 300;
        let left = self.previous.as_ref().map_or(u64::MAX, |p| {
            group[0].start_ms.saturating_sub(p.last().unwrap().end_ms)
        });
        let right = next.map_or(u64::MAX, |ms| {
            ms.saturating_sub(group.last().unwrap().end_ms)
        });
        if short && left.min(right) > 1000 {
            return;
        }
        if short && left <= right {
            if let Some(p) = &mut self.previous {
                p.append(&mut group);
                return;
            }
        }
        if let Some(p) = self.previous.take() {
            self.emit(p, emit)
        }
        if short && next.is_some() {
            self.group = group
        } else {
            self.previous = Some(group)
        }
    }
    fn drain(&mut self, emit: &mut impl FnMut(Utterance)) {
        while let Some(segment) = self.vad.front() {
            let start = segment.start().max(0) as u64;
            let end = start + segment.n().max(0) as u64;
            self.vad.pop();
            let r = SpeechRegion {
                start_ms: start / 16,
                end_ms: end / 16,
            };
            if r.start_ms >= r.end_ms {
                continue;
            }
            if !self.group.is_empty()
                && (r.start_ms.saturating_sub(self.group.last().unwrap().end_ms) > GROUP_GAP
                    || r.end_ms - self.group[0].start_ms > GROUP_SPAN)
            {
                self.close(Some(r.start_ms), emit)
            }
            self.group.push(r);
        }
    }
    pub fn feed(&mut self, frame: &[f32], position: Ms, mut emit: impl FnMut(Utterance)) {
        self.vad.accept_waveform(frame);
        self.drain(&mut emit);
        // After a second of confirmed silence an isolated sub-300 ms neighbor cannot attach.
        if !self.vad.detected()
            && self
                .group
                .last()
                .is_some_and(|r| position.saturating_sub(r.end_ms) > 1000)
        {
            self.close(None, &mut emit);
            if let Some(p) = self.previous.take() {
                self.emit(p, &mut emit)
            }
        }
    }
    pub fn finish(&mut self, mut emit: impl FnMut(Utterance)) {
        self.vad.flush();
        self.drain(&mut emit);
        self.close(None, &mut emit);
        if let Some(p) = self.previous.take() {
            self.emit(p, &mut emit)
        }
    }
}
