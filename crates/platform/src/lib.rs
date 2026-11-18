pub mod fake;

use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerEvent {
    pub state: TriggerState,
    pub at: Instant,
}

impl TriggerEvent {
    pub fn pressed(at: Instant) -> Self {
        Self {
            state: TriggerState::Pressed,
            at,
        }
    }

    pub fn released(at: Instant) -> Self {
        Self {
            state: TriggerState::Released,
            at,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Ready,
    Listening,
    Transcribing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillError {
    ModelNotLoaded,
    MicrophoneUnavailable,
    TranscriptionFailed,
    PasteFailed,
}

impl PillError {
    pub fn message(self) -> &'static str {
        match self {
            Self::ModelNotLoaded => "Model not loaded",
            Self::MicrophoneUnavailable => "No microphone available",
            Self::TranscriptionFailed => "Transcription failed",
            Self::PasteFailed => "Could not paste the text",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PillState {
    Listening,
    Partial(String),
    Transcribing,
    Error(PillError),
    Hidden,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("cannot watch the Trigger key: {0}")]
    TriggerKey(String),
    #[error("cannot paste: {0}")]
    Paste(String),
}

pub type TriggerSink = Box<dyn FnMut(TriggerEvent) + Send>;

pub trait Platform {
    fn listen_trigger(&mut self, sink: TriggerSink) -> Result<(), PlatformError>;
    fn show_status(&mut self, status: Status);
    fn show_pill(&mut self, state: PillState);
    fn paste(&mut self, text: &str) -> Result<(), PlatformError>;
}
