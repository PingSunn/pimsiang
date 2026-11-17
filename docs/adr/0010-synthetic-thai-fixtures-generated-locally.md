# Thai test fixtures are synthesized locally, not committed

The opt-in real-Model tests need Thai recordings with known text. `scripts/make-fixtures.sh` speaks the sentences in `crates/engine/tests/fixtures/transcripts.tsv` with the macOS Thai voice (Kanya) into 16 kHz WAV files, plus one 44.1 kHz copy for resampling. Only the transcripts and the script are in git; the WAV files are gitignored, which avoids redistributing audio from a system voice in an MIT repository and keeps the repo small. Synthetic speech is cleaner than a real voice, so these fixtures catch broken front ends and decoders, not small accuracy differences.

## Considered options

- **Clips from a public Thai corpus (e.g. FLEURS):** real voices, but licences vary and the files would have to be downloaded or committed.
- **Recording the maintainer's own voice:** the most realistic, but needs a person and puts a voice recording in the repo.
