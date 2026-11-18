use engine::LanguageMode;
use platform::TriggerKey;
use store::Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub trigger_key: TriggerKey,
    pub language: LanguageMode,
    pub sounds: bool,
    pub clipboard_restore: bool,
    pub launch_at_login: bool,
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
            sounds: settings.sounds(),
            clipboard_restore: settings.clipboard_restore(),
            launch_at_login: settings.launch_at_login(),
        }
    }

    pub fn apply(&self, settings: &mut Settings) {
        settings.trigger_key = Some(self.trigger_key.name().to_owned());
        settings.language = Some(self.language.name().to_owned());
        settings.sounds = Some(self.sounds);
        settings.clipboard_restore = Some(self.clipboard_restore);
        settings.launch_at_login = Some(self.launch_at_login);
    }
}
