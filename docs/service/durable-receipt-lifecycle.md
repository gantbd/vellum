# Durable receipt lifecycle

Status: native protocol, wire, client, service/daemon composition,
descriptor-anchored local storage and authenticated MCP projection implemented

The identity is `(server-bound stable workspace, freshly authenticated subject,
operation reference, exact canonical intent)`. Client/request ordinals are live
correlation only and generation is internal provenance, not a discriminator.
Retirement commits a permanent absorbing tombstone. Exact delayed execute
returns `Retired`, exact lookup returns `AbsentOrForeign`, and changed intent
conflicts. Active receipt capacity and bounded monotone retired-identity
capacity are separate.

The fixed-capacity request journal prevents invisible eviction. Active receipt
capacity can now be reclaimed lawfully, while monotone tombstone saturation is
an explicit safe terminal until provisioning or a separately authorized new
workspace incarnation. Future epoch rotation remains a nonclaim; availability
must not silently weaken replay, uncertainty, identity, or privacy laws.

## Why retirement is not adapter deletion

The journal adapter can compare stored subject bytes, but those bytes are
identity evidence, not a bearer credential or authority. A public
`delete(client, request, subject)` method would let any same-process caller
forge the subject argument. The control socket's owner boundary alone does not
establish the required authenticated subject or receipt-retirement authority.

Consequently Vellum must not expose an adapter-only delete, age-based expiry,
least-recently-used eviction, startup purge, or unauthenticated operator reset.
Hermes acknowledgement is also not substitutable: it advances an independent
consumer cursor and does not prove that this subject abandons exact replay of a
Vellum filesystem effect. Keystone's checkpoint mechanism may eventually store
the transition, but it supplies neither this authority nor this domain law.

## Minimal closed operation

The additive `DurableReceiptRetirementV1` operation and distinct
`RetireReceipt` action use the corrected exact target above. The current local
profile requires fresh authentication for passive owner-scoped classification.
Mutation of a complete receipt additionally requires the independent global
fixed action ceiling to contain `RetireReceipt`; this is not dynamic
per-principal authorization. Possession of the target alone grants nothing.

The disclosure-sensitive terminal results are:

- `Retired` means a complete receipt owned by the authenticated subject became
  durably unreachable for replay.
- `AbsentOrForeign` coalesces absence, an already-retired receipt and a receipt
  belonging to another subject. It is idempotent and prevents an existence or
  principal oracle.

`RequestConflict`, `OutcomeUnknown`, and `Unavailable` separately preserve
changed-input, pending/indeterminate, and unavailable/authority/capacity facts.

A pending intent is never eligible. It continues to fence recovery because the
effect outcome is unknown. Provenance from a future generation is refused, but
an exact receipt from an older generation remains targetable after membership
advances. Retirement never releases or reassigns the operation reference.

## Crash-closed storage transition

Retirement is a three-state durable transition, not two unrelated unlinks:

1. Validate the private directory and exact complete intent/reply pair. Check
   workspace, historical generation provenance, operation, canonical bytes and
   authenticated subject before preparing any mutation.
2. Exclusively create and synchronize a private retirement marker containing
   the exact target and bounded exact canonical intent. Synchronize the
   directory. This is the retirement commit
   point.
3. Validate all surviving active components, remove the reply and intent,
   synchronize the directory, and retain the tombstone permanently.

Recovery recognizes a valid committed tombstone and finishes every attributable
cleanup subset. It validates all surviving components before any unlink.
A marker without the exact attributable pair, malformed/unsafe storage, an
unmarked partial pair, or an indeterminate synchronization outcome fences. No
state is guessed from absence. Marker commit absorbs the identity, but active
capacity releases and a successful reply crosses transport only after validated
cleanup and the step-3 directory sync. Cleanup uncertainty fences the live
daemon and restart finishes it. A lost reply is safely replayed as
`AbsentOrForeign` because committed retirement is absorbing.

Only validated cleanup and its directory sync release active receipt capacity.
Tombstones consume a separate bounded monotone lifetime capacity and are never
reused, closing ABA for caller-chosen 128-bit operation references.

Active-receipt compaction is just a bounded batch of these independently
authorized retirements. It never selects records by age or pressure and never
reclaims tombstone identity capacity. Epoch rotation and
operator migration are separate future operations: they require explicit
authority over every abandoned receipt, proof that no pending intent exists,
an immutable old epoch, a new non-reused epoch/workspace binding, crash-safe
publication and rollback detection. Renaming or deleting the journal directory
is not rotation.

## Availability and observability

The daemon publishes content-free active receipt and retired identity used,
capacity and saturation gauges in its passive in-process USE/golden-signal
snapshot. The byte-compatible external V1 status frame does not yet project the
retired-identity gauge. Operators can alert before capacity is exhausted;
the dimensions cannot identify a subject, operation, path or source. The
existing full/refusal/failure/replay counters remain RED evidence. The supported
response to retired-identity saturation is to provision a larger bounded journal
before startup or perform a future explicitly authorized epoch rotation; manual
file deletion is unsupported.

## Required qualification

Implementation is gated on all of the following deterministic behaviors:

- same-subject, separately authenticated mutation succeeds only with explicit
  `RetireReceipt` authority; authentication without authority cannot mutate a
  complete receipt, while passive classifications match authenticated lookup;
  authority without authentication fails closed;
- another subject and an unknown target receive the identical public result
  and metric class;
- exact retirement replay is inert before and after restart;
- pending and indeterminate records cannot be retired or compacted;
- crashes before the marker leave exact replay, while crashes after the marker
  recover to retirement without repeating the original effect;
- workspace/epoch/generation transplants and changed targets conflict;
- retirement never reuses an operation reference or removes its tombstone;
- capacity becomes reusable only after durable retirement, never at marker
  preparation or an uncertain outcome;
- storage remains owner-only plaintext, debug/metrics remain content-free, and
  tests cover unsafe modes, symlinks, hard links, truncation and trailing bytes.

### STRIDE/LINDDUN closure

The new authority action prevents elevation through a generic delete; exact
authenticated binding mitigates spoofing and tampering; a durable marker and
idempotent result mitigate repudiation and crash ambiguity; coalesced
`AbsentOrForeign` prevents disclosure and linkability; aggregate saturation
supports availability without user-controlled labels. Plaintext remnants,
backups, hostile same-UID processes, media sanitization, rollback witnesses and
legal retention policy remain explicit nonclaims. The authenticated MCP tool
can initiate only the same exact native retirement request; it calls no lookup
or execute fallback, and its discovery grants neither authentication nor the
daemon's distinct `RetireReceipt` authority.
