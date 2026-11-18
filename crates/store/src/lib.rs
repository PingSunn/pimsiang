use std::io::{self, Write};
use std::path::Path;

const MICROPHONE_KEY: &str = "microphone=";
const LAST_UPDATE_CHECK_KEY: &str = "last_update_check=";
const AVAILABLE_UPDATE_KEY: &str = "available_update=";
const DISMISSED_UPDATE_KEY: &str = "dismissed_update=";
const KEYS: [&str; 4] = [
    MICROPHONE_KEY,
    LAST_UPDATE_CHECK_KEY,
    AVAILABLE_UPDATE_KEY,
    DISMISSED_UPDATE_KEY,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub microphone: Option<String>,
    pub last_update_check: Option<u64>,
    pub available_update: Option<String>,
    pub dismissed_update: Option<String>,
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
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
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
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        for line in existing.lines().map(|line| line.trim_end_matches('\r')) {
            if !line.is_empty() && !KEYS.iter().any(|key| line.starts_with(key)) {
                text.push_str(line);
                text.push('\n');
            }
        }
        let temporary = path.with_extension("tmp");
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    }
}
