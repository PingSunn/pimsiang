use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ort::environment::Environment;
use ort::session::Session;
use ort::value::Tensor;

use crate::features::MelFrontEnd;
use crate::{EngineError, LanguageMode, Recognizer, StreamingEngine};

const ENCODER_WIDTH: usize = 1024;
const STATE_SIZE: usize = 2 * 640;
const MAX_SYMBOLS_PER_FRAME: usize = 10;
const SHORT_CLIP_SAMPLES: usize = 2 * 16_000;
const BLANK_BIAS: f32 = -1.0;
const SHORT_CLIP_BLANK_BIAS: f32 = -2.5;

pub type OnnxEngine = StreamingEngine<OnnxRecognizer>;

impl OnnxEngine {
    pub fn new(files: ModelFiles) -> Self {
        StreamingEngine::with_recognizer(OnnxRecognizer::new(files))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Precision {
    #[default]
    Int8,
    Fp32,
}

impl Precision {
    pub fn name(self) -> &'static str {
        match self {
            Self::Int8 => "int8",
            Self::Fp32 => "fp32",
        }
    }
}

impl fmt::Display for Precision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for Precision {
    type Err = EngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        [Self::Int8, Self::Fp32]
            .into_iter()
            .find(|p| p.name() == s)
            .ok_or_else(|| EngineError::UnknownPrecision(s.to_owned()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelFiles {
    pub dir: PathBuf,
    pub precision: Precision,
}

impl ModelFiles {
    pub fn new(dir: impl Into<PathBuf>, precision: Precision) -> Self {
        Self {
            dir: dir.into(),
            precision,
        }
    }

    pub fn encoder(&self) -> PathBuf {
        self.dir.join(match self.precision {
            Precision::Int8 => "encoder.int8.onnx",
            Precision::Fp32 => "encoder.onnx",
        })
    }

    pub fn decoder_joint(&self) -> PathBuf {
        self.dir.join("decoder_joint.onnx")
    }

    pub fn tokens(&self) -> PathBuf {
        self.dir.join("tokens.txt")
    }
}

struct LoadedModel {
    _environment: Environment,
    encoder: Session,
    decoder_joint: Session,
    tokens: Vec<String>,
    blank_id: usize,
}

pub struct OnnxRecognizer {
    files: ModelFiles,
    front_end: MelFrontEnd,
    model: Option<LoadedModel>,
}

impl OnnxRecognizer {
    pub fn new(files: ModelFiles) -> Self {
        Self {
            files,
            front_end: MelFrontEnd::new(),
            model: None,
        }
    }
}

impl Recognizer for OnnxRecognizer {
    fn load(&mut self) -> Result<(), EngineError> {
        let encoder_path = self.files.encoder();
        let decoder_path = self.files.decoder_joint();
        let tokens_path = self.files.tokens();
        for path in [&encoder_path, &decoder_path, &tokens_path] {
            if !path.is_file() {
                return Err(EngineError::MissingModelFile(path.clone()));
            }
        }
        let tokens: Vec<String> = std::fs::read_to_string(&tokens_path)
            .map_err(|e| EngineError::Model(format!("{}: {e}", tokens_path.display())))?
            .lines()
            .map(str::to_owned)
            .collect();
        let environment = ort::init().build().map_err(model_error)?;
        self.model = Some(LoadedModel {
            encoder: open_session(&environment, &encoder_path)?,
            decoder_joint: open_session(&environment, &decoder_path)?,
            _environment: environment,
            blank_id: tokens.len(),
            tokens,
        });
        Ok(())
    }

    fn unload(&mut self) {
        self.model = None;
    }

    fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    fn recognize(
        &mut self,
        samples: &[f32],
        language: LanguageMode,
    ) -> Result<String, EngineError> {
        let model = self.model.as_mut().ok_or(EngineError::NotLoaded)?;
        if samples.is_empty() {
            return Ok(String::new());
        }
        let features = self.front_end.compute(samples);
        let encoded = model.encode(features.values, features.frames, language)?;
        let blank_bias = if samples.len() < SHORT_CLIP_SAMPLES {
            SHORT_CLIP_BLANK_BIAS
        } else {
            BLANK_BIAS
        };
        let ids = model.greedy_decode(&encoded, blank_bias)?;
        Ok(model.detokenize(&ids))
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

impl LoadedModel {
    fn encode(
        &mut self,
        features: Vec<f32>,
        frames: usize,
        language: LanguageMode,
    ) -> Result<Vec<f32>, EngineError> {
        let inputs = ort::inputs! {
            "audio_signal" => Tensor::from_array(([1, 128, frames], features)).map_err(model_error)?,
            "length" => Tensor::from_array(([1], vec![frames as i64])).map_err(model_error)?,
            "prompt_index" => Tensor::from_array(([1], vec![language.prompt_index()])).map_err(model_error)?,
        };
        let outputs = self.encoder.run(inputs).map_err(model_error)?;
        let (_, encoded) = outputs["encoded"]
            .try_extract_tensor::<f32>()
            .map_err(model_error)?;
        let (_, length) = outputs["encoded_length"]
            .try_extract_tensor::<i64>()
            .map_err(model_error)?;
        let valid = usize::try_from(length.first().copied().unwrap_or(0)).unwrap_or(0);
        let valid = valid.min(encoded.len() / ENCODER_WIDTH);
        Ok(encoded[..valid * ENCODER_WIDTH].to_vec())
    }

    fn greedy_decode(
        &mut self,
        encoded: &[f32],
        blank_bias: f32,
    ) -> Result<Vec<usize>, EngineError> {
        let mut ids = Vec::new();
        let mut last = self.blank_id;
        let mut state_1 = vec![0.0f32; STATE_SIZE];
        let mut state_2 = vec![0.0f32; STATE_SIZE];

        for frame in encoded.as_chunks::<ENCODER_WIDTH>().0 {
            for _ in 0..MAX_SYMBOLS_PER_FRAME {
                let inputs = ort::inputs! {
                    "encoder_outputs" => Tensor::from_array(([1, ENCODER_WIDTH, 1], frame.to_vec())).map_err(model_error)?,
                    "targets" => Tensor::from_array(([1, 1], vec![last as i32])).map_err(model_error)?,
                    "target_length" => Tensor::from_array(([1], vec![1i32])).map_err(model_error)?,
                    "input_states_1" => Tensor::from_array(([2, 1, 640], state_1.clone())).map_err(model_error)?,
                    "input_states_2" => Tensor::from_array(([2, 1, 640], state_2.clone())).map_err(model_error)?,
                };
                let outputs = self.decoder_joint.run(inputs).map_err(model_error)?;
                let (_, scores) = outputs["outputs"]
                    .try_extract_tensor::<f32>()
                    .map_err(model_error)?;
                let best = argmax_with_blank_bias(scores, self.blank_id, blank_bias);
                if best == self.blank_id {
                    break;
                }
                ids.push(best);
                last = best;
                state_1 = outputs["output_states_1"]
                    .try_extract_tensor::<f32>()
                    .map_err(model_error)?
                    .1
                    .to_vec();
                state_2 = outputs["output_states_2"]
                    .try_extract_tensor::<f32>()
                    .map_err(model_error)?
                    .1
                    .to_vec();
            }
        }
        Ok(ids)
    }

    fn detokenize(&self, ids: &[usize]) -> String {
        ids.iter()
            .filter_map(|&id| self.tokens.get(id))
            .filter(|piece| !(piece.starts_with('<') && piece.ends_with('>')))
            .map(String::as_str)
            .collect::<String>()
            .replace('\u{2581}', " ")
            .replace("\u{0E4D}\u{0E32}", "\u{0E33}")
            .trim()
            .to_owned()
    }
}

fn argmax_with_blank_bias(scores: &[f32], blank_id: usize, blank_bias: f32) -> usize {
    let mut best = blank_id;
    let mut best_score = f32::NEG_INFINITY;
    for (id, &score) in scores.iter().enumerate() {
        let score = if id == blank_id {
            score + blank_bias
        } else {
            score
        };
        if score > best_score {
            best = id;
            best_score = score;
        }
    }
    best
}
