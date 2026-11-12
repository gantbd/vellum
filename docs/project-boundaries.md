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
| `vellum/protocol`, `client`, `service`, `transport` | Native protocol/service profile plus separately bounded semantic-service/client substrate | UI policy, provider selection or language-semantic truth |
| `vellum/protocol/vellum-semantic-protocol` | Provider-neutral singleton semantic-session values and lifecycle algebra | Sockets, authentication, provider processes, persistence or editing commands |
| `vellum/protocol/vellum-semantic-wire` | Strict canonical bounded encoding over semantic-session and host semantic values | Transport policy, replay storage or service implementation |
| `vellum/service/vellum-semantic-session-broker` | Pure authenticated-context reducer, replay/request ledgers and typed affine semantic effects | Provider ownership, polling, sockets, authentication or filesystem effects |
| `vellum/service/vellum-semantic-session-actor` | Bounded serialized mailbox, private affine-effect interpretation, trusted control completion and volatile drain/stop lifecycle | Sockets, authentication, role policy, provider selection, persistence, clocks or connector behavior |
| `vellum/client/vellum-semantic-session-client` | Transport-neutral phase truth, exact mutation uncertainty, compact synchronization witnesses and witness-backed reply revalidation; exports only content-safe binding facts to trusted composition | Sockets, authentication, grants, broker replay storage, provider selection or automatic retry |
| `vellum/transport/vellum-semantic-transport` | Canonical one-attempt frame adapter preserving `NotSent | Reply | OutcomeUnknown` and exact reply binding | Sockets, authentication, reconnect, retry, server dispatch, broker/provider ownership or MCP policy |
| `vellum/adapters` | Qualified interpretations of host ports | Semantic authority, application navigation or hidden retry policy |
| `vellum/connectors`, `applications` | Inbound projections and explicit composition roots; `vellum-semantic-mcp` owns bounded stdio framing and an explicit unavailable qualification composition | Hidden engine or adapter authority; the semantic MCP peer cannot infer snapshot membership or mint sovereign-validation evidence |
| `vellum/distribution/cargo-install` | One-command source installation of the daemon and MCP peer binaries | Runtime behavior, adapter selection, authority or protocol policy |
| `vellum/sdk` | Zero-authority embedded/client distribution facades | New behavior or dependency inversion |
| `vellum-tui` | Terminal composition root, bindings, rendering and lifecycle | GUI policy, independent source/history authority or reusable adapter policy |
| `vellum-gui-macos` | Native macOS GUI composition root, bindings, rendering and lifecycle | Terminal policy, independent source/history authority or reusable adapter policy |

```text
vellum-tui --------+----> vellum-editor ----> vellum-session -> vellum-engine
                   |          |                    ^
vellum-gui-macos --+          +--> vellum-host <---+--- vellum-adapters

vellum-mcp -> vellum-sdk-client -> vellum-client -> vellum-protocol <- vellum-service
      +----> local-authentication adapter                              ^
                         +------> vellum-transport -> vellum-wire         |
                                                                         v
vellum-daemon -> local-fs adapter -------------------------------> vellum-editor
      +-------> local-authentication adapter
      +-------> keystone-observe (bounded passive histogram; no clock/export)
      +-------> keystone-lifecycle (mechanical bounded obligation ledger)

vellum-semantic-client -> vellum-semantic-service -> vellum-host::semantic
                                                     |
                                      injected provider port (no launcher)

agent/human -> semantic-session client -> semantic frame transport (no socket)
                                      -> bounded semantic-session actor
                                      -> pure semantic-session broker
                                      -> vellum-semantic-wire/protocol

vellum-semantic-local -> process group + bounded stdio transport
        +-------------> vellum-adapter-semantic-lsp (composition supplied)

vellum-semantic-apply-service -> vellum-editor -> vellum-session -> vellum-engine
              +--------------> vellum-semantic-service + vellum-host::authority
```

Applications are composition roots and inject adapters. The editor/host cannot
select a concrete adapter through a shipped dependency. An adapter may depend
on host ports and engine evidence types; neither the host nor engine may depend
on an adapter.

`vellum-daemon` may consume `keystone-observe` for bounded passive storage, but
the daemon owns clock selection and timing scope. Keystone receives no Vellum
content, filesystem locator, authority, lifecycle control, subscriber, or
export capability through that dependency.

The daemon also composes `keystone-lifecycle::ObligationLedger` for its
serialized connection obligation. Keystone owns only the mechanical
open/draining/closed ledger and exact unresolved-obligation disposition. Vellum
retains its Running/Draining/Stopped/Fenced projection, ownership fencing,
readiness, refusals, counters, and control policy; none of those become
Keystone authority.

The former repository histories are retained in the consolidated Git graph:

- former `vellum-sdk` history, joined before import
- former `vellum-adapters` history, joined before import
- `vellum-tui` — extracted with path history from `vellum-apps`
- `vellum-gui-macos` — extracted with path history from `vellum-apps`

The workspace has an executable dependency-boundary gate covering crate
presence, categorical placement, dependency direction, adapter selection, SDK
facade closure, and qualification isolation. The two UI applications are not yet
published and do not advertise an install contract. The gate rejects every
upward dependency from `vellum-engine`.

`make check-effect-boundaries` additionally closes the declared production
callers of the filesystem, local-network, process-launch, and native-FFI
mechanisms recognized by its source scan. The declaration is exact: an
undeclared caller and a stale trusted member both fail, and adversarial mutants
exercise both directions on every run. This is mechanism-specific evidence,
not proof that textual scanning recognizes every effect library, raw syscall,
build script, macro expansion, dynamically loaded component, or compromised
dependency. Cargo dependency policy and platform qualification remain separate
gates.

The local-filesystem identity metadata interpreter is an explicit filesystem
effect caller (`adapters/local-fs/src/workspace_identity.rs`). It owns only the
private bounded-record encoding, locator-provenance validation,
load/stage/rename/synchronization mechanism described in
`workspace-identity-recovery.md`; workspace/document meaning and recovery
policy remain in host/service composition.

The local-authentication adapter is separately declared as a filesystem effect
caller. Its descriptor-anchored Unix implementation owns credential record
loading, exclusive provisioning, staged rotation and OS-CSPRNG acquisition; it
does not own authentication policy or authority. The effect scanner also sees
its `cfg(test)` FIFO fixture's process launch because the gate intentionally
classifies complete source files. No production process-launch claim follows.

The excluded semantic-provider qualifier is also an authority-bearing
composition root. Its `lib.rs` owns strict package/profile/artifact inspection;
its `scenario.rs` owns creation and removal of the private qualification
workspace and copying of the already-verified fixture into that workspace.
Provider process authority remains mediated by the semantic-local supervised
composition rather than a second direct command-launch surface.

`vellum-editor` is a headless editor application layer, not authoritative
document state. Its presentation-free `Workspace` serializes shared document
authority while bounded client-owned `EditorView` values keep projections
independent; the service separately scopes request identities and witnesses per
client. `vellum-host` owns trusted capability ports. The native service profile
is multi-client and has a bounded macOS local Unix daemon. The daemon is the
composition root selecting the local-filesystem adapter and, when configured,
the local-authentication adapter. Authentication produces a stable subject but
does not grant an action; the daemon's independent fixed authority ceiling
remains authoritative. It makes no MCP, remote-network, reconnect, or
durable-registration claim.
`vellum-mcp` is a separate current-protocol stdio process over the public
client SDK facade. It is an inbound peer to the human applications, not a
capability of the daemon or a dependency of the semantic engine. It exposes snapshot and
revision-checked replacement tools, deriving a fresh native witness within the
mutation request; resource projection waits for explicit stable document
identity.
When configured, MCP selects the same local-authentication adapter only to
produce a credential proof for the native transport handshake; it receives no
credential-derived authority.
Its production dependency boundary is prepared for extraction into a sibling
repository; the Vellum-owned live-daemon qualification remains separate from
the connector's portable conformance suite.

Language semantics are another peer boundary, not an editor feature or native
Vellum service command. `vellum-host::semantic` is the bounded shared value and
provider contract: seven narrow query forms, exact Vellum
source-revision, peer-index and composition-installed provider-provenance
binding, a read/rename-plan capability lattice, closed
stale/unsupported/unavailable outcomes and content-safe counters. It
is still exposed by the embedded SDK through the host re-export.
`vellum-semantic-service` adds bounded exact-snapshot synchronization,
readiness/capability state, cancellation, stale invalidation and isolated
out-of-order reply routing. `vellum-semantic-client` adds an asynchronous,
zero-authority port already bound to one composition-selected client partition;
its ordinary API cannot retarget another partition and projects no provider
identity. Exact expiry and one bounded pump are a distinct trusted driver port.
`vellum-semantic-session-client` is the separate transport-neutral client for
the background session protocol. It models `NotSent | Reply | OutcomeUnknown`,
retains one exact source-bearing mutation for explicit replay only, validates
incarnation/membership/operation bindings, and derives compact exact-language,
revision, byte-length and UTF-8-boundary witnesses from accepted synchronization.
It has no socket, authentication, grant, broker, provider-selection or automatic
query-retry authority. Its witness-validated result is not the embedded semantic
client's private sovereign seal; trusted composition must bridge that boundary.
`vellum-semantic-session-broker` owns the composition-facing ordered report
algebra without depending on the provider service. It captures one bounded
provider/request expectation per affine effect, validates provider-before plus
observed/settled pre/operation/post evidence against a shadow set, and commits
only exact receipt identities. Composition-evidence mismatch is a latched fatal
divergence, distinct from an ordinary malformed late provider delivery.
`vellum-semantic-session-actor` is the volatile serialized shell around the
pure broker. It owns bounded command and trusted-control mailboxes, keeps affine
continuations private while a synchronous typed port interprets one effect, and
preserves pre-admission `NotSent` versus post-admission `OutcomeUnknown`. Its
owner-loss, drain and stop states are not socket, authentication, persistence or
provider-lifecycle claims. Fatal broker divergence fails the actor and reconciles
admitted uncertainty; a separate affine recovery capability returns the exact
broker and port after worker exit. Construction refusal likewise retains both.
Recovery is cleanup ownership, not client/provider retirement evidence. A future composition must derive Query and
Synchronize roles from separate closed authority actions and retain one exact
authenticated session across the persistent channel.
`vellum-semantic-transport` is the zero-authority byte-frame adapter for that
future channel. It canonicalizes one command, delegates exactly one exchange,
and admits only a canonical reply bound to the command's kind, exchange,
operation, incarnation and membership. Local encoding/capacity refusal is
`NotSent`; every untrustworthy result after delegation is `OutcomeUnknown`.
The underlying frame port remains responsible for proving that its own
`NotSent` means no request byte reached the peer.
`vellum-adapter-semantic-lsp` supplies bounded framing, virtual-document
synchronization, readiness-gated diagnostics and document symbols plus strict
rename planning over an already-owned byte transport. Its composition-owned
dynamic URI issuer proves a path-free opaque grammar while keeping LSP locators
out of the semantic service. Its fallible bridge poll preserves cancelled
correlation and distinguishes runtime, delivery and poison failure; it owns no
process launch or adapter-limit policy.
`vellum-semantic-apply-service` is a trusted composition service: it binds exact
semantic snapshots to presentation-free workspace documents, independently
checks current provider and edit authority, and lowers a reviewed plan into one
atomic in-memory workspace transaction. Its workspace effect still never
persists document files and reports `NotAttempted` explicitly. Its separately
injected append journal may durably retain opaque operation evidence; that
journal neither performs nor implies document persistence. The executable crate-boundary gate
forbids the semantic service/client from
depending on the editor, engine, session or native service and neither crate
contains a filesystem, network or process-launch effect caller.

No semantic crate discovers or launches an LSP/PSI/nREPL provider. A trusted
composition must inject readiness evidence and an already-owned transport;
Rune/Dendrite or another peer retains language-semantic authority. Rename
results themselves remain plans with no Vellum edit authority or atomicity
claim; only the separate apply service can re-authorize and publish them. See
[semantic peer contract](semantic-peer-contract.md).

`vellum-semantic-local` is the one Vellum application allowed to select and
supervise a local provider process. Its operator-installed profile fixes a
canonical absolute executable, argv, cleared/allowlisted environment, private
non-workspace cwd, deadlines and stream bounds. Its reusable library supplies a
process-group-backed byte port and an embedded legacy adapter/service runtime facade.
That facade exposes admission, partition-bound clients, exact expiry, one
bounded fair drive, content-safe state, staged replay-validated provider
replacement and an explicit retirement driver. It has no background loop,
automatic restart policy, remote transport, daemon/session protocol or public
process-control surface. A separate ordered module provides affine timed and
deadline-disabled actor compositions with coordinated broker genesis, exact
receipt/epoch mapping, mandatory scheduler obligations, lossless build/recovery
ownership and cleanup-only exits. Its LSP path consumes a fresh-started adapter
and closes result/document/source/pending/framing limits. Checked and forced
bridge extraction feed consuming provider retirement receipts without permitting
forced abandonment to claim graceful cleanup. The ordered module deliberately
does not yet select or launch a process or prove transport pre-construction
buffer emptiness. The current
binary is a qualification/probe surface, not a daemon protocol or MCP peer.

Local filesystem behavior remains a replaceable adapter, not an engine
intrinsic. A future navigable human/agent product that composes Vellum, Chronos,
chat, and orchestration belongs in a separate product repository.
