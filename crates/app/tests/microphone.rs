use std::time::{Duration, Instant};

use app::controller::{Config, Controller, Input};
use audio::MicrophoneChoice;
use audio::fake::{FakeInputDevices, FakeMicrophone};
use engine::Engine;
use engine::fake::FakeEngine;
use platform::fake::FakePlatform;
use platform::{MicrophoneEntry, PillError, PillNotice, PillState, Status, TriggerSink};

const HOLD: Duration = Duration::from_millis(200);
const POLL: Duration = Duration::from_secs(1);

type TestController = Controller<FakePlatform, FakeEngine, FakeMicrophone>;

fn headset() -> MicrophoneChoice {
    MicrophoneChoice::Device("usb".to_owned())
}

fn controller(devices: FakeInputDevices) -> TestController {
    let mut engine = FakeEngine::new(["สวัส"], "สวัสดี");
    engine.load().unwrap();
    Controller::new(
        FakePlatform::new(),
        engine,
        FakeMicrophone::with_devices(devices),
        Config {
            hold_threshold: HOLD,
            ..Config::default()
        },
    )
}

fn two_devices() -> FakeInputDevices {
    FakeInputDevices::new(
        &[("built-in", "Built-in Microphone"), ("usb", "USB Headset")],
        Some("built-in"),
    )
}

fn entry(id: &str, name: &str) -> MicrophoneEntry {
    MicrophoneEntry {
        id: id.to_owned(),
        name: name.to_owned(),
    }
}

fn dictate(c: &mut TestController, t0: Instant) {
    c.handle(Input::Trigger(platform::TriggerEvent::pressed(t0)));
    c.handle(Input::Tick(t0 + HOLD));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    c.handle(Input::Trigger(platform::TriggerEvent::released(
        t0 + Duration::from_secs(1),
    )));
}

#[test]
fn choosing_a_microphone_publishes_the_list_with_it_selected() {
    let mut c = controller(two_devices());

    c.handle(Input::Microphone(headset()));

    let menu = c.platform().microphones().last().unwrap();
    assert_eq!(
        menu.devices,
        [
            entry("built-in", "Built-in Microphone"),
            entry("usb", "USB Headset")
        ]
    );
    assert_eq!(menu.selected.as_deref(), Some("usb"));
}

#[test]
fn the_system_default_is_selected_until_a_microphone_is_chosen() {
    let mut c = controller(two_devices());

    c.handle(Input::Microphone(MicrophoneChoice::SystemDefault));

    assert_eq!(c.platform().microphones().last().unwrap().selected, None);
}

#[test]
fn the_chosen_microphone_is_the_one_opened() {
    let mut c = controller(two_devices());
    c.handle(Input::Microphone(headset()));

    dictate(&mut c, Instant::now());

    assert_eq!(c.microphone().opened().unwrap().id, "usb");
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
    assert!(
        !c.platform()
            .pills()
            .contains(&PillState::Notice(PillNotice::MicrophoneFellBack))
    );
}

#[test]
fn an_unplugged_microphone_falls_back_to_the_default_and_tells_the_user() {
    let mut c = controller(two_devices());
    c.handle(Input::Microphone(headset()));
    c.microphone_mut().unplug("usb");

    dictate(&mut c, Instant::now());

    assert_eq!(c.microphone().opened().unwrap().id, "built-in");
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
    let pills = c.platform().pills();
    let listening = pills
        .iter()
        .position(|p| *p == PillState::Listening)
        .unwrap();
    let notice = pills
        .iter()
        .position(|p| *p == PillState::Notice(PillNotice::MicrophoneFellBack))
        .unwrap();
    assert!(notice > listening);
}

#[test]
fn the_menu_drops_an_unplugged_microphone_and_selects_the_default() {
    let mut c =
        controller(two_devices()).watch_permissions(POLL, || -> TriggerSink { Box::new(|_| {}) });
    c.handle(Input::Microphone(headset()));
    let t0 = Instant::now();
    c.handle(Input::Tick(t0));
    let published = c.platform().microphones().len();

    c.microphone_mut().unplug("usb");
    c.handle(Input::Tick(t0 + POLL));

    assert_eq!(c.platform().microphones().len(), published + 1);
    let menu = c.platform().microphones().last().unwrap();
    assert_eq!(menu.devices, [entry("built-in", "Built-in Microphone")]);
    assert_eq!(menu.selected, None);
}

#[test]
fn an_unchanged_menu_is_not_published_again() {
    let mut c =
        controller(two_devices()).watch_permissions(POLL, || -> TriggerSink { Box::new(|_| {}) });
    let t0 = Instant::now();

    c.handle(Input::Tick(t0));
    c.handle(Input::Tick(t0 + POLL));
    c.handle(Input::Tick(t0 + POLL * 2));

    assert_eq!(c.platform().microphones().len(), 1);
}

#[test]
fn replugging_the_chosen_microphone_selects_it_again_without_a_new_choice() {
    let mut c =
        controller(two_devices()).watch_permissions(POLL, || -> TriggerSink { Box::new(|_| {}) });
    c.handle(Input::Microphone(headset()));
    let t0 = Instant::now();
    c.microphone_mut().unplug("usb");
    c.handle(Input::Tick(t0));
    assert_eq!(c.platform().microphones().last().unwrap().selected, None);

    c.microphone_mut().plug("usb", "USB Headset");
    c.handle(Input::Tick(t0 + POLL));

    assert_eq!(
        c.platform()
            .microphones()
            .last()
            .unwrap()
            .selected
            .as_deref(),
        Some("usb")
    );
}

#[test]
fn no_microphone_at_all_shows_an_error_in_the_pill() {
    let mut c = controller(FakeInputDevices::new(&[], None));

    dictate(&mut c, Instant::now());

    assert_eq!(
        c.platform().pills().first(),
        Some(&PillState::Error(PillError::MicrophoneUnavailable))
    );
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn choosing_the_current_microphone_again_publishes_the_menu_again() {
    let mut c = controller(two_devices());
    c.handle(Input::Microphone(headset()));
    let published = c.platform().microphones().len();

    c.handle(Input::Microphone(headset()));

    assert_eq!(c.platform().microphones().len(), published + 1);
    assert_eq!(
        c.platform()
            .microphones()
            .last()
            .unwrap()
            .selected
            .as_deref(),
        Some("usb")
    );
}

#[test]
fn choosing_the_system_default_while_the_choice_is_unplugged_publishes_the_menu_again() {
    let mut c = controller(two_devices());
    c.handle(Input::Microphone(headset()));
    c.microphone_mut().unplug("usb");
    c.handle(Input::Microphone(headset()));
    let published = c.platform().microphones().len();

    c.handle(Input::Microphone(MicrophoneChoice::SystemDefault));

    assert_eq!(c.platform().microphones().len(), published + 1);
    assert_eq!(c.platform().microphones().last().unwrap().selected, None);
}

#[test]
fn losing_the_microphone_mid_utterance_delivers_the_text_so_far_and_says_why() {
    let mut c = controller(two_devices());
    let t0 = Instant::now();
    c.handle(Input::Trigger(platform::TriggerEvent::pressed(t0)));
    c.handle(Input::Tick(t0 + HOLD));
    c.handle(Input::Audio(vec![0.0; 1_600]));

    c.handle(Input::MicrophoneLost);

    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
    assert!(!c.microphone().is_capturing());
    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Notice(PillNotice::MicrophoneLost))
    );
    assert_eq!(c.platform().statuses().last(), Some(&Status::Ready));
}

#[test]
fn losing_the_microphone_ends_a_hands_free_utterance() {
    let mut c = controller(two_devices());
    let t0 = Instant::now();
    c.handle(Input::Trigger(platform::TriggerEvent::pressed(t0)));
    c.handle(Input::Trigger(platform::TriggerEvent::released(
        t0 + HOLD / 4,
    )));
    c.handle(Input::Trigger(platform::TriggerEvent::pressed(
        t0 + HOLD / 2,
    )));
    c.handle(Input::Trigger(platform::TriggerEvent::released(
        t0 + HOLD * 3 / 4,
    )));
    c.handle(Input::Audio(vec![0.0; 1_600]));
    assert!(c.microphone().is_capturing());

    c.handle(Input::MicrophoneLost);

    assert!(!c.microphone().is_capturing());
    assert_eq!(c.platform().pastes(), ["สวัสดี"]);
}

#[test]
fn a_lost_microphone_report_while_idle_is_ignored() {
    let mut c = controller(two_devices());

    c.handle(Input::MicrophoneLost);

    assert!(c.platform().pills().is_empty());
    assert!(c.platform().pastes().is_empty());
}

#[test]
fn the_fallback_notice_stays_until_it_has_been_readable_for_a_moment() {
    let mut c = controller(two_devices());
    c.handle(Input::Microphone(headset()));
    c.microphone_mut().unplug("usb");
    let t0 = Instant::now();
    c.handle(Input::Trigger(platform::TriggerEvent::pressed(t0)));
    c.handle(Input::Tick(t0 + HOLD));

    c.handle(Input::Audio(vec![0.0; 1_600]));

    let pills = c.platform().pills();
    assert_eq!(
        pills.last(),
        Some(&PillState::Notice(PillNotice::MicrophoneFellBack))
    );
    assert!(!pills.iter().any(|p| matches!(p, PillState::Partial(_))));

    c.handle(Input::Tick(
        Instant::now() + Config::default().notice_linger,
    ));

    assert_eq!(
        c.platform().pills().last(),
        Some(&PillState::Partial("สวัส".to_owned()))
    );
}
