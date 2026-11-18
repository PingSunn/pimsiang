use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use audio::Microphone;
use engine::{Engine, LanguageMode, UtteranceOptions};
use platform::{
    Permission, PermissionReport, PermissionStatus, PillError, PillState, Platform, Status,
    TriggerEvent, TriggerSink, TriggerState,
};

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub hold_threshold: Duration,
    pub language: LanguageMode,
    pub pill_linger: Duration,
    pub error_linger: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hold_threshold: Duration::from_millis(250),
            language: LanguageMode::default(),
            pill_linger: Duration::from_millis(400),
            error_linger: Duration::from_secs(3),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionCommand {
    Grant(Permission),
}

#[derive(Debug)]
pub enum Input {
    Trigger(TriggerEvent),
    Audio(Vec<f32>),
    Tick(Instant),
    Permission(PermissionCommand),
}

type TriggerSource = Box<dyn FnMut() -> TriggerSink + Send>;

struct PermissionWatch {
    interval: Duration,
    next_poll: Instant,
    trigger_source: TriggerSource,
    shown: Option<PermissionReport>,
    listening: bool,
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
    hide_at: Option<Instant>,
    shown_partial: Option<String>,
    watch: Option<PermissionWatch>,
}

impl<P: Platform, E: Engine, M: Microphone> Controller<P, E, M> {
    pub fn new(platform: P, engine: E, microphone: M, config: Config) -> Self {
        Self {
            platform,
            engine,
            microphone,
            config,
            state: State::Idle,
            hide_at: None,
            shown_partial: None,
            watch: None,
        }
    }

    pub fn watch_permissions(
        mut self,
        interval: Duration,
        trigger_source: impl FnMut() -> TriggerSink + Send + 'static,
    ) -> Self {
        self.watch = Some(PermissionWatch {
            interval,
            next_poll: Instant::now(),
            trigger_source: Box::new(trigger_source),
            shown: None,
            listening: false,
        });
        self
    }

    pub fn platform(&self) -> &P {
        &self.platform
    }

    pub fn platform_mut(&mut self) -> &mut P {
        &mut self.platform
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }

    pub fn microphone(&self) -> &M {
        &self.microphone
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        let pending = match self.state {
            State::Pending { since } => Some(since + self.config.hold_threshold),
            State::Idle | State::Listening => None,
        };
        let polling = self.watch.as_ref().map(|watch| watch.next_poll);
        [pending, self.hide_at, polling].into_iter().flatten().min()
    }

    pub fn run(mut self, inputs: Receiver<Input>) -> Self {
        let mut carried = None;
        loop {
            let input = match carried.take() {
                Some(input) => input,
                None => match self.next_deadline() {
                    Some(deadline) => {
                        match inputs
                            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                        {
                            Ok(input) => input,
                            Err(RecvTimeoutError::Timeout) => Input::Tick(Instant::now()),
                            Err(RecvTimeoutError::Disconnected) => return self,
                        }
                    }
                    None => match inputs.recv() {
                        Ok(input) => input,
                        Err(_) => return self,
                    },
                },
            };
            let input = match input {
                Input::Audio(samples) => {
                    let (samples, next) = coalesce(samples, &inputs);
                    carried = next;
                    Input::Audio(samples)
                }
                other => other,
            };
            if let Some(next) = self.dispatch(input, Some(&inputs)) {
                carried = Some(next);
            }
        }
    }

    pub fn handle(&mut self, input: Input) {
        self.dispatch(input, None);
    }

    fn dispatch(&mut self, input: Input, queued: Option<&Receiver<Input>>) -> Option<Input> {
        match input {
            Input::Trigger(event) => match event.state {
                TriggerState::Pressed => self.on_pressed(event.at),
                TriggerState::Released => return self.on_released(queued),
            },
            Input::Audio(samples) => self.on_audio(&samples),
            Input::Tick(now) => self.on_tick(now),
            Input::Permission(PermissionCommand::Grant(permission)) => self.grant(permission),
        }
        None
    }

    fn on_pressed(&mut self, at: Instant) {
        if matches!(self.state, State::Idle) {
            self.state = State::Pending { since: at };
        }
    }

    fn on_released(&mut self, queued: Option<&Receiver<Input>>) -> Option<Input> {
        match self.state {
            State::Idle => None,
            State::Pending { .. } => {
                self.state = State::Idle;
                None
            }
            State::Listening => self.finish(queued),
        }
    }

    fn on_tick(&mut self, now: Instant) {
        if let Some(hide_at) = self.hide_at
            && now >= hide_at
        {
            self.hide_at = None;
            self.platform.show_pill(PillState::Hidden);
        }
        if let State::Pending { since } = self.state
            && now >= since + self.config.hold_threshold
        {
            self.begin();
        }
        if let Some(watch) = &self.watch
            && now >= watch.next_poll
        {
            self.poll_permissions(now);
        }
    }

    fn poll_permissions(&mut self, now: Instant) {
        let report = PermissionReport::read(&self.platform);
        let Some(watch) = self.watch.as_mut() else {
            return;
        };
        watch.next_poll = now + watch.interval;
        if watch.shown != Some(report) {
            watch.shown = Some(report);
            self.platform.show_permissions(report);
        }
        if !watch.listening && report.input_monitoring == PermissionStatus::Granted {
            match self.platform.listen_trigger((watch.trigger_source)()) {
                Ok(()) => {
                    watch.listening = true;
                    tracing::info!("watching the Trigger key");
                }
                Err(error) => tracing::warn!(%error, "cannot watch the Trigger key yet"),
            }
        }
    }

    fn grant(&mut self, permission: Permission) {
        let status = self.platform.permission_status(permission);
        match permission {
            Permission::Microphone if status == PermissionStatus::NotDetermined => {
                self.platform.request_permission(permission);
            }
            Permission::Accessibility | Permission::InputMonitoring => {
                self.platform.request_permission(permission);
                self.platform.open_settings(permission);
            }
            Permission::Microphone => self.platform.open_settings(permission),
        }
    }

    fn missing_permission(&self) -> Option<Permission> {
        [Permission::Microphone, Permission::Accessibility]
            .into_iter()
            .find(|permission| {
                self.platform.permission_status(*permission) != PermissionStatus::Granted
            })
    }

    fn on_audio(&mut self, samples: &[f32]) {
        if !matches!(self.state, State::Listening) {
            return;
        }
        match self.engine.push_audio(samples) {
            Ok(Some(text)) => self.show_partial(text),
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, "decoding failed while listening"),
        }
    }

    fn show_partial(&mut self, text: String) {
        let text = text.trim().to_owned();
        if text.is_empty() || self.shown_partial.as_deref() == Some(text.as_str()) {
            return;
        }
        self.shown_partial = Some(text.clone());
        self.platform.show_pill(PillState::Partial(text));
    }

    fn fail(&mut self, error: PillError) {
        self.hide_at = Some(Instant::now() + self.config.error_linger);
        self.platform.show_pill(PillState::Error(error));
    }

    fn begin(&mut self) {
        self.state = State::Idle;
        if let Some(permission) = self.missing_permission() {
            tracing::warn!(permission = permission.name(), "permission is missing");
            self.fail(PillError::PermissionMissing(permission));
            return;
        }
        if !self.engine.is_loaded()
            && let Err(error) = self.engine.load()
        {
            tracing::error!(%error, "cannot load the Model");
            self.fail(PillError::ModelNotLoaded);
            return;
        }
        let options = UtteranceOptions {
            language: self.config.language,
        };
        if let Err(error) = self.engine.start_utterance(options) {
            tracing::error!(%error, "cannot start an Utterance");
            self.fail(PillError::TranscriptionFailed);
            return;
        }
        if let Err(error) = self.microphone.start() {
            tracing::error!(%error, "cannot start the microphone");
            self.engine.cancel_utterance();
            self.fail(PillError::MicrophoneUnavailable);
            return;
        }
        self.state = State::Listening;
        self.hide_at = None;
        self.shown_partial = None;
        self.platform.show_status(Status::Listening);
        self.platform.show_pill(PillState::Listening);
    }

    fn finish(&mut self, queued: Option<&Receiver<Input>>) -> Option<Input> {
        self.microphone.stop();
        let mut next = None;
        if let Some(queued) = queued {
            let (tail, other) = coalesce(Vec::new(), queued);
            next = other;
            if !tail.is_empty()
                && let Err(error) = self.engine.push_tail(&tail)
            {
                tracing::warn!(%error, "decoding failed while draining the last audio");
            }
        }
        self.state = State::Idle;
        self.platform.show_status(Status::Transcribing);
        self.platform.show_pill(PillState::Transcribing);
        let delivered = match self.engine.finish_utterance() {
            Ok(text) => self.deliver(text.trim()),
            Err(error) => {
                tracing::error!(%error, "cannot finish the Utterance");
                Err(PillError::TranscriptionFailed)
            }
        };
        self.platform.show_status(Status::Ready);
        match delivered {
            Ok(()) => self.hide_at = Some(Instant::now() + self.config.pill_linger),
            Err(error) => self.fail(error),
        }
        next
    }

    fn deliver(&mut self, text: &str) -> Result<(), PillError> {
        if text.is_empty() {
            tracing::info!("Utterance produced no text");
            return Ok(());
        }
        match self.platform.paste(text) {
            Ok(()) => {
                tracing::info!(chars = text.chars().count(), "pasted Final text");
                Ok(())
            }
            Err(error) => {
                tracing::error!(%error, "cannot paste the Final text");
                Err(PillError::PasteFailed)
            }
        }
    }
}

fn coalesce(mut samples: Vec<f32>, inputs: &Receiver<Input>) -> (Vec<f32>, Option<Input>) {
    loop {
        match inputs.try_recv() {
            Ok(Input::Audio(more)) => samples.extend_from_slice(&more),
            Ok(other) => return (samples, Some(other)),
            Err(_) => return (samples, None),
        }
    }
}
