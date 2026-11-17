use crate::{Engine, EngineError, Recognizer, UtteranceOptions};

pub const PARTIAL_INTERVAL_SAMPLES: usize = 8_000;

struct OpenUtterance {
    options: UtteranceOptions,
    audio: Vec<f32>,
    decoded_up_to: usize,
}

pub struct StreamingEngine<R: Recognizer> {
    recognizer: R,
    utterance: Option<OpenUtterance>,
}

impl<R: Recognizer> StreamingEngine<R> {
    pub fn with_recognizer(recognizer: R) -> Self {
        Self {
            recognizer,
            utterance: None,
        }
    }
}

impl<R: Recognizer> Engine for StreamingEngine<R> {
    fn load(&mut self) -> Result<(), EngineError> {
        if self.recognizer.is_loaded() {
            return Ok(());
        }
        self.recognizer.load()
    }

    fn unload(&mut self) {
        self.utterance = None;
        self.recognizer.unload();
    }

    fn is_loaded(&self) -> bool {
        self.recognizer.is_loaded()
    }

    fn start_utterance(&mut self, options: UtteranceOptions) -> Result<(), EngineError> {
        if !self.recognizer.is_loaded() {
            return Err(EngineError::NotLoaded);
        }
        self.utterance = Some(OpenUtterance {
            options,
            audio: Vec::new(),
            decoded_up_to: 0,
        });
        Ok(())
    }

    fn push_audio(&mut self, samples: &[f32]) -> Result<Option<String>, EngineError> {
        let utterance = self.utterance.as_mut().ok_or(EngineError::NoUtterance)?;
        utterance.audio.extend_from_slice(samples);
        if utterance.audio.len() - utterance.decoded_up_to < PARTIAL_INTERVAL_SAMPLES {
            return Ok(None);
        }
        utterance.decoded_up_to = utterance.audio.len();
        let language = utterance.options.language;
        self.recognizer
            .recognize(&utterance.audio, language)
            .map(Some)
    }

    fn finish_utterance(&mut self) -> Result<String, EngineError> {
        let utterance = self.utterance.take().ok_or(EngineError::NoUtterance)?;
        if utterance.audio.is_empty() {
            return Ok(String::new());
        }
        self.recognizer
            .recognize(&utterance.audio, utterance.options.language)
    }

    fn cancel_utterance(&mut self) {
        self.utterance = None;
    }
}
