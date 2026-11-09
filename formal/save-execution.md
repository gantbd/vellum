# Save execution budget and lifecycle correspondence

PS-SE invokes actual `SaveComposition` actions and the existing `Execution!Step`
operator. It covers one request A; the composite's B remains unused. This is a
restricted reference wrapper, not a new product scheduler or complete refinement.

## Charged control versus effect lifetime

Offer/Prepare/Launch map to Admit/Prepare/Dispatch. Immediately before launch,
current authority and binding/provider-context mismatches become declared Revoke/
DependencyChanged events followed by Dispatch. Every event in that finite list is
processed through the actual Step operator. If fuel runs out before dispatch,
the queued entry is cancelled and refunded instead of launching a write.

Tick, deadline, cancellation and an unexpected internal protocol event use the
corresponding kernel event. On terminal control, cleanup calls the composite's
actual queued cancellation or active stop in the same wrapper transition. This
removes undispatched reservations but retains any unresolved physical obligation.
Kernel Stopped/Fault supplies the higher-level cause; the queue/gate's lower-level
removal/stop label must not replace that cause at the public boundary.

Exact authenticated Completed results map to ReceiptOK or ReceiptFailed using
the explicit stored-result field. Quiescent maps to a distinct neutral
ReceiptQuiescent fact, neither OK nor Failed. Progress/cancel-intent/bad input consumes Tick
while active and sufficiently budgeted. At the final remaining fuel unit,
nonterminal input processing is deferred in favor of a control-stop transition;
accepted evidence can still be reconciled afterward. Terminal caller control
never restarts when a late result adds evidence.

## Checked correspondence and scope

- Recorded event lists reconstruct the exact next kernel state through Step.
  Source edits and unrelated environment/control events stutter this operation's
  kernel; they are not falsely charged as its processing steps.
- Each processed active-control transition strictly decreases natural-number
  rank; rank never refills. Multi-event launch refresh consumes each Step charge.
- Kernel types, stable terminal phase, monotone late facts, dispatched identity
  and absence of orphaned active/queued control are checked with the composite's
  actual reservation, launch and baseline invariants.
- Each composite state transition satisfies actual SaveComposition Next or
  stutters its complete state tuple. Internal Prepare/Tick work may stutter the
  composite while still consuming caller-control fuel.

Thirteen cases comprise three safety runs, three controls and seven witnesses. Exhaustive
five-transition profiles use fuel 6 and 2. A fuel-4 five-transition script admits,
prepares, dispatches, exhausts waiting control and accepts late success. Witnesses
also show ordinary success, pre-dispatch exhaustion/refund, stopped unknown effect,
late success without restart, protocol fault/refund and rejected current authority.
Controls omit a Tick charge, omit required terminal cleanup, or misclassify
quiescence as failure. The named correspondence/orphan/grade checks must fail.

These are harness bounds, not production capacity or latency promises. The local
rank argument excludes an infinite sequence of charged active steps for one
initial fuel grant. It does not guarantee that a host schedules those steps,
preempts native code, terminates a physical write or bounds all future input.
No fairness/temporal theorem is inherited by this finite-horizon wrapper. The
standalone execution theorem still states its own weak-fair service assumption.

## Vocabulary and assurance residuals

The original kernel lacked a quiescence-only fact. The run-down closes that
avoidable mapping gap using the already-defined disposition: ReceiptQuiescent
adds a neutral fact, can end waiting, and creates neither success nor failure.
A witness requires released coordination with no acknowledgment/negative grade.
This extends the internal finite vocabulary, not a public command/schema choice.
The separate fixed-universe Evidence lattice example remains scoped to its own
declared fact set; it is not claimed to enumerate every kernel receipt grade.

Completed outcome truth and provenance remain supplier assumptions here. The
abstract [Persistence bridge](storage-bridge.md) removes the free-result oracle
for its fixed one-source-attempt profile, not every dynamic provider. Late reconciliation has
its own processing/retention/service obligations; rank zero of a stopped caller
is not a free, unbounded callback/receipt-handling lane. Real input, byte, native
work, descendant/restart and cost accounting still need implementation contracts.

Observe charged work, terminal cause, pre/post-dispatch exhaustion and unresolved
reservation age at INS-01/INS-04/INS-13. Keep caller stop, physical quiescence and
successful persistence separate. Do not expose raw source, host tokens or attempt
identity as public metric dimensions. No instrumentation-overhead or complete
disclosure/clean-room assurance claim follows from these checks.
