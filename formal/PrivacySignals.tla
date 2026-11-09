--------------------------- MODULE PrivacySignals ---------------------------
EXTENDS Naturals, Sequences, FiniteSets
CONSTANTS Bound, LeakTag, LeakCount, SkipEpoch
VARIABLES disc, queued, prepared, drained, captureEpoch, outL, outR, releaseSafe, delivery
D == INSTANCE Disclosure WITH s <- disc, Bound <- Bound, LeakCache <- FALSE
vars == <<disc,queued,prepared,drained,captureEpoch,outL,outR,releaseSafe,delivery>>
diag == <<queued,prepared,drained,captureEpoch,outL,outR,releaseSafe,delivery>>
Init == /\ D!Init /\ queued=FALSE /\ prepared=FALSE /\ drained=FALSE
  /\ captureEpoch=0 /\ outL = <<>> /\ outR = <<>> /\ releaseSafe=TRUE
  /\ delivery=[grant |-> FALSE, disposed |-> FALSE, epoch |-> 0,
       captureEpoch |-> 0, hiddenDifferent |-> FALSE, kind |-> "None"]
\* A deliberately fixed recipient profile. Hidden existence and identity are
\* not permitted operational disclosures; actual clocks/scheduling are absent.
Health(side) == [kind |-> "Health", class |-> "Available",
  count |-> IF LeakCount /\ disc[side].present["Hidden"] THEN 1 ELSE 0,
  correlation |-> IF LeakTag THEN (IF side="left" THEN "private-A" ELSE "private-B") ELSE "scoped"]
Denied == [kind |-> "Unavailable", class |-> "Unknown", count |-> 0, correlation |-> "none"]
Allowed == disc.grant /\ ~disc.disposed /\ captureEpoch=disc.epoch
Guard == disc.grant /\ ~disc.disposed /\ (captureEpoch=disc.epoch \/ SkipEpoch)
Prepare == /\ ~prepared /\ disc.grant /\ ~disc.disposed
  /\ prepared'=TRUE /\ queued'=TRUE /\ captureEpoch'=disc.epoch
  /\ UNCHANGED <<disc,drained,outL,outR,releaseSafe,delivery>>
Drain == /\ queued /\ queued'=FALSE /\ drained'=TRUE
  /\ outL'=Append(outL,IF Guard THEN Health("left") ELSE Denied)
  /\ outR'=Append(outR,IF Guard THEN Health("right") ELSE Denied)
  /\ releaseSafe'=(releaseSafe /\ (Guard => Allowed))
  /\ delivery'=[grant |-> disc.grant, disposed |-> disc.disposed,
       epoch |-> disc.epoch, captureEpoch |-> captureEpoch,
       hiddenDifferent |-> disc.left.present["Hidden"] # disc.right.present["Hidden"],
       kind |-> IF Guard THEN "Health" ELSE "Unavailable"]
  /\ UNCHANGED <<disc,prepared,captureEpoch>>
Core == /\ D!Next /\ UNCHANGED diag
Next == Core \/ Prepare \/ Drain
Spec == Init /\ [][Next]_vars
EqualAuthorizedState == D!EqualPublicState
FunctionalPrivacy == outL=outR /\ D!FunctionalNoninterference
CurrentDiagnosticRelease == releaseSafe
CoreSteps == [][D!Next]_disc
DiagnosticIsolation == [][(queued # queued' \/ drained # drained') => disc'=disc]_vars
DeliveryStable == [][drained => delivery'=delivery]_vars
DeliveryMatchesOutput == drained => delivery.kind=outL[1].kind
AuthorizedHealth == drained /\ delivery.kind="Health" /\ delivery.grant
  /\ ~delivery.disposed /\ delivery.captureEpoch=delivery.epoch
GapDenied == drained /\ delivery.grant /\ ~delivery.disposed
  /\ delivery.captureEpoch=0 /\ delivery.epoch=2 /\ delivery.kind="Unavailable"
DisposedDenied == drained /\ delivery.grant /\ delivery.disposed
  /\ delivery.captureEpoch=delivery.epoch /\ delivery.kind="Unavailable"
HiddenExistenceMasked == AuthorizedHealth /\ delivery.hiddenDifferent /\ outL=outR
NotAuthorizedHealth == ~AuthorizedHealth
NotGapDenied == ~GapDenied
NotDisposedDenied == ~DisposedDenied
NotHiddenExistenceMasked == ~HiddenExistenceMasked

\* Regression paths must reach the old predicate's false-positive state while
\* the delivery-time witness stays false. Index is derived, not a new clock.
Index == disc.tick+(IF prepared THEN 1 ELSE 0)+(IF drained THEN 1 ELSE 0)
RevokeCore == Core /\ disc'.last="Revoke"
RegrantCore == Core /\ disc'.last="Regrant"
DisposeCore == Core /\ disc'.last="Dispose"
HiddenCore == Core /\ disc'.last="Hidden"
LateRegrantScript == CASE Index=0 -> Prepare [] Index=1 -> RevokeCore
  [] Index=2 -> Drain [] Index=3 -> RegrantCore [] OTHER -> FALSE
LateDisposeScript == CASE Index=0 -> Prepare [] Index=1 -> RevokeCore
  [] Index=2 -> Drain [] Index=3 -> DisposeCore [] OTHER -> FALSE
LateHiddenScript == CASE Index=0 -> Prepare [] Index=1 -> Drain
  [] Index=2 -> HiddenCore [] OTHER -> FALSE
DeniedMaskScript == CASE Index=0 -> Prepare [] Index=1 -> RevokeCore
  [] Index=2 -> HiddenCore [] Index=3 -> Drain [] OTHER -> FALSE
LateRegrantSpec == Init /\ [][Next /\ LateRegrantScript]_vars
LateDisposeSpec == Init /\ [][Next /\ LateDisposeScript]_vars
LateHiddenSpec == Init /\ [][Next /\ LateHiddenScript]_vars
DeniedMaskSpec == Init /\ [][Next /\ DeniedMaskScript]_vars
NoFalseGap == ~GapDenied
NoFalseDisposal == ~DisposedDenied
NoFalseMask == ~HiddenExistenceMasked
LateRegrantRejected == Index=4 /\ drained /\ disc.epoch=2
  /\ outL[1].kind="Unavailable" /\ ~GapDenied
LateDisposeRejected == Index=4 /\ drained /\ disc.disposed
  /\ outL[1].kind="Unavailable" /\ ~DisposedDenied
LateHiddenRejected == Index=3 /\ drained
  /\ disc.left.present["Hidden"] # disc.right.present["Hidden"] /\ outL=outR
  /\ ~HiddenExistenceMasked
DeniedMaskRejected == Index=4 /\ drained /\ delivery.hiddenDifferent
  /\ outL[1].kind="Unavailable" /\ outL=outR /\ ~HiddenExistenceMasked
NotLateRegrantRejected == ~LateRegrantRejected
NotLateDisposeRejected == ~LateDisposeRejected
NotLateHiddenRejected == ~LateHiddenRejected
NotDeniedMaskRejected == ~DeniedMaskRejected
=============================================================================
