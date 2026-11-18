use crate::{AudioError, Microphone};

#[derive(Default)]
pub struct FakeMicrophone {
    capturing: bool,
    start_count: usize,
    fails: bool,
}

impl FakeMicrophone {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn failing() -> Self {
        Self {
            fails: true,
            ..Self::default()
        }
    }

    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    pub fn start_count(&self) -> usize {
        self.start_count
    }
}

impl Microphone for FakeMicrophone {
    fn start(&mut self) -> Result<(), AudioError> {
        if self.fails {
            return Err(AudioError::Microphone("scripted failure".to_owned()));
        }
        self.capturing = true;
        self.start_count += 1;
        Ok(())
    }

    fn stop(&mut self) {
        self.capturing = false;
    }
}
