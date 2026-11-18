# Launch at login uses SMAppService.mainApp and is offered only from the bundled app

`platform_macos::login_item` wraps `SMAppService.mainApp` (`objc2-service-management`): `status()` reads whether the item is enabled, waiting for approval in System Settings, or not registered, and `set(bool)` registers or unregisters it. The first launch of the bundled app registers it once, because the default is on, and records `launch_at_login` together with a separate `login_item_decided=on` key only when that succeeded (the first run is read from `login_item_decided`, not from `launch_at_login` being absent, so settings saved by an unbundled run do not block the registration); from then on the stored value is the user's intent and the switch in the settings window mirrors the real status at start, so a change made in System Settings > Login Items is not undone. When macOS answers "requires approval", the window says so and offers a button that opens the Login Items pane.

`SMAppService` only works for an app that lives in a bundle. When PimSiang is started from a terminal or from `target/` the switch is disabled with an explanation instead of failing, and nothing is registered. A registration error is shown under the switch and the switch returns to its previous value.

## Considered options

- **A LaunchAgent plist written by the app:** works unbundled, but is invisible in the system's Login Items list and needs its own cleanup.
- **Re-registering on every launch:** would silently undo the user turning it off in System Settings.
