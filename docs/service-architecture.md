# Vellum service architecture

Status: bounded multi-client local-daemon profile implemented on 2026-11-09.

## Dependency direction

```text
vellum-client -> vellum-protocol <- vellum-service
      |                    ^             |
vellum-transport -> vellum-wire     vellum-editor
      |                                  |
vellum-daemon ----------------> vellum-session
      |                                  |
local-fs adapter              vellum-host -> vellum-engine
```

`vellum-host` contains trusted effect ports. `vellum-session` binds an engine
owner to explicitly supplied host capabilities. `vellum-editor` provides both
the presentation-free shared `Workspace` owner and client-owned `EditorView`
projections alongside the richer headless `Editor` used by presentations.
`vellum-protocol` contains closed client/service values and has no engine,
editor, adapter, or transport dependency. `vellum-wire` maps those values to a
canonical bounded V1 encoding using only Keystone's bounded codec cursor; it
selects no socket, runtime, authentication, or daemon policy.
`vellum-transport` adds bounded one-request framing and a remote `ServicePort`
interpretation. `vellum-daemon` is the only composition root here: it selects a
private Unix socket and the qualified local-filesystem adapter.

The service is the sole serialized owner of its workspace. Every registered
client receives an independent view, request namespace, and witness namespace.
The client assigns monotone request identities and does not reinterpret
refusals.

## Implemented profile

The current protocol supports:

- connecting an explicitly bounded population of clients;
- obtaining a content-redacted-debug snapshot with an opaque version witness;
- replacing an exact UTF-8 range against that witness;
- replaying an identical request without repeating the edit;
- refusing conflicting request-identity reuse;
- allowing the same request identity in different client namespaces;
- refusing a version witness presented by a different client;
- refusing stale, unknown, exhausted, oversized, or invalid requests;
- bounding retained requests, witnesses, snapshot bytes, and insertion bytes;
- canonical request/reply encoding with golden fixtures and closed decoding;
- explicit idempotent disconnect with bounded retired-client tombstones and
  reclamation of that client's requests and witnesses;
- bounded Unix framing, remote-client access errors, I/O timeouts, and a local
  daemon composition.

The service projects the replacement before publication and reserves its
retained response slot before changing the workspace. It supplies the engine
with the exact expected source fragment from the retained witness. No cursor,
selection, active tab, filesystem path, or engine identity crosses the
protocol.

## Operational evidence

`Service::usage` exposes saturating, content-free counters for connection and
request rate, accepted/refused results, replay, conflict, stale, invalid,
limited, and unavailable outcomes. It also reports retained request/witness
utilization against configured capacity. Saturation is sticky and explicit.

This is RED/USE evidence, not semantic authority. Counters cannot affect
admission, replay, editing, or refusal. No latency signal is fabricated: a
qualified clock and measurement boundary must be supplied before duration can
be claimed. Process/service health, dependency health, traffic, errors, and
saturation are the initial golden signals; filesystem and editor details stay
behind their own evidence boundaries.

## Threat and privacy posture

The following STRIDE/LINDDUN review closes the semantic service profile:

- spoofing and elevation: client IDs are correlation handles, not
  authentication or authorization; no host capability crosses the protocol;
- tampering and replay: version witnesses bind edits to retained source and
  request IDs make identical replay inert while conflicting reuse refuses;
- repudiation: volatile retained replies are replay evidence, not a durable
  audit log or non-repudiation claim;
- information disclosure: snapshots intentionally disclose source to the
  connected caller, while IDs and `Debug` output redact content and identity;
- denial of service: request, witness, snapshot, insertion, and client bounds
  are explicit, with saturation visible rather than silently wrapped;
- linkability, identifiability, and detectability: opaque process-local IDs and
  content-free counters reduce incidental disclosure, but traffic observation
  and the source-bearing reply remain visible to the embedding process;
- unawareness, consent, and policy: this layer cannot infer human consent,
  purpose, retention policy, or legal basis; those belong to the authenticated
  product/orchestration boundary.

The local daemon analysis is recorded in [local-daemon.md](local-daemon.md).
MCP and any network transport must repeat the analysis for credentials,
authorization, transport metadata, logs, cancellation, and cross-process
retention. The local-user review must not be inherited as a network claim.

## Explicit non-claims

This profile does not yet claim:

- principal authentication, Aegis authorization, durable client registration,
  reconnect, or session-resumption semantics;
- TCP, HTTP, JSON-RPC, cross-user, or network transport;
- MCP conformance;
- event subscriptions, recovery, or durable request receipts;
- cross-process replay after restart;
- background coexistence with the current directly embedded TUI or GUI;
- platform qualification beyond the existing application and adapter profiles.

The current protocol operates on the initial document in each independent
view; document enumeration and explicit view selection have not yet crossed
the protocol boundary. The richer legacy `Editor` still combines its own UI
state with a host for directly embedded TUI/GUI use. Those applications have
not yet migrated through `vellum-client` and make no background-service parity
claim.

## Next earned boundaries

1. Add durable registration/reconnect and bounded event delivery.
2. Migrate the TUI and macOS GUI through the client port.
3. Add `vellum-mcp` as a thin peer adapter over `vellum-client`.

The MCP adapter will expose semantic resources and tools; it will not accept
keystrokes, borrow a human view, or receive raw host capability objects.
