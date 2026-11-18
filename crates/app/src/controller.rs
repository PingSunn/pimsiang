use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use audio::Microphone;
use engine::{Engine, LanguageMode, UtteranceOptions};
use platform::{Platform, Status, TriggerEvent, TriggerState};

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub hold_threshold: Duration,
    pub language: LanguageMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hold_threshold: Duration::from_millis(250),
            language: LanguageMode::default(),
        }
    }
}

#[derive(Debug)]
pub enum Input {
    Trigger(TriggerEvent),
    Audio(Vec<f32>),
    Tick(Instant),
}

enum State {
    Idle,
    Pending { since: Instant },
    Listening,
}

pub struct Controller<P, E, M> {
    platform: P,
    engine: E,
    microphone: M,
    config: Config,
    state: State,
}

impl<P: Platform, E: Engine, M: Microphone> Controller<P, E, M> {
    pub fn new(platform: P, engine: E, microphone: M, config: Config) -> Self {
        Self {
            platform,
            engine,
            microphone,
            config,
            state: State::Idle,
        }
    }

    pub fn platform(&self) -> &P {
        &self.platform
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }

    pub fn microphone(&self) -> &M {
        &self.microphone
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        match self.state {
            State::Pending { since } => Some(since + self.config.hold_threshold),
            State::Idle | State::Listening => None,
        }
    }

    pub fn run(mut self, inputs: Receiver<Input>) -> Self {
        loop {
            let input = match self.next_deadline() {
                Some(deadline) => {
                    match inputs.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                        Ok(input) => input,
                        Err(RecvTimeoutError::Timeout) => Input::Tick(Instant::now()),
                        Err(RecvTimeoutError::Disconnected) => return self,
                    }
                }
                None => match inputs.recv() {
                    Ok(input) => input,
                    Err(_) => return self,
                },
            };
            self.handle(input);
        }
    }

    pub fn handle(&mut self, input: Input) {
        match input {
            Input::Trigger(event) => match event.state {
                TriggerState::Pressed => self.on_pressed(event.at),
                TriggerState::Released => self.on_released(),
            },
            Input::Audio(samples) => self.on_audio(&samples),
            Input::Tick(now) => self.on_tick(now),
        }
    }

    fn on_pressed(&mut self, at: Instant) {
        if matches!(self.state, State::Idle) {
            self.state = State::Pending { since: at };
        }
    }

    fn on_released(&mut self) {
        match self.state {
            State::Idle => {}
            State::Pending { .. } => self.state = State::Idle,
            State::Listening => self.finish(),
        }
    }

    fn on_tick(&mut self, now: Instant) {
        if let Some(deadline) = self.next_deadline()
            && now >= deadline
        {
            self.begin();
        }
    }

    fn on_audio(&mut self, samples: &[f32]) {
        if !matches!(self.state, State::Listening) {
            return;
        }
        if let Err(error) = self.engine.push_audio(samples) {
            tracing::warn!(%error, "decoding failed while listening");
        }
    }

    fn begin(&mut self) {
        self.state = State::Idle;
        if !self.engine.is_loaded()
            && let Err(error) = self.engine.load()
        {
            tracing::error!(%error, "cannot load the Model");
            return;
        }
        let options = UtteranceOptions {
            language: self.config.language,
        };
        if let Err(error) = self.engine.start_utterance(options) {
            tracing::error!(%error, "cannot start an Utterance");
            return;
        }
        if let Err(error) = self.microphone.start() {
            tracing::error!(%error, "cannot start the microphone");
            self.engine.cancel_utterance();
            return;
        }
        self.state = State::Listening;
        self.platform.show_status(Status::Listening);
    }

    fn finish(&mut self) {
        self.state = State::Idle;
        self.microphone.stop();
        self.platform.show_status(Status::Transcribing);
        match self.engine.finish_utterance() {
            Ok(text) => self.deliver(text.trim()),
            Err(error) => tracing::error!(%error, "cannot finish the Utterance"),
        }
        self.platform.show_status(Status::Ready);
    }

    fn deliver(&mut self, text: &str) {
        if text.is_empty() {
            tracing::info!("Utterance produced no text");
            return;
        }
        match self.platform.paste(text) {
            Ok(()) => tracing::info!(chars = text.chars().count(), "pasted Final text"),
            Err(error) => tracing::error!(%error, "cannot paste the Final text"),
        }
    }
}
