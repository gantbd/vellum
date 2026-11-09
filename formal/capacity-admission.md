# Finite profiles and capacity admission

Section 17.2.3 separately composes this actual admission/activation helper with
native observation charges, a unique owned authority pool, protected commitments
and verified local context retirement before reusable refund. See
[observation-admission.md](observation-admission.md). Physical/all-owner/origin
coverage and authentic host policy/clock/discharge remain gates; the original
finite-profile checker's narrower claims/counts below are unchanged.

D06 adopts requirements §7.5 and specification §17.2.2's complete finite profile,
fail-fast new atomic admission and incompatible capacity-activation refusal.
Measured reference defaults may complement host-supplied profiles; no universal
numbers, clock/service guarantee, scheduler or quota service is selected. Existing
save FIFO and other explicitly bounded queues retain their separate contracts.

## Conditional accounting and activation argument

For each comparable resource coordinate and applicable scope, validate the current
charge U within capacity C and bound additional demand D by `C−U`. Then `U+D≤C`.
Checking each independent coordinate and scope preserves their conjunction; passing
a host ceiling does not waive an originating-scope ceiling. Unknown demand must
obtain conservative evidence or bounded staged acquisition before growth.
Actual physical bounds additionally require complete truthful accounting and demand
upper bounds, as in [resource-profiles.md](resource-profiles.md). Neither a work
estimate nor logical bytes establish provider CPU or process RSS.

One conservative protected-slot fixture partitions C into an ordinary envelope
`C−F` and protected completion envelope F. With ordinary use O and held completion
promises H satisfying `O≤C−F` and `H≤F`, aggregate `O+H≤C`. Check both ordinary
and completion headroom before adding a promise. This is a nonborrowing reference
partition, not a selected scheduler, service quantum, actual protected minimum or
proof that all real completion payloads are charged.

Capacity activation validates the proposed context against current charges and
commitments rather than rewriting them to fit. In the fixture, the proposed
ordinary partition must still contain O and its completion partition H; all other
coordinates retain their charges. Refusal retains the active profile and ledger.
Compatible activation preserves consumed work; a quota's remaining portion is
`newQuota−spent`, not a fresh whole quota copied to every child. Induct over admitted
charge and compatible context transitions to preserve these declared bounds.

Soft/adaptive targets guide optional budget planning but do not occur in the hard
admission predicate. Changing one cannot create authority or increase hard limits.
Optional plans truthfully distinguish full versus reduced requested byte budgets;
they neither perform analysis nor certify semantic completeness. Actual growth
must still reserve under current hard limits, not rely on an earlier plan.
Cancellation intent is not a discharge witness; retained snapshots/history/effect
evidence remain charged under their own lifecycle contracts.

These are authored conditional arguments, not proof-assistant replay, actual
allocator/scheduler verification or a physical-resource/termination theorem.
Changing capacities or scope domains does not create a single finite lattice for
editor execution; the existing fixed-coordinate occupancy lattice remains scoped.

## Finite reference checks

`capacity-admission.mjs` has two fixed scopes (host and origin) and four declared
coordinates: accounted bytes, outstanding ordinary slots, consumed work steps and
consumed child count. Each scope also has a protected completion partition and a
soft byte target. It enumerates 81 capacity records per scope (bytes/work/fanout
0–2, slots 1–3), giving 6,561 profile combinations. Three ledger patterns cover
empty, full ordinary/consumption and held completion/retention states.

Eight demand vectors yield 157,464 admission cases. A prospective-addition oracle
is compared with headroom subtraction; all applicable scope/coordinate and protected
partition bounds are checked. Six candidate profile transformations give 118,098
activation cases: unchanged, grown, byte-shrunk, slot-shrunk, protected-floor-grown
and soft-target-reduced. They require the exact compatibility/refusal disposition
and preserve the ledger, not merely reject every change. Three optional requested
byte amounts give 59,049 planning cases with bound and budget-coverage checks.
All operations frame their input profiles and ledgers.

Eighteen direct sensitivity controls cover missing/infinite dimensions, wrong
units/kinds, invalid time descriptor, missing protected capacity, soft target over
hard ceiling, host-only checking, reserve borrowing, unknown demand, inherited
quota duplication, cancellation refunds, incompatible shrink, stranded ordinary
commitments, unauthorized growth, near-integer-overflow demand, reduced budget
mislabeling and adaptive admission expansion. They are selected fault examples,
not general mutation adequacy or independent Cleanroom expectations.

The coordinate names, units, integer ranges, zero capacities, one-slot protected
floor and record shapes are private mathematical fixtures, not actual supported
profile minima, API fields or numerical defaults. Same increments are charged to
both scopes here; real accounting may have different justified charge relations.
The positive finite `deadlineTicks` descriptor checks presence/type only: the
fixture does not advance clocks, vary time contracts during activation, compare
epochs or prove deadline validity. D22/D35 must supply those semantics and retained
contexts; a capacity check is not a deadline or whole-context activation proof.

The ledger represents declared charges, not a job queue, actual text/history,
allocator, inherited identity implementation or complete instruction-cost map.
Its cancellation example only preserves charges; actual stopped-execution gates,
safe discharge, effect quiescence, staging rollback and publication are not modeled.
Predicate evaluation/cloning overhead is not physically charged by this fixture.
Ingress/preparation needs its already valid bounded parent context and actual
accounting refinement; proposed limits cannot bootstrap their own authority.
Configured hard-capacity growth also requires host/profile truth beyond the
fixture's trusted authorization parameter. External host resource loss is not a
configuration transaction this reference can reject away.

## Local symbolic checks

`capacity-laws.smt2` adds six positive conditions, each SAT premises then UNSAT
negated conclusion: protected partition, admission headroom, compatible activation,
soft-target nonauthority, refusal frame and preserved consumed quota on activation.
Four SAT controls exhibit protected reserve borrowing, shrink below held charges,
an estimate fitting despite another coordinate exceeding capacity and copied
full quota after prior consumption. Integer arithmetic is unbounded in these
local conditions; concrete overflow/representation/allocator proofs remain open.
The exact ordered `capacityCases` manifest uses trusted Z3 4.15.4 without independent
certificate replay. Existing ten `resourceCases` and PS-AC/DR/UP evidence remain
separate; neither their conjunction nor a capacity vector alone proves end-to-end
charging, protected service, scheduler fairness or native preemption.

## Observation security and residuals

IF-01/03/04/05 and PATH-12 retain admission, publication and host-work boundaries;
INS-01/02/04/05/12 retain resource and protected diagnostic/effect observations.
USE/RED/golden profiles distinguish reservations, observed use, consumed work,
pending age, rejection/degradation, actual service and useful outcomes. Measure
the constrained coordinate and scope without identity-valued metric labels or
private annotation/workspace topology. Telemetry cannot grant headroom or prove
an unavailable physical quantity is zero.

STRIDE/LINDDUN cover quota/origin spoofing, missing/tampered charges, fan-out,
profile-shrink confiscation, false completeness, reservation-pressure leakage and
diagnostic feedback used as authority. Preserve unsaved authored state, retained
history promises and unresolved effects rather than shedding them for optional
work. Optional budget planning is not permission to weaken an atomic operation.

Actual profile schemas/units/default values, workload/hardware/corpus evidence,
complete allocation/work/queue/retention refinement, supported protected minima,
time/service contracts and planned drain/migration remain D06/D08/D14/D22/D35 work.
The core decision does not demand users configure every field: reference hosts
may eventually supply justified defaults after measurement. Until that evidence,
literal fixture capacities are not release settings or benchmark results.

Mechanism precedent includes [Neovim feature budgets](https://neovim.io/doc/user/options/),
[CodeMirror bounded parsing/availability](https://codemirror.net/docs/ref/#language.ensureSyntaxTree)
and [Emacs's approximate GC thresholds](https://www.gnu.org/software/emacs/manual/html_node/elisp/Garbage-Collection.html).
These motivate feature-specific controls and honest limits, not importing their
values or claiming equivalent implementation behavior.
