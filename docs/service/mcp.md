# Vellum MCP adapter

Status: bounded MCP `2026-07-28` stdio profile implemented 2026-11-09;
unpublished.

`vellum-mcp` is the agent/harness peer to the TUI and GUI. It is not an editor,
service, or canonical protocol. It launches under an MCP host, speaks one
newline-delimited JSON-RPC message per line on standard input/output, and uses
the client and transport projections from the public `vellum-sdk-client`
facade to connect to an already-running local `vellum-daemon`.

```sh
cargo run --locked -p vellum-mcp -- --socket PRIVATE_DIR/vellum.sock
```

When the daemon selects the local credential profile, the MCP peer selects the
matching provider with a path-only argument:

```sh
cargo run --locked -p vellum-mcp -- \
  --socket PRIVATE_DIR/vellum.sock \
  --credential PRIVATE_DIR/credential
```

The credential-enabled binary uses the authenticated MCP constructor and thus
advertises the restart-stable durable execute/receipt/retirement tools. Omitting
`--credential` preserves the owner-only connector and five-tool catalog. The
adapter loads no credential bytes from argv or environment and never emits the
configured path or record fields.

An MCP host should supervise or otherwise arrange the daemon separately, then
spawn this adapter as its stdio server. A host-neutral configuration shape is:

```json
{
  "mcpServers": {
    "vellum": {
      "command": "/absolute/path/to/vellum-mcp",
      "args": ["--socket", "/absolute/private/path/vellum.sock"]
    }
  }
}
```

The harness owns the child process and closes stdin for graceful shutdown. The
daemon owns the workspace and may concurrently serve TUI, GUI, MCP, or other
native clients once those peers migrate to the same client port.

The implemented surface is deliberately closed:

- `server/discover`, advertising only `2026-07-28`, fixed tools, and
  non-subscribable resources. Legacy `initialize` is always rejected because
  the modern stateless protocol removed that handshake;
- `resources/list`, returning one native inventory page of content-free
  document/root descriptors and an opaque bounded continuation;
- `resources/read`, accepting only canonical scoped `vellum://workspace/...`
  URIs. Each document read selects its URI identity within that same request;
  root reads return only their file/directory kind, never their locator;
- `tools/list`, returning a deterministic five-tool catalog with no pagination;
- `vellum.snapshot`, returning text and a workspace revision;
- `vellum.replace`, replacing an exact UTF-8 byte range only against that
  revision;
- `vellum.replace_and_save`, preserving the native durable, not-durable,
  unknown, and not-attempted persistence sum;
- `vellum.poll_changes`, returning at most one content-free event, an opaque
  continuation, or an explicit retention gap;
- `vellum.membership`, projecting add/remove operations only through the
  daemon's independent authority decision. Presence in the catalog is not a
  grant; the read-only daemon returns `authority_denied` without an effect.
  File admission returns only its exact document, optional new root, and
  added/already-present status. Directory admission returns its exact root,
  added/joined counts, and the complete bounded added-document identity list in
  admission order. Neither response substitutes a later whole-workspace
  inventory or exposes a locator.

The MCP library and credential-enabled CLI have an authenticated-only catalog
with `vellum.durable_execute`, the structurally separate read-only
`vellum.durable_receipt`, the content-free read-only `vellum.durable_status`,
and `vellum.durable_retire`. The ordinary constructor
and owner-only CLI invocation neither advertise nor accept these tools. A trusted embedding must
construct the server over `RemoteTransport<AuthenticatedUnixConnector>` and
call `McpServer::new_authenticated`; this is a type-level composition
requirement, and the shipped CLI selects that same composition only when given
an explicit descriptor-anchored credential path.

The harness generates and persists a nonzero 128-bit operation reference
before the first execute, encoded canonically as `vdo1.` followed by 32
lowercase hexadecimal digits. The request repeats one closed stable intent:
file/directory admission, explicit document/root retirement, explicit
revision save, explicit revision/range replacement-and-save, or an explicit
document/revision/create-exclusive destination copy export. Export uses the
daemon's independently installed exact operation/document/revision/absolute
UTF-8 destination policy; catalog presence is not authority. Lookup requires
the identical operation and intent, including the export destination. Changed
intent is `request_conflict`, pending is `outcome_unknown`, and absence and
foreign ownership are the same `absent_or_foreign` result. The lookup handler
calls only the native lookup port and never executes on absence. Retirement
requires the same exact nonzero operation reference and identical stable
intent, calls only the native retirement port, and returns the closed
`retired`, `absent_or_foreign`, `outcome_unknown`, `unavailable`, or
`request_conflict` algebra. Tool discovery and authentication grant no
authority: the daemon must independently admit its distinct `RetireReceipt`
action before a complete receipt can be mutated.

Status takes only the canonical operation reference and returns pending,
complete, retired, absent-or-foreign, or unavailable. The daemon binds that
classification to the active authenticated subject and workspace scope.
Foreign and absent targets coalesce, and status neither reveals intent/receipt
content nor falls through to execute, full lookup, or retirement.

An export response is locator- and content-free: it contains only the scoped
document, source revision and the closed persistence/failure sum. Destination,
source bytes, operation identity, subject, session and authority evidence are
absent from responses, discovery, Debug, errors and metrics. The canonical
intent necessarily retains the literal destination in the private plaintext
journal and absorbing tombstone; encryption, unlinkability and backup erasure
are not claimed.

The durable-execute projection understands the native absorbing `retired`
terminal. `vellum.durable_retire` exposes the exact authenticated retirement
lifecycle without widening authority or falling through to lookup/execute.

Every request must carry the current protocol version and per-request client
capabilities in `_meta`. Legacy `initialize` receives an unsupported-version
error that names the supported revision. Tool inputs reject unknown fields.
Native semantic refusals are closed `isError` tool results; JSON-RPC framing,
method, and parameter failures remain protocol errors. Access failures remain
distinct from semantic refusals. Per modern resource semantics, an unknown
document/root URI is JSON-RPC `-32602` with a closed native data code; authority
and service access failures remain in the bounded server-error class.

Resource URIs contain only nonzero decimal scoped identities and cannot carry a
path, query, fragment, capability, witness, or authority. Inventory and change
cursors use separate bounded opaque token families; malformed, foreign, stale,
and invalid continuations remain distinct protocol/native refusals. A change
gap is accepted evidence that callers must relist resources and restart polling,
not a fabricated complete history.

Prompts, subscriptions, tasks, Streamable HTTP, semantic-provider tools, and
server-to-client requests remain absent.

## Bounds and operating evidence

Input is drained through a bounded line reader rather than an allocating
`read_line`. A nonempty EOF fragment is rejected as truncated because stdio
messages require newline delimiters. The reference profile admits at most
262,144 request bytes and 524,288 response bytes. Native wire, insertion,
snapshot, request, witness, client, and client-lifetime bounds remain
independently enforced.

`McpServer::usage` provides saturating content-free request/response byte
counts, method rates, protocol/tool error counts, and tool-call total/maximum
latency in microseconds. These are RED/golden-signal observations only. They do
not affect native admission or authorization, and identity, source, inserted
text, paths, witnesses, arguments, results, and error text are never metric
dimensions.

Authenticated durable projection adds only bounded counters for executes,
lookups, retirements, complete/retired evidence, conflicts, pending/unknown,
absent-or-foreign, and unavailable outcomes. Retirement has an exhaustive
fixed-cardinality outcome partition. Operation references and intent content
are never metric dimensions.

The stdio loop is sequential and native calls are synchronous and bounded.
Cancellation notifications produce no response but cannot preempt an operation
already executing in that same loop; no asynchronous-cancellation claim is
made. EOF is the graceful shutdown signal and triggers explicit native-client
disconnect.

## STRIDE/LINDDUN boundary

- MCP `clientInfo` is self-asserted description, never authentication or
  authorization. The private daemon socket is only a same-OS-user boundary.
- Each MCP replacement carries its expected workspace revision. The adapter
  takes a fresh native snapshot in that request, derives its client-scoped
  witness, and submits the witness-bound native edit. Thus MCP semantics do not
  depend on a witness retained by an earlier stdio request or process lifetime.
  Native request identities still make retained native replay explicit. MCP
  request IDs are correlation, not native idempotency keys; retrying a
  replacement may receive `stale`.
- A durable edit that loses its reply returns an opaque operation handle and
  retains both its exact MCP argument tuple and exact native command/request
  identity for this one supervised stdio process lifetime. Only a subsequent call with
  that handle and otherwise identical arguments replays the request. A different call
  receives `service_outcome_unknown`; no second edit is manufactured.
- That `vo1` mechanism is not the native cross-process durable-operation
  reference. The default MCP invocation uses the owner-only connector and
  cannot recover after its own restart. The authenticated projection requires
  the harness-supplied 128-bit operation reference on the first write and uses
  the required-auth connector; it never fresh-executes after an absent lookup.
  The credential-enabled CLI selects this projection through the
  descriptor-anchored local profile.
- The real daemon/MCP restart qualification deliberately discards the first
  MCP response, terminates both MCP client and daemon, then retrieves the
  historical receipt after both restart while proving zero second effects.
  The export case uses the real create-exclusive local-filesystem effect and
  additionally proves unchanged source text, revision, binding and membership,
  one destination publication, changed-destination conflict and absence of the
  destination from the receipt. Foreign-subject non-disclosure remains
  indistinguishable from genuine absence through the production credential
  adapter. This scenario currently composes the production daemon, connector
  and MCP constructor in process; separate shipped-process qualification is
  tracked explicitly.
- Resource enumeration and change observations disclose workspace shape,
  scoped identities, revisions, and traffic timing. They never disclose root
  locators or source text. A selected document read deliberately discloses its
  source to the invoking harness.
- Snapshots and successful replacements intentionally disclose source to the
  invoking harness. Stdio, process inspection, traffic shape, and the harness's
  own transcript or logs remain disclosure and linkability surfaces.
- Fixed method/tool catalogs, closed schemas, byte ceilings, canonical native
  framing, and bounded daemon state constrain malformed input and exhaustion.
- Volatile counters and native retained replies are not durable audit evidence
  or non-repudiation. Human consent, purpose, retention, and per-principal
  grants belong to the host/orchestration layer.

This is a local stdio adapter, not a remotely exposed MCP resource server. A
future HTTP boundary must independently earn TLS, OAuth resource-server,
origin, rate, admission, cancellation, audit, and privacy semantics.

## Distribution

No crate is published or pushed to crates.io. Install the connector from the
source checkout with:

```sh
cargo install --path connectors/vellum-mcp
```

A single-URL `cargo install --git` claim waits for the Vellum release commit
and pinned Keystone object to be reachable from their declared remotes. The
transitive dependency graph already uses an exact Git revision.
The qualified extraction boundary and remaining repository/workflow blockers
are recorded in [`../mcp-extraction.md`](../mcp-extraction.md).

Normative protocol references are the official MCP
[versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning),
[stdio transport](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio),
and [tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
specifications.
