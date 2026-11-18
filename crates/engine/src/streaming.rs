use std::collections::VecDeque;

use crate::vad::{AlwaysSpeech, SpeechDetector, WINDOW_SAMPLES};
use crate::{Engine, EngineError, Recognizer, UtteranceOptions};

pub const PARTIAL_INTERVAL_SAMPLES: usize = 8_000;
pub const MIN_SILENCE_SAMPLES: usize = 8_000;
pub const SPEECH_PAD_SAMPLES: usize = 4_800;
pub const MAX_SEGMENT_SAMPLES: usize = 240_000;
const CUT_SEARCH_SAMPLES: usize = 24_000;
const CUT_SEARCH_WINDOWS: usize = CUT_SEARCH_SAMPLES / WINDOW_SAMPLES + 1;
const SPEECH_THRESHOLD: f32 = 0.5;
const SILENCE_THRESHOLD: f32 = 0.35;

struct LockedSegment {
    text: String,
    forced: bool,
}

struct OpenUtterance {
    options: UtteranceOptions,
    audio: Vec<f32>,
    scanned: usize,
    in_speech: bool,
    silent: usize,
    since_partial: usize,
    locked: Vec<LockedSegment>,
    recent: VecDeque<(usize, f32)>,
}

impl OpenUtterance {
    fn new(options: UtteranceOptions) -> Self {
        Self {
            options,
            audio: Vec::new(),
            scanned: 0,
            in_speech: false,
            silent: 0,
            since_partial: 0,
            locked: Vec::new(),
            recent: VecDeque::with_capacity(CUT_SEARCH_WINDOWS + 1),
        }
    }

    fn advance<R: Recognizer>(
        &mut self,
        recognizer: &mut R,
        probability: f32,
        window_end: usize,
    ) -> Result<bool, EngineError> {
        let window_start = self.scanned;
        self.scanned = window_end;
        if probability >= SPEECH_THRESHOLD {
            if !self.in_speech {
                self.in_speech = true;
                let start = window_start.saturating_sub(SPEECH_PAD_SAMPLES);
                self.audio.drain(..start);
                self.scanned -= start;
            }
            self.silent = 0;
        } else if self.in_speech {
            if probability < SILENCE_THRESHOLD {
                self.silent += window_end - window_start;
            } else {
                self.silent = 0;
            }
            if self.silent >= MIN_SILENCE_SAMPLES {
                self.close_segment(recognizer)?;
                return Ok(true);
            }
        } else if self.scanned > SPEECH_PAD_SAMPLES {
            let excess = self.scanned - SPEECH_PAD_SAMPLES;
            self.audio.drain(..excess);
            self.scanned -= excess;
        }
        if self.in_speech {
            self.recent.push_back((self.scanned, probability));
            if self.recent.len() > CUT_SEARCH_WINDOWS {
                self.recent.pop_front();
            }
            if self.scanned >= MAX_SEGMENT_SAMPLES {
                self.cut_segment(recognizer)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn segment_end(&self) -> usize {
        let speech_end = self.scanned.saturating_sub(self.silent);
        (speech_end + SPEECH_PAD_SAMPLES).min(self.audio.len())
    }

    fn close_segment<R: Recognizer>(&mut self, recognizer: &mut R) -> Result<(), EngineError> {
        let text =
            recognizer.recognize(&self.audio[..self.segment_end()], self.options.language)?;
        self.lock(text, false);
        self.audio.drain(..self.scanned);
        self.scanned = 0;
        self.in_speech = false;
        self.silent = 0;
        self.recent.clear();
        Ok(())
    }

    fn cut_segment<R: Recognizer>(&mut self, recognizer: &mut R) -> Result<(), EngineError> {
        let cut = self
            .recent
            .iter()
            .rev()
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(self.scanned, |&(end, _)| {
                end.saturating_sub(WINDOW_SAMPLES / 2).max(1)
            })
            .min(self.scanned);
        let text = recognizer.recognize(&self.audio[..cut], self.options.language)?;
        self.lock(text, true);
        self.audio.drain(..cut);
        self.scanned -= cut;
        self.recent.retain(|&(end, _)| end > cut);
        for (end, _) in &mut self.recent {
            *end -= cut;
        }
        Ok(())
    }

    fn lock(&mut self, text: String, forced: bool) {
        if !text.is_empty() {
            self.locked.push(LockedSegment { text, forced });
        }
    }

    fn text_with(&self, open: &str) -> String {
        let mut text = String::new();
        let mut separator = "";
        for segment in &self.locked {
            text.push_str(separator);
            text.push_str(&segment.text);
            separator = if segment.forced { "" } else { " " };
        }
        if !open.is_empty() {
            text.push_str(separator);
            text.push_str(open);
        }
        text
    }
}

pub struct StreamingEngine<R: Recognizer, D: SpeechDetector = AlwaysSpeech> {
    recognizer: R,
    detector: D,
    utterance: Option<OpenUtterance>,
}

impl<R: Recognizer> StreamingEngine<R> {
    pub fn with_recognizer(recognizer: R) -> Self {
        Self::with_detector(recognizer, AlwaysSpeech)
    }
}

impl<R: Recognizer, D: SpeechDetector> StreamingEngine<R, D> {
    pub fn with_detector(recognizer: R, detector: D) -> Self {
        Self {
            recognizer,
            detector,
            utterance: None,
        }
    }
}

impl<R: Recognizer, D: SpeechDetector> Engine for StreamingEngine<R, D> {
    fn load(&mut self) -> Result<(), EngineError> {
        if !self.recognizer.is_loaded() {
            self.recognizer.load()?;
        }
        if !self.detector.is_loaded() {
            self.detector.load()?;
        }
        Ok(())
    }

    fn unload(&mut self) {
        self.utterance = None;
        self.recognizer.unload();
        self.detector.unload();
    }

    fn is_loaded(&self) -> bool {
        self.recognizer.is_loaded() && self.detector.is_loaded()
    }

    fn start_utterance(&mut self, options: UtteranceOptions) -> Result<(), EngineError> {
        if !self.is_loaded() {
            return Err(EngineError::NotLoaded);
        }
        self.detector.reset();
        self.utterance = Some(OpenUtterance::new(options));
        Ok(())
    }

    fn push_audio(&mut self, samples: &[f32]) -> Result<Option<String>, EngineError> {
        self.push(samples, true)
    }

    fn push_tail(&mut self, samples: &[f32]) -> Result<(), EngineError> {
        self.push(samples, false).map(drop)
    }

    fn finish_utterance(&mut self) -> Result<String, EngineError> {
        self.finish()
    }

    fn cancel_utterance(&mut self) {
        self.utterance = None;
    }
}

impl<R: Recognizer, D: SpeechDetector> StreamingEngine<R, D> {
    fn push(&mut self, samples: &[f32], partials: bool) -> Result<Option<String>, EngineError> {
        let utterance = self.utterance.as_mut().ok_or(EngineError::NoUtterance)?;
        utterance.audio.extend_from_slice(samples);
        utterance.since_partial += samples.len();
        let mut closed = false;
        while utterance.audio.len() - utterance.scanned >= WINDOW_SAMPLES {
            let start = utterance.scanned;
            let window_end = start + WINDOW_SAMPLES;
            let probability = self
                .detector
                .speech_probability(&utterance.audio[start..window_end])?;
            closed |= utterance.advance(&mut self.recognizer, probability, window_end)?;
        }
        if partials
            && !closed
            && utterance.in_speech
            && utterance.since_partial >= PARTIAL_INTERVAL_SAMPLES
        {
            utterance.since_partial = 0;
            let open = self
                .recognizer
                .recognize(&utterance.audio, utterance.options.language)?;
            return Ok(Some(utterance.text_with(&open)));
        }
        Ok(closed.then(|| utterance.text_with("")))
    }

    fn finish(&mut self) -> Result<String, EngineError> {
        let mut utterance = self.utterance.take().ok_or(EngineError::NoUtterance)?;
        while utterance.audio.len() - utterance.scanned >= WINDOW_SAMPLES {
            let start = utterance.scanned;
            let window_end = start + WINDOW_SAMPLES;
            let probability = self
                .detector
                .speech_probability(&utterance.audio[start..window_end])?;
            utterance.advance(&mut self.recognizer, probability, window_end)?;
        }
        let remaining = utterance.audio.len() - utterance.scanned;
        if remaining > 0 {
            let mut window = [0.0f32; WINDOW_SAMPLES];
            window[..remaining].copy_from_slice(&utterance.audio[utterance.scanned..]);
            let probability = self.detector.speech_probability(&window)?;
            let window_end = utterance.audio.len();
            utterance.advance(&mut self.recognizer, probability, window_end)?;
        }
        if utterance.in_speech {
            utterance.close_segment(&mut self.recognizer)?;
        }
        Ok(utterance.text_with(""))
    }
}
