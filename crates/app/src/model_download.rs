use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use engine::Precision;
use sha2::{Digest, Sha256};

pub const REVISION: &str = "9f142a43e4d35a422c1c7dac40772ef707387ebc";
const MODEL_BASE: &str = "https://huggingface.co/iapp/OpenThai2.0-ASR-Streaming-ONNX/resolve";
const VAD_URL: &str = "https://raw.githubusercontent.com/snakers4/silero-vad/6478567951ae5c9979ad7b234185b5515f4be7a1/src/silero_vad/data/silero_vad.onnx";

const CHUNK: u64 = 8 * 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const BODY_TIMEOUT: Duration = Duration::from_secs(120);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
const BUFFER: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the server answered {0}")]
    Status(u16),
    #[error("network: {0}")]
    Network(String),
    #[error("the server ignored the resume request for {0}")]
    RangeIgnored(String),
    #[error(
        "not enough free disk space: {} needed, {} free / พื้นที่ว่างไม่เพียงพอ: ต้องการ {} เหลือ {}",
        mib(*needed), mib(*free), mib(*needed), mib(*free)
    )]
    NoSpace { needed: u64, free: u64 },
    #[error("{0} ended before its expected size")]
    Incomplete(String),
    #[error("{0} does not match its pinned checksum")]
    Checksum(String),
    #[error("`{0}` is not a plain file name")]
    UnsafeName(String),
    #[error("archive: {0}")]
    Archive(String),
    #[error("the download was cancelled")]
    Cancelled,
    #[error("disk: {0}")]
    Io(#[from] io::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extracted {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    pub unzip: Vec<Extracted>,
}

impl Item {
    fn outputs(&self) -> Vec<(&str, u64)> {
        if self.unzip.is_empty() {
            vec![(self.name.as_str(), self.size)]
        } else {
            self.unzip
                .iter()
                .map(|entry| (entry.name.as_str(), entry.size))
                .collect()
        }
    }

    fn present(&self, dir: &Path) -> bool {
        self.outputs().into_iter().all(|(name, size)| {
            fs::metadata(dir.join(name)).is_ok_and(|meta| meta.is_file() && meta.len() == size)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub items: Vec<Item>,
}

struct Build {
    zip: &'static str,
    zip_size: u64,
    zip_sha256: &'static str,
    encoder: &'static str,
    encoder_size: u64,
    encoder_sha256: &'static str,
    data: &'static str,
    data_size: u64,
    data_sha256: &'static str,
}

const INT8: Build = Build {
    zip: "openthai2.0-asr-streaming-int8.asr.zip",
    zip_size: 109_014_005,
    zip_sha256: "6714a3333dbbef388aab0a0623a73da19953d9473dc59b98a77dda1a48911553",
    encoder: "encoder.int8.onnx",
    encoder_size: 1_589_571,
    encoder_sha256: "b8076c61921444977d4e290a76ad3f1b30078ba939117540306441ffe3cb7462",
    data: "encoder.int8.onnx.data",
    data_size: 883_175_424,
    data_sha256: "39176685a17087bc45a2a002cdc760951ccc4781693780a59db2b2a05ae964a5",
};

const FP32: Build = Build {
    zip: "openthai2.0-asr-streaming-fp32.asr.zip",
    zip_size: 149_365_631,
    zip_sha256: "d4161d14e1c61c779ae01cf02356a99d2752f7931ce2d04be8098c57270d463f",
    encoder: "encoder.onnx",
    encoder_size: 41_941_207,
    encoder_sha256: "0bc1e7e79f1ae78124d610f93584ad319f01d1f771479ba44dd2c433973115be",
    data: "encoder.onnx.data",
    data_size: 2_454_405_120,
    data_sha256: "a139201333ab89d5e965bc12abb1c534aa285d82e40051c2dda605365c74f18b",
};

impl Manifest {
    pub fn pinned(precision: Precision) -> Self {
        let build = match precision {
            Precision::Int8 => &INT8,
            Precision::Fp32 => &FP32,
        };
        let revision = format!("{MODEL_BASE}/{REVISION}");
        Self {
            items: vec![
                Item {
                    name: "silero_vad.onnx".to_owned(),
                    url: VAD_URL.to_owned(),
                    size: 2_327_524,
                    sha256: "2623a2953f6ff3d2c1e61740c6cdb7168133479b267dfef114a4a3cc5bdd788f"
                        .to_owned(),
                    unzip: Vec::new(),
                },
                Item {
                    name: build.zip.to_owned(),
                    url: format!("{revision}/{}", build.zip),
                    size: build.zip_size,
                    sha256: build.zip_sha256.to_owned(),
                    unzip: vec![
                        Extracted {
                            name: build.encoder.to_owned(),
                            size: build.encoder_size,
                            sha256: build.encoder_sha256.to_owned(),
                        },
                        Extracted {
                            name: "decoder_joint.onnx".to_owned(),
                            size: 107_330_778,
                            sha256:
                                "7b330cc00b1a65aa702ab4150e16411615a17c3765f274ecd811b1ea7edeacfe"
                                    .to_owned(),
                        },
                        Extracted {
                            name: "tokens.txt".to_owned(),
                            size: 93_160,
                            sha256:
                                "0936fbd4f4f0e273328fd6856c8a0a105c5b723cc71dfbf4071e388fd34f9240"
                                    .to_owned(),
                        },
                    ],
                },
                Item {
                    name: build.data.to_owned(),
                    url: format!("{revision}/{}", build.data),
                    size: build.data_size,
                    sha256: build.data_sha256.to_owned(),
                    unzip: Vec::new(),
                },
            ],
        }
    }

    pub fn total_bytes(&self) -> u64 {
        self.items.iter().map(|item| item.size).sum()
    }
}

pub fn is_ready(dir: &Path, manifest: &Manifest) -> bool {
    manifest.items.iter().all(|item| item.present(dir))
}

pub fn missing_files(files: &engine::ModelFiles) -> Vec<String> {
    [
        files.encoder(),
        files.decoder_joint(),
        files.vad(),
        files.tokens(),
    ]
    .into_iter()
    .filter(|path| !path.is_file())
    .filter_map(|path| Some(path.file_name()?.to_string_lossy().into_owned()))
    .collect()
}

pub fn build_of(precision: store::Precision) -> Precision {
    match precision {
        store::Precision::Accurate => Precision::Fp32,
        store::Precision::Light => Precision::Int8,
    }
}

pub fn download_bytes(precision: Precision) -> u64 {
    Manifest::pinned(precision).total_bytes()
}

pub fn models_root(home: &Path) -> PathBuf {
    home.join("Library/Application Support/PimSiang/models")
}

pub fn build_dir(root: &Path, precision: Precision) -> PathBuf {
    root.join(format!("{}-{}", precision.name(), &REVISION[..8]))
}

pub fn default_dir(home: &Path, precision: Precision) -> PathBuf {
    build_dir(&models_root(home), precision)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Progress { done: u64, total: u64 },
    Verifying,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelStatus {
    Downloading { done: u64, total: u64 },
    Verifying,
    Failed(String),
    Ready,
}

pub struct Downloader {
    pub chunk: u64,
    pub free_space: fn(&Path) -> io::Result<u64>,
    pub cancel: Option<Arc<AtomicBool>>,
}

impl Default for Downloader {
    fn default() -> Self {
        Self {
            chunk: CHUNK,
            free_space,
            cancel: None,
        }
    }
}

fn mib(bytes: u64) -> String {
    format!("{} MB", bytes.div_ceil(1024 * 1024))
}

fn free_space(dir: &Path) -> io::Result<u64> {
    let stat = rustix::fs::statvfs(dir)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}

pub fn space_needed(dir: &Path, manifest: &Manifest) -> u64 {
    manifest
        .items
        .iter()
        .filter(|item| !item.present(dir))
        .map(|item| {
            let archive_ready =
                fs::metadata(dir.join(&item.name)).is_ok_and(|meta| meta.len() == item.size);
            let download = if archive_ready {
                0
            } else {
                let have = fs::metadata(dir.join(format!("{}.part", item.name)))
                    .map_or(0, |meta| meta.len());
                item.size.saturating_sub(have.min(item.size))
            };
            let extract: u64 = item
                .unzip
                .iter()
                .filter(|entry| {
                    !fs::metadata(dir.join(&entry.name)).is_ok_and(|meta| meta.len() == entry.size)
                })
                .map(|entry| entry.size)
                .sum();
            download + extract
        })
        .sum()
}

struct Tracker<'a> {
    done: u64,
    total: u64,
    last: Option<Instant>,
    emit: &'a mut dyn FnMut(Event),
}

impl Tracker<'_> {
    fn set(&mut self, done: u64, force: bool) {
        self.done = done;
        let due = self
            .last
            .is_none_or(|last| last.elapsed() >= PROGRESS_INTERVAL);
        if force || due {
            self.last = Some(Instant::now());
            (self.emit)(Event::Progress {
                done: self.done,
                total: self.total,
            });
        }
    }

    fn verifying(&mut self) {
        (self.emit)(Event::Verifying);
    }
}

impl Downloader {
    fn cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    pub fn run(
        &self,
        dir: &Path,
        manifest: &Manifest,
        emit: &mut dyn FnMut(Event),
    ) -> Result<(), Error> {
        for item in &manifest.items {
            safe_name(&item.name)?;
            for entry in &item.unzip {
                safe_name(&entry.name)?;
            }
        }
        fs::create_dir_all(dir)?;
        let needed = space_needed(dir, manifest);
        let free = (self.free_space)(dir)?;
        if free < needed {
            return Err(Error::NoSpace { needed, free });
        }
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT))
            .timeout_recv_body(Some(BODY_TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        let mut tracker = Tracker {
            done: 0,
            total: manifest.total_bytes(),
            last: None,
            emit,
        };
        let mut finished = 0;
        tracker.set(0, true);
        for item in &manifest.items {
            if self.cancelled() {
                return Err(Error::Cancelled);
            }
            if !item.present(dir) {
                self.fetch_item(&agent, dir, item, finished, &mut tracker)?;
            }
            finished += item.size;
            tracker.set(finished, true);
        }
        Ok(())
    }

    fn fetch_item(
        &self,
        agent: &ureq::Agent,
        dir: &Path,
        item: &Item,
        finished: u64,
        tracker: &mut Tracker<'_>,
    ) -> Result<(), Error> {
        let target = dir.join(&item.name);
        let has_archive = fs::metadata(&target).is_ok_and(|meta| meta.len() == item.size);
        if !has_archive {
            let part = dir.join(format!("{}.part", item.name));
            self.download(agent, item, &part, finished, tracker)?;
            tracker.verifying();
            if sha256_file(&part)? != item.sha256.to_ascii_lowercase() {
                let _ = fs::remove_file(&part);
                return Err(Error::Checksum(item.name.clone()));
            }
            fs::rename(&part, &target)?;
        }
        if !item.unzip.is_empty() {
            tracker.verifying();
            extract(&target, dir, &item.unzip)?;
            fs::remove_file(&target)?;
        }
        Ok(())
    }

    fn download(
        &self,
        agent: &ureq::Agent,
        item: &Item,
        part: &Path,
        finished: u64,
        tracker: &mut Tracker<'_>,
    ) -> Result<(), Error> {
        let mut have = fs::metadata(part).map(|meta| meta.len()).unwrap_or(0);
        if have > item.size {
            fs::remove_file(part)?;
            have = 0;
        }
        let mut restarted = false;
        tracker.set(finished + have, true);
        let mut buffer = vec![0u8; BUFFER];
        while have < item.size {
            if self.cancelled() {
                return Err(Error::Cancelled);
            }
            let end = (have + self.chunk).min(item.size) - 1;
            let mut response = agent
                .get(&item.url)
                .header("Range", format!("bytes={have}-{end}"))
                .header("User-Agent", "PimSiang")
                .call()
                .map_err(|error| Error::Network(error.to_string()))?;
            let mut wanted = end + 1 - have;
            match response.status().as_u16() {
                206 => {
                    let start = response
                        .headers()
                        .get("Content-Range")
                        .and_then(|value| value.to_str().ok())
                        .and_then(content_range_start);
                    if start != Some(have) {
                        if restarted || have == 0 {
                            return Err(Error::RangeIgnored(item.name.clone()));
                        }
                        restarted = true;
                        File::create(part)?;
                        have = 0;
                        tracker.set(finished, true);
                        continue;
                    }
                }
                200 => {
                    if have > 0 {
                        if restarted {
                            return Err(Error::RangeIgnored(item.name.clone()));
                        }
                        restarted = true;
                        File::create(part)?;
                        have = 0;
                        tracker.set(finished, true);
                    }
                    wanted = item.size;
                }
                other => return Err(Error::Status(other)),
            }
            let mut file = OpenOptions::new().create(true).append(true).open(part)?;
            let mut reader = response.body_mut().as_reader().take(wanted);
            let mut got = 0u64;
            loop {
                let read = reader
                    .read(&mut buffer)
                    .map_err(|error| Error::Network(error.to_string()))?;
                if read == 0 {
                    break;
                }
                if self.cancelled() {
                    return Err(Error::Cancelled);
                }
                file.write_all(&buffer[..read])?;
                got += read as u64;
                tracker.set(finished + have + got, false);
            }
            file.sync_data()?;
            have += got;
            if got < wanted {
                return Err(Error::Incomplete(item.name.clone()));
            }
        }
        Ok(())
    }
}

fn content_range_start(value: &str) -> Option<u64> {
    value
        .strip_prefix("bytes ")?
        .split_once('-')?
        .0
        .trim()
        .parse()
        .ok()
}

fn safe_name(name: &str) -> Result<(), Error> {
    let plain =
        !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0']);
    if plain {
        Ok(())
    } else {
        Err(Error::UnsafeName(name.to_owned()))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            return Ok(hex(&hasher.finalize()));
        }
        hasher.update(&buffer[..read]);
    }
}

fn extract(archive: &Path, dir: &Path, entries: &[Extracted]) -> Result<(), Error> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?)
        .map_err(|error| Error::Archive(error.to_string()))?;
    for expected in entries {
        let target = dir.join(&expected.name);
        if fs::metadata(&target).is_ok_and(|meta| meta.len() == expected.size) {
            continue;
        }
        let mut entry = zip
            .by_name(&expected.name)
            .map_err(|error| Error::Archive(format!("{}: {error}", expected.name)))?;
        let part = dir.join(format!("{}.part", expected.name));
        let mut file = File::create(&part)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; BUFFER];
        let mut written = 0u64;
        loop {
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            file.write_all(&buffer[..read])?;
            written += read as u64;
        }
        file.sync_all()?;
        drop(file);
        if written != expected.size
            || hex(&hasher.finalize()) != expected.sha256.to_ascii_lowercase()
        {
            let _ = fs::remove_file(&part);
            return Err(Error::Checksum(expected.name.clone()));
        }
        fs::rename(&part, &target)?;
    }
    Ok(())
}

pub fn spawn(
    dir: PathBuf,
    manifest: Manifest,
    notify: impl Fn(ModelStatus) + Send + 'static,
) -> io::Result<()> {
    spawn_cancellable(dir, manifest, Arc::new(AtomicBool::new(false)), notify)
}

pub fn spawn_cancellable(
    dir: PathBuf,
    manifest: Manifest,
    cancel: Arc<AtomicBool>,
    notify: impl Fn(ModelStatus) + Send + 'static,
) -> io::Result<()> {
    std::thread::Builder::new()
        .name("model-download".to_owned())
        .spawn(move || {
            let mut emit = |event| {
                notify(match event {
                    Event::Progress { done, total } => ModelStatus::Downloading { done, total },
                    Event::Verifying => ModelStatus::Verifying,
                });
            };
            let downloader = Downloader {
                cancel: Some(cancel),
                ..Downloader::default()
            };
            match downloader.run(&dir, &manifest, &mut emit) {
                Ok(()) => notify(ModelStatus::Ready),
                Err(Error::Cancelled) => tracing::info!("the Model download was cancelled"),
                Err(error) => {
                    tracing::error!(%error, "cannot download the Model");
                    notify(ModelStatus::Failed(error.to_string()));
                }
            }
        })
        .map(|_| ())
}
