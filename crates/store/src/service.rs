use std::path::Path;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::history::{History, HistoryError, Kind};

type Job = Box<dyn FnOnce(&History) + Send>;

pub struct HistoryService {
    jobs: Option<Sender<Job>>,
    worker: Option<JoinHandle<()>>,
}

impl HistoryService {
    pub fn spawn(path: &Path) -> Result<Self, HistoryError> {
        let history = History::open(path)?;
        let (jobs, queue) = channel::<Job>();
        let worker = std::thread::Builder::new()
            .name("history".to_owned())
            .spawn(move || {
                for job in queue {
                    job(&history);
                }
            })?;
        Ok(Self {
            jobs: Some(jobs),
            worker: Some(worker),
        })
    }

    pub fn run(&self, job: impl FnOnce(&History) + Send + 'static) {
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(Box::new(job));
        }
    }

    pub fn add(
        &self,
        text: String,
        kind: Kind,
        model_id: Option<String>,
        on_error: impl FnOnce(HistoryError) + Send + 'static,
    ) {
        let created = now();
        self.run(move |history| {
            if let Err(error) = history.add(&text, created, kind, model_id.as_deref()) {
                on_error(error);
            }
        });
    }

    pub fn flush(&self) {
        let (done, wait) = channel();
        self.run(move |_| {
            let _ = done.send(());
        });
        let _ = wait.recv();
    }
}

impl Drop for HistoryService {
    fn drop(&mut self) {
        self.jobs.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}
