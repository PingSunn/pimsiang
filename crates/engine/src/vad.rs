use std::path::{Path, PathBuf};

use ort::environment::Environment;
use ort::session::Session;
use ort::value::Tensor;

use crate::{EngineError, ModelFiles};

pub const WINDOW_SAMPLES: usize = 512;
const CONTEXT_SAMPLES: usize = 64;
const STATE_SIZE: usize = 2 * 128;
const SAMPLE_RATE: i64 = 16_000;

pub trait SpeechDetector {
    fn load(&mut self) -> Result<(), EngineError>;
    fn unload(&mut self);
    fn is_loaded(&self) -> bool;
    fn use_files(&mut self, _files: &ModelFiles) {}
    fn reset(&mut self);
    fn speech_probability(&mut self, window: &[f32]) -> Result<f32, EngineError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AlwaysSpeech;

impl SpeechDetector for AlwaysSpeech {
    fn load(&mut self) -> Result<(), EngineError> {
        Ok(())
    }

    fn unload(&mut self) {}

    fn is_loaded(&self) -> bool {
        true
    }

    fn reset(&mut self) {}

    fn speech_probability(&mut self, _window: &[f32]) -> Result<f32, EngineError> {
        Ok(1.0)
    }
}

struct LoadedVad {
    _environment: Environment,
    session: Session,
}

pub struct SileroVad {
    path: PathBuf,
    model: Option<LoadedVad>,
    state: Vec<f32>,
    context: [f32; CONTEXT_SAMPLES],
}

impl SileroVad {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            model: None,
            state: vec![0.0; STATE_SIZE],
            context: [0.0; CONTEXT_SAMPLES],
        }
    }
}

impl SpeechDetector for SileroVad {
    fn load(&mut self) -> Result<(), EngineError> {
        if !self.path.is_file() {
            return Err(EngineError::MissingModelFile(self.path.clone()));
        }
        let environment = crate::runtime::environment()?;
        let session = open_session(&environment, &self.path)?;
        self.model = Some(LoadedVad {
            _environment: environment,
            session,
        });
        self.reset();
        Ok(())
    }

    fn unload(&mut self) {
        self.model = None;
    }

    fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    fn use_files(&mut self, files: &ModelFiles) {
        self.path = files.vad();
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
        self.context.fill(0.0);
    }

    fn speech_probability(&mut self, window: &[f32]) -> Result<f32, EngineError> {
        let model = self.model.as_mut().ok_or(EngineError::NotLoaded)?;
        if window.len() != WINDOW_SAMPLES {
            return Err(EngineError::Model(format!(
                "VAD window must be {WINDOW_SAMPLES} samples, got {}",
                window.len()
            )));
        }
        let mut input = Vec::with_capacity(CONTEXT_SAMPLES + WINDOW_SAMPLES);
        input.extend_from_slice(&self.context);
        input.extend_from_slice(window);
        let inputs = ort::inputs! {
            "input" => Tensor::from_array(([1, input.len()], input)).map_err(model_error)?,
            "state" => Tensor::from_array(([2, 1, 128], self.state.clone())).map_err(model_error)?,
            "sr" => Tensor::from_array(([0usize; 0], vec![SAMPLE_RATE])).map_err(model_error)?,
        };
        let outputs = model.session.run(inputs).map_err(model_error)?;
        let (_, probability) = outputs["output"]
            .try_extract_tensor::<f32>()
            .map_err(model_error)?;
        let probability = probability.first().copied().unwrap_or(0.0);
        let (_, state) = outputs["stateN"]
            .try_extract_tensor::<f32>()
            .map_err(model_error)?;
        self.state.clear();
        self.state.extend_from_slice(state);
        self.context
            .copy_from_slice(&window[WINDOW_SAMPLES - CONTEXT_SAMPLES..]);
        Ok(probability)
    }
}

fn open_session(environment: &Environment, path: &Path) -> Result<Session, EngineError> {
    let fail = |e: String| EngineError::Model(format!("{}: {e}", path.display()));
    let mut builder = Session::builder(environment).map_err(|e| fail(e.to_string()))?;
    builder
        .commit_from_file(path)
        .map_err(|e| fail(e.to_string()))
}

fn model_error(e: ort::Error) -> EngineError {
    EngineError::Model(e.to_string())
}
