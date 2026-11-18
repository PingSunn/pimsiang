use objc2_service_management::{SMAppService, SMAppServiceStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginItem {
    Enabled,
    RequiresApproval,
    Disabled,
    Unavailable,
}

#[allow(unsafe_code)]
pub fn status() -> LoginItem {
    let status = unsafe { SMAppService::mainAppService().status() };
    if status == SMAppServiceStatus::Enabled {
        LoginItem::Enabled
    } else if status == SMAppServiceStatus::RequiresApproval {
        LoginItem::RequiresApproval
    } else if status == SMAppServiceStatus::NotRegistered {
        LoginItem::Disabled
    } else {
        LoginItem::Unavailable
    }
}

#[allow(unsafe_code)]
pub fn set(enabled: bool) -> Result<(), String> {
    let service = unsafe { SMAppService::mainAppService() };
    let outcome = unsafe {
        if enabled {
            service.registerAndReturnError()
        } else {
            service.unregisterAndReturnError()
        }
    };
    outcome.map_err(|error| error.localizedDescription().to_string())
}

#[allow(unsafe_code)]
pub fn open_login_items_settings() {
    unsafe { SMAppService::openSystemSettingsLoginItems() };
}
