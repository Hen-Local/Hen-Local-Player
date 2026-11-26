//! Recognition state, language voting and playhead-priority work selection.
use crate::store::{self, Audio, Segments, Transcript, Translations};
use anyhow::{ensure, Result};
use hlp_core::{asr_text::ParsedAsr, events::*, lang, settings::Settings, types::*};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
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
fn speech_segment(u: &Utterance) -> Utterance {
    Utterance {
        id: u.id.clone(),
        start_ms: u.start_ms,
        end_ms: u.end_ms,
        regions: u.regions.clone(),
        status: UttStatus::Pending,
        text: None,
        language: None,
        forced_language: None,
        asr_model: None,
    }
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
// A short dialogue window fits well within the server's 4096-token context,
// including for scripts that use several tokens per character.
const PRECEDING_LINES: usize = 3;
const MAX_CONTEXT_GAP_MS: Ms = 8_000;
const MAX_CONTEXT_AGE_MS: Ms = 30_000;
const MAX_BACKGROUND_BYTES: usize = 1024;

pub struct Timeline {
    pub track: PathBuf,
    pub vad_hash: String,
    pub rows: BTreeMap<String, Utterance>,
    pub language: LanguageEvent,
    pub translations: Translations,
    // Per-row model input signatures. A source edit affects this row and at
    // most the next three. Playback ticks only compare the cached signatures.
    translation_keys: HashMap<String, (String, String)>,
    pub target: String,
    pub model: String,
    pub audio_complete: bool,
    pub vad_complete: bool,
    pub extracted: Ms,
    pub language_dirty: bool,
    pub dirty: bool,
    cue_builder: hlp_core::cues::CueBuilder,
    pub asr_ms: u64,
    pub audio_ms: u64,
    pub tr_ms: u64,
    pub tr_count: u64,
}
impl Timeline {
    pub fn load(track: PathBuf, s: &Settings, id: &str) -> Result<Self> {
        std::fs::create_dir_all(&track)?;
        // Finish an interrupted deletion before the legacy cache loader can
        // restore an old model's transcript or resume from its speech regions.
        let deleting = track.join("subtitles.deleting");
        let finish_deletion = deleting.try_exists()?;
        if finish_deletion {
            store::remove_generated_subtitles(&track)?;
        }
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
        let mut generation = 0;
        if let Some(t) =
            transcript.filter(|v| v.asr_model == s.asr_model && v.vad_params_hash == hash)
        {
            previous_mode = t.source_mode;
            generation = t.generation;
            for (id, u) in t.items {
                if let Some(row) = rows.get_mut(&id) {
                    if row.regions == u.regions {
                        *row = u
                    }
                }
            }
        }
        let translations = Translations::load(
            &track,
            &s.target_language,
            s.translation_context,
            generation,
        );
        let mut t = Self {
            track,
            vad_hash: hash,
            rows,
            language: initial_language(id, &s.source_language),
            translations,
            translation_keys: HashMap::new(),
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
            cue_builder: Default::default(),
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
        t.rebuild_keys();
        if finish_deletion {
            std::fs::remove_file(deleting)?;
        }
        Ok(t)
    }
    pub fn set_target(&mut self, target: &str, context: bool) -> Result<()> {
        self.save_translations()?;
        self.target = target.into();
        let mode_changed = self.translations.context != context;
        self.translations =
            Translations::load(&self.track, target, context, self.translations.generation);
        if mode_changed {
            self.rebuild_keys()
        }
        self.dirty = true;
        Ok(())
    }
    fn save_translations(&self) -> Result<()> {
        if self.target != "none" && !self.translations.items.is_empty() {
            store::write_json(
                &Translations::path(&self.track, &self.target, self.translations.context),
                &self.translations,
            )?
        }
        Ok(())
    }
    pub fn persist(&self) -> Result<()> {
        let utterances = self.rows.values().map(speech_segment).collect();
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
                generation: self.translations.generation,
                items: self.rows.clone(),
            },
        )?;
        self.save_translations()
    }
    pub fn regenerate(&mut self, source_mode: &str) -> Result<()> {
        // A single atomic write requeues recognition and invalidates translations
        // in every language, including after a quit before the next checkpoint.
        let transcript = Transcript {
            source_mode: source_mode.into(),
            asr_model: self.model.clone(),
            vad_params_hash: self.vad_hash.clone(),
            generation: rand::random(),
            items: BTreeMap::new(),
        };
        store::write_json(&self.track.join("transcript.json"), &transcript)?;
        for row in self.rows.values_mut() {
            *row = speech_segment(row);
        }
        self.translations.items.clear();
        self.translations.fallback.clear();
        self.translation_keys.clear();
        self.translations.generation = transcript.generation;
        self.language = initial_language(&self.language.session_id, source_mode);
        self.language_dirty = true;
        self.dirty = true;
        self.asr_ms = 0;
        self.audio_ms = 0;
        self.tr_ms = 0;
        self.tr_count = 0;
        Ok(())
    }
    pub fn set_source(&mut self, mode: &str) {
        self.language = initial_language(&self.language.session_id, mode);
        self.language_dirty = true;
        self.requeue_mismatches(true);
        if mode == "auto" {
            self.vote()
        }
        self.rebuild_keys();
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
    /// VAD IDs are issued in media-time order even when ASR finishes out of order.
    /// Never reach across a missing utterance, another language, or a scene-sized gap.
    fn predecessors<'a>(&'a self, u: &'a Utterance) -> impl Iterator<Item = &'a Utterance> {
        use std::ops::Bound::{Excluded, Unbounded};
        let mut next_start = u.start_ms;
        self.rows
            .range::<str, _>((Unbounded, Excluded(u.id.as_str())))
            .rev()
            .take(PRECEDING_LINES)
            .map(|(_, row)| row)
            .take_while(move |row| {
                let adjacent = row.end_ms <= next_start
                    && next_start - row.end_ms <= MAX_CONTEXT_GAP_MS
                    && u.start_ms.saturating_sub(row.start_ms) <= MAX_CONTEXT_AGE_MS;
                next_start = row.start_ms;
                adjacent
            })
    }

    pub(super) fn background(&self, u: &Utterance) -> Option<String> {
        if !self.translations.context {
            return None;
        }
        let language = self.source_for(u)?;
        let mut remaining = MAX_BACKGROUND_BYTES;
        let mut lines = Vec::with_capacity(PRECEDING_LINES);
        for row in self.predecessors(u) {
            if row.status != UttStatus::Done
                || self.source_for(row) != Some(language)
                || row
                    .language
                    .as_ref()
                    .zip(u.language.as_ref())
                    .is_some_and(|(a, b)| a != b)
            {
                break;
            }
            let Some(line) = row.text.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
                break;
            };
            if line.len() > remaining {
                // Retain the nearest end of a long line, always at a UTF-8 boundary.
                let from = line.ceil_char_boundary(line.len() - remaining);
                if from < line.len() {
                    lines.push(&line[from..])
                }
                break;
            }
            lines.push(line);
            remaining = remaining.saturating_sub(line.len() + 1);
            if remaining == 0 {
                break;
            }
        }
        lines.reverse();
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
    pub(super) fn context_hash(&self, u: &Utterance, bg: Option<&str>) -> String {
        store::hash(&format!(
            "{}\0{}",
            self.source_for(u).unwrap_or(""),
            bg.unwrap_or("")
        ))
    }
    fn key(&self, row: &Utterance) -> Option<(String, String)> {
        Some((
            store::hash(row.text.as_deref()?),
            self.context_hash(row, self.background(row).as_deref()),
        ))
    }
    pub(super) fn rebuild_keys(&mut self) {
        self.translation_keys = self
            .rows
            .values()
            .filter_map(|row| self.key(row).map(|key| (row.id.clone(), key)))
            .collect();
    }
    pub(super) fn refresh_keys_from(&mut self, id: &str) {
        use std::ops::Bound::{Included, Unbounded};
        let affected: Vec<_> = self
            .rows
            .range::<str, _>((Included(id), Unbounded))
            .take(if self.translations.context {
                PRECEDING_LINES + 1
            } else {
                1
            })
            .map(|(id, row)| (id.clone(), self.key(row)))
            .collect();
        for (id, key) in affected {
            if let Some(key) = key {
                self.translation_keys.insert(id, key);
            } else {
                self.translation_keys.remove(&id);
            }
        }
    }
    fn applicable(&self, u: &Utterance) -> bool {
        self.target != "none"
            && self.source_for(u) != Some(self.target.as_str())
            && u.text.is_some()
    }
    fn current_entry(&self, u: &Utterance) -> Option<&store::Translation> {
        let item = self.translations.items.get(&u.id)?;
        let (source_hash, context_hash) = self.translation_keys.get(&u.id)?;
        if &item.source_hash != source_hash {
            return None;
        }
        match &item.context_hash {
            // Historical literal translations are valid in their original mode.
            None if !self.translations.context => Some(item),
            Some(hash) if hash == context_hash => Some(item),
            _ => None,
        }
    }
    pub fn tr_needed(&self, u: &Utterance) -> bool {
        u.status == UttStatus::Done && self.applicable(u) && self.current_entry(u).is_none()
    }
    pub fn translation_failed(&self, u: &Utterance) -> bool {
        u.status == UttStatus::Done
            && self.applicable(u)
            && self.current_entry(u).is_some_and(|item| item.failed)
    }
    pub fn retry_failed_translations(&mut self) {
        self.translations.items.retain(|_, item| !item.failed);
    }
    fn asr_rtf(&self) -> f32 {
        if self.asr_ms == 0 {
            return 0.;
        }
        self.audio_ms as f32 / self.asr_ms as f32
    }
    pub fn defer_translation(&self, candidate: &Utterance, playhead: Ms, lookahead_s: u32) -> bool {
        // Wait for several utterances of evidence. Keep the line playing now
        // and the next two seconds eligible even when both models share a slow GPU.
        if self.audio_ms < 8_000
            || self.asr_rtf() >= 8.
            || (candidate.end_ms >= playhead
                && candidate.start_ms <= playhead.saturating_add(2_000))
        {
            return false;
        }
        let horizon = playhead.saturating_add(u64::from(lookahead_s) * 1000);
        self.rows.values().any(|u| {
            u.status == UttStatus::Pending
                && u.end_ms >= playhead
                && u.start_ms <= horizon
                && (self.audio_complete || u.end_ms <= self.extracted)
        })
    }
    fn waits_for_background(&self, row: &Utterance, playhead: Ms, s: &Settings) -> bool {
        // When processing backwards, the next ASR result may otherwise invalidate
        // every recent translation in turn. Keep urgent subtitles responsive.
        if !self.translations.context
            || self.source_for(row).is_none()
            || (row.end_ms > playhead && row.start_ms <= playhead.saturating_add(2_000))
        {
            return false;
        }
        for previous in self.predecessors(row) {
            if previous.status == UttStatus::Pending {
                return s.process_whole_file
                    || previous.end_ms > playhead
                    || previous.start_ms.saturating_add(2_000) >= playhead;
            }
            if previous.status != UttStatus::Done
                || self.source_for(previous) != self.source_for(row)
            {
                break;
            }
        }
        false
    }
    pub fn pick(&self, translation: bool, playhead: Ms, s: &Settings) -> Option<&Utterance> {
        let next = self
            .rows
            .values()
            .filter(|u| {
                s.process_whole_file
                    || u.start_ms <= playhead.saturating_add(s.lookahead_seconds as u64 * 1000)
            })
            .filter_map(|u| {
                // A seek can land more than two seconds inside a long utterance.
                if u.start_ms <= playhead && playhead < u.end_ms {
                    Some(((false, 0), u))
                } else {
                    hlp_core::schedule::priority(u.start_ms, playhead, s.process_whole_file)
                        .map(|p| ((true, p), u))
                }
            })
            .filter(|(_, u)| {
                if translation {
                    self.tr_needed(u) && !self.waits_for_background(u, playhead, s)
                } else {
                    u.status == UttStatus::Pending
                        && (self.audio_complete || u.end_ms <= self.extracted)
                }
            })
            .min_by_key(|(p, _)| *p)
            .map(|(_, u)| u);
        // A seek should display its first recognized line immediately. After it
        // does, fill its nearby missing background even in lookahead-only mode.
        if !translation && self.translations.context && self.target != "none" {
            let anchor = self.rows.values().find(|row| row.end_ms > playhead);
            if let Some(anchor) = anchor.filter(|u| {
                u.status == UttStatus::Done
                    && self.applicable(u)
                    && self.source_for(u).is_some()
                    && u.start_ms <= playhead.saturating_add(2_000)
            }) {
                for row in self.predecessors(anchor) {
                    if row.status == UttStatus::Pending
                        && (self.audio_complete || row.end_ms <= self.extracted)
                    {
                        return Some(row);
                    }
                    if row.status != UttStatus::Done
                        || self.source_for(row) != self.source_for(anchor)
                    {
                        break;
                    }
                }
            }
        }
        next
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
        let was_locked = self.language.locked;
        self.vote();
        if self.language.locked != was_locked {
            self.rebuild_keys()
        } else {
            self.refresh_keys_from(id)
        }
        self.dirty = true;
    }
    fn translation(&self, u: &Utterance) -> Option<&store::Translation> {
        if !self.applicable(u) {
            return None;
        }
        let language = self.source_for(u);
        let (source_hash, _) = self.translation_keys.get(&u.id)?;
        [
            self.translations.items.get(&u.id),
            self.translations.fallback.get(&u.id),
        ]
        .into_iter()
        .flatten()
        .find(|v| {
            !v.text.is_empty()
                && &v.source_hash == source_hash
                && v.source_language
                    .as_deref()
                    .is_none_or(|l| Some(l) == language)
        })
    }
    pub fn cues(&mut self) -> Vec<Cue> {
        let mut builder = std::mem::take(&mut self.cue_builder);
        let cues =
            builder.update_aligned(self.rows.values().map(|row| match self.translation(row) {
                Some(t) => (row, t.text.as_str(), t.aligned_phrases.as_slice()),
                None => (row, "", [].as_slice()),
            }));
        self.cue_builder = builder;
        cues
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
        let mut translation_failed = 0;
        let (mut asr, mut tr) = (vec![], vec![]);
        for u in self.rows.values() {
            match u.status {
                UttStatus::Done => done += 1,
                UttStatus::NoSpeech => silent += 1,
                UttStatus::Failed { .. } => failed += 1,
                UttStatus::Pending => append(&mut asr, u),
            }
            if u.status == UttStatus::Done && self.applicable(u) {
                match self.current_entry(u) {
                    Some(item) if !item.failed => translated += 1,
                    None => append(&mut tr, u),
                    Some(_) => translation_failed += 1,
                }
            }
        }
        let rtf = self.asr_rtf();
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
            translation_failed,
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
    fn rejected_translation_does_not_block_other_lines_or_repeat_good_work() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let s = Settings {
            target_language: "zh".into(),
            source_language: "auto-per-line".into(),
            ..Settings::default()
        };
        let mut t = Timeline::load(dir.path().join("track"), &s, "test")?;
        let rows = [utterance(0, "en"), utterance(1, "en"), utterance(2, "en")];
        for u in &rows {
            t.rows.insert(u.id.clone(), u.clone());
            t.refresh_keys_from(&u.id);
        }
        for (u, failed) in [(&rows[0], true), (&rows[1], false)] {
            let (source_hash, context_hash) = t.translation_keys[&u.id].clone();
            t.translations.items.insert(
                u.id.clone(),
                store::Translation {
                    source_hash,
                    context_hash: Some(context_hash),
                    source_language: Some("en".into()),
                    text: if failed {
                        String::new()
                    } else {
                        "已翻译".into()
                    },
                    aligned_phrases: Vec::new(),
                    failed,
                },
            );
        }
        let s1 = t.status("processing");
        assert_eq!(
            (
                s1.utt_done,
                s1.utt_failed,
                s1.translation_failed,
                s1.translated_count
            ),
            (3, 0, 1, 1)
        );
        assert_eq!(t.pick(true, 0, &s).unwrap().id, rows[2].id);
        t.retry_failed_translations();
        assert!(t.tr_needed(&rows[0]));
        assert!(!t.tr_needed(&rows[1]));
        assert!(t.pick(false, 0, &s).is_none());
        assert_eq!(t.status("processing").translation_failed, 0);
        Ok(())
    }

    #[test]
    fn slow_recognition_defers_background_translation_until_the_lookahead_is_ready() {
        let temp = tempfile::tempdir().unwrap();
        let mut t =
            Timeline::load(temp.path().join("track"), &Settings::default(), "test").unwrap();
        let later = utterance(3, "en");
        let mut missing = utterance(4, "en");
        missing.status = UttStatus::Pending;
        t.rows.insert(missing.id.clone(), missing.clone());
        t.audio_complete = true;
        t.asr_ms = 1500;
        assert!(!t.defer_translation(&later, 0, 180));
        t.audio_ms = 9000;
        assert!(t.defer_translation(&later, 0, 180));
        assert!(!t.defer_translation(&later, 8500, 180));
        assert!(!t.defer_translation(&later, 0, 5));
        assert!(!t.defer_translation(&later, 20000, 180));
        t.asr_ms = 1000;
        assert!(!t.defer_translation(&later, 0, 180));
        t.asr_ms = 1500;
        t.audio_complete = false;
        assert!(!t.defer_translation(&later, 0, 180));
        t.extracted = missing.end_ms;
        assert!(t.defer_translation(&later, 0, 180));
        t.rows.clear();
        assert!(!t.defer_translation(&later, 0, 180));
    }

    #[test]
    fn uncertain_first_six_wait_until_twelfth_vote() {
        let temp = tempfile::tempdir().unwrap();
        let settings = Settings {
            source_language: "auto".into(),
            ..Settings::default()
        };
        let mut t = Timeline::load(temp.path().join("track"), &settings, "test").unwrap();
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
        let settings = Settings {
            source_language: "ja".into(),
            ..Settings::default()
        };
        let mut t = Timeline::load(temp.path().join("track"), &settings, "test").unwrap();
        let mut u = utterance(0, "ja");
        u.forced_language = u.language.clone();
        t.rows.insert(u.id.clone(), u);
        t.set_source("auto");
        assert_eq!(t.rows.values().next().unwrap().status, UttStatus::Pending);
        assert!(t.language.detected.is_none());
        assert!(t.cues()[0].stale);
    }

    #[test]
    fn interrupted_deletion_clears_all_generated_text_from_only_the_selected_track() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let track = dir.path().join("track-0");
        let other = dir.path().join("track-1");
        let settings = Settings {
            target_language: "en".into(),
            ..Settings::default()
        };
        let mut t = Timeline::load(track.clone(), &settings, "test")?;
        let u = utterance(0, "ja");
        t.rows.insert(u.id.clone(), u);
        t.persist()?;
        for name in [
            "generate.requested",
            "transcript.qwen3-asr-0.6b.json",
            "translations/zh.context.json",
            "translations/fr.json",
            "audio.pcm",
            "playback.json",
            "translations/export.srt",
        ] {
            store::atomic_write(&track.join(name), b"saved")?;
        }
        store::atomic_write(&other.join("transcript.json"), b"other track")?;

        // The process died after recording the intent and removing one file.
        store::atomic_write(&track.join("subtitles.deleting"), b"1")?;
        std::fs::remove_file(track.join("transcript.json"))?;
        let mut reopened = Timeline::load(track.clone(), &settings, "reopen")?;
        assert!(reopened.cues().is_empty());
        assert!(reopened.rows.is_empty() && !reopened.vad_complete);
        for name in [
            "generate.requested",
            "subtitles.deleting",
            "segments.json",
            "transcript.qwen3-asr-0.6b.json",
            "translations/zh.context.json",
            "translations/fr.json",
        ] {
            assert!(!track.join(name).exists(), "{name}");
        }
        for name in ["audio.pcm", "playback.json", "translations/export.srt"] {
            assert_eq!(std::fs::read(track.join(name))?, b"saved");
        }
        assert_eq!(
            std::fs::read(other.join("transcript.json"))?,
            b"other track"
        );
        Ok(())
    }

    #[test]
    fn regeneration_checkpoint_requeues_segments_and_invalidates_all_translation_targets(
    ) -> Result<()> {
        let temp = tempfile::tempdir()?;
        let track = temp.path().join("track-0");
        let settings = Settings {
            target_language: "en".into(),
            ..Settings::default()
        };
        store::atomic_write(&track.join("audio.pcm"), &[0; 32])?;
        store::write_json(
            &track.join("audio.json"),
            &Audio {
                sample_rate: 16000,
                total_samples: 16,
                complete: true,
                dialogue_focus: settings.dialogue_channel_focus,
            },
        )?;
        let mut t = Timeline::load(track.clone(), &settings, "test")?;
        t.vad_complete = true;
        let u = utterance(0, "ja");
        t.rows.insert(u.id.clone(), u.clone());
        for target in ["fr", "en"] {
            t.set_target(target, settings.translation_context)?;
            t.translations.items.insert(
                u.id.clone(),
                store::Translation {
                    source_hash: store::hash(u.text.as_ref().unwrap()),
                    context_hash: Some(t.context_hash(&u, None)),
                    source_language: Some("ja".into()),
                    text: target.into(),
                    aligned_phrases: Vec::new(),
                    failed: false,
                },
            );
        }
        t.persist()?;
        assert_eq!(
            Timeline::load(track.clone(), &settings, "test")?
                .translations
                .items
                .len(),
            1
        );

        t.regenerate(&settings.source_language)?;
        assert!(t.cues().is_empty());
        // Simulate a quit immediately after the request, before a periodic save.
        let mut reopened = Timeline::load(track, &settings, "test")?;
        assert!(reopened.vad_complete && reopened.audio_complete);
        assert_eq!(reopened.rows[&u.id], speech_segment(&u));
        assert!(reopened.translations.items.is_empty());
        reopened.set_target("fr", settings.translation_context)?;
        assert!(reopened.translations.items.is_empty());
        Ok(())
    }
    #[test]
    fn ordered_context_stops_at_language_gap_and_byte_budget() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut t = Timeline::load(dir.path().join("track-0"), &Settings::default(), "test")?;
        for index in [3, 1, 4, 0, 2] {
            let row = utterance(index, "en");
            t.rows.insert(row.id.clone(), row);
        }
        assert_eq!(
            t.background(&t.rows["u000004"]).as_deref(),
            Some("phrase 1\nphrase 2\nphrase 3")
        );
        t.rows.get_mut("u000002").unwrap().language = Some(LangCode("fr".into()));
        assert_eq!(
            t.background(&t.rows["u000004"]).as_deref(),
            Some("phrase 3")
        );
        t.rows.get_mut("u000003").unwrap().text = Some("音".repeat(900));
        let bg = t.background(&t.rows["u000004"]).unwrap();
        assert_eq!(bg.len(), 1023);
        assert!(bg.chars().all(|c| c == '音'));
        t.rows.get_mut("u000003").unwrap().status = UttStatus::Pending;
        assert!(t.background(&t.rows["u000004"]).is_none());
        t.rows.get_mut("u000003").unwrap().status = UttStatus::Done;
        t.rows.get_mut("u000004").unwrap().start_ms = 40_000;
        assert!(t.background(&t.rows["u000004"]).is_none());
        Ok(())
    }
    #[test]
    fn seeked_line_is_immediate_and_refreshes_when_missing_background_arrives() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let s = Settings {
            process_whole_file: false,
            target_language: "zh".into(),
            source_language: "auto-per-line".into(),
            ..Settings::default()
        };
        let mut t = Timeline::load(dir.path().join("track-0"), &s, "test")?;
        t.audio_complete = true;
        let first = utterance(0, "en");
        let mut anchor = utterance(1, "en");
        anchor.end_ms = 8_000;
        anchor.regions[0].end_ms = 8_000;
        for mut row in [first.clone(), anchor.clone()] {
            row.text = None;
            row.status = UttStatus::Pending;
            t.rows.insert(row.id.clone(), row);
        }
        let playhead = 6500;
        assert_eq!(t.pick(false, playhead, &s).unwrap().id, anchor.id);
        let observed = |line: &str| ParsedAsr::Text {
            text: line.into(),
            language: Some(LangCode("en".into())),
        };
        t.apply(&anchor.id, observed("There it is."), None);
        let current = t.pick(true, playhead, &s).unwrap();
        assert_eq!(current.id, anchor.id);
        assert!(t.background(current).is_none());
        let (source_hash, context_hash) = t.translation_keys[&anchor.id].clone();
        t.translations.items.insert(
            anchor.id.clone(),
            store::Translation {
                source_hash,
                context_hash: Some(context_hash),
                source_language: Some("en".into()),
                text: "在那里。".into(),
                aligned_phrases: Vec::new(),
                failed: false,
            },
        );
        assert!(!t.tr_needed(&t.rows[&anchor.id]));
        assert_eq!(t.pick(false, playhead, &s).unwrap().id, first.id);
        t.apply(&first.id, observed("Watch for the boat."), None);
        assert_eq!(
            t.background(&t.rows[&anchor.id]).as_deref(),
            Some("Watch for the boat.")
        );
        assert_eq!(t.pick(true, playhead, &s).unwrap().id, anchor.id);
        assert_eq!(
            t.translation(&t.rows[&anchor.id]).map(|v| v.text.as_str()),
            Some("在那里。")
        );
        t.set_target("fr", true)?;
        assert!(t.translation(&t.rows[&anchor.id]).is_none());
        let mut other = Timeline::load(dir.path().join("track-1"), &s, "other")?;
        assert!(
            other.cues().is_empty()
                && other.translations.items.is_empty()
                && other.translations.fallback.is_empty()
        );
        Ok(())
    }
    #[test]
    fn background_translation_waits_for_recognition_without_delaying_an_urgent_line() -> Result<()>
    {
        let dir = tempfile::tempdir()?;
        let s = Settings {
            target_language: "zh".into(),
            source_language: "auto-per-line".into(),
            ..Settings::default()
        };
        let mut t = Timeline::load(dir.path().join("track-0"), &s, "test")?;
        let mut prev = utterance(0, "en");
        let row = utterance(1, "en");
        prev.status = UttStatus::Pending;
        prev.text = None;
        t.rows.insert(prev.id.clone(), prev.clone());
        t.rows.insert(row.id.clone(), row.clone());
        assert!(t.pick(true, 20_000, &s).is_none());
        assert_eq!(t.pick(true, row.start_ms, &s).unwrap().id, row.id);
        t.apply(&prev.id, ParsedAsr::NoSpeech, None);
        assert_eq!(t.pick(true, 20_000, &s).unwrap().id, row.id);
        Ok(())
    }

    #[test]
    fn legacy_subtitles_survive_an_upgrade_and_reopen_without_any_model() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let track = dir.path().join("track-0");
        let s = Settings {
            target_language: "zh".into(),
            source_language: "auto-per-line".into(),
            ..Settings::default()
        };
        let row = utterance(1, "en");
        store::atomic_write(&track.join("audio.pcm"), &[0; 32])?;
        store::write_json(
            &track.join("audio.json"),
            &Audio {
                sample_rate: 16000,
                total_samples: 16,
                complete: true,
                dialogue_focus: true,
            },
        )?;
        let mut t = Timeline::load(track.clone(), &s, "backup")?;
        t.rows.insert(row.id.clone(), row.clone());
        t.persist()?;
        let legacy = Translations::path(&track, "zh", false);
        let old = serde_json::json!({"translatorId":"hy-mt2-1.8b-q4km","promptVersion":1,"context":false,"items":{row.id.clone():{"sourceHash":store::hash("phrase 1"),"text":"旧字幕"}}});
        store::write_json(&legacy, &old)?;
        let original = std::fs::read(&legacy)?;
        let mut ctx = Timeline::load(track.clone(), &s, "reopen")?;
        assert_eq!(ctx.cues()[0].translation.as_deref(), Some("旧字幕"));
        assert!(ctx.tr_needed(&row));
        let (source_hash, context_hash) = ctx.translation_keys[&row.id].clone();
        ctx.translations.items.insert(
            row.id.clone(),
            store::Translation {
                source_hash,
                context_hash: Some(context_hash),
                source_language: Some("en".into()),
                text: "新字幕".into(),
                aligned_phrases: Vec::new(),
                failed: false,
            },
        );
        ctx.persist()?;
        let mut reopened = Timeline::load(track.clone(), &s, "again")?;
        assert!(!reopened.tr_needed(&row));
        assert_eq!(reopened.cues()[0].translation.as_deref(), Some("新字幕"));
        assert_eq!(std::fs::read(&legacy)?, original);
        reopened.set_target("zh", false)?;
        assert_eq!(reopened.cues()[0].translation.as_deref(), Some("旧字幕"));
        assert!(!reopened.tr_needed(&row));
        Ok(())
    }

    #[test]
    fn saved_phrase_alignment_reaches_the_picture_after_reopen_and_mode_switch() -> Result<()> {
        use hlp_core::cues::AlignedPhrase;
        let dir = tempfile::tempdir()?;
        let track = dir.path().join("track-0");
        let s = Settings {
            source_language: "auto-per-line".into(),
            target_language: "zh".into(),
            translation_context: true,
            ..Settings::default()
        };
        let sources = [
            "I found the missing keys under the old kitchen table ",
            "before everyone else had even opened their eyes",
        ];
        let targets = ["我在旧餐桌下面找到了丢失的钥匙", "那时候其他人都还没有睁眼"];
        let mut row = utterance(0, "en");
        row.text = Some(sources.concat());
        store::write_json(
            &track.join("audio.json"),
            &Audio {
                sample_rate: 16_000,
                total_samples: 0,
                complete: false,
                dialogue_focus: s.dialogue_channel_focus,
            },
        )?;
        let mut t = Timeline::load(track.clone(), &s, "saved")?;
        t.rows.insert(row.id.clone(), row.clone());
        t.refresh_keys_from(&row.id);
        let (source_hash, context_hash) = t.translation_keys[&row.id].clone();
        t.translations.items.insert(
            row.id.clone(),
            store::Translation {
                source_hash,
                source_language: Some("en".into()),
                context_hash: Some(context_hash),
                text: targets.concat(),
                aligned_phrases: vec![
                    AlignedPhrase {
                        source_end: sources[0].len(),
                        target_end: targets[0].len(),
                    },
                    AlignedPhrase {
                        source_end: sources.concat().len(),
                        target_end: targets.concat().len(),
                    },
                ],
                failed: false,
            },
        );
        t.persist()?;
        let mut reopened = Timeline::load(track, &s, "reopened")?;
        assert!(!reopened.tr_needed(&row));
        for mode in [true, false] {
            reopened.set_target("zh", mode)?;
            let cues = reopened.cues();
            assert_eq!(cues.len(), 2);
            for ((cue, source), target) in cues.iter().zip(sources).zip(targets) {
                assert_eq!(cue.original, source.trim());
                assert_eq!(cue.translation.as_deref(), Some(target));
            }
        }
        Ok(())
    }
}
