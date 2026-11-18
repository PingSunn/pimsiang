# Permissions are polled by the Utterance controller; the Trigger key starts when Input Monitoring is granted

PimSiang needs three macOS permissions: Microphone (capture), Accessibility (posting the paste keystroke) and Input Monitoring (the listen-only `CGEventTap` of ADR 0013 is refused without it). The `Platform` trait gains `permission_status`, `request_permission`, `open_settings` and `show_permissions`. The Utterance controller polls the statuses once a second, publishes a report to the UI only when it changes, and starts watching the Trigger key on the first poll that finds Input Monitoring granted, retrying each poll if the tap is still refused. A missing permission therefore no longer ends the app with a fatal error, and a grant is noticed without a restart. The egui window and the menu bar are views of the published report; button clicks come back as controller inputs, so the whole flow runs against `FakePlatform` in tests.

When the Trigger key is pressed with Microphone or Accessibility missing, the Pill shows which one. With Input Monitoring missing the key cannot be observed at all, so the menu bar warning (amber icon, a line naming what is missing, and a "Set up permissions" item) is the only signal.

The setup window opens at launch whenever a permission is missing, not only on the very first launch, because there is no settings store yet to remember a first run; it closes itself shortly after everything is granted and can be reopened from the menu while something is missing. Microphone is asked for with the system prompt while undetermined and with its Settings pane once denied; Accessibility and Input Monitoring request access first, which makes macOS list PimSiang, and then open their pane. macOS ties each grant to the process that is responsible for the app: today that is the terminal it was started from, later the signed PimSiang.app (ADR 0007).

## Considered options

- **Poll in the main thread or a separate supervisor:** needs a second owner of the platform; the controller already owns it and has a deadline loop.
- **Show the window only on a stored first run:** needs persisted settings that do not exist yet.
- **Prompt for Accessibility with `AXIsProcessTrustedWithOptions`:** the system dialog duplicates our own guidance and the button already opens the right pane.
