# Proof-slice contracts, arguments and remaining obligations

This is design assurance, not an editor implementation or a full Cleanroom
certification. The agreed requirements remain normative. The finite models make
selected contracts precise without choosing storage structures, APIs, schedulers,
numeric production budgets, codecs or policy services. All model constants in
[slices.mjs](slices.mjs) are exploration parameters, not product decisions.

The authoritative case manifest is `slices.mjs`; [obligations.json](obligations.json)
maps each slice to requirement sections, partial invariant coverage, interfaces,
instrumentation and assumptions. The generated RDF uses `partiallyAddresses`,
never a whole-invariant `verified` flag. [results.json](results.json) records the
exact checked inputs, configurations, outputs and witnesses. Results apply only
when hashes match and `completeToolRun` and `passed` are true.

## Structural target and candidate evidence

The native annotation products (§§13.6–13.7) retain F13's four variants; see
[annotation-operations.md](annotation-operations.md). The normalized one-record/
point-target reference checks 1,024 first-admission gate cases and 86 controls,
including complete success/refusal frames and mandatory dependency union. Full
targets, companions/history, actual registry/adapter/load planning, replay and
implementation refinement remain separately scoped obligations.

The reference annotation snapshot (§29.4.1) now has an exact structural schema,
decimal encoding and separate authored/current-placement products; see
[annotation-snapshot.md](annotation-snapshot.md). The design reference checks
540 valid cases, 256 staged-load decisions/57 controls; the pinned JSON Schema
tool adds metaschema validation and 15 structural cases. Host identity/source
truth, real decoder/ownership, historical placement and final atomic-load/persistence
refinement remain obligations, not supplied by a schema or staged candidate.

The adopted §29 reference profile is recorded in reference-profile.json and
[reference-profile.md](reference-profile.md). Its byte-scanning line reference
checks 3,906 generated texts/33,594 byte positions against a separate string oracle;
256 normalized staging cases and 17 controls cover source association, owned
preparation and selected boundaries. Unicode segmentation and hostile JSON
decoding, actual provider/native/retention/measurement evidence remain outside
this reference. Existing scoped TLC/SMT assumptions are not discharged by adopting
defaults or validating their inventory.

Specification §§26.5–26.6 and [preparation-boundary.md](preparation-boundary.md)
add engine-owned candidate/descriptor binding and normalized required-report
intake: 256 private admission cases/30 controls. A separate bounded preloaded
definition-closure reference checks 1,536 graphs/roots and six controls. These
conditional arguments do not prove actual ownership/allocation/authentication,
public decoding, physical retention, provider truth or validator termination.

Specification §13.5 and [annotation-records.md](annotation-records.md) add authored
content/target frame contracts: 48 private cases/18 controls. Equal-field tests
check unchanged versions/state and reject version-changing mutants. Full annotation
history/mapping, metadata, persistence and recipient projection remain separate.

Specification §26.4 and [validation-reports.md](validation-reports.md) add semantic
source/candidate tags, named assertion/definition/options, independent verdict/
coverage and four-way satisfaction. The normalized fixture checks 64 report cases,
260 order/duplicate cases and 24 controls. No public decoder, actual schema/provider
truth, trust/closure/candidate construction, retention or new solver theorem is
claimed. Existing exact-publication and structural references remain separately scoped.

Specification §26.3 and [structural-targets.md](structural-targets.md) add supplied
structural-target/candidate claim obligations: 1,024 private claim-matching cases,
three literal prose/code/fence source fixtures and 26 controls. The conditional
argument assumes provider truth and sound mappings, retains final exact-edit gates
and introduces no new TLC/SMT theorem or required structural feature.

## Source read and exact edit bridge

Specification §22.1.2 and [source-edit-contracts.md](source-edit-contracts.md)
define mandatory semantic fields/results for F02 bounded-range and F11 exact-batch.
The reference checks 512 source reads, 64 source-only admission/publication decisions
and 29 controls with actual UTF-8 bytes. A conditional composition argument reuses
the existing release, normalization, publication and no-op obligations. No new
TLC/SMT theorem, full family/companion schema, actual decoder, concurrency/retention
or implementation refinement is claimed.

## Semantic operation-family catalog

Specification §22.1.1 and [operation-catalog.md](operation-catalog.md) adopt D33's
semantic families/bounded variants with one typed catalog. The 28-family inventory
has 85 design labels and selected links to all 12 facade bullets; 43,520 reference
routing contexts/28 controls and ten local SMT conditions support category and
decoder congruence guards. No new TLC model, public command ID/schema, actual
Rust handler or full execution/refinement proof is claimed.

## Complete baseline decision traversal

Specification §28 and [baseline-contracts.md](baseline-contracts.md) complete the
remaining approved baseline traversal. All D01–D48 appear in the exact checked
contract/claim/scenario/observation/gate ledger. Fifteen private mechanism families
cover 5,861 cases with 18 controls; 16 local SMT conditions comprise 12 positive
laws/four controls. Eleven ledger controls check malformed/missing references and
false-closure status, not semantic adequacy. Concrete API/settings/format choices,
assets/Unicode, actual native/providers, measurements and operational processes
remain gates. No new TLC model or whole-engine theorem is claimed.

## Adopted scoped reference and current release

D09–D12/D42 are specified in §12.6. See
[scoped-release.md](scoped-release.md) for current-gate, capture-ceiling,
owned-handoff and visible-domain arguments. Finite checks cover 34 masks,
2,312 paired projections, 19,652 handoffs and 1,156 frames with 18 controls;
a closed six-action stream model explores 28 states/168 transitions without a
transition cutoff, not a termination/fairness claim. Nine local SMT conditions
comprise five positive laws/four controls. Actual authentication/policy truth,
ownership/queueing, historical lookup and all-surface refinement remain open.

## Adopted identity and residency

D07/D08 are specified in §§11.1.1/11.3.1. See
[identity-residency.md](identity-residency.md) for conditional registry uniqueness,
fresh-incarnation, disposal and retained-obligation arguments. The finite fixture
checks 96 supplied-evidence decisions, 1,920 serialized registry scenarios and
1,088 lifecycle states with 14 controls. Seven local SMT conditions comprise four
positive laws and three SAT controls. No new TLC exploration, actual filesystem
identity oracle, complete obligation inventory or implementation proof is claimed.

## Adopted finite capacity profiles

D06's baseline is specification §17.2.2. See
[capacity-admission.md](capacity-admission.md) for conditional pointwise admission,
protected partitions, activation framing and soft-target arguments. The private
declared-charge reference covers 6,561 profiles, 157,464 admission cases, 118,098
activation cases, 59,049 optional plans and 18 controls. Ten local SMT conditions
comprise six positive laws and four SAT controls; no new TLC model is claimed.
Units, floors and charges are fixture choices, not public defaults. Actual
accounting, clocks, service, fairness, queues and implementation refinement remain
separate obligations; these checks do not prove physical resource ceilings.

## Adopted layered outcome projection

D05's baseline is specification §23.1.1. See
[outcome-projection.md](outcome-projection.md) for conditional projection/frame
arguments, 9,216 protected-world / 3,456 reason-order / 3,456 release cases,
19 controls and nine local SMT conditions. No TLC slice is added. Private
vocabulary/precedence and supplied policy/retry bases do not establish actual
products/codes, all fields/sinks, provider/retention or implementation refinement.

## Adopted mapping profiles

D04's purpose-specific mapping baseline is specification §4.3.1. See
[mapping-profiles.md](mapping-profiles.md) for conditional arguments, 2,338 finite
batches / 18,056 point / 88,504 target cases, 15 direct controls and nine local
SMT conditions. No TLC slice is added. Point/replacement rules in this reference
are representative, not chosen public schemas, all context-loss/history/geometry
profiles, D45 interaction defaults or full coordinator/implementation refinement.

## Adopted bounded replay

D03's contract is specification §6.5. [replay-reference.md](replay-reference.md)
gives a conditional trace induction, one entire finite reference graph (48,664
states / 2,092,552 transitions), seven witnesses, nine sensitivity controls and
nine local SMT conditions. No TLC slice is added. The reference uses opaque
publication and modeled effect discharge; it is not concrete identity/intent,
retention/clock, provider or coordinator refinement, nor crash-durable execution.

## Adopted batch normalization

D02 accepts unambiguous unsorted original-snapshot batches through one shared
normalization path, preserving complete records and logical boundaries. See
[batch-normalization.md](batch-normalization.md) for the authored rank/uniqueness
argument, 8,008 finite permutation evaluations, 15 invalid-domain and eight
mutation controls, one context-separation fixture and seven local SMT conditions.
No new TLC slice is introduced. This is not production canonicalizer refinement,
an actual resource envelope or proof of atomic multi-document publication.

## Adopted semantic no-op direction

D01 now selects validated unchanged success under requirements §5.7 and
specification §6.4. PS-NO adds 43 bounded configurations and eight local SMT
conditions with the general frame argument in [noop-publication.md](noop-publication.md).
It preserves current edit bookkeeping after normal final validation, distinguishes
meaningful equal-text companions/effect intent and rejects fabricated publication.
The reference mapping/history fixtures are not new defaults, and actual bounded
sound/complete operation classifiers, complete participants, result/status schemas
and cross-slice/implementation refinement remain open. PS-CH remains restricted
to text-changing work; this adoption does not retroactively strengthen its proof.

## Evidence and completion definitions

- **Bounded checked:** every reachable state/transition under the recorded
  configuration/horizon was checked by TLC, subject to its recorded fingerprint
  risk. This is not unbounded deductive proof.
- **Witness checked:** checking `Not<target>` deliberately produces a trace to
  the target while the ordinary safety checks remain enabled. This is an
  existential reachability result, not universal progress. A parser/type error
  or violation of a different property is a failure, not a witness.
- **Negative control checked:** an intentionally weakened model produces the
  named safety/liveness counterexample. This tests sensitivity, not completeness
  of the property suite or a production bug.
- **Construction argument:** the induction/rank reasoning below is explicitly
  authored reasoning, not independently reviewed or proof-assistant checked.
- **Parametric VC checked:** `parametric.smt2` verifies symbolic rank/algebra/
  projection/composition conditions beyond finite TLC bounds, with satisfiable
  premises and unsatisfiable negated conclusions. Z3 and the human encoding are
  trusted; no independent certificate replay or automatic TLA+ translation is
  claimed. This does not by itself check the complete inductive/temporal proof.
- **Slice pass complete:** scoped contract, model, case manifest, safety checks,
  useful witnesses, negative controls, assumptions and residual obligations are
  recorded and the full suite passes. It does not mean all associated product
  requirements, cross-slice composition or implementation refinement are proved.

Most slices are finite-prefix transition systems. At the harness horizon their
only behavior is stuttering; deadlock checking is disabled deliberately. They do
not acquire a useful-progress theorem merely because the exploration stops.
The separate `Execution.tla` fuel/rank model remains the control-termination
argument. The bridge below explicitly reuses its `Step` function and checks
control correspondence; its temporal fairness guarantee is not inherited.
PS-UP is the explicit exception: its Bound is a finite initial backlog cardinality,
not a transition horizon; weak-fair service supports temporal eventual drain in
that separate finite-state model. It does not give the other slices inherited
fairness or prove an actual scheduler. Each slice enumerates enabled semantic
transitions, not a complete public input byte/schema parser.

## Symbolic-kernel reconciliation boundary

Current addition: PS-EC below supplies new finite actual-sequence change/mapping
and boundary evidence for the adopted editor ideas. Earlier statements about
unchanged models refer to the reconciliation checkpoint, not this addition.

Requirements §§3.4.1 and 5.2.1 separate presentation-independent authoritative
state, contextual semantic evidence and authorized projections. Existing models
below already abstract away pixels and file mechanisms; their checked transitions,
constants and properties are unchanged by this documentation reconciliation.

- PS-TX/PS-BR check revalidation of declared object/context tokens and coherent
  publication, not discovery completeness, real query/predicate invalidation,
  stable symbol identity or semantic-to-source edit correctness.
- PS-HI checks abstract frontier/content relationships, not an actual inverse
  representation, graph-aware selective undo or restoration of prior authority.
- PS-DI/PS-RB check their specified annotation/source projections and erasure,
  not arbitrary graph edges, hidden topology, query counts or scoped token design.
- PS-PE/PS-CS/PS-SA/PS-DG/PS-SC/PS-SE/PS-SB retain effect/admission/storage and
  execution assumptions. They do not acquire a Git implementation bridge,
  physical serialization, durability, cryptographic provenance or adapter proof.
- FM evidence-lattice laws apply to a fixed declared finite observation scope,
  not a complete semantic graph or globally closed world of meaning. Rank/fuel
  applies to admitted bounded processing; a closed query name does not bound an
  unrestricted traversal, inference engine or native provider.

Before a stronger capability is claimed, specify and refine: (a) object/symbol
identity and source mapping, (b) object-read plus discovery-scope dependencies,
(c) complete-within-scope versus partial/unsupported claims, (d) current-authority
projection of evidence/edges/tokens, and (e) bounded lifecycle/retention. Then add
positive profiles, property-specific weakened controls and witnesses for those
actual contracts. Intended scenarios include a new reference outside the old
result set, stale region/provider generations, hidden-only edge changes, and
partial discovery offered to an operation requiring completeness. None is newly
proved by re-running the present suite. Kernel refinement and edge conformance
remain separate obligations; approved v1 does not require a semantic indexer.

The obligation links below retain their original partial coverage. New interface
traceability to these requirements is not a new checked semantic proof slice.

## PS-TX — competing local transactions

Sources: requirements §§3.4, 5.1–5.3, 12.7; `Transactions.tla`.

Profile: two one-shot operations, X writing A+B and reading A+B+R, Y writing B
and reading B. R is a read-only dependency of X and may be changed by a separate
local operation. Independent context tokens represent membership/discovery,
configuration, selections, binding and incarnation evidence. An exact-revision,
conservative declared-dependency profile is already required/allowed; this model
does not choose an optional remapping or semantic-discovery policy.

| Transition | Preconditions | Observable postcondition / failure |
|---|---|---|
| Prepare | Operation is new | Capture source revisions/context; authoritative root unchanged |
| Build participant | Preparing; participant not yet staged | Private staged set grows; no published root changes |
| Commit | All write participants staged | Recheck all declared read revisions/context/current grant in the serialized boundary; publish complete next root or reject unchanged |
| Observe | Any bounded point | Obtain one whole published root, never a partially staged candidate |
| Stop/revoke/invalidate | Applicable active operation or token | Stopped operation does not publish; stale/denied final context cannot commit |
| Capture Off/OK/Full/Failed | Optional bounded diagnostic step | Authoritative projection unchanged; no diagnostic status is an admission grant |

`SerialRoot` compares the current root against replay of the actual publication
log, including the separate R edit. `PublishValidated` checks the final gate;
`RejectedUnchanged` checks rejection. Each participant's revision advances once
per changing publication. `NoPartialObservation` checks coherent observations.
The leak control deliberately returns A's staged value in an old published root
and must fail that invariant. It does not simulate unsafe pointer publication.

Witnesses include X success, both operations succeeding in valid serial order,
Y winning against an already prepared X, changed R, changed non-text context,
revocation, stop before publication, partial staging still private, and successful
publication with diagnostic capture full/failed. Both-commit reachability does
not assert that two stale conflicting preparations should both succeed.

Construction arguments:

1. Initially root = Replay(empty). Private preparation/build/control/capture
   steps preserve root/log. Commit and R-edit append exactly one operation and
   replace root by Apply(old root, operation). Induction gives `SerialRoot`.
2. Published roots are added only with whole-root publication. Observe returns
   a current root already in that set. Hence observations are published roots.
3. Final guard evaluation and root replacement are one modeled serialized
   transition. No mutation can interleave inside it. This discharges the abstract
   TOCTOU obligation, not the future locking/ownership implementation.
4. Erase diagnostic state and harness tick. Capture is an authoritative stutter;
   other transition decisions do not consult diagnostic state. Thus capture
   modes do not change authoritative results for the same admitted semantic
   event trace. No identical timing/scheduling, allocation or native-fault
   isolation claim follows. The harness horizon is not an operational quota.

Residuals: real UTF-8 change-set/selections/history mappings; alias/incarnation
registry; concrete root publication and reclamation; all public observer paths;
actual context-generation contracts; dependency-discovery completeness; semantic
rename correctness; bounded callback-free implementation and overhead. This is
not proof that an agent declared every dependency.

## PS-HI — frontier-only linked history

Sources: requirements §§3.8, 5.8; `History.tla`.

Profile: source scopes A/B, independent source N and a comment-content scope.
G changes A+B, H changes B, I changes N, C changes the comment, F changes A.
The compound profile also includes the comment as a participant of G. Content
is a symbolic sequence of authored groups, deliberately not actual text.

| Transition | Preconditions | Observable postcondition / failure |
|---|---|---|
| New authored group | New group; current rights | Append all participants; invalidate affected linked redo and dependent continuation |
| Undo | Group retained/current; current rights; top undo frontier in every participant | Pop all participants and push matching redo; fresh changed versions; otherwise blocked unchanged |
| Redo | Same matching redo frontier everywhere | Move the complete group back; fresh versions; otherwise blocked unchanged |
| Inverse unavailable | Retained group loses inverse eligibility | No content destruction or partial history movement; later reversal blocks |
| Rights removed | Scope loses mutation right | No restoration of historical authority; reversal rechecks current rights |

Checks: no split group membership in either history direction, content/history
agreement, no invalid redo handles, disjoint undo/redo positions, unchanged
blocked outcomes, fresh versions and current frontier/rights eligibility.
The partial-undo control intentionally reverses only A of G and must fail.

Redo dependency is specified in this finite linear-history abstraction, not
guessed by wall-clock proximity. Stack tops replay first. Invalidating an entry
invalidates the lower entries whose replay follows/depends on it, then propagates
through every linked participant to a least fixed point. Filtering removes the
complete resulting set from every redo scope. This is a mathematical reference
construction, not a selected production traversal/storage algorithm.

Witnesses: H blocks G; unrelated I does not; unrelated C survives G undo;
compound C blocks G; undo/redo round trip has fresh versions; F invalidates G's
redo everywhere; invalidating G also invalidates dependent H; unavailable inverse
and changed rights block reversal.

Construction arguments:

1. Apply/undo/redo change group position in every declared participant or none.
   New/blocked/control steps cannot move only part of a linked group. Induction
   preserves complete group membership; the negative control breaks this premise.
2. Redo invalidation grows a subset of finite Groups monotonically. Each strict
   iteration reduces |Groups minus invalidated|. Closure terminates, is the least
   set closed under the declared dependency edges, and filtering every scope
   preserves linked invalidation. No cyclic graph can refill that rank.
3. Versions count accepted changes, not historical positions. Reversing back to
   earlier symbolic content still increments current versions; it cannot revive
   an old revision identity.

Residuals: actual inverse/expected-content laws, fresh operation IDs, authored
group coalescing, selection/anchor mapping, deletion/tombstone identity, concrete
retention floors and multi-operation atomic admission. The unavailable transition
models refusal, not physical eviction/reclamation. Source history, annotation
body history and passive mapping remain distinct; complete interaction with the
disclosure/attachment models is not yet a composed theorem. External effect
replay is outside this model, not proved absent in a future implementation.

## PS-DI — annotation attachment and recipient disclosure

Sources: requirements §§3.8, 15.5–15.6; `Disclosure.tla`.

Profile: two worlds with equal authorized visible annotation/source state and
initially different hidden bodies. Hidden body/version/existence transitions can
occur independently in either world. Visible updates and public requests follow
the same event sequence in both worlds. The recipient cannot see the hidden
record in this profile. Policy controls current release eligibility, not ownership
of retained bodies. Targets abstract located versus detached, not coordinates.

| Transition | Preconditions | Observable postcondition / failure |
|---|---|---|
| Visible query | Current grant and live incarnation | Visible records/count/source revision only; otherwise common unavailable envelope |
| Hidden mutation | Bounded private change | No source revision change or protected output; internal hidden versions may differ |
| Delete source target | Target still located | Detach targets; retain authored bodies, presence and body versions |
| Queue/request | Current grant/live target | Capture internal policy epoch; no perpetual grant |
| Release | Pending batch | Recheck current grant/lifecycle/epoch; compute current projection or reject safely |
| Revoke/dispose | Current lifecycle transition | No later unauthorized release; already disclosed output is not recalled |
| Regrant | Revoked epoch | Old queued stream still has a gap; new requests may use current state |
| Hidden probe/diagnostic | Supported safe envelope | Common unavailable/constant health projection; no hidden identity/count/body |

Checks compare the **whole protected output traces**, not just annotation bodies.
They also check equal public state, source independence of hidden edits, retained
body/version on target deletion and authorization of every queued data release.
The privileged-cache control reuses a richer owner projection for the recipient;
the paired-output invariant must fail immediately.

Witnesses: permitted release, revoke-before-delivery rejection, regrant gap
rejection, retained detached body, and equal protected outputs despite different
hidden contents. Queue delivery computes a fresh projection, one allowed safe
choice; this is not a newly mandated snapshot/subscription result policy.

Construction argument: establish equality of public projections initially.
Independent hidden transitions preserve it. Coupled visible transitions preserve
it. Public outputs are a deterministic function of that projection and the
shared current grant/lifecycle/epoch, or the same safe envelope. By induction,
protected output traces stay equal. The argument is for this restricted policy
profile and observation alphabet, not arbitrary real APIs.

Residuals: coordinates/affinity and real edit mapping, authorized reattachment,
all metadata/provenance/quote/search/pagination fields, scoped token encodings,
cache keys, variable policy membership and permission classes, native UI and
accessibility agreement, operational interference disclosures, trace/metric
schemas, byte-level serialization and timing. Constant health output is a safe
minimal observation, not verification of a rich observability product.

## PS-PE — persistence and effect evidence

Sources: requirements §§3.6–3.8, 5.4, 5.9; `Persistence.tla`.

Profile: one unique source attempt and one unique annotation attempt, one initial
incarnation, two possible binding/profile/token generations, source edits and
annotation edits. Snapshots carry artifact, attempt, incarnation, revision,
binding, profile, exact symbolic payload and freshness token. Source payload
abstracts text plus serialization profile; annotation payload abstracts body
plus source-mapping generation. These are not real serialized bytes.

This slice requests **conditional overwrite**, so lack of that host capability
rejects admission. It does not forbid the separately supported best-effort or
explicit force-overwrite profiles; those are not modeled here.

| Transition | Preconditions | Observable postcondition / failure |
|---|---|---|
| Capture | New attempt | Immutable versioned payload/context; later edits cannot change it |
| Encoding failure | Captured, not dispatched | Reject without dispatch or replacement of authored state |
| Dispatch | Required capability; captured binding/token applicable | Create explicit outstanding effect; otherwise reject before dispatch |
| Host write | Dispatched, not completed | Conditional comparison and write are one host-guaranteed transition; competing token change causes failure |
| Receive | Matching accepted host result | Retain exact attempt facts; promote baseline only for current binding and nonconflicting success |
| Mismatched receipt | Altered tuple field | Reject; do not create effect facts or change baseline |
| Stop/timeout | Active captured/dispatched attempt | Stop control; retain outstanding dispatch and unknown write evidence |
| Rebind | All dispatched effects have accepted safe completion/quiescence evidence | Activate new binding and clear old current baseline; late old evidence remains historical |
| Contradictory report | Prior accepted success | Retain both facts, invalidate that success baseline; no fabricated reconciliation |

Checks cover immutable capture, baseline attribution, no undispatched evidence,
required conditional capability, token comparison at actual host write,
monotone evidence, stop/bad-receipt noninterference, safe rebind and separate
source/annotation baselines. Eight mismatch variants change attempt,
incarnation, revision, binding, profile, artifact, payload or token. Exact tuple
comparison rejects every variant; attempt-only comparison fails its negative
control. Cryptographic/channel authenticity is still a host-boundary assumption.

Other controls bypass binding attribution or capability admission and must fail.
Witnesses include successful source and annotation saves, both artifacts saved,
old-revision receipt, fresh revision with equal source output, stopped unknown
write, late success after stopped control, historical success after safe rebind,
source-only save with unsaved comment, conditional-write race rejection,
contradictory evidence, malformed receipt rejection and unsupported capability.
The two-artifact success witness uses an explicitly scripted subset of `Next`
for eight steps; it is **not** eight-step exhaustive safety exploration.

Construction arguments:

1. Capture is single-assignment; every dispatched payload is its immutable
   captured snapshot. Editing live state changes neither captured revision nor
   bytes represented by its symbolic payload.
2. Evidence changes only by union of accepted observation facts. Stop cannot
   remove a dispatch/fact or turn no receipt into known failure. Late receipts
   do not restart stopped control.
3. Baseline promotion requires a matching captured binding and success without
   contradictory failure. Rebind clears current baselines; contradictory facts
   invalidate that attempt's baseline. Induction preserves attribution.
4. The host conditional-write action compares the captured token at the actual
   write boundary. Moving this comparison to earlier local admission would not
   establish isolation. The library cannot synthesize this host capability.

Residuals: multiple/reused attempts and dedup retention, concurrent same-artifact
saves and baseline ordering, actual resource equivalence/locator races, receipt
channel authentication, codec/BOM/newline round trips, complete annotation
serialization, replacement/metadata preservation, durability levels, process
crash, recovery, partial destructive writes and a real provider's conditional
write/quiescence guarantees. No durability or joint multi-artifact atomicity is
inferred from the successful two-artifact trace.

## PS-BR — shared-state bridge and control-step correspondence

Sources: requirements §§3.4, 3.6, 3.8, 5.3, 5.8–5.9, 14.5, 15.5–15.6;
`Bridges.tla`. This is a reference composition of selected contracts, not a
refinement theorem importing every behavior of the four base slice modules.

Profile: five one-shot operations (source Edit, Comment, source Undo, source Save,
source Read), source documents A/B and one private comment. The initial fuel is
6 **per operation**, not one repeatedly refilled budget; the operation set is
fixed at initialization. The exhaustive run explores six global steps. Three
additional positive scripted runs explore the specified 8-, 9- and 11-step
scenarios, not all interleavings at those horizons. The case manifest identifies
each run and every witness's actual specification.

The shared authoritative root contains source content/revisions, source undo/
redo frontiers, authored comment body/version and attachment/mapping evidence.
This is a mathematical record for stating coherence, not a chosen Rust struct,
copying algorithm or memory-management implementation. Source edit abstracts
deletion of the target; undo restores source but conservatively keeps an existing
detached annotation detached. It does not introduce automatic reattachment.

| Bridge | Relation/check in the common state | What remains outside it |
|---|---|---|
| Publication → history | Each document's source value agrees with its G undo frontier; text, fresh revisions and linked frontiers publish together | Arbitrary groups, full stacks, real inverse data, redo execution and participant disposal |
| Source/history → annotation | Mapping revision follows current source; source edit/undo preserves current authored body/version; comment publication preserves source history | Actual range/affinity mapping, reattachment and compound source/comment group reversal |
| Published root → effect | Save captures `Source(root)` from a published root; immutable capture becomes the exact dispatched payload even after a later edit | Codec bytes, provider write behavior, multiple attempts, rebinding and durability |
| State/evidence → recipient | Source-only public projection excludes private annotation/global generation; current release guard applies; revoked reads do not erase late write facts | All disclosure classes, richer metadata, subscription lifecycle and receipt-channel authenticity |
| Rich work → control kernel | Each operation uses imported `Execution!Step`; recorded event lists reconstruct the exact next control state; nonterminal work reduces rank | Full public vocabulary/parser, dynamic descendants/restarts, cost bounds and fair executor service |

### Actual reuse versus claimed refinement

`K` is a named TLA+ instance of `Execution`, not a copied transition function.
`KernelRefinement` checks each operation against `Run(old control, events)` and
frames the other operations. Normal work expands to one kernel event; a rejected
stale commit expands to DependencyChanged then Commit, charging both. Build uses
Tick rather than performing unaccounted preparation work. `KernelTypes`,
`WorkCharged` and terminal/evidence stability are checked together.

This is finite safety correspondence of **control steps** with the kernel.
Kernel content vectors are per-operation abstract commit markers, not a claim
that commenting changed the source documents. Initial kernel records are
explicitly constructed for their operation family. The bridge does not prove
full observable trace refinement of every base slice, nor does it import the
kernel's weak-fairness assumption into a scheduler that might starve an operation.

The same root supplies history, attachment, save capture and source reads; there
is no independent successful "history commit" or "save snapshot" that can refer
to an unrelated staged generation. Kernel terminal control and external outcome
evidence remain separate: a Save can stay Stopped while a late OK fact is retained,
and a revoked Read receives a safe unavailable result rather than those retained
private owner facts.

### Evidence, controls and arguments

Seventeen TLC cases add four positive safety runs, eight reachability witnesses
and five weakened controls. The six-step exhaustive run visits 360,330 distinct
states. The complete model bounds/configurations and TLC risk are recorded in
`results.json`; this count is not coverage of arbitrary operation counts/lengths.

Witnesses cover coherent linked edit, source undo preserving a later comment,
save captured before a later edit and dispatched unchanged, late save success
retained after cancellation while a revoked queued Read is denied, fuel exhaustion
without publication, surprise fault, source capture after publication and an
authorized read release. Negative controls separately:

- Publish source without B's matching history frontier.
- Restore a historical comment body as a side effect of source undo.
- Capture A's staged text paired with its old published revision.
- Expose the private owner generation through the public cache token.
- Stage work without taking the recorded kernel Tick step.

Each must fail its named invariant/action property. The old-body control uses
the explicit 11-step Edit → later Comment → Undo script. It does not claim
exhaustive 11-step concurrency coverage.

Construction argument: source publication updates the source/history/mapping
fields of the same root and frames current authored body/version; comment
publication frames source/history. A save snapshots that published root exactly
once. Public projection is a function only of permitted source fields. Each
control-changing operation computes the imported kernel step(s), and unrelated
controls frame. These local obligations yield the checked common-state contracts
for this profile; general implementation and multi-profile composition still
require additional refinement relations.

Three additional **parametric** SMT conditions check source/history/body frame
relations, comment/source frame relations and private-state projection erasure
using symbolic typed roots and unbounded integer fields. Premises must be
satisfiable and negated conclusions unsatisfiable. The SMT record encoding is
authored, not automatically translated from TLA+; it does not prove the future
implementation or memory/publication mechanism.

## Assumption discharge and compositional gates

The additional [concurrent-save slice](concurrent-saves.md), PS-CS, compares
candidate coordination policies without approving one for v1. It extends receipt
and token-order reasoning beyond the one-attempt-per-artifact persistence model.

The [save-admission comparison](save-admission.md), PS-SA, checks candidate busy
and bounded per-resource FIFO policies under proposed single-flight coordination.
Its immutable queued snapshots, explicit cancellation/deadline paths, symbolic
aggregate cost/completion reservations and late safe disposition do not establish
fairness, real retained-byte/clock bounds, adopt a policy or compose provider models.

The [dispatch/evidence gate slice](dispatch-gates.md), PS-DG, adds final local
launch checks and exact historical evidence attribution, separating established
provenance from a provider-supported safe disposition. It leaves supplier truth,
physical-effect revocation and full composition as explicit proof gates.

[Save composition](save-composition.md), PS-SC, now invokes the actual PS-SA/PS-DG
transition relations in a restricted shared profile and checks Next-or-stutter
correspondence. Refund, acknowledgment grading, baseline ordering and current
reference read projection are checked together; complete lifecycle, provider,
Persistence/Disclosure relations and liveness remain open.

The composition now also checks queued cancellation/expiry, active expiry,
admission close and a wait cut that reports unknown effects without refunding them.
[Save execution](save-execution.md), PS-SE, reuses actual Execution Step lists and
composite actions for rank, terminal cleanup and late-fact correspondence. Its
profile now maps quiescence to a distinct neutral kernel fact, not success/failure.
The [storage bridge](storage-bridge.md), PS-SB, invokes actual Persistence
HostWrite/Receive/Conflict operations to constrain composite result grades and
baseline agreement. The [recipient bridge](recipient-bridge.md), PS-RB, invokes
actual Disclosure transitions with the storage path, checking paired annotation
traces, current/regrant-gap release and explicit internal-evidence erasure.

| Supplier / obligation | What these models assume | Evidence still needed |
|---|---|---|
| Coordinator | Exclusive serialized publication and coherent root observation | Concrete ownership/locking/memory model and public-read refinement |
| Context/identity registry | Tokens reflect declared dependencies; IDs not confused | Membership discovery, incarnation/resource equivalence and generation lifecycle contracts |
| Host policy | Truthful established context/current grants | Supported policy profile, freshness/revocation and interface conformance |
| Host storage | Conditional compare/write and trustworthy completed/quiescent result | Provider-specific guarantee/contract and independent fault integration tests |
| Executor | Service/finite processing for termination claims | Fair-sharing direction subsequently agreed; concrete cost/capacity/service profile and progress proof pending |
| Disclosure interfaces | Every public release goes through the modeled boundary | Inventory including history, metadata, cache, diagnostics, UI and accessibility |
| Verification process | Sources/tools are identified; properties mean intended behavior | Independent contract/oracle review, trusted-tool policy, general deductive checking |

The shared-state bridge above covers a restricted reference profile for these
relations. The next general refinement must connect the complete models through
explicit relations, not merely matching component names. Required extensions include:

1. Transaction root → history content/frontiers: the same publication advances
   content, mappings and every history participant; failure advances none.
2. Transaction/history → annotation attachment: mapping preserves authored body
   history and current authority; source undo does not erase later comment edits.
3. Published snapshot → effect capture: actual incarnation/revision/binding and
   serialization profile are captured together; no staged state reaches storage.
4. Authoritative state/evidence → recipient projection: retention is not release
   authority; even conflicts/receipts/diagnostic pressure get safe projections.
5. Rich transitions → closed control kernel: each internal step/descendant/retry
   is charged to a non-refillable rank, and unmatched inputs have a total
   disposition without preventing accepted completion evidence from being kept.

These remain **open general refinement obligations** despite bounded shared-state
coverage. No shared concrete implementation/public profile is specified yet, so
a whole-system refinement theorem would invent or hide choices. The reference
root is not that implementation decision. Production correctness is not claimed.

## Completion boundary and remaining decisions

The current pass covers the four base slices, common-state bridge, concurrent
save/admission/gate composition, lifecycle/rank, abstract storage and recipient
bridges, construction arguments and parametric VCs. See the
[completion audit](completion-audit.md) for exact scope and residual gates.
It cannot complete the remaining theorem classes by
renaming assumptions as proofs. In particular:

- Actual text/anchor/inverse representation and complete public state/event
  contracts are needed for concrete refinement.
- Layout direction (§8.1) was subsequently agreed by user review; concrete
  geometry/profile conformance remains outside these slice claims. Protected
  admission/fairness (§7.5) direction was also subsequently agreed by user review;
  concrete reservation/accounting/scheduler fairness remains outside these claims.
- Numeric budgets, host/platform guarantees, full telemetry schemas and codec
  profiles need explicit specifications before resource/platform claims.
- No proof assistant is installed in the inspected environment. The temporary
  pinned SMT solver checks parametric verification conditions, not a complete
  independently kernel-checked induction/ranking or refinement theorem. Trusted
  proof boundaries and that stronger workflow remain explicit follow-up work.
- Review/oracles are not independent simply because several files or languages
  were authored. Existing upstream inspection is provenance, not source isolation.
- Usage-based reliability evaluation requires its own operational profile and
  failure/exposure definitions; it is separate from these adversarial witnesses.

The general evidence-lattice/categorical scope remains as stated in
[README.md](README.md): complete fixed-scope powerset lattice, not categorical
completeness of business/execution states. No additional categorical closure
claim is needed to discharge the explicit bridge obligations above.

Requirements §§7.6/19.7–19.8 and model revision 0.32 complete the architectural
direction review and consolidate existing editor-inspired contracts. The current
models do not specify concrete clocks, shutdown disposal, provider quarantine,
shared change representations, incremental invalidation or capability metadata.
A full rerun can validate catalog consistency and these unchanged abstract
protocols against the recorded inputs; it does not prove those additional
contracts or supply independent adapter/provider conformance evidence.

## PS-EC — adopted editor contracts

Sources: requirements §§3.2–3.3, 3.4.2, 3.8, 5.1, 5.3, 6.2, 19.1 and 19.9;
`EditorContracts.tla`; the five `SMT-EDIT-*` conditions in `parametric.smt2`.
The authoritative executable case set is `slices.mjs`, not the counts below.

| Adopted idea | New checked law / existing bridge | Residual boundary |
|---|---|---|
| Shared changes and explicit target sets | Actual sequence splice framing/inverse; two nonoverlapping edits expressed in starting coordinates equal correctly shifted sequential application; inverse restores that finite source | Real UTF-8 boundaries, arbitrary batch representation and concrete implementation refinement remain unproved |
| Bounded operation scopes | Current scope/grant checks at publication, whole-root unchanged rejection; arbitrary-integer edit/scope frame VC | Dynamic model uses a one-unit source-write footprint; exact profiles, arbitrary target sets and mapping-policy refinement remain specification work; no implicit rebase selected |
| Typed participating state | Source/attachment/history stage privately and publish one coherent root; split-state mutation fails | Uses one representative transaction; not every internal module, arbitrary history inversion or all participant combinations |
| Common attachment semantics | Finite left-affinity point mapping, deleted-interior detachment and valid-coordinate bounds; arbitrary-integer survivor round trip | This representative profile is not a newly selected universal affinity policy. Collapsed endpoints deliberately do not establish semantic identity; authored bodies survive. All range-endpoint/reattachment profiles remain open |
| Command discovery | Discovery is current-authorized and advisory; a discovered command cannot commit after revocation | Full command metadata, schema decoding, all information-flow fields and registry activation are not modeled. Existing Execution/Disclosure slices retain their own partial scope |
| Snapshot handoff and events | Old immutable value coexists with publication; current-root snapshot release checks current authority/retention; a new event release after resync rechecks both at delivery | Release models the current root at revision 1, not receipt-named historical snapshot lookup. Historical identity/availability, real queues and arbitrary subscriptions remain unproved; no raw-memory revocation or perpetual retention |
| Indexed snapshot storage | All modeled split points reconstruct identical source/range results and yield identical edit results | Representation law only: no tree/allocator, asymptotic cost, measured latency or actual reclamation proof |
| Mixed-language ownership | The representative source owner/body are preserved through source/mapping publication | Does not prove a grammar, region discovery, combined-injection mapping or parsing completeness. Ownership remains independent of interpretation |

Finite profiles: alphabet of two valid-boundary symbols, source length zero to
three, single replacement insertion length zero to two; batch insertions zero to
one with strict separation excluding ambiguous touching insertions. Static
sequence checks enumerate 10,647 input/point/split combinations; batch checks
enumerate 1,278 source/edit-pair combinations. These are harness bounds, not
product limits. The boundary profile has an eight-transition horizon and one
representative edit, three staged components, revocation, scope narrowing,
retention eviction, disclosure and subscription gap/resync events. It reaches
publication, scope rejection, denial after discovery, handoff, gap recovery and
coexistence of old/current snapshots; witnesses are existence, not fairness.

Nine controls independently remove point mapping, second-edit coordinate shift,
scope enforcement, coherent component publication, release gates, gap handling or
current-authority recheck, including separate grant/retention bypass after resync.
Each must fail its named law/property, not parse or
type checking. The five symbolic conditions check satisfiable premises and
refuted negations for length/inverse-length, point-domain, surviving-point round
trip and scope frame; the fifth retains a satisfiable collapsed-endpoint identity
counterexample. SMT does not automatically translate or refine the TLA+ model.

No new unbounded halting, categorical-completeness, actual storage performance,
native containment or implementation theorem follows. User-excluded future
feature designs have been removed; their exclusion is not a remaining proof task.

## Symbolic residual reduction — arbitrary source lengths and UTF-8

The later `SMT-TEXT-*` conditions use arbitrary integer-indexed source/insertion
arrays, arbitrary nonnegative lengths and valid half-open edit boundaries. A free
in-range index represents a possible distinguishing element; unsatisfiability of
the negated pointwise equality establishes equality throughout the valid output
domain. There is no finite alphabet or length cutoff in these conditions.

- Inverse content: storing the removed source range and replacing the exact
  inserted range reconstructs every original source element. This is conditional
  inversion at the matching resulting state, not permission to undo later edits.
- Prefix/suffix frame: source outside the replaced interval retains its values
  under the declared index shift.
- Two-edit composition: strictly separated edits in starting coordinates equal
  sequential application with the correct second-edit shift. A control omitting
  the shift must have a counterexample. Ambiguous touching insertions and arbitrary
  public batch representations are not silently included.
- Two-chunk read refinement: arbitrary valid split/read indexes reconstruct the
  flat source element; omitting the right-chunk offset has a counterexample. This
  establishes a representation law, not a tree or asymptotic performance bound.
- Region-to-source mapping: the declared affine interval profile stays in bounds
  and round trips its positions. It is not evidence of semantic symbol identity,
  arbitrary tangling, ownership transfer or a provider's truthful mapping.

`SMT-UTF8-*` follows [RFC 3629 §§3–4](https://www.rfc-editor.org/rfc/rfc3629.html).
The checked scalar domain is 0 through U+10FFFF excluding U+D800–U+DFFF. Integer
division/remainder encode one-to-four-byte sequences; decoding is checked against
the RFC's restricted leading/continuation-byte cases. Used bytes and declared
length, not unused tuple slots, define a sequence. The obligations establish
scalar round trip, encoded-byte validity, decoded scalar validity, canonical
re-encoding and separation of continuation bytes from valid leading bytes.
Removing surrogate, overlong or upper-range restrictions yields the required
countermodels. All positive conditions first establish satisfiable premises.

This closes the former finite-only limitation for these symbolic source laws
and the scalar codec, not all Unicode behavior. It does not prove an arbitrary
stream decoder, boundary scanner, byte/scalar index, BOM/newline policy, malformed
input recovery, normalization, grapheme segmentation, shaping or visual geometry.
[UAX #29](https://www.unicode.org/reports/tr29/) treats segmentation separately;
no Unicode property-table version, tailoring or implementation is selected here.
Z3 and the handwritten encoding remain trusted; no independent certificate replay
or automatic refinement from these conditions to Rust/TLA+ is claimed.

## PS-DR — live retention and derived-result acceptance

Sources: requirements §§3.2, 3.4.2, 5.2.1, 6.1–6.2, 7.5 and 15.5;
`DerivedRetention.tla`; manifest and partial mappings in obligations.json.

| Obligation | Checked mechanism | Explicit limit |
|---|---|---|
| Immutable observations coexist with editing | Root and worker share immutable cell IDs; publication allocates a distinct value; captured source remains stable | Two source cells, one edit and one worker, not arbitrary allocation graphs |
| Live source-cell obligations remain charged | Allocated source cells equal the union of root/worker-held cells, not the count of aliases | Ghost captured values are not modeled buffers. Completed results, queues, history, save payloads/receipts and native allocations are outside this ledger; no whole-engine or allocator memory bound |
| Cancellation is not resource release | Cancellation records intent but retains worker-held cells; only completion releases that reference | No claim that arbitrary native workers can be forced to finish |
| Admission respects resource capacity | One/two-cell profiles forbid publication that would exceed live-cell capacity | Harness capacities, not numeric product limits, process RSS or a scheduler |
| Reclamation preserves live readers | No held cell can be absent from allocated storage; completed work permits old-cell reclamation | Eager abstract reclamation, no destructor/GC latency or crash semantics |
| Mixed-language/analysis results are contextual | Result acceptance checks captured source revision, analysis context and current grant; changes hide previously visible stale evidence | Context is an abstract profile token whose invalidation truth is a supplier obligation; grammar correctness, discovery completeness and actual parsing are unproved |
| A provider is not a source owner | Worker completion cannot publish source or transfer owner identity | One local owner; no new ownership or extension runtime selected |

Two positive safety profiles, six independently weakened controls and seven
witnesses explore a six-transition horizon. Controls remove cancellation retention,
context/revision/grant acceptance checks, ownership isolation or capacity admission;
each must fail its named property rather than merely exit with a tool error.
Witnesses reach old/current coexistence, cancelled-but-held work, reclamation,
fresh acceptance, stale context/revision retention and a capacity-blocked edit.
Existential witnesses are not fairness or eventual-worker-completion proofs.

## Remaining proof inputs — no implementation or policy invented

The specified mathematical/protocol residuals above now have additional checked
evidence. The following cannot be established by adding more abstract states:

| Claim still open | Input needed before a conclusive proof/test |
|---|---|
| All public verbs, fields, coordinates, attachment/history/event profiles | Concrete versioned schemas, profile validity and representation/refinement relations. D01/D02/D03/D04 are adopted in specification §§6.4/4.2.1/6.5/4.3.1, with scoped evidence in noop-publication.md, batch-normalization.md, replay-reference.md and mapping-profiles.md. Concrete classifier/canonicalizer/admission/retention/point/replacement/history/geometry refinement and D45 interaction profiles remain; this is not a complete executable specification |
| Full Unicode/navigation/codec/UI behavior | Declared segmentation/property/tailoring and platform profiles plus actual algorithms/adapters; scalar UTF-8 correctness does not supply these |
| Concrete storage speed, memory and reclamation | A chosen data structure, implementation/allocator ownership, resource units and workload/hardware envelope; no production implementation exists |
| Parser/semantic correctness and complete dependency discovery | Concrete grammar/query/provider contracts, real source mappings and independent completeness/correctness oracles; context gates only reject stale/ungranted evidence |
| Real host persistence/identity and fairness/service guarantees | Actual provider/platform protocols and service assumptions with conformance evidence; abstract receipts and cell budgets cannot prove them |
| Whole implementation refinement and independent assurance | Concrete code/refinement mapping, general inductive/certificate workflow and independent review; same-author models and trusted-solver output are not that evidence |

These are missing prerequisites for stronger v1 claims, not deferred feature
designs. No new backend, numerical budget, Unicode tailoring, grammar or proof
assistant is chosen to manufacture their discharge.

## Complementary lens contracts — PS-AC, PS-PV and PS-UP

See [review-lenses.md](review-lenses.md) for the selected resource/RED/outcome,
LINDDUN and interaction/failure review and [assurance.json](assurance.json) for
four selected claim/argument/evidence mappings. No full-method certification or
complete implementation refinement is implied.

| Contract | Model / cases | Argument and limits |
|---|---|---|
| PS-AC conservation | AccountingBridge imports actual SaveAdmission actions and checks offered/admitted/rejected partition, reservation-count and weighted-cost conservation, nonnegative balances and imported Next correspondence. Nine cases: one positive, four controls, four witnesses | Ghost counters increment independently at actual admission/discharge actions and are compared with held-set state. Controls omit admission, double a refund, refund active cancellation, or use wrong cost. Four one-shot requests/two resources/five transitions; not a production metric or arbitrary-history/physical-memory theorem |
| PS-PV privacy/current diagnostic release | PrivacySignals imports actual Disclosure steps with a single prepare/drain; paired output equality, frozen delivery-time facts, core Next/stutter correspondence and diagnostic nonmutation. Sixteen cases: five positive profiles, three controls, eight witnesses | Causal witnesses require delivery-time conditions and Health for authorized masking. Four regression paths reach legacy false-positive endpoints while strengthened witnesses remain false. Deliberately exposed tags/counts or bypassed regrant epoch fail. Same public schedule/fixed recipient profile; no timing/resource/cache side-channel, statistical anonymity, legal compliance or erasure theorem. Grant and epoch changes are coupled by the imported model, so independent grant-guard sensitivity is not established |
| PS-UP useful progress | UsefulProgress checks complete identity partition and stable applied/superseded/cancelled dispositions, current applied acceptance, cancellation intent retention and strict service rank decrease. Ten cases: two positive sizes, four controls, four witnesses | Bound=2/3 is initial closed backlog cardinality, with no later arrivals/retries and NO transition cutoff. Explicit weak fairness of Serve plus a strictly decreasing natural pending rank yields eventual drain. Missing fairness admits infinite unfinished stuttering; silent removal/stale success/cancellation discharge each fail. Not an actual scheduler refinement, open-load fairness, response-time bound, universal applied success or native preemption |

The PS-UP construction argument is: while pending is nonempty, cooperative Serve
is enabled; non-service actions cannot increase pending, and each Serve removes
one job. Weak fairness precludes an infinite suffix with enabled service but no
service. Finite initial cardinality therefore implies eventual empty pending.
TLC checks the complete finite state space of each configured backlog, including
infinite stuttering/cycles for its temporal property; the general argument is
authored reasoning, not independently kernel-checked induction. Witnesses prove
reachability only and deliberately omit EventualDrain from their checked property
list so a safety counterexample is not confused with a liveness result.

Six further symbolic conditions check arbitrary nonnegative ledger-update balance
and non-underflow, strict unit drain rank and fixed public-field erasure, with
omitted-discharge/no-service/private-tag countermodels. Positive premises must be
satisfiable and negated conclusions unsatisfiable. These formulas are not an
automatic induction or refinement mapping to TLA or production code.

The assurance graph requires arguments, conditions, limits, contract/scenario and
assumption links; positive supports must name actually configured checked
properties in the declared slices. Controls and witnesses have different expected
types. SHACL controls remove a progress assumption and substitute an INS point
for a proof check. Success establishes supplied-artifact consistency/sensitivity,
not the truth of assumptions, installed observations or independent assurance.

## Review-finding closure — delivery witnesses and exploration metadata

The separate revision 0.41 reviewer found no in-scope safety/progress defect but
two minor evidence issues. Revision 0.42 captures delivery grant/disposal/epoch,
captured epoch, hidden divergence and output kind in a ghost record that is
stable after drain and agrees with the emitted output. Gap/disposal witnesses
now require those conditions at delivery; hidden masking also requires an
authorized Health output. This is proof instrumentation, not a public schema.

Four scripted positive regressions plus endpoint witnesses establish reachability
of the misleading legacy states without satisfying the strengthened witnesses:
regrant after denied delivery, disposal after an unrelated denial, hidden
divergence after delivery, and a denied-only response despite hidden divergence.
Ordinary safety/correspondence checks remain enabled throughout.

AlgebraSpec/BatchSpec statically enumerate initial states and then stutter.
Their inherited Bound parameter is unused. The unchanged source/insertion limits
are now explicit SourceLimit=3, InsertLimit=2, BatchInsertLimit=1; the alphabet
still has two representative symbols. Generated graph and report metadata record
the selected specification and separate static domains from transition horizons
and finite initial backlogs. SHACL controls reject a false transition-horizon
role and missing static source limit; these do not add an algebra proof claim.
