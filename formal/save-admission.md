# Save admission and ordering — candidate contracts

PS-SA compares two admission policies under single-flight coordination. Subsequent
user residual review selected bounded FIFO of exact admitted captures for v1
(requirements §5.10), with explicit capacity/expiry outcomes and no implicit
coalescing. Passing either policy's checks did not select it. Numeric budgets,
deadlines, service and shutdown remain open. This is not an executor/storage adapter.

## Decision comparison

| Option | Pros | Cons / tradeoffs | Recommendation |
|---|---|---|---|
| Reject busy before admission | Smallest retained state; explicit backpressure; no queued snapshot lifetime | Caller owns resubmission; blind polling can amplify load; repeated competition can starve callers | Modeled comparison, not the selected ordinary v1 policy |
| Bounded per-resource FIFO of exact admitted snapshots | Deterministic local dispatch order; preserves each accepted request's meaning; supports agent bursts without replacing intent | Retains potentially stale snapshots; head-of-line blocking; needs byte/cost bounds, expiry and host service assumptions | Selected direction by user review; budget/lifecycle specification still required |
| Coalesce undispatched requests | Reduces traffic and retained intermediate versions | Changes accepted-request semantics; needs explicit Superseded outcomes and provenance; cannot rewrite an already-dispatched attempt | Defer, not an implicit optimization |

FIFO is admission order within one resolved resource, not global author time,
revision order, fairness across resources or guaranteed host completion. It is
not a global FIFO for editing, control traffic or completions (§7.5). Canonical
resource identity must prevent alias/workspace/profile splits. ConcurrentSaves
still supplies the separate conditional-write and evidence-order comparison;
no automatic composition/refinement theorem connects these models yet.

## Candidate transition contract

- Submit either admits an exact immutable snapshot, rejects Busy, or rejects
  Full. Rejected requests have not dispatched. The abstract admission/capture
  transition is atomic; payload materialization/encoding and reservation costs
  require the existing preparation contracts, not free work before admission.
- Dispatch removes only the oldest eligible queued request for its resource,
  and only when that resource has no unresolved dispatch. Dispatch uses the
  admitted capture, not whatever happens to be live then. If a product API
  instead means “save whatever is current when serviced,” that is a different
  contract and needs an explicit capture/admission boundary.
- Cancelling queued work removes it with an explicit Cancelled outcome.
  Cancelling active work stops caller control, retains the resource obligation,
  and does not grant another dispatch. Stopped control stays stopped.
- Resolve means accepted exact-attempt evidence of a safe disposition: confirmed
  completion or confirmed no-future-effect. It is **not** a deadline, abandonment,
  “worker disappeared,” success assertion, or proof of crash durability. A late
  resolution can release coordination without changing Stopped to Finished.
- Unrelated resources have independent dispatch eligibility. A checked path
  demonstrates that possibility while another resource remains stopped/unknown;
  it does not establish temporal fairness or aggregate executor isolation.

## Machine-checkable scope

`SaveAdmission.tla` uses four one-shot request identities, three targeting R and
one S, two symbolic live versions, six processing transitions and a queue bound
of two per resource. Those are exploration parameters, not product limits.
The state vocabulary and enabled transitions are finite. The horizon is a safety
exploration cutoff, not a termination proof for waiting on external effects.

The original fourteen cases comprise two exhaustive safety profiles (busy and
bounded FIFO), four property-specific weakened controls and eight witnesses:

- busy rejection, capacity rejection, queued cancellation;
- dispatch of a capture older than the then-live version;
- stopped/unknown obligation blocks another same-resource queued request;
- late safe disposition permits a subsequent same-resource dispatch;
- distinct-resource dispatch while the first remains stopped/unknown;
- a previously waiting request progresses after its FIFO predecessor resolves.

Controls deliberately release a stopped slot, recapture at dispatch, silently
drop a queued entry, or bypass the FIFO head. Each must fail its named safety
property, not merely generate an arbitrary error. Capture immutability, monotone
resolution evidence and stable stopped labels are checked action properties.

## Deadline and reservation extension — candidate, not policy adoption

Thirteen additional cases bring PS-SA to **27 cases**: five safety profiles,
seven weakened controls and fifteen witnesses in total. The extension adds three
six-step lifecycle profiles, three controls and seven witnesses. It enables
deadline observations and two independent aggregate admission limits: symbolic
work-cost units and completion reservations. A request costs one unit except A,
which costs two. Test budgets 2/3/8 and slot limits 1/2/3 expose different binding
constraints; none is a product quota, byte count or timing promise.

| Point of deadline observation | Candidate control result | Effect / reservation treatment |
|---|---|---|
| Queued, never dispatched | Expired, terminal | Remove from queue; release its work and completion reservations; it cannot later dispatch |
| Dispatched, unresolved | Stopped, terminal caller control | Preserve unresolved obligation and reservations; not confirmed failure or confirmed no-write |
| Stopped, later exact safe disposition | Still Stopped | Release outstanding-work reservations; retain accepted resolution evidence under its own storage contract |

The abstract deadline event competes atomically with dispatch: expiry first
prevents dispatch; dispatch first leaves a physical-effect obligation. There is
no modeled clock, restart epoch, timer accuracy or service-delay bound. The
model does not prove that actual wall-clock expiry is observed before dispatch.
An implementation needs a host-owned deadline/final-dispatch check contract;
remote timestamps or refreshed idle timers cannot silently enlarge a granted
budget. Exact expiry rules and shutdown behavior remain specification decisions.

The reference ledger is `held = queued ∪ unresolvedDispatched`. Admission checks
both aggregate cost and completion slots before accepting a capture. Dispatch
and caller stop leave reservations unchanged. Queued cancellation/expiry removes
only that undispatched request. An accepted safe disposition removes the resolved
request. The extension reserves one completion slot even for queued requests;
this is a deliberately conservative candidate, not a required queue design.

| Reservation strategy | Benefit | Tradeoff / recommendation |
|---|---|---|
| Reserve completion capacity at admission | Accepted work already has a completion-handling reservation; simple conservation argument | Idle queued work occupies future capacity; strongest reference profile checked here |
| Reserve at dispatch with bounded staged acquisition | Better utilization; fewer idle completion reservations | Queue admission cannot promise dispatch without an additional service/acquisition argument; only an explicitly staged alternative, not checked here |

Inductive design argument, **not a proof-assistant theorem**: initially both
totals are zero. A guarded admission adds a known positive upper-bound cost and
one slot only if both new totals fit. Dispatch/stop preserve the totals. Removing
a held member cannot increase either total. Thus these transitions preserve the
two bounds for arbitrary fixed nonnegative capacities, given truthful costs and
the exact ledger. TLC checks the declared finite instances plus non-vacuity
paths, not that general theorem or the truth of future resource estimates.

Additional witnesses cover queued expiry, retained active-expiry reservations,
late resolution after expiry, later admission after queued expiry, cost pressure,
slot pressure and an expired-but-unresolved write still exhausting the budget.
Controls bypass each admission bound or prematurely refund an unresolved write;
they must violate CostBound, CompletionBound or ReservationLedger respectively.
Expired queued control remains terminal, and its attempt never dispatches.

Work reservations are **not** all retained memory. Captures remain as immutable
proof-history symbols in this finite model; accepted resolutions remain monotone
evidence. This does not establish real snapshot reclamation, receipt-store bounds
over arbitrarily many attempts, crash recovery or retention transfer. Production
accounting must separately bound/charge live captures, authoritative receipts,
history and any reconciliation state before reclaiming work reservations.
Reservation coverage also does not prove that protected completion consumers
receive executor service, or that a bounded slot handles arbitrary receipt sizes.

## Instrumentation and threat feedback

At IF-04/INS-04 distinguish admission delay, queue wait, host service, receipt
delay and unresolved age. Count Busy/Full/Cancelled separately from confirmed
failure or unknown effect. Track retained snapshot bytes, queue saturation and
protected completion reservations, not only request count. INS-07 accounts for
capture/encoding costs; IF-10/INS-08 accounts for resource equivalence and fences.
Distinguish queued expiry from post-dispatch stop/unknown, and work-reservation
release from retained-evidence storage. Measure each budget dimension separately;
an occupancy count alone cannot establish available bytes or service capacity.

STRIDE review: forged completion must not release a slot; aliases must not evade
serialization/quotas; floods must not consume protected completion capacity;
queued work must not bypass current dispatch authorization. Queue positions,
cross-author existence, raw resource IDs, payloads and tokens are not default
public projections or metric labels. Model request IDs are proof symbols, not a
proposed telemetry schema. Instrumentation loss must not become evidence loss.

## Remaining decisions / proof gates

No numeric budget, deadline, fairness guarantee or provider contract is invented:

1. Select busy versus pending admission and the public request/capture boundary.
2. For pending work, select an expiry and reservation profile; specify actual
   aggregate bytes/cost limits, host clock/epoch semantics, shutdown/retention and
   terminal outcomes. The new symbolic checks do not supply those contracts.
3. Refine the [dispatch/evidence gates](dispatch-gates.md) into the full queued
   operation's authority/dependency/binding checks. Their new bounded evidence
   does not freeze permission or authorize stale-precondition retry.
4. Discharge actual provenance, safe-disposition/provider and alias-equivalence
   assumptions. The receiver model distinguishes these gates but does not prove
   supplier truth; unresolved effects may occupy a slot indefinitely. Timeout
   may end caller control but cannot establish physical quiescence.
5. Generalize the [bounded admission/gate composition](save-composition.md) to
   complete lifecycle, execution, persistence and concurrent evidence relations;
   prove service assumptions separately from FIFO safety.
6. Decide supersession separately if needed. No silent latest-wins, auto-refresh,
   force overwrite or background retry is introduced by this comparison.
