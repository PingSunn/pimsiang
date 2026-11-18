use store::Settings;

#[test]
fn a_missing_file_gives_the_defaults() {
    let dir = tempfile::tempdir().unwrap();

    let settings = Settings::load(&dir.path().join("settings"));

    assert_eq!(settings, Settings::default());
    assert_eq!(settings.microphone, None);
}

#[test]
fn the_microphone_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/settings");
    let settings = Settings {
        microphone: Some("coreaudio:BuiltInMicrophoneDevice".to_owned()),
        ..Settings::default()
    };

    settings.save(&path).unwrap();

    assert_eq!(Settings::load(&path), settings);
}

#[test]
fn choosing_the_system_default_clears_the_stored_microphone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    Settings {
        microphone: Some("usb".to_owned()),
        ..Settings::default()
    }
    .save(&path)
    .unwrap();

    Settings::default().save(&path).unwrap();

    assert_eq!(Settings::load(&path).microphone, None);
}

#[test]
fn a_corrupt_file_gives_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    std::fs::write(&path, [0xff, 0xfe, 0x00, b'=']).unwrap();

    assert_eq!(Settings::load(&path), Settings::default());
}

#[test]
fn a_windows_line_ending_does_not_end_up_in_the_microphone_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    std::fs::write(&path, "microphone=usb\r\n").unwrap();
    assert_eq!(Settings::load(&path).microphone.as_deref(), Some("usb"));

    std::fs::write(&path, "microphone=usb\r").unwrap();
    assert_eq!(Settings::load(&path).microphone.as_deref(), Some("usb"));
}

#[test]
fn saving_keeps_keys_it_does_not_know() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    std::fs::write(&path, "theme=dark\r\nmicrophone=old\nhotkey=right-option\n").unwrap();

    Settings {
        microphone: Some("usb".to_owned()),
        ..Settings::default()
    }
    .save(&path)
    .unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("theme=dark\n"));
    assert!(text.contains("hotkey=right-option\n"));
    assert!(!text.contains("microphone=old"));
    assert_eq!(Settings::load(&path).microphone.as_deref(), Some("usb"));

    Settings::default().save(&path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("theme=dark\n"));
    assert!(!text.contains("microphone="));
}

#[test]
fn the_update_state_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    let settings = Settings {
        microphone: Some("usb".to_owned()),
        last_update_check: Some(1_700_000_000),
        available_update: Some("0.2.0".to_owned()),
        dismissed_update: Some("0.1.5".to_owned()),
        ..Settings::default()
    };

    settings.save(&path).unwrap();

    assert_eq!(Settings::load(&path), settings);
}

#[test]
fn an_unreadable_check_time_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    std::fs::write(&path, "last_update_check=yesterday\nmicrophone=usb\n").unwrap();

    let settings = Settings::load(&path);

    assert_eq!(settings.last_update_check, None);
    assert_eq!(settings.microphone.as_deref(), Some("usb"));
}

#[test]
fn two_threads_updating_different_keys_both_survive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    let rounds = 50;

    std::thread::scope(|scope| {
        scope.spawn(|| {
            for round in 0..rounds {
                Settings::update(&path, |settings| {
                    settings.microphone = Some(format!("mic-{round}"));
                })
                .unwrap();
            }
        });
        scope.spawn(|| {
            for round in 0..rounds {
                Settings::update(&path, |settings| {
                    settings.last_update_check = Some(round);
                })
                .unwrap();
            }
        });
    });

    let settings = Settings::load(&path);
    assert_eq!(settings.microphone, Some(format!("mic-{}", rounds - 1)));
    assert_eq!(settings.last_update_check, Some(rounds - 1));
}

#[test]
fn update_changes_one_key_and_keeps_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    Settings {
        microphone: Some("usb".to_owned()),
        ..Settings::default()
    }
    .save(&path)
    .unwrap();

    let saved = Settings::update(&path, |settings| {
        settings.dismissed_update = Some("0.2.0".to_owned());
    })
    .unwrap();

    assert_eq!(saved, Settings::load(&path));
    assert_eq!(saved.microphone.as_deref(), Some("usb"));
    assert_eq!(saved.dismissed_update.as_deref(), Some("0.2.0"));
}

#[test]
fn no_temporary_file_is_left_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");

    Settings::update(&path, |settings| {
        settings.microphone = Some("usb".to_owned())
    })
    .unwrap();

    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("settings")]);
}

#[test]
fn a_missing_file_gives_the_documented_defaults() {
    let dir = tempfile::tempdir().unwrap();

    let settings = Settings::load(&dir.path().join("settings"));

    assert_eq!(settings.trigger_key, None);
    assert_eq!(settings.language, None);
    assert!(settings.sounds());
    assert!(settings.clipboard_restore());
    assert!(settings.launch_at_login());
}

#[test]
fn the_preferences_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    let settings = Settings {
        trigger_key: Some("right-command".to_owned()),
        language: Some("th-TH".to_owned()),
        sounds: Some(false),
        clipboard_restore: Some(false),
        launch_at_login: Some(false),
        ..Settings::default()
    };

    settings.save(&path).unwrap();

    let loaded = Settings::load(&path);
    assert_eq!(loaded, settings);
    assert!(!loaded.sounds());
    assert!(!loaded.clipboard_restore());
    assert!(!loaded.launch_at_login());
}

#[test]
fn a_switch_turned_back_on_is_stored_as_on() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    Settings::update(&path, |settings| settings.sounds = Some(false)).unwrap();

    Settings::update(&path, |settings| settings.sounds = Some(true)).unwrap();

    assert_eq!(Settings::load(&path).sounds, Some(true));
}

#[test]
fn an_unreadable_switch_falls_back_to_its_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    std::fs::write(&path, "sounds=maybe\nclipboard_restore=\nmicrophone=usb\n").unwrap();

    let settings = Settings::load(&path);

    assert_eq!(settings.sounds, None);
    assert!(settings.sounds());
    assert!(settings.clipboard_restore());
    assert_eq!(settings.microphone.as_deref(), Some("usb"));
}

#[test]
fn changing_a_preference_keeps_the_microphone_and_the_update_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    Settings {
        microphone: Some("usb".to_owned()),
        dismissed_update: Some("0.2.0".to_owned()),
        ..Settings::default()
    }
    .save(&path)
    .unwrap();

    let saved = Settings::update(&path, |settings| {
        settings.language = Some("th-TH".to_owned());
    })
    .unwrap();

    assert_eq!(saved.microphone.as_deref(), Some("usb"));
    assert_eq!(saved.dismissed_update.as_deref(), Some("0.2.0"));
    assert_eq!(Settings::load(&path).language.as_deref(), Some("th-TH"));
}

#[test]
fn saving_the_launch_at_login_choice_does_not_mark_the_login_item_decided() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");

    Settings::update(&path, |settings| settings.launch_at_login = Some(true)).unwrap();

    assert!(!Settings::load(&path).login_item_decided());

    Settings::update(&path, |settings| settings.login_item_decided = Some(true)).unwrap();

    let loaded = Settings::load(&path);
    assert!(loaded.login_item_decided());
    assert_eq!(loaded.launch_at_login, Some(true));
}

#[test]
fn unload_when_idle_defaults_to_on_and_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    assert!(Settings::load(&path).unload_when_idle());

    Settings::update(&path, |settings| settings.unload_when_idle = Some(false)).unwrap();

    assert!(!Settings::load(&path).unload_when_idle());
}
