use crate::AudioError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDevice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MicrophoneChoice {
    #[default]
    SystemDefault,
    Device(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    AsChosen,
    FellBackToDefault,
}

pub trait InputDevices {
    fn list(&self) -> Result<Vec<InputDevice>, AudioError>;
    fn default_device(&self) -> Option<InputDevice>;
}

pub fn select(
    devices: &dyn InputDevices,
    choice: &MicrophoneChoice,
) -> Result<(InputDevice, Opened), AudioError> {
    if let MicrophoneChoice::Device(id) = choice {
        let chosen = devices
            .list()
            .unwrap_or_default()
            .into_iter()
            .find(|device| device.id == *id);
        if let Some(device) = chosen {
            return Ok((device, Opened::AsChosen));
        }
        let fallback = devices
            .default_device()
            .ok_or_else(|| AudioError::Microphone("no input device".to_owned()))?;
        return Ok((fallback, Opened::FellBackToDefault));
    }
    let device = devices
        .default_device()
        .ok_or_else(|| AudioError::Microphone("no input device".to_owned()))?;
    Ok((device, Opened::AsChosen))
}
