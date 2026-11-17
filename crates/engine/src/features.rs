use std::f64::consts::PI;
use std::sync::Arc;

use realfft::{RealFftPlanner, RealToComplex};

const SAMPLE_RATE: f64 = 16_000.0;
const N_FFT: usize = 512;
const WIN_LENGTH: usize = 400;
const HOP_LENGTH: usize = 160;
const N_MELS: usize = 128;
const PRE_EMPHASIS: f32 = 0.97;
const LOG_GUARD: f32 = 1.0 / (1u32 << 24) as f32;

pub struct LogMel {
    pub bands: usize,
    pub frames: usize,
    pub values: Vec<f32>,
}

impl LogMel {
    pub fn value(&self, band: usize, frame: usize) -> f32 {
        self.values[band * self.frames + frame]
    }
}

pub fn log_mel(samples: &[f32]) -> LogMel {
    MelFrontEnd::new().compute(samples)
}

pub(crate) struct MelFrontEnd {
    window: Vec<f32>,
    filterbank: Vec<Vec<(usize, f32)>>,
    fft: Arc<dyn RealToComplex<f32>>,
}

impl MelFrontEnd {
    pub(crate) fn new() -> Self {
        let pad = (N_FFT - WIN_LENGTH) / 2;
        let mut window = vec![0.0f32; N_FFT];
        for (i, w) in window[pad..pad + WIN_LENGTH].iter_mut().enumerate() {
            *w = (0.5 - 0.5 * (2.0 * PI * i as f64 / (WIN_LENGTH - 1) as f64).cos()) as f32;
        }
        Self {
            window,
            filterbank: slaney_filterbank(),
            fft: RealFftPlanner::<f32>::new().plan_fft_forward(N_FFT),
        }
    }

    pub(crate) fn compute(&self, samples: &[f32]) -> LogMel {
        let emphasized: Vec<f32> = samples
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                if i == 0 {
                    s
                } else {
                    s - PRE_EMPHASIS * samples[i - 1]
                }
            })
            .collect();

        let half = N_FFT / 2;
        let mut padded = vec![0.0f32; emphasized.len() + 2 * half];
        padded[half..half + emphasized.len()].copy_from_slice(&emphasized);
        let frames = (padded.len() - N_FFT) / HOP_LENGTH + 1;

        let mut values = vec![0.0f32; N_MELS * frames];
        let mut input = self.fft.make_input_vec();
        let mut spectrum = self.fft.make_output_vec();
        let mut power = vec![0.0f32; spectrum.len()];
        for frame in 0..frames {
            let start = frame * HOP_LENGTH;
            for ((dst, &src), &w) in input
                .iter_mut()
                .zip(&padded[start..start + N_FFT])
                .zip(&self.window)
            {
                *dst = src * w;
            }
            if self.fft.process(&mut input, &mut spectrum).is_err() {
                continue;
            }
            for (p, c) in power.iter_mut().zip(&spectrum) {
                *p = c.norm_sqr();
            }
            for (band, weights) in self.filterbank.iter().enumerate() {
                let energy: f32 = weights.iter().map(|&(bin, w)| w * power[bin]).sum();
                values[band * frames + frame] = (energy + LOG_GUARD).ln();
            }
        }

        LogMel {
            bands: N_MELS,
            frames,
            values,
        }
    }
}

fn hz_to_mel(hz: f64) -> f64 {
    let f_sp = 200.0 / 3.0;
    let min_log_hz = 1000.0;
    let min_log_mel = min_log_hz / f_sp;
    let log_step = 6.4f64.ln() / 27.0;
    if hz >= min_log_hz {
        min_log_mel + (hz / min_log_hz).ln() / log_step
    } else {
        hz / f_sp
    }
}

fn mel_to_hz(mel: f64) -> f64 {
    let f_sp = 200.0 / 3.0;
    let min_log_hz = 1000.0;
    let min_log_mel = min_log_hz / f_sp;
    let log_step = 6.4f64.ln() / 27.0;
    if mel >= min_log_mel {
        min_log_hz * (log_step * (mel - min_log_mel)).exp()
    } else {
        mel * f_sp
    }
}

fn slaney_filterbank() -> Vec<Vec<(usize, f32)>> {
    let bins = N_FFT / 2 + 1;
    let nyquist = SAMPLE_RATE / 2.0;
    let fft_freqs: Vec<f64> = (0..bins)
        .map(|k| k as f64 * nyquist / (bins - 1) as f64)
        .collect();
    let max_mel = hz_to_mel(nyquist);
    let mel_freqs: Vec<f64> = (0..N_MELS + 2)
        .map(|i| mel_to_hz(max_mel * i as f64 / (N_MELS + 1) as f64))
        .collect();

    (0..N_MELS)
        .map(|m| {
            let (left, centre, right) = (mel_freqs[m], mel_freqs[m + 1], mel_freqs[m + 2]);
            let norm = 2.0 / (right - left);
            fft_freqs
                .iter()
                .enumerate()
                .filter_map(|(bin, &f)| {
                    let lower = (f - left) / (centre - left);
                    let upper = (right - f) / (right - centre);
                    let w = lower.min(upper).max(0.0) * norm;
                    (w > 0.0).then_some((bin, w as f32))
                })
                .collect()
        })
        .collect()
}
