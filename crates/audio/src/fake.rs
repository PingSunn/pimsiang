use crate::{AudioError, InputDevice, InputDevices, Microphone, MicrophoneChoice, Opened, select};

pub struct FakeInputDevices {
    devices: Vec<InputDevice>,
    default: Option<String>,
}

impl FakeInputDevices {
    pub fn new(devices: &[(&str, &str)], default: Option<&str>) -> Self {
        Self {
            devices: devices
                .iter()
                .map(|(id, name)| InputDevice {
                    id: (*id).to_owned(),
                    name: (*name).to_owned(),
                })
                .collect(),
            default: default.map(str::to_owned),
        }
    }

    pub fn plug(&mut self, id: &str, name: &str) {
        self.devices.push(InputDevice {
            id: id.to_owned(),
            name: name.to_owned(),
        });
    }

    pub fn unplug(&mut self, id: &str) {
        self.devices.retain(|device| device.id != id);
        if self.default.as_deref() == Some(id) {
            self.default = self.devices.first().map(|device| device.id.clone());
        }
    }
}

impl InputDevices for FakeInputDevices {
    fn list(&self) -> Result<Vec<InputDevice>, AudioError> {
        Ok(self.devices.clone())
    }

    fn default_device(&self) -> Option<InputDevice> {
        self.devices
            .iter()
            .find(|device| Some(&device.id) == self.default.as_ref())
            .cloned()
    }
}

pub struct FakeMicrophone {
    devices: FakeInputDevices,
    choice: MicrophoneChoice,
    capturing: bool,
    start_count: usize,
    fails: bool,
    opened: Option<InputDevice>,
}

impl Default for FakeMicrophone {
    fn default() -> Self {
        Self::with_devices(FakeInputDevices::new(
            &[("built-in", "Built-in Microphone")],
            Some("built-in"),
        ))
    }
}

impl FakeMicrophone {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_devices(devices: FakeInputDevices) -> Self {
        Self {
            devices,
            choice: MicrophoneChoice::default(),
            capturing: false,
            start_count: 0,
            fails: false,
            opened: None,
        }
    }

    pub fn failing() -> Self {
        Self {
            fails: true,
            ..Self::default()
        }
    }

    pub fn plug(&mut self, id: &str, name: &str) {
        self.devices.plug(id, name);
    }

    pub fn unplug(&mut self, id: &str) {
        self.devices.unplug(id);
    }

    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    pub fn start_count(&self) -> usize {
        self.start_count
    }

    pub fn opened(&self) -> Option<&InputDevice> {
        self.opened.as_ref()
    }

    pub fn choice(&self) -> &MicrophoneChoice {
        &self.choice
    }
}

impl Microphone for FakeMicrophone {
    fn choose(&mut self, choice: MicrophoneChoice) {
        self.choice = choice;
    }

    fn devices(&self) -> Vec<InputDevice> {
        self.devices.list().unwrap_or_default()
    }

    fn start(&mut self) -> Result<Opened, AudioError> {
        if self.fails {
            return Err(AudioError::Microphone("scripted failure".to_owned()));
        }
        let (device, opened) = select(&self.devices, &self.choice)?;
        self.opened = Some(device);
        self.capturing = true;
        self.start_count += 1;
        Ok(opened)
    }

    fn stop(&mut self) {
        self.capturing = false;
    }
}
