use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use audio::fake::FakeMicrophone;
use engine::fake::FakeEngine;
use engine::{Engine, LanguageMode};
use platform::fake::FakePlatform;
use platform::{Focus, PillError, PillNotice, PillState, TriggerEvent, TriggerKey};
use store::Kind;

const IDLE: Duration = Duration::from_secs(600);
const HOLD: Duration = Duration::from_millis(200);
const NOTICE_LINGER: Duration = Duration::from_millis(1500);
const ERROR_LINGER: Duration = Duration::from_secs(3);

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        language: LanguageMode::ThMix,
        recognition_mode: engine::RecognitionMode::Standard,
        pill_linger: Duration::from_millis(400),
        error_linger: ERROR_LINGER,
        notice_linger: NOTICE_LINGER,
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

fn focused(focus: Focus, final_text: &str) -> (TestController, Recorded) {
    controller_on(FakePlatform::new().with_focus(focus), final_text)
}

fn recorded(recorded: &Recorded) -> Vec<(String, Kind)> {
    recorded.lock().unwrap().clone()
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn press(c: &mut TestController, event: TriggerEvent) {
    c.handle(Input::Trigger(event));
}

fn speak_and_release(c: &mut TestController, at: Instant) {
    c.handle(Input::Audio(vec![0.0; 1_600]));
    c.handle(Input::Trigger(TriggerEvent::released(at)));
}

fn push_to_talk(c: &mut TestController, t0: Instant, shift: bool) {
    let pressed = TriggerEvent::pressed(t0);
    press(c, if shift { pressed.with_shift() } else { pressed });
    c.handle(Input::Tick(t0 + HOLD));
    speak_and_release(c, t0 + Duration::from_secs(1));
}

fn hands_free(c: &mut TestController, t0: Instant, shift_on_second: bool) {
    press(c, TriggerEvent::pressed(t0));
    press(c, TriggerEvent::released(t0 + ms(50)));
    let second = TriggerEvent::pressed(t0 + ms(150));
    press(
        c,
        if shift_on_second {
            second.with_shift()
        } else {
            second
        },
    );
    press(c, TriggerEvent::released(t0 + ms(200)));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    press(c, TriggerEvent::pressed(t0 + Duration::from_secs(5)));
    press(
        c,
        TriggerEvent::released(t0 + Duration::from_secs(5) + ms(50)),
    );
}

#[test]
fn a_quick_note_is_stored_as_a_note_and_never_pasted_or_probed() {
    let (mut c, history) = focused(Focus::TextField, "จดไว้");

    push_to_talk(&mut c, Instant::now(), true);

    assert!(c.platform().pastes().is_empty());
    assert!(c.platform().copies().is_empty());
    assert_eq!(c.platform().probes(), 0);
    assert_eq!(recorded(&history), [("จดไว้".to_owned(), Kind::Note)]);
}

#[test]
fn a_quick_note_is_confirmed_in_the_pill() {
    let (mut c, _) = focused(Focus::TextField, "จดไว้");

    push_to_talk(&mut c, Instant::now(), true);

    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::QuickNoteSaved))
    );
    c.handle(Input::Tick(
        Instant::now() + NOTICE_LINGER + Duration::from_secs(1),
    ));
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
}

#[test]
fn a_quick_note_leaves_the_clipboard_alone() {
    let (mut c, _) = focused(Focus::TextField, "จดไว้");

    push_to_talk(&mut c, Instant::now(), true);

    assert!(c.platform().copies().is_empty());
    assert!(c.platform().paste_restores().is_empty());
}

#[test]
fn shift_counts_when_it_is_held_as_the_trigger_key_goes_down() {
    let (mut c, history) = focused(Focus::TextField, "จดไว้");

    push_to_talk(&mut c, Instant::now(), false);
    push_to_talk(&mut c, Instant::now() + Duration::from_secs(10), true);

    assert_eq!(c.platform().pastes(), ["จดไว้"]);
    assert_eq!(
        recorded(&history),
        [
            ("จดไว้".to_owned(), Kind::Pasted),
            ("จดไว้".to_owned(), Kind::Note)
        ]
    );
}

#[test]
fn a_quick_note_can_be_made_hands_free() {
    let (mut c, history) = focused(Focus::TextField, "จดยาว");

    hands_free(&mut c, Instant::now(), true);

    assert!(c.platform().pastes().is_empty());
    assert_eq!(recorded(&history), [("จดยาว".to_owned(), Kind::Note)]);
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::QuickNoteSaved))
    );
}

#[test]
fn shift_on_the_ending_tap_of_hands_free_changes_nothing() {
    let (mut c, _) = focused(Focus::TextField, "วาง");
    let t0 = Instant::now();

    press(&mut c, TriggerEvent::pressed(t0));
    press(&mut c, TriggerEvent::released(t0 + ms(50)));
    press(&mut c, TriggerEvent::pressed(t0 + ms(150)));
    press(&mut c, TriggerEvent::released(t0 + ms(200)));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    press(
        &mut c,
        TriggerEvent::pressed(t0 + Duration::from_secs(5)).with_shift(),
    );
    press(
        &mut c,
        TriggerEvent::released(t0 + Duration::from_secs(5) + ms(50)),
    );

    assert_eq!(c.platform().pastes(), ["วาง"]);
}

#[test]
fn the_next_utterance_after_a_quick_note_is_a_dictation_again() {
    let (mut c, history) = focused(Focus::TextField, "ข้อความ");
    let t0 = Instant::now();

    push_to_talk(&mut c, t0, true);
    push_to_talk(&mut c, t0 + Duration::from_secs(10), false);

    assert_eq!(c.platform().pastes(), ["ข้อความ"]);
    assert_eq!(
        recorded(&history),
        [
            ("ข้อความ".to_owned(), Kind::Note),
            ("ข้อความ".to_owned(), Kind::Pasted)
        ]
    );
}

#[test]
fn an_empty_quick_note_stores_nothing_and_shows_no_confirmation() {
    let (mut c, history) = focused(Focus::TextField, "  ");

    push_to_talk(&mut c, Instant::now(), true);

    assert!(recorded(&history).is_empty());
    assert!(
        !c.platform()
            .pills()
            .contains(&PillState::Notice(PillNotice::QuickNoteSaved))
    );
}

#[test]
fn cancelling_a_quick_note_stores_nothing() {
    let (mut c, history) = focused(Focus::TextField, "ยกเลิก");
    let t0 = Instant::now();

    press(&mut c, TriggerEvent::pressed(t0).with_shift());
    c.handle(Input::Tick(t0 + HOLD));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    press(&mut c, TriggerEvent::escape(t0 + ms(500)));
    press(&mut c, TriggerEvent::released(t0 + ms(600)));

    assert!(recorded(&history).is_empty());
    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
}

#[test]
fn a_cancelled_quick_note_does_not_turn_the_next_dictation_into_a_note() {
    let (mut c, _) = focused(Focus::TextField, "วาง");
    let t0 = Instant::now();

    press(&mut c, TriggerEvent::pressed(t0).with_shift());
    c.handle(Input::Tick(t0 + HOLD));
    press(&mut c, TriggerEvent::escape(t0 + ms(500)));
    press(&mut c, TriggerEvent::released(t0 + ms(600)));
    push_to_talk(&mut c, t0 + Duration::from_secs(10), false);

    assert_eq!(c.platform().pastes(), ["วาง"]);
}

#[test]
fn a_focused_text_field_gets_the_paste_and_the_text_is_stored_as_pasted() {
    let (mut c, history) = focused(Focus::TextField, "สวัสดี");

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(c.platform().probes(), 1);
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
    assert_eq!(c.platform().paste_restores(), [true]);
    assert!(c.platform().copies().is_empty());
    assert_eq!(recorded(&history), [("สวัสดี".to_owned(), Kind::Pasted)]);
}

#[test]
fn an_unknown_focus_pastes_anyway_and_stores_the_text_as_pasted() {
    let (mut c, history) = focused(Focus::Unknown, "สวัสดี");

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
    assert!(c.platform().copies().is_empty());
    assert_eq!(recorded(&history), [("สวัสดี".to_owned(), Kind::Pasted)]);
    assert_ne!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::NoTextFieldKeptAsNote))
    );
}

#[test]
fn no_text_field_keeps_the_text_as_a_note_and_copies_it() {
    let (mut c, history) = focused(Focus::NoTextField, "ไม่มีช่อง");

    push_to_talk(&mut c, Instant::now(), false);

    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.platform().copies(), ["ไม่มีช่อง"]);
    assert_eq!(recorded(&history), [("ไม่มีช่อง".to_owned(), Kind::Note)]);
}

#[test]
fn no_text_field_is_explained_in_the_pill_for_long_enough_to_read() {
    let (mut c, _) = focused(Focus::NoTextField, "ไม่มีช่อง");

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::NoTextFieldKeptAsNote))
    );
    assert!(c.next_deadline().is_some());
    c.handle(Input::Tick(Instant::now() + NOTICE_LINGER));
    assert_ne!(c.platform().pills().last(), Some(&PillState::Hidden));
    c.handle(Input::Tick(
        Instant::now() + ERROR_LINGER + Duration::from_secs(1),
    ));
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
}

#[test]
fn a_note_kept_for_lack_of_a_text_field_never_restores_the_clipboard() {
    let (mut c, _) = focused(Focus::NoTextField, "ไม่มีช่อง");

    push_to_talk(&mut c, Instant::now(), false);

    assert!(c.platform().paste_restores().is_empty());
    assert_eq!(c.platform().copies(), ["ไม่มีช่อง"]);
}

#[test]
fn an_empty_dictation_is_not_probed_or_pasted() {
    let (mut c, history) = focused(Focus::NoTextField, "  ");

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(c.platform().probes(), 0);
    assert!(c.platform().copies().is_empty());
    assert!(recorded(&history).is_empty());
}

#[test]
fn the_focus_is_probed_when_the_utterance_ends_not_when_it_starts() {
    let (mut c, _) = focused(Focus::TextField, "x");
    let t0 = Instant::now();

    press(&mut c, TriggerEvent::pressed(t0));
    c.handle(Input::Tick(t0 + HOLD));
    assert_eq!(c.platform().probes(), 0);
    c.platform_mut().set_focus(Focus::NoTextField);
    speak_and_release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().probes(), 1);
    assert_eq!(c.platform().copies(), ["x"]);
}

#[test]
fn hands_free_dictation_is_probed_like_push_to_talk() {
    let (mut c, history) = focused(Focus::NoTextField, "ยาว");

    hands_free(&mut c, Instant::now(), false);

    assert_eq!(c.platform().copies(), ["ยาว"]);
    assert_eq!(recorded(&history), [("ยาว".to_owned(), Kind::Note)]);
}

#[test]
fn a_failed_paste_keeps_the_text_as_a_note_copies_it_and_says_so() {
    let (mut c, history) = controller_on(FakePlatform::new().failing_paste(), "พลาด");

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(c.platform().copies(), ["พลาด"]);
    assert_eq!(recorded(&history), [("พลาด".to_owned(), Kind::Note)]);
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::PasteFailedKeptAsNote))
    );
}

#[test]
fn a_failed_paste_that_cannot_be_copied_is_an_error() {
    let (mut c, history) =
        controller_on(FakePlatform::new().failing_paste().failing_copy(), "พลาด");

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(recorded(&history), [("พลาด".to_owned(), Kind::Note)]);
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Error(PillError::PasteFailed))
    );
}

#[test]
fn no_text_field_and_a_failing_copy_is_an_error_and_still_a_note() {
    let (mut c, history) = controller_on(
        FakePlatform::new()
            .with_focus(Focus::NoTextField)
            .failing_copy(),
        "ไม่มีช่อง",
    );

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(recorded(&history), [("ไม่มีช่อง".to_owned(), Kind::Note)]);
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Error(PillError::PasteFailed))
    );
}

#[test]
fn clipboard_restore_stays_off_when_the_setting_is_off_and_the_focus_is_fine() {
    let (mut c, _) = focused(Focus::TextField, "x");
    c.handle(Input::Preferences(app::preferences::Preferences {
        clipboard_restore: false,
        ..app::preferences::Preferences::default()
    }));

    push_to_talk(&mut c, Instant::now(), false);

    assert_eq!(c.platform().paste_restores(), [false]);
}
