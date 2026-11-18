use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use audio::fake::FakeMicrophone;
use engine::fake::FakeEngine;
use engine::{Engine, ModelFiles, Precision};
use platform::TriggerEvent;
use platform::fake::FakePlatform;

const HOLD: Duration = Duration::from_millis(200);

type TestController = Controller<FakePlatform, FakeEngine, FakeMicrophone>;

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        ..Config::default()
    }
}

fn loaded(final_text: &str) -> TestController {
    let mut engine = FakeEngine::new(["สวัส"], final_text);
    engine.load().unwrap();
    Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config())
}

fn fp32() -> ModelFiles {
    ModelFiles::new("/models/fp32-9f142a43", Precision::Fp32)
}

fn hold(c: &mut TestController, at: Instant) {
    c.handle(Input::Trigger(TriggerEvent::pressed(at)));
    c.handle(Input::Tick(at + HOLD));
}

#[test]
fn switching_while_idle_points_the_engine_at_the_new_files_and_loads_them() {
    let mut c = loaded("x");

    c.handle(Input::SwitchModel(fp32()));
    c.handle(Input::ModelLoaded);

    assert_eq!(c.engine().switched_to(), [fp32()]);
    assert!(c.engine().is_loaded());
    assert_eq!(c.engine().load_count(), 2);
}

#[test]
fn switching_an_unloaded_model_waits_for_the_next_utterance_to_load() {
    let mut c = Controller::new(
        FakePlatform::new(),
        FakeEngine::new(["a"], "x"),
        FakeMicrophone::new(),
        config(),
    );

    c.handle(Input::SwitchModel(fp32()));

    assert_eq!(c.engine().switched_to(), [fp32()]);
    assert!(!c.engine().is_loaded());
    assert_eq!(c.engine().load_count(), 0);
}

#[test]
fn a_switch_during_an_utterance_waits_until_it_has_been_delivered() {
    let mut c = loaded("สวัสดี");
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::SwitchModel(fp32()));
    assert!(c.engine().switched_to().is_empty());
    assert!(c.engine().is_loaded());

    c.handle(Input::Trigger(TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )));

    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
    assert_eq!(c.engine().switched_to(), [fp32()]);
}

#[test]
fn a_switch_during_an_utterance_is_dropped_by_a_newer_one() {
    let mut c = loaded("x");
    let t0 = Instant::now();
    let int8 = ModelFiles::new("/models/int8-9f142a43", Precision::Int8);

    hold(&mut c, t0);
    c.handle(Input::SwitchModel(fp32()));
    c.handle(Input::SwitchModel(int8.clone()));
    c.handle(Input::Trigger(TriggerEvent::escape(
        t0 + Duration::from_secs(1),
    )));

    assert_eq!(c.engine().switched_to(), [int8]);
}

#[test]
fn the_switch_is_announced_once_it_has_been_applied() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut engine = FakeEngine::new(["สวัส"], "x");
    engine.load().unwrap();
    let mut c = Controller::new(FakePlatform::new(), engine, FakeMicrophone::new(), config())
        .on_model_switched(move |files| {
            let _ = tx.send(files.clone());
        });
    let t0 = Instant::now();

    hold(&mut c, t0);
    c.handle(Input::SwitchModel(fp32()));
    assert!(rx.try_recv().is_err());

    c.handle(Input::Trigger(TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )));

    assert_eq!(rx.try_recv().unwrap(), fp32());
    assert!(rx.try_recv().is_err());
}
