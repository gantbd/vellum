# Extraction provenance

This repository was extracted from `gantbd/vellum` at source commit
`920dbfc` (`refactor: separate engine sdk adapters and apps`). The filtered
history retains changes to the adapter crates, `vellum-unicode-reference`, the
Unicode formal assets, and the qualified AppKit patch used by the macOS
clipboard adapter.

The root workspace files establish the new project boundary. Exact-version
path dependencies support sibling-repository development; published packages
resolve the same versions from the registry.
