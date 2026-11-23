//! Recognition state, language voting and playhead-priority work selection.
use crate::store::{self, Audio, Segments, Transcript, Translations};
use anyhow::{ensure, Result};
use hlp_core::{asr_text::ParsedAsr, events::*, lang, settings::Settings, types::*};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};
pub fn validate_settings(s: &Settings) -> Result<()> {
    ensure!(
        s.target_language == "none"
            || lang::lookup(&s.target_language).is_some_and(|l| l.hy_mt2_target),
        "Unknown translation target"
    );
    ensure!(
        matches!(s.source_language.as_str(), "auto" | "auto-per-line")
            || lang::lookup(&s.source_language).is_some_and(|l| l.qwen3_asr_name.is_some()),
        "Unknown spoken language"
    );
    ensure!(
        matches!(
            s.asr_model.as_str(),
            "qwen3-asr-1.7b" | "qwen3-asr-0.6b" | "sherpa-qwen3-asr-0.6b"
        ),
        "Unknown ASR model"
    );
    Ok(())
}
pub fn initial_language(id: &str, mode: &str) -> LanguageEvent {
    let code = lang::lookup(mode).map(|l| LangCode(l.code.into()));
    LanguageEvent {
        session_id: id.into(),
        mode: mode.into(),
        detected: code.clone(),
        confidence: if code.is_some() { 1. } else { 0. },
        locked: code.is_some(),
        unverified_translation: code
            .as_ref()
            .is_some_and(|c| lang::lookup(&c.0).is_some_and(|l| !l.hy_mt2_target)),
    }
}
pub fn speech_ms(u: &Utterance) -> u64 {
    u.regions
        .iter()
        .map(|r| r.end_ms.saturating_sub(r.start_ms))
        .sum()
}
pub fn guessed(text: &str) -> Option<(LangCode, f64)> {
    if hlp_core::cues::display_cols(text) < 10 {
        return None;
    }
    let info = whatlang::detect(text)?;
    let code = lang::languages()
        .iter()
        .find(|l| l.whatlang == Some(info.lang()))?;
    (info.confidence() >= 0.5).then(|| (LangCode(code.code.into()), info.confidence()))
}
pub struct Timeline {
    pub track: PathBuf,
    pub vad_hash: String,
    pub rows: BTreeMap<String, Utterance>,
    pub language: LanguageEvent,
    pub translations: Translations,
    pub target: String,
    pub model: String,
    pub audio_complete: bool,
    pub vad_complete: bool,
    pub extracted: Ms,
    pub language_dirty: bool,
    pub dirty: bool,
    pub asr_ms: u64,
    pub audio_ms: u64,
    pub tr_ms: u64,
    pub tr_count: u64,
}
impl Timeline {
    pub fn load(track: PathBuf, s: &Settings, id: &str) -> Result<Self> {
        std::fs::create_dir_all(&track)?;
        let segments = store::read::<Segments>(&track.join("segments.json"));
        let hash = crate::vad::params_hash(&hlp_models::vad_path()).unwrap_or_else(|_| {
            segments
                .as_ref()
                .map(|s| s.vad_params_hash.clone())
                .unwrap_or_default()
        });
        let audio = store::read::<Audio>(&track.join("audio.json"))
            .filter(|a| a.dialogue_focus == s.dialogue_channel_focus && a.sample_rate == 16000);
        let complete = audio.as_ref().is_some_and(|a| {
            a.complete
                && std::fs::metadata(track.join("audio.pcm"))
                    .is_ok_and(|m| m.len() == a.total_samples.saturating_mul(2))
        });
        let segments = segments.filter(|v| v.vad_params_hash == hash && audio.is_some());
        let vad_complete = complete && segments.as_ref().is_some_and(|v| v.complete);
        let mut rows: BTreeMap<_, _> = segments
            .into_iter()
            .flat_map(|s| s.utterances)
            .map(|u| (u.id.clone(), u))
            .collect();
        let active_path = track.join("transcript.json");
        let mut transcript = store::read::<Transcript>(&active_path);
        if let Some(other) = transcript.as_ref().filter(|t| t.asr_model != s.asr_model) {
            if ["qwen3-asr-1.7b", "qwen3-asr-0.6b", "sherpa-qwen3-asr-0.6b"]
                .contains(&other.asr_model.as_str())
            {
                store::write_json(
                    &track.join(format!("transcript.{}.json", other.asr_model)),
                    other,
                )?;
            }
            transcript = store::read(&track.join(format!("transcript.{}.json", s.asr_model)));
        }
        let mut previous_mode = String::new();
        if let Some(t) =
            transcript.filter(|v| v.asr_model == s.asr_model && v.vad_params_hash == hash)
        {
            previous_mode = t.source_mode;
            for (id, u) in t.items {
                if let Some(row) = rows.get_mut(&id) {
                    if row.regions == u.regions {
                        *row = u
                    }
                }
            }
        }
        let translations = Translations::load(
            &Self::tr_path(&track, &s.target_language),
            s.translation_context,
        );
        let mut t = Self {
            track,
            vad_hash: hash,
            rows,
            language: initial_language(id, &s.source_language),
            translations,
            target: s.target_language.clone(),
            model: s.asr_model.clone(),
            audio_complete: complete,
            vad_complete,
            extracted: if complete {
                audio.unwrap().total_samples / 16
            } else {
                0
            },
            dirty: true,
            language_dirty: true,
            asr_ms: 0,
            audio_ms: 0,
            tr_ms: 0,
            tr_count: 0,
        };
        // Older transcripts did not record the source mode. An unforced probe is
        // evidence that an automatic session produced its subsequent locked rows.
        if previous_mode.is_empty()
            && t.rows
                .values()
                .any(|u| u.status == UttStatus::Done && u.forced_language.is_none())
        {
            previous_mode = "auto".into();
        }
        if previous_mode != s.source_language || lang::lookup(&s.source_language).is_some() {
            t.requeue_mismatches(true);
        }
        t.vote();
        Ok(t)
    }
    fn tr_path(track: &Path, target: &str) -> PathBuf {
        track.join("translations").join(format!("{target}.json"))
    }
    pub fn set_target(&mut self, target: &str, context: bool) -> Result<()> {
        self.save_translations()?;
        self.target = target.into();
        self.translations = Translations::load(&Self::tr_path(&self.track, target), context);
        self.dirty = true;
        Ok(())
    }
    fn save_translations(&self) -> Result<()> {
        if self.target != "none" && !self.translations.items.is_empty() {
            store::write_json(
                &Self::tr_path(&self.track, &self.target),
                &self.translations,
            )?
        }
        Ok(())
    }
    pub fn persist(&self) -> Result<()> {
        let utterances = self
            .rows
            .values()
            .map(|u| Utterance {
                status: UttStatus::Pending,
                text: None,
                language: None,
                forced_language: None,
                asr_model: None,
                ..u.clone()
            })
            .collect();
        store::write_json(
            &self.track.join("segments.json"),
            &Segments {
                vad_params_hash: self.vad_hash.clone(),
                complete: self.vad_complete,
                utterances,
            },
        )?;
        store::write_json(
            &self.track.join("transcript.json"),
            &Transcript {
                source_mode: if self.language.mode == "mixed" {
                    "auto".into()
                } else {
                    self.language.mode.clone()
                },
                asr_model: self.model.clone(),
                vad_params_hash: self.vad_hash.clone(),
                items: self.rows.clone(),
            },
        )?;
        self.save_translations()
    }
    pub fn set_source(&mut self, mode: &str) {
        self.language = initial_language(&self.language.session_id, mode);
        self.language_dirty = true;
        self.requeue_mismatches(true);
        if mode == "auto" {
            self.vote()
        }
        self.dirty = true;
    }
    fn requeue_mismatches(&mut self, all: bool) {
        if cfg!(windows) {
            return;
        }
        for u in self.rows.values_mut() {
            if u.status == UttStatus::Done
                && if all {
                    u.forced_language != self.language.detected
                } else {
                    u.language != self.language.detected
                }
            {
                u.status = UttStatus::Pending
            }
        }
    }
    fn vote(&mut self) {
        if self.language.locked || self.language.mode != "auto" {
            return;
        }
        let mut weights: HashMap<LangCode, u64> = HashMap::new();
        let mut n = 0;
        let mut total = 0;
        let mut tentative = None;
        for u in self.rows.values().filter(|u| u.status == UttStatus::Done) {
            let Some(l) = &u.language else { continue };
            tentative.get_or_insert_with(|| l.clone());
            let weight = speech_ms(u);
            if weight < 1500 {
                continue;
            }
            *weights.entry(l.clone()).or_default() += weight;
            total += weight;
            n += 1;
            if n == 6 && weights.values().max().is_some_and(|m| m * 10 >= total * 6) || n == 12 {
                break;
            }
        }
        let leader = weights
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0 .0.cmp(&a.0 .0)));
        let (code, confidence) =
            leader.map_or((tentative, 0.), |(l, w)| (Some(l), w as f32 / total as f32));
        self.language.detected = code;
        self.language.confidence = confidence;
        self.language.locked = matches!(n, 6 | 12) && confidence >= 0.6;
        if n == 12 && !self.language.locked {
            self.language.mode = "mixed".into();
            self.language.detected = None
        }
        self.language.unverified_translation = self
            .language
            .detected
            .as_ref()
            .is_some_and(|c| lang::lookup(&c.0).is_some_and(|l| !l.hy_mt2_target));
        if self.language.locked {
            self.requeue_mismatches(false)
        }
        self.language_dirty = true;
    }
    pub fn source_for<'a>(&'a self, u: &'a Utterance) -> Option<&'a str> {
        if self.language.locked {
            self.language.detected.as_ref()
        } else {
            u.language.as_ref()
        }
        .map(|c| c.0.as_str())
    }
    pub fn tr_needed(&self, u: &Utterance) -> bool {
        u.status == UttStatus::Done
            && self.target != "none"
            && self.source_for(u) != Some(self.target.as_str())
            && u.text.as_ref().is_some_and(|v| {
                self.translations
                    .items
                    .get(&u.id)
                    .is_none_or(|item| item.source_hash != store::hash(v))
            })
    }
    pub fn pick(&self, translation: bool, playhead: Ms, s: &Settings) -> Option<&Utterance> {
        self.rows
            .values()
            .filter(|u| {
                if translation {
                    self.tr_needed(u)
                } else {
                    u.status == UttStatus::Pending
                        && (self.audio_complete || u.end_ms <= self.extracted)
                }
            })
            .filter(|u| {
                s.process_whole_file
                    || u.start_ms <= playhead.saturating_add(s.lookahead_seconds as u64 * 1000)
            })
            .filter_map(|u| {
                hlp_core::schedule::priority(u.start_ms, playhead, s.process_whole_file)
                    .map(|p| (p, u))
            })
            .min_by_key(|(p, _)| *p)
            .map(|(_, u)| u)
    }
    pub fn apply(&mut self, id: &str, result: ParsedAsr, forced: Option<LangCode>) {
        let Some(u) = self.rows.get_mut(id) else {
            return;
        };
        u.asr_model = Some(self.model.clone());
        u.forced_language = forced.clone();
        match result {
            ParsedAsr::Text { text, language } => {
                let guess = guessed(&text);
                if let (Some(l), Some((g, c))) = (&language, &guess) {
                    if *c >= 0.9 && l != g {
                        tracing::debug!(asr=%l.0,estimated=%g.0,"Transcript language disagreement")
                    }
                }
                u.text = Some(text);
                u.language = forced.or(language).or_else(|| guess.map(|(g, _)| g));
                u.status = UttStatus::Done;
            }
            ParsedAsr::NoSpeech => {
                u.text = None;
                u.language = None;
                u.status = UttStatus::NoSpeech
            }
            ParsedAsr::Failed(e) => {
                u.status = UttStatus::Failed {
                    attempts: 2,
                    error: e,
                }
            }
        }
        if matches!(self.language.mode.as_str(), "auto-per-line" | "mixed") {
            self.language.detected = u.language.clone();
            self.language.confidence = 0.;
            self.language_dirty = true;
        }
        self.vote();
        self.dirty = true;
    }
    fn translation(&self, u: &Utterance) -> Option<&String> {
        if self.target == "none" || self.source_for(u) == Some(self.target.as_str()) {
            return None;
        }
        let source = u.text.as_ref()?;
        self.translations
            .items
            .get(&u.id)
            .filter(|v| !v.failed && v.source_hash == store::hash(source))
            .map(|v| &v.text)
    }
    pub fn cues(&self) -> Vec<Cue> {
        let texts = self
            .rows
            .values()
            .filter_map(|u| self.translation(u).map(|v| (u.id.clone(), v.clone())))
            .collect();
        hlp_core::cues::build_cues(&self.rows.values().cloned().collect::<Vec<_>>(), &texts)
    }
    pub fn status(&self, stage: &str) -> PipelineStatus {
        fn append(spans: &mut Vec<[Ms; 2]>, u: &Utterance) {
            if let Some(last) = spans.last_mut().filter(|l| u.start_ms <= l[1] + 80) {
                last[1] = last[1].max(u.end_ms);
                return;
            }
            spans.push([u.start_ms, u.end_ms]);
        }
        let (mut done, mut silent, mut failed, mut translated) = (0, 0, 0, 0);
        let (mut asr, mut tr) = (vec![], vec![]);
        for u in self.rows.values() {
            match u.status {
                UttStatus::Done => done += 1,
                UttStatus::NoSpeech => silent += 1,
                UttStatus::Failed { .. } => failed += 1,
                UttStatus::Pending => append(&mut asr, u),
            }
            if self.translation(u).is_some() {
                translated += 1
            } else if self.tr_needed(u) {
                append(&mut tr, u)
            }
        }
        let rtf = if self.asr_ms > 0 {
            self.audio_ms as f32 / self.asr_ms as f32
        } else {
            0.
        };
        let mean = if self.tr_count > 0 {
            self.tr_ms as f32 / self.tr_count as f32
        } else {
            0.
        };
        let pending = asr.iter().map(|r| r[1] - r[0]).sum::<u64>();
        PipelineStatus {
            session_id: self.language.session_id.clone(),
            extracted_until_ms: self.extracted,
            vad_complete: self.vad_complete,
            utt_total: self.rows.len(),
            utt_done: done,
            utt_no_speech: silent,
            utt_failed: failed,
            translated_count: translated,
            asr_pending_spans: asr,
            tr_pending_spans: tr.clone(),
            asr_rtf: rtf,
            ms_per_translation: mean,
            eta_seconds: (rtf > 0.)
                .then(|| ((pending as f32 / rtf + tr.len() as f32 * mean) / 1000.).ceil() as u32),
            stage: stage.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utterance(i: usize, language: &str) -> Utterance {
        let start = i as u64 * 3000;
        Utterance {
            id: format!("u{i:06}"),
            start_ms: start,
            end_ms: start + 1800,
            regions: vec![SpeechRegion {
                start_ms: start,
                end_ms: start + 1800,
            }],
            status: UttStatus::Done,
            text: Some(format!("phrase {i}")),
            language: Some(LangCode(language.into())),
            forced_language: None,
            asr_model: Some("qwen3-asr-1.7b".into()),
        }
    }

    #[test]
    fn uncertain_first_six_wait_until_twelfth_vote() {
        let temp = tempfile::tempdir().unwrap();
        let mut t =
            Timeline::load(temp.path().join("track"), &Settings::default(), "test").unwrap();
        for (i, lang) in [
            "en", "ja", "en", "ja", "en", "ja", "en", "en", "en", "en", "en", "en",
        ]
        .into_iter()
        .enumerate()
        {
            let u = utterance(i, lang);
            t.rows.insert(u.id.clone(), u);
            t.vote();
            assert_eq!(t.language.locked, i == 11);
        }
        assert_eq!(t.language.detected.unwrap().0, "en");
    }

    #[test]
    #[cfg(not(windows))]
    fn reset_manual_source_queues_forced_text_and_preserves_stale_cues() {
        let temp = tempfile::tempdir().unwrap();
        let mut t =
            Timeline::load(temp.path().join("track"), &Settings::default(), "test").unwrap();
        let mut u = utterance(0, "ja");
        u.forced_language = u.language.clone();
        t.rows.insert(u.id.clone(), u);
        t.set_source("auto");
        assert_eq!(t.rows.values().next().unwrap().status, UttStatus::Pending);
        assert!(t.language.detected.is_none());
        assert!(t.cues()[0].stale);
    }
}
