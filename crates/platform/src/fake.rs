use crate::{PillState, Platform, PlatformError, Status, TriggerEvent, TriggerSink};

#[derive(Default)]
pub struct FakePlatform {
    sink: Option<TriggerSink>,
    statuses: Vec<Status>,
    pills: Vec<PillState>,
    pastes: Vec<String>,
    paste_fails: bool,
}

impl FakePlatform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn failing_paste(mut self) -> Self {
        self.paste_fails = true;
        self
    }

    pub fn emit(&mut self, event: TriggerEvent) -> bool {
        match self.sink.as_mut() {
            Some(sink) => {
                sink(event);
                true
            }
            None => false,
        }
    }

    pub fn statuses(&self) -> &[Status] {
        &self.statuses
    }

    pub fn pills(&self) -> &[PillState] {
        &self.pills
    }

    pub fn pastes(&self) -> &[String] {
        &self.pastes
    }
}

impl Platform for FakePlatform {
    fn listen_trigger(&mut self, sink: TriggerSink) -> Result<(), PlatformError> {
        self.sink = Some(sink);
        Ok(())
    }

    fn show_status(&mut self, status: Status) {
        self.statuses.push(status);
    }

    fn show_pill(&mut self, state: PillState) {
        self.pills.push(state);
    }

    fn paste(&mut self, text: &str) -> Result<(), PlatformError> {
        if self.paste_fails {
            return Err(PlatformError::Paste("scripted failure".to_owned()));
        }
        self.pastes.push(text.to_owned());
        Ok(())
    }
}
