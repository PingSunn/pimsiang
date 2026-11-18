use std::process::Command;

use block2::RcBlock;
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use objc2::runtime::Bool;
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};
use platform::{Permission, PermissionStatus};

#[link(name = "ApplicationServices", kind = "framework")]
#[allow(unsafe_code)]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
#[allow(unsafe_code)]
unsafe extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

pub fn status(permission: Permission) -> PermissionStatus {
    match permission {
        Permission::Microphone => microphone_status(),
        Permission::Accessibility => {
            if accessibility_trusted() {
                PermissionStatus::Granted
            } else {
                PermissionStatus::Denied
            }
        }
        Permission::InputMonitoring => {
            if listen_event_granted() {
                PermissionStatus::Granted
            } else {
                PermissionStatus::NotDetermined
            }
        }
    }
}

pub fn request(permission: Permission) {
    match permission {
        Permission::Microphone => request_microphone(),
        Permission::InputMonitoring => request_listen_event(),
        Permission::Accessibility => request_accessibility(),
    }
}

pub fn pane_url(permission: Permission) -> &'static str {
    match permission {
        Permission::Microphone => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        Permission::Accessibility => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        Permission::InputMonitoring => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        }
    }
}

pub fn open_settings(permission: Permission) {
    if let Err(error) = Command::new("open").arg(pane_url(permission)).spawn() {
        tracing::warn!(%error, permission = permission.name(), "cannot open System Settings");
    }
}

#[allow(unsafe_code)]
fn accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[allow(unsafe_code)]
fn listen_event_granted() -> bool {
    unsafe { CGPreflightListenEventAccess() }
}

#[allow(unsafe_code)]
fn request_listen_event() {
    let _ = unsafe { CGRequestListenEventAccess() };
}

#[allow(unsafe_code)]
fn microphone_status() -> PermissionStatus {
    let Some(media_type) = (unsafe { AVMediaTypeAudio }) else {
        return PermissionStatus::NotDetermined;
    };
    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) };
    if status == AVAuthorizationStatus::Authorized {
        PermissionStatus::Granted
    } else if status == AVAuthorizationStatus::NotDetermined {
        PermissionStatus::NotDetermined
    } else {
        PermissionStatus::Denied
    }
}

#[allow(unsafe_code)]
fn request_microphone() {
    let Some(media_type) = (unsafe { AVMediaTypeAudio }) else {
        return;
    };
    let handler = RcBlock::new(|granted: Bool| {
        tracing::info!(
            granted = granted.as_bool(),
            "microphone permission answered"
        );
    });
    unsafe {
        AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &handler);
    }
}

#[allow(unsafe_code)]
fn request_accessibility() {
    let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
    let _ = unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };
}
