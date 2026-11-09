# Save composition: admission → launch → evidence → projection

PS-SC is a restricted composition of the **actual** SaveAdmission and DispatchGate
transition relations. It closes part of the gap identified in
[dispatch-gates.md](dispatch-gates.md), rather than asserting that similarly named
states automatically compose. Subsequent user review selects single-flight/
bounded FIFO (§5.10); this model does not itself select policy or prove a concrete
implementation's service, capacities, deadlines or shutdown behavior.

## Imported relations and translation

`SaveComposition.tla` instantiates both modules with separate state variables.
Init invokes both imported Init predicates. Every outer transition must satisfy
each imported Next relation or stutter that module's state; `AdmissionSteps` and
`GateSteps` check that correspondence. Selected imported invariants and explicit
cross-module relations are also checked in the same reachable states. No fairness
or complete public-observation refinement is inherited from this construction.

| Composite event | Admission transition | Gate transition | Shared obligation |
|---|---|---|---|
| Offer accepted | Submit | Capture | Same source representation; queued cost/completion reservations acquired |
| Launch permitted | Dispatch | Dispatch | FIFO head, current gates and unchanged exact capture; reservations retained |
| Launch rejected | Cancel queued entry | Dispatch to Denied/StaleBinding/StaleEpoch | Refund undispatched reservations; retain the precise gate rejection |
| Stop active caller | Cancel active entry | Stop | Both controls stop; unresolved effect and reservations remain |
| Incoming progress/bad input/duplicate | Stutter unless a new safe disposition is established | Receive | No early/double refund; retain accepted observations |
| First accepted safe disposition | Resolve | Receive | Resolve the same attempt and refund its outstanding-work reservations |
| Source edit | Change | Edit | Both models observe the same live representation |
| Grant/context invalidation | Stutter | Revoke/InvalidateBinding/NewEpoch | No permission to activate a new binding or erase historical evidence |

The lower-level admission Cancelled state represents queue removal after gate
rejection; it is not the public outcome. The gate retains Denied/StaleBinding/
StaleEpoch. This explicit translation prevents a ledger implementation detail
from relabeling an authorization failure as a user's cancellation.

## Safe resolution is not a successful save

The gate's Completed class does not encode success versus failure. The composition
therefore adds a separate typed **success-acknowledgment refinement field**,
`stored`, to incoming observations. Only exact Writer-provenance Completed
observations with that field true enter the success-acknowledgment set. Quiescent
can resolve coordination but cannot establish successful persistence. An authentic
Completed observation with `stored = FALSE` also resolves without creating a save
acknowledgment. This is an explicit supplier-result assumption, not a success fact
deduced from a terminal control label.

Negative Completed grades now accumulate in a separate monotone `failures` set.
An attempt with both positive and negative grades is ineligible as a successful
baseline, even after duplicate success. Neither observation is erased. Three
additional conflict-script cases (safety, ignored-conflict control and witness)
bring PS-SC to **29 cases**: five safety runs, eight controls and sixteen witnesses.
The [storage bridge](storage-bridge.md) derives these grades from actual abstract
host outcomes in its restricted profile.

The fixed reference profile assumes the provider's safe-disposition guarantee.
Authenticity, truthful result grading and no-future-effect semantics still require
supplier evidence. There is no physical storage variable or cryptographic proof
in this model. An implementation cannot trust a caller-provided boolean to satisfy
the refinement field. Real schemas, guarantee grades and actual writes must be
connected through the host/persistence contract.

## Historical acknowledgment versus current baseline

Acknowledgments remain monotone historical facts. Current baseline eligibility
requires the current binding; incarnation, canonical resource and serialization
profile are fixed in this profile. Observed binding invalidation clears the
current baseline but does not erase history. An old-binding success can refund its
resolved attempt without becoming the current binding's baseline.

For eligible acknowledgments, select the greatest **local dispatch position**.
This is sound only for the modeled single-flight profile: a later dispatch cannot
start until the earlier attempt has a safe disposition. Duplicate or delayed
observations for the earlier attempt do not change that ordering. Provider-context
epoch fences launch here; it is not silently treated as a storage version or
an additional baseline identity dimension. Other provider profiles need their own
identity and ordering contracts, including the PS-CS token/order analysis.

The source may advance after capture. A valid acknowledgment remains attributed
to that exact captured representation, not relabeled as the newest source. The
public `matchesKnownAck` flag compares live source with the eligible known
acknowledgment; it does **not** assert current external contents, absence of foreign
writes, crash durability or that no newer unresolved write exists.

## Current release boundary

The composition adds a small reference Read transition, separate from both
imported modules. It checks current read authority and releases only live source
plus `matchesKnownAck`, or a constant Unavailable envelope. It exposes no raw
receipt, attempt/binding/provider identifier or historical acknowledgment set.
Read authority is independent of write authority and of internal evidence retention.

The checked late path queues a second save, stops the first caller, invalidates
the binding, revokes read access, accepts the first save's late success, rejects
the second save's stale launch, and returns Unavailable. Both reservation refunds
are justified independently; historical evidence survives; no old-binding baseline
or unauthorized source projection is produced.

This is not an imported Disclosure/Persistence refinement, paired-world
noninterference proof, stream/regrant-gap model, annotation projection or native UI
proof. Those models and surfaces still need explicit relations to this composite
state. Previously authorized released data is not retroactively recalled by
later revocation; new release is what the current gate controls.

## Scope and non-vacuity

The original nineteen cases comprise three safety runs, six weakened controls
and ten witnesses:

- Five-transition exhaustive profile: two one-shot requests on one canonical
  resource, FIFO capacity two, aggregate work budget three symbolic units and
  completion capacity two; only A/B of the imported admission request universe
  participate. These are exploration parameters, not production capacities.
- Two nine-transition scripts are subsets of the same Next relation. One follows
  late historical evidence through reservation refund and denied release. The
  other acknowledges A, edits source, acknowledges B, re-delivers A, then reads;
  the selected baseline must remain B.
- Witnesses cover successful acknowledgments, acknowledged older captures,
  progress and bad input retaining reservations, rejected-launch refund, two
  queued requests, stopped unresolved reservations, projected known-ack equality
  and the two scripted end-to-end outcomes.
- Controls refund on progress, omit rejected-launch refund, apply historical
  binding evidence as current baseline, treat quiescence as successful storage,
  select baseline by receipt arrival, or bypass read authority. Named bridge
  properties must fail; syntax/type/tool errors do not pass as counterexamples.

The lifecycle extension added seven cases, bringing that stage to **26 cases**:
four safety runs, seven controls and fifteen witnesses. Actual queued Cancel/
Expire calls now compose with DispatchGate EndQueued; active expiry composes with
Stop. CloseAdmission refuses new offers but leaves explicit drain/cancel/expiry
choices available. No particular shutdown order or numerical deadline is selected.

EndWait records the unresolved set at that cut and ends only the modeled waiting
phase. It frames effect state, reservations, acknowledgments and baseline. The
report is an immutable historical snapshot, not a claim about the current unknown
set after later evidence. A control that refunds an unresolved attempt on EndWait
must fail ResolutionCoherent. An eight-transition script closes admission, removes
a queued request, expires an active caller, ends waiting and accepts late success.
Witnesses separately cover queued cancellation/expiry refunds, active-expiry
retention, reported unknown work and post-wait reconciliation.

Caller regrant, multiple resources, real byte accounting, clock/service guarantees
and complete shutdown policy remain outside this wrapper. A wait-cut event is not
a wall-clock deadline proof. Horizon bounds establish finite-prefix checks, not
service or physical termination. [The execution wrapper](save-execution.md) adds
actual Step/rank correspondence including a distinct neutral quiescence fact.

## Instrumentation and assurance feedback

Correlate IF-04/INS-04 admission → local launch → observation classification →
reservation reconciliation without exposing raw identity or source data in metric
dimensions. Record rejected-launch refunds separately from dispatched safe refunds;
progress, failed completion and successful-save acknowledgment remain distinct.
Duplicate receipt traffic must not look like a second logical completion or
second refund. Protect accepted evidence independently of optional telemetry.

IF-10/INS-08 distinguishes observed binding invalidation, baseline ineligibility
and actual authorized binding activation. IF-13/INS-07 distinguishes success
grading/profile attribution from mere completion. IF-08/INS-03 controls each
reference read projection under current authority. Instrumentation collection and
release remain bounded and authorized; these checks do not establish its overhead
or flood resistance.

STRIDE feedback: exact authenticated input can still be mistranslated downstream.
Check refund, outcome grading, baseline selection and disclosure in their shared
state, not only inside isolated suppliers. A rejected launch can strand capacity;
a progress observation can incorrectly release it; an authentic duplicate can
regress a baseline if arrival order is mistaken for causal order. Preserved
history must neither grant new authority nor become an unfiltered public view.

## Remaining proof gates

1. Generalize the [abstract storage-outcome bridge](storage-bridge.md) to actual
   provider receipts/writes; discharge provenance, safe-disposition and identity.
2. Generalize the checked cancellation/expiry/admission-close/wait-cut subset to
   complete resource/shutdown policy, safe binding activation, retained evidence
   storage and actual cost/service envelopes.
3. Generalize actual Persistence and [recipient Disclosure](recipient-bridge.md)
   correspondence to dynamic attempts, ConcurrentSaves profiles, annotations and
   every release surface. The fixed bridges do not prove the whole system.
4. Generalize the checked [execution wrapper](save-execution.md), including
   full outcome/schema mapping; prove service/liveness assumptions separately
   from rank safety and finite horizons.
5. Obtain independent contract/oracle/tool and eventual implementation assurance.
   This is bounded reference composition, not whole-system correctness or
   source-isolated clean-room certification.
