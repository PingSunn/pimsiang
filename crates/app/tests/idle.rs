use std::sync::mpsc;
use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use app::preferences::Preferences;
use audio::fake::FakeMicrophone;
use engine::Engine;
use engine::fake::{FakeEngine, LoadGate};
use platform::fake::FakePlatform;
use platform::{PillError, PillState, Status, TriggerEvent, TriggerKey};

const IDLE: Duration = Duration::from_secs(600);
const HOLD: Duration = Duration::from_millis(200);
const MINUTE: Duration = Duration::from_secs(60);

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        language: engine::LanguageMode::ThMix,
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

type TestController = Controller<FakePlatform, FakeEngine, FakeMicrophone>;

fn loaded(final_text: &str) -> TestController {
    let mut engine = FakeEngine::new(["สวัส"], final_text);
    engine.load().unwrap();
    Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config())
}

fn unloaded_and_gated(final_text: &str) -> (TestController, LoadGate) {
    let (engine, gate) = FakeEngine::new(["สวัส"], final_text).gated_load();
    let c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    (c, gate)
}

fn hold(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::pressed(at)));
    c.handle(Input::Tick(at + HOLD));
}

fn release(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::released(at)));
}

fn escape(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::escape(at)));
}

#[test]
fn the_model_is_unloaded_after_ten_idle_minutes() {
    let mut c = loaded("x");
    let t0 = Instant::now();

    c.handle(Input::Tick(t0 + IDLE - MINUTE));
    assert!(c.engine().is_loaded());

    c.handle(Input::Tick(t0 + IDLE + MINUTE));
    assert!(!c.engine().is_loaded());
}

#[test]
fn the_run_loop_wakes_up_for_the_idle_deadline() {
    let c = loaded("x");
    let deadline = c.next_deadline();

    assert!(deadline.is_some_and(|at| at > Instant::now() + IDLE - MINUTE));
}

#[test]
fn there_is_no_idle_deadline_once_the_model_is_unloaded_or_the_switch_is_off() {
    let mut c = loaded("x");
    let t0 = Instant::now();
    c.handle(Input::Tick(t0 + IDLE + MINUTE));
    assert_eq!(c.next_deadline(), None);

    let mut c = loaded("x");
    c.handle(Input::Preferences(Preferences {
        unload_when_idle: false,
        ..Preferences::default()
    }));
    assert_eq!(c.next_deadline(), None);
}

#[test]
fn with_the_switch_off_the_model_is_never_unloaded() {
    let mut c = loaded("x");
    c.handle(Input::Preferences(Preferences {
        unload_when_idle: false,
        ..Preferences::default()
    }));
    let t0 = Instant::now();

    c.handle(Input::Tick(t0 + IDLE * 10));

    assert!(c.engine().is_loaded());
}

#[test]
fn switching_the_setting_back_on_starts_counting_again() {
    let mut c = loaded("x");
    let t0 = Instant::now();
    for unload_when_idle in [false, true] {
        c.handle(Input::Preferences(Preferences {
            unload_when_idle,
            ..Preferences::default()
        }));
    }

    c.handle(Input::Tick(t0 + IDLE + MINUTE));

    assert!(!c.engine().is_loaded());
}

#[test]
fn the_model_stays_while_an_utterance_is_open_and_the_clock_restarts_after_it() {
    let mut c = loaded("สวัสดี");
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::Tick(t0 + IDLE + MINUTE));
    assert!(c.engine().is_loaded());
    assert!(c.microphone().is_capturing());

    let released = t0 + IDLE + 2 * MINUTE;
    release(&mut c, released);
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);

    c.handle(Input::Tick(released + IDLE - MINUTE));
    assert!(c.engine().is_loaded());

    c.handle(Input::Tick(released + IDLE + MINUTE));
    assert!(!c.engine().is_loaded());
}

#[test]
fn a_cancelled_utterance_restarts_the_idle_clock() {
    let mut c = loaded("x");
    let t0 = Instant::now();

    hold(&mut c, t0 + 5 * MINUTE);
    escape(&mut c, t0 + 6 * MINUTE);

    c.handle(Input::Tick(t0 + 6 * MINUTE + IDLE - MINUTE));
    assert!(c.engine().is_loaded());
    c.handle(Input::Tick(t0 + 6 * MINUTE + IDLE + MINUTE));
    assert!(!c.engine().is_loaded());
}

#[test]
fn an_utterance_after_the_model_was_unloaded_loads_it_again_and_delivers_everything() {
    let mut c = loaded("สวัสดีครับ");
    let t0 = Instant::now();
    c.handle(Input::Tick(t0 + IDLE + MINUTE));
    assert!(!c.engine().is_loaded());

    let t1 = t0 + IDLE + 2 * MINUTE;
    hold(&mut c, t1);
    c.handle(Input::Audio(vec![0.0; 1_600]));
    c.handle(Input::ModelLoaded);
    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t1 + Duration::from_secs(2));

    assert_eq!(c.engine().load_count(), 2);
    assert_eq!(c.engine().pushed_samples(), 3_200);
    assert_eq!(c.platform().pastes(), ["สวัสดีครับ"]);
}

#[test]
fn audio_captured_while_the_model_loads_is_buffered_and_decoded_once_it_is_ready() {
    let (mut c, gate) = unloaded_and_gated("ข้อความเต็ม");
    let t0 = Instant::now();

    hold(&mut c, t0);
    assert!(c.microphone().is_capturing());
    assert_eq!(c.platform().pills(), [PillState::LoadingModel]);
    assert_eq!(c.platform().statuses(), [Status::Listening]);

    c.handle(Input::Audio(vec![0.0; 1_600]));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t0 + Duration::from_secs(2));
    c.handle(Input::Audio(vec![0.0; 800]));
    assert!(!c.microphone().is_capturing());
    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.platform().pills().last(), Some(&PillState::LoadingModel));

    gate.open();
    c.handle(Input::ModelLoaded);

    assert_eq!(c.engine().pushed_samples(), 4_000);
    assert_eq!(c.platform().pastes(), ["ข้อความเต็ม"]);
    assert_eq!(
        c.platform().statuses(),
        [Status::Listening, Status::Transcribing, Status::Ready]
    );
}

#[test]
fn still_holding_when_the_model_is_ready_goes_on_listening_with_the_buffered_audio() {
    let (mut c, gate) = unloaded_and_gated("ข้อความเต็ม");
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::Audio(vec![0.0; 1_600]));
    gate.open();
    c.handle(Input::ModelLoaded);

    assert_eq!(c.platform().pills()[1], PillState::Listening);
    assert_eq!(c.engine().pushed_samples(), 1_600);

    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t0 + Duration::from_secs(2));

    assert_eq!(c.engine().pushed_samples(), 3_200);
    assert_eq!(c.platform().pastes(), ["ข้อความเต็ม"]);
}

#[test]
fn escape_while_the_model_loads_cancels_and_keeps_the_loaded_model() {
    let (mut c, gate) = unloaded_and_gated("ไม่ควรถูกวาง");
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::Audio(vec![0.0; 1_600]));
    escape(&mut c, t0 + Duration::from_secs(1));
    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));

    gate.open();
    c.handle(Input::ModelLoaded);

    assert!(c.engine().is_loaded());
    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.platform().pills().last(), Some(&PillState::Hidden));
    assert_eq!(c.engine().pushed_samples(), 0);
}

#[test]
fn a_new_utterance_while_the_cancelled_load_is_still_running_waits_for_the_same_load() {
    let (mut c, gate) = unloaded_and_gated("สวัสดี");
    let t0 = Instant::now();

    hold(&mut c, t0);
    escape(&mut c, t0 + Duration::from_secs(1));
    hold(&mut c, t0 + Duration::from_secs(2));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t0 + Duration::from_secs(3));
    gate.open();
    c.handle(Input::ModelLoaded);

    assert_eq!(c.engine().load_count(), 1);
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
}

#[test]
fn a_model_that_fails_to_reload_is_a_pill_error_and_stops_the_microphone() {
    let engine = FakeEngine::new(["a"], "a").failing_load();
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config());
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::Audio(vec![0.0; 1_600]));
    release(&mut c, t0 + Duration::from_secs(1));
    c.handle(Input::ModelLoaded);

    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Error(PillError::ModelNotLoaded))
    );
    assert!(!c.microphone().is_capturing());
    assert!(c.platform().pastes().is_empty());
    assert_eq!(c.platform().statuses().last(), Some(&Status::Ready));
}

#[test]
fn the_run_loop_keeps_handling_input_while_the_model_loads() {
    let (engine, gate) = FakeEngine::new(["สวัส"], "สวัสดีครับ").gated_load();
    let (tx, rx) = mpsc::channel::<Input>();
    let (woken, woken_rx) = mpsc::channel::<()>();
    let c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config())
        .on_model_loaded(move || {
            let _ = woken.send(());
        });
    let handle = std::thread::spawn(move || c.run(rx));
    let t0 = Instant::now();

    tx.send(Input::Trigger(TriggerEvent::pressed(t0))).unwrap();
    tx.send(Input::Tick(t0 + HOLD)).unwrap();
    tx.send(Input::Audio(vec![0.0; 1_600])).unwrap();
    tx.send(Input::Trigger(TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )))
    .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    gate.open();
    woken_rx.recv().unwrap();
    tx.send(Input::ModelLoaded).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    drop(tx);

    let c = handle.join().unwrap();
    assert_eq!(c.platform().pastes(), ["สวัสดีครับ"]);
    assert_eq!(c.engine().pushed_samples(), 1_600);
}
