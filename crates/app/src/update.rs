use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use semver::Version;
use store::Settings;

pub const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
pub const WAKE_INTERVAL: Duration = Duration::from_secs(60 * 60);
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
pub const UPGRADE_COMMAND: &str = "brew upgrade --cask PingSunn/tap/pimsiang";
pub const GITHUB_API: &str = "https://api.github.com";
pub const REPOSITORY: &str = "PingSunn/pimsiang";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub struct Source {
    pub api_base: String,
    pub repository: String,
}

impl Source {
    pub fn github() -> Self {
        Self {
            api_base: GITHUB_API.to_owned(),
            repository: REPOSITORY.to_owned(),
        }
    }

    fn latest_url(&self) -> String {
        format!(
            "{}/repos/{}/releases/latest",
            self.api_base.trim_end_matches('/'),
            self.repository
        )
    }
}

pub fn is_newer(current: &Version, candidate: &Version) -> bool {
    candidate.pre.is_empty() && candidate > current
}

pub fn parse_release(body: &str) -> Result<Option<Version>> {
    let release: serde_json::Value = serde_json::from_str(body).context("reading the release")?;
    let flag = |name: &str| {
        release
            .get(name)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    };
    if flag("draft") || flag("prerelease") {
        return Ok(None);
    }
    let tag = release
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .context("the release has no tag")?;
    let version = parse_version(tag).context("reading the release tag")?;
    Ok((version.pre.is_empty()).then_some(version))
}

pub fn fetch_latest(source: &Source) -> Result<Option<Version>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .build()
        .into();
    let body = agent
        .get(source.latest_url())
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "PimSiang")
        .call()
        .context("asking GitHub for the latest release")?
        .body_mut()
        .read_to_string()
        .context("reading the latest release")?;
    parse_release(&body)
}

pub fn is_due(last_check: Option<u64>, now: u64) -> bool {
    match last_check {
        Some(last) if last <= now => now - last >= CHECK_INTERVAL.as_secs(),
        Some(_) | None => true,
    }
}

pub fn visible(current: &Version, settings: &Settings) -> Option<Version> {
    let available = parse_version(settings.available_update.as_deref()?)?;
    if !is_newer(current, &available) {
        return None;
    }
    match settings.dismissed_update.as_deref().and_then(parse_version) {
        Some(dismissed) if available <= dismissed => None,
        Some(_) | None => Some(available),
    }
}

fn parse_version(text: &str) -> Option<Version> {
    let text = text.trim_start_matches('v');
    let (core, rest) = match text.find(['-', '+']) {
        Some(at) => text.split_at(at),
        None => (text, ""),
    };
    let missing = 3usize.saturating_sub(core.split('.').count());
    let padded = format!("{core}{}{rest}", ".0".repeat(missing));
    Version::parse(&padded).ok()
}

pub fn check(
    source: &Source,
    settings_path: &Path,
    current: &Version,
    now: u64,
) -> Option<Version> {
    let settings = Settings::load(settings_path);
    if !is_due(settings.last_update_check, now) {
        return visible(current, &settings);
    }
    let outcome = fetch_latest(source);
    if let Err(error) = &outcome {
        tracing::info!(error = %format!("{error:#}"), "no update known");
    }
    let saved = Settings::update(settings_path, |settings| {
        settings.last_update_check = Some(now);
        if let Ok(Some(latest)) = &outcome {
            settings.available_update = is_newer(current, latest).then(|| latest.to_string());
        }
    });
    match saved {
        Ok(saved) => visible(current, &saved),
        Err(error) => {
            tracing::warn!(%error, "cannot save the update check");
            visible(current, &settings)
        }
    }
}

pub fn dismiss(settings_path: &Path) -> io::Result<()> {
    Settings::update(settings_path, |settings| {
        settings.dismissed_update = settings.available_update.clone();
    })
    .map(|_| ())
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

pub fn current_version() -> Option<Version> {
    Version::parse(env!("CARGO_PKG_VERSION")).ok()
}

pub fn spawn_checker(
    source: Source,
    settings_path: std::path::PathBuf,
    current: Version,
    first_delay: Duration,
    notify: impl Fn(Option<Version>) + Send + 'static,
) -> io::Result<()> {
    std::thread::Builder::new()
        .name("update-check".to_owned())
        .spawn(move || {
            std::thread::sleep(first_delay);
            let mut first = true;
            loop {
                let now = unix_now();
                let due = is_due(Settings::load(&settings_path).last_update_check, now);
                let found = check(&source, &settings_path, &current, now);
                if first || due {
                    notify(found);
                }
                first = false;
                std::thread::sleep(WAKE_INTERVAL);
            }
        })
        .map(|_| ())
}
