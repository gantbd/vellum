# Coherent multi document read contexts

Section 12.8.9 adds selected actual-owner current capture, native parent/member
installation, bounded queries and synchronous search ownership. See
[native-version-set.md](native-version-set.md) for that separate composition and
its evidence limits. The abstract bank below does not itself establish those
native guarantees; historical/async/public and host/physical assurance remain gates.

Section 12.8.8 additionally checks class isolation in the shared abstract producer:
an annotation-only member retains no source bytes/revision/profile, and a
source-only member retains no annotation projection. A mixed-class set preserves
that separation after installation and counts only admitted retention units.
See read-class-isolation-mutations.mjs. The existing version-set bank still checks
256 admissions, 32 coherent captures, 270 transitions/116 states and nine mutations;
these checks do not establish native owner-root capture or physical accounting.

Specification §12.10 selects `read-version-set/1` on the existing F02
snapshot-handoff variant. It gives a finite explicit participant vector one
coherent published cut, all-or-none acquisition and one owning read-context
lifecycle. Each participant still has separate source/annotation grants. This is
not a transaction formed by joining previously issued single-document selectors.

Specification §22.1.4's source-query/1 range/line products pass through this same
all-required gate and the actual member registry. source-products.md adds composed
line/range release controls; no separate line token can bypass another required
member's current state. The original evidence counts below remain scoped to this
module, not a claim of complete native/physical/concurrent refinement.

## Precedents and selected design

[Roslyn immutable solutions](https://learn.microsoft.com/en-us/dotnet/csharp/roslyn-sdk/work-with-workspace)
provide an editor-oriented whole-model snapshot precedent: a captured solution
remains unchanged as the host advances. [PostgreSQL isolation documentation](https://www.postgresql.org/docs/current/transaction-iso.html)
distinguishes independently refreshed Read Committed statements from a stable
Repeatable Read snapshot. [SQLite snapshot isolation](https://sqlite.org/isolation.html)
likewise permits reading an older snapshot while later commits occur. Vellum
adopts one identified immutable published cut for the named local participants,
not SQL, a database dependency, long-held read locks or database serializability
for edits. None of these sources proves Vellum's permissions, memory accounting
or actual multi-document coordinator implementation.

The alternatives are independent current reads, one coherent all-required set,
or a best-effort set that silently omits denied/unavailable members. Independent
reads are useful but do not prove a common cut. Best-effort omission changes the
declared dependency domain and can expose hidden membership through omissions or
per-member errors. The selected coherent product requires every named participant;
a different subset needs a new explicit authorized acquisition. Its cost is
bounded shared retention and conservative whole-context invalidation when one
required member loses validity. Ordinary single-document contexts remain useful
and unchanged; no new execution mode or distributed transaction is added.

## Conditional acquisition and read argument

Assume the local authority exposes one immutable published-root read, and exact
retained references resolve to an available root under the requested scope. Select
the cut once, then resolve every declared document incarnation and source/annotation
projection from it. Sequential projection/preparation over that already immutable
root is safe; sequential recapture of independently current roots is not equivalent.
Document revision values need not be equal or advance together. Their comparison
domains remain local to each document incarnation. The internal common-cut witness
is not a raw public publication ordinal.

The native request declares one authority, unique explicit participants, selection
and profile. Each participant supplies a live document reference and nonempty
source/annotation data set. Acquisition validates all actual per-member grants,
incarnations, relevant dependency stamps, shared recipient/destination/purpose and
interpretation, plus whole-set retention/registry/reply capacity. Prepare outside
the final serialized handoff. Install parent and every member together, then expose
the complete permitted envelope, or install/expose none. A private staging loop is
not a sequence of public child acquisitions. Provisional local IDs have no validity
until whole-set commit; an allocator that issues observable reservations earlier
must preserve their non-reuse rather than roll them back.

Each member selector is a facet of its owning version-set context, not an
independently leased read context. Before every bounded member handoff, validate
all required member availability/access/continuity, then the requested member's
projection and exact query. The composed reference does both in one synchronous
functional action with no external callback/yield. Base member reads require a
trusted same-boundary parent-validation result; a public request cannot supply it.
Real adapters must route all member reads and closes through this ownership gate.
No cached boolean or retained permit survives a policy/lifecycle update.

The result names only the authorized requested participants and opaque parent/
member selectors. It returns no world inventory, private cut ordinal or failing
member index. Given equal authorized requested projections and the same schedule,
hidden-only objects/publications preserve the protected functional outputs.
Operational capacity interference remains allowed only under the declared profile;
timing and contention noninterference are not claimed. A later explicit subset
gets a new context, never the old parent relabeled smaller.

Reading a coherent set is neither permission nor proof that it is still current
for editing. The F11 proposal uses actual per-document revision dependencies and
all real companions at publication. A change to a read-only participant must
refuse a proposal derived from its old revision even when the write participant
is unchanged. Coherence also does not prove complete semantic discovery, absence
of new references, grammar correctness or cross-authority consistency. Those
claims need their separate scope/provider/evidence contracts.

## Retirement and boundedness argument

Parent close or explicit member-context stop closes the owning version set, not
one facet behind the parent's back. Dropping a borrowed member selector alone is
not a close. Relevant required-member grant/incarnation/dependency/continuity or
retention loss invalidates the set; no later read succeeds as a smaller set.
Regrant does not reactivate its identity. Already delivered bytes cannot be recalled.

Retirement removes all member/record/continuation delivery selectors. An internal
worker may still hold one member root: reclaim eligible unheld member rows while
retaining the actual held rows, charges and bounded parent ownership metadata.
The last required hold discharge permits final parent retirement. Other shared
allocation owners can remain; secure erasure and full heap reclamation are not
implied. Protected owner stop does not require a surviving content grant and
cannot be blocked by ordinary acquisition saturation.

Let n be the finite number of required members. Preparation and final validation
consume each member once, with bounded underlying registry/query loops. A simple
remaining-members rank decreases at each iteration; any missing precondition
refuses, and no action invokes arbitrary callbacks. Registry bounds charge parent
metadata/selectors, all member contexts and actual pending reply/hold obligations.
By induction, every active parent owns all active members; invalidation never
reactivates an identity, and held roots remain charged until discharged. This is
an authored conditional finite-action argument, not unconditional environment
progress, whole-engine categorical completeness or an implementation theorem.

## Executable evidence and sensitivity

version-set-contexts.mjs composes read-contexts.mjs, annotation-queries.mjs and
source-edit-contracts.mjs. It checks 256 final-acquisition gate cases; 32 capture/
later-change-or-no-op/order cases for source and annotation versions; nine combinations
of member source/annotation scopes; eight paired hidden-object/publication cases;
and two source-only edit compositions, including a changed read-only participant.
The actual closed actions explore 270 edges over 116 states through four steps.
Fifty-nine controls cover owner/selector isolation, parent-gate bypass refusal,
required-member validity and retention, aggregate capacity and late partial failure,
exact history/no fallback, explicit subsets, protected stop and held retirement.
An explicit witness shows two individually valid current reads forming a revision
vector that occurred in neither fixture publication.

Nine successfully imported in-memory mutants must fail an assertion: mixing
member snapshots, checking only one member, ignoring required retention, omitting
the parent selector charge, exposing a partial installation, falling back to latest,
closing only one facet, reclaiming a held member and ignoring a relevant dependency
stamp at query time. Import/runtime errors are not accepted as successful property
counterexamples. These are same-author checks, not independent Cleanroom oracles.
No new SMT/TLC theorem is claimed.

The reference uses two representative documents, source bytes and bounded plain
comment rows, private string selector labels, safe integer revisions and abstract
charges. One fixture publication updates both roots atomically when changed and
preserves versions on a complete fixture no-op; this demonstrates
the composed reference's cut selection, not actual host atomic-root acquisition.
Trusted normalized roots/projections, grant truth, accepted policy context and
historical lookup, preparation/reply reservation facts and real dependency stamps
remain premises. A stamp represents an actual relevant context dependency, not a
mandatory workspace-membership dependency for every explicit target. The fixture's
common policy token represents the accepted authority access context, not equality
of all documents' independently scoped policy/version domains.

The parent registry is an ownership overlay on the same engine's read registry,
not a second authoritative store or per-document engine. The private staged copy
and abstract root-cell accounting do not establish native heap charges, safe
concurrent memory handoff, token encoding/issuer restart, real queues, all target
shapes or whole-operation/refinement coverage. Single-document reference evidence
remains scoped to its own products. Required concrete adapters and provider/native
conformance remain gates; no HTTP, federation, automatic semantic closure or
durable execution dependency is selected.

## Instrumentation

INS-03 distinguishes set offers/preparation, all-required installation/refusal,
member delivery, whole-set invalidation and owner stop. INS-08 binds exact live
incarnations; INS-01/13 distinguish cancellation/stop intent from hold discharge;
INS-12 records observation loss independently. MELT/golden/RED/USE measure complete
useful handoffs, preparation/final-validation/service phases and parent/member/
selector/hold pressure in declared units. Requested participant count is not global
discovery coverage, and no failing hidden participant becomes a public error label.
No body, selector, document/principal identity or raw cut ordinal enters ordinary
metrics. STRIDE/LINDDUN cover mixed-cut substitution, cross-member tokens, stale
grants and topology leakage; STPA/FMEA cover partial installation, silent subset
fallback, parent-gate bypass, early refund and incomplete dependency validation.
