use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input, PermissionCommand};
use audio::fake::FakeMicrophone;
use engine::Engine;
use engine::fake::FakeEngine;
use platform::fake::FakePlatform;
use platform::{
    Permission, PermissionReport, PermissionStatus, PillError, PillState, TriggerEvent, TriggerSink,
};

const HOLD: Duration = Duration::from_millis(200);
const POLL: Duration = Duration::from_secs(1);

type TestController = Controller<FakePlatform, FakeEngine, FakeMicrophone>;

fn config() -> Config {
    Config {
        hold_threshold: HOLD,
        ..Config::default()
    }
}

fn watching(platform: FakePlatform) -> (TestController, Arc<AtomicUsize>) {
    let mut engine = FakeEngine::new(["สวัส"], "สวัสดี");
    engine.load().unwrap();
    let sinks = Arc::new(AtomicUsize::new(0));
    let made = sinks.clone();
    let controller = Controller::new(platform, engine, FakeMicrophone::new(), config())
        .watch_permissions(POLL, move || -> TriggerSink {
            made.fetch_add(1, Ordering::SeqCst);
            Box::new(|_| {})
        });
    (controller, sinks)
}

fn missing(permission: Permission, status: PermissionStatus) -> FakePlatform {
    FakePlatform::new().with_permission(permission, status)
}

#[test]
fn missing_permissions_are_reported_on_the_first_poll() {
    let (mut c, _) = watching(missing(Permission::Accessibility, PermissionStatus::Denied));

    c.handle(Input::Tick(Instant::now()));

    let reports = c.platform().reports();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].missing(), [Permission::Accessibility]);
}

#[test]
fn an_unchanged_report_is_not_published_again() {
    let (mut c, _) = watching(missing(
        Permission::Microphone,
        PermissionStatus::NotDetermined,
    ));
    let t0 = Instant::now();

    c.handle(Input::Tick(t0));
    c.handle(Input::Tick(t0 + POLL));
    c.handle(Input::Tick(t0 + POLL * 2));

    assert_eq!(c.platform().reports().len(), 1);
}

#[test]
fn a_grant_is_noticed_by_polling_without_a_restart() {
    let (mut c, _) = watching(missing(Permission::Accessibility, PermissionStatus::Denied));
    let t0 = Instant::now();
    c.handle(Input::Tick(t0));

    c.platform_mut()
        .set_permission(Permission::Accessibility, PermissionStatus::Granted);
    c.handle(Input::Tick(t0 + POLL));

    let reports = c.platform().reports();
    assert_eq!(reports.len(), 2);
    assert!(reports[1].all_granted());
}

#[test]
fn polling_waits_for_its_interval() {
    let (mut c, _) = watching(missing(Permission::Accessibility, PermissionStatus::Denied));
    let t0 = Instant::now();
    c.handle(Input::Tick(t0));
    c.platform_mut()
        .set_permission(Permission::Accessibility, PermissionStatus::Granted);

    c.handle(Input::Tick(t0 + POLL / 2));

    assert_eq!(c.platform().reports().len(), 1);
    assert_eq!(c.next_deadline(), Some(t0 + POLL));
}

#[test]
fn the_trigger_key_is_not_watched_while_input_monitoring_is_missing() {
    let (mut c, sinks) = watching(missing(
        Permission::InputMonitoring,
        PermissionStatus::NotDetermined,
    ));
    let t0 = Instant::now();

    c.handle(Input::Tick(t0));
    c.handle(Input::Tick(t0 + POLL));

    assert_eq!(c.platform().listen_attempts(), 0);
    assert_eq!(sinks.load(Ordering::SeqCst), 0);
}

#[test]
fn the_trigger_key_starts_once_input_monitoring_is_granted() {
    let (mut c, sinks) = watching(missing(
        Permission::InputMonitoring,
        PermissionStatus::NotDetermined,
    ));
    let t0 = Instant::now();
    c.handle(Input::Tick(t0));

    c.platform_mut()
        .set_permission(Permission::InputMonitoring, PermissionStatus::Granted);
    c.handle(Input::Tick(t0 + POLL));
    c.handle(Input::Tick(t0 + POLL * 2));

    assert!(c.platform().is_listening());
    assert_eq!(c.platform().listen_attempts(), 1);
    assert_eq!(sinks.load(Ordering::SeqCst), 1);
}

#[test]
fn a_refused_trigger_key_is_tried_again_on_the_next_poll() {
    let (mut c, _) = watching(FakePlatform::new().failing_listen());
    let t0 = Instant::now();

    c.handle(Input::Tick(t0));
    c.handle(Input::Tick(t0 + POLL));

    assert_eq!(c.platform().listen_attempts(), 2);
    assert!(!c.platform().is_listening());
}

#[test]
fn without_watching_nothing_is_polled() {
    let mut engine = FakeEngine::new(["a"], "a");
    engine.load().unwrap();
    let mut c = Controller::new(
        missing(Permission::Microphone, PermissionStatus::Denied),
        engine,
        FakeMicrophone::new(),
        config(),
    );

    c.handle(Input::Tick(Instant::now()));

    assert!(c.platform().reports().is_empty());
    assert_eq!(c.next_deadline(), None);
}

#[test]
fn the_microphone_prompt_is_requested_while_undetermined() {
    let (mut c, _) = watching(missing(
        Permission::Microphone,
        PermissionStatus::NotDetermined,
    ));

    c.handle(Input::Permission(PermissionCommand::Grant(
        Permission::Microphone,
    )));

    assert_eq!(c.platform().requested(), [Permission::Microphone]);
    assert!(c.platform().opened().is_empty());
}

#[test]
fn a_denied_microphone_opens_its_settings_pane() {
    let (mut c, _) = watching(missing(Permission::Microphone, PermissionStatus::Denied));

    c.handle(Input::Permission(PermissionCommand::Grant(
        Permission::Microphone,
    )));

    assert!(c.platform().requested().is_empty());
    assert_eq!(c.platform().opened(), [Permission::Microphone]);
}

#[test]
fn accessibility_is_requested_then_its_pane_opens() {
    let (mut c, _) = watching(missing(Permission::Accessibility, PermissionStatus::Denied));

    c.handle(Input::Permission(PermissionCommand::Grant(
        Permission::Accessibility,
    )));

    assert_eq!(c.platform().requested(), [Permission::Accessibility]);
    assert_eq!(c.platform().opened(), [Permission::Accessibility]);
}

#[test]
fn input_monitoring_is_requested_then_its_pane_opens() {
    let (mut c, _) = watching(missing(
        Permission::InputMonitoring,
        PermissionStatus::NotDetermined,
    ));

    c.handle(Input::Permission(PermissionCommand::Grant(
        Permission::InputMonitoring,
    )));

    assert_eq!(c.platform().requested(), [Permission::InputMonitoring]);
    assert_eq!(c.platform().opened(), [Permission::InputMonitoring]);
}

#[test]
fn a_report_can_be_read_from_any_platform() {
    let platform = missing(Permission::Microphone, PermissionStatus::Denied);
    let report = PermissionReport::read(&platform);
    assert_eq!(
        report.status(Permission::Microphone),
        PermissionStatus::Denied
    );
    assert!(!report.all_granted());
}

fn hold(c: &mut TestController, t0: Instant) {
    c.handle(Input::Trigger(TriggerEvent::pressed(t0)));
    c.handle(Input::Tick(t0 + HOLD));
}

#[test]
fn pressing_the_trigger_without_the_microphone_permission_says_why() {
    let mut engine = FakeEngine::new(["a"], "a");
    engine.load().unwrap();
    let mut c = Controller::new(
        missing(Permission::Microphone, PermissionStatus::Denied),
        engine,
        FakeMicrophone::new(),
        config(),
    );

    hold(&mut c, Instant::now());

    assert_eq!(c.microphone().start_count(), 0);
    assert_eq!(
        c.platform().pills(),
        [PillState::Error(PillError::PermissionMissing(
            Permission::Microphone
        ))]
    );
}

#[test]
fn pressing_the_trigger_without_accessibility_says_why() {
    let mut engine = FakeEngine::new(["a"], "a");
    engine.load().unwrap();
    let mut c = Controller::new(
        missing(Permission::Accessibility, PermissionStatus::Denied),
        engine,
        FakeMicrophone::new(),
        config(),
    );

    hold(&mut c, Instant::now());

    assert_eq!(c.microphone().start_count(), 0);
    assert_eq!(
        c.platform().pills(),
        [PillState::Error(PillError::PermissionMissing(
            Permission::Accessibility
        ))]
    );
}

#[test]
fn run_polls_permissions_without_any_input() {
    let mut engine = FakeEngine::new(["a"], "a");
    engine.load().unwrap();
    let c = Controller::new(
        missing(Permission::Accessibility, PermissionStatus::Denied),
        engine,
        FakeMicrophone::new(),
        config(),
    )
    .watch_permissions(Duration::from_millis(10), || -> TriggerSink {
        Box::new(|_| {})
    });
    let (tx, rx) = mpsc::channel::<Input>();
    let handle = std::thread::spawn(move || c.run(rx));
    std::thread::sleep(Duration::from_millis(100));
    drop(tx);

    let c = handle.join().unwrap();
    assert_eq!(c.platform().reports().len(), 1);
}
