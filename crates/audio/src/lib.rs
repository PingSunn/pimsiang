use std::f64::consts::PI;
use std::path::Path;

mod capture;
mod devices;
pub mod fake;

pub use capture::{CpalDevices, MicCapture};
pub use devices::{InputDevice, InputDevices, MicrophoneChoice, Opened, select};

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

const SINC_ZERO_CROSSINGS: f64 = 16.0;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("cannot read WAV file")]
    Wav(#[from] hound::Error),
    #[error("WAV file has no channels")]
    NoChannels,
    #[error("microphone error: {0}")]
    Microphone(String),
}

pub trait Microphone {
    fn choose(&mut self, choice: MicrophoneChoice);
    fn devices(&self) -> Vec<InputDevice>;
    fn start(&mut self) -> Result<Opened, AudioError>;
    fn stop(&mut self);
}

pub type AudioSink = std::sync::Arc<dyn Fn(Vec<f32>) + Send + Sync>;

pub fn read_wav_16k_mono(path: &Path) -> Result<Vec<f32>, AudioError> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(AudioError::NoChannels);
    }
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()?
        }
    };
    let mono = downmix(&interleaved, usize::from(spec.channels));
    Ok(resample(&mono, spec.sample_rate, TARGET_SAMPLE_RATE))
}

pub fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let step = f64::from(from_rate) / f64::from(to_rate);
    let cutoff = (f64::from(to_rate) / f64::from(from_rate)).min(1.0);
    let half_width = SINC_ZERO_CROSSINGS / cutoff;
    let out_len = (samples.len() as u64 * u64::from(to_rate) / u64::from(from_rate)) as usize;
    let last = samples.len() as i64 - 1;

    (0..out_len)
        .map(|j| {
            let centre = j as f64 * step;
            let first = ((centre - half_width).ceil() as i64).max(0);
            let end = ((centre + half_width).floor() as i64).min(last);
            let mut acc = 0.0f64;
            let mut weight_sum = 0.0f64;
            for i in first..=end {
                let x = i as f64 - centre;
                let w = cutoff * sinc(cutoff * x) * hann(x / half_width);
                acc += w * f64::from(samples[i as usize]);
                weight_sum += w;
            }
            if weight_sum.abs() > f64::EPSILON {
                (acc / weight_sum) as f32
            } else {
                0.0
            }
        })
        .collect()
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

fn hann(x: f64) -> f64 {
    if x.abs() >= 1.0 {
        0.0
    } else {
        0.5 * (1.0 + (PI * x).cos())
    }
}
