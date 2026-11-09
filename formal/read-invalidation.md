# Native read frontier invalidation

Specification §12.8.3 extends absorbing observed invalidation to actual F01
discovery, F04 history inspection and F06 invocation observation. A bound loss
closes delivery, not the producer or its authoritative state. Unknown facts,
transient refusal and foreign/malformed requests have separate behavior. The
existing F07 settings/resource obligations below remain in force.

Specification §22.2.2.1 makes observed invalidation absorbing in the actual F07
resource-metadata reader. This follows the adopted read/disclosure lifecycle in
requirements §15.5 and specification §12.8: retention is not permission, and new
authorization does not silently resume an old frontier across a gap. The settings
reader's corresponding contract is §22.2.1.1. Neither closure proves all read
surfaces or real host policy truth.

## Owned resource reader

[registration-reads.mjs](registration-reads.mjs) binds an actual context, scope
and recognized exact envelope before recording explicit false authorization,
scope permission, continuity, liveness or retention. It also records a current
cut's loss of captured visibility/facets or changed permitted metadata. The private
closure transition stops delivery, clears data and dependent selectors, and retains
the charged context until explicit discharge. It does not mint control authority,
change native catalogs, cancel producers or refund issuance. Explicit owner stop
still has its original authority check.

Regrant and actual native metadata ABA cannot revive the old context. Fresh
acquisition remains available within the existing bounds. Missing/non-Boolean
facts and transient resource/transfer/reply refusal or query cancellation are not
themselves an observed authority loss. Malformed envelopes and foreign context/
scope claims cannot poison another context. Current host policy/continuity truth
and delivery of actual invalidation events remain premises; a receiver cannot
establish those facts solely from a flag or handle.

## Audit coverage and remaining work

| Surface | Current evidence | Remaining distinction |
| --- | --- | --- |
| Native settings F07 | Owned absorbing query invalidation, current cuts, fresh frontiers and explicit discharge; engine-settings-reads.md | Mutable registry/consumer/membership/policy truth and complete public products remain separate |
| Native resource F07 | Owned absorbing invalidation and actual metadata ABA; evidence below | No universal policy-event propagation, global budget, provider truth or wire/Rust claim |
| Native discovery F01 | Exact bound authority/interpretation/retention loss, owner fault and changed permitted metadata close delivery; publication guard precedes closure | Advisory-only target/permission loss remains separate from metadata; actual accepted catalog/policy truth and aggregate charging remain gates |
| Native history F04 | Exact bound grant/continuity/retention/history loss or actual relevant-basis mismatch marks context nonlive, including trusted alias resolution | Capture remains retained until discharge; actual inverse/companion/policy truth, public acquisition and aggregate charging remain gates |
| Native invocation F06 | Exact bound policy/grant/continuity/retention or actual ledger-binding loss closes delivery and every waiter; exact-true readiness/control/discharge | Missing outcome alone preserves phase observation; actual host policy/clock/global admission and producer retirement integration remain separate |
| Publication event/outcome observer | Its internal context argument represents accepted current host state; observed context change deliberately fences the old stream under publication-observation.md | Do not reinterpret that argument as an untrusted caller scope. Complete northbound caller binding, strict readiness/control/discharge and host-fact roles still need their own audit |
| Accepted geometry/layout artifact | View/document/provider interpretation and layout/target validity are checked by geometry-products.md; the artifact is not a complete recipient-owned frontier | Per-recipient release integration and actual supplier truth remain separate; one recipient refusal does not establish shared artifact invalidity |
| Accepted analysis/parse artifact | Two matching views share one accepted snapshot; producer query validity and retained parse-base validity have distinct transitions in analysis-work.md | Inventory actual recipient release wrappers before claiming frontier closure; do not destroy a reusable parse fact merely to stop one recipient's delivery |
| Mediated F02/F03/F04 helper | Explicit close/invalidate transitions are absorbing; caller installs returned registry state | Query refusal alone is not that transition; actual owner/event installation is still a reference-integration obligation |

This is a selected cross-reader audit, not an inventory closure for every history,
execution-observation, geometry, analysis, export or retained projection profile.
Their actual lifetime/owner paths must be inspected before claiming a common
frontier guarantee. Pure helper state, actual owned state and real host integration
are different evidence levels. Unfinished native owner integration is design/
reference work, not a gate that can be relabeled as measured external truth.

Context roles are part of each interface contract. A caller scope claim must not
poison another context, while accepted host current-state evidence may deliberately
invalidate an internal observer. Shared producer/cache artifacts likewise cannot
be equated with recipient-scoped delivery handles. Retention, producer acceptance,
current disclosure and reply-route invalidation require their own facts. These
distinctions preserve the existing ownership boundary; the table is not a new
universal recipient registry or proof that remaining wrappers are implemented.
Shared parent/global logical-budget integration likewise remains design/reference
work where required; measured physical ceilings and real allocator behavior are
separate external evidence. This reader's fixed local reservation is not proof of
either aggregate charging or physical memory accounting.

## Executable evidence

Three additional checkers exercise the selected actual owners:

| Checker | Focused cases | Mutation evidence |
| --- | --- | --- |
| discovery-invalidation-checks.mjs | 13 observed losses, 24 unknown facts, three transient refusals, twelve request-isolation cases and nine protected-discharge cases; native root/producer and held-capacity frames | Eight assertion-killed mutants after an unmodified baseline |
| history-invalidation-checks.mjs | Five observed losses, fifteen unknown facts, two transients, eight foreign scopes, three malformed envelopes, six exact-control cases, five trusted bridge cases and nine discharge cases | Eight assertion-killed mutants after an unmodified baseline |
| invocation-invalidation-checks.mjs | Six observed losses, fifteen unknown facts, four transients, eighteen request/policy/wait isolation cases, twelve readiness, nine wait-admission, six control and 27 discharge checks; all-waiter closure, Applied outcome and producer frames | Thirteen assertion-killed mutants after an unmodified baseline |

Run each with `node formal/<checker>`. Their mutation modules replace selected
rules in the actual reader module and retain the unchanged native owner/kernel
composition; import/syntax errors do not count as kills. The legacy discovery
suite retains 42 native stages, 48 release vectors, 23 controls and 32 mutants;
history retains 56 native cases/112 stages, 8,192 acquisition/1,536 release cases
and fourteen mutants; invocation retains 7,168 admission/21,504 release cases,
4,096 projections, 585 wait traces/836 edges and fourteen mutants. Each release
vector now gets an independent issued context and explicit cleanup. These are
reference workload bounds, not selected production defaults.

Discovery observes exact false authority/interpretation/retention or owner fault;
unknown facts deny without closure. The outer guard and recognized request envelope
precede private changes. Current captured metadata/facet loss is absorbing, while
advisory-only target/advice refusal does not retire still-current metadata.
History applies its bound loss transition to both query and trusted alias bridge;
closing inspection does not cancel an independently owned reversal preparation.

Invocation invalidation clears delivery references and ends all owned waiters,
retaining records and charges for protected discharge. Current policy must first
match captured recipient/epoch; unrelated caller policy cannot poison a context.
Foreign/invalid wait selectors and nonpositive/nonintegral budgets refuse before
query-induced closure. Unknown facts and ordinary capacity/transfer refusal do not
establish context loss; an unsuccessful poll may nevertheless consume its one wait
eligibility. Missing/forgotten outcome data is distinct from losing the actual
phase/policy binding. Closing an observer never cancels or refunds its producer,
rewrites historical Applied facts, grants retry or guarantees physical erasure.

Run `node formal/read-invalidation-checks.mjs`. Five gap cases, four transient
cases and ten unknown-fact cases accompany eight controls and eight assertion-
killed mutations after a coherent unchanged baseline. The checks exercise actual
native resource registration away from and back to a prior public value, unchanged
source/producer frames, selector removal, retained capacity, spent issuance,
foreign claims and independent context holds. Syntax/import errors are not kills.

The existing resource-reader suite still checks 42 stages, 378 traversals/2,268
inspections, 2,048 preparation/512 acquisition/1,024 release vectors, 96 selected
hidden-world pairs and 21 controls, plus forty mutations in five eight-case banks.
Release vectors use a fresh issued frontier per case with a finite workload-derived
issuance bound. That fixture bound is not a product default. Query refusal leaves
native roots, reader reservations and spent issuance unchanged; observed loss
additionally closes private delivery state. No regrant-resurrection assumption
is used to keep the gate-vector tests running.

Existing REL-SETTING observed-gap absorption/unobserved-frame laws and their
weakened-rule witnesses apply only under their declared bound-observation model.
For this reader, the delivery-closed bit maps to its owned stopped state, while
the invalidated marker distinguishes the observed reason for supervision. These
abstract step laws are not a JavaScript or real-policy refinement proof. Totals
remain 274 solver conditions/fourteen manifests, 60 baseline. Full frozen-input
verification is required for the completed batch.

## Observation and threat coverage

INS-03/08/09/12/13 distinguish preparation, current release, observed closure,
retained invalidated contexts and final discharge. MELT/golden/RED count/query
durations and outcomes by safe bounded stage; USE distinguishes occupied capacity
and refusal without early-refund claims. STRIDE covers forged scope/handles,
stale authority, metadata replay and premature release; LINDDUN covers hidden
existence, private metadata and cross-context correlation. Invalidated counts are
trusted supervision only, not ordinary recipient metadata. Exclude raw contexts,
identities, stamps, handles and metadata values from ordinary logs/metric labels.
No new interface, network, job kind or UI/filesystem ownership is introduced.
