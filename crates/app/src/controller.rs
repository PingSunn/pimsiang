use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use audio::{Microphone, MicrophoneChoice, Opened};
use engine::{Engine, LanguageMode, UtteranceOptions};
use platform::{
    Focus, MicrophoneEntry, MicrophoneMenu, Permission, PermissionReport, PermissionStatus,
    PillError, PillNotice, PillState, Platform, Sound, Status, TriggerEvent, TriggerKey,
    TriggerSink, TriggerState,
};
use store::Kind;

use crate::preferences::Preferences;

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub hold_threshold: Duration,
    pub language: LanguageMode,
    pub pill_linger: Duration,
    pub error_linger: Duration,
    pub notice_linger: Duration,
    pub double_tap_window: Duration,
    pub sounds: bool,
    pub clipboard_restore: bool,
    pub trigger_key: TriggerKey,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hold_threshold: Duration::from_millis(250),
            language: LanguageMode::default(),
            pill_linger: Duration::from_millis(400),
            error_linger: Duration::from_secs(3),
            notice_linger: Duration::from_millis(1500),
            double_tap_window: Duration::from_millis(400),
            sounds: true,
            clipboard_restore: true,
            trigger_key: TriggerKey::default(),
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
    Microphone(MicrophoneChoice),
    MicrophoneLost,
    ModelReady,
    PasteText(String),
    NoTranscript,
    Preferences(Preferences),
}

type HistorySink = Box<dyn FnMut(&str, Kind) + Send>;
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
    Pending {
        since: Instant,
        second_tap: bool,
        shift: bool,
    },
    Listening {
        hands_free: bool,
    },
    EndingHandsFree,
}

pub struct Controller<P, E, M> {
    platform: P,
    engine: E,
    microphone: M,
    config: Config,
    state: State,
    quick_note: bool,
    last_tap: Option<Instant>,
    hide_at: Option<Instant>,
    shown_partial: Option<String>,
    notice_until: Option<Instant>,
    held_partial: Option<String>,
    watch: Option<PermissionWatch>,
    shown_microphones: Option<MicrophoneMenu>,
    choice: MicrophoneChoice,
    prompted: Vec<Permission>,
    model_ready: bool,
    history: Option<HistorySink>,
}

impl<P: Platform, E: Engine, M: Microphone> Controller<P, E, M> {
    pub fn new(platform: P, engine: E, microphone: M, config: Config) -> Self {
        Self {
            platform,
            engine,
            microphone,
            config,
            state: State::Idle,
            quick_note: false,
            last_tap: None,
            hide_at: None,
            shown_partial: None,
            notice_until: None,
            held_partial: None,
            watch: None,
            shown_microphones: None,
            choice: MicrophoneChoice::default(),
            prompted: Vec::new(),
            model_ready: true,
            history: None,
        }
    }

    pub fn record_history(mut self, sink: impl FnMut(&str, Kind) + Send + 'static) -> Self {
        self.history = Some(Box::new(sink));
        self
    }

    pub fn model_pending(mut self) -> Self {
        self.model_ready = false;
        self
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

    pub fn preload_model(&mut self) {
        if !self.model_ready {
            return;
        }
        if let Err(error) = self.engine.load() {
            tracing::error!(%error, "cannot load the Model");
            self.fail(PillError::ModelNotLoaded);
        }
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

    pub fn microphone_mut(&mut self) -> &mut M {
        &mut self.microphone
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        let pending = match self.state {
            State::Pending { since, .. } => Some(since + self.config.hold_threshold),
            State::Idle | State::Listening { .. } | State::EndingHandsFree => None,
        };
        let polling = self.watch.as_ref().map(|watch| watch.next_poll);
        let notice = self.notice_until.filter(|_| self.held_partial.is_some());
        [pending, self.hide_at, polling, notice]
            .into_iter()
            .flatten()
            .min()
    }

    pub fn run(mut self, inputs: Receiver<Input>) -> Self {
        let mut carried: VecDeque<Input> = VecDeque::new();
        loop {
            let input = match carried.pop_front() {
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
                    if let Some(next) = next {
                        carried.push_front(next);
                    }
                    Input::Audio(samples)
                }
                other => other,
            };
            carried.extend(self.dispatch(input, Some(&inputs)));
        }
    }

    pub fn handle(&mut self, input: Input) {
        self.dispatch(input, None);
    }

    fn dispatch(&mut self, input: Input, queued: Option<&Receiver<Input>>) -> Vec<Input> {
        match input {
            Input::Trigger(event) => match event.state {
                TriggerState::Pressed => self.on_pressed(event.at, event.shift),
                TriggerState::Released => return self.on_released(event.at, queued),
                TriggerState::Combined => self.on_combined(),
                TriggerState::Escape => self.on_escape(),
            },
            Input::Audio(samples) => self.on_audio(&samples),
            Input::Tick(now) => self.on_tick(now),
            Input::Permission(PermissionCommand::Grant(permission)) => self.grant(permission),
            Input::Microphone(choice) => {
                self.choice = choice.clone();
                self.microphone.choose(choice);
                self.shown_microphones = None;
                self.publish_microphones();
            }
            Input::MicrophoneLost => return self.on_microphone_lost(),
            Input::ModelReady => {
                self.model_ready = true;
                self.preload_model();
            }
            Input::PasteText(text) => self.paste_again(&text),
            Input::NoTranscript => self.show_notice(PillNotice::NoTranscriptYet),
            Input::Preferences(preferences) => self.apply_preferences(preferences),
        }
        Vec::new()
    }

    fn apply_preferences(&mut self, preferences: Preferences) {
        self.config.language = preferences.language;
        self.config.sounds = preferences.sounds;
        self.config.clipboard_restore = preferences.clipboard_restore;
        if preferences.trigger_key != self.config.trigger_key {
            self.config.trigger_key = preferences.trigger_key;
            self.platform.set_trigger_key(preferences.trigger_key);
        }
    }

    fn on_pressed(&mut self, at: Instant, shift: bool) {
        match self.state {
            State::Idle => {
                let second_tap = self.last_tap.take().is_some_and(|tap| {
                    at.saturating_duration_since(tap) <= self.config.double_tap_window
                });
                self.state = State::Pending {
                    since: at,
                    second_tap,
                    shift,
                };
            }
            State::Listening { hands_free: true } => self.state = State::EndingHandsFree,
            State::Pending { .. }
            | State::Listening { hands_free: false }
            | State::EndingHandsFree => {}
        }
    }

    fn on_released(&mut self, at: Instant, queued: Option<&Receiver<Input>>) -> Vec<Input> {
        match self.state {
            State::Idle | State::Listening { hands_free: true } => Vec::new(),
            State::Pending {
                second_tap, shift, ..
            } => {
                self.state = State::Idle;
                if second_tap {
                    self.begin(true, shift);
                } else {
                    self.last_tap = Some(at);
                }
                Vec::new()
            }
            State::Listening { hands_free: false } | State::EndingHandsFree => self.finish(queued),
        }
    }

    fn on_combined(&mut self) {
        self.last_tap = None;
        match self.state {
            State::Pending { .. } => self.state = State::Idle,
            State::Listening { hands_free: false } => self.cancel(),
            State::EndingHandsFree => self.state = State::Listening { hands_free: true },
            State::Idle | State::Listening { hands_free: true } => {}
        }
    }

    fn on_escape(&mut self) {
        self.last_tap = None;
        match self.state {
            State::Pending { .. } => self.state = State::Idle,
            State::Listening { .. } | State::EndingHandsFree => self.cancel(),
            State::Idle => {}
        }
    }

    fn cancel(&mut self) {
        self.microphone.stop();
        self.held_partial = None;
        self.notice_until = None;
        self.engine.cancel_utterance();
        self.play(Sound::Stop);
        self.state = State::Idle;
        self.hide_at = None;
        self.platform.show_status(Status::Ready);
        self.platform.show_pill(PillState::Hidden);
    }

    fn play(&mut self, sound: Sound) {
        if self.config.sounds {
            self.platform.play_sound(sound);
        }
    }

    fn on_microphone_lost(&mut self) -> Vec<Input> {
        if !matches!(self.state, State::Listening { .. } | State::EndingHandsFree) {
            return Vec::new();
        }
        tracing::warn!("the microphone stream ended in the middle of an Utterance");
        self.end(None, Some(PillNotice::MicrophoneLost))
    }

    fn on_tick(&mut self, now: Instant) {
        if let Some(until) = self.notice_until
            && now >= until
        {
            self.notice_until = None;
            if let Some(text) = self.held_partial.take() {
                self.shown_partial = Some(text.clone());
                self.platform.show_pill(PillState::Partial(text));
            }
        }
        if let Some(hide_at) = self.hide_at
            && now >= hide_at
        {
            self.hide_at = None;
            self.platform.show_pill(PillState::Hidden);
        }
        if let State::Pending { since, shift, .. } = self.state
            && now >= since + self.config.hold_threshold
        {
            self.state = State::Idle;
            self.begin(false, shift);
        }
        if let Some(watch) = &self.watch
            && now >= watch.next_poll
        {
            self.poll_permissions(now);
            self.publish_microphones();
        }
    }

    fn publish_microphones(&mut self) {
        let devices = self.microphone.devices();
        let selected = match &self.choice {
            MicrophoneChoice::Device(id) if devices.iter().any(|device| device.id == *id) => {
                Some(id.clone())
            }
            MicrophoneChoice::Device(_) | MicrophoneChoice::SystemDefault => None,
        };
        let menu = MicrophoneMenu {
            devices: devices
                .into_iter()
                .map(|device| MicrophoneEntry {
                    id: device.id,
                    name: device.name,
                })
                .collect(),
            selected,
        };
        if self.shown_microphones.as_ref() != Some(&menu) {
            self.shown_microphones = Some(menu.clone());
            self.platform.show_microphones(menu);
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
        if watch.listening && report.input_monitoring != PermissionStatus::Granted {
            watch.listening = false;
            self.platform.stop_trigger();
            tracing::warn!("Input Monitoring was revoked; the Trigger key is no longer watched");
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
            Permission::Accessibility if !self.prompted.contains(&permission) => {
                self.prompted.push(permission);
                self.platform.request_permission(permission);
            }
            Permission::InputMonitoring => {
                self.platform.request_permission(permission);
                self.platform.open_settings(permission);
            }
            Permission::Accessibility => self.platform.open_settings(permission),
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
        if !matches!(self.state, State::Listening { .. } | State::EndingHandsFree) {
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
        if self
            .notice_until
            .is_some_and(|until| Instant::now() < until)
        {
            self.held_partial = Some(text);
            return;
        }
        self.shown_partial = Some(text.clone());
        self.platform.show_pill(PillState::Partial(text));
    }

    fn fail(&mut self, error: PillError) {
        self.hide_at = Some(Instant::now() + self.config.error_linger);
        self.platform.show_pill(PillState::Error(error));
    }

    fn begin(&mut self, hands_free: bool, quick_note: bool) {
        if let Some(permission) = self.missing_permission() {
            tracing::warn!(permission = permission.name(), "permission is missing");
            self.fail(PillError::PermissionMissing(permission));
            return;
        }
        if !self.model_ready {
            self.fail(PillError::ModelNotReady);
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
        self.play(Sound::Start);
        let opened = match self.microphone.start() {
            Ok(opened) => opened,
            Err(error) => {
                tracing::error!(%error, "cannot start the microphone");
                self.engine.cancel_utterance();
                self.fail(PillError::MicrophoneUnavailable);
                return;
            }
        };
        self.state = State::Listening { hands_free };
        self.quick_note = quick_note;
        self.hide_at = None;
        self.shown_partial = None;
        self.held_partial = None;
        self.notice_until = None;
        self.platform.show_status(Status::Listening);
        self.platform.show_pill(PillState::Listening);
        if opened == Opened::FellBackToDefault {
            self.notice_until = Some(Instant::now() + self.config.notice_linger);
            self.platform
                .show_pill(PillState::Notice(PillNotice::MicrophoneFellBack));
        }
    }

    fn finish(&mut self, queued: Option<&Receiver<Input>>) -> Vec<Input> {
        self.end(queued, None)
    }

    fn end(&mut self, queued: Option<&Receiver<Input>>, notice: Option<PillNotice>) -> Vec<Input> {
        self.microphone.stop();
        self.held_partial = None;
        self.notice_until = None;
        self.play(Sound::Stop);
        let mut waiting = Vec::new();
        if let Some(queued) = queued {
            let (tail, other) = coalesce(Vec::new(), queued);
            waiting.extend(other);
            if !tail.is_empty()
                && let Err(error) = self.engine.push_tail(&tail)
            {
                tracing::warn!(%error, "decoding failed while draining the last audio");
            }
        }
        self.state = State::Idle;
        self.platform.show_status(Status::Transcribing);
        self.platform.show_pill(PillState::Transcribing);
        let result = self.engine.finish_utterance();
        if let Some(queued) = queued {
            waiting.extend(std::iter::from_fn(|| queued.try_recv().ok()));
        }
        let cancelled = waiting.iter().any(
            |input| matches!(input, Input::Trigger(event) if event.state == TriggerState::Escape),
        );
        let held = waiting
            .iter()
            .filter_map(|input| match input {
                Input::Trigger(event) => Some(event.state),
                _ => None,
            })
            .rfind(|state| {
                matches!(
                    state,
                    TriggerState::Pressed | TriggerState::Released | TriggerState::Combined
                )
            })
            == Some(TriggerState::Pressed);
        waiting.retain(|input| matches!(input, Input::Tick(_) | Input::Permission(_)));
        self.last_tap = None;
        if held && !cancelled {
            self.state = State::Pending {
                since: Instant::now(),
                second_tap: false,
                shift: false,
            };
        }
        if cancelled {
            tracing::info!("Utterance cancelled while transcribing");
            self.platform.show_status(Status::Ready);
            self.platform.show_pill(PillState::Hidden);
            return waiting;
        }
        let delivered = match result {
            Ok(text) => self.deliver(text.trim()),
            Err(error) => {
                tracing::error!(%error, "cannot finish the Utterance");
                Err(PillError::TranscriptionFailed)
            }
        };
        self.platform.show_status(Status::Ready);
        match delivered {
            Ok(shown) => match shown.or(notice) {
                Some(notice) => {
                    self.hide_at = Some(Instant::now() + self.notice_duration(notice));
                    self.platform.show_pill(PillState::Notice(notice));
                }
                None => self.hide_at = Some(Instant::now() + self.config.pill_linger),
            },
            Err(error) => self.fail(error),
        }
        waiting
    }

    fn notice_duration(&self, notice: PillNotice) -> Duration {
        match notice {
            PillNotice::QuickNoteSaved => self.config.notice_linger,
            _ => self.config.error_linger,
        }
    }

    fn deliver(&mut self, text: &str) -> Result<Option<PillNotice>, PillError> {
        if text.is_empty() {
            tracing::info!("Utterance produced no text");
            return Ok(None);
        }
        if self.quick_note {
            tracing::info!(chars = text.chars().count(), "kept a Quick note");
            self.record(text, Kind::Note);
            return Ok(Some(PillNotice::QuickNoteSaved));
        }
        if self.platform.focus_probe() == Focus::NoTextField {
            tracing::info!("no text field is focused; keeping the Final text as a Note");
            return self.keep_and_copy(text, PillNotice::NoTextFieldKeptAsNote);
        }
        match self.platform.paste(text, self.config.clipboard_restore) {
            Ok(()) => {
                tracing::info!(chars = text.chars().count(), "pasted Final text");
                self.record(text, Kind::Pasted);
                Ok(None)
            }
            Err(error) => {
                tracing::error!(%error, "cannot paste the Final text");
                self.keep_and_copy(text, PillNotice::PasteFailedKeptAsNote)
            }
        }
    }

    fn keep_and_copy(
        &mut self,
        text: &str,
        notice: PillNotice,
    ) -> Result<Option<PillNotice>, PillError> {
        self.record(text, Kind::Note);
        match self.platform.copy(text) {
            Ok(()) => Ok(Some(notice)),
            Err(error) => {
                tracing::error!(%error, "cannot copy the Note to the clipboard");
                Err(PillError::PasteFailed)
            }
        }
    }

    fn record(&mut self, text: &str, kind: Kind) {
        if let Some(history) = &mut self.history {
            history(text, kind);
        }
    }

    fn show_notice(&mut self, notice: PillNotice) {
        if !matches!(self.state, State::Idle) {
            return;
        }
        self.hide_at = Some(Instant::now() + self.config.error_linger);
        self.platform.show_pill(PillState::Notice(notice));
    }

    fn paste_again(&mut self, text: &str) {
        if !matches!(self.state, State::Idle) || text.is_empty() {
            return;
        }
        if let Err(error) = self.platform.paste(text, self.config.clipboard_restore) {
            tracing::error!(%error, "cannot paste the last transcript");
            self.fail(PillError::PasteFailed);
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
