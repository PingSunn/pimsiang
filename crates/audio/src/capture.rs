use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{
    AudioError, AudioSink, InputDevice, InputDevices, Microphone, MicrophoneChoice, Opened,
    StreamErrorSink, TARGET_SAMPLE_RATE, downmix, resample, select,
};

pub struct CpalDevices;

fn describe(device: &cpal::Device) -> Option<InputDevice> {
    let id = device.id().ok()?.to_string();
    let name = device.description().ok()?.name().to_owned();
    Some(InputDevice { id, name })
}

impl InputDevices for CpalDevices {
    fn list(&self) -> Result<Vec<InputDevice>, AudioError> {
        let devices = cpal::default_host().input_devices().map_err(mic_error)?;
        Ok(devices.filter_map(|device| describe(&device)).collect())
    }

    fn default_device(&self) -> Option<InputDevice> {
        describe(&cpal::default_host().default_input_device()?)
    }
}

pub struct MicCapture {
    sink: AudioSink,
    on_error: Option<StreamErrorSink>,
    choice: MicrophoneChoice,
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
            on_error: None,
            choice: MicrophoneChoice::default(),
            running: None,
        }
    }

    pub fn on_stream_error(mut self, on_error: StreamErrorSink) -> Self {
        self.on_error = Some(on_error);
        self
    }
}

impl Microphone for MicCapture {
    fn choose(&mut self, choice: MicrophoneChoice) {
        self.choice = choice;
    }

    fn devices(&self) -> Vec<InputDevice> {
        CpalDevices.list().unwrap_or_default()
    }

    fn start(&mut self) -> Result<Opened, AudioError> {
        if self.running.is_some() {
            return Ok(Opened::AsChosen);
        }
        let (device, opened) = select(&CpalDevices, &self.choice)?;
        if opened == Opened::FellBackToDefault {
            tracing::warn!(chosen = ?self.choice, using = %device.name, "chosen microphone is gone; using the system default");
        }
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let sink = self.sink.clone();
        let on_error = self.on_error.clone();
        let thread = thread::spawn(move || match open_stream(&device.id, sink, on_error) {
            Ok((stream, chunker)) => {
                let _ = ready_tx.send(Ok(()));
                let _ = stop_rx.recv();
                drop(stream);
                if let Ok(mut chunker) = chunker.lock() {
                    chunker.flush();
                }
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
                Ok(opened)
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

fn open_stream(
    id: &str,
    sink: AudioSink,
    on_stream_error: Option<StreamErrorSink>,
) -> Result<(cpal::Stream, Arc<Mutex<Chunker>>), AudioError> {
    let host = cpal::default_host();
    let device = id
        .parse::<cpal::DeviceId>()
        .ok()
        .and_then(|id| host.device_by_id(&id))
        .ok_or_else(|| AudioError::Microphone("input device not found".to_owned()))?;
    let supported = device.default_input_config().map_err(mic_error)?;
    let format = supported.sample_format();
    let channels = usize::from(supported.channels());
    let rate = supported.sample_rate();
    let chunker = Arc::new(Mutex::new(Chunker::new(channels, rate, sink)));
    let f32_chunker = Arc::clone(&chunker);
    let i16_chunker = Arc::clone(&chunker);
    let on_error = move |error: cpal::Error| {
        tracing::warn!(%error, "microphone stream error");
        if let Some(report) = &on_stream_error {
            report();
        }
    };
    let stream = match format {
        SampleFormat::F32 => device
            .build_input_stream(
                supported.into(),
                move |data: &[f32], _: &_| {
                    if let Ok(mut chunker) = f32_chunker.lock() {
                        chunker.push(data);
                    }
                },
                on_error.clone(),
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
                    if let Ok(mut chunker) = i16_chunker.lock() {
                        chunker.push(&floats);
                    }
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
    Ok((stream, chunker))
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

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let rest = std::mem::take(&mut self.pending);
        let mono = downmix(&rest, self.channels);
        if mono.is_empty() {
            return;
        }
        (self.sink)(resample(&mono, self.rate, TARGET_SAMPLE_RATE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collecting_sink() -> (AudioSink, Arc<Mutex<Vec<Vec<f32>>>>) {
        let blocks = Arc::new(Mutex::new(Vec::new()));
        let store = Arc::clone(&blocks);
        let sink: AudioSink = Arc::new(move |block| {
            if let Ok(mut store) = store.lock() {
                store.push(block);
            }
        });
        (sink, blocks)
    }

    #[test]
    fn flush_emits_the_partial_block_once() {
        let (sink, blocks) = collecting_sink();
        let mut chunker = Chunker::new(1, TARGET_SAMPLE_RATE, sink);

        chunker.push(&vec![0.5; 1_600 + 700]);
        chunker.flush();
        chunker.flush();

        let blocks = blocks.lock().unwrap();
        let lengths: Vec<usize> = blocks.iter().map(Vec::len).collect();
        assert_eq!(lengths, [1_600, 700]);
    }

    #[test]
    fn flush_with_nothing_pending_emits_nothing() {
        let (sink, blocks) = collecting_sink();
        let mut chunker = Chunker::new(2, TARGET_SAMPLE_RATE, sink);

        chunker.push(&vec![0.5; 3_200]);
        chunker.flush();

        assert_eq!(blocks.lock().unwrap().len(), 1);
    }
}
