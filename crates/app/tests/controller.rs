use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use app::preferences::Preferences;
use audio::fake::FakeMicrophone;
use engine::fake::FakeEngine;
use engine::{Engine, LanguageMode, UtteranceOptions};
use platform::fake::FakePlatform;
use platform::{PillError, PillNotice, PillState, Sound, Status, TriggerEvent, TriggerKey};

const IDLE: Duration = Duration::from_secs(600);
const HOLD: Duration = Duration::from_millis(200);
const TAP_WINDOW: Duration = Duration::from_millis(300);
const LINGER: Duration = Duration::from_millis(400);
const ERROR_LINGER: Duration = Duration::from_secs(3);

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        language: LanguageMode::ThMix,
        pill_linger: LINGER,
        error_linger: ERROR_LINGER,
        notice_linger: Duration::from_millis(1500),
        double_tap_window: TAP_WINDOW,
        sounds: true,
        clipboard_restore: true,
        trigger_key: TriggerKey::RightOption,
        unload_when_idle: false,
        idle_after: IDLE,
    }
}

type TestController = Controller<FakePlatform, FakeEngine, FakeMicrophone>;

fn controller(final_text: &str) -> TestController {
    let mut engine = FakeEngine::new(["สวัส"], final_text);
    engine.load().unwrap();
    Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config())
}

fn press(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::pressed(at)));
}

fn release(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::released(at)));
}

fn hold(c: &mut TestController, t0: Instant) {
    press(c, t0);
    c.handle(Input::Tick(t0 + HOLD));
}

#[test]
fn hold_speak_release_pastes_the_final_text_once() {
    let mut c = controller("สวัสดีครับ");
    let t0 = Instant::now();

    hold(&mut c, t0);
    assert!(c.microphone().is_capturing());
    assert_eq!(c.platform().statuses(), [Status::Listening]);

    c.handle(Input::Audio(vec![0.0; 1_600]));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t0 + Duration::from_secs(2));

    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pastes(), ["สวัสดีครับ"]);
    assert_eq!(
        c.platform().statuses(),
        [Status::Listening, Status::Transcribing, Status::Ready]
    );
    assert_eq!(c.engine().pushed_samples(), 3_200);
}

#[test]
fn releasing_before_the_threshold_is_not_an_utterance() {
    let mut c = controller("ไม่ควรถูกวาง");
    let t0 = Instant::now();

    press(&mut c, t0);
    release(&mut c, t0 + HOLD / 2);
    c.handle(Input::Tick(t0 + HOLD * 2));

    assert_eq!(c.microphone().start_count(), 0);
    assert!(c.platform().statuses().is_empty());
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn nothing_starts_before_the_threshold_passes() {
    let mut c = controller("x");
    let t0 = Instant::now();

    press(&mut c, t0);
    c.handle(Input::Tick(t0 + HOLD / 2));

    assert_eq!(c.microphone().start_count(), 0);
    assert_eq!(c.next_deadline(), Some(t0 + HOLD));
}

#[test]
fn deadline_is_only_set_while_waiting_for_the_threshold() {
    let mut c = controller("x");
    let t0 = Instant::now();

    assert_eq!(c.next_deadline(), None);
    hold(&mut c, t0);
    assert_eq!(c.next_deadline(), None);
}

#[test]
fn empty_final_text_pastes_nothing() {
    let mut c = controller("  ");
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.platform().statuses().last(), Some(&Status::Ready));
}

#[test]
fn key_repeat_while_listening_does_not_restart_the_utterance() {
    let mut c = controller("สวัสดี");
    let t0 = Instant::now();

    hold(&mut c, t0);
    press(&mut c, t0 + Duration::from_secs(1));
    release(&mut c, t0 + Duration::from_secs(2));

    assert_eq!(c.microphone().start_count(), 1);
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
}

#[test]
fn a_second_utterance_works_after_the_first() {
    let mut c = controller("หนึ่ง");
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));
    let t1 = t0 + Duration::from_secs(5);
    hold(&mut c, t1);
    release(&mut c, t1 + Duration::from_secs(1));

    assert_eq!(c.platform().pastes(), ["หนึ่ง", "หนึ่ง"]);
}

#[test]
fn audio_outside_an_utterance_is_dropped() {
    let mut c = controller("x");

    c.handle(Input::Audio(vec![0.0; 1_600]));

    assert_eq!(c.engine().pushed_samples(), 0);
}

#[test]
fn model_is_loaded_when_an_utterance_starts() {
    let engine = FakeEngine::new(Vec::<String>::new(), "สวัสดี");
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::ModelLoaded);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.engine().load_count(), 1);
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
}

#[test]
fn utterance_uses_the_configured_language_mode() {
    let mut c = controller("x");
    hold(&mut c, Instant::now());

    assert_eq!(
        c.engine().last_options(),
        Some(UtteranceOptions {
            language: LanguageMode::ThMix
        })
    );
}

#[test]
fn microphone_failure_leaves_the_app_ready_and_pastes_nothing() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let mut c = Controller::new(
        FakePlatform::new(),
        engine,
        FakeMicrophone::failing(),
        config(),
    );
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert!(c.platform().pastes().is_empty());
    assert!(c.platform().statuses().is_empty());
    assert!(!c.microphone().is_capturing());
}

#[test]
fn paste_failure_still_returns_to_ready() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let mut c = Controller::new(
        FakePlatform::new().failing_paste(),
        engine,
        FakeMicrophone::new(),
        config(),
    );
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.platform().statuses().last(), Some(&Status::Ready));
}

#[test]
fn run_turns_channel_events_into_ticks_and_returns_when_the_channel_closes() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut engine = FakeEngine::new(Vec::<String>::new(), "สวัสดี");
    engine.load().unwrap();
    let c = Controller::new(
        FakePlatform::new(),
        engine,
        FakeMicrophone::new(),
        Config {
            hold_threshold: Duration::from_millis(20),
            ..config()
        },
    );
    let worker = std::thread::spawn(move || c.run(rx));

    tx.send(Input::Trigger(TriggerEvent::pressed(Instant::now())))
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    tx.send(Input::Trigger(TriggerEvent::released(Instant::now())))
        .unwrap();
    drop(tx);
    let c = worker.join().unwrap();

    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
}

#[test]
fn audio_queued_behind_the_release_still_reaches_the_engine() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut engine = FakeEngine::new(Vec::<String>::new(), "จบประโยค");
    engine.load().unwrap();
    let c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();

    tx.send(Input::Trigger(TriggerEvent::pressed(t0))).unwrap();
    tx.send(Input::Tick(t0 + HOLD)).unwrap();
    tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    tx.send(Input::Trigger(TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )))
    .unwrap();
    tx.send(Input::Audio(vec![0.0; 800])).unwrap();
    tx.send(Input::Audio(vec![0.0; 400])).unwrap();
    drop(tx);
    let c = c.run(rx);

    assert_eq!(c.engine().pushed_samples(), 2_800);
    assert_eq!(c.platform().pastes(), ["จบประโยค"]);
}

fn after_linger(c: &mut TestController) {
    c.handle(Input::Tick(
        Instant::now() + LINGER + Duration::from_secs(1),
    ));
}

#[test]
fn pill_follows_listening_partial_transcribing_then_hides() {
    let mut c = controller("สวัสดีครับ");
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t0 + Duration::from_secs(2));
    assert_eq!(
        c.platform().pills(),
        [
            PillState::Listening,
            PillState::partial("สวัส"),
            PillState::Transcribing
        ]
    );

    after_linger(&mut c);
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
}

#[test]
fn pill_stays_until_the_linger_has_passed() {
    let mut c = controller("สวัสดี");
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));
    c.handle(Input::Tick(Instant::now()));

    assert_eq!(c.platform().pills().last(), Some(&PillState::Transcribing));
    assert!(c.next_deadline().is_some());
}

#[test]
fn repeated_partial_text_is_not_shown_twice() {
    let mut engine = FakeEngine::new(["สวัส", "สวัส", "สวัสดี"], "สวัสดี");
    engine.load().unwrap();
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());

    hold(&mut c, Instant::now());
    for _ in 0..3 {
        c.handle(Input::Audio(vec![0.0; 1_600]));
    }

    assert_eq!(
        c.platform().pills(),
        [
            PillState::Listening,
            PillState::partial("สวัส"),
            PillState::partial("สวัสดี")
        ]
    );
}

#[test]
fn a_new_utterance_cancels_the_pending_hide() {
    let mut c = controller("หนึ่ง");
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));
    hold(&mut c, t0 + Duration::from_secs(2));
    after_linger(&mut c);

    assert_eq!(c.platform().pills().last(), Some(&PillState::Listening));
}

#[test]
fn microphone_failure_is_shown_in_the_pill_then_hidden() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let mut c = Controller::new(
        FakePlatform::new(),
        engine,
        FakeMicrophone::failing(),
        config(),
    );

    hold(&mut c, Instant::now());
    assert_eq!(
        c.platform().pills(),
        [PillState::Error(PillError::MicrophoneUnavailable)]
    );

    c.handle(Input::Tick(
        Instant::now() + LINGER + Duration::from_secs(1),
    ));
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Error(PillError::MicrophoneUnavailable))
    );
    c.handle(Input::Tick(
        Instant::now() + ERROR_LINGER + Duration::from_secs(1),
    ));
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
}

#[test]
fn model_that_cannot_load_is_shown_in_the_pill() {
    let engine = FakeEngine::new(Vec::<String>::new(), "x").failing_load();
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());

    hold(&mut c, Instant::now());
    c.handle(Input::ModelLoaded);

    assert_eq!(
        c.platform().pills(),
        [
            PillState::LoadingModel,
            PillState::Error(PillError::ModelNotLoaded)
        ]
    );
    assert!(!c.microphone().is_capturing());
}

#[test]
fn paste_failure_is_shown_in_the_pill_as_a_kept_note() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let mut c = Controller::new(
        FakePlatform::new().failing_paste(),
        engine,
        FakeMicrophone::new(),
        config(),
    );
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::PasteFailedKeptAsNote))
    );
}

#[test]
fn empty_final_text_still_hides_the_pill() {
    let mut c = controller("  ");
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));
    after_linger(&mut c);

    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
}

#[test]
fn backlog_of_audio_costs_one_decode_and_does_not_delay_the_release() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut engine = FakeEngine::new(["a", "ab", "abc", "abcd"], "abcd");
    engine.load().unwrap();
    let c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();

    tx.send(Input::Trigger(TriggerEvent::pressed(t0))).unwrap();
    tx.send(Input::Tick(t0 + HOLD)).unwrap();
    for _ in 0..4 {
        tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    }
    tx.send(Input::Trigger(TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )))
    .unwrap();
    for _ in 0..3 {
        tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    }
    drop(tx);
    let c = c.run(rx);

    assert_eq!(c.engine().pushed_samples(), 7 * 1_600);
    assert_eq!(c.engine().push_count(), 2);
    assert_eq!(c.platform().pills()[1], PillState::partial("a"));
}

fn escape(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::escape(at)));
}

fn combined(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::combined(at)));
}

fn tap(c: &mut TestController, at: Instant) {
    press(c, at);
    release(c, at + Duration::from_millis(60));
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn is_hidden(c: &TestController) -> bool {
    c.platform().pills().last() == Some(&PillState::Hidden)
}

#[test]
fn double_tap_starts_hands_free_and_the_next_tap_pastes() {
    let mut c = controller("สวัสดีครับ");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(200));

    assert!(c.microphone().is_capturing());
    assert_eq!(c.platform().statuses(), [Status::Listening]);
    assert_eq!(c.next_deadline(), None);

    c.handle(Input::Audio(vec![0.0; 1_600]));
    c.handle(Input::Tick(t0 + Duration::from_secs(5)));
    assert!(c.microphone().is_capturing());
    assert!(c.platform().pastes().is_empty());

    let t1 = t0 + Duration::from_secs(6);
    press(&mut c, t1);
    assert!(c.microphone().is_capturing());
    assert!(c.platform().pastes().is_empty());

    release(&mut c, t1 + ms(60));
    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pastes(), ["สวัสดีครับ"]);
    assert_eq!(c.microphone().start_count(), 1);
    assert_eq!(c.platform().pastes().len(), 1);
}

#[test]
fn taps_too_far_apart_are_not_hands_free() {
    let mut c = controller("x");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(60) + TAP_WINDOW + ms(10));

    assert_eq!(c.microphone().start_count(), 0);
    assert!(c.platform().statuses().is_empty());
}

#[test]
fn tap_then_hold_is_push_to_talk() {
    let mut c = controller("ถือค้าง");
    let t0 = Instant::now();

    tap(&mut c, t0);
    let t1 = t0 + ms(200);
    press(&mut c, t1);
    assert_eq!(c.microphone().start_count(), 0);
    c.handle(Input::Tick(t1 + HOLD));
    assert!(c.microphone().is_capturing());

    release(&mut c, t1 + Duration::from_secs(2));

    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pastes(), ["ถือค้าง"]);
}

#[test]
fn a_tap_after_hands_free_ended_starts_counting_afresh() {
    let mut c = controller("x");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(200));
    tap(&mut c, t0 + ms(400));
    assert_eq!(c.platform().pastes().len(), 1);

    tap(&mut c, t0 + ms(600));

    assert_eq!(c.microphone().start_count(), 1);
    assert!(!c.microphone().is_capturing());
}

#[test]
fn a_triple_tap_starts_and_ends_one_hands_free_utterance() {
    let mut c = controller("สาม");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(150));
    tap(&mut c, t0 + ms(300));

    assert_eq!(c.microphone().start_count(), 1);
    assert_eq!(c.platform().pastes(), ["สาม"]);
}

#[test]
fn hands_free_survives_very_long_speech() {
    let mut c = controller("ยาวมาก");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(200));
    for minute in 1..=30 {
        c.handle(Input::Audio(vec![0.0; 16_000]));
        c.handle(Input::Tick(t0 + Duration::from_secs(60 * minute)));
    }

    assert!(c.microphone().is_capturing());
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::partial("สวัส"))
    );
    assert!(c.platform().pastes().is_empty());

    tap(&mut c, t0 + Duration::from_secs(60 * 31));
    assert_eq!(c.platform().pastes(), ["ยาวมาก"]);
}

#[test]
fn escape_cancels_push_to_talk_and_the_release_does_nothing() {
    let mut c = controller("ไม่ควรถูกวาง");
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::Audio(vec![0.0; 1_600]));
    escape(&mut c, t0 + Duration::from_secs(1));

    assert!(!c.microphone().is_capturing());
    assert_eq!(c.engine().cancel_count(), 1);
    assert!(is_hidden(&c));
    assert_eq!(c.platform().statuses().last(), Some(&Status::Ready));
    assert_eq!(c.next_deadline(), None);

    release(&mut c, t0 + Duration::from_secs(2));
    assert!(c.platform().pastes().is_empty());
    assert!(!c.platform().statuses().contains(&Status::Transcribing));
}

#[test]
fn escape_cancels_hands_free() {
    let mut c = controller("ไม่ควรถูกวาง");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(200));
    escape(&mut c, t0 + Duration::from_secs(3));

    assert!(!c.microphone().is_capturing());
    assert!(is_hidden(&c));
    assert!(c.platform().pastes().is_empty());

    tap(&mut c, t0 + Duration::from_secs(5));
    assert_eq!(c.microphone().start_count(), 1);
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn escape_while_waiting_for_the_threshold_prevents_the_utterance() {
    let mut c = controller("x");
    let t0 = Instant::now();

    press(&mut c, t0);
    escape(&mut c, t0 + HOLD / 2);
    c.handle(Input::Tick(t0 + HOLD * 2));

    assert_eq!(c.microphone().start_count(), 0);
    assert_eq!(c.next_deadline(), None);
}

#[test]
fn escape_between_two_taps_breaks_the_double_tap() {
    let mut c = controller("x");
    let t0 = Instant::now();

    tap(&mut c, t0);
    escape(&mut c, t0 + ms(100));
    tap(&mut c, t0 + ms(200));

    assert_eq!(c.microphone().start_count(), 0);
}

#[test]
fn escape_when_idle_changes_nothing() {
    let mut c = controller("x");

    escape(&mut c, Instant::now());

    assert!(c.platform().pills().is_empty());
    assert!(c.platform().statuses().is_empty());
    assert!(c.platform().sounds().is_empty());
}

#[test]
fn escape_after_the_final_text_arrived_cannot_undo_the_paste() {
    let mut c = controller("วางแล้ว");
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));
    escape(&mut c, t0 + Duration::from_secs(2));

    assert_eq!(c.platform().pastes(), ["วางแล้ว"]);
}

#[test]
fn trigger_with_another_key_before_the_threshold_is_not_an_utterance() {
    let mut c = controller("x");
    let t0 = Instant::now();

    press(&mut c, t0);
    combined(&mut c, t0 + HOLD / 2);
    c.handle(Input::Tick(t0 + HOLD * 2));
    release(&mut c, t0 + HOLD * 3);

    assert_eq!(c.microphone().start_count(), 0);
    assert!(c.platform().statuses().is_empty());
    assert!(c.platform().pills().is_empty());
    assert!(c.platform().sounds().is_empty());
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn a_shortcut_does_not_count_as_the_first_tap_of_a_double_tap() {
    let mut c = controller("x");
    let t0 = Instant::now();

    press(&mut c, t0);
    combined(&mut c, t0 + ms(20));
    release(&mut c, t0 + ms(60));
    tap(&mut c, t0 + ms(150));

    assert_eq!(c.microphone().start_count(), 0);
}

#[test]
fn another_key_while_holding_past_the_threshold_cancels_push_to_talk() {
    let mut c = controller("ไม่ควรถูกวาง");
    let t0 = Instant::now();

    hold(&mut c, t0);
    combined(&mut c, t0 + HOLD * 2);
    release(&mut c, t0 + HOLD * 3);

    assert!(!c.microphone().is_capturing());
    assert!(is_hidden(&c));
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn start_and_stop_sounds_play_around_push_to_talk() {
    let mut c = controller("x");
    let t0 = Instant::now();

    press(&mut c, t0);
    assert!(c.platform().sounds().is_empty());
    c.handle(Input::Tick(t0 + HOLD));
    assert_eq!(c.platform().sounds(), [Sound::Start]);

    release(&mut c, t0 + Duration::from_secs(1));
    assert_eq!(c.platform().sounds(), [Sound::Start, Sound::Stop]);
}

#[test]
fn start_and_stop_sounds_play_around_hands_free() {
    let mut c = controller("x");
    let t0 = Instant::now();

    tap(&mut c, t0);
    assert!(c.platform().sounds().is_empty());
    tap(&mut c, t0 + ms(200));
    assert_eq!(c.platform().sounds(), [Sound::Start]);
    tap(&mut c, t0 + Duration::from_secs(2));

    assert_eq!(c.platform().sounds(), [Sound::Start, Sound::Stop]);
}

#[test]
fn cancelling_plays_the_stop_sound() {
    let mut c = controller("x");
    let t0 = Instant::now();

    hold(&mut c, t0);
    escape(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().sounds(), [Sound::Start, Sound::Stop]);
}

#[test]
fn the_start_sound_plays_before_the_microphone_opens() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let mut c = Controller::new(
        FakePlatform::new(),
        engine,
        FakeMicrophone::failing(),
        config(),
    );

    hold(&mut c, Instant::now());

    assert_eq!(c.platform().sounds(), [Sound::Start, Sound::Stop]);
    assert_eq!(c.microphone().start_count(), 0);
}

#[test]
fn no_sound_plays_when_a_permission_is_missing() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let platform = FakePlatform::new().with_permission(
        platform::Permission::Microphone,
        platform::PermissionStatus::Denied,
    );
    let mut c = Controller::new(platform, engine, FakeMicrophone::new(), config());

    hold(&mut c, Instant::now());

    assert!(c.platform().sounds().is_empty());
}

#[test]
fn a_shortcut_while_ending_hands_free_keeps_the_utterance_running() {
    let mut c = controller("ต่อ");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(200));
    let t1 = t0 + Duration::from_secs(3);
    press(&mut c, t1);
    combined(&mut c, t1 + ms(30));
    release(&mut c, t1 + ms(90));

    assert!(c.microphone().is_capturing());
    assert!(c.platform().pastes().is_empty());

    tap(&mut c, t1 + Duration::from_secs(2));

    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pastes(), ["ต่อ"]);
}

#[test]
fn escape_while_ending_hands_free_cancels() {
    let mut c = controller("x");
    let t0 = Instant::now();

    tap(&mut c, t0);
    tap(&mut c, t0 + ms(200));
    let t1 = t0 + Duration::from_secs(3);
    press(&mut c, t1);
    escape(&mut c, t1 + ms(30));
    release(&mut c, t1 + ms(90));

    assert!(!c.microphone().is_capturing());
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn sounds_can_be_switched_off() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let mut c = Controller::new(
        FakePlatform::new(),
        engine,
        FakeMicrophone::new(),
        Config {
            sounds: false,
            ..config()
        },
    );
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().pastes(), ["x"]);
    assert!(c.platform().sounds().is_empty());
}

fn run_script(inputs: Vec<Input>) -> TestController {
    let (tx, rx) = std::sync::mpsc::channel();
    for input in inputs {
        tx.send(input).unwrap();
    }
    drop(tx);
    controller("วางแล้ว").run(rx)
}

fn trigger(event: TriggerEvent) -> Input {
    Input::Trigger(event)
}

#[test]
fn escape_while_transcribing_discards_the_final_text() {
    let t0 = Instant::now();
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t0 + Duration::from_secs(1))),
        trigger(TriggerEvent::escape(t0 + Duration::from_secs(1) + ms(100))),
    ]);

    assert!(c.platform().pastes().is_empty());
    assert!(is_hidden(&c));
    assert_eq!(c.platform().statuses().last(), Some(&Status::Ready));
}

#[test]
fn a_new_utterance_cannot_start_while_the_previous_one_is_transcribing() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        trigger(TriggerEvent::pressed(t1 + ms(50))),
        Input::Tick(t1 + ms(50) + HOLD),
        Input::Tick(t1 + Duration::from_secs(5)),
    ]);

    assert_eq!(c.platform().pastes(), ["วางแล้ว"]);
    assert_eq!(
        c.platform().statuses(),
        [
            Status::Listening,
            Status::Transcribing,
            Status::Ready,
            Status::Listening
        ]
    );
}

#[test]
fn a_key_still_held_after_transcription_starts_push_to_talk() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        trigger(TriggerEvent::pressed(t1 + ms(50))),
        Input::Tick(t1 + Duration::from_secs(5)),
    ]);

    assert_eq!(c.microphone().start_count(), 2);
    assert!(c.microphone().is_capturing());
    assert_eq!(c.platform().pastes(), ["วางแล้ว"]);
    assert_eq!(
        c.platform().sounds(),
        [Sound::Start, Sound::Stop, Sound::Start]
    );
}

#[test]
fn a_tap_during_transcription_is_ignored() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        trigger(TriggerEvent::pressed(t1 + ms(50))),
        trigger(TriggerEvent::released(t1 + ms(100))),
        Input::Tick(t1 + Duration::from_secs(5)),
    ]);

    assert_eq!(c.microphone().start_count(), 1);
    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pastes(), ["วางแล้ว"]);
}

#[test]
fn a_shortcut_during_transcription_does_not_start_push_to_talk() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        trigger(TriggerEvent::pressed(t1 + ms(50))),
        trigger(TriggerEvent::combined(t1 + ms(80))),
        Input::Tick(t1 + Duration::from_secs(5)),
    ]);

    assert_eq!(c.microphone().start_count(), 1);
}

#[test]
fn trigger_events_after_transcribing_work_normally() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let t2 = t1 + Duration::from_secs(5);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
    ]);
    let (tx, rx) = std::sync::mpsc::channel();
    for input in [
        trigger(TriggerEvent::pressed(t2)),
        Input::Tick(t2 + HOLD),
        trigger(TriggerEvent::released(t2 + Duration::from_secs(1))),
    ] {
        tx.send(input).unwrap();
    }
    drop(tx);
    let c = c.run(rx);

    assert_eq!(c.platform().pastes(), ["วางแล้ว", "วางแล้ว"]);
}

#[test]
fn dictation_is_unavailable_until_the_model_is_ready() {
    let engine = FakeEngine::new(["สวัส"], "สวัสดีครับ");
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config())
        .model_pending();
    c.preload_model();
    assert_eq!(c.engine().load_count(), 0);

    let t0 = Instant::now();
    hold(&mut c, t0);

    assert_eq!(
        c.platform().pills(),
        [PillState::Error(PillError::ModelNotReady)]
    );
    assert_eq!(c.engine().load_count(), 0);
    assert_eq!(c.microphone().start_count(), 0);

    c.handle(Input::ModelReady);
    c.handle(Input::ModelLoaded);
    assert!(c.engine().is_loaded());

    let t1 = t0 + Duration::from_secs(10);
    hold(&mut c, t1);
    assert!(c.microphone().is_capturing());
}

fn preferences() -> Preferences {
    Preferences::default()
}

#[test]
fn a_dictation_asks_the_platform_to_restore_the_clipboard_by_default() {
    let mut c = controller("x");
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().pastes(), ["x"]);
    assert_eq!(c.platform().paste_restores(), [true]);
}

#[test]
fn clipboard_restoring_can_be_switched_off() {
    let mut c = controller("x");
    c.handle(Input::Preferences(Preferences {
        clipboard_restore: false,
        ..preferences()
    }));
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().paste_restores(), [false]);
}

#[test]
fn clipboard_restoring_can_be_switched_back_on() {
    let mut c = controller("x");
    for restore in [false, true] {
        c.handle(Input::Preferences(Preferences {
            clipboard_restore: restore,
            ..preferences()
        }));
    }
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().paste_restores(), [true]);
}

#[test]
fn paste_last_transcript_follows_the_clipboard_restore_switch() {
    let mut c = controller("x");
    c.handle(Input::Preferences(Preferences {
        clipboard_restore: false,
        ..preferences()
    }));

    c.handle(Input::PasteText("ข้อความล่าสุด".to_owned()));

    assert_eq!(c.platform().pastes(), ["ข้อความล่าสุด"]);
    assert_eq!(c.platform().paste_restores(), [false]);
}

#[test]
fn sounds_switched_off_in_the_settings_are_silent() {
    let mut c = controller("x");
    c.handle(Input::Preferences(Preferences {
        sounds: false,
        ..preferences()
    }));
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().pastes(), ["x"]);
    assert!(c.platform().sounds().is_empty());
}

#[test]
fn sounds_switched_back_on_play_again() {
    let mut c = controller("x");
    for sounds in [false, true] {
        c.handle(Input::Preferences(Preferences {
            sounds,
            ..preferences()
        }));
    }
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().sounds(), [Sound::Start, Sound::Stop]);
}

#[test]
fn a_language_mode_change_applies_to_the_next_utterance_only() {
    let mut c = controller("x");
    let t0 = Instant::now();
    hold(&mut c, t0);

    c.handle(Input::Preferences(Preferences {
        language: LanguageMode::ThTh,
        ..preferences()
    }));

    assert_eq!(
        c.engine().last_options(),
        Some(UtteranceOptions {
            language: LanguageMode::ThMix
        })
    );
    release(&mut c, t0 + Duration::from_secs(1));
    hold(&mut c, t0 + Duration::from_secs(5));
    assert_eq!(
        c.engine().last_options(),
        Some(UtteranceOptions {
            language: LanguageMode::ThTh
        })
    );
}

#[test]
fn a_trigger_key_change_reaches_the_platform_without_a_restart() {
    let mut c = controller("x");
    assert_eq!(c.platform().trigger_key(), TriggerKey::RightOption);

    c.handle(Input::Preferences(Preferences {
        trigger_key: TriggerKey::RightCommand,
        ..preferences()
    }));

    assert_eq!(c.platform().trigger_key(), TriggerKey::RightCommand);
}

#[test]
fn the_new_trigger_key_starts_an_utterance_straight_away() {
    let mut c = controller("สวัสดี");
    c.handle(Input::Preferences(Preferences {
        trigger_key: TriggerKey::RightControl,
        ..preferences()
    }));
    let t0 = Instant::now();

    hold(&mut c, t0);
    release(&mut c, t0 + Duration::from_secs(1));

    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
}

#[test]
fn saving_the_other_preferences_does_not_touch_the_trigger_key_watcher() {
    let mut c = controller("x");
    c.handle(Input::Preferences(Preferences {
        trigger_key: TriggerKey::RightCommand,
        ..preferences()
    }));
    c.handle(Input::Preferences(Preferences {
        trigger_key: TriggerKey::RightCommand,
        sounds: false,
        ..preferences()
    }));

    assert_eq!(c.platform().trigger_key(), TriggerKey::RightCommand);
    assert_eq!(c.platform().trigger_key_changes(), 1);
}

#[test]
fn a_failed_microphone_start_closes_the_start_sound_with_the_stop_sound() {
    let mut engine = FakeEngine::new(Vec::<String>::new(), "x");
    engine.load().unwrap();
    let mut c = Controller::new(
        FakePlatform::new(),
        engine,
        FakeMicrophone::failing(),
        config(),
    );

    hold(&mut c, Instant::now());

    assert_eq!(c.platform().sounds(), [Sound::Start, Sound::Stop]);
}

#[test]
fn preferences_saved_while_transcribing_still_reach_the_controller() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        Input::Preferences(Preferences {
            trigger_key: TriggerKey::RightCommand,
            ..Preferences::default()
        }),
    ]);

    assert_eq!(c.platform().trigger_key(), TriggerKey::RightCommand);
    assert_eq!(c.platform().pastes(), ["วางแล้ว"]);
}

#[test]
fn a_paste_again_queued_while_transcribing_is_not_lost() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        Input::PasteText("ข้อความเก่า".to_owned()),
    ]);

    assert_eq!(c.platform().pastes(), ["วางแล้ว", "ข้อความเก่า"]);
}

#[test]
fn shift_on_a_press_queued_while_transcribing_makes_the_next_utterance_a_quick_note() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let mut c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        trigger(TriggerEvent::pressed(t1 + ms(50)).with_shift()),
    ]);
    let t2 = Instant::now();

    c.handle(Input::Tick(t2 + HOLD));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t2 + Duration::from_secs(1));

    assert_eq!(c.platform().pastes(), ["วางแล้ว"]);
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::QuickNoteSaved))
    );
}

#[test]
fn quit_waits_for_the_transcription_in_flight_and_stops_the_loop() {
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);
    let c = run_script(vec![
        trigger(TriggerEvent::pressed(t0)),
        Input::Tick(t0 + HOLD),
        trigger(TriggerEvent::released(t1)),
        Input::Quit,
        Input::PasteText("ไม่ควรถูกวาง".to_owned()),
    ]);

    assert_eq!(c.platform().pastes(), ["วางแล้ว"]);
}

#[test]
fn the_pill_carries_the_settled_prefix_of_a_partial() {
    let mut engine = FakeEngine::new(["สวัสดี ครับ"], "x").settled_len_of_partials("สวัสดี ".len());
    engine.load().unwrap();
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::Audio(vec![0.0; 1_600]));

    let Some(PillState::Partial(partial)) = c.platform().pills().get(1) else {
        panic!("expected a Partial on the Pill");
    };
    assert_eq!(partial.split(), ("สวัสดี ", "ครับ"));
}

#[test]
fn paste_last_while_the_trigger_key_is_held_pastes_and_starts_no_utterance() {
    let mut c = controller("x");
    let t0 = Instant::now();

    press(&mut c, t0);
    c.handle(Input::PasteText("ข้อความล่าสุด".to_owned()));
    c.handle(Input::Tick(t0 + HOLD * 2));
    release(&mut c, t0 + HOLD * 3);

    assert_eq!(c.platform().pastes(), ["ข้อความล่าสุด"]);
    assert!(!c.microphone().is_capturing());
    assert!(c.platform().statuses().is_empty());
}

#[test]
fn two_paste_last_shortcuts_with_the_trigger_key_do_not_start_hands_free() {
    let mut c = controller("x");
    let t0 = Instant::now();
    let gap = Duration::from_millis(50);

    press(&mut c, t0);
    release(&mut c, t0 + gap);
    c.handle(Input::PasteText("ข้อความล่าสุด".to_owned()));
    press(&mut c, t0 + gap * 2);
    c.handle(Input::PasteText("ข้อความล่าสุด".to_owned()));
    release(&mut c, t0 + gap * 3);
    c.handle(Input::Tick(t0 + HOLD * 2));

    assert_eq!(c.platform().pastes(), ["ข้อความล่าสุด", "ข้อความล่าสุด"]);
    assert!(!c.microphone().is_capturing());
}

#[test]
fn escape_after_a_double_tap_with_real_key_timings_cancels_hands_free() {
    let mut c = controller("ไม่ควรถูกวาง");
    let t0 = Instant::now();

    press(&mut c, t0);
    release(&mut c, t0 + ms(80));
    press(&mut c, t0 + ms(200));
    release(&mut c, t0 + ms(280));
    assert!(c.microphone().is_capturing());
    c.handle(Input::Audio(vec![0.0; 1_600]));
    escape(&mut c, t0 + ms(580));

    assert!(!c.microphone().is_capturing());
    assert!(is_hidden(&c));
    assert_eq!(c.engine().cancel_count(), 1);
    assert!(c.platform().pastes().is_empty());

    tap(&mut c, t0 + ms(3000));
    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.microphone().start_count(), 1);
}

#[test]
fn escape_queued_behind_audio_cancels_without_decoding_it_first() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut engine = FakeEngine::new(["a", "ab"], "ไม่ควรถูกวาง");
    engine.load().unwrap();
    let c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();

    tx.send(Input::Trigger(TriggerEvent::pressed(t0))).unwrap();
    tx.send(Input::Trigger(TriggerEvent::released(t0 + ms(80))))
        .unwrap();
    tx.send(Input::Trigger(TriggerEvent::pressed(t0 + ms(200))))
        .unwrap();
    tx.send(Input::Trigger(TriggerEvent::released(t0 + ms(280))))
        .unwrap();
    tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    tx.send(Input::Trigger(TriggerEvent::escape(t0 + ms(580))))
        .unwrap();
    drop(tx);
    let c = c.run(rx);

    assert_eq!(c.engine().push_count(), 0);
    assert_eq!(c.engine().cancel_count(), 1);
    assert!(!c.microphone().is_capturing());
    assert!(is_hidden(&c));
    assert!(c.platform().pastes().is_empty());
}

struct SlowEngine {
    inner: FakeEngine,
    delay: Duration,
}

impl Engine for SlowEngine {
    fn load(&mut self) -> Result<(), engine::EngineError> {
        self.inner.load()
    }
    fn unload(&mut self) {
        self.inner.unload();
    }
    fn is_loaded(&self) -> bool {
        self.inner.is_loaded()
    }
    fn start_utterance(&mut self, options: UtteranceOptions) -> Result<(), engine::EngineError> {
        self.inner.start_utterance(options)
    }
    fn push_audio(&mut self, samples: &[f32]) -> Result<Option<String>, engine::EngineError> {
        std::thread::sleep(self.delay);
        self.inner.push_audio(samples)
    }
    fn finish_utterance(&mut self) -> Result<String, engine::EngineError> {
        self.inner.finish_utterance()
    }
    fn cancel_utterance(&mut self) {
        self.inner.cancel_utterance();
    }
}

#[test]
fn escape_arriving_during_a_slow_decode_still_cancels_hands_free() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut inner = FakeEngine::new(["a", "ab", "abc"], "ไม่ควรถูกวาง");
    inner.load().unwrap();
    let engine = SlowEngine {
        inner,
        delay: ms(300),
    };
    let c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();

    tx.send(Input::Trigger(TriggerEvent::pressed(t0))).unwrap();
    tx.send(Input::Trigger(TriggerEvent::released(t0 + ms(80))))
        .unwrap();
    tx.send(Input::Trigger(TriggerEvent::pressed(t0 + ms(200))))
        .unwrap();
    tx.send(Input::Trigger(TriggerEvent::released(t0 + ms(280))))
        .unwrap();
    tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    let sender = std::thread::spawn(move || {
        std::thread::sleep(ms(100));
        tx.send(Input::Trigger(TriggerEvent::escape(Instant::now())))
            .unwrap();
        tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
        std::thread::sleep(ms(50));
        tx.send(Input::Trigger(TriggerEvent::pressed(Instant::now())))
            .unwrap();
        tx.send(Input::Trigger(TriggerEvent::released(Instant::now())))
            .unwrap();
    });
    let c = c.run(rx);
    sender.join().unwrap();

    assert_eq!(c.engine().inner.cancel_count(), 1);
    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.microphone().start_count(), 1);
}
