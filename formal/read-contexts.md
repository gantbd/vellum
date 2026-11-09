# Read context acquisition and selector lifecycle

Section 12.8.8 corrects the abstract preparation/installed retention boundary.
Annotation-only candidates contain no source bytes, source revision or source
profile; their declared units count only projected annotation rows. Source-only
candidates contain no annotation projection. Source-byte limits apply only when
source is admitted; separately owned root/input/preparation limits remain in force.
read-class-isolation-mutations.mjs checks all three nonempty class sets, a
mixed-class version set and three assertion-killed regressions against coherent
copied graphs. The existing registry bank still checks 128 admission cases,
64 pairs, 630 transitions/222 states and eight mutations. This correction is not
native coherent-root capture, physical isolation or complete noninterference.

Specification §§12.8–12.9 refine existing F02 `snapshot-handoff` and F19
`stop-observation`. The field catalog is read-contexts.json. A mediated context
selects one immutable coherent document projection for source, annotations or
both. It is neither a read lease nor a second mutation path. Annotation-only
access does not require a source-byte grant; source access does not grant comments.

Specification §22.1.4 now supplies tagged source-query/1 range/line products and
captured finite source profiles through this actual registry. See source-products.md
for field/line/delivery and parent-composition evidence; the counts below still
refer to this registry's own scoped lifecycle suite. It is not a native decoder or
physical ownership refinement.

## Precedents and boundaries

[Roslyn's immutable solution model](https://learn.microsoft.com/en-us/dotnet/csharp/roslyn-sdk/work-with-workspace)
supports working with an unchanged captured model while its host advances.
[SQLite snapshot reads](https://sqlite.org/isolation.html) illustrate readers
remaining on an older committed snapshot while later writes occur. Neither
establishes Vellum's field authorization, multi-object publication or retention
guarantees; no database dependency or write-isolation mode is imported.
[PostgreSQL CLOSE](https://www.postgresql.org/docs/current/sql-close.html)
provides the explicit cursor-retirement precedent: subsequent cursor operations
are disallowed after closure. Vellum does not import transaction-held cursors,
SQL, network sessions or public cursor catalogues.

[Rust Weak references](https://doc.rust-lang.org/std/sync/struct.Weak.html)
distinguish a selector from ownership keeping a value alive. Vellum likewise
separates selector validity from strong internal retention, without selecting
Arc/Weak as the implementation. Weak references can still retain backing
allocation metadata; neither weak handles nor logical invalidation prove zero
memory, secure erasure or accurate heap accounting.

The selected base context is explicitly scoped, bounded and evictable. It has no
mandatory wall-clock TTL, renewal operation or availability promise. Optional
timed profiles need the existing explicit clock/suspend contracts. This avoids
inventing a duration while preserving current checks at every release. Holding
an old projection is a resource cost, not continuing permission. A separate
trusted raw snapshot API discloses its readable extent at handoff and cannot
inherit mediated revocation claims.

## Acquisition and publication points

An explicit current capture selects one coherent published root at its owner
capture point. A retained selection resolves the exact named historical context,
not whatever is current when a worker finishes. Later ordinary publications need
not invalidate the immutable candidate. Current identity, grant, profile and
continuity still govern acquisition and delivery; a missing historical lookup
never falls back to latest. Actual coherent root/history lookup is a premise of
the design reference, not established by comparing fixture labels.

Preparation reserves its complete bounded work/retention obligations before
retaining or copying data. It makes an owned projection outside final handoff.
At the serialized local installation/handoff, recheck current recipient,
document incarnation, destination/purpose, accepted policy/continuity, profile,
capacity and stop state. Install the context and transfer its permitted selector
envelope coherently, or disclose no selector/payload and publish no partial
registry entry. Authored roots, history and source revisions are unchanged.
No I/O, arbitrary callbacks or synchronous provider consultation belongs here.

The registry stores typed context, record and continuation selectors. Record
selectors bind the context and exact record incarnation through trusted registry
resolution; continuation also binds query interpretation, limit and next visible
position. Issue only the selectors needed for a complete reply. Repeated page
reads reuse equivalent live selectors instead of manufacturing unbounded entries.
No private ordinal is released as a token or raw next position. The same shared
F03 query projection supplies page/record/location products and the F02 source
reference supplies exact bytes; neither can acquire the other's data scope.

## Lifecycle and accounting arguments

Prepared candidates are not yet public contexts. Installation makes a context
active. Close, relevant policy/continuity/lifecycle invalidation, or permitted
eviction prevents further delivery and retires all dependent selectors. A closed
or invalidated identity never becomes active again. New authorization creates a
new non-confusable context. Selector copies do not create independent leases,
quota or a right to keep a context open after owner closure.

Let H(c) be the set of actual internal retention obligations for context c.
Invalidation removes delivery eligibility immediately. If H(c) is nonempty,
retain its root and charges; final hold release permits reclamation of the
context-owned root. A duplicate release cannot discharge another obligation.
Shared allocation charges and other owners may remain after this local retirement.
Cancel intent, caller timeout or dropping a selector is not proof of H(c)=empty.
F19 owner stop uses protected control capacity and does not require the content
grant to remain valid. It cannot close a foreign recipient's context by possession.

Live registry and hold counts have finite profile ceilings. Issuance advances in
a non-confusable authority-session domain even when registry rows are removed;
exhaustion refuses before reuse. Restart requires a fresh non-confusable issuer,
not replay of old numeric counters. This permits bounded live registry storage
without an unbounded set of tombstones. This is an issuer obligation, not a claim
that native fixture strings are secure tokens or that identifiers survive crashes.

Inductively, active selectors resolve only active matching contexts. Retirement
removes that context's selectors and no action reactivates its identity. Holds
retain invalidated rows until discharged, so their abstract root charges cannot
be refunded early. Admission checks the complete next registry/charge bounds;
selector creation is all-or-nothing for one reply. Each normalized action has
finite bounded loops over roots, registry rows and selectors and invokes no
arbitrary code. This proves neither global service progress nor termination of
an arbitrary environment that keeps making new requests. No whole-engine
categorical completeness or bounded-lattice theorem follows from this FSM.

## Executable evidence

read-contexts.mjs composes the actual annotation-queries.mjs and
source-edit-contracts.mjs reference functions. It checks 128 acquisition gate
combinations, 64 paired hidden-state acquisition/page cases and 630 actual action
edges over 222 states through five transitions. Fifty-seven controls cover exact
source/annotation scope separation and combined context, emitted record/cursor
resolution, selector kinds/owners/issuers, replay allocation stability, historical
selection, alias/input ownership, close/regrant, held retirement, complete reply
capacity, issuance exhaustion and protected owner stop. All explored states check
bounded registries/charges, live selector ownership, hold ownership and issuance
monotonicity; invalidated identities never re-enter active state.

Eight in-memory mutants must import successfully and then fail an assertion:
omitting policy from the scope, ignoring current policy, refunding held roots,
reusing retired identity counters, accepting cross-context record selectors,
exposing raw next offsets, omitting complete selector reservation and ignoring
registry readiness. Scope keys are checked against the separate field catalog;
the direct policy-change scenario is not generated only from the implementation's
own key loop. These are same-author sensitivity tests, not independent oracles
or Cleanroom certification. No new solver theorem is asserted.

The reference uses one normalized trusted document/root, point attachment examples,
finite string labels and serialized functional actions. Its units are source-byte
cells plus visible record cells, not heap bytes or complete metadata/result costs.
All host grants, input validity, actual source coherence, captured field masks,
interpretation, historical reference resolution and retention facts are premises.
It does not implement Rust ownership, hostile native decoding, authenticated
host sessions, physical allocation reclamation, real token encodings, public
reply queues or concurrent memory handoff. Prepared reservations and reply-byte
capacity are supplied facts; actual resource proofs remain required. The private
hold helper is engine bookkeeping, not a recipient-controlled pin API. Multi-
document version sets and raw trusted snapshots retain their separate contracts;
this reference does not prove them.

Specification §12.10 and version-set-contexts.md subsequently select the native
multi-document product and compose these functions under all-required parent
validation. A base query refuses a parent-owned member without the trusted same-
boundary parent result; base close cannot detach it. The parent result is an
internal enforcement premise, not a public request field or reusable grant.
The older single-document counts above do not independently prove that new
composition or actual adapter routing.

## Instrumentation and residual work

INS-03 observes context preparation/installation, delivery, invalidation, stop,
lookup unavailability and authorized resynchronization. INS-08 observes exact
owner incarnation/disposal interactions; INS-01/13 distinguish stop acceptance
from actual hold discharge; INS-12 retains observation loss separately. MELT,
golden signals, RED and USE record useful handoffs, wait/service time, offered/
admitted/refused work and retained registry/hold pressure in declared units.
No raw selector, root generation, principal/document label, body or hidden-record
count becomes an ordinary diagnostic label. Internal exact causes remain truthful
under independent diagnostic authorization while public unavailable products stay
coarse. STRIDE/LINDDUN address forgery, stale scope, cross-context correlation and
counter leakage; STPA/FMEA address false empty completion, premature refund,
unbounded retries and confusing cancellation with reclamation.

The acquisition/retirement products and base lifecycle direction are selected;
they no longer await a user choice. Actual package/native encoding and constructors,
complete target/version-set adapters, issuer restart procedure, coherent host/
historical lookup, physical charge ownership, queue/lock mechanics and measured
capacities remain named implementation/integration gates. No numeric lifetime,
new network service, policy engine, pinning API or durability guarantee is invented.
