use app::i18n::Language;
use platform::{PermissionReport, PermissionStatus, Status, TriggerKey};

fn missing_microphone() -> PermissionReport {
    PermissionReport {
        microphone: PermissionStatus::Denied,
        accessibility: PermissionStatus::Granted,
        input_monitoring: PermissionStatus::Granted,
    }
}

#[test]
fn the_ready_header_names_the_trigger_key() {
    let (title, subtitle) =
        Language::Thai.menu_header(Status::Ready, None, TriggerKey::RightOption);
    assert_eq!(title, "พร้อมฟังแล้ว");
    assert_eq!(subtitle, "กด ⌥ Right Option ค้างแล้วพูด");
    let (_, subtitle) =
        Language::English.menu_header(Status::Ready, None, TriggerKey::RightCommand);
    assert_eq!(subtitle, "Hold ⌘ Right Command and speak");
}

#[test]
fn a_missing_permission_replaces_the_ready_header() {
    let report = missing_microphone();
    let (title, subtitle) =
        Language::Thai.menu_header(Status::Ready, Some(&report), TriggerKey::RightOption);
    assert_eq!(title, "ยังไม่ได้ให้สิทธิ์");
    assert_eq!(subtitle, "ไมโครโฟน");
}

#[test]
fn listening_and_transcribing_have_their_own_headers() {
    let listening = Language::Thai.menu_header(Status::Listening, None, TriggerKey::RightOption);
    let writing = Language::Thai.menu_header(Status::Transcribing, None, TriggerKey::RightOption);
    assert_eq!(listening.0, "กำลังฟัง");
    assert_eq!(writing.0, "กำลังเขียนให้…");
}
