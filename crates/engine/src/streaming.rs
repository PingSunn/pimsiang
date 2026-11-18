use crate::vad::{AlwaysSpeech, SpeechDetector, WINDOW_SAMPLES};
use crate::{Engine, EngineError, Recognizer, UtteranceOptions};

pub const PARTIAL_INTERVAL_SAMPLES: usize = 8_000;
pub const MIN_SILENCE_SAMPLES: usize = 8_000;
pub const SPEECH_PAD_SAMPLES: usize = 4_800;
const SPEECH_THRESHOLD: f32 = 0.5;
const SILENCE_THRESHOLD: f32 = 0.35;

struct OpenUtterance {
    options: UtteranceOptions,
    audio: Vec<f32>,
    scanned: usize,
    in_speech: bool,
    silent: usize,
    since_partial: usize,
    locked: Vec<String>,
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
        Ok(false)
    }

    fn segment_end(&self) -> usize {
        let speech_end = self.scanned.saturating_sub(self.silent);
        (speech_end + SPEECH_PAD_SAMPLES).min(self.audio.len())
    }

    fn close_segment<R: Recognizer>(&mut self, recognizer: &mut R) -> Result<(), EngineError> {
        let text =
            recognizer.recognize(&self.audio[..self.segment_end()], self.options.language)?;
        if !text.is_empty() {
            self.locked.push(text);
        }
        self.audio.drain(..self.scanned);
        self.scanned = 0;
        self.in_speech = false;
        self.silent = 0;
        Ok(())
    }

    fn text_with(&self, open: &str) -> String {
        self.locked
            .iter()
            .map(String::as_str)
            .chain((!open.is_empty()).then_some(open))
            .collect::<Vec<_>>()
            .join(" ")
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
        if utterance.in_speech && utterance.since_partial >= PARTIAL_INTERVAL_SAMPLES {
            utterance.since_partial = 0;
            let open = self
                .recognizer
                .recognize(&utterance.audio, utterance.options.language)?;
            return Ok(Some(utterance.text_with(&open)));
        }
        Ok(closed.then(|| utterance.text_with("")))
    }

    fn finish_utterance(&mut self) -> Result<String, EngineError> {
        let mut utterance = self.utterance.take().ok_or(EngineError::NoUtterance)?;
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

    fn cancel_utterance(&mut self) {
        self.utterance = None;
    }
}
