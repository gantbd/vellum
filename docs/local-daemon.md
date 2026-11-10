# Vellum local daemon

Status: bounded macOS Unix-socket profile implemented 2026-11-09; unpublished.

`vellum-daemon` is the standalone composition root for one serialized Vellum
service. It selects the qualified local-filesystem adapter, a shared Workspace,
bounded multi-client service limits, canonical V1 wire encoding, and one-request
Unix-socket exchanges. No semantic crate depends on the daemon or concrete
adapter.

Run from coordinated sibling checkouts:

```sh
cargo run --locked -p vellum-daemon -- --socket PRIVATE_DIR/vellum.sock EXISTING_FILE...
```

The socket parent must already exist, must be a real directory rather than a
symlink, and must grant no group or other permission bits. The daemon refuses
an existing socket path and never deletes it speculatively. After binding, it
sets mode `0600`. Clean Rust shutdown removes only the same socket inode/device
created by the daemon; replacement paths are not removed.

Every connection carries exactly one unsigned-32-bit-length-prefixed V1 frame
and receives at most one reply. Frame, text, field, request, witness, client,
and lifetime-client populations are bounded. Read and write timeouts bound a
connected peer that stops making progress. Malformed requests close only their
connection and cannot become semantic refusals.

Clients explicitly disconnect. Disconnect is idempotent through bounded retired
client tombstones and releases that client's retained requests and witnesses.
Lost disconnect delivery remains an unavailable transport outcome; it is not
reported as successful cleanup. The daemon's client-lifetime ceiling prevents
unbounded reconnect identity growth.

## Operational evidence

Daemon counters report accepted, completed, accept-failed, and request-failed
connections with sticky saturation. Transport counters separately report
exchange completion and I/O, limit, malformed, or service-access failures.
Service counters retain semantic accepted/refused/replay/conflict/stale/limit
populations and live/issued/retired client utilization. None of these counters
authorizes an edit or proves delivery, durability, authentication, or cleanup.

## STRIDE/LINDDUN boundary

- The private socket directory and `0600` socket provide local OS-user
  isolation. They do not authenticate a human, agent, process, or Aegis
  principal; any process under the same OS account may connect.
- Canonical framing, strict decoding, opaque identities, version witnesses,
  replay handling, and exact bounds address malformed/tampered/replayed input
  within the selected profile. They do not provide cryptographic integrity.
- Replies intentionally disclose source. Socket payloads, traffic shape, source
  files, process inspection, and operator logs remain disclosure/linkability
  surfaces. The daemon prints no source, identity, file path, or socket path.
- Volatile request receipts and disconnect tombstones are not durable audit or
  non-repudiation evidence and disappear on restart.
- A default signal termination may leave the socket inode behind because no
  unsafe signal hook is installed. Startup refuses that path rather than
  guessing it is stale or deleting another owner's socket. A supervisor must
  arrange clean shutdown or explicit, independently verified cleanup.
- Consent, purpose, retention, principal authorization, and cross-user or
  remote networking remain outside this profile.

This daemon is suitable for local integration and coordinated GitHub source
distribution, not a crates.io or general remote-service release. `vellum-mcp`
is the peer inbound adapter over `vellum-client`; it calls the same socket
contract as any remote client and receives no daemon internals or host
capabilities.
