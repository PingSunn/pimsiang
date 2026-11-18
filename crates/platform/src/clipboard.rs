pub trait Pasteboard {
    type Snapshot;

    fn change_count(&self) -> i64;
    fn snapshot(&mut self) -> Option<Self::Snapshot>;
    fn write_text(&mut self, text: &str) -> Result<(), String>;
    fn restore(&mut self, snapshot: Self::Snapshot) -> Result<(), String>;
}

pub struct PendingRestore<S> {
    snapshot: S,
    written: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Restore {
    Restored,
    ClipboardChanged,
    Failed(String),
}

pub fn put_text<P: Pasteboard>(
    pasteboard: &mut P,
    text: &str,
    restore: bool,
) -> Result<Option<PendingRestore<P::Snapshot>>, String> {
    let snapshot = if restore { pasteboard.snapshot() } else { None };
    pasteboard.write_text(text)?;
    Ok(snapshot.map(|snapshot| PendingRestore {
        snapshot,
        written: pasteboard.change_count(),
    }))
}

impl<S> PendingRestore<S> {
    pub fn restore<P: Pasteboard<Snapshot = S>>(self, pasteboard: &mut P) -> Restore {
        if pasteboard.change_count() != self.written {
            return Restore::ClipboardChanged;
        }
        match pasteboard.restore(self.snapshot) {
            Ok(()) => Restore::Restored,
            Err(error) => Restore::Failed(error),
        }
    }
}
