use std::io::{self, Write};
use std::path::Path;

const MICROPHONE_KEY: &str = "microphone=";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub microphone: Option<String>,
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let microphone = text
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .find_map(|line| line.strip_prefix(MICROPHONE_KEY))
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
        Self { microphone }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut text = String::new();
        if let Some(id) = self.microphone.as_deref().filter(|id| !id.contains('\n')) {
            text.push_str(MICROPHONE_KEY);
            text.push_str(id);
            text.push('\n');
        }
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        for line in existing.lines().map(|line| line.trim_end_matches('\r')) {
            if !line.is_empty() && !line.starts_with(MICROPHONE_KEY) {
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
