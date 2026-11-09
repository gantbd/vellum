------------------------- MODULE DerivedRetention -------------------------
EXTENDS Naturals, FiniteSets, TLC
CONSTANTS Bound, Capacity, ForgetOnCancel, IgnoreContext, IgnoreRevision,
          IgnoreGrant, TransferOwner, IgnoreCapacity
VARIABLE s

\* Cell IDs denote immutable source values. The owner root and worker may
\* share one allocation; scopes are not separately chargeable aliases.
\* Only source-cell references are charged. capturedValue is a ghost value,
\* not a modeled result buffer; history, result copies and native memory are
\* outside this ledger and need separate accounting/refinement evidence.
Value(c) == IF c=0 THEN "old source" ELSE "new source"
Live(t) == {t.root} \cup t.held
Advance(t) == [t EXCEPT !.tick=s.tick+1]
Init == s=[root |-> 0, revision |-> 0, context |-> 0, owner |-> "local",
  allocated |-> {0}, held |-> {}, worker |-> "Idle", cancelled |-> FALSE,
  capturedRevision |-> 0, capturedContext |-> 0, capturedValue |-> Value(0),
  result |-> FALSE, visible |-> FALSE, grant |-> TRUE,
  acceptedRevision |-> 0, acceptedContext |-> 0, tick |-> 0]
Start == /\ s.worker="Idle" /\ s.grant
  /\ s'=Advance([s EXCEPT !.worker="Running", !.held={s.root},
       !.capturedRevision=s.revision, !.capturedContext=s.context,
       !.capturedValue=Value(s.root)])
Cancel == /\ s.worker="Running" /\ ~s.cancelled
  /\ s'=Advance([s EXCEPT !.cancelled=TRUE,
       !.allocated=IF ForgetOnCancel THEN {s.root} ELSE @])
Edit == /\ s.revision=0
  /\ (Cardinality({1} \cup s.held)<=Capacity \/ IgnoreCapacity)
  /\ s'=Advance([s EXCEPT !.root=1, !.revision=1,
       !.allocated={1} \cup s.held, !.visible=FALSE])
Invalidate == /\ s.context=0
  /\ s'=Advance([s EXCEPT !.context=1, !.visible=FALSE])
Revoke == /\ s.grant
  /\ s'=Advance([s EXCEPT !.grant=FALSE, !.visible=FALSE])
Complete == /\ s.worker="Running"
  /\ s'=Advance([s EXCEPT !.worker="Done", !.result=TRUE,
       !.held={}, !.allocated={s.root},
       !.owner=IF TransferOwner THEN "provider" ELSE @])
Accept == /\ s.result /\ ~s.visible /\ ~s.cancelled
  /\ (s.capturedRevision=s.revision \/ IgnoreRevision)
  /\ (s.capturedContext=s.context \/ IgnoreContext)
  /\ (s.grant \/ IgnoreGrant)
  /\ s'=Advance([s EXCEPT !.visible=TRUE,
       !.acceptedRevision=s.capturedRevision, !.acceptedContext=s.capturedContext])
Next == /\ s.tick<Bound
        /\ (Start \/ Cancel \/ Edit \/ Invalidate \/ Revoke \/ Complete \/ Accept)
Spec == Init /\ [][Next]_s
TypeOK == /\ s.tick \in 0..Bound /\ s.root \in {0,1}
  /\ s.worker \in {"Idle","Running","Done"}
  /\ s.allocated \subseteq {0,1} /\ s.held \subseteq {0,1}
NoDangling == Live(s) \subseteq s.allocated
ExactAccounting == s.allocated=Live(s)
Budget == Cardinality(s.allocated)<=Capacity
OwnerStable == s.owner="local"
SnapshotStable == s.worker # "Idle" =>
  s.capturedValue=Value(s.capturedRevision)
CurrentEvidence == s.visible =>
  (s.acceptedRevision=s.revision /\ s.acceptedContext=s.context /\ s.grant)
CancelRetains == [][(s'.cancelled /\ ~s.cancelled) =>
  (s'.held=s.held /\ Live(s) \subseteq s'.allocated)]_s
WorkerCannotPublish == [][(s.worker="Running" /\ s'.worker="Done") =>
  (s'.root=s.root /\ s'.revision=s.revision /\ s'.owner=s.owner)]_s
LiveOldAfterEdit == s.root=1 /\ s.held={0} /\ s.allocated={0,1}
CancelledStillHeld == LiveOldAfterEdit /\ s.cancelled
CompletionReclaims == s.root=1 /\ s.worker="Done" /\ s.allocated={1}
FreshAccepted == s.visible
StaleContextHeld == s.result /\ s.capturedContext # s.context /\ ~s.visible
StaleRevisionHeld == s.result /\ s.capturedRevision # s.revision /\ ~s.visible
CapacityBlocksEdit == Capacity=1 /\ s.root=0 /\ s.worker="Running" /\ s.held={0}
NotLiveOldAfterEdit == ~LiveOldAfterEdit
NotCancelledStillHeld == ~CancelledStillHeld
NotCompletionReclaims == ~CompletionReclaims
NotFreshAccepted == ~FreshAccepted
NotStaleContextHeld == ~StaleContextHeld
NotStaleRevisionHeld == ~StaleRevisionHeld
NotCapacityBlocksEdit == ~CapacityBlocksEdit
=============================================================================
