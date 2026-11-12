# Durable local operation receipts

Status: implemented for the persistent macOS daemon with required
authentication; bounded local plaintext, not distributed exactly-once

The standalone daemon has a Vellum-owned request journal for effectful native
commands: file/directory admission, membership retirement, `Save`, and
`ReplaceAndSave`. This journal is separate from the service's volatile request
register. It survives disconnect and process restart; the ordinary register
remains client-session state and is reclaimed on disconnect.

## Closed ordering

For a fresh authenticated effectful request, the daemon:

1. Canonically encodes and durably synchronizes the exact request intent.
2. Executes the ordinary service command and retains its closed reply.
3. For accepted membership change, durably commits workspace identity before
   advancing.
4. Canonically encodes and durably synchronizes the exact reply.
5. Only then allows the reply to cross the transport.

An intent- or receipt-write refusal/indeterminate outcome suppresses the reply
and fences the daemon. A complete exact pair is recoverable. An intent without
a receipt, corruption, truncation, trailing bytes, unsafe mode/owner/type/link
state, symlink, future generation, or conflicting pair refuses restart before
the data socket is published. Vellum never infers failure from incomplete
intent and never repeats that effect.

Capacity is fixed with no eviction or compaction. A new operation at capacity
receives request-capacity refusal before an effect. Disconnect does not delete
entries. The required authenticated retirement contract, crash ordering and
qualification gates are fixed in
[`durable-receipt-lifecycle.md`](durable-receipt-lifecycle.md); its storage
mutation is not implemented yet because the current protocol has neither a
receipt resource/action nor an authenticated authority-bound operation. The
daemon does expose aggregate occupancy/capacity/saturation so an operator can
detect the limit before availability is lost.

## Identity and disclosure

Durable receipt creation and retrieval are disabled in `OwnerOnlyOffline`.
Unix UID and socket reachability are not principal evidence. Required
authentication records only its stable subject reference, never session,
challenge, claim, or credential. Restart retrieval requires new successful
authentication resolving to that subject and the exact workspace, historical
generation, client, request, and canonical command bytes.

A different subject receives the ordinary inactive-client result, identical to
an absent record, and cannot enumerate another subject's receipts. Matching
retrieval is historical disclosure, not fresh action authorization. The
original fixed action-ceiling decision remains frozen; dynamic revocation and
policy revalidation are not claimed.

Recovered client ordinals are reserved before endpoint publication. New
connections allocate above every retained owner, so a new session cannot
inherit a receipt by ordinal reuse. Historical retrieval does not require the
old service client to be active; fresh requests still do.

Workspace scope is exact and restart-stable. A record also retains its exact
pre-operation identity generation. Recovery admits only the same scope and a
generation no later than the recovered workspace generation. This preserves a
membership receipt after its checkpoint advances generation; it cannot
transpose a record between workspace incarnations.

## File format and privacy ceiling

The local adapter uses an owner-only `0700` directory and one `0600`, singly
linked regular intent file plus receipt file per operation. Records contain a
fixed magic/version, workspace scope/generation, client/request ordinals,
authenticated subject reference, exact canonical request, and—for receipts—the
exact canonical reply. Entry, request, reply, and encoded sizes are bounded.
Creation is exclusive and data plus directory entries are synchronized,
including `F_FULLFSYNC` in the qualified macOS profile.

The format is plaintext: not encrypted, signed, authenticated, or sanitized.
`ReplaceAndSave` receipts contain returned source snapshots; requests contain
inserted source and filesystem locators. Permissions reduce accidental
disclosure but do not isolate hostile same-user processes. Backups, forensic
remnants, media sanitization, credential compromise, deletion, expiry, and
legal-purpose policy are outside this contract. Metrics/errors expose only
closed classes and counts, never bytes, subjects, paths, or source.

This is local crash-safe receipt retrieval under stated premises. It is not
distributed exactly-once, remote replay, durable audit/non-repudiation, Hermes
interoperability, automatic pending-effect reconciliation, or a universal
power-loss guarantee.

The current raw historical-request recovery is not the product resume API.
The required read-only, non-executing SDK/MCP shape is specified in
[durable operation resume contract](durable-operation-resume-contract.md).

## Operation-reference format

The product resume path uses the separate
`.vellum-durable-operation-v2` relation. Its key is the authenticated subject
plus the fixed 128-bit operation reference, so separate subjects may safely use
the same opaque reference without creating an existence oracle. Presence of the
superseded `.vellum-durable-operation-v1` directory refuses recovery and new
writes; no implicit migration or reinterpretation is performed. The stable
workspace identity and
canonical stable intent contain no transport client/request identity, selected
view, or session witness. Pending intents recover as inspectable
`OutcomeUnknown`; unlike the legacy relation, their presence does not refuse
startup. Complete records carry the canonical stable durable outcome. Startup
decodes both halves and validates the closed intent/receipt correspondence,
including exact workspace/target and source-revision relation.

The two on-disk formats are never reinterpreted or migrated implicitly. The
daemon composition applies one shared `max_entries` ceiling across legacy and
operation-reference records, including pending intents. Its aggregate
utilization gauge therefore reports retained journal slots, not only complete
receipts. The adapter directories are separately versioned mechanisms; direct
adapter use does not weaken the daemon's shared-capacity law.
