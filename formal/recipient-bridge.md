# Storage-to-recipient disclosure bridge

PS-RB invokes actual `StorageBridge` and paired-world `Disclosure` transitions.
It connects storage to an already-defined recipient profile: source and Visible
annotation records, never Hidden annotation bodies/existence or internal save
evidence. It selects no universal field-permission/API policy.

## Reference-state relation

Source has two symbolic representations: before/after a target deletion. Deletion
invokes actual storage Edit and disclosure DeleteTarget together. Source/revision
markers agree; annotation targets detach without erasing authored bodies. Numeric
representatives are not a real codec or a proof about arbitrary edits/anchors/undo.

Hidden bodies/existence vary independently between disclosure worlds; Visible
edits occur in lockstep. Storage outcomes/source are shared. Outcomes pass through
the storage adapter; annotation persistence is not silently connected to Source.

The wrapper uses Disclosure's own grant, revocation epoch, queued request,
disposal and regrant predicates. The composite's separate reference
source-plus-known-ack read surface is not invoked/exported here; its permission
is not this recipient's field scope. A complete multi-view policy inventory is
still required for the product specification.

## Actual release, not retained privilege

Query and queued Release invoke actual Disclosure operations. The wrapper attaches
current symbolic source only when the disclosure envelope permits Data.
Unavailable contains no source/private evidence. A pre-revocation queued request
remains ineligible after regrant through the existing epoch check; a fresh direct
query can be authorized again.

Released data contains source and the permitted annotation projection. Constant
zero `internalCount` is an **erasure-check marker**, not a proposed API/telemetry
field. A control fills it from internal success history and must fail even when
both worlds leak the same value. Paired equality alone does not establish
authorization for a shared private field.

Checks cover source-state agreement, equality of complete paired release traces,
evidence erasure, current release authority and inherited storage safety.
Both imported Next relations or full-state stutters must hold on each step;
output sequences only grow.

## Cases and scope

Twelve cases comprise three safety runs, three controls and six witnesses.
A five-transition exhaustive profile interleaves storage, hidden/visible changes,
deletion, request/release and policy changes. A five-step script acknowledges a
save before querying. A four-step script queues, revokes, regrants and denies
the old queued release.

Controls expose a rich hidden cache, include private evidence counts or bypass
current recipient authority. Witnesses show authorized read, preserved hidden
differences with equal outputs, projected deletion, revoked-read denial, erased
save evidence and regrant-gap denial. Failures must be property-specific.

The paired proof varies annotations, not every possible receipt history, metadata
field, timing or scheduling observation. Evidence erasure also rests on explicit
wrapper construction/allowlisting. This is not full noninterference over all
internal histories, native UI/accessibility, diagnostics/export or field policies.
Actual schemas, token encoding, stream lifecycle and all release surfaces still
need concrete contracts and implementation assurance.

## Instrumentation and threats

IF-02/IF-08 and INS-03 distinguish request, release, current denial, regrant-gap
denial and disposal. IF-04/IF-13, INS-04/INS-07 distinguish retained evidence from
recipient-visible state. Raw attempts, provider tokens, hidden counts and internal
outcome history are not public metric labels. Required retained evidence does
not become a public diagnostic or grant authority.

STRIDE: stale/spoofed privilege cannot authorize release; a rich cache cannot
contaminate a poorer recipient; identical leakage to multiple recipients is not
authorized merely because paired equality holds. Conformance, timing channels,
actual byte budgets and instrumentation overhead remain outside these checks.
