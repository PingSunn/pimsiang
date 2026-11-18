use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use audio::fake::FakeMicrophone;
use engine::fake::FakeEngine;
use engine::{Engine, LanguageMode, UtteranceOptions};
use platform::fake::FakePlatform;
use platform::{PillError, PillState, Status, TriggerEvent};

const HOLD: Duration = Duration::from_millis(200);
const LINGER: Duration = Duration::from_millis(400);
const ERROR_LINGER: Duration = Duration::from_secs(3);

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        language: LanguageMode::ThMix,
        pill_linger: LINGER,
        error_linger: ERROR_LINGER,
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
            PillState::Partial("สวัส".to_owned()),
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
            PillState::Partial("สวัส".to_owned()),
            PillState::Partial("สวัสดี".to_owned())
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

    assert_eq!(
        c.platform().pills(),
        [PillState::Error(PillError::ModelNotLoaded)]
    );
    assert_eq!(c.microphone().start_count(), 0);
}

#[test]
fn paste_failure_is_shown_in_the_pill() {
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
        Some(&PillState::Error(PillError::PasteFailed))
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
    assert_eq!(c.platform().pills()[1], PillState::Partial("a".to_owned()));
}

#[test]
fn a_trigger_press_queued_behind_the_release_starts_the_next_utterance() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut engine = FakeEngine::new(["a"], "one");
    engine.load().unwrap();
    let c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(1);

    tx.send(Input::Trigger(TriggerEvent::pressed(t0))).unwrap();
    tx.send(Input::Tick(t0 + HOLD)).unwrap();
    tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    tx.send(Input::Trigger(TriggerEvent::released(t1))).unwrap();
    tx.send(Input::Audio(vec![0.0; 800])).unwrap();
    tx.send(Input::Trigger(TriggerEvent::pressed(t1))).unwrap();
    tx.send(Input::Tick(t1 + HOLD)).unwrap();
    drop(tx);
    let c = c.run(rx);

    assert_eq!(c.microphone().start_count(), 2);
    assert!(c.microphone().is_capturing());
}
