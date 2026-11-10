# Vellum engine

Vellum is a headless, agent-friendly Rust editing engine for code and prose.
It owns authoritative source and history state, bounded admission, retained
outcomes, and content-safe instrumentation. It owns no UI state, filesystem
paths, platform service, connector, or application policy.

The engine is deliberately one library crate:

- `vellum-engine` — closed semantic state and effects

Sibling projects interpret and present the engine without becoming part of its
closure:

- `vellum-sdk` — host/session/editor contracts and the initial native service profile
- `vellum-adapters` — filesystem, serialization, Unicode, and clipboard ports
- `vellum-tui` — terminal composition root and `vellum` executable
- `vellum-gui-macos` — native macOS GUI composition root

The extraction preserves project history and exact-version development paths.
See [project boundaries](docs/project-boundaries.md) for the dependency law and
the extracted commit identities. Superseded monorepo materials remain under
`docs/history` as audit evidence.

## Verify

```sh
make check
```

The engine requires the sibling `keystone` checkout while
`keystone-observe` remains an unpublished exact-version dependency. It has no
UI, native filesystem, clipboard, ICU, or application-framework prerequisite.

Run the headless example with:

```sh
cargo run --offline -p vellum-engine --example headless_edit
```

The application repositories are currently source-distributed and explicitly
non-publishable. Each owns its own build and qualification workflow; neither is
part of the engine workspace or a dependency of the other.
