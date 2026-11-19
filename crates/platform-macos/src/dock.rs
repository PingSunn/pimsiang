use std::ffi::CString;
use std::sync::OnceLock;

use objc2::MainThreadMarker;
use objc2::ffi;
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, Sel};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

type ReopenCallback = Box<dyn Fn() + Send + Sync>;

static REOPEN: OnceLock<ReopenCallback> = OnceLock::new();

#[cfg(target_arch = "aarch64")]
const REOPEN_ENCODING: &str = "B@:@B";
#[cfg(not(target_arch = "aarch64"))]
const REOPEN_ENCODING: &str = "c@:@c";

pub fn set_visible(visible: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("the Dock icon can only change on the main thread");
        return;
    };
    let policy = if visible {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    };
    if !NSApplication::sharedApplication(mtm).setActivationPolicy(policy) {
        tracing::warn!(visible, "cannot change the activation policy");
    }
}

pub fn activate() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
}

#[allow(unsafe_code)]
extern "C" fn should_handle_reopen(
    _this: *mut AnyObject,
    _cmd: Sel,
    _app: *mut AnyObject,
    _has_visible_windows: Bool,
) -> Bool {
    if let Some(callback) = REOPEN.get() {
        callback();
    }
    Bool::NO
}

#[allow(unsafe_code)]
pub fn on_reopen(callback: impl Fn() + Send + Sync + 'static) {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("the Dock click handler can only be set on the main thread");
        return;
    };
    if REOPEN.get().is_some() {
        return;
    }
    let Some(delegate) = NSApplication::sharedApplication(mtm).delegate() else {
        tracing::warn!("the application has no delegate to receive Dock clicks");
        return;
    };
    let object: &AnyObject = delegate.as_ref();
    let class: &AnyClass = object.class();
    let types = CString::new(REOPEN_ENCODING).expect("the encoding has no NUL");
    let imp: Imp = unsafe {
        std::mem::transmute::<extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, Bool) -> Bool, Imp>(
            should_handle_reopen,
        )
    };
    let Some(selector) = (unsafe {
        ffi::sel_registerName(c"applicationShouldHandleReopen:hasVisibleWindows:".as_ptr())
    }) else {
        return;
    };
    let added = unsafe {
        ffi::class_addMethod(
            std::ptr::from_ref(class).cast_mut().cast(),
            selector,
            imp,
            types.as_ptr(),
        )
    };
    if !added.as_bool() {
        tracing::warn!("cannot add the Dock click handler");
        return;
    }
    let _ = REOPEN.set(Box::new(callback));
}
