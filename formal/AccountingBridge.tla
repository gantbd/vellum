-------------------------- MODULE AccountingBridge --------------------------
EXTENDS Integers, FiniteSets
CONSTANTS Bound, SkipAdmission, DoubleRefund, PrematureRefund, WrongCost
VARIABLES admission, offered, admitted, rejected, discharged, charged, refunded
A == INSTANCE SaveAdmission WITH s <- admission, Bound <- Bound,
  QueuePolicy <- TRUE, Capacity <- 2, ForgetStopped <- FALSE,
  Recapture <- FALSE, DropQueued <- FALSE, Bypass <- FALSE,
  Lifecycle <- TRUE, BudgetUnits <- 3, CompletionSlots <- 2,
  IgnoreCost <- FALSE, IgnoreSlots <- FALSE, ReleaseUnknown <- FALSE
vars == <<admission,offered,admitted,rejected,discharged,charged,refunded>>
Init == /\ A!Init /\ offered=0 /\ admitted=0 /\ rejected=0
  /\ discharged=0 /\ charged=0 /\ refunded=0
Submit(a) == /\ A!Submit(a)
  /\ offered'=offered+1
  /\ admitted'=admitted+(IF admission'.status[a]="Queued" /\ ~SkipAdmission THEN 1 ELSE 0)
  /\ rejected'=rejected+(IF admission'.status[a] # "Queued" THEN 1 ELSE 0)
  /\ charged'=charged+(IF admission'.status[a]="Queued" THEN A!Cost(a) ELSE 0)
  /\ UNCHANGED <<discharged,refunded>>
ReleaseStep(a,action) == /\ action
  /\ LET real == a \in admission.held /\ a \notin admission'.held
         early == PrematureRefund /\ admission'.last="Cancel" /\ a \in A!Outstanding
         delta == IF real THEN (IF DoubleRefund THEN 2 ELSE 1) ELSE IF early THEN 1 ELSE 0
     IN /\ discharged'=discharged+delta
        /\ refunded'=refunded+delta*(IF WrongCost THEN 1 ELSE A!Cost(a))
  /\ UNCHANGED <<offered,admitted,rejected,charged>>
Other == /\ ((\E a \in A!Requests: A!Dispatch(a)) \/ A!Change)
  /\ UNCHANGED <<offered,admitted,rejected,discharged,charged,refunded>>
Next == /\ admission.tick<Bound
  /\ ((\E a \in A!Requests: Submit(a) \/ ReleaseStep(a,A!Cancel(a))
       \/ ReleaseStep(a,A!Expire(a)) \/ ReleaseStep(a,A!Resolve(a))) \/ Other)
Spec == Init /\ [][Next]_vars
KernelSafety == A!TypeOK /\ A!ReservationLedger /\ A!CostBound /\ A!CompletionBound
LogicalPartition == offered=admitted+rejected
CountConservation == admitted=Cardinality(admission.held)+discharged
CostConservation == charged=A!Used(admission.held)+refunded
Nonnegative == /\ discharged<=admitted /\ refunded<=charged
  /\ {offered,admitted,rejected,discharged,charged,refunded} \subseteq Nat
AdmissionSteps == [][A!Next]_admission
StoppedHeld == admission.stopped \cap A!Outstanding \cap admission.held # {}
RefundedOnce == discharged>0 /\ admitted=Cardinality(admission.held)+discharged
RejectedOffer == rejected>0
LateDischarge == admission.stopped \cap admission.resolved # {} /\ discharged>0
NotStoppedHeld == ~StoppedHeld
NotRefundedOnce == ~RefundedOnce
NotRejectedOffer == ~RejectedOffer
NotLateDischarge == ~LateDischarge
=============================================================================
