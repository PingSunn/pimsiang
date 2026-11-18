use std::ffi::c_void;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use platform::Focus;
use platform::focus::classify;

const MESSAGING_TIMEOUT_SECONDS: f32 = 0.2;
const ANSWER_DEADLINE: Duration = Duration::from_millis(500);
const AX_SUCCESS: i32 = 0;

type AxElement = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
#[allow(unsafe_code)]
unsafe extern "C" {
    fn AXUIElementCreateSystemWide() -> AxElement;
    fn AXUIElementSetMessagingTimeout(element: AxElement, seconds: f32) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: AxElement,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
}

pub fn probe() -> Focus {
    let (answer_tx, answer_rx) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("focus-probe".to_owned())
        .spawn(move || {
            let _ = answer_tx.send(ask());
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "cannot start the focus probe");
        return Focus::Unknown;
    }
    answer_rx.recv_timeout(ANSWER_DEADLINE).unwrap_or_else(|_| {
        tracing::warn!("the focus probe did not answer in time");
        Focus::Unknown
    })
}

fn ask() -> Focus {
    let Some(system) = system_wide() else {
        return Focus::Unknown;
    };
    set_timeout(&system);
    let Some(focused) = attribute(&system, "AXFocusedUIElement") else {
        return Focus::Unknown;
    };
    set_timeout(&focused);
    let role = attribute(&focused, "AXRole")
        .and_then(|value| value.downcast::<CFString>())
        .map(|role| role.to_string());
    let has_range = attribute(&focused, "AXSelectedTextRange").is_some();
    let focus = classify(role.as_deref(), has_range);
    tracing::debug!(?role, has_range, ?focus, "focus probe");
    focus
}

#[allow(unsafe_code)]
fn system_wide() -> Option<CFType> {
    let element = unsafe { AXUIElementCreateSystemWide() };
    if element.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(element) })
}

#[allow(unsafe_code)]
fn set_timeout(element: &CFType) {
    unsafe {
        AXUIElementSetMessagingTimeout(element.as_CFTypeRef(), MESSAGING_TIMEOUT_SECONDS);
    }
}

#[allow(unsafe_code)]
fn attribute(element: &CFType, name: &str) -> Option<CFType> {
    let name = CFString::new(name);
    let mut value: CFTypeRef = std::ptr::null();
    let status = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            &mut value,
        )
    };
    if status != AX_SUCCESS || value.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(value) })
}
