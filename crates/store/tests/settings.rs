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
