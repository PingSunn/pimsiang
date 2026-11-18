#![cfg(target_os = "macos")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod pill;

pub use pill::PillPanel;

use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes, kCFRunLoopDefaultMode};
use core_foundation::string::CFStringRef;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, CallbackResult, EventField, KeyCode,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use platform::{PillState, Platform, PlatformError, Status, TriggerEvent, TriggerSink};

const RIGHT_OPTION_DEVICE_MASK: u64 = 0x40;
const TAP_POLL: Duration = Duration::from_millis(500);

pub type StatusSink = Box<dyn Fn(Status) + Send>;
pub type PillSink = Box<dyn Fn(PillState) + Send>;

pub struct MacPlatform {
    status: StatusSink,
    pill: PillSink,
}

impl MacPlatform {
    pub fn new(status: StatusSink, pill: PillSink) -> Self {
        Self { status, pill }
    }
}

impl Platform for MacPlatform {
    fn listen_trigger(&mut self, sink: TriggerSink) -> Result<(), PlatformError> {
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::Builder::new()
            .name("trigger-key".to_owned())
            .spawn(move || run_event_tap(sink, ready_tx))
            .map_err(|error| PlatformError::TriggerKey(error.to_string()))?;
        ready_rx
            .recv()
            .map_err(|_| PlatformError::TriggerKey("event tap thread ended".to_owned()))?
    }

    fn show_status(&mut self, status: Status) {
        (self.status)(status);
    }

    fn show_pill(&mut self, state: PillState) {
        (self.pill)(state);
    }

    fn paste(&mut self, text: &str) -> Result<(), PlatformError> {
        let paste_error = |error: String| PlatformError::Paste(error);
        let mut clipboard =
            arboard::Clipboard::new().map_err(|error| paste_error(error.to_string()))?;
        clipboard
            .set_text(text)
            .map_err(|error| paste_error(error.to_string()))?;
        send_command_v().map_err(paste_error)
    }
}

fn send_command_v() -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|()| "cannot create an event source".to_owned())?;
    for key_down in [true, false] {
        let event = CGEvent::new_keyboard_event(source.clone(), KeyCode::ANSI_V, key_down)
            .map_err(|()| "cannot create a key event".to_owned())?;
        event.set_flags(CGEventFlags::CGEventFlagCommand);
        event.post(CGEventTapLocation::HID);
    }
    Ok(())
}

fn emit(sink: &Mutex<TriggerSink>, event: TriggerEvent) {
    if let Ok(mut sink) = sink.lock() {
        sink(event);
    }
}

fn run_event_tap(sink: TriggerSink, ready: mpsc::Sender<Result<(), PlatformError>>) {
    let sink = Arc::new(Mutex::new(sink));
    let disabled = Arc::new(AtomicBool::new(false));
    let down = Arc::new(AtomicBool::new(false));
    let callback_sink = sink.clone();
    let callback_disabled = disabled.clone();
    let callback_down = down.clone();

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::FlagsChanged],
        move |_proxy, event_type, event| {
            match event_type {
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                    callback_disabled.store(true, Ordering::SeqCst);
                }
                CGEventType::FlagsChanged => {
                    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                    if keycode == i64::from(KeyCode::RIGHT_OPTION) {
                        let pressed = event.get_flags().bits() & RIGHT_OPTION_DEVICE_MASK != 0;
                        if callback_down.swap(pressed, Ordering::SeqCst) != pressed {
                            let at = Instant::now();
                            emit(
                                &callback_sink,
                                if pressed {
                                    TriggerEvent::pressed(at)
                                } else {
                                    TriggerEvent::released(at)
                                },
                            );
                        }
                    }
                }
                _ => {}
            }
            CallbackResult::Keep
        },
    );
    let Ok(tap) = tap else {
        let _ = ready.send(Err(PlatformError::TriggerKey(
            "macOS refused the event tap; grant PimSiang Accessibility and Input Monitoring in System Settings"
                .to_owned(),
        )));
        return;
    };
    let Ok(source) = tap.mach_port().create_runloop_source(0) else {
        let _ = ready.send(Err(PlatformError::TriggerKey(
            "cannot attach the event tap to a run loop".to_owned(),
        )));
        return;
    };
    CFRunLoop::get_current().add_source(&source, common_modes());
    tap.enable();
    let _ = ready.send(Ok(()));

    loop {
        CFRunLoop::run_in_mode(default_mode(), TAP_POLL, false);
        if disabled.swap(false, Ordering::SeqCst) {
            tap.enable();
            if down.swap(false, Ordering::SeqCst) {
                emit(&sink, TriggerEvent::released(Instant::now()));
            }
        }
    }
}

#[allow(unsafe_code)]
fn common_modes() -> CFStringRef {
    unsafe { kCFRunLoopCommonModes }
}

#[allow(unsafe_code)]
fn default_mode() -> CFStringRef {
    unsafe { kCFRunLoopDefaultMode }
}
