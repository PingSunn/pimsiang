use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{AudioError, AudioSink, Microphone, TARGET_SAMPLE_RATE, downmix, resample};

pub struct MicCapture {
    sink: AudioSink,
    running: Option<Running>,
}

struct Running {
    stop: mpsc::Sender<()>,
    thread: JoinHandle<()>,
}

impl MicCapture {
    pub fn new(sink: AudioSink) -> Self {
        Self {
            sink,
            running: None,
        }
    }
}

impl Microphone for MicCapture {
    fn start(&mut self) -> Result<(), AudioError> {
        if self.running.is_some() {
            return Ok(());
        }
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let sink = self.sink.clone();
        let thread = thread::spawn(move || match open_stream(sink) {
            Ok(stream) => {
                let _ = ready_tx.send(Ok(()));
                let _ = stop_rx.recv();
                drop(stream);
            }
            Err(error) => {
                let _ = ready_tx.send(Err(error));
            }
        });
        match ready_rx.recv() {
            Ok(Ok(())) => {
                self.running = Some(Running {
                    stop: stop_tx,
                    thread,
                });
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => {
                let _ = thread.join();
                Err(AudioError::Microphone(
                    "capture thread ended unexpectedly".to_owned(),
                ))
            }
        }
    }

    fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            let _ = running.stop.send(());
            let _ = running.thread.join();
        }
    }
}

impl Drop for MicCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn mic_error(error: impl std::fmt::Display) -> AudioError {
    AudioError::Microphone(error.to_string())
}

fn open_stream(sink: AudioSink) -> Result<cpal::Stream, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AudioError::Microphone("no input device".to_owned()))?;
    let supported = device.default_input_config().map_err(mic_error)?;
    let format = supported.sample_format();
    let channels = usize::from(supported.channels());
    let rate = supported.sample_rate();
    let mut chunker = Chunker::new(channels, rate, sink);
    let on_error = |error: cpal::Error| tracing::warn!(%error, "microphone stream error");
    let stream = match format {
        SampleFormat::F32 => device
            .build_input_stream(
                supported.into(),
                move |data: &[f32], _: &_| chunker.push(data),
                on_error,
                None,
            )
            .map_err(mic_error)?,
        SampleFormat::I16 => device
            .build_input_stream(
                supported.into(),
                move |data: &[i16], _: &_| {
                    let floats: Vec<f32> = data
                        .iter()
                        .map(|&s| f32::from(s) / f32::from(i16::MAX))
                        .collect();
                    chunker.push(&floats);
                },
                on_error,
                None,
            )
            .map_err(mic_error)?,
        other => {
            return Err(AudioError::Microphone(format!(
                "unsupported sample format {other}"
            )));
        }
    };
    stream.play().map_err(mic_error)?;
    Ok(stream)
}

struct Chunker {
    channels: usize,
    rate: u32,
    block_frames: usize,
    pending: Vec<f32>,
    sink: AudioSink,
}

impl Chunker {
    fn new(channels: usize, rate: u32, sink: AudioSink) -> Self {
        Self {
            channels: channels.max(1),
            rate,
            block_frames: (rate / 10).max(1) as usize,
            pending: Vec::new(),
            sink,
        }
    }

    fn push(&mut self, interleaved: &[f32]) {
        self.pending.extend_from_slice(interleaved);
        let block_samples = self.block_frames * self.channels;
        while self.pending.len() >= block_samples {
            let block: Vec<f32> = self.pending.drain(..block_samples).collect();
            let mono = downmix(&block, self.channels);
            (self.sink)(resample(&mono, self.rate, TARGET_SAMPLE_RATE));
        }
    }
}
