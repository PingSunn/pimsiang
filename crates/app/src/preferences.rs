use engine::{LanguageMode, RecognitionMode};
use platform::{PillPlacement, PillPosition, TriggerKey};
use store::{Precision, Settings};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preferences {
    pub trigger_key: TriggerKey,
    pub language: LanguageMode,
    pub recognition_mode: RecognitionMode,
    pub sounds: bool,
    pub clipboard_restore: bool,
    pub launch_at_login: bool,
    pub unload_when_idle: bool,
    pub pill_placement: PillPlacement,
    pub pill_position: PillPosition,
    pub precision: Precision,
}

impl Default for Preferences {
    fn default() -> Self {
        Self::from_settings(&Settings::default())
    }
}

impl Preferences {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            trigger_key: settings
                .trigger_key
                .as_deref()
                .and_then(TriggerKey::from_name)
                .unwrap_or_default(),
            language: settings
                .language
                .as_deref()
                .and_then(|name| name.parse().ok())
                .unwrap_or_default(),
            recognition_mode: settings
                .recognition_mode
                .as_deref()
                .and_then(|name| name.parse().ok())
                .unwrap_or_default(),
            sounds: settings.sounds(),
            clipboard_restore: settings.clipboard_restore(),
            launch_at_login: settings.launch_at_login(),
            unload_when_idle: settings.unload_when_idle(),
            pill_placement: settings
                .pill_placement
                .as_deref()
                .and_then(PillPlacement::from_name)
                .unwrap_or_default(),
            pill_position: settings
                .pill_position
                .as_deref()
                .and_then(PillPosition::parse)
                .unwrap_or_default(),
            precision: settings.precision(),
        }
    }

    pub fn apply_keeping_overrides(
        &self,
        settings: &mut Settings,
        keep_language: bool,
        keep_recognition_mode: bool,
    ) {
        settings.trigger_key = Some(self.trigger_key.name().to_owned());
        if !keep_language {
            settings.language = Some(self.language.name().to_owned());
        }
        if !keep_recognition_mode {
            settings.recognition_mode = Some(self.recognition_mode.name().to_owned());
        }
        settings.sounds = Some(self.sounds);
        settings.clipboard_restore = Some(self.clipboard_restore);
        settings.launch_at_login = Some(self.launch_at_login);
        settings.unload_when_idle = Some(self.unload_when_idle);
        settings.pill_placement = Some(self.pill_placement.name().to_owned());
        settings.pill_position = Some(self.pill_position.text());
        settings.precision = Some(self.precision.name().to_owned());
    }
}
