# Vellum MCP adapter

Status: bounded MCP `2026-07-28` stdio profile implemented 2026-11-09;
unpublished.

`vellum-mcp` is the agent/harness peer to the TUI and GUI. It is not an editor,
service, or canonical protocol. It launches under an MCP host, speaks one
newline-delimited JSON-RPC message per line on standard input/output, and uses
`vellum-client` plus `vellum-transport` to connect to an already-running local
`vellum-daemon`.

```sh
cargo run --locked -p vellum-mcp -- --socket PRIVATE_DIR/vellum.sock
```

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

- `server/discover`, advertising only `2026-07-28` and fixed tool capability;
- `tools/list`, returning a deterministic two-tool catalog with no pagination;
- `vellum.snapshot`, returning text and a workspace revision;
- `vellum.replace`, replacing an exact UTF-8 byte range only against that
  revision.

Every request must carry the current protocol version and per-request client
capabilities in `_meta`. Legacy `initialize` receives an unsupported-version
error that names the supported revision. Tool inputs reject unknown fields.
Native semantic refusals are closed `isError` tool results; JSON-RPC framing,
method, and parameter failures remain protocol errors. Access failures remain
distinct from semantic refusals.

MCP resources are intentionally absent. The native profile does not yet expose
document enumeration or stable document identity, so publishing a fabricated
“active document” resource would borrow presentation state and break
categorical closure. Prompts, subscriptions, tasks, Streamable HTTP, and
server-to-client requests are also absent.

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

No crate is published or pushed to crates.io. Today, installation is from a
coordinated sibling checkout because exact path dependencies preserve the
repository boundaries during development:

```sh
cargo install --locked --path crates/vellum-mcp
```

A single-URL `cargo install --git` claim waits until the cross-repository
Keystone/Vellum dependency graph has a reproducible Git resolution or release
bundle. The current repository must not advertise that command prematurely.

Normative protocol references are the official MCP
[versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning),
[stdio transport](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio),
and [tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
specifications.
