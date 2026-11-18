# History lives behind a single-writer thread, with a LIKE fallback for short searches

`store::History` wraps one SQLite file (`~/Library/Application Support/PimSiang/history.sqlite`) opened through `rusqlite` with the `bundled` feature, so FTS5 and the `trigram` tokenizer (SQLite 3.34+) do not depend on the macOS system library. The schema is versioned with `PRAGMA user_version` and migrated forward in one transaction per step: version 1 holds `entries` plus an external-content FTS5 table kept in sync by triggers, version 2 adds a nullable `model_id` column so each entry can record the Model that produced it (ADR 0011). A database with a higher version than the app knows is refused untouched. Time is stored as unix seconds; kind as `pasted` or `note`; order is `created DESC, id DESC`.

All app access goes through `HistoryService`, one thread that owns the connection and runs queued closures in order. The Utterance controller only enqueues an insert, so a slow disk never delays a paste; the History window enqueues a search and gets the entries back as an event on the winit loop, so the UI thread never touches SQLite. Because the queue is FIFO, a refresh sent after a delete always sees the delete, and the app flushes the queue before exiting.

Trigram needs at least three characters, so a query of one or two characters (common in Thai, where a word can be two letters) is answered with `LIKE '%q%' ESCAPE '\'` with `%`, `_` and `\` escaped; longer queries are one quoted FTS5 phrase with `"` doubled, so operators such as `OR` and `*` are plain text. Both paths are newest first and page with `LIMIT/OFFSET`.

A Final text whose paste failed is stored as a Note, so text the user spoke is never lost; "Paste last transcript" pastes the newest entry again without recording a second copy. The History window shows 50 entries and a "Show more" button, and confirms "Clear all" inside the window rather than with a native alert.

## Considered options

- **Querying from the UI thread on its own connection:** simpler, but a locked database or a large clear would freeze the window.
- **Only FTS5, rejecting short queries:** leaves two-letter Thai searches with no results.
- **Storing paste failures as nothing:** matches "only pasted text is History", but throws away a transcript the user cannot get back.
