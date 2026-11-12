# Durable edit composition

Status: implemented local V1 contract; no universal crash-durability claim

## Existing guarantees retained

Ordinary `Replace` remains an in-memory, revision-witnessed edit. It does not
write a native resource and its accepted snapshot is not a durability receipt.
Ordinary `Save` remains an independent operation requiring Persist authority.
It pins the exact document version, asks the local adapter to compare the
original external-resource condition before replacement, requests synchronized
acknowledgment, and preserves application, synchronization, quiescence,
no-future-effects, adapter-failure, cleanup, baseline-publication and unknown
facts. A changed native file is not overwritten or adopted as a new baseline.

The service request register already makes a matching live-process replay return
the retained reply without reauthorization or another effect. A changed command
under the same request identity conflicts, and the client fences a request whose
transport outcome became unknown. The persistent daemon additionally provides
authenticated bounded restart replay for effectful commands through a separate
journal; see [Durable local operation receipts](service/durable-request-replay.md).

## `ReplaceAndSave`

The additive native command/outcome tag `14` composes the existing operations in
one serialized service request:

1. Before any editor effect, the service obtains independent `EditSource` and
   `PersistSource` decisions bound to the same client request, authority context,
   and workspace resource.
2. The replacement consumes the caller's exact client-scoped witness and checks
   its document version, byte range, capacity and source limits.
3. The resulting exact version and fresh witness are published in memory and to
   the bounded change feed.
4. Only then does ordinary synchronized persistence run against that exact new
   version and the adapter's pinned external-resource condition.
5. The accepted receipt preserves the edited snapshot and one closed persistence
   outcome. A post-edit save-admission failure cannot be returned as an apparent
   all-or-nothing refusal.

The persistence sum is:

- `Durable(receipt)`: synchronized write, quiescence, no future effects, clean
  adapter completion and exact baseline publication all succeeded. Reopening
  the same qualified local resource observes the replacement.
- `NotDurable(receipt)`: evidence is terminal, but the complete durability
  conjunction is not established. This name does not assert that no bytes could
  exist; the nested facts remain authoritative.
- `Unknown(receipt)`: application or synchronization is indeterminate, or the
  task cannot prove quiescence/no future effects. It must not be retried under a
  new operation identity without reconciliation.
- `NotAttempted(refusal)`: the edit was applied, but save admission did not
  begin. The exact closed refusal is retained.

Instrumentation counts only content-free fresh-command outcome classes. Exact
replay does not increment the durable-edit counters or dispatch another write.

## Bounds and nonclaims

This composition does not make ordinary `Replace` persistent and does not add
autosave. It is not a filesystem transaction spanning an external writer and
the in-memory history. If the edit succeeds and persistence does not, the live
workspace is intentionally ahead of disk and the receipt says so.

The underlying macOS adapter uses staged same-directory replacement, exact
pre-write external-change checking and file/directory synchronization. These are
strong local operational guarantees, not universal power-loss durability across
all filesystems, storage controllers, network mounts or hostile same-user
processes. The edit witness and ordinary request register are process-local. A
required-authentication persistent daemon can retrieve an exact completed
historical `Save` or `ReplaceAndSave` reply without repeating the write. A
pending intent or indeterminate journal commit fences and requires operator
reconciliation; it is never inferred safe to retry.

Save As, multi-document atomic editing, remote operation, journal compaction,
and distributed outbox delivery remain separate contracts. Hermes's
durable append replay can support a future Vellum-owned outbox, but it cannot
directly persist this session-retired request/reply register without changing
its identity, lifetime, privacy, and authority laws; see the
[Hermes durability seam](service/hermes-durability-seam.md).
