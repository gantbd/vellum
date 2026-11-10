# Vellum project boundaries

Status: consolidated and dependency-closed on 2026-11-09.

Vellum's non-UI semantic spine, service profile, first-party adapters,
connectors, and composition roots share one atomic-evolution repository.
Separate crates and categorical directories preserve dependency and authority
boundaries. Presentation repositories retain independent release lifecycles.

| Project | Owns | Must not own |
| --- | --- | --- |
| `vellum/engine` | Closed semantic engine, authoritative source/history state, admission, retained outcomes and bounded instrumentation | UI state, filesystem paths, platform services or connector policy |
| `vellum/host` | `vellum-host` capability ports, sessions, and the headless `vellum-editor` | Concrete OS, storage, clipboard, transport or rendering implementations |
| `vellum/protocol`, `client`, `service`, `transport` | Native protocol and service profile | UI policy or alternate semantic authority |
| `vellum/adapters` | Qualified interpretations of host ports | Semantic authority, application navigation or hidden retry policy |
| `vellum/connectors`, `applications` | Inbound projections and explicit composition roots | Hidden engine or adapter authority |
| `vellum/sdk` | Zero-authority embedded/client distribution facades | New behavior or dependency inversion |
| `vellum-tui` | Terminal composition root, bindings, rendering and lifecycle | GUI policy, independent source/history authority or reusable adapter policy |
| `vellum-gui-macos` | Native macOS GUI composition root, bindings, rendering and lifecycle | Terminal policy, independent source/history authority or reusable adapter policy |

```text
vellum-tui --------+----> vellum-editor ----> vellum-session -> vellum-engine
                   |          |                    ^
vellum-gui-macos --+          +--> vellum-host <---+--- vellum-adapters

vellum-client ----> vellum-protocol <---- vellum-service ----> vellum-editor
      |                    ^                    ^
vellum-transport -> vellum-wire          vellum-daemon -> local-fs adapter
      ^
vellum-mcp
```

Applications are composition roots and inject adapters. The editor/host cannot
select a concrete adapter through a shipped dependency. An adapter may depend
on host ports and engine evidence types; neither the host nor engine may depend
on an adapter.

The former repository histories are retained in the consolidated Git graph:

- former `vellum-sdk` history, joined before import
- former `vellum-adapters` history, joined before import
- `vellum-tui` — extracted with path history from `vellum-apps`
- `vellum-gui-macos` — extracted with path history from `vellum-apps`

The workspace has one executable boundary gate covering crate presence,
categorical placement, dependency direction, adapter selection, SDK facade
closure, and qualification isolation. The two UI applications are not yet
published and do not advertise an install contract. The gate rejects every
upward dependency from `vellum-engine`.

`vellum-editor` is a headless editor application layer, not authoritative
document state. Its presentation-free `Workspace` serializes shared document
authority while bounded client-owned `EditorView` values keep projections
independent; the service separately scopes request identities and witnesses per
client. `vellum-host` owns trusted capability ports. The native service profile
is multi-client and has a bounded macOS local Unix daemon. The daemon is the
composition root selecting the local-filesystem adapter; it makes no MCP,
principal-authentication, network, reconnect, or durable-registration claim.
`vellum-mcp` is a separate current-protocol stdio process over the native client
port. It is an inbound peer to the human applications, not a capability of the
daemon or a dependency of the semantic engine. It exposes snapshot and
revision-checked replacement tools, deriving a fresh native witness within the
mutation request; resource projection waits for explicit stable document
identity.
Local filesystem behavior remains a replaceable adapter, not an engine
intrinsic. A future navigable human/agent product that composes Vellum, Chronos,
chat, and orchestration belongs in a separate product repository.
