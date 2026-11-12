# Bounded workspace change feed

Status: implemented local V1 contract, 2026-11-10.

The native service exposes `WorkspaceChanges { after, page_size }` as a
read-only `InspectWorkspace` command. It is a pull projection for coordinating
multiple clients, not an event bus and not a Hermes API.

## Closed contract

Each content-free event has a nonzero, process-local workspace revision and one
closed kind: document added, document removed, root added, root removed, or
document changed. A document-change event carries only its opaque document ID
and source revision; it carries no source text, locator, edit range, client ID,
request ID, witness, subject, or session.

V1 returns at most one event per page so the largest event and continuation fit
the canonical sixteen-field wire profile. `after` is the last successfully
observed feed revision, bound to a nonzero feed epoch from the durable workspace
identity generation. `after: None` begins immediately before the oldest event
still retained; when the feed is empty it begins at the current head. A page
reports the head sampled with that page and an exact next cursor. An empty page
does not promise that no later event exists.

If `after + 1` predates the oldest retained event, the accepted result is an
explicit `Gap { requested_after, oldest_available, observed_head }`. Consumers
must resynchronize through a bounded inventory/snapshot workflow; Vellum never
fabricates missed events. A cursor from another workspace and a cursor beyond
the current head are distinct semantic refusals.

The service reserves revision space before every publishing mutation. Only a
successful fresh mutation publishes. Exact request replay returns its retained
reply without reauthorizing, reapplying, or republishing. Failed and idempotent
no-op mutations publish nothing.

## Bounds and observations

Retention and requested page size are explicit service limits. Oldest events
are evicted deterministically when the fixed retention ring is full. Saturating
content-free counters report polls, pages, gaps, published events, and evicted
events. They do not expose event kinds, resource IDs, source revisions, clients,
or timing and never participate in semantic decisions.

## Restart and Hermes seam

The feed and its revision space are volatile. Service restart advances the feed
epoch (even when the durable workspace scope is retained) and resets the feed;
an old cursor is refused rather than reinterpreted as a reconnect or resume
token. Durable delivery, acknowledgements, consumer offsets, fan-out,
backpressure across processes, and restart recovery remain absent.

Hermes now has a concrete durable local ledger profile in the broader workspace,
but its no-compaction stream and persisted consumer-cursor laws do not directly
match this evicting page/gap relation. A future composition must add its own
authenticated subjects, Vellum event codec, retention/compaction law,
delivery/effect acknowledgement separation, privacy analysis, and restart
qualification. The exact mismatch and earned adoption gate are recorded in
[Hermes durability seam](hermes-durability-seam.md); no Hermes dependency is
selected here.

## Multi-client qualification

The service qualification explores every ordering of three independent
human/agent/observer view transitions, then exercises both possible first
writers against one shared document. Across those bounded serialized
interleavings it proves that selections remain client-local, the first exact
revision-witnessed replacement wins, the other client's replacement and save
refuse as stale, and changed-input request reuse refuses without execution.
Exact replacement, save, refusal, and poll replays retain their original reply
without duplicating persistence or feed publication. The observer receives the
single source-change event at revision one and then a deterministic empty page
at that head. The test also checks that usage and event debug evidence contains
neither source text nor locators.

The same request ordinal is valid independently in two client namespaces. A
separate read-only composition test proves that replaying a denied edit does
not reconsult or bypass authority, and that changing the denied request into a
read is a conflict rather than an escape. Recovered-identity qualification
proves that a pre-restart cursor is rejected after the feed epoch advances.

This is deterministic state-machine evidence for a serialized service owner.
It is not thread-parallel execution, remote transport isolation, a linearizable
distributed system, or durable Hermes delivery.

## Non-claims

This contract is polling, not push subscription. It does not claim total event
history, cross-process ordering, wall-clock ordering, exactly-once delivery,
durable audit, causal ordering across workspaces, or notification of private
per-client view/selection changes. Ordinary save does not publish because it
does not change the semantic source snapshot.
