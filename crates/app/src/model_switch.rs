use std::path::Path;

use engine::Precision;

use crate::model_download::{self, Manifest, ModelStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    Absent,
    Partial,
    Complete,
}

pub fn presence(root: &Path, precision: Precision) -> Presence {
    let dir = model_download::build_dir(root, precision);
    if !dir.exists() {
        Presence::Absent
    } else if model_download::is_ready(&dir, &Manifest::pinned(precision)) {
        Presence::Complete
    } else {
        Presence::Partial
    }
}

pub fn delete_build(root: &Path, precision: Precision) -> std::io::Result<()> {
    match std::fs::remove_dir_all(model_download::build_dir(root, precision)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Download { id: u64, precision: Precision },
    Cancel { id: u64 },
    Use(Precision),
    Ready,
    Delete(Precision),
    Report(ModelStatus),
}

#[derive(Clone, Copy, Debug)]
struct Job {
    id: u64,
    precision: Precision,
}

#[derive(Debug)]
pub struct Switcher {
    wanted: Precision,
    engine_files: Precision,
    using: Option<Precision>,
    requested: Option<Precision>,
    job: Option<Job>,
    next_id: u64,
}

fn other(precision: Precision) -> Precision {
    match precision {
        Precision::Fp32 => Precision::Int8,
        Precision::Int8 => Precision::Fp32,
    }
}

impl Switcher {
    pub fn start(wanted: Precision, disk: &dyn Fn(Precision) -> Presence) -> (Self, Vec<Command>) {
        let mut switcher = Self {
            wanted,
            engine_files: wanted,
            using: None,
            requested: None,
            job: None,
            next_id: 0,
        };
        let mut commands = Vec::new();
        if disk(wanted) == Presence::Complete {
            switcher.using = Some(wanted);
            switcher.delete_unused(disk, &mut commands);
            commands.push(Command::Report(ModelStatus::Ready));
        } else if disk(other(wanted)) == Presence::Complete {
            switcher.using = Some(other(wanted));
            switcher.engine_files = other(wanted);
            switcher.begin_download(&mut commands);
        } else {
            switcher.begin_download(&mut commands);
        }
        (switcher, commands)
    }

    pub fn engine_files(&self) -> Precision {
        self.engine_files
    }

    pub fn wanted(&self) -> Precision {
        self.wanted
    }

    pub fn usable(&self) -> bool {
        self.using.is_some()
    }

    pub fn want(
        &mut self,
        wanted: Precision,
        disk: &dyn Fn(Precision) -> Presence,
    ) -> Vec<Command> {
        let mut commands = Vec::new();
        self.wanted = wanted;
        if self.job.is_some_and(|job| job.precision == wanted) {
            return commands;
        }
        let cancelled = self.job.take();
        if let Some(job) = cancelled {
            commands.push(Command::Cancel { id: job.id });
        }
        if disk(wanted) == Presence::Complete {
            self.adopt(wanted, cancelled.is_none(), disk, &mut commands);
            commands.push(Command::Report(ModelStatus::Ready));
        } else {
            self.begin_download(&mut commands);
        }
        commands
    }

    pub fn retry(&mut self, disk: &dyn Fn(Precision) -> Presence) -> Vec<Command> {
        let wanted = self.wanted;
        if self.job.is_some() {
            return Vec::new();
        }
        self.want(wanted, disk)
    }

    pub fn download_event(
        &mut self,
        id: u64,
        status: ModelStatus,
        disk: &dyn Fn(Precision) -> Presence,
    ) -> Vec<Command> {
        let Some(job) = self.job.filter(|job| job.id == id) else {
            return Vec::new();
        };
        let mut commands = Vec::new();
        match status {
            ModelStatus::Downloading { .. } | ModelStatus::Verifying => {
                commands.push(Command::Report(status));
            }
            ModelStatus::Failed(_) => {
                self.job = None;
                commands.push(Command::Report(status));
            }
            ModelStatus::Ready => {
                self.job = None;
                if disk(job.precision) == Presence::Complete {
                    self.adopt(job.precision, true, disk, &mut commands);
                    commands.push(Command::Report(ModelStatus::Ready));
                } else {
                    commands.push(Command::Report(ModelStatus::Failed(format!(
                        "{} is incomplete after the download",
                        job.precision
                    ))));
                }
            }
        }
        commands
    }

    pub fn switched(
        &mut self,
        precision: Precision,
        disk: &dyn Fn(Precision) -> Presence,
    ) -> Vec<Command> {
        let mut commands = Vec::new();
        self.using = Some(precision);
        if self.requested == Some(precision) {
            self.requested = None;
        }
        if self.requested.is_none() {
            self.delete_unused(disk, &mut commands);
        }
        commands
    }

    fn adopt(
        &mut self,
        precision: Precision,
        cleanup: bool,
        disk: &dyn Fn(Precision) -> Presence,
        commands: &mut Vec<Command>,
    ) {
        match self.using {
            None => {
                if self.engine_files != precision {
                    self.engine_files = precision;
                    commands.push(Command::Use(precision));
                }
                self.using = Some(precision);
                commands.push(Command::Ready);
                if cleanup {
                    self.delete_unused(disk, commands);
                }
            }
            Some(using) => {
                let target = self.requested.unwrap_or(using);
                if target != precision {
                    self.requested = Some(precision);
                    self.engine_files = precision;
                    commands.push(Command::Use(precision));
                } else if cleanup && self.requested.is_none() {
                    self.delete_unused(disk, commands);
                }
            }
        }
    }

    fn delete_unused(&self, disk: &dyn Fn(Precision) -> Presence, commands: &mut Vec<Command>) {
        let Some(using) = self.using else {
            return;
        };
        let unused = other(using);
        if disk(unused) != Presence::Absent && self.job.is_none_or(|job| job.precision != unused) {
            commands.push(Command::Delete(unused));
        }
    }

    fn begin_download(&mut self, commands: &mut Vec<Command>) {
        self.next_id += 1;
        let job = Job {
            id: self.next_id,
            precision: self.wanted,
        };
        self.job = Some(job);
        commands.push(Command::Download {
            id: job.id,
            precision: job.precision,
        });
    }
}
