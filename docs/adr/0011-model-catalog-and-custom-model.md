# Several Models on disk: a pinned Model catalog plus an unverified Custom model

After v0.1 the user can change the Active model in settings. The Models PimSiang offers come from a Model catalog compiled into each release, every entry pinned to a Hugging Face revision with a sha256 per file as in ADR 0002, so a new Model only arrives with a new release. Next to the catalog, an advanced option lets the user point at a folder holding a Custom model, which PimSiang loads without verifying it: this is the one place the pinning promise of ADR 0002 does not hold, accepted because it lets people try a fine-tuned or newly exported Model without waiting for a release. Downloading a Model and making it active are separate steps, downloaded Models stay on disk until the user deletes them (never the Active model), a switch takes effect on the next Utterance with the old Model unloaded at once, the Language mode is offered only by Models that support it and remembered per Model, and every History entry records the id of the Model that produced it.

## Considered options

- **Catalog only:** every Model verified, but trying a home-made export means shipping a release.
- **Any Hugging Face repo typed in by the user:** most flexible, but nothing guarantees the files fit the Engine, and nothing is pinned.
- **Deleting the previous Model on every switch:** saves about 1 GB of disk per Model, but switching back means downloading it again.
- **Switching as soon as the download finishes:** one click fewer, but the Model changes under the user without them asking at that moment.
