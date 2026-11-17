# History keeps text only, in SQLite with trigram full-text search

History (every Final text and every Note) is stored in one SQLite file under `~/Library/Application Support/PimSiang`, with an FTS5 table using the trigram tokenizer because Thai has no spaces between words. Audio is never written to disk, and entries never expire; the user deletes single entries or clears everything. This keeps the privacy promise simple: nothing but text stays on the machine.

## Considered options

- **Markdown files per day:** readable in any editor, but slow to search.
- **Keeping audio for 7 days to replay mistakes:** helpful for corrections, but leaves recordings of the user's voice on disk.
