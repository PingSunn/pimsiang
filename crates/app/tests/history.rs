use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use app::history_view;
use audio::fake::FakeMicrophone;
use engine::fake::FakeEngine;
use engine::{Engine, LanguageMode};
use platform::fake::FakePlatform;
use platform::{PillNotice, PillState, TriggerEvent, TriggerKey};
use store::{History, HistoryService, Kind};

const HOLD: Duration = Duration::from_millis(200);

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        language: LanguageMode::ThMix,
        pill_linger: Duration::from_millis(400),
        error_linger: Duration::from_secs(3),
        notice_linger: Duration::from_millis(1500),
        double_tap_window: Duration::from_millis(300),
        sounds: true,
        clipboard_restore: true,
        trigger_key: TriggerKey::RightOption,
    }
}

type Recorded = Arc<Mutex<Vec<(String, Kind)>>>;
type TestController = Controller<FakePlatform, FakeEngine, FakeMicrophone>;

fn controller_on(platform: FakePlatform, final_text: &str) -> (TestController, Recorded) {
    let mut engine = FakeEngine::new(["x"], final_text);
    engine.load().unwrap();
    let recorded = Recorded::default();
    let sink = recorded.clone();
    let controller = Controller::new(platform, engine, FakeMicrophone::new(), config())
        .record_history(move |text, kind| sink.lock().unwrap().push((text.to_owned(), kind)));
    (controller, recorded)
}

fn controller(final_text: &str) -> (TestController, Recorded) {
    controller_on(FakePlatform::new(), final_text)
}

fn utterance(c: &mut TestController, t0: Instant) {
    c.handle(Input::Trigger(TriggerEvent::pressed(t0)));
    c.handle(Input::Tick(t0 + HOLD));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    c.handle(Input::Trigger(TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )));
}

#[test]
fn a_pasted_final_text_is_recorded_as_pasted() {
    let (mut c, recorded) = controller("สวัสดีครับ");

    utterance(&mut c, Instant::now());

    assert_eq!(c.platform().pastes(), ["สวัสดีครับ"]);
    assert_eq!(
        *recorded.lock().unwrap(),
        [("สวัสดีครับ".to_owned(), Kind::Pasted)]
    );
}

#[test]
fn every_utterance_is_recorded_once_in_order() {
    let (mut c, recorded) = controller("หนึ่ง");
    let t0 = Instant::now();

    utterance(&mut c, t0);
    utterance(&mut c, t0 + Duration::from_secs(5));

    assert_eq!(recorded.lock().unwrap().len(), 2);
}

#[test]
fn empty_final_text_records_nothing() {
    let (mut c, recorded) = controller("   ");

    utterance(&mut c, Instant::now());

    assert!(recorded.lock().unwrap().is_empty());
}

#[test]
fn a_cancelled_utterance_records_nothing() {
    let (mut c, recorded) = controller("ไม่ควรเก็บ");
    let t0 = Instant::now();
    c.handle(Input::Trigger(TriggerEvent::pressed(t0)));
    c.handle(Input::Tick(t0 + HOLD));
    c.handle(Input::Trigger(TriggerEvent::escape(
        t0 + Duration::from_millis(500),
    )));
    c.handle(Input::Trigger(TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )));

    assert!(recorded.lock().unwrap().is_empty());
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn a_failed_paste_is_kept_as_a_note_so_the_text_is_not_lost() {
    let (mut c, recorded) = controller_on(FakePlatform::new().failing_paste(), "เก็บไว้");

    utterance(&mut c, Instant::now());

    assert_eq!(
        *recorded.lock().unwrap(),
        [("เก็บไว้".to_owned(), Kind::Note)]
    );
}

#[test]
fn paste_last_pastes_the_given_text_and_records_nothing() {
    let (mut c, recorded) = controller("x");

    c.handle(Input::PasteText("ข้อความล่าสุด".to_owned()));

    assert_eq!(c.platform().pastes(), ["ข้อความล่าสุด"]);
    assert!(recorded.lock().unwrap().is_empty());
}

#[test]
fn utterances_reach_a_real_store_and_can_be_searched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let service = HistoryService::spawn(&path).unwrap();
    let mut engine = FakeEngine::new(["x"], "วันนี้อากาศดีมาก");
    engine.load().unwrap();
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config())
        .record_history(move |text, kind| {
            service.add(text.to_owned(), kind, Some("int8".to_owned()), |_| {});
        });

    utterance(&mut c, Instant::now());
    drop(c);

    let found = History::open(&path)
        .unwrap()
        .search("อากาศ", 0, 10)
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, Kind::Pasted);
    assert_eq!(found[0].model_id.as_deref(), Some("int8"));
}

#[test]
fn entry_times_are_shown_in_the_given_zone_to_the_minute() {
    use app::history_view::format_created_at;
    let bangkok = chrono::FixedOffset::east_opt(7 * 3600).unwrap();

    assert_eq!(
        format_created_at(1_700_000_000, bangkok),
        "2023-11-15 05:13"
    );
}

#[test]
fn every_history_string_has_a_thai_and_an_english_version() {
    use app::i18n::Language;
    let has_thai = |text: &str| text.chars().any(|c| ('\u{0E00}'..='\u{0E7F}').contains(&c));
    for language in [Language::English, Language::Thai] {
        let strings = [
            language.menu_history(),
            language.menu_paste_last(),
            language.history_title(),
            language.history_search_hint(),
            language.history_empty(),
            language.history_no_match(),
            language.history_kind(Kind::Pasted),
            language.history_kind(Kind::Note),
            language.history_copy(),
            language.history_copied(),
            language.history_delete(),
            language.history_clear_all(),
            language.history_confirm_clear(),
            language.history_confirm_yes(),
            language.history_cancel(),
            language.history_show_more(),
            language.history_unavailable(),
        ];
        for text in strings {
            assert!(!text.is_empty());
            assert_eq!(has_thai(text), language == Language::Thai, "{text}");
        }
    }
}

#[test]
fn paste_last_with_an_empty_history_shows_a_notice_and_pastes_nothing() {
    let (mut c, _recorded) = controller("x");
    let now = Instant::now();

    c.handle(Input::NoTranscript);

    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::NoTranscriptYet))
    );
    assert!(c.platform().pastes().is_empty());

    c.handle(Input::Tick(now + Duration::from_secs(4)));

    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
}

#[test]
fn the_no_transcript_notice_is_bilingual() {
    let message = PillNotice::NoTranscriptYet.message();

    assert!(message.contains("No transcript yet"));
    assert!(message.contains("ยังไม่มี"));
}

#[test]
fn the_recorder_stores_the_text_then_tells_the_window_to_reload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let service = Arc::new(HistoryService::spawn(&path).unwrap());
    let (changed, wait) = std::sync::mpsc::channel();
    let reader = path.clone();
    let mut record = history_view::recorder(service, Some("int8".to_owned()), move || {
        let found = History::open(&reader)
            .unwrap()
            .search("บันทึก", 0, 10)
            .unwrap();
        let _ = changed.send(found.len());
    });

    record("ข้อความที่บันทึก", Kind::Pasted);

    assert_eq!(wait.recv_timeout(Duration::from_secs(5)).unwrap(), 1);
}
