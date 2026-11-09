# Actual invocation status and bounded observation

Specification §23.1.4 connects F06's existing four products to actual admitted
invocations. It reads the shared ledger's phase, original policy and retained
authoritative outcome, not an execution replay fixture or a fresh command evaluator.
Observation never starts, stops, retargets or refunds the producer.

Section 17.2.3 separately couples this actual producer's canonical charge and native
context retirement to shared owned host/origin admission; see
[observation-admission.md](observation-admission.md). The older local checker below
retains its global-reservation exclusion; physical/all-owner/origin and real host
discharge remain separate gates. This lower-level opener is not a public fallback
around required shared admission.

## Precedent and boundary

[Erlang monitors and process aliases](https://www.erlang.org/doc/system/ref_man_processes.html#monitors)
separate monitoring from linked lifetime and support deactivating a reply route.
This supplies the observation-lifetime precedent, not an Erlang runtime dependency,
host timer proof or permission to discard authoritative provider evidence. The
existing F06/F19 and §§6.5/17/23 contracts remain the owners of these distinctions.

The trusted metadata hook returns only the actual admitted invocation's reference,
pending/terminal phase, original policy, copied retained outcome and declared result
bound. It returns nothing for unissued, foreign-context or retired tickets. It is
not a public raw-state API. No source root, contents, receipt target, participant
graph or global publication counter enters this status path.

The internal opener binds the actual opaque ticket and complete original admitted
authority/engine/recipient/purpose/destination/profile/continuity, with an independent
captured observation ceiling. This selected path does not delegate to a different
audience or silently change purpose. Original invocation, captured observation and
current policy ceilings all apply; current epoch/recipient and release gates must
match. Public acquisition, authenticated delegation and wire/Rust representation
remain gates, not new public verbs or transport assumptions.

## Products and reference isolation

Reuse the existing closed execution-observation/1 fields and variants. A fresh owned
native context is not a source snapshot, history handle or invocation ticket. Exact
request shape, invocation, context and profile are required. Copied/foreign handles
do not resolve. Status reports permitted pending/terminal state, with current
projected retained facts where available. Retained-outcome requires actual retained
facts; forgetting does not imply rejection. Faulted actual ledger facts report
Unknown, and a local terminal phase does not establish external quiescence/success.

| Projection | References and resources | Permitted progress |
|---|---|---|
| ordinary status/outcome | Bounded ordinary references, reply space and current ordinary resources | Changes to the permitted control/outcome projection; unchanged reads reuse their reference |
| minimal control status | Separate protected reply and two reserved reference units | Only pending to terminal; no rich outcome or outcome-retention progress |

Issue opaque observation references only after the complete frame fits. Ordinary
reference signatures use the currently permitted phase/outcome projection, not raw
private facts or the engine counter. Control signatures use phase alone. Hidden
rich changes and forgetting an outcome cannot advance a control reference. Unrelated
native work and identical reads do not mint progress. A failed stage spends no
reference. Broader current rights cannot exceed either original or captured ceilings.

The protected reserve is derived from the two monotone admitted control phases,
not an arbitrary tunable default. The original ticket cannot return from terminal
to pending. Each control phase therefore needs at most one reference, independently
of ordinary lifetime issuance exhaustion. Current control permission/transfer and
protected bounds remain mandatory; the reserve grants no authority. Control and
ordinary references occupy distinct projection domains. A minimal control reference
is not an ordinary bounded-wait selector merely because it names the same operation.

Opening and protected control request metadata without copying the retained outcome;
only ordinary release requests that payload. This keeps rich outcome data and copy
work out of the protected path as well as excluding it from the reply/signature.

## Wait lifecycle and ownership

Section 12.8.3 makes bound explicit policy/grant, authority/scope/continuity/
liveness/retention and actual ledger-binding losses absorbing. Closure clears
delivery references and ends every owned waiter without changing producer state
or authoritative outcomes. Retain waiter/context records and charges until
protected discharge. Match actual context and current policy recipient/epoch
before interpreting a false grant; foreign policy cannot poison a context.
Unknown facts and transient readiness/transfer/resources do not prove context
loss. Forgotten outcome data alone still permits authorized phase/control reads.

Validate owned wait selectors and finite budgets before query-induced closure.
Readiness, admission, clock validity, control and discharge use exact true facts,
not truthy values. An invalid clock or failed poll can end that waiter without
closing an otherwise live context or proving elapsed time/producer completion.
See read-invalidation.md and invocation-invalidation-checks.mjs for six observed
losses, all-waiter/producer/outcome frames and thirteen new mutation controls.

A bounded wait accepts only a real ordinary reference from that exact context,
finite positive step budget, current ordinary control permission, valid declared
clock/service and reserved waiter capacity. It neither admits nor dispatches work.
Poll reads fresh actual ledger state; a permitted change or terminal state produces
one observed result. Otherwise it remains waiting without decrementing time.

Trusted tick service decreases remaining rank and ends at cutoff. Invalid clock
ends unavailable, not completed. Polling alone cannot prove elapsed time or eventual
cutoff. No millisecond default, actual monotonic clock/epoch, suspend behavior,
scheduler fairness, worker preemption or deadline tie priority is supplied by these
logical ticks. Real clock/service evidence remains required under §17.4.

Observer stop/expiry uses exact context and owner control even after content grant
or ordinary resource loss. It ends only the observer. Producer completion may occur
later and remains recorded. Delivery, denial, clock loss, stop and expiry consume
wait eligibility; later regrant cannot revive that waiter. A copied/wrong-context
wait handle cannot poll or decrement it. Unknown internal events refuse without
executing anything. Context close ends its waiters, not their producer.

Ended wait records retain capacity until separate owner-controlled delivery drain
and retention discharge. Context retirement cannot bypass remaining waiter records.
Actual provider/producer/source/history holds are independent; no observer transition
calls the producer stop, run or retirement path. Observed or terminal is not a
safe replay/discharge certificate. Native invocation retirement makes old contexts
unavailable and cannot alias a newly issued ticket.

## Bounds and conditional argument

Reserve two ordinary reply maxima, one protected reply maximum, copied context
maximum, (maxWaiters + 1) wait-record maxima and (maxIssued + 2) reference maxima.
maxIssued is ordinary lifetime issuance; the two extra units are protected phases.
Check safe arithmetic and complete copied context/ceiling/reference size before
opening. Reply reservation includes a full observed outer envelope above the actual
ledger's maximum result; protected space fits the complete minimal product. Wait
records and reference units have their own checked bounds.

These are local logical per-context charges, not global engine admission or physical
heap/transient/RSS proof. The host must actually reserve the context registry, clock,
retention, transfer and service work across owners. A Boolean reservation fixture
does not establish those global facts or actual discharge. Metadata/outcomes are
copied without a separate historical source snapshot. The context retains its
parent invocation-authority handle; that shared authority/root must remain charged
by its existing owner until child references are discharged. This local charge
does not prove whole-engine parent lifetime or physical reclamation, and must not
be used to declare those ancestor charges freed.

Assume the existing serialized ledger, authentic current context/policy/clock/
reservation/retention facts and trusted finite native products. Acquisition fixes
one admitted native ticket and copied ceiling. Every release re-reads that same
ticket; original/captured/current intersection bounds disclosure. Signature issuance
depends only on the admitted permitted projection; the control signature factors
through the two-phase projection, independently of hidden rich facts. Repeated
reads preserve references unless that permitted projection changes.

Induction over observe/poll/tick/stop/close/retire transitions preserves producer
state and execution/publication counts because none calls a mutating producer
operation. Native completion/forgetting changes only the facts actually read at a
later release. Waiting rank is nonnegative and strictly decreases on valid ticks;
ended waits never become waiting again. Separate discharge guards prevent observation
termination from manufacturing freed obligations. This is an authored conditional
argument, not proof-assistant-checked refinement or unconditional time progress.

All operations use bounded known fields, reference/wait maps and the existing four
F06 variants. No callback, external effect, provider poll or interpreter is added.
Finite local calls do not prove physical scheduling or hostile decoder halting.

## Evidence and instrumentation

invocation-observation-checks.mjs uses actual four-variant F13, two-document source
compound and F14 undo/redo admission/publication, not a parallel producer model.
It checks 7,168 context-admission and 21,504 release combinations, 4,096 captured/
current ceiling combinations, thirteen controls and fourteen assertion-rejected
semantic mutants with a passing dynamically loaded baseline. It explores 585 traces
through depth three and 836 observation edges over poll/tick/stop/expire/complete/
revoke/restore/forget. Observation edges preserve actual producer root and usage;
closed observers still permit their independently admitted producer to finish once.
The earlier replay checker retains its own two-epoch/two-generation scope.

Six local scoped-release conditions cover control-projection isolation, independent
control reserve, tick rank and observer nonexecution; SAT controls witness unsafe
rich-driven control progress and a non-decreasing poll loop. Current totals are
33 scoped/223 suite conditions across thirteen manifests. They check the declared
predicates, not automatic code/TLA+/Rust correspondence or host fact truth.

PATH-01/02/04/12 and IF-01/03/08/12 separate producer admission/publication from
context/reference issuance, protected control, wait delivery and retirement.
INS-01/02 count producer actions; INS-03 observation delivery/refusal; INS-04/07
retain external-effect/discharge distinctions; INS-08 context holds; INS-12 diagnostic
loss; INS-13 useful delivery/control progress. Private invocationObservationUsage
reports local issuance partitions, waiter/rank state and declared charge for trusted
supervision, not public counts, scheduler metrics or physical memory measurements.

MELT/RED/golden signals distinguish producer completion, delivered/lost/withheld
observations, wait cutoff and phase latency. USE tracks ordinary/protected reference
capacity, held waits and reply saturation separately. STRIDE challenges handle/
projection confusion, stale contexts, false completion and premature discharge;
LINDDUN challenges hidden retention/progress and cross-context linkage. No raw
invocation IDs, handles, references, outcomes/content or private counters as ordinary
metric labels. Observation never grants execution authority.

Global registry/reservation/retention/policy/clock truth, delegated audiences, all
provider/clipboard/effect gateways, public acquisition/encoding, physical/concurrent/
native/decoder/production and independent assurance remain. No whole F06/F19/D-row
or all-sections goal closes from these selected native observation cases alone.
