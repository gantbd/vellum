# Vellum SDK

Trusted host capability contracts, session orchestration, and the headless
editor application layer for the Vellum semantic engine. The repository's
crates are `vellum-host`, `vellum-session`, `vellum-editor`,
`vellum-protocol`, `vellum-service`, and `vellum-client`. Concrete filesystem,
serialization, Unicode, clipboard, remote transport, MCP, and presentation
implementations live in sibling projects or later layers.

The first service profile is deliberately in-process and admits one client.
It exposes semantic version-bound range replacement without using editor cursor
state. This is a real vertical slice, not a multi-client or remote-service
claim. See [service architecture](docs/service-architecture.md).

Local development resolves exact-version path dependencies through the sibling
`vellum` and `vellum-adapters` repositories. Published packages use the same
exact versions without relying on those paths.
