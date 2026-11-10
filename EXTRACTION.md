# Extraction provenance

This repository was extracted from `gantbd/vellum` at source commit
`920dbfc` (`refactor: separate engine sdk adapters and apps`). The filtered
history retains changes to:

- `crates/vellum-host`
- `crates/vellum-session`
- `crates/vellum-editor`

The root workspace files establish the new project boundary. Exact-version
path dependencies support sibling-repository development; published packages
resolve the same versions from the registry.
