use std::ffi::c_void;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use objc2_app_kit::NSWorkspace;
use platform::Focus;
use platform::focus::{classify, classify_missing};

const MESSAGING_TIMEOUT_SECONDS: f32 = 0.2;
const ANSWER_DEADLINE: Duration = Duration::from_millis(500);
const AX_SUCCESS: i32 = 0;
const AX_NO_VALUE: i32 = -25212;
const CHROMIUM_FRAMEWORKS: [&str; 2] = [
    "Contents/Frameworks/Electron Framework.framework",
    "Contents/Frameworks/Chromium Embedded Framework.framework",
];

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
    let focused = match attribute(&system, "AXFocusedUIElement") {
        Ok(focused) => focused,
        Err(AX_NO_VALUE) => return nothing_focused(),
        Err(_) => return Focus::Unknown,
    };
    set_timeout(&focused);
    let role = text_attribute(&focused, "AXRole");
    let subrole = text_attribute(&focused, "AXSubrole");
    let has_range = attribute(&focused, "AXSelectedTextRange").is_ok();
    let focus = classify(role.as_deref(), subrole.as_deref(), has_range);
    tracing::debug!(?role, ?subrole, has_range, ?focus, "focus probe");
    focus
}

fn nothing_focused() -> Focus {
    let front = NSWorkspace::sharedWorkspace().frontmostApplication();
    let bundle_id = front
        .as_ref()
        .and_then(|app| app.bundleIdentifier())
        .map(|id| id.to_string());
    let bundle_path = front
        .as_ref()
        .and_then(|app| app.bundleURL())
        .and_then(|url| url.path())
        .map(|path| std::path::PathBuf::from(path.to_string()));
    let chromium = bundle_path.is_some_and(|path| {
        CHROMIUM_FRAMEWORKS
            .iter()
            .any(|framework| path.join(framework).exists())
    });
    let focus = classify_missing(bundle_id.as_deref(), chromium);
    tracing::debug!(?bundle_id, chromium, ?focus, "focus probe, nothing focused");
    focus
}

fn text_attribute(element: &CFType, name: &str) -> Option<String> {
    attribute(element, name)
        .ok()
        .and_then(|value| value.downcast::<CFString>())
        .map(|text| text.to_string())
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
fn attribute(element: &CFType, name: &str) -> Result<CFType, i32> {
    let name = CFString::new(name);
    let mut value: CFTypeRef = std::ptr::null();
    let status = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            &mut value,
        )
    };
    if status != AX_SUCCESS {
        return Err(status);
    }
    if value.is_null() {
        return Err(AX_NO_VALUE);
    }
    Ok(unsafe { CFType::wrap_under_create_rule(value) })
}
