pub mod clipboard;
pub mod fake;
pub mod focus;
pub mod pill_layout;

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
    pub shift: bool,
}

impl TriggerEvent {
    pub fn pressed(at: Instant) -> Self {
        Self {
            state: TriggerState::Pressed,
            at,
            shift: false,
        }
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn released(at: Instant) -> Self {
        Self {
            state: TriggerState::Released,
            at,
            shift: false,
        }
    }

    pub fn combined(at: Instant) -> Self {
        Self {
            state: TriggerState::Combined,
            at,
            shift: false,
        }
    }

    pub fn escape(at: Instant) -> Self {
        Self {
            state: TriggerState::Escape,
            at,
            shift: false,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillNotice {
    MicrophoneFellBack,
    MicrophoneLost,
    NoTranscriptYet,
    QuickNoteSaved,
    NoTextFieldKeptAsNote,
    PasteFailedKeptAsNote,
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
pub struct PartialText {
    pub text: String,
    pub settled: usize,
}

impl PartialText {
    pub fn new(text: impl Into<String>, settled: usize) -> Self {
        Self {
            text: text.into(),
            settled,
        }
    }

    pub fn split(&self) -> (&str, &str) {
        let mut at = self.settled.min(self.text.len());
        while !self.text.is_char_boundary(at) {
            at -= 1;
        }
        while at > 0 && self.text[at..].chars().next().is_some_and(joins_previous) {
            at = self.text[..at]
                .char_indices()
                .next_back()
                .map_or(0, |(index, _)| index);
        }
        self.text.split_at(at)
    }
}

fn joins_previous(character: char) -> bool {
    matches!(
        character,
        '\u{0300}'..='\u{036F}'
            | '\u{0E31}'
            | '\u{0E33}'..='\u{0E3A}'
            | '\u{0E47}'..='\u{0E4E}'
            | '\u{200D}'
            | '\u{FE00}'..='\u{FE0F}'
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PillState {
    Listening,
    LoadingModel,
    Partial(PartialText),
    Transcribing,
    Notice(PillNotice),
    Error(PillError),
    Hidden,
}

impl PillState {
    pub fn partial(text: &str) -> Self {
        Self::Partial(PartialText::new(text, 0))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PillContent {
    pub settled: String,
    pub tentative: String,
    pub caret: bool,
    pub dots: bool,
    pub badge: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillAction {
    OpenSettings(Permission),
}

pub fn pill_click(state: &PillState) -> Option<PillAction> {
    match state {
        PillState::Error(PillError::PermissionMissing(permission)) => {
            Some(PillAction::OpenSettings(*permission))
        }
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("cannot watch the Trigger key: {0}")]
    TriggerKey(String),
    #[error("cannot paste: {0}")]
    Paste(String),
    #[error("cannot copy: {0}")]
    Copy(String),
}

pub use focus::Focus;
pub use pill_layout::{PillPlacement, PillPosition};

pub type TriggerSink = Box<dyn FnMut(TriggerEvent) + Send>;

pub trait Platform {
    fn listen_trigger(&mut self, sink: TriggerSink) -> Result<(), PlatformError>;
    fn stop_trigger(&mut self);
    fn show_status(&mut self, status: Status);
    fn show_pill(&mut self, state: PillState);
    fn play_sound(&mut self, sound: Sound);
    fn set_trigger_key(&mut self, key: TriggerKey);
    fn paste(&mut self, text: &str, restore_clipboard: bool) -> Result<(), PlatformError>;
    fn copy(&mut self, text: &str) -> Result<(), PlatformError>;
    fn focus_probe(&mut self) -> Focus;
    fn permission_status(&self, permission: Permission) -> PermissionStatus;
    fn request_permission(&mut self, permission: Permission);
    fn open_settings(&mut self, permission: Permission);
    fn show_permissions(&mut self, report: PermissionReport);
    fn show_microphones(&mut self, menu: MicrophoneMenu);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_splits_into_settled_and_uncertain_text() {
        let partial = PartialText::new("ช่วยส่งรายงาน สามโมง", "ช่วยส่งรายงาน ".len());
        assert_eq!(partial.split(), ("ช่วยส่งรายงาน ", "สามโมง"));
    }

    #[test]
    fn nothing_settled_means_the_whole_text_is_uncertain() {
        assert_eq!(PartialText::new("สวัส", 0).split(), ("", "สวัส"));
    }

    #[test]
    fn everything_settled_leaves_no_uncertain_tail() {
        let text = "สวัสดี";
        assert_eq!(PartialText::new(text, text.len()).split(), (text, ""));
    }

    #[test]
    fn a_settled_length_past_the_end_is_clamped() {
        assert_eq!(PartialText::new("abc", 99).split(), ("abc", ""));
    }

    #[test]
    fn a_settled_length_inside_a_character_moves_back_to_a_boundary() {
        assert_eq!(PartialText::new("สว", 4).split(), ("ส", "ว"));
    }

    #[test]
    fn a_settled_length_before_a_thai_mark_keeps_the_mark_with_its_consonant() {
        let text = "สวัสดีครับ";
        let before_vowel = "สวัสด".len();
        assert_eq!(
            PartialText::new(text, before_vowel).split(),
            ("สวัส", "ดีครับ")
        );
        let tone = "ที่";
        assert_eq!(PartialText::new(tone, "ที".len()).split(), ("", "ที่"));
        assert_eq!(PartialText::new("น้ำ", "น้".len()).split(), ("", "น้ำ"));
    }

    #[test]
    fn a_settled_length_before_a_latin_combining_accent_keeps_the_accent() {
        let text = "cafe\u{0301} ok";
        assert_eq!(PartialText::new(text, 4).split(), ("caf", "e\u{0301} ok"));
    }

    #[test]
    fn only_a_missing_permission_makes_the_pill_clickable() {
        for permission in Permission::ALL {
            assert_eq!(
                pill_click(&PillState::Error(PillError::PermissionMissing(permission))),
                Some(PillAction::OpenSettings(permission))
            );
        }
        for state in [
            PillState::Listening,
            PillState::LoadingModel,
            PillState::partial("a"),
            PillState::Transcribing,
            PillState::Notice(PillNotice::QuickNoteSaved),
            PillState::Error(PillError::MicrophoneUnavailable),
            PillState::Error(PillError::PasteFailed),
            PillState::Hidden,
        ] {
            assert_eq!(pill_click(&state), None);
        }
    }
}
