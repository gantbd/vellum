# Native annotation baseline reservation floors

Specification §16.5.5 attaches an actual bounded ledger to the existing invocation
authority and checks its holds at the common final root-installation boundary.
It composes §16.5.4's sizing law with the accepted finite-capacity discipline in
[capacity-admission.md](capacity-admission.md) and the unique actual-authority pool
precedent in [observation-admission.md](observation-admission.md). It does not add
ordinary-save admission, a public token protocol or an external quota service.

## Actual ownership and accounting

The native invocation domain owns one pool attachment. A cloned domain or pool,
another domain's pool, duplicate attachment and reopening after stop refuse.
Opening independently checks exact-true owner control, capacity activation and
preparation facts, finite limits and precharged bookkeeping. Each reservation
requires current ordinary binding-pair validation, the captured annotation binding
generation/provider, current source/set and matching authority/engine context.
An envelope is still sizing data, not authenticated storage or write evidence.

The private pool has fixed finite live/lifetime, complete-record and logical-byte
limits. Its base charge covers the declared serialized bookkeeping envelope;
each live row charges maxRecordBytes plus its future baseline bound. The complete
copied document/binding/envelope record must fit maxRecordBytes. Prospective held
headroom, row/issuance limits and root/counter floors all precede record installation
and issuance. Charges are declared logical envelopes, not allocator or RSS bounds.
Copies returned by trusted supervision cannot mutate the actual profile or ledger.

## Complete root and counter floors

For held target documents D, let b(d) be the maximum of the existing baseline-slot
size and each held envelope's baselineBytes for d. An absent slot contributes
null's size until its field is inserted for the complete calculation. Construct
R0 from the current root by setting those slots to null and publication to its
maximum safe width. Require:

`F = S(R0) + sum(d in D, b(d) - S(null)) <= maxRootBytes`

S is the declared compact JSON UTF-8 byte size. Distinct documents accumulate;
multiple holds for one document share one current root slot maximum. Their
independent retained templates/future evidence remain independently charged in the
pool. Old-binding history insertion and other future companion changes need their
own complete candidate accounting; they are not hidden in this slot maximum.

For N live holds and current publication p, also require:

`N <= MAX_SAFE_INTEGER - p`

One promised baseline publication belongs to each hold, even when several address
the same slot. All arithmetic must remain natural and safe. A missing/duplicate/
disposed target or a changed held annotation binding refuses the floor. Source
and annotation content versions may advance; they do not refresh the held capture.
With no holds, the ordinary current root bound remains in force.

## Native publication and release

Every closed native invocation kind reaches the same candidate check before root
installation, terminal results and clipboard-stage commit. Over-floor candidates
become notApplied with the original root. The actual clipboard path discards and
restages the local refusal before committing its task disposition, preserving the
confirmed external effect separately. No caller-supplied callback or alternative
publication mechanism is introduced. Compatible editing, compound work and
undo/redo continue; this is not a document mutex held while saving.

Pool stop is monotone admission control. It leaves rows, byte charges, counter
promises and final-boundary checks active. Invocation stop/result retirement do
not release them. Explicit release checks the exact private reservation and four
exact-true facts: owner control, obligation discharge, retention release and
delivery drain. Only that row's captured charge is debited; lifetime issuance
persists. These supplied discharge facts do not prove real provider quiescence,
safe disposition, installed baseline ownership or delivery completion.

## Conditional preservation arguments

For finite normalized roots/envelopes, truthful supplied authority/discharge facts
and serialized native transitions:

1. Pool attachment and reservation lookup use actual native/private map identity.
   Copied public shapes cannot select another owner's ledger. Complete copied
   values preserve admitted capture and charges against later caller mutation.
2. Let U be current declared bytes, C the fixed ceiling and Q a new charge.
   Admission checks Q ≤ C−U before U+Q. Base charge and every copied record are
   separately bounded. Release subtracts the captured Q once, while issuance
   remains consumed. Induction preserves U = base + sum(live charges) ≤ C.
3. A successful reserve validates F and N against the actual current root and the
   prospective row set. Any successful changed native publication increments p
   once and rechecks F/N against its complete candidate with the same holds.
   Thus it cannot consume promised byte/counter capacity. Refusal/unchanged paths
   preserve the previous root and ledger; compatible edits may proceed.
4. Release can only remove an envelope from a per-document maximum and decrease N.
   It cannot increase the floor. Stop changes neither F's rows nor their charges.
   The counter floor counts holds, not documents; sharing a slot is not sharing
   a promised future publication unit.
5. Clipboard local commit follows the common floor check and its actual refusal
   restaging protocol. A rejected local continuation cannot leave the task marked
   applied; an already confirmed external clipboard effect remains confirmed.

These are authored conditional arguments, not proof-assistant induction, verified
host facts, whole-engine accounting or independent Cleanroom certification.

## Executable evidence

baseline-reservation-checks.mjs covers 42 actual native annotation/edit/undo/redo
stages across fourteen variants with a held envelope and unchanged ledger charges.
Sixteen positive/refusal cells exercise all eight native job kinds, including
clipboard disposition. Those route-isolation cells use synthetic sizing descriptors,
not claimed authentic storage captures; their unheld positive siblings exclude
blanket rejection. Multi-document controls use actual native output for each role.

All eight open, 32 reserve and sixteen release Boolean gate cases are checked.
Twelve controls cover actual-root growth that fits the ordinary root ceiling but
spends held capacity; same-document maximum versus independent staging charges;
counter exhaustion; private ownership, nonrecycled issuance and stop; local
record/held/live/lifetime bounds; full captured binding/source/set/context and pair;
strict grants and startup bookkeeping; copy safety; cumulative distinct-document
slots; per-hold counter promises; and held target/binding loss. Refusals preserve
the root and ledger. Multi-document capacity is checked against an independently
constructed complete-root oracle, not the ledger's own predicate.

Twenty assertion-detected mutations follow a passing dynamically rewired pool,
native invocation and artifact-producer baseline. They remove the actual final
guard, unique attachment/ownership, grants, row/lifetime/record/byte/counter bounds,
staging charge, captured binding/source/current pair, stop holds and consumed
issuance, or sum same-document slots incorrectly. Current solver totals remain
234 conditions across fourteen manifests; no new SMT theorem is claimed.

## Remaining integration and observation gates

The pool reserves capacity but does not admit an ordinary annotation save or bind
a hold to an actual save ticket. Atomic qualified-baseline installation, hold
transfer, actual effect/discharge and complete non-invocation publication routing
remain required. The existing explicit baseline helper returns raw candidates;
it has not thereby gained an installing authority or permission to consume a hold.
No whole F21/D20/all-sections closure, complete public invocation/events, hostile
decoder, global/origin/physical budget, production/concurrency refinement or
independent assurance is asserted.

INS-01/02/07/08/13 and PATH-05/12 separate offered/admitted reservations, logical
byte occupancy, promised publication units, ordinary root publication/refusal,
stopped admission, acknowledgments and discharge. MELT, RED/USE/golden signals,
STRIDE and LINDDUN cover foreign/forged holds, duplicate pools, undercharge, stale
root estimates, counter exhaustion and occupancy linkage. No installed telemetry
or timing noninterference is proved. Raw paths, content, identities, contexts,
origins and version tokens are not ordinary metric labels.
