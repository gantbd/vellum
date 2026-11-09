# Resource profile validity and accounting laws

Requirements §§7.5–7.6 and specification §17 require independent resource
ceilings, inherited work attribution, protected service and reservations that
outlive cancellation where obligations remain. The following algebra makes those
requirements precise without choosing numeric limits, clocks, classes, scheduling
algorithms, a public schema or a new transfer API. D06/D22/D35 remain open for
concrete profiles, actual clock/context behavior and evidence. D06's subsequently
adopted admission/capacity-activation baseline and separate scoped evidence are in
[capacity-admission.md](capacity-admission.md); these original equations and
solver conditions remain unchanged and do not choose numerical defaults.

## Units and accounting domains

A resource coordinate identifies a unit, accounting domain, applicable scope and
meaning. Bytes, slots, work units and time cannot be added or substituted merely
because their representations are integers. A scheduling weight is not a byte,
concurrency, fan-out or deadline bound. Relevant coordinates and scope bindings
come from the validated profile, not an untrusted origin/priority label.

Distinguish reusable occupancy/reservations from consumed work. Occupancy can
decrease after real discharge; consumed work does not become unconsumed when a
request completes. Children/retries consume or reserve from the same inherited
envelope. Copying a parent's remaining budget into each child is not conservation.
Deadline milestones, clock epochs and unavailable observations retain §17.4's
separate contract; a local time observation is not a reusable capacity credit.

Shared allocations need truthful ownership/retention accounting. An alias does
not create quota or require a duplicate whole-allocation charge merely for naming
the same storage; its own metadata and retained references can still have costs.
Allocation/reclamation/refinement evidence must relate actual retained resources
to charges. A symbolic cost counter alone cannot bound process RSS, native
allocation, provider CPU or any unaccounted resource.

## Admission and lifecycle equations

For one arbitrary resource coordinate, let C be a nonnegative mathematical
capacity, L the live accounted charge and D an additional nonnegative reservation.
The pointwise premise `0 ≤ L ≤ C` and `D ≤ C − L` yields `0 ≤ L + D ≤ C`.
Every applicable host/authority/origin ceiling must hold; passing only one is
insufficient. Repeat the pointwise condition for each required coordinate.
Charge increments can differ under declared scope/accounting models; the dual-
ceiling lemma covers the case where one increment is chargeable to both scopes.
Theorems allowing C=0 do not select whether any concrete supported profile may
configure zero; protected-service and actual workload obligations still apply.

A physical bound additionally needs truthful premises: actual retained use A
satisfies `A ≤ L`, and the new actual demand is no greater than D. Only then does
the accounted admission law bound actual post-admission use. Unknown demand
requires a conservative bound or bounded staged acquisition before growth under
§17.2, not substitution of zero or post-hoc measurement after exceeding a bound.

For a discharge R, `0 ≤ R ≤ L` gives a nonnegative new charge `L − R`. This is
physically meaningful only if the still-retained obligation is bounded by that
remaining charge. Cancellation, expiry and caller completion do not prove that
premise. Releasing an outstanding-work slot does not release separately retained
history/evidence storage. Refunds apply only to the eligible reusable coordinate,
not to already consumed processing work.

If a supported same-unit ownership transfer moves D from source to recipient,
require source credit, recipient capacity and current transfer authority. Its
accounting equations are `Lsource' = Lsource − D` and
`Lrecipient' = Lrecipient + D`; aggregate charge is unchanged. Common ancestor
obligations persist. Cross-domain conversions or transfers need their own
truthful conversion/lifetime contract. No transfer/retry may manufacture a grant.

For finite work quota C, preserve `spent + remaining = C`. A permitted step of
cost D with `0 ≤ D ≤ remaining` sets `spent' = spent + D` and
`remaining' = remaining − D`. The step preserves the equation and cannot increase
remaining quota. This does not show every real step is charged, or prove progress
when zero-cost continuation, missing host service or stuck native work is allowed.

## Finite product lattice boundary

For a fixed finite coordinate set and fixed finite integer capacities, the
integer-valued occupancy domain is the product of integer intervals `[0,Ci]`,
ordered componentwise.
Pointwise minimum/maximum provide meet/join, with zero bottom and capacity-vector
top. The checked coordinate laws establish greatest lower/least upper bounds,
interval closure and absorption. This component argument yields a finite bounded
lattice for that declared occupancy
domain, not for the whole editor, authority, clocks, histories or executions.

Admission/discharge can move in opposite directions; lattice closure does not
make execution monotone or terminating. Changing capacities or coordinate/scope
sets requires a separately validated context. No activation may assert a resource
bound while hiding retained obligations. Adopted D06 rejects incompatible proposed
capacity activation; separately planned drain/migration and concrete context
behavior remain D35 refinements, not consequences of this lattice theorem.

## Executable evidence and residuals

`resource-profiles.smt2` supplies six local positive conditions, each checking SAT
premises then UNSAT of the negated conclusion: dual-ceiling admission, conditional
actual-use refinement, discharge, same-unit transfer, finite-quota step and the
pointwise lattice laws. Four SAT controls exhibit missing resource dimensions,
premature release, copied child quota and sender debit without recipient charge.
The harness matches the complete ordered `resourceCases` manifest and rejects
solver errors/unknown/missing/extra output. Z3 4.15.4 and the encoding are trusted;
there is no independently replayed proof certificate or actual allocator proof.

Existing PS-AC/PS-DR/PS-UP evidence remains separate: imported admission/count/cost
bookkeeping, representative retained cells and conditional cooperative backlog
drain. None proves the actual profile's units, demand upper bounds, complete
charging, retention supplier, transfer authority or open-load scheduler. Required
conformance includes tiny-request fan-out, shared retention, failed cancellation,
late completion, profile shrink, missing dimensions and on/off/full/fault diagnostics.

Keep existing IF-01/03/04/05, PATH-12 and INS-01/02/04/05/12 placements. Resource
measurements distinguish reserved versus observed use, occupancy versus consumed
work, logical operations versus attempts, and unknown versus zero. USE/RED and
golden signals observe the declared coordinate and boundary; they cannot supply
missing admission authority or physical-resource evidence. STRIDE/LINDDUN apply
to quota/origin spoofing, lost/tampered charges, adversarial fan-out, accounting
attribution and private scope disclosure. No identity-valued metric labels or new
telemetry schemas are introduced by these mathematical coordinate names.
