# Vellum MCP

`vellum-mcp` is the unpublished, bounded MCP `2026-07-28` stdio peer for an
already-running local Vellum daemon. It is not an editor or semantic authority.
It consumes Vellum only through the public `vellum-sdk-client` facade.

```sh
cargo run --locked -- --socket PRIVATE_DIR/vellum.sock
```

For a daemon started with the local credential profile:

```sh
cargo run --locked -- \
  --socket PRIVATE_DIR/vellum.sock \
  --credential PRIVATE_DIR/credential
```

The harness owns this child process and closes stdin for graceful shutdown.
Without `--credential`, client identity is descriptive and the socket is only
a local-user boundary. The optional private credential establishes a stable
receipt subject and enables the authenticated durable-tool catalog; it grants
no action and does not replace the daemon's independent authority ceiling.

The library also implements three additive restart-stable tools for trusted
compositions: `vellum.durable_execute`, the structurally separate read-only
`vellum.durable_receipt`, and `vellum.durable_retire`. They are neither
advertised nor accepted by `McpServer::new`; enabling them requires
`McpServer<RemoteTransport<AuthenticatedUnixConnector>>::new_authenticated`.
The shipped CLI selects this constructor only when given `--credential`; the
default command above remains owner-only and does not expose these tools.

The implemented surface is deliberately bounded. Current-version
per-request discovery advertises tools plus non-subscribable resources. Legacy
`initialize` is rejected because MCP `2026-07-28` removed that handshake. Content-free
`resources/list` pages expose only scoped workspace/document/root identities;
`resources/read` accepts only canonical `vellum://workspace/...` identities and
selects its scoped document within the same request. Five tools cover snapshot,
replacement, durable replacement, change polling, and authority-mediated
membership. Prompts, tasks, HTTP transport, subscriptions,
semantic-provider queries, and server-to-client requests are not implemented.
Membership admission results are operation-scoped: file results identify only
the admitted document and optional root, while directory results include the
bounded exact added-document identities and joined count. They contain no raw
filesystem locator and make no claim about later membership.

An `outcome_unknown` durable edit returns an opaque operation handle and may be
retried only within the same supervised stdio process lifetime with that handle and otherwise identical
tool arguments. The adapter then replays the exact retained native request
identity; it does not snapshot again or synthesize a second operation. Any
different operation remains fenced until that uncertainty closes.

The authenticated library projection is a different algebra from that `vo1`
register. Before its first effect attempt, the harness must generate and
persist a canonical nonzero 128-bit reference encoded as
`vdo1.` plus exactly 32 lowercase hexadecimal digits. Execute, lookup, and
retirement carry the complete stable intent. Reusing a reference with changed arguments
returns `request_conflict`; pending remains `outcome_unknown`; foreign and
absent references share `absent_or_foreign`. Receipt lookup never falls
through to execution, and retirement calls only the native retirement port.
Catalog presence and authenticated composition do not grant the daemon's
distinct `RetireReceipt` authority. The authenticated intent union includes create-exclusive
document-copy export, but only an exact daemon-installed operation/document/
revision/destination policy can authorize it. Export receipts contain the
document, revision and closed persistence result, never the destination or
source bytes. The exact destination necessarily remains in the caller's
request and durable daemon intent record for conflict-safe lookup, so both are
private retained data. The restart qualification uses a test-only static claim
to exercise the existing authentication seam; it is not a credential design
or a shipped authentication claim.

See the upstream [MCP profile](https://github.com/gantbd/vellum/blob/main/docs/service/mcp.md)
and [threat model](https://github.com/gantbd/vellum/blob/main/docs/security-threat-model.md)
for current bounds and nonclaims. No crate or binary is published yet.
