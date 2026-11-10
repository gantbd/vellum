# Vellum

Vellum is a headless, agent-friendly Rust editing engine for code and prose.
Its engine owns authoritative source and history state, bounded admission,
retained outcomes, and content-safe instrumentation. The repository also owns
the adapter-neutral host, native protocol/service/client profile, qualified
first-party adapters, connectors, and non-UI composition roots. These remain
separate crates with enforced dependency direction.

The workspace is organized by architectural role rather than packaging type:

- `engine/` — categorically closed `vellum-engine` semantics
- `host/` — trusted ports, session orchestration, and headless editor substrate
- `protocol/`, `client/`, `service/`, and `transport/` — native service contract
- `adapters/` — qualified outbound effect interpretations
- `connectors/` — inbound protocol projections such as MCP
- `applications/` — non-UI executable composition roots such as the daemon
- `qualification/` — unpublished conformance and live-platform probes
- `sdk/` — zero-authority embedded and client distribution facades

Presentation projects remain independent:

- `vellum-tui` — terminal composition root and `vellum` executable
- `vellum-gui-macos` — native macOS GUI composition root

The consolidated Git graph retains the former `vellum-sdk` and
`vellum-adapters` histories. See [project boundaries](docs/project-boundaries.md)
for the dependency law. Superseded layouts remain audit evidence, not active
package boundaries.

## Verify

```sh
make check
```

The workspace requires the sibling `keystone` checkout for the canonical wire
codec. The engine crate itself has no upward Vellum, UI, filesystem, clipboard,
ICU, service-runtime, or application-framework dependency.

Run the headless example with:

```sh
cargo run --offline -p vellum-engine --example headless_edit
```

The UI repositories are currently source-distributed and explicitly
non-publishable. Neither is part of this workspace or a dependency of the
other. No crate is being released to crates.io yet.
