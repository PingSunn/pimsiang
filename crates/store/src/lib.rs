mod history;
mod service;

pub use history::{Entry, History, HistoryError, Kind, SCHEMA_VERSION};
pub use service::HistoryService;

use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static WRITE_LOCK: Mutex<()> = Mutex::new(());
static WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

const MICROPHONE_KEY: &str = "microphone=";
const LAST_UPDATE_CHECK_KEY: &str = "last_update_check=";
const AVAILABLE_UPDATE_KEY: &str = "available_update=";
const DISMISSED_UPDATE_KEY: &str = "dismissed_update=";
const TRIGGER_KEY_KEY: &str = "trigger_key=";
const LANGUAGE_KEY: &str = "language=";
const SOUNDS_KEY: &str = "sounds=";
const CLIPBOARD_RESTORE_KEY: &str = "clipboard_restore=";
const LAUNCH_AT_LOGIN_KEY: &str = "launch_at_login=";
const LOGIN_ITEM_DECIDED_KEY: &str = "login_item_decided=";
const UNLOAD_WHEN_IDLE_KEY: &str = "unload_when_idle=";
const KEYS: [&str; 11] = [
    MICROPHONE_KEY,
    LAST_UPDATE_CHECK_KEY,
    AVAILABLE_UPDATE_KEY,
    DISMISSED_UPDATE_KEY,
    TRIGGER_KEY_KEY,
    LANGUAGE_KEY,
    SOUNDS_KEY,
    CLIPBOARD_RESTORE_KEY,
    LAUNCH_AT_LOGIN_KEY,
    LOGIN_ITEM_DECIDED_KEY,
    UNLOAD_WHEN_IDLE_KEY,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub microphone: Option<String>,
    pub last_update_check: Option<u64>,
    pub available_update: Option<String>,
    pub dismissed_update: Option<String>,
    pub trigger_key: Option<String>,
    pub language: Option<String>,
    pub sounds: Option<bool>,
    pub clipboard_restore: Option<bool>,
    pub launch_at_login: Option<bool>,
    pub login_item_decided: Option<bool>,
    pub unload_when_idle: Option<bool>,
}

fn switch(text: &str, key: &str) -> Option<bool> {
    match value(text, key)? {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

fn switch_text(value: Option<bool>) -> Option<&'static str> {
    value.map(|on| if on { "on" } else { "off" })
}

fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .map(|line| line.trim_end_matches('\r'))
        .find_map(|line| line.strip_prefix(key))
        .filter(|value| !value.is_empty())
}

fn push(text: &mut String, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.contains('\n')) {
        text.push_str(key);
        text.push_str(value);
        text.push('\n');
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        Self {
            microphone: value(&text, MICROPHONE_KEY).map(str::to_owned),
            last_update_check: value(&text, LAST_UPDATE_CHECK_KEY)
                .and_then(|seconds| seconds.parse().ok()),
            available_update: value(&text, AVAILABLE_UPDATE_KEY).map(str::to_owned),
            dismissed_update: value(&text, DISMISSED_UPDATE_KEY).map(str::to_owned),
            trigger_key: value(&text, TRIGGER_KEY_KEY).map(str::to_owned),
            language: value(&text, LANGUAGE_KEY).map(str::to_owned),
            sounds: switch(&text, SOUNDS_KEY),
            clipboard_restore: switch(&text, CLIPBOARD_RESTORE_KEY),
            launch_at_login: switch(&text, LAUNCH_AT_LOGIN_KEY),
            login_item_decided: switch(&text, LOGIN_ITEM_DECIDED_KEY),
            unload_when_idle: switch(&text, UNLOAD_WHEN_IDLE_KEY),
        }
    }

    pub fn sounds(&self) -> bool {
        self.sounds.unwrap_or(true)
    }

    pub fn clipboard_restore(&self) -> bool {
        self.clipboard_restore.unwrap_or(true)
    }

    pub fn launch_at_login(&self) -> bool {
        self.launch_at_login.unwrap_or(true)
    }

    pub fn unload_when_idle(&self) -> bool {
        self.unload_when_idle.unwrap_or(true)
    }

    pub fn login_item_decided(&self) -> bool {
        self.login_item_decided.unwrap_or(false)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let _guard = WRITE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.write(path)
    }

    pub fn update(path: &Path, change: impl FnOnce(&mut Self)) -> io::Result<Self> {
        let _guard = WRITE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut settings = Self::load(path);
        change(&mut settings);
        settings.write(path)?;
        Ok(settings)
    }

    fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut text = String::new();
        push(&mut text, MICROPHONE_KEY, self.microphone.as_deref());
        push(
            &mut text,
            LAST_UPDATE_CHECK_KEY,
            self.last_update_check.map(|at| at.to_string()).as_deref(),
        );
        push(
            &mut text,
            AVAILABLE_UPDATE_KEY,
            self.available_update.as_deref(),
        );
        push(
            &mut text,
            DISMISSED_UPDATE_KEY,
            self.dismissed_update.as_deref(),
        );
        push(&mut text, TRIGGER_KEY_KEY, self.trigger_key.as_deref());
        push(&mut text, LANGUAGE_KEY, self.language.as_deref());
        push(&mut text, SOUNDS_KEY, switch_text(self.sounds));
        push(
            &mut text,
            CLIPBOARD_RESTORE_KEY,
            switch_text(self.clipboard_restore),
        );
        push(
            &mut text,
            LAUNCH_AT_LOGIN_KEY,
            switch_text(self.launch_at_login),
        );
        push(
            &mut text,
            LOGIN_ITEM_DECIDED_KEY,
            switch_text(self.login_item_decided),
        );
        push(
            &mut text,
            UNLOAD_WHEN_IDLE_KEY,
            switch_text(self.unload_when_idle),
        );
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        for line in existing.lines().map(|line| line.trim_end_matches('\r')) {
            if !line.is_empty() && !KEYS.iter().any(|key| line.starts_with(key)) {
                text.push_str(line);
                text.push('\n');
            }
        }
        let temporary = path.with_extension(format!(
            "{}.{}.tmp",
            std::process::id(),
            WRITE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    }
}
