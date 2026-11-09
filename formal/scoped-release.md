# Scoped references and current recipient release

Specification §12.6 adopts D09–D12/D42's baseline: scoped non-grant references,
current checks at every bounded delivery without a lease, serialized owned-value
handoff and recipient-safe whole-envelope projection. The embedding host is trusted;
logical recipient distinctions apply to mediated APIs, not arbitrary shared memory.

## Conditional release and projection arguments

Section 23.1.4 adds six actual invocation-observation conditions for isolated control
progress/reserve, tick rank and nonexecution, plus two unsafe SAT controls. Current
totals are 33 scoped/223 suite conditions across thirteen manifests; see
[invocation-observation.md](invocation-observation.md) for actual native composition
and the distinct global reservation/host-clock/physical/delegation gates.

Section 14.6.1 adds six native reversal-receipt conditions: captured/current target
meet, original scope, historical direction, confirmed refusal and two unsafe SAT
controls. At that batch totals were 27 scoped/217 across thirteen manifests;
see [reversal-results.md](reversal-results.md) for the actual reference and limits.

Section 14.5.1 adds six local owned-history context conditions to the fifteen below:
complete participant basis, unrelated-publication independence, absorbing observed
invalidation, lifetime issuance and two unsafe SAT controls. Current scoped total
at that batch was 21; suite total 211 across thirteen manifests. See
[history-contexts.md](history-contexts.md) for the actual native composition and
the distinct host truth/unobserved-continuity/public encoding/physical gates.

Section 13.6.4 separately instantiates scoped disclosure for native annotation
decision receipts in [annotation-results.md](annotation-results.md). Its five
identity/version capabilities form a bounded distributive lattice, with release
by captured/current meet and exact invocation/native-record binding. Six new local
solver conditions extended the original nine to fifteen: four positive gate/closure
laws and two SAT controls. The original executable profile/mask counts below remain
unchanged. Neither lattice closure nor a solver pass proves host grant truth,
whole-engine completeness or automatic implementation refinement.

Let a retained candidate bind its relevant authority/recipient, incarnation,
snapshot/history context, purpose/destination, projection and stream dependencies.
Handoff requires current accepted grant, scope match, valid retained availability
and continuity. Serialize this check and transfer relative to accepted local
policy/lifecycle changes. If invalidation precedes handoff, the candidate cannot
release under the obsolete context. If handoff precedes invalidation, its already
disclosed immutable value remains as-observed. This is a local ordering argument,
not recall of memory, a distributed revocation protocol or physical-export proof.

An internal queue retains engine ownership until actual recipient handoff. Merely
enqueuing under a prior grant supplies no later authorization. A recipient-readable
reference already crosses the boundary; calling it internal does not restore
mediation. Actual memory ownership, channel/lock implementation, bounded allocation
and exclusion of arbitrary callbacks must refine the chosen handoff operation.

Let C be the authorized capture field set and A the currently approved projection.
The effective set is their intersection. Thus delivery cannot exceed either C or A;
grant expansion cannot enlarge an existing candidate. Within one comparable fixed
field universe, intersection is the meet in a finite powerset lattice. This is not
a lattice theorem for policy, identity, temporal validity or whole engine execution.
Invalidation may require rejection/new preparation rather than merely intersecting
fields. Opaque selectors, retained bytes and policy-cache existence are not grants.

For a fixed declared projection, equal visible data and equal permitted operational
inputs yield equal protected functional envelopes when inaccessible records vary.
Compute counts/cursors/events over the visible domain; never expose hidden global
positions or hidden-only revision changes through those fields. Where existence is
withheld, one unavailable envelope masks the internal rejection cause. This argument
assumes complete field/dependency classification and correctly supplied visibility
and policy facts. Timing, resource contention and trusted-host observations remain
outside this functional-output equivalence.

## Finite reference and stream exploration

`scoped-release.mjs` has six private field flags: source, annotation existence,
body, target, author and events. Body/target/author/events require existence,
yielding 34 valid masks and 1,156 capture/current-mask pairs. Each pair checks
one authorized owned-value handoff, 14 independently invalid current contexts,
an absent candidate and a candidate prepared without a grant: 19,652 handoff
cases. The invalid contexts change nine scope coordinates or current grant,
epoch, availability, validity or continuity. An independent exact field oracle
checks successful projection; blanket refusal cannot pass.

Two hidden-world variations per pair (absent private record or different private
body/ID/target/author/event) give 2,312 paired projection checks. There are 1,156
input/delivered-value frame checks. Eighteen direct controls cover grant possession,
recipient/purpose/snapshot/projection substitution, capture expansion, hidden counts/
events, unavailable retention, regrant/gap handling, a faulty second emission after
prior success and unsupported private actions.
These selected examples do not establish complete mutation adequacy.

A separate closed six-action stream model explores all 28 reachable states and
168 outgoing transitions without a transition cutoff. Its actions are prepare,
revoke, regrant, gap, resynchronize and release. One pending batch, two grant
invalidation epochs, a delivered-ever bit and a per-step handoff result let the
monitor check each new delivery against prior grant, baseline and queued context.
The per-step result remains sensitive after an earlier successful delivery;
a deliberately faulty second emission must fail the independent observation check.
Revocation/regrant never retract an already delivered observation. A gap clears the
pending batch in this fixture so resynchronization cannot revive an old delta;
the contract also permits a justified reproject/rebuild under newly valid continuity.
This finite model can cycle or stutter indefinitely: closed vocabulary and finite
state are not a liveness/termination guarantee for queues or external waiting.
Individual reference calls have fixed finite loops; no scheduler or clock is modeled.

Nine `scoped-release.smt2` conditions comprise five positive local laws (SAT premises,
UNSAT negated conclusion) and four SAT controls: current gate, capture meet,
invalidated context, stream continuity and visible count; controls show possession,
old enqueue grant, global hidden counts and regrant without a new baseline. The
visible-count law assumes the defined count omits hidden state; it does not prove
that every real endpoint implements that definition. Trusted Z3 checks the local
encoding without independent proof-certificate replay.

## Scope and supplier obligations

The helpers consume valid private records and host-approved visibility, grants,
scope facts and current projection masks. They are not parsers, policy evaluators,
authentication, secure handles or safe interfaces for hostile in-process callers.
The prepared candidate is engine-private, including inaccessible source facts;
returning it to a recipient would itself be disclosure. The projected result is an
owned immutable value. Six fields, nine scope coordinates, literal cursor labels,
epochs and capacities are fixtures, not a public schema or token encoding.

Exact snapshot availability is a supplied fact, not a historical lookup algorithm;
the model never proves that a receipt can still find its snapshot. It models neither
token issuance/retirement/generation reuse nor actual queue ownership, native UI,
physical clipboard/export, arbitrary subscriptions, runtime contention or complete
release-surface coverage. D03/D07/D08/D14/D22/D33/D35/D36–D38/D43 retain their relevant
refinement gates. Existing PS-DI/RB/PV/EC and D05 outcome evidence remain separate;
combining their passing results is not a whole-interface composition proof.

## Instrumentation and security boundaries

IF-02/08/09 and INS-03/12 retain current release, denied/unavailable projection,
retention availability, gap/resynchronization and diagnostic boundaries. PATH-02/11
must distinguish request admission, internal queueing and actual recipient handoff.
Count offered/prepared/handed-off/refused batches separately; measure internal
pending age, handoff cost, retained/reserved occupancy, loss and useful consumption
only where observable. Recipient rendering/physical export are not inferred from
local transfer. Actual units, field schemas, cardinality and budgets remain open.

STRIDE covers recipient/context spoofing, selector tampering, stale grants/caches,
handoff races, queue floods and diagnostics-as-authority. LINDDUN covers existence,
private counts/cursors/events, correlation and retention linkage. No raw content,
paths, selectors, identities or global sequence values become default metric labels.
Telemetry loss/off cannot erase authoritative outcomes or authorize release.
