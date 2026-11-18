use audio::fake::FakeInputDevices;
use audio::{MicrophoneChoice, Opened, select};

fn devices() -> FakeInputDevices {
    FakeInputDevices::new(
        &[("built-in", "Built-in Microphone"), ("usb", "USB Headset")],
        Some("built-in"),
    )
}

#[test]
fn system_default_opens_the_default_device() {
    let (device, opened) = select(&devices(), &MicrophoneChoice::SystemDefault).unwrap();

    assert_eq!(device.id, "built-in");
    assert_eq!(opened, Opened::AsChosen);
}

#[test]
fn a_chosen_device_that_is_present_is_opened() {
    let (device, opened) = select(&devices(), &MicrophoneChoice::Device("usb".to_owned())).unwrap();

    assert_eq!(device.name, "USB Headset");
    assert_eq!(opened, Opened::AsChosen);
}

#[test]
fn a_chosen_device_that_disappeared_falls_back_to_the_default() {
    let mut devices = devices();
    devices.unplug("usb");

    let (device, opened) = select(&devices, &MicrophoneChoice::Device("usb".to_owned())).unwrap();

    assert_eq!(device.id, "built-in");
    assert_eq!(opened, Opened::FellBackToDefault);
}

#[test]
fn no_device_at_all_is_an_error() {
    let none = FakeInputDevices::new(&[], None);

    assert!(select(&none, &MicrophoneChoice::SystemDefault).is_err());
    assert!(select(&none, &MicrophoneChoice::Device("usb".to_owned())).is_err());
}
