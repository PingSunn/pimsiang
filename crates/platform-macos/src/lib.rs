#![cfg(target_os = "macos")]

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub mod dock;
mod focus;
pub mod login_item;
mod pasteboard;
mod permissions;
mod pill;
mod sound;

pub use pill::{PillArt, PillEvent, PillEvents, PillLayout, PillPanel, PlacementEnd};
pub use sound::play as play_sound;

use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes, kCFRunLoopDefaultMode};
use core_foundation::string::CFStringRef;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, CallbackResult, EventField, KeyCode,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use platform::{
    Focus, MicrophoneMenu, Permission, PermissionReport, PermissionStatus, PillState, Platform,
    PlatformError, Sound, Status, TriggerEvent, TriggerKey, TriggerSink, clipboard,
};

static CLIPBOARD: LazyLock<clipboard::Clipboard<Vec<clipboard::Item>>> =
    LazyLock::new(clipboard::Clipboard::new);

const RESTORE_DELAY: Duration = Duration::from_millis(400);
const TAP_POLL: Duration = Duration::from_millis(20);

pub type StatusSink = Box<dyn Fn(Status) + Send>;
pub type PillSink = Box<dyn Fn(PillState) + Send>;
pub type PermissionSink = Box<dyn Fn(PermissionReport) + Send>;
pub type SoundSink = Box<dyn Fn(Sound) + Send>;
pub type MicrophoneSink = Box<dyn Fn(MicrophoneMenu) + Send>;

pub struct MacPlatform {
    status: StatusSink,
    pill: PillSink,
    permissions: PermissionSink,
    sound: SoundSink,
    microphones: MicrophoneSink,
    tap_stop: Option<Arc<AtomicBool>>,
    trigger_key: Arc<AtomicU8>,
}

impl MacPlatform {
    pub fn new(
        status: StatusSink,
        pill: PillSink,
        permissions: PermissionSink,
        sound: SoundSink,
        microphones: MicrophoneSink,
    ) -> Self {
        Self {
            status,
            pill,
            permissions,
            sound,
            microphones,
            tap_stop: None,
            trigger_key: Arc::new(AtomicU8::new(key_index(TriggerKey::default()))),
        }
    }
}

pub fn copy_text(text: &str) -> Result<(), PlatformError> {
    let mut pasteboard = pasteboard::GeneralPasteboard::new();
    CLIPBOARD
        .put_text(&mut pasteboard, text, false)
        .map(|_| ())
        .map_err(PlatformError::Copy)
}

pub fn read_text() -> Option<String> {
    pasteboard::GeneralPasteboard::new().read_text()
}

impl Platform for MacPlatform {
    fn listen_trigger(&mut self, sink: TriggerSink) -> Result<(), PlatformError> {
        self.stop_trigger();
        let (ready_tx, ready_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let tap_stop = stop.clone();
        let key = self.trigger_key.clone();
        thread::Builder::new()
            .name("trigger-key".to_owned())
            .spawn(move || run_event_tap(sink, ready_tx, tap_stop, key))
            .map_err(|error| PlatformError::TriggerKey(error.to_string()))?;
        ready_rx
            .recv()
            .map_err(|_| PlatformError::TriggerKey("event tap thread ended".to_owned()))??;
        self.tap_stop = Some(stop);
        Ok(())
    }

    fn stop_trigger(&mut self) {
        if let Some(stop) = self.tap_stop.take() {
            stop.store(true, Ordering::SeqCst);
        }
    }

    fn show_status(&mut self, status: Status) {
        (self.status)(status);
    }

    fn show_pill(&mut self, state: PillState) {
        (self.pill)(state);
    }

    fn play_sound(&mut self, sound: Sound) {
        (self.sound)(sound);
    }

    fn permission_status(&self, permission: Permission) -> PermissionStatus {
        permissions::status(permission)
    }

    fn request_permission(&mut self, permission: Permission) {
        permissions::request(permission);
    }

    fn open_settings(&mut self, permission: Permission) {
        permissions::open_settings(permission);
    }

    fn show_permissions(&mut self, report: PermissionReport) {
        (self.permissions)(report);
    }

    fn show_microphones(&mut self, menu: MicrophoneMenu) {
        (self.microphones)(menu);
    }

    fn set_trigger_key(&mut self, key: TriggerKey) {
        self.trigger_key.store(key_index(key), Ordering::SeqCst);
    }

    fn focus_probe(&mut self) -> Focus {
        focus::probe()
    }

    fn copy(&mut self, text: &str) -> Result<(), PlatformError> {
        copy_text(text)
    }

    fn paste(&mut self, text: &str, restore_clipboard: bool) -> Result<(), PlatformError> {
        let mut pasteboard = pasteboard::GeneralPasteboard::new();
        let pending = CLIPBOARD
            .put_text(&mut pasteboard, text, restore_clipboard)
            .map_err(PlatformError::Paste)?;
        send_command_v().map_err(PlatformError::Paste)?;
        if let Some(ticket) = pending {
            let spawned = thread::Builder::new()
                .name("clipboard-restore".to_owned())
                .spawn(move || {
                    thread::sleep(RESTORE_DELAY);
                    let mut pasteboard = pasteboard::GeneralPasteboard::new();
                    match CLIPBOARD.restore(&mut pasteboard, ticket) {
                        clipboard::Restore::Restored => {}
                        clipboard::Restore::Superseded => {}
                        clipboard::Restore::ClipboardChanged => {
                            tracing::info!("the clipboard changed after the paste; not restoring");
                        }
                        clipboard::Restore::Failed(error) => {
                            tracing::warn!(%error, "cannot restore the clipboard");
                        }
                    }
                });
            if let Err(error) = spawned {
                tracing::warn!(%error, "cannot schedule the clipboard restore");
            }
        }
        Ok(())
    }
}

pub fn restore_clipboard_now() {
    let mut pasteboard = pasteboard::GeneralPasteboard::new();
    if let clipboard::Restore::Failed(error) = CLIPBOARD.restore_pending(&mut pasteboard) {
        tracing::warn!(%error, "cannot restore the clipboard");
    }
}

fn key_index(key: TriggerKey) -> u8 {
    TriggerKey::ALL
        .iter()
        .position(|candidate| *candidate == key)
        .map_or(0, |index| index as u8)
}

fn key_from_index(index: u8) -> TriggerKey {
    TriggerKey::ALL
        .get(usize::from(index))
        .copied()
        .unwrap_or_default()
}

fn key_code(key: TriggerKey) -> i64 {
    match key {
        TriggerKey::RightOption => i64::from(KeyCode::RIGHT_OPTION),
        TriggerKey::RightCommand => i64::from(KeyCode::RIGHT_COMMAND),
        TriggerKey::RightControl => i64::from(KeyCode::RIGHT_CONTROL),
    }
}

fn device_mask(key: TriggerKey) -> u64 {
    match key {
        TriggerKey::RightOption => 0x40,
        TriggerKey::RightCommand => 0x10,
        TriggerKey::RightControl => 0x2000,
    }
}

fn trigger_flags(bits: u64, key: TriggerKey) -> (bool, bool) {
    (
        bits & device_mask(key) != 0,
        bits & CGEventFlags::CGEventFlagShift.bits() != 0,
    )
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

fn run_event_tap(
    sink: TriggerSink,
    ready: mpsc::Sender<Result<(), PlatformError>>,
    stop: Arc<AtomicBool>,
    key: Arc<AtomicU8>,
) {
    let sink = Arc::new(Mutex::new(sink));
    let disabled = Arc::new(AtomicBool::new(false));
    let down = Arc::new(AtomicBool::new(false));
    let callback_sink = sink.clone();
    let callback_disabled = disabled.clone();
    let callback_down = down.clone();
    let callback_key = key.clone();

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![CGEventType::FlagsChanged, CGEventType::KeyDown],
        move |_proxy, event_type, event| {
            match event_type {
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                    tracing::info!(?event_type, "macOS disabled the event tap");
                    callback_disabled.store(true, Ordering::SeqCst);
                }
                CGEventType::FlagsChanged => {
                    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                    let trigger = key_from_index(callback_key.load(Ordering::SeqCst));
                    if keycode == key_code(trigger) {
                        let (pressed, shift) = trigger_flags(event.get_flags().bits(), trigger);
                        if callback_down.swap(pressed, Ordering::SeqCst) != pressed {
                            let at = Instant::now();
                            emit(
                                &callback_sink,
                                if pressed && shift {
                                    TriggerEvent::pressed(at).with_shift()
                                } else if pressed {
                                    TriggerEvent::pressed(at)
                                } else {
                                    TriggerEvent::released(at)
                                },
                            );
                        }
                    }
                }
                CGEventType::KeyDown => {
                    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                    let at = Instant::now();
                    if keycode == i64::from(KeyCode::ESCAPE) {
                        tracing::info!("the event tap saw Esc");
                        emit(&callback_sink, TriggerEvent::escape(at));
                    } else if callback_down.load(Ordering::SeqCst) {
                        emit(&callback_sink, TriggerEvent::combined(at));
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
    raise_thread_priority();
    CFRunLoop::get_current().add_source(&source, common_modes());
    tap.enable();
    let _ = ready.send(Ok(()));

    let mut watched = key.load(Ordering::SeqCst);
    while !stop.load(Ordering::SeqCst) {
        CFRunLoop::run_in_mode(default_mode(), TAP_POLL, false);
        let current = key.load(Ordering::SeqCst);
        if current != watched {
            watched = current;
            if down.swap(false, Ordering::SeqCst) {
                emit(&sink, TriggerEvent::released(Instant::now()));
            }
        }
        if disabled.swap(false, Ordering::SeqCst) {
            tap.enable();
            tracing::info!("re-enabled the event tap");
            if down.swap(false, Ordering::SeqCst) {
                emit(&sink, TriggerEvent::released(Instant::now()));
            }
        }
    }
    if down.swap(false, Ordering::SeqCst) {
        emit(&sink, TriggerEvent::released(Instant::now()));
    }
}

#[allow(unsafe_code)]
fn raise_thread_priority() {
    let status = unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0)
    };
    if status != 0 {
        tracing::warn!(status, "cannot raise the event tap thread priority");
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

pub fn system_languages() -> Vec<String> {
    objc2_foundation::NSLocale::preferredLanguages()
        .iter()
        .map(|tag| tag.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [TriggerKey; 3] = [
        TriggerKey::RightOption,
        TriggerKey::RightCommand,
        TriggerKey::RightControl,
    ];

    #[test]
    fn shift_with_each_trigger_key_down_is_a_quick_note_press() {
        let shift = CGEventFlags::CGEventFlagShift.bits();
        for key in KEYS {
            assert_eq!(trigger_flags(device_mask(key) | shift, key), (true, true));
        }
    }

    #[test]
    fn each_trigger_key_down_without_shift_is_a_dictation_press() {
        for key in KEYS {
            assert_eq!(trigger_flags(device_mask(key), key), (true, false));
        }
    }

    #[test]
    fn releasing_each_trigger_key_is_not_pressed() {
        let shift = CGEventFlags::CGEventFlagShift.bits();
        for key in KEYS {
            assert_eq!(trigger_flags(shift, key), (false, true));
            assert_eq!(trigger_flags(0, key), (false, false));
        }
    }
}
