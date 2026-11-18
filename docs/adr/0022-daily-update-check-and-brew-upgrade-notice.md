# Daily update check against GitHub Releases, notice in the menu bar

A background thread (`update::spawn_checker`, off the Utterance controller and the main thread) asks `GET /repos/PingSunn/pimsiang/releases/latest` on the GitHub API about 20 seconds after launch and then wakes every hour and checks again when the persisted check time is a day old, so a sleeping Mac or a late launch never stretches the gap. `Source` holds the API base and the repository, so tests point it at a local TCP server and no test touches the network. The response is read with `ureq` (blocking, rustls, 5 s connect and 10 s overall timeout), so no async runtime is pulled in for one request. Versions are compared with the `semver` crate against the crate's own `CARGO_PKG_VERSION`; drafts, prereleases and prerelease tags are ignored, a leading `v` on the tag is stripped and a missing minor or patch (`v0.1`) is read as `.0`. This is the only network call besides the Model download, and it sends no identifier beyond `User-Agent: PimSiang`.

The repository is private until v0.1, so the API answers 404 without credentials; that, a rate limit (403), a timeout, a malformed body and an unreachable network all mean "no update known". They are logged at info level, never shown, and the check time is still recorded, so the retry is tomorrow's, not a loop. A notice found earlier is kept across a failed check.

State lives in the interim settings file (ADR 0021): `last_update_check` (unix seconds), `available_update` (version) and `dismissed_update` (version), so the notice shows at once after a restart without a request and a check is due only when `last_update_check` is a day old (or in the future, after a clock change). The notice shows while `available_update` is newer than the running version (so it disappears by itself after `brew upgrade` and a restart) and newer than `dismissed_update`. "Dismiss this update notice" stores the shown version; a still newer release shows again.

The menu bar gets "Update available: vX — run brew upgrade" (Thai and English) plus a dismiss item, inserted only while there is something to show. Clicking the notice copies `brew upgrade --cask PingSunn/tap/pimsiang` (the tap named in full, ADR 0007) to the clipboard with `/usr/bin/pbcopy` and the item text turns into a "copied" confirmation. The app never opens a browser or runs brew itself.

Every settings change goes through `Settings::update`, which holds a process-wide lock while it re-loads the file, applies the change and writes it through a uniquely named temporary file, so the update thread and the microphone menu cannot overwrite each other's keys. The GitHub request runs outside the lock, and its result is applied to a fresh load.

## Considered options

- **Open the release page on click:** one more click for the user and the page is private until v0.1; the command is what the user needs.
- **Run `brew upgrade` from the app:** fights Homebrew over who owns the installed app (ADR 0007).
- **`tokio` and `reqwest`:** an async runtime for one request a day.
- **Keep the last check in memory:** the app would check again on every launch.
