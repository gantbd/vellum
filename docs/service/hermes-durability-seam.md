# Hermes durability seam

Status: architecture comparison against the local Hermes and Keystone workspace
contracts, 2026-11-10. No Vellum adapter is selected by this note.

Hermes now has a concrete bounded ledger, receipt-gated host, and exclusive
local-filesystem delivery profile in the broader workspace. That makes it a
credible future persistence mechanism for Vellum events, but not a drop-in
replacement for either of Vellum's current volatile relations. The domain
codec, authority policy, retention policy, and recovery interpretation must
remain Vellum-owned.

## Exact contract comparison

| Law | Vellum request retention | Vellum workspace feed | Hermes local durable profile |
| --- | --- | --- | --- |
| Record identity | `(ClientId, RequestId)` plus the exact `RequestV1` and retained `ReplyV1` | Workspace, process epoch, monotone feed revision, and a closed content-free change kind | `(ProducerId, IdempotencyKey)` plus exact stream and complete canonical envelope; accepted events receive a stream sequence |
| Lifetime | Removed when its client disconnects; bounded with no eviction while that client remains | Fixed ring; oldest events are evicted deterministically | No eviction or compaction; the configured event and idempotency ceilings become terminal refusals |
| Replay conflict | Same key and request returns the original reply; changed request conflicts | Replaying the poll request is handled by the separate request relation and does not republish | Same producer key, stream, and envelope replays; any changed stream or envelope conflicts before predecessor comparison |
| Cursor | None; request identity is not a delivery cursor | Caller-carried workspace/epoch/revision cursor; a retention loss is an accepted explicit `Gap` | Persisted independent `(ConsumerId, StreamId)` monotone cursor; regression and advancement beyond the published edge refuse |
| Durability point | None across restart | None across restart | A whole canonical checkpoint is published only after an exact next-revision durable receipt |
| Unknown outcome | Transport uncertainty fences the native client request; daemon restart loses the register | Restart changes the epoch and rejects the old cursor | Any error, refusal, indeterminate result, weak receipt, or wrong receipt after a store offer fences all ledger reads and mutations; reopen observes the actual winner |
| Filesystem publication | Not applicable | Not applicable | Keystone's directory store creates a private exclusive leaf, writes and synchronizes a stage, atomically renames it over the head, then synchronizes the directory; macOS also requires `F_FULLFSYNC` for the file |
| Recovery | New process starts with no retained request replies | New process starts an empty feed under a different epoch | Existing state only: complete checkpoint, replay index, and consumer cursors are decoded under host limits; missing, corrupt, weak, or revision-mismatched state refuses without reset |
| Authority | Exact request-bound Vellum authority precedes fresh interpretation; client identity is scoped by the service | `InspectWorkspace` authority controls polling; events intentionally omit client/request/subject data | Producer and consumer IDs are explicitly not authentication or authority; the embedding host must establish both |
| Privacy | Replies may contain source; retention ends with the client, absent other owners | Events omit source, paths, clients, requests, witnesses, subjects, and sessions | Payload is arbitrary plaintext and persists in whole checkpoints; metrics redact identity and payload, but the domain must minimize payload and set access/retention policy |

The two-consumer semantics therefore do **not** close under a direct
substitution:

- Persisting Vellum's existing request register in Hermes would change
  disconnect retirement, capacity behavior, authority/session binding, and the
  meaning of a producer key. It would also persist source-bearing replies
  without a declared privacy-retention contract.
- Projecting the current feed directly onto a Hermes stream would change
  eviction and `Gap` behavior. Hermes's present no-compaction profile cannot
  prove the feed's bounded-retention law, while the feed cannot prove Hermes
  consumer acknowledgement or restart recovery.
- A Hermes consumer acknowledgement says only that its cursor advanced. It is
  not evidence that an agent, connector, or downstream side effect completed.

## Truthful composition

A future durable Vellum pathway should define a separate, Vellum-owned journal
schema and interpreter with these steps:

1. Authenticate the connection and authorize the exact Vellum action before
   constructing a journal record. Hermes identifiers remain correlation, not
   grants.
2. Encode a bounded, versioned Vellum record. Minimize it to the information
   needed for reconciliation; do not copy source-bearing replies by default.
3. Append under an exact Vellum operation identity and expected predecessor.
   Release success only after Hermes returns its receipt-gated durable result.
4. On an indeterminate append, fence the Vellum operation and reopen the ledger.
   Resolve the original operation by exact-key replay; never issue a fresh key
   or recompute an effect automatically.
5. Keep delivery consumers distinct from effect completion. A consumer that
   performs an effect needs its own idempotency and receipt law before its
   Hermes cursor may advance.
6. Define retention/compaction before promising a durable replacement for the
   bounded feed. Until Hermes has an explicit compaction and unavailable-range
   profile, a durable stream is an outbox/audit-shaped source, not the current
   page/gap feed.

Keystone's `CheckpointStore` and directory adapter are mechanism-level pieces:
they can carry a Vellum-owned whole-state checkpoint, but `Committed` does not
validate Vellum schema, replay meaning, authority, or recovery. The trusted
Vellum host must bind the storage revision into its canonical record, validate
receipt strength and exact revision, publish the candidate only afterward, and
fence on every uncertain offered commit. Reusing Hermes's already-qualified host
is preferable once the Vellum event interpretation is closed; independently
rebuilding the same transaction protocol in the service would duplicate a
cross-cutting law.

This mechanism boundary has genuine second-consumer evidence: Aegis uses the
same Keystone checkpoint/directory contracts while retaining a different fixed
authority schema, mandatory epoch advance on reopen, and grant-revocation
recovery law. That is evidence for sharing the storage mechanism, not for
sharing either domain checkpoint. Moira separately qualifies mapping one of its
effect identities to a Hermes producer idempotency key, but only against the
volatile Hermes core; its current test is useful event-identity precedent, not
durable Vellum integration or restart evidence.

## Adoption gate

The inspected Hermes host/local adapter and Keystone directory/replay work are
currently workspace changes beyond their repositories' recorded revisions.
Vellum must not add a distributable path dependency or claim GitHub-source
closure until those public contracts are committed at immutable revisions and
the coordinated source archive includes them. Adoption also requires:

- a Vellum journal schema and golden compatibility fixtures;
- bounded recovery, full-capacity, replay-conflict, and lost-receipt tests;
- restart tests with at least two independent consumers;
- STRIDE/LINDDUN review for persisted operation correlation and source-derived
  payloads;
- explicit retention, compaction, and operator-recovery policy; and
- release/supply-chain qualification of the pinned Hermes and Keystone inputs.

Until those gates close, Vellum's request register and workspace feed remain
truthfully process-local, and any Vellum-owned durable journal remains a separate
composition rather than being mislabeled as either one.
