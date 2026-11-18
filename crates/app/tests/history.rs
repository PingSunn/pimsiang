use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use app::history_view;
use audio::fake::FakeMicrophone;
use engine::fake::FakeEngine;
use engine::{Engine, LanguageMode};
use platform::fake::FakePlatform;
use platform::{Permission, PillError, PillNotice, PillState, Status, TriggerEvent, TriggerKey};
use store::{History, HistoryService, Kind};

const IDLE: Duration = Duration::from_secs(600);
const HOLD: Duration = Duration::from_millis(200);

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        language: LanguageMode::ThMix,
        recognition_mode: engine::RecognitionMode::Standard,
        pill_linger: Duration::from_millis(400),
        error_linger: Duration::from_secs(3),
        notice_linger: Duration::from_millis(1500),
        double_tap_window: Duration::from_millis(300),
        sounds: true,
        clipboard_restore: true,
        trigger_key: TriggerKey::RightOption,
        unload_when_idle: true,
        idle_after: IDLE,
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
            language.menu_open_logs(),
            language.menu_quit(),
            language.tooltip(Status::Ready, false),
            language.tooltip(Status::Listening, false),
            language.tooltip(Status::Transcribing, false),
            language.tooltip(Status::Ready, true),
            language.pill_listening(),
            language.pill_loading_model(),
            language.pill_transcribing(),
            language.pill_notice(PillNotice::MicrophoneFellBack),
            language.pill_notice(PillNotice::MicrophoneLost),
            language.pill_notice(PillNotice::NoTranscriptYet),
            language.pill_notice(PillNotice::QuickNoteSaved),
            language.pill_notice(PillNotice::NoTextFieldKeptAsNote),
            language.pill_notice(PillNotice::PasteFailedKeptAsNote),
        ];
        for text in strings {
            assert!(!text.is_empty());
            assert_eq!(has_thai(text), language == Language::Thai, "{text}");
        }
        let errors = [
            PillError::PermissionMissing(Permission::Microphone),
            PillError::PermissionMissing(Permission::Accessibility),
            PillError::PermissionMissing(Permission::InputMonitoring),
            PillError::ModelNotLoaded,
            PillError::ModelNotReady,
            PillError::MicrophoneUnavailable,
            PillError::TranscriptionFailed,
            PillError::PasteFailed,
        ];
        for error in errors {
            let text = language.pill_error(error);
            assert!(!text.is_empty());
            assert_eq!(has_thai(&text), language == Language::Thai, "{text}");
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
fn the_no_transcript_notice_follows_the_system_language() {
    use app::i18n::Language;

    assert_eq!(
        Language::English.pill_notice(PillNotice::NoTranscriptYet),
        "No transcript yet"
    );
    assert_eq!(
        Language::Thai.pill_notice(PillNotice::NoTranscriptYet),
        "ยังไม่มีข้อความ"
    );
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

fn bangkok() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(7 * 3600).unwrap()
}

fn at(day: u32, hour: u32, minute: u32) -> i64 {
    use chrono::TimeZone;
    bangkok()
        .with_ymd_and_hms(2025, 11, day, hour, minute, 0)
        .unwrap()
        .timestamp()
}

fn dated(id: i64, created: i64) -> store::Entry {
    store::Entry {
        id,
        text: format!("entry {id}"),
        created,
        kind: Kind::Pasted,
        model_id: None,
    }
}

#[test]
fn entries_are_grouped_under_today_yesterday_and_then_dates() {
    use app::history_view::{Day, group_by_day};
    let now = at(18, 19, 24);
    let entries = vec![
        dated(1, at(18, 13, 13)),
        dated(2, at(18, 0, 5)),
        dated(3, at(17, 23, 59)),
        dated(4, at(17, 9, 31)),
        dated(5, at(16, 10, 0)),
    ];

    let groups = group_by_day(&entries, now, bangkok());

    let days: Vec<Day> = groups.iter().map(|group| group.day).collect();
    let sizes: Vec<usize> = groups.iter().map(|group| group.entries.len()).collect();
    assert_eq!(
        days,
        [
            Day::Today,
            Day::Yesterday,
            Day::On(chrono::NaiveDate::from_ymd_opt(2025, 11, 16).unwrap())
        ]
    );
    assert_eq!(sizes, [2, 2, 1]);
}

#[test]
fn the_day_boundary_follows_the_given_zone_not_utc() {
    use app::history_view::{Day, day_of};
    let now = at(18, 0, 30);

    assert_eq!(day_of(at(18, 0, 10), now, bangkok()), Some(Day::Today));
    assert_eq!(day_of(at(17, 23, 50), now, bangkok()), Some(Day::Yesterday));
    let utc = chrono::FixedOffset::east_opt(0).unwrap();
    assert_eq!(day_of(at(17, 23, 50), now, utc), Some(Day::Today));
}

#[test]
fn no_entries_make_no_groups() {
    assert!(history_view::group_by_day(&[], at(18, 9, 0), bangkok()).is_empty());
}

#[test]
fn a_thai_search_is_highlighted_wherever_it_occurs() {
    let text = "ช่วยส่งรายงานประจำสัปดาห์ และรายงานผล";
    let ranges = history_view::match_ranges(text, "รายงาน");

    let found: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
    assert_eq!(found, ["รายงาน", "รายงาน"]);
}

#[test]
fn a_highlight_never_splits_a_thai_syllable_from_its_vowel_or_tone_mark() {
    let text = "ผู้ใช้ที่ไม่มีสิทธิ์";
    let ranges = history_view::match_ranges(text, "ที");
    let found: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
    assert_eq!(found, ["ที่"]);

    let ranges = history_view::match_ranges(text, "ผ");
    let found: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
    assert_eq!(found, ["ผู้"]);

    let ranges = history_view::match_ranges(text, "\u{0E34}");
    let found: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
    assert_eq!(found, ["สิ", "ธิ์"]);
}

#[test]
fn highlighting_ignores_case_and_mixed_scripts() {
    let text = "ขอ Deploy ขึ้น production";
    let found: Vec<&str> = history_view::match_ranges(text, "deploy")
        .iter()
        .map(|range| &text[range.clone()])
        .collect();
    assert_eq!(found, ["Deploy"]);
    assert!(history_view::match_ranges(text, "").is_empty());
    assert!(history_view::match_ranges(text, "   ").is_empty());
    assert!(history_view::match_ranges(text, "ไม่มี").is_empty());
}

#[test]
fn highlight_ranges_are_valid_char_boundaries_and_do_not_overlap() {
    let text = "กกก ก่ก กกกก";
    for query in ["ก", "กก", "ก่", "่"] {
        let ranges = history_view::match_ranges(text, query);
        let mut last_end = 0;
        for range in ranges {
            assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
            assert!(range.start >= last_end && range.end > range.start);
            last_end = range.end;
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CentralEurope;

impl CentralEurope {
    fn offset_at_utc(utc: &chrono::NaiveDateTime) -> chrono::FixedOffset {
        let summer_starts = chrono::NaiveDate::from_ymd_opt(2025, 3, 30)
            .unwrap()
            .and_hms_opt(1, 0, 0)
            .unwrap();
        let hours = if *utc >= summer_starts { 2 } else { 1 };
        chrono::FixedOffset::east_opt(hours * 3600).unwrap()
    }
}

impl chrono::TimeZone for CentralEurope {
    type Offset = chrono::FixedOffset;

    fn from_offset(_: &chrono::FixedOffset) -> Self {
        CentralEurope
    }

    fn offset_from_local_date(
        &self,
        local: &chrono::NaiveDate,
    ) -> chrono::LocalResult<chrono::FixedOffset> {
        self.offset_from_local_datetime(&local.and_hms_opt(12, 0, 0).unwrap())
    }

    fn offset_from_local_datetime(
        &self,
        local: &chrono::NaiveDateTime,
    ) -> chrono::LocalResult<chrono::FixedOffset> {
        chrono::LocalResult::Single(Self::offset_at_utc(&(*local - chrono::Duration::hours(1))))
    }

    fn offset_from_utc_date(&self, utc: &chrono::NaiveDate) -> chrono::FixedOffset {
        Self::offset_at_utc(&utc.and_hms_opt(0, 0, 0).unwrap())
    }

    fn offset_from_utc_datetime(&self, utc: &chrono::NaiveDateTime) -> chrono::FixedOffset {
        Self::offset_at_utc(utc)
    }
}

#[test]
fn an_entry_from_before_a_daylight_saving_change_keeps_its_own_local_time_and_day() {
    use app::history_view::{Day, day_of, format_time, group_by_day};
    use chrono::TimeZone;
    let now = chrono::Utc
        .with_ymd_and_hms(2025, 6, 18, 10, 0, 0)
        .unwrap()
        .timestamp();
    let winter_late_evening = chrono::Utc
        .with_ymd_and_hms(2025, 1, 10, 22, 30, 0)
        .unwrap()
        .timestamp();

    assert_eq!(format_time(winter_late_evening, CentralEurope), "23:30");
    assert_eq!(
        day_of(winter_late_evening, now, CentralEurope),
        Some(Day::On(
            chrono::NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()
        ))
    );
    let entries = vec![dated(1, now), dated(2, winter_late_evening)];
    let days: Vec<Day> = group_by_day(&entries, now, CentralEurope)
        .iter()
        .map(|group| group.day)
        .collect();
    assert_eq!(
        days,
        [
            Day::Today,
            Day::On(chrono::NaiveDate::from_ymd_opt(2025, 1, 10).unwrap())
        ]
    );
}
