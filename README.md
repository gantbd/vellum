# Vellum SDK

Trusted host capability contracts, session orchestration, and the headless
editor application layer for the Vellum semantic engine. The repository's
crates are `vellum-host`, `vellum-session`, `vellum-editor`,
`vellum-protocol`, `vellum-service`, `vellum-client`, `vellum-wire`,
`vellum-transport`, `vellum-daemon`, and `vellum-mcp`.
Concrete filesystem, Unicode, clipboard, network transport, and
presentation implementations live in sibling projects or later layers.
`vellum-wire` supplies the canonical bounded V1 encoding without selecting a
transport. `vellum-transport` supplies bounded one-request framing and a remote
client interpretation. `vellum-daemon` is the macOS local-filesystem
composition root and remains unpublished.
`vellum-mcp` is an unpublished, current-protocol-only stdio peer that projects
the native client contract for agent harnesses.

The current service profile is deliberately in-process and admits a bounded
population of clients over one shared `Workspace`. Each client has an
independent `EditorView`, request namespace, and version-witness namespace. It
exposes semantic version-bound range replacement without borrowing presentation
cursor state. The bounded macOS Unix daemon exercises the remote client path,
but makes no cross-user, network, principal-authentication, or durable-recovery
claim. See [service architecture](docs/service-architecture.md). The MCP
process connects to that daemon as an ordinary client; it has no editor or
host-capability dependency. See [MCP adapter](docs/mcp.md).

Local development resolves exact-version path dependencies through the sibling
`vellum`, `vellum-adapters`, and `keystone` repositories. No package is being
published. A future remote release must replace those checkout-relative edges
with reproducibly pinned Git or release dependencies before claiming
single-URL installation.
