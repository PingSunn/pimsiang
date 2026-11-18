pub mod clipboard;
pub mod fake;

use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerState {
    Pressed,
    Released,
    Combined,
    Escape,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TriggerKey {
    #[default]
    RightOption,
    RightCommand,
    RightControl,
}

impl TriggerKey {
    pub const ALL: [TriggerKey; 3] = [
        TriggerKey::RightOption,
        TriggerKey::RightCommand,
        TriggerKey::RightControl,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::RightOption => "right-option",
            Self::RightCommand => "right-command",
            Self::RightControl => "right-control",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.name() == name)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RightOption => "Right Option",
            Self::RightCommand => "Right Command",
            Self::RightControl => "Right Control",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Start,
    Stop,
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

    pub fn combined(at: Instant) -> Self {
        Self {
            state: TriggerState::Combined,
            at,
        }
    }

    pub fn escape(at: Instant) -> Self {
        Self {
            state: TriggerState::Escape,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    Microphone,
    Accessibility,
    InputMonitoring,
}

impl Permission {
    pub const ALL: [Permission; 3] = [
        Permission::Microphone,
        Permission::Accessibility,
        Permission::InputMonitoring,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Microphone => "Microphone",
            Self::Accessibility => "Accessibility",
            Self::InputMonitoring => "Input Monitoring",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermissionStatus {
    Granted,
    NotDetermined,
    Denied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PermissionReport {
    pub microphone: PermissionStatus,
    pub accessibility: PermissionStatus,
    pub input_monitoring: PermissionStatus,
}

impl PermissionReport {
    pub fn read(platform: &(impl Platform + ?Sized)) -> Self {
        Self {
            microphone: platform.permission_status(Permission::Microphone),
            accessibility: platform.permission_status(Permission::Accessibility),
            input_monitoring: platform.permission_status(Permission::InputMonitoring),
        }
    }

    pub fn status(&self, permission: Permission) -> PermissionStatus {
        match permission {
            Permission::Microphone => self.microphone,
            Permission::Accessibility => self.accessibility,
            Permission::InputMonitoring => self.input_monitoring,
        }
    }

    pub fn missing(&self) -> Vec<Permission> {
        Permission::ALL
            .into_iter()
            .filter(|permission| self.status(*permission) != PermissionStatus::Granted)
            .collect()
    }

    pub fn all_granted(&self) -> bool {
        self.missing().is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillError {
    PermissionMissing(Permission),
    ModelNotLoaded,
    ModelNotReady,
    MicrophoneUnavailable,
    TranscriptionFailed,
    PasteFailed,
}

impl PillError {
    pub fn message(self) -> &'static str {
        match self {
            Self::PermissionMissing(Permission::Microphone) => "Microphone permission is missing",
            Self::PermissionMissing(Permission::Accessibility) => {
                "Accessibility permission is missing"
            }
            Self::PermissionMissing(Permission::InputMonitoring) => {
                "Input Monitoring permission is missing"
            }
            Self::ModelNotLoaded => "Model not loaded",
            Self::ModelNotReady => "Model not ready",
            Self::MicrophoneUnavailable => "No microphone available",
            Self::TranscriptionFailed => "Transcription failed",
            Self::PasteFailed => "Could not paste the text",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillNotice {
    MicrophoneFellBack,
    MicrophoneLost,
    NoTranscriptYet,
}

impl PillNotice {
    pub fn message(self) -> &'static str {
        match self {
            Self::MicrophoneFellBack => "Microphone not found, using the system default",
            Self::MicrophoneLost => "Microphone disconnected",
            Self::NoTranscriptYet => "No transcript yet · ยังไม่มีข้อความ",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MicrophoneEntry {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MicrophoneMenu {
    pub devices: Vec<MicrophoneEntry>,
    pub selected: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PillState {
    Listening,
    Partial(String),
    Transcribing,
    Notice(PillNotice),
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
    fn stop_trigger(&mut self);
    fn show_status(&mut self, status: Status);
    fn show_pill(&mut self, state: PillState);
    fn play_sound(&mut self, sound: Sound);
    fn set_trigger_key(&mut self, key: TriggerKey);
    fn paste(&mut self, text: &str, restore_clipboard: bool) -> Result<(), PlatformError>;
    fn permission_status(&self, permission: Permission) -> PermissionStatus;
    fn request_permission(&mut self, permission: Permission);
    fn open_settings(&mut self, permission: Permission);
    fn show_permissions(&mut self, report: PermissionReport);
    fn show_microphones(&mut self, menu: MicrophoneMenu);
}
