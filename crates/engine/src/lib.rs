mod cer;
pub mod fake;
mod features;
mod onnx;
mod runtime;
mod streaming;
mod vad;

pub use cer::character_error_rate;
pub use features::{LogMel, log_mel};
pub use onnx::{ModelFiles, OnnxEngine, OnnxRecognizer, Precision};
pub use streaming::{
    MAX_SEGMENT_SAMPLES, MIN_SILENCE_SAMPLES, PARTIAL_INTERVAL_SAMPLES, SPEECH_PAD_SAMPLES,
    StreamingEngine,
};
pub use vad::{AlwaysSpeech, SileroVad, SpeechDetector, WINDOW_SAMPLES};

use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LanguageMode {
    #[default]
    ThMix,
    ThTh,
}

impl LanguageMode {
    pub fn prompt_index(self) -> i64 {
        match self {
            Self::ThMix => 63,
            Self::ThTh => 32,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::ThMix => "th-mix",
            Self::ThTh => "th-TH",
        }
    }
}

impl fmt::Display for LanguageMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for LanguageMode {
    type Err = EngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        [Self::ThMix, Self::ThTh]
            .into_iter()
            .find(|mode| mode.name() == s)
            .ok_or_else(|| EngineError::UnknownLanguageMode(s.to_owned()))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UtteranceOptions {
    pub language: LanguageMode,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("the Model is not loaded")]
    NotLoaded,
    #[error("no Utterance is in progress")]
    NoUtterance,
    #[error("unknown language mode `{0}` (expected th-mix or th-TH)")]
    UnknownLanguageMode(String),
    #[error("unknown Model precision `{0}` (expected int8 or fp32)")]
    UnknownPrecision(String),
    #[error("Model file not found: {}", .0.display())]
    MissingModelFile(std::path::PathBuf),
    #[error("Model error: {0}")]
    Model(String),
}

pub trait Engine {
    fn load(&mut self) -> Result<(), EngineError>;
    fn unload(&mut self);
    fn is_loaded(&self) -> bool;
    fn start_utterance(&mut self, options: UtteranceOptions) -> Result<(), EngineError>;
    fn push_audio(&mut self, samples: &[f32]) -> Result<Option<String>, EngineError>;
    fn settled_len(&self) -> usize {
        0
    }
    fn push_tail(&mut self, samples: &[f32]) -> Result<(), EngineError> {
        self.push_audio(samples).map(drop)
    }
    fn finish_utterance(&mut self) -> Result<String, EngineError>;
    fn cancel_utterance(&mut self);
}

pub trait Recognizer {
    fn load(&mut self) -> Result<(), EngineError>;
    fn unload(&mut self);
    fn is_loaded(&self) -> bool;
    fn recognize(&mut self, samples: &[f32], language: LanguageMode)
    -> Result<String, EngineError>;
}
