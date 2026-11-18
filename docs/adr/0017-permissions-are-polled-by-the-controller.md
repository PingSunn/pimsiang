# Permissions are polled by the Utterance controller; the Trigger key starts when Input Monitoring is granted

PimSiang needs three macOS permissions: Microphone (capture), Accessibility (posting the paste keystroke) and Input Monitoring (the listen-only `CGEventTap` of ADR 0013 is refused without it). The `Platform` trait gains `permission_status`, `request_permission`, `open_settings` and `show_permissions`. The Utterance controller polls the statuses once a second, publishes a report to the UI only when it changes, and starts watching the Trigger key on the first poll that finds Input Monitoring granted, retrying each poll if the tap is still refused. A missing permission therefore no longer ends the app with a fatal error, and a grant is noticed without a restart. The egui window and the menu bar are views of the published report; button clicks come back as controller inputs, so the whole flow runs against `FakePlatform` in tests.

When the Trigger key is pressed with Microphone or Accessibility missing, the Pill shows which one. With Input Monitoring missing the key cannot be observed at all, so the menu bar warning (amber icon, a line naming what is missing, and a "Set up permissions" item) is the only signal.

The setup window opens at launch whenever a permission is missing, not only on the very first launch, because there is no settings store yet to remember a first run; it closes itself shortly after everything is granted and can be reopened from the menu while something is missing. Microphone is asked for with the system prompt while undetermined and with its Settings pane once denied; Accessibility shows the system prompt on the first click and opens its pane on the next; Input Monitoring requests access, which makes macOS list PimSiang, and then opens its pane. macOS ties each grant to the signed PimSiang.app bundle (ADR 0019, ADR 0007); a bare binary run from a terminal is attributed to the terminal instead. A revoked Input Monitoring grant stops the event tap, and the next grant creates a new one.

## Considered options

- **Poll in the main thread or a separate supervisor:** needs a second owner of the platform; the controller already owns it and has a deadline loop.
- **Show the window only on a stored first run:** needs persisted settings that do not exist yet.
- **Open the pane and prompt together:** two windows appear at once for Accessibility, so the prompt comes first and the pane on a later click.
