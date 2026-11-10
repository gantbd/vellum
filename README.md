# Vellum SDK

Trusted host capability contracts, session orchestration, and the headless
editor application layer for the Vellum semantic engine. The repository's
crates are `vellum-host`, `vellum-session`, `vellum-editor`,
`vellum-protocol`, `vellum-service`, `vellum-client`, `vellum-wire`,
`vellum-transport`, and `vellum-daemon`.
Concrete filesystem, Unicode, clipboard, network transport, MCP, and
presentation implementations live in sibling projects or later layers.
`vellum-wire` supplies the canonical bounded V1 encoding without selecting a
transport. `vellum-transport` supplies bounded one-request framing and a remote
client interpretation. `vellum-daemon` is the macOS local-filesystem
composition root and remains unpublished.

The current service profile is deliberately in-process and admits a bounded
population of clients over one shared `Workspace`. Each client has an
independent `EditorView`, request namespace, and version-witness namespace. It
exposes semantic version-bound range replacement without borrowing presentation
cursor state. The bounded macOS Unix daemon exercises the remote client path,
but makes no cross-user, network, principal-authentication, or durable-recovery
claim. See [service architecture](docs/service-architecture.md).

Local development resolves exact-version path dependencies through the sibling
`vellum` and `vellum-adapters` repositories. Published packages use the same
exact versions without relying on those paths.
