# Vellum SDK

Trusted host capability contracts, session orchestration, and the headless
editor application layer for the Vellum semantic engine. The repository's
crates are `vellum-host`, `vellum-session`, `vellum-editor`,
`vellum-protocol`, `vellum-service`, and `vellum-client`. Concrete filesystem,
serialization, Unicode, clipboard, remote transport, MCP, and presentation
implementations live in sibling projects or later layers.

The current service profile is deliberately in-process and admits a bounded
population of clients over one shared `Workspace`. Each client has an
independent `EditorView`, request namespace, and version-witness namespace. It
exposes semantic version-bound range replacement without borrowing presentation
cursor state. This is a real multi-client vertical slice, not a remote-service,
daemon, or authentication claim. See [service architecture](docs/service-architecture.md).

Local development resolves exact-version path dependencies through the sibling
`vellum` and `vellum-adapters` repositories. Published packages use the same
exact versions without relying on those paths.
