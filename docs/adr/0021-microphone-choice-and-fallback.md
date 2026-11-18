# Microphone choice: stable device ids, fallback at open time, interim settings file

The `audio` crate lists input devices (`InputDevices`, backed by `cpal` in `CpalDevices` and by `FakeInputDevices` in tests) and resolves a `MicrophoneChoice` (system default, or a device id) with one pure function, `select`: a chosen device that is present opens as chosen, one that is gone opens the system default and reports `Opened::FellBackToDefault`, and no device at all is an error. Devices are identified by the `cpal` `DeviceId` string (`host:device`), not the display name, so two headsets with the same name stay distinct and a rename does not lose the choice. The fallback happens each time an Utterance opens the microphone, so the choice is never forgotten: replugging the headset selects it again without a new click. If the device disappears in the middle of an Utterance the stream ends and the next Utterance falls back; nothing is switched mid-stream.

The Utterance controller owns the choice. A menu click arrives as `Input::Microphone`; on every permission poll the controller republishes the device list through `Platform::show_microphones` only when it changed (the pattern of ADR 0017), with the chosen device marked only while it is present, so an unplugged choice shows "System default" checked. When a fallback happened the Pill shows a `PillNotice` right after "Listening"; the first Partial text replaces it. No device at all keeps showing `PillError::MicrophoneUnavailable`.

Persistence is the minimum that fits: ticket 10 owns the settings window and the SQLite-backed settings in `store` and is not built yet, so `store::Settings` is a one-key text file (`microphone=<id>`, in `~/Library/Application Support/PimSiang/settings`) written by the app on a menu click and read at start. A missing or unreadable file means the defaults. Ticket 10 is expected to keep `Settings::load`/`save` and move the storage into SQLite.

## Considered options

- **Identify devices by name:** readable, but duplicates collide and a renamed device loses the choice.
- **Forget the choice when the device disappears:** simpler menu, but the user would have to pick the headset again after every replug.
- **Add `rusqlite` now for one setting:** pulls in the History decisions (ticket 08) early; the file is trivial to replace.
