use app::preferences::Preferences;
use engine::LanguageMode;
use platform::TriggerKey;
use store::Settings;

#[test]
fn an_empty_settings_file_gives_the_defaults() {
    let preferences = Preferences::from_settings(&Settings::default());

    assert_eq!(preferences.trigger_key, TriggerKey::RightOption);
    assert_eq!(preferences.language, LanguageMode::ThMix);
    assert!(preferences.sounds);
    assert!(preferences.clipboard_restore);
    assert!(preferences.launch_at_login);
    assert!(preferences.unload_when_idle);
}

#[test]
fn stored_values_round_trip_through_the_settings() {
    let preferences = Preferences {
        trigger_key: TriggerKey::RightControl,
        language: LanguageMode::ThTh,
        sounds: false,
        clipboard_restore: false,
        launch_at_login: false,
        unload_when_idle: false,
    };
    let mut settings = Settings::default();

    preferences.apply(&mut settings);

    assert_eq!(settings.trigger_key.as_deref(), Some("right-control"));
    assert_eq!(settings.language.as_deref(), Some("th-TH"));
    assert_eq!(Preferences::from_settings(&settings), preferences);
}

#[test]
fn an_unknown_trigger_key_or_language_falls_back_to_the_default() {
    let settings = Settings {
        trigger_key: Some("fn".to_owned()),
        language: Some("en-US".to_owned()),
        ..Settings::default()
    };

    let preferences = Preferences::from_settings(&settings);

    assert_eq!(preferences.trigger_key, TriggerKey::RightOption);
    assert_eq!(preferences.language, LanguageMode::ThMix);
}

#[test]
fn applying_preferences_keeps_the_microphone_and_update_state() {
    let mut settings = Settings {
        microphone: Some("usb".to_owned()),
        dismissed_update: Some("0.2.0".to_owned()),
        ..Settings::default()
    };

    Preferences::default().apply(&mut settings);

    assert_eq!(settings.microphone.as_deref(), Some("usb"));
    assert_eq!(settings.dismissed_update.as_deref(), Some("0.2.0"));
}

#[test]
fn a_language_override_is_not_written_to_the_settings() {
    let mut settings = Settings {
        language: Some("th-TH".to_owned()),
        ..Settings::default()
    };
    let preferences = Preferences {
        language: LanguageMode::ThMix,
        sounds: false,
        ..Preferences::default()
    };

    preferences.apply_keeping_language(&mut settings, true);

    assert_eq!(settings.language.as_deref(), Some("th-TH"));
    assert_eq!(settings.sounds, Some(false));
}
