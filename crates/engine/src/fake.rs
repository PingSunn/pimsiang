use std::collections::VecDeque;

use crate::{Engine, EngineError, UtteranceOptions};

pub struct FakeEngine {
    partials: Vec<String>,
    final_text: String,
    loaded: bool,
    load_count: usize,
    pending: Option<VecDeque<String>>,
    last_options: Option<UtteranceOptions>,
    pushed_samples: usize,
}

impl FakeEngine {
    pub fn new<P: Into<String>>(
        partials: impl IntoIterator<Item = P>,
        final_text: impl Into<String>,
    ) -> Self {
        Self {
            partials: partials.into_iter().map(Into::into).collect(),
            final_text: final_text.into(),
            loaded: false,
            load_count: 0,
            pending: None,
            last_options: None,
            pushed_samples: 0,
        }
    }

    pub fn last_options(&self) -> Option<UtteranceOptions> {
        self.last_options
    }

    pub fn pushed_samples(&self) -> usize {
        self.pushed_samples
    }

    pub fn load_count(&self) -> usize {
        self.load_count
    }
}

impl Engine for FakeEngine {
    fn load(&mut self) -> Result<(), EngineError> {
        if !self.loaded {
            self.loaded = true;
            self.load_count += 1;
        }
        Ok(())
    }

    fn unload(&mut self) {
        self.loaded = false;
        self.pending = None;
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn start_utterance(&mut self, options: UtteranceOptions) -> Result<(), EngineError> {
        if !self.loaded {
            return Err(EngineError::NotLoaded);
        }
        self.last_options = Some(options);
        self.pending = Some(self.partials.iter().cloned().collect());
        Ok(())
    }

    fn push_audio(&mut self, samples: &[f32]) -> Result<Option<String>, EngineError> {
        let pending = self.pending.as_mut().ok_or(EngineError::NoUtterance)?;
        self.pushed_samples += samples.len();
        Ok(pending.pop_front())
    }

    fn finish_utterance(&mut self) -> Result<String, EngineError> {
        self.pending.take().ok_or(EngineError::NoUtterance)?;
        Ok(self.final_text.clone())
    }

    fn cancel_utterance(&mut self) {
        self.pending = None;
    }
}
