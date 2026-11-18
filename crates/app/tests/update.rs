use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use app::update::{self, CHECK_INTERVAL, Source};
use semver::Version;
use store::Settings;

const DAY: u64 = CHECK_INTERVAL.as_secs();
const NOW: u64 = 1_800_000_000;

fn version(text: &str) -> Version {
    Version::parse(text).unwrap()
}

struct Server {
    source: Source,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn start(status: u16, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let body = body.to_owned();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut buffer = [0u8; 4096];
                let Ok(read) = stream.read(&mut buffer) else {
                    continue;
                };
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
                seen.lock().unwrap().push(path);
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            source: Source {
                api_base: format!("http://127.0.0.1:{port}"),
                repository: "owner/repo".to_owned(),
            },
            requests,
        }
    }

    fn unreachable() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        Self {
            source: Source {
                api_base: format!("http://127.0.0.1:{port}"),
                repository: "owner/repo".to_owned(),
            },
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

fn settings_file() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings");
    (dir, path)
}

fn release(tag: &str) -> String {
    format!(r#"{{"tag_name":"{tag}","draft":false,"prerelease":false}}"#)
}

#[test]
fn a_higher_version_is_newer_and_an_equal_or_lower_one_is_not() {
    let current = version("0.1.0");

    assert!(update::is_newer(&current, &version("0.1.1")));
    assert!(update::is_newer(&current, &version("0.2.0")));
    assert!(update::is_newer(&current, &version("1.0.0")));
    assert!(!update::is_newer(&current, &version("0.1.0")));
    assert!(!update::is_newer(&current, &version("0.0.9")));
}

#[test]
fn versions_compare_by_number_not_by_text() {
    assert!(update::is_newer(&version("0.9.0"), &version("0.10.0")));
    assert!(update::is_newer(&version("1.2.9"), &version("1.10.0")));
}

#[test]
fn a_prerelease_is_never_newer() {
    assert!(!update::is_newer(
        &version("0.1.0"),
        &version("0.2.0-beta.1")
    ));
}

#[test]
fn a_tag_with_a_leading_v_is_read() {
    assert_eq!(
        update::parse_release(&release("v0.3.1")).unwrap(),
        Some(version("0.3.1"))
    );
}

#[test]
fn drafts_and_prereleases_are_ignored() {
    let draft = r#"{"tag_name":"v9.0.0","draft":true,"prerelease":false}"#;
    let pre = r#"{"tag_name":"v9.0.0","draft":false,"prerelease":true}"#;
    let beta = r#"{"tag_name":"v9.0.0-rc.1","draft":false,"prerelease":false}"#;

    assert_eq!(update::parse_release(draft).unwrap(), None);
    assert_eq!(update::parse_release(pre).unwrap(), None);
    assert_eq!(update::parse_release(beta).unwrap(), None);
}

#[test]
fn a_malformed_release_is_an_error() {
    assert!(update::parse_release("not json").is_err());
    assert!(update::parse_release(r#"{"draft":false}"#).is_err());
    assert!(update::parse_release(r#"{"tag_name":"latest"}"#).is_err());
}

#[test]
fn a_check_is_due_once_a_day() {
    assert!(update::is_due(None, NOW));
    assert!(!update::is_due(Some(NOW - DAY + 1), NOW));
    assert!(update::is_due(Some(NOW - DAY), NOW));
    assert!(update::is_due(Some(NOW + 5), NOW));
}

#[test]
fn a_newer_release_is_found_and_remembered() {
    let server = Server::start(200, &release("v0.2.0"));
    let (_dir, path) = settings_file();

    let found = update::check(&server.source, &path, &version("0.1.0"), NOW);

    assert_eq!(found, Some(version("0.2.0")));
    assert_eq!(server.requests(), ["/repos/owner/repo/releases/latest"]);
    let settings = Settings::load(&path);
    assert_eq!(settings.last_update_check, Some(NOW));
    assert_eq!(settings.available_update.as_deref(), Some("0.2.0"));
}

#[test]
fn the_same_version_gives_no_notice() {
    let server = Server::start(200, &release("v0.1.0"));
    let (_dir, path) = settings_file();

    assert_eq!(
        update::check(&server.source, &path, &version("0.1.0"), NOW),
        None
    );
}

#[test]
fn a_second_check_within_a_day_makes_no_request_and_keeps_the_notice() {
    let server = Server::start(200, &release("v0.2.0"));
    let (_dir, path) = settings_file();
    let current = version("0.1.0");
    update::check(&server.source, &path, &current, NOW);

    let again = update::check(&server.source, &path, &current, NOW + DAY - 1);

    assert_eq!(again, Some(version("0.2.0")));
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn the_next_day_asks_again() {
    let server = Server::start(200, &release("v0.2.0"));
    let (_dir, path) = settings_file();
    let current = version("0.1.0");
    update::check(&server.source, &path, &current, NOW);

    update::check(&server.source, &path, &current, NOW + DAY);

    assert_eq!(server.requests().len(), 2);
}

#[test]
fn a_private_repository_answering_404_is_silent_and_retried_tomorrow() {
    let server = Server::start(404, r#"{"message":"Not Found"}"#);
    let (_dir, path) = settings_file();
    let current = version("0.1.0");

    assert_eq!(update::check(&server.source, &path, &current, NOW), None);
    assert_eq!(Settings::load(&path).last_update_check, Some(NOW));
    assert_eq!(
        update::check(&server.source, &path, &current, NOW + 60),
        None
    );
    assert_eq!(server.requests().len(), 1);
    update::check(&server.source, &path, &current, NOW + DAY);
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn a_rate_limit_is_silent() {
    let server = Server::start(403, r#"{"message":"API rate limit exceeded"}"#);
    let (_dir, path) = settings_file();

    assert_eq!(
        update::check(&server.source, &path, &version("0.1.0"), NOW),
        None
    );
}

#[test]
fn an_unreachable_server_is_silent_and_keeps_an_earlier_notice() {
    let server = Server::unreachable();
    let (_dir, path) = settings_file();
    Settings {
        available_update: Some("0.2.0".to_owned()),
        last_update_check: Some(NOW - DAY),
        ..Settings::default()
    }
    .save(&path)
    .unwrap();

    let found = update::check(&server.source, &path, &version("0.1.0"), NOW);

    assert_eq!(found, Some(version("0.2.0")));
    assert_eq!(Settings::load(&path).last_update_check, Some(NOW));
}

#[test]
fn an_upgraded_app_no_longer_shows_the_old_notice() {
    let (_dir, path) = settings_file();
    Settings {
        available_update: Some("0.2.0".to_owned()),
        last_update_check: Some(NOW),
        ..Settings::default()
    }
    .save(&path)
    .unwrap();
    let server = Server::unreachable();

    assert_eq!(
        update::check(&server.source, &path, &version("0.2.0"), NOW + 1),
        None
    );
}

#[test]
fn dismissing_hides_the_notice_until_a_newer_version_appears() {
    let (_dir, path) = settings_file();
    let current = version("0.1.0");
    Settings {
        available_update: Some("0.2.0".to_owned()),
        last_update_check: Some(NOW),
        ..Settings::default()
    }
    .save(&path)
    .unwrap();
    let server = Server::unreachable();

    update::dismiss(&path).unwrap();

    assert_eq!(
        update::check(&server.source, &path, &current, NOW + 1),
        None
    );

    let newer = Server::start(200, &release("v0.3.0"));
    assert_eq!(
        update::check(&newer.source, &path, &current, NOW + DAY),
        Some(version("0.3.0"))
    );
}

#[test]
fn a_dismissal_survives_a_restart_and_the_same_version_stays_hidden() {
    let (_dir, path) = settings_file();
    let current = version("0.1.0");
    let server = Server::start(200, &release("v0.2.0"));
    update::check(&server.source, &path, &current, NOW);
    update::dismiss(&path).unwrap();

    let after_restart = update::check(&server.source, &path, &current, NOW + DAY);

    assert_eq!(after_restart, None);
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn the_default_source_is_the_github_api() {
    let source = Source::github();

    assert_eq!(source.api_base, "https://api.github.com");
    assert_eq!(source.repository, "PingSunn/pimsiang");
}
