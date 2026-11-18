# Daily update check against GitHub Releases, notice in the menu bar

A background thread (`update::spawn_checker`, off the Utterance controller and the main thread) asks `GET /repos/PingSunn/pimsiang/releases/latest` on the GitHub API about 20 seconds after launch and then every 24 hours. `Source` holds the API base and the repository, so tests point it at a local TCP server and no test touches the network. The response is read with `ureq` (blocking, rustls, 10 s timeout), so no async runtime is pulled in for one request. Versions are compared with the `semver` crate against the crate's own `CARGO_PKG_VERSION`; drafts, prereleases and prerelease tags are ignored, and a leading `v` on the tag is stripped. This is the only network call besides the Model download, and it sends no identifier beyond `User-Agent: PimSiang`.

The repository is private until v0.1, so the API answers 404 without credentials; that, a rate limit (403), a timeout, a malformed body and an unreachable network all mean "no update known". They are logged at info level, never shown, and the check time is still recorded, so the retry is tomorrow's, not a loop. A notice found earlier is kept across a failed check.

State lives in the interim settings file (ADR 0021): `last_update_check` (unix seconds), `available_update` (version) and `dismissed_update` (version), so the notice shows at once after a restart without a request and a check is due only when `last_update_check` is a day old (or in the future, after a clock change). The notice shows while `available_update` is newer than the running version (so it disappears by itself after `brew upgrade` and a restart) and newer than `dismissed_update`. "Dismiss this update notice" stores the shown version; a still newer release shows again.

The menu bar gets "Update available: vX — run brew upgrade" (Thai and English) plus a dismiss item, inserted only while there is something to show. Clicking the notice copies `brew upgrade --cask pimsiang` to the clipboard with `pbcopy` and the item text turns into a "copied" confirmation. The app never opens a browser or runs brew itself.

The update thread and the menu-click handler both load, change and save the settings file without a lock; the two writers act at most a few times a day and a lost write only repeats a check or re-shows a notice.

## Considered options

- **Open the release page on click:** one more click for the user and the page is private until v0.1; the command is what the user needs.
- **Run `brew upgrade` from the app:** fights Homebrew over who owns the installed app (ADR 0007).
- **`tokio` and `reqwest`:** an async runtime for one request a day.
- **Keep the last check in memory:** the app would check again on every launch.
