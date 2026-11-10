# Vellum service architecture

Status: first bounded in-process profile implemented on 2026-11-09.

## Dependency direction

```text
vellum-client -> vellum-protocol <- vellum-service
                                      |
                                vellum-editor
                                      |
                                vellum-session
                                      |
                     vellum-host -> vellum-engine
```

`vellum-host` contains trusted effect ports. `vellum-session` binds an engine
owner to explicitly supplied host capabilities. `vellum-editor` provides the
headless editor application behavior used by presentations. `vellum-protocol`
contains closed client/service values and has no engine, editor, adapter, or
transport dependency.

The service is the sole serialized owner of its editor. The client assigns
monotone request identities and does not reinterpret refusals.

## Implemented profile

The current protocol supports:

- connecting one explicitly bounded client;
- obtaining a content-redacted-debug snapshot with an opaque version witness;
- replacing an exact UTF-8 range against that witness;
- replaying an identical request without repeating the edit;
- refusing conflicting request-identity reuse;
- refusing stale, unknown, exhausted, oversized, or invalid requests;
- bounding retained requests, witnesses, snapshot bytes, and insertion bytes.

The service projects the replacement before publication and reserves its
retained response slot before changing the editor. It supplies the engine with
the exact expected source fragment from the retained witness. No cursor,
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

The current STRIDE/LINDDUN review closes only the in-process profile:

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

A daemon or MCP adapter must repeat this analysis for framing, decoding,
credentials, authorization, transport metadata, logs, cancellation, and
cross-process retention. This review must not be inherited as a remote claim.

## Explicit non-claims

This profile does not yet claim:

- multiple simultaneous clients or per-client editor views;
- a daemon, socket, HTTP, JSON-RPC, or stable wire encoding;
- authentication or Aegis authorization;
- MCP conformance;
- event subscriptions, recovery, or durable request receipts;
- cross-process replay after restart;
- background coexistence with the current directly embedded TUI or GUI;
- platform qualification beyond the existing application and adapter profiles.

Until editor presentation state is decomposed, the service enforces
`max_clients == 1`. It must not simulate multi-client support by sharing one
cursor or by creating independent editors over the same files.

## Next earned boundaries

1. Split shared workspace/document ownership from per-client `EditorView` state.
2. Extend the service with bounded client registration and independent views.
3. Define a canonical versioned wire codec with golden fixtures.
4. Add a local daemon transport and make `vellum-client` transport-neutral in
   deployment as well as type shape.
5. Migrate the TUI and macOS GUI through the client port.
6. Add `vellum-mcp` as a thin peer adapter over `vellum-client`.

The MCP adapter will expose semantic resources and tools; it will not accept
keystrokes, borrow a human view, or receive raw host capability objects.
