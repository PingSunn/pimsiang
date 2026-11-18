use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use app::model_download::{
    Downloader, Error, Event, Extracted, Item, Manifest, ModelStatus, is_ready, missing_files,
    space_needed,
};
use sha2::{Digest, Sha256};

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn bytes(seed: u8, len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, content) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

type Requests = Mutex<Vec<(String, Option<String>)>>;

#[derive(Default)]
struct Behaviour {
    fail_next: AtomicUsize,
    cut_next_at: Mutex<Option<usize>>,
    ignore_range: Mutex<bool>,
    skew_resumed_range: AtomicUsize,
}

struct Server {
    base: String,
    requests: Arc<Requests>,
    behaviour: Arc<Behaviour>,
}

impl Server {
    fn start(files: Vec<(&str, Vec<u8>)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let files: Vec<(String, Vec<u8>)> = files
            .into_iter()
            .map(|(name, content)| (format!("/{name}"), content))
            .collect();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let behaviour = Arc::new(Behaviour::default());
        let (seen, rules) = (requests.clone(), behaviour.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                serve(stream, &files, &seen, &rules);
            }
        });
        Self {
            base,
            requests,
            behaviour,
        }
    }

    fn url(&self, name: &str) -> String {
        format!("{}/{name}", self.base)
    }

    fn ranges(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(_, range)| range.clone())
            .collect()
    }
}

fn serve(mut stream: TcpStream, files: &[(String, Vec<u8>)], seen: &Requests, rules: &Behaviour) {
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }
    let text = String::from_utf8_lossy(&request).into_owned();
    let path = text.split_whitespace().nth(1).unwrap_or("").to_owned();
    let range = text
        .lines()
        .find_map(|line| {
            line.strip_prefix("Range: bytes=")
                .or(line.strip_prefix("range: bytes="))
        })
        .map(str::to_owned);
    seen.lock().unwrap().push((path.clone(), range.clone()));
    if rules
        .fail_next
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
        .is_ok()
    {
        let _ =
            stream.write_all(b"HTTP/1.1 500 X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    let Some((_, content)) = files.iter().find(|(name, _)| *name == path) else {
        let _ =
            stream.write_all(b"HTTP/1.1 404 X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    };
    let ignore = *rules.ignore_range.lock().unwrap();
    let (status, from, to) = match range.as_deref().filter(|_| !ignore) {
        Some(spec) => {
            let (a, b) = spec.split_once('-').unwrap();
            let from: usize = a.parse().unwrap();
            let to = b.parse::<usize>().unwrap().min(content.len() - 1);
            ("206 Partial Content", from, to)
        }
        None => ("200 OK", 0, content.len() - 1),
    };
    let body = &content[from..=to];
    let skewed = from > 0
        && rules
            .skew_resumed_range
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok();
    let claimed = if skewed { 0 } else { from };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Range: bytes {claimed}-{to}/{}\r\nConnection: close\r\n\r\n",
        body.len(),
        content.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let cut = rules.cut_next_at.lock().unwrap().take();
    let send = cut.map_or(body.len(), |at| at.min(body.len()));
    let _ = stream.write_all(&body[..send]);
    let _ = stream.flush();
}

fn plain(server: &Server, name: &str, content: &[u8]) -> Item {
    Item {
        name: name.to_owned(),
        url: server.url(name),
        size: content.len() as u64,
        sha256: sha(content),
        unzip: Vec::new(),
    }
}

fn downloader() -> Downloader {
    Downloader {
        chunk: 1000,
        ..Downloader::default()
    }
}

fn run(dir: &Path, manifest: &Manifest) -> (Result<(), Error>, Vec<Event>) {
    let mut events = Vec::new();
    let result = downloader().run(dir, manifest, &mut |event| events.push(event));
    (result, events)
}

fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".part"))
        .collect()
}

#[test]
fn downloads_verifies_and_unzips_every_file() {
    let vad = bytes(1, 2500);
    let data = bytes(2, 4100);
    let encoder = bytes(3, 700);
    let tokens = bytes(4, 90);
    let archive = zip_of(&[("encoder.onnx", &encoder), ("tokens.txt", &tokens)]);
    let server = Server::start(vec![
        ("vad.onnx", vad.clone()),
        ("model.data", data.clone()),
        ("model.asr.zip", archive.clone()),
    ]);
    let manifest = Manifest {
        items: vec![
            plain(&server, "vad.onnx", &vad),
            Item {
                name: "model.asr.zip".to_owned(),
                url: server.url("model.asr.zip"),
                size: archive.len() as u64,
                sha256: sha(&archive),
                unzip: vec![
                    Extracted {
                        name: "encoder.onnx".to_owned(),
                        size: encoder.len() as u64,
                        sha256: sha(&encoder),
                    },
                    Extracted {
                        name: "tokens.txt".to_owned(),
                        size: tokens.len() as u64,
                        sha256: sha(&tokens),
                    },
                ],
            },
            plain(&server, "model.data", &data),
        ],
    };
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_ready(dir.path(), &manifest));

    let (result, events) = run(dir.path(), &manifest);

    result.unwrap();
    assert!(is_ready(dir.path(), &manifest));
    assert_eq!(std::fs::read(dir.path().join("vad.onnx")).unwrap(), vad);
    assert_eq!(std::fs::read(dir.path().join("model.data")).unwrap(), data);
    assert_eq!(
        std::fs::read(dir.path().join("encoder.onnx")).unwrap(),
        encoder
    );
    assert_eq!(
        std::fs::read(dir.path().join("tokens.txt")).unwrap(),
        tokens
    );
    assert!(!dir.path().join("model.asr.zip").exists());
    assert!(leftovers(dir.path()).is_empty());
    let total = manifest.total_bytes();
    assert_eq!(events.last(), Some(&Event::Progress { done: total, total }));
    assert!(events.contains(&Event::Verifying));
    let dones: Vec<u64> = events
        .iter()
        .filter_map(|event| match event {
            Event::Progress { done, .. } => Some(*done),
            Event::Verifying => None,
        })
        .collect();
    assert!(dones.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn a_finished_model_is_not_downloaded_again() {
    let content = bytes(5, 1500);
    let server = Server::start(vec![("a.bin", content.clone())]);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();
    run(dir.path(), &manifest).0.unwrap();
    let before = server.ranges().len();

    run(dir.path(), &manifest).0.unwrap();

    assert_eq!(server.ranges().len(), before);
}

#[test]
fn an_interrupted_download_resumes_where_it_stopped() {
    let content = bytes(6, 3500);
    let server = Server::start(vec![("a.bin", content.clone())]);
    *server.behaviour.cut_next_at.lock().unwrap() = Some(400);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();

    let (first, _) = run(dir.path(), &manifest);

    assert!(first.is_err());
    assert!(!dir.path().join("a.bin").exists());
    assert_eq!(
        std::fs::metadata(dir.path().join("a.bin.part"))
            .unwrap()
            .len(),
        400
    );

    let (second, _) = run(dir.path(), &manifest);

    second.unwrap();
    assert_eq!(std::fs::read(dir.path().join("a.bin")).unwrap(), content);
    assert!(
        server
            .ranges()
            .iter()
            .any(|range| range.starts_with("400-"))
    );
    assert!(
        !server
            .ranges()
            .iter()
            .skip(1)
            .any(|range| range.starts_with("0-"))
    );
}

#[test]
fn a_checksum_mismatch_is_rejected_and_never_kept() {
    let served = bytes(7, 2000);
    let mut tampered = served.clone();
    tampered[10] ^= 0xff;
    let server = Server::start(vec![("a.bin", tampered)]);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &served)],
    };
    let dir = tempfile::tempdir().unwrap();

    let (result, _) = run(dir.path(), &manifest);

    assert!(matches!(result, Err(Error::Checksum(name)) if name == "a.bin"));
    assert!(!dir.path().join("a.bin").exists());
    assert!(leftovers(dir.path()).is_empty());
    assert!(!is_ready(dir.path(), &manifest));
}

#[test]
fn a_tampered_file_inside_the_archive_is_rejected() {
    let tokens = bytes(8, 90);
    let archive = zip_of(&[("tokens.txt", &bytes(9, 90))]);
    let server = Server::start(vec![("m.zip", archive.clone())]);
    let manifest = Manifest {
        items: vec![Item {
            name: "m.zip".to_owned(),
            url: server.url("m.zip"),
            size: archive.len() as u64,
            sha256: sha(&archive),
            unzip: vec![Extracted {
                name: "tokens.txt".to_owned(),
                size: tokens.len() as u64,
                sha256: sha(&tokens),
            }],
        }],
    };
    let dir = tempfile::tempdir().unwrap();

    let (result, _) = run(dir.path(), &manifest);

    assert!(matches!(result, Err(Error::Checksum(name)) if name == "tokens.txt"));
    assert!(!dir.path().join("tokens.txt").exists());
    assert!(leftovers(dir.path()).is_empty());
}

#[test]
fn a_server_error_is_reported_and_a_retry_succeeds() {
    let content = bytes(10, 1200);
    let server = Server::start(vec![("a.bin", content.clone())]);
    server.behaviour.fail_next.store(1, Ordering::SeqCst);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();

    let (first, _) = run(dir.path(), &manifest);

    assert!(matches!(first, Err(Error::Status(500))));
    assert!(!is_ready(dir.path(), &manifest));

    run(dir.path(), &manifest).0.unwrap();
    assert!(is_ready(dir.path(), &manifest));
}

#[test]
fn an_unknown_file_is_a_status_error() {
    let content = bytes(11, 100);
    let server = Server::start(vec![]);
    let manifest = Manifest {
        items: vec![plain(&server, "gone.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();

    assert!(matches!(
        run(dir.path(), &manifest).0,
        Err(Error::Status(404))
    ));
}

#[test]
fn a_server_that_ignores_resume_requests_restarts_the_file_and_completes() {
    let content = bytes(12, 3000);
    let server = Server::start(vec![("a.bin", content.clone())]);
    *server.behaviour.cut_next_at.lock().unwrap() = Some(300);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();
    assert!(run(dir.path(), &manifest).0.is_err());
    *server.behaviour.ignore_range.lock().unwrap() = true;

    let (result, _) = run(dir.path(), &manifest);

    result.unwrap();
    assert_eq!(std::fs::read(dir.path().join("a.bin")).unwrap(), content);
    assert!(leftovers(dir.path()).is_empty());
}

#[test]
fn a_resume_answered_from_the_wrong_offset_restarts_the_file() {
    let content = bytes(13, 3500);
    let server = Server::start(vec![("a.bin", content.clone())]);
    *server.behaviour.cut_next_at.lock().unwrap() = Some(400);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();
    assert!(run(dir.path(), &manifest).0.is_err());
    server
        .behaviour
        .skew_resumed_range
        .store(1, Ordering::SeqCst);

    let (result, _) = run(dir.path(), &manifest);

    result.unwrap();
    assert_eq!(std::fs::read(dir.path().join("a.bin")).unwrap(), content);
    let ranges = server.ranges();
    let resumed = ranges.iter().position(|r| r.starts_with("400-")).unwrap();
    assert!(ranges[resumed + 1..].iter().any(|r| r.starts_with("0-")));
}

#[test]
fn a_server_that_always_misreports_the_offset_fails_instead_of_looping() {
    let content = bytes(14, 3000);
    let server = Server::start(vec![("a.bin", content.clone())]);
    server
        .behaviour
        .skew_resumed_range
        .store(usize::MAX, Ordering::SeqCst);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();

    let (result, _) = run(dir.path(), &manifest);

    assert!(matches!(result, Err(Error::RangeIgnored(_))));
}

#[test]
fn a_volume_without_room_fails_before_any_request() {
    let content = bytes(15, 3000);
    let server = Server::start(vec![("a.bin", content.clone())]);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();
    let downloader = Downloader {
        chunk: 1000,
        free_space: |_| Ok(2999),
    };

    let result = downloader.run(dir.path(), &manifest, &mut |_| {});

    assert!(matches!(
        result,
        Err(Error::NoSpace {
            needed: 3000,
            free: 2999
        })
    ));
    assert!(server.ranges().is_empty());
    assert!(result.unwrap_err().to_string().contains("พื้นที่ว่างไม่เพียงพอ"));
}

#[test]
fn space_needed_counts_only_what_is_still_missing_plus_extraction() {
    let encoder = bytes(16, 700);
    let tokens = bytes(17, 90);
    let archive = zip_of(&[("encoder.onnx", &encoder), ("tokens.txt", &tokens)]);
    let manifest = Manifest {
        items: vec![Item {
            name: "model.asr.zip".to_owned(),
            url: "http://unused.invalid/model.asr.zip".to_owned(),
            size: archive.len() as u64,
            sha256: sha(&archive),
            unzip: vec![
                Extracted {
                    name: "encoder.onnx".to_owned(),
                    size: 700,
                    sha256: sha(&encoder),
                },
                Extracted {
                    name: "tokens.txt".to_owned(),
                    size: 90,
                    sha256: sha(&tokens),
                },
            ],
        }],
    };
    let dir = tempfile::tempdir().unwrap();
    let size = archive.len() as u64;

    assert_eq!(space_needed(dir.path(), &manifest), size + 790);

    std::fs::write(dir.path().join("model.asr.zip.part"), vec![0u8; 100]).unwrap();
    assert_eq!(space_needed(dir.path(), &manifest), size - 100 + 790);

    std::fs::write(dir.path().join("encoder.onnx"), &encoder).unwrap();
    assert_eq!(space_needed(dir.path(), &manifest), size - 100 + 90);
}

#[test]
fn archive_entries_that_try_to_leave_the_folder_are_never_written() {
    let tokens = bytes(13, 90);
    let archive = zip_of(&[("../evil.txt", b"x"), ("tokens.txt", &tokens)]);
    let server = Server::start(vec![("m.zip", archive.clone())]);
    let manifest = Manifest {
        items: vec![Item {
            name: "m.zip".to_owned(),
            url: server.url("m.zip"),
            size: archive.len() as u64,
            sha256: sha(&archive),
            unzip: vec![Extracted {
                name: "tokens.txt".to_owned(),
                size: tokens.len() as u64,
                sha256: sha(&tokens),
            }],
        }],
    };
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("models");

    run(&dir, &manifest).0.unwrap();

    assert!(!root.path().join("evil.txt").exists());
    assert!(!dir.join("evil.txt").exists());
    assert_eq!(std::fs::read(dir.join("tokens.txt")).unwrap(), tokens);
}

#[test]
fn a_manifest_naming_a_path_outside_the_folder_is_refused() {
    let server = Server::start(vec![]);
    let content = bytes(14, 10);
    let mut item = plain(&server, "a.bin", &content);
    item.name = "../a.bin".to_owned();
    let dir = tempfile::tempdir().unwrap();

    let (result, _) = run(dir.path(), &Manifest { items: vec![item] });

    assert!(matches!(result, Err(Error::UnsafeName(_))));
}

#[test]
fn the_pinned_manifest_names_the_files_the_engine_loads() {
    let manifest = Manifest::pinned();
    let mut outputs: Vec<&str> = Vec::new();
    for item in &manifest.items {
        assert_eq!(item.sha256.len(), 64);
        if item.unzip.is_empty() {
            outputs.push(&item.name);
        }
        for entry in &item.unzip {
            assert_eq!(entry.sha256.len(), 64);
            outputs.push(&entry.name);
        }
        assert!(item.url.starts_with("https://"));
    }
    for name in [
        "encoder.int8.onnx",
        "encoder.int8.onnx.data",
        "decoder_joint.onnx",
        "tokens.txt",
        "silero_vad.onnx",
    ] {
        assert!(outputs.contains(&name), "{name}");
    }
    assert!(
        manifest
            .items
            .iter()
            .filter(|item| item.url.contains("huggingface.co"))
            .all(|item| item.url.contains(app::model_download::REVISION))
    );
}

#[test]
fn spawn_reports_progress_then_ready_and_failures() {
    let content = bytes(15, 1500);
    let server = Server::start(vec![("a.bin", content.clone())]);
    server.behaviour.fail_next.store(1, Ordering::SeqCst);
    let manifest = Manifest {
        items: vec![plain(&server, "a.bin", &content)],
    };
    let dir = tempfile::tempdir().unwrap();
    for expect_ready in [false, true] {
        let (tx, rx) = std::sync::mpsc::channel();
        app::model_download::spawn(dir.path().to_path_buf(), manifest.clone(), move |status| {
            let _ = tx.send(status);
        })
        .unwrap();
        let last = rx.iter().last().unwrap();
        if expect_ready {
            assert_eq!(last, ModelStatus::Ready);
        } else {
            assert!(matches!(last, ModelStatus::Failed(message) if message.contains("500")));
        }
    }
}

#[test]
fn a_custom_model_folder_without_its_files_names_what_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let files = engine::ModelFiles::new(dir.path(), engine::Precision::Int8);
    assert_eq!(
        missing_files(&files),
        [
            "encoder.int8.onnx",
            "decoder_joint.onnx",
            "silero_vad.onnx",
            "tokens.txt"
        ]
    );

    for name in ["encoder.int8.onnx", "decoder_joint.onnx", "tokens.txt"] {
        std::fs::write(dir.path().join(name), b"x").unwrap();
    }
    assert_eq!(missing_files(&files), ["silero_vad.onnx"]);

    std::fs::write(dir.path().join("silero_vad.onnx"), b"x").unwrap();
    assert!(missing_files(&files).is_empty());
}
