use crate::{
    MicrophoneMenu, Permission, PermissionReport, PermissionStatus, PillState, Platform,
    PlatformError, Sound, Status, TriggerEvent, TriggerKey, TriggerSink,
};

pub struct FakePlatform {
    sink: Option<TriggerSink>,
    statuses: Vec<Status>,
    pills: Vec<PillState>,
    pastes: Vec<String>,
    paste_restores: Vec<bool>,
    trigger_key: TriggerKey,
    trigger_key_changes: usize,
    sounds: Vec<Sound>,
    paste_fails: bool,
    permissions: PermissionReport,
    reports: Vec<PermissionReport>,
    microphones: Vec<MicrophoneMenu>,
    requested: Vec<Permission>,
    opened: Vec<Permission>,
    listen_fails: bool,
    listen_attempts: usize,
}

impl Default for FakePlatform {
    fn default() -> Self {
        Self {
            sink: None,
            statuses: Vec::new(),
            pills: Vec::new(),
            pastes: Vec::new(),
            paste_restores: Vec::new(),
            trigger_key: TriggerKey::default(),
            trigger_key_changes: 0,
            sounds: Vec::new(),
            paste_fails: false,
            permissions: PermissionReport {
                microphone: PermissionStatus::Granted,
                accessibility: PermissionStatus::Granted,
                input_monitoring: PermissionStatus::Granted,
            },
            reports: Vec::new(),
            microphones: Vec::new(),
            requested: Vec::new(),
            opened: Vec::new(),
            listen_fails: false,
            listen_attempts: 0,
        }
    }
}

impl FakePlatform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_permission(mut self, permission: Permission, status: PermissionStatus) -> Self {
        self.set_permission(permission, status);
        self
    }

    pub fn failing_listen(mut self) -> Self {
        self.listen_fails = true;
        self
    }

    pub fn set_permission(&mut self, permission: Permission, status: PermissionStatus) {
        match permission {
            Permission::Microphone => self.permissions.microphone = status,
            Permission::Accessibility => self.permissions.accessibility = status,
            Permission::InputMonitoring => self.permissions.input_monitoring = status,
        }
    }

    pub fn is_listening(&self) -> bool {
        self.sink.is_some()
    }

    pub fn listen_attempts(&self) -> usize {
        self.listen_attempts
    }

    pub fn reports(&self) -> &[PermissionReport] {
        &self.reports
    }

    pub fn microphones(&self) -> &[MicrophoneMenu] {
        &self.microphones
    }

    pub fn requested(&self) -> &[Permission] {
        &self.requested
    }

    pub fn opened(&self) -> &[Permission] {
        &self.opened
    }

    pub fn failing_paste(mut self) -> Self {
        self.paste_fails = true;
        self
    }

    pub fn emit(&mut self, event: TriggerEvent) -> bool {
        match self.sink.as_mut() {
            Some(sink) => {
                sink(event);
                true
            }
            None => false,
        }
    }

    pub fn statuses(&self) -> &[Status] {
        &self.statuses
    }

    pub fn pills(&self) -> &[PillState] {
        &self.pills
    }

    pub fn pastes(&self) -> &[String] {
        &self.pastes
    }

    pub fn paste_restores(&self) -> &[bool] {
        &self.paste_restores
    }

    pub fn trigger_key(&self) -> TriggerKey {
        self.trigger_key
    }

    pub fn trigger_key_changes(&self) -> usize {
        self.trigger_key_changes
    }

    pub fn sounds(&self) -> &[Sound] {
        &self.sounds
    }
}

impl Platform for FakePlatform {
    fn listen_trigger(&mut self, sink: TriggerSink) -> Result<(), PlatformError> {
        self.listen_attempts += 1;
        if self.listen_fails {
            return Err(PlatformError::TriggerKey("scripted failure".to_owned()));
        }
        self.sink = Some(sink);
        Ok(())
    }

    fn stop_trigger(&mut self) {
        self.sink = None;
    }

    fn show_status(&mut self, status: Status) {
        self.statuses.push(status);
    }

    fn show_pill(&mut self, state: PillState) {
        self.pills.push(state);
    }

    fn play_sound(&mut self, sound: Sound) {
        self.sounds.push(sound);
    }

    fn set_trigger_key(&mut self, key: TriggerKey) {
        self.trigger_key = key;
        self.trigger_key_changes += 1;
    }

    fn paste(&mut self, text: &str, restore_clipboard: bool) -> Result<(), PlatformError> {
        if self.paste_fails {
            return Err(PlatformError::Paste("scripted failure".to_owned()));
        }
        self.pastes.push(text.to_owned());
        self.paste_restores.push(restore_clipboard);
        Ok(())
    }

    fn permission_status(&self, permission: Permission) -> PermissionStatus {
        self.permissions.status(permission)
    }

    fn request_permission(&mut self, permission: Permission) {
        self.requested.push(permission);
    }

    fn open_settings(&mut self, permission: Permission) {
        self.opened.push(permission);
    }

    fn show_permissions(&mut self, report: PermissionReport) {
        self.reports.push(report);
    }

    fn show_microphones(&mut self, menu: MicrophoneMenu) {
        self.microphones.push(menu);
    }
}
