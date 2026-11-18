# Settings stay in the text file, are saved as `key=value` lines and apply without a restart

ADR 0021 expected ticket 10 to move `store::Settings` into SQLite. It does not: the settings are a handful of scalar values, `Settings::update` already serialises read-modify-write under one process-wide lock and writes through a synced temporary file, unknown lines survive, and a settings file that cannot be read simply gives the defaults, so a damaged History database can never take the settings with it. The file gains five keys: `trigger_key` (`right-option`, `right-command` or `right-control`), `language` (`th-mix` or `th-TH`), and the switches `sounds`, `clipboard_restore` and `launch_at_login` (`on` or `off`). A missing, unknown or unparsable value means the default (Right Option, `th-mix`, all three switches on), and the switches expose `sounds()`, `clipboard_restore()` and `launch_at_login()` so callers never repeat the defaults. `launch_at_login` being absent also means the login item was never offered, which the app uses to register it once on the first launch of the bundled app.

The settings window sends the whole `Preferences` value to the app, which saves it with `Settings::update` and hands it to the Utterance controller as `Input::Preferences`. The controller copies language, sounds and clipboard restore into its `Config`, so the language mode applies from the next Utterance (the one in progress keeps the options it started with), and calls `Platform::set_trigger_key` only when the key actually changed. At start the app sends the stored preferences the same way; `--language` on the command line still overrides the stored language for that run.

## Considered options

- **Move the settings into the History SQLite database:** one file less, but couples the settings to a database that can fail to open, and needs a migration for no gain.
- **A JSON or TOML settings file:** nicer for nesting, but adds a parser dependency and a migration for the existing `microphone` and update keys.
- **Restart the controller to apply a change:** simpler to reason about, but a changed Trigger key would drop an Utterance in progress and re-prompt nothing useful.
