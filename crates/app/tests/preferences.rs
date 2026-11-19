use app::preferences::Preferences;
use engine::{LanguageMode, RecognitionMode};
use platform::{PillPlacement, PillPosition, ShowInDock, TriggerKey};
use store::{Precision, Settings};

#[test]
fn an_empty_settings_file_gives_the_defaults() {
    let preferences = Preferences::from_settings(&Settings::default());

    assert_eq!(preferences.trigger_key, TriggerKey::RightOption);
    assert_eq!(preferences.language, LanguageMode::ThMix);
    assert!(preferences.sounds);
    assert!(preferences.clipboard_restore);
    assert!(preferences.launch_at_login);
    assert!(preferences.unload_when_idle);
    assert_eq!(preferences.pill_placement, PillPlacement::Floating);
    assert_eq!(preferences.pill_position, PillPosition { x: 0.5, y: 0.12 });
    assert_eq!(preferences.precision, Precision::Light);
    assert_eq!(preferences.show_in_dock, ShowInDock::WhileWindowOpen);
}

#[test]
fn stored_values_round_trip_through_the_settings() {
    let preferences = Preferences {
        trigger_key: TriggerKey::RightControl,
        language: LanguageMode::ThTh,
        recognition_mode: RecognitionMode::Conversation,
        sounds: false,
        clipboard_restore: false,
        launch_at_login: false,
        unload_when_idle: false,
        pill_placement: PillPlacement::Notch,
        pill_position: PillPosition { x: 0.25, y: 0.75 },
        precision: Precision::Light,
        show_in_dock: ShowInDock::Never,
    };
    let mut settings = Settings::default();

    preferences.apply_keeping_overrides(&mut settings, false, false);

    assert_eq!(settings.trigger_key.as_deref(), Some("right-control"));
    assert_eq!(settings.language.as_deref(), Some("th-TH"));
    assert_eq!(settings.recognition_mode.as_deref(), Some("conversation"));
    assert_eq!(settings.pill_placement.as_deref(), Some("notch"));
    assert_eq!(settings.pill_position.as_deref(), Some("0.2500,0.7500"));
    assert_eq!(settings.precision.as_deref(), Some("light"));
    assert_eq!(settings.show_in_dock.as_deref(), Some("never"));
    assert_eq!(Preferences::from_settings(&settings), preferences);
}

#[test]
fn an_unknown_trigger_key_or_language_falls_back_to_the_default() {
    let settings = Settings {
        trigger_key: Some("fn".to_owned()),
        language: Some("en-US".to_owned()),
        recognition_mode: Some("fast".to_owned()),
        ..Settings::default()
    };

    let preferences = Preferences::from_settings(&settings);

    assert_eq!(preferences.trigger_key, TriggerKey::RightOption);
    assert_eq!(preferences.language, LanguageMode::ThMix);
    assert_eq!(preferences.recognition_mode, RecognitionMode::Standard);
}

#[test]
fn applying_preferences_keeps_the_microphone_and_update_state() {
    let mut settings = Settings {
        microphone: Some("usb".to_owned()),
        dismissed_update: Some("0.2.0".to_owned()),
        ..Settings::default()
    };

    Preferences::default().apply_keeping_overrides(&mut settings, false, false);

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

    preferences.apply_keeping_overrides(&mut settings, true, false);

    assert_eq!(settings.language.as_deref(), Some("th-TH"));
    assert_eq!(settings.sounds, Some(false));
}

#[test]
fn a_bad_pill_placement_or_position_falls_back_to_the_default() {
    for position in [
        "",
        "0.5",
        "left,bottom",
        "1.5,0.2",
        "NaN,0.1",
        "0.5,0.1,0.2",
    ] {
        let settings = Settings {
            pill_placement: Some("corner".to_owned()),
            pill_position: Some(position.to_owned()),
            ..Settings::default()
        };

        let preferences = Preferences::from_settings(&settings);

        assert_eq!(preferences.pill_placement, PillPlacement::Floating);
        assert_eq!(preferences.pill_position, PillPosition::default());
    }
}

#[test]
fn a_saved_pill_placement_and_position_are_read_back() {
    let settings = Settings {
        pill_placement: Some("notch".to_owned()),
        pill_position: Some("0.8,0.9".to_owned()),
        ..Settings::default()
    };

    let preferences = Preferences::from_settings(&settings);

    assert_eq!(preferences.pill_placement, PillPlacement::Notch);
    assert_eq!(preferences.pill_position, PillPosition { x: 0.8, y: 0.9 });
}

#[test]
fn a_recognition_mode_override_is_not_written_to_the_settings() {
    let mut settings = Settings {
        recognition_mode: Some("conversation".to_owned()),
        ..Settings::default()
    };
    let preferences = Preferences {
        recognition_mode: RecognitionMode::Standard,
        sounds: false,
        ..Preferences::default()
    };

    preferences.apply_keeping_overrides(&mut settings, false, true);

    assert_eq!(settings.recognition_mode.as_deref(), Some("conversation"));
    assert_eq!(settings.sounds, Some(false));
}

#[test]
fn an_unknown_precision_gives_light() {
    let settings = Settings {
        precision: Some("int8".to_owned()),
        ..Settings::default()
    };

    assert_eq!(
        Preferences::from_settings(&settings).precision,
        Precision::Light
    );
}

#[test]
fn a_missing_or_unknown_show_in_dock_falls_back_to_while_a_window_is_open() {
    for stored in [None, Some(""), Some("sometimes"), Some("NEVER")] {
        let settings = Settings {
            show_in_dock: stored.map(str::to_owned),
            ..Settings::default()
        };

        let preferences = Preferences::from_settings(&settings);

        assert_eq!(preferences.show_in_dock, ShowInDock::WhileWindowOpen);
    }
}

#[test]
fn a_saved_show_in_dock_is_read_back() {
    for show in ShowInDock::ALL {
        let settings = Settings {
            show_in_dock: Some(show.name().to_owned()),
            ..Settings::default()
        };

        assert_eq!(Preferences::from_settings(&settings).show_in_dock, show);
    }
}
