# Paste restores the previous clipboard after a delay, unless the user copied something meanwhile

`MacPlatform::paste` takes a `restore_clipboard` flag that the controller fills from the setting (on by default, ADR 0004). The sequence lives in `platform::clipboard`, written against a small `Pasteboard` trait so it is tested without a real pasteboard: `put_text` snapshots the pasteboard when restoring is on, writes the Final text and records the change count that write produced; after the paste keystroke has been sent, a restore thread waits 400 ms so the target app has read the text, then `PendingRestore::restore` writes the snapshot back only if the change count is still the one recorded. If the user (or another app) copied something in between, the new contents are left alone. A pasteboard that cannot be snapshotted is pasted over without a restore, and a failed paste keeps the text on the clipboard so it can be pasted by hand.

The macOS snapshot keeps every item and every type (`NSPasteboardItem` types with their data), not only text, so images, files and rich text come back too; promised types that cannot produce data are skipped. The write and restore go through `NSPasteboard` instead of `arboard`, so the change count seen by the check is the one the write produced, and `arboard` is no longer a dependency. The restore runs on its own short-lived thread so the Utterance controller never sleeps. "Paste last transcript" follows the same setting.

## Considered options

- **Restore immediately after the keystroke:** the target app may not have read the pasteboard yet and would paste the old contents.
- **Always restore, without a change count check:** can overwrite something the user just copied.
- **Snapshot text only:** simpler, but silently loses an image or file on the clipboard.
