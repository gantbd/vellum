# Owned geometry provider work

Specification §19.6 supplies the geometry branch of F25 request admission and
F28 derived-result acceptance. The captured-geometry-work/1 reference profile uses
a serialized bounded owner. Captured source/context, correlated receipt evidence
and current accepted geometry are separate products. No provider executes inside
the local transition and no worker can publish authored source or source history.

## Precedent and placement

[CodeMirror's view implementation](https://raw.githubusercontent.com/codemirror/view/main/src/editorview.ts)
separates update and measurement phases and schedules/coalesces explicit measure
requests. This supports the separation, not adoption of its DOM scheduler,
callbacks, iteration limits or runtime. [LSP 3.17 cancellation](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/specification.md#cancellation-support)
still requires a response to a cancelled request; its client guidance distinguishes
stale results that remain useful from invalid ones. It is precedent for correlation
and cancellation/result separation, not a new LSP/JSON-RPC/HTTP/network dependency.
Vellum's stricter current-geometry checks follow §§17/19 and PS-DR's retained-work
rules rather than assuming that every old editor result can safely be remapped.

| Choice | Benefit | Cost and disposition |
|---|---|---|
| Owned capture and correlated result intake | Deterministic bounded transitions, current checks, no ambient native callback | Retains capture/evidence and needs actual host correlation; selected reference |
| Native callback inside a query/publication | Direct access to provider state | Cannot inherit boundedness, re-entry or failure isolation; not the selected path |
| Treat cancellation/result as worker discharge | Fewer state dimensions | Discards unresolved obligations without evidence; rejected |
| Separate quiescence from retained evidence retirement | Honest late/reordered evidence and resource attribution | Requires explicit attestation/drain/retention predicates; selected profile |

## Capture launch and authority

geometry-work/1 names enabled-provider-measurement, profile, exact originating
operation/effect/attempt, provider incarnation and complete geometry context. The
view must explicitly own viewport generation and unit/configuration/metrics/
projection context; no missing-as-zero rule. Current document/view/source and
annotation generations, geometry context and release predicates must agree.
Authority, disclosure, continuity, liveness, stop, service, capacity and reservation
predicates are checked at admission and again before submission. Provider identity
must agree with the actual current service. Availability is not disclosure authority.

Capture the actual immutable source and request, not a caller-supplied replacement
snapshot. Bound capture/request JSON bytes, source bytes, tasks, outstanding work,
total issued origins, receipt envelopes and aggregate held logical bytes. Reserve
capture charge plus two maximum envelope charges before admission, and ensure the
profile can receive its minimal quiescence and every declared failure envelope.
Fixed record/fact metadata and used-origin bookkeeping are bounded by task/issuance
counts; the serialized-byte charge is not process/native memory accounting.
Accepted-layout/cache storage has separate geometry obligations; the task charge
does not establish aggregate layout/native memory accounting. Registry uniqueness
and ledger claims are per owner. Actual integration must bind the engine's owner
and inherited host envelopes; new registry construction cannot reset real quotas.

Ready work is admitted but not yet submitted. Submission rechecks current context,
exact source bytes and current launch/disclosure gates, then emits one immutable
capture. It does not invoke the provider. Repeating submission cannot relaunch;
an origin triple is never reused within the registry, even after retirement.
The finite issued-origin bound prevents retirement from resetting this ledger.
This is bounded explicit staging, not a service/fairness or executor guarantee.
Actual atomic handoff to a host, retry and full invocation/event composition remain
separate obligations. No deadline or measured numerical default is invented.

## Receipt evidence and current geometry

geometry-work-evidence/1 identifies the owned task, exact origin and provider, with
closed result, failure or quiescence variants. Failure reasons are unsupported,
exhausted, cancelled or fault. Authenticity, admissible evidence service and bounded
ingress are premises; exact actual correlation precedes payload traversal. A
forged/unrelated event neither consumes a witness nor faults an honest task.
Current edit/read permission is not required merely to retain authentic historical
evidence; a new current acceptance still needs its own current release authority.

The first bounded result/failure is retained as evidence. Receipt alone does not
validate geometry or prove quiescence. Exact duplicates, including different record
property order, are idempotent. A second distinct terminal witness retains both,
marks the attempt contested and invalidates any issued layout. A third distinct
witness returns evidence-full without growing or replacing the pair. It is not
silently accepted, truncated to success or treated as worker discharge. Actual
overflow handling/escalation and external delivery retention remain host obligations.
This two-witness store is not a lattice closed under arbitrary payload union.
PS-DR and Evidence.tla retain their separately stated abstract domains; neither
becomes a theorem about all request/result payloads or whole-editor state.

Acceptance requires one uncontested result, no cancellation/supersession, current
provider identity and current source/view/annotation/viewport/metric context.
The packet must match the captured context/source, then pass the actual bounded
geometry validator. A malformed or stale bounded receipt cannot become current
layout. Repeated acceptance returns the same owned layout, subject to current
geometry checks, not a new allocation/reference per query. No source transformation,
implicit selection or viewport movement follows. F09/F12/F15 consumers retain their
existing current-context checks. At most one task has current accepted geometry
for a view within the owner: a competing current result refuses until explicit
supersession; accepting a new current context retires stale peer references.
Trusted retirement of a layout also invalidates
already issued targets and prepared reveals through those actual consumers.

Cancellation/supersession before submission closes unsent work without launching.
After submission it stops applicability but preserves the outstanding-work slot.
Late authenticated evidence remains admissible. A separate correlated quiescence
fact, under actual attestation, releases that work slot; it does not erase capture/
evidence charges or imply geometry success. Reordered quiescence before result
delivery is supported. Retirement requires no outstanding work, current retirement
authority, released retention obligations and drained delivery. It invalidates any
remaining layout and removes the capture/evidence record, not used-origin history.
After retirement, the old task cannot be reused. These actual attestation/drain/
retention facts are supplier premises, not established by reference booleans.

## Finite execution and checks

For bounded normalized input and trusted engine state, each call uses finite
catalog branches and bounded tasks/witnesses/JSON/source/
geometry traversal, with no recursion, provider callback, retry or wait loop.
Cancellation, supersession, quiescence, witness accumulation, conflict and retirement
are monotone within a task; duplicate observations may stutter. This establishes
per-call bounded reference work, not worker halting, fair service, whole-service
termination or categorical completeness. Repeated external inputs need not stop.

The admission invariant is tasks ≤ maxTasks, in-work tasks ≤ maxOutstanding,
issued origins ≤ maxIssued and total retained charge ≤ maxHeldBytes. Initially
all are zero. Admission checks every delta before insertion; submission preserves
charges/counts; cancellation only removes an unsent slot; receipt/acceptance cannot
increase the reserved charge; attested quiescence removes only a work slot;
retirement removes retained charge and never used-origin history. Thus induction
preserves the declared ledger, without establishing physical-use ≤ charge.
Accepted-layout safety follows separately: acceptance requires one matching current
result, invalidating transitions retire its actual layout, and each consumer checks
that retirement plus current context. No arithmetic ledger law proves provider
authenticity, correct measurement or safe host discharge.

The checks enumerate 32,768 admission and 32,768 dispatch gate combinations,
256 acceptance and 16 quiescence-intake combinations. An independent lifecycle
oracle explores to reachable closure: 101 states and 909 transitions for its three
terminal-witness symbols, controls and fixed context. There is no fixed trace
horizon. This is a selected finite abstraction, not all packet/context/resource
states. One hundred controls and twenty-two load-success/assertion-failing mutants
cover ownership, correlation-before-traversal, capture/evidence aliasing, usable
completion reservation, aggregate charges, late/stale/conflicting results,
cancellation, quiescence, retirement and issuance. Actual geometry queries and a
prepared F15 reveal demonstrate revocation; authored source/history remain unchanged.
Native measurement/provider truth, analysis work F24/F28, policy/authentication,
physical accounting, hostile decoding, concurrent/complete companion-state and
full effect/invocation/observer refinement remain separate evidence gates.

## Instrumentation and threat coverage

INS-10 separates admission, capture, final submission, provider service, receipt,
current acceptance/contention, geometry reuse and invalidation. INS-01/03/08 cover current
interpretation/release, task/issuance/capture/evidence reservations and pressure;
INS-04/13 cover correlated quiescence, cancellation, late evidence, drain and
retirement. MELT/golden/RED distinguish admitted/submitted/recorded/duplicate/
contested/stale/unavailable/accepted/quiescent/retired outcomes and their actual
queue/service/receipt/acceptance/discharge latencies. USE tracks outstanding slots,
retained captures, witness storage and native resources under their declared units.
STRIDE covers forged/wrong-attempt/provider evidence, source/context substitution,
stale accepted references, witness floods, early refunds and replay after retirement.
LINDDUN covers source disclosure to providers, reading/viewport activity and
correlation/retention leakage. Use bounded reason/work/budget classes, never source,
task/provider/document IDs, origin strings, positions or raw metric tags as labels.
Actual signal placement, privacy, overhead and host measurements require evidence.

The implementation is a trusted reference owner with private WeakMap state and
internal inspection/composition facets, not a public plugin ABI or security boundary
against malicious same-process code. Rust/wire formats, production supervision,
transport, runtime, dependency and host matrix are not selected here. Requirements
and annotation point persistence are unchanged. This evidence is not independent
source-isolated Cleanroom certification.
