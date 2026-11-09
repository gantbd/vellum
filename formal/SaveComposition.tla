------------------------ MODULE SaveComposition -------------------------
EXTENDS Integers, Sequences, FiniteSets
CONSTANTS Bound, EarlyRefund, SkipRejectRefund, HistoricalBaseline,
          QuiescentBaseline, ReceiptOrderBaseline, LeakRead, DropOnWait, IgnoreContradiction
VARIABLES admission, gate, acks, failures, baseline, readGranted, readDone, output,
          readSafe, badHeld, progressHeld, accepting, waitEnded, unknownReport, tick
vars == <<admission,gate,acks,failures,baseline,readGranted,readDone,output,
          readSafe,badHeld,progressHeld,accepting,waitEnded,unknownReport,tick>>
A == INSTANCE SaveAdmission WITH s <- admission, Bound <- Bound,
  QueuePolicy <- TRUE, Capacity <- 2, ForgetStopped <- FALSE, Recapture <- FALSE,
  DropQueued <- FALSE, Bypass <- FALSE, Lifecycle <- TRUE, BudgetUnits <- 3,
  CompletionSlots <- 2, IgnoreCost <- FALSE, IgnoreSlots <- FALSE, ReleaseUnknown <- FALSE
G == INSTANCE DispatchGate WITH s <- gate, Bound <- Bound, ProviderSafe <- TRUE,
  IgnoreAuthority <- FALSE, IgnoreBinding <- FALSE, IgnoreEpoch <- FALSE,
  TrustAnything <- FALSE, MatchAttemptOnly <- FALSE, PromoteProgress <- FALSE,
  IgnoreCapability <- FALSE, RequireCurrentReceipt <- FALSE
Attempts == {"A","B"}
Absent == [absent |-> TRUE]
Unavailable == [kind |-> "Unavailable", data |-> Absent]
Rank(t,order) == CHOOSE i \in 1..Len(order): order[i]=t.attempt
Latest(xs,failed,b,order) == LET candidates == {t \in xs: t.binding=b /\ t.attempt \notin failed} IN
  IF candidates={} THEN Absent ELSE CHOOSE t \in candidates:
    \A u \in candidates: Rank(u,order)<=Rank(t,order)
Public(g,b) == [source |-> g.text,
  matchesKnownAck |-> IF b=Absent THEN FALSE ELSE b.payload=g.text]
Init == /\ A!Init /\ G!Init /\ acks={} /\ failures={} /\ baseline=Absent
  /\ readGranted=TRUE /\ readDone=FALSE /\ output=Unavailable /\ readSafe=TRUE
  /\ badHeld=FALSE /\ progressHeld=FALSE /\ accepting=TRUE /\ waitEnded=FALSE /\ unknownReport={} /\ tick=0
Frame == UNCHANGED <<acks,failures,baseline,readGranted,readDone,output,readSafe,badHeld,progressHeld,accepting,waitEnded,unknownReport>>
Offer(a) == /\ accepting /\ A!Submit(a)
  /\ (IF admission'.status[a]="Queued" THEN G!Capture(a) ELSE UNCHANGED gate)
  /\ Frame /\ tick'=tick+1
Eligible(a) == gate.granted /\ gate.capture[a].binding=gate.binding /\ gate.capture[a].epoch=gate.epoch
Launch(a) == /\ admission.status[a]="Queued" /\ admission.queue["R"][1]=a
  /\ G!Dispatch(a)
  /\ (IF Eligible(a) THEN A!Dispatch(a)
     ELSE IF SkipRejectRefund THEN UNCHANGED admission ELSE A!Cancel(a))
  /\ Frame /\ tick'=tick+1
Stop(a) == /\ A!Cancel(a) /\ G!Stop(a) /\ Frame /\ tick'=tick+1
CancelQueued(a) == /\ admission.status[a]="Queued"
  /\ A!Cancel(a) /\ G!EndQueued(a,"Cancelled") /\ Frame /\ tick'=tick+1
Expire(a) == /\ A!Expire(a)
  /\ (IF admission.status[a]="Queued" THEN G!EndQueued(a,"Expired") ELSE G!Stop(a))
  /\ Frame /\ tick'=tick+1
CloseAdmission == /\ accepting /\ accepting'=FALSE
  /\ UNCHANGED <<admission,gate,acks,failures,baseline,readGranted,readDone,output,readSafe,badHeld,progressHeld,waitEnded,unknownReport>>
  /\ tick'=tick+1
EndWait == /\ ~accepting /\ ~waitEnded /\ waitEnded'=TRUE /\ unknownReport'=G!Outstanding
  /\ (IF DropOnWait /\ A!Outstanding # {} THEN
        \E a \in A!Outstanding: A!Resolve(a) ELSE UNCHANGED admission)
  /\ UNCHANGED <<gate,acks,failures,baseline,readGranted,readDone,output,readSafe,badHeld,progressHeld,accepting>>
  /\ tick'=tick+1
Receive(a,v,issuer,kind,stored) == /\ G!Receive(a,v,issuer,kind)
  /\ LET newResolution == a \in gate'.resolved \ gate.resolved
         wrongRefund == EarlyRefund /\ a \in A!Outstanding /\ v=0 /\ issuer="Writer" /\ kind="Progress"
         good == v=0 /\ issuer="Writer"
         saved == good /\ kind="Completed" /\ stored
         xs == IF saved THEN acks \cup {gate.capture[a]} ELSE acks
         fs == IF good /\ kind="Completed" /\ ~stored THEN failures \cup {a} ELSE failures
         selected == Latest(xs,IF IgnoreContradiction THEN {} ELSE fs,gate.binding,admission.order)
         \* Weaken only binding attribution; keep grade/order checks intact so
         \* this control cannot fail first on an unrelated contradiction.
         bugHistory == HistoricalBaseline /\ saved /\ a \notin fs /\ gate.capture[a].binding # gate.binding
         bugQuiescent == QuiescentBaseline /\ good /\ kind="Quiescent"
         bugArrival == ReceiptOrderBaseline /\ saved /\ gate.capture[a].binding=gate.binding
     IN /\ (IF newResolution \/ wrongRefund THEN A!Resolve(a) ELSE UNCHANGED admission)
        /\ acks'=xs
        /\ failures'=fs
        /\ baseline'=(IF bugHistory \/ bugQuiescent \/ bugArrival THEN gate.capture[a] ELSE selected)
        /\ badHeld'=(badHeld \/ (~good /\ a \in A!Outstanding /\ a \in admission'.held))
        /\ progressHeld'=(progressHeld \/ (good /\ kind="Progress" /\ a \in A!Outstanding /\ a \in admission'.held))
  /\ UNCHANGED <<readGranted,readDone,output,readSafe,accepting,waitEnded,unknownReport>> /\ tick'=tick+1
Edit == /\ A!Change /\ G!Edit /\ Frame /\ tick'=tick+1
RevokeWrite == /\ G!Revoke /\ UNCHANGED admission /\ Frame /\ tick'=tick+1
NewEpoch == /\ G!NewEpoch /\ UNCHANGED admission /\ Frame /\ tick'=tick+1
Invalidate == /\ G!InvalidateBinding /\ UNCHANGED admission /\ baseline'=Absent
  /\ UNCHANGED <<acks,failures,readGranted,readDone,output,readSafe,badHeld,progressHeld,accepting,waitEnded,unknownReport>> /\ tick'=tick+1
RevokeRead == /\ readGranted /\ readGranted'=FALSE
  /\ UNCHANGED <<admission,gate,acks,failures,baseline,readDone,output,readSafe,badHeld,progressHeld,accepting,waitEnded,unknownReport>> /\ tick'=tick+1
Read == /\ ~readDone /\ readDone'=TRUE
  /\ output'=(IF readGranted \/ LeakRead THEN [kind |-> "Data",data |-> Public(gate,baseline)] ELSE Unavailable)
  /\ readSafe'=(readSafe /\ (readGranted \/ ~LeakRead))
  /\ UNCHANGED <<admission,gate,acks,failures,baseline,readGranted,badHeld,progressHeld,accepting,waitEnded,unknownReport>> /\ tick'=tick+1
Next == /\ tick<Bound
  /\ ((\E a \in Attempts: Offer(a) \/ Launch(a) \/ Stop(a) \/ CancelQueued(a) \/ Expire(a) \/
        (\E v \in 0..7, issuer \in {"Writer","Stranger"}, kind \in G!Kinds, stored \in BOOLEAN:
          Receive(a,v,issuer,kind,stored)))
      \/ Edit \/ RevokeWrite \/ NewEpoch \/ Invalidate \/ RevokeRead \/ Read \/ CloseAdmission \/ EndWait)
Spec == Init /\ [][Next]_vars
LateScript == CASE tick=0 -> Offer("A") [] tick=1 -> Launch("A")
  [] tick=2 -> Offer("B") [] tick=3 -> Stop("A") [] tick=4 -> Invalidate
  [] tick=5 -> RevokeRead [] tick=6 -> Receive("A",0,"Writer","Completed",TRUE)
  [] tick=7 -> Launch("B") [] tick=8 -> Read [] OTHER -> FALSE
LateSpec == Init /\ [][Next /\ LateScript]_vars
OrderScript == CASE tick=0 -> Offer("A") [] tick=1 -> Launch("A")
  [] tick=2 -> Receive("A",0,"Writer","Completed",TRUE) [] tick=3 -> Edit
  [] tick=4 -> Offer("B") [] tick=5 -> Launch("B")
  [] tick=6 -> Receive("B",0,"Writer","Completed",TRUE)
  [] tick=7 -> Receive("A",0,"Writer","Completed",TRUE) [] tick=8 -> Read [] OTHER -> FALSE
OrderSpec == Init /\ [][Next /\ OrderScript]_vars
ShutdownScript == CASE tick=0 -> Offer("A") [] tick=1 -> Launch("A")
  [] tick=2 -> Offer("B") [] tick=3 -> CloseAdmission [] tick=4 -> CancelQueued("B")
  [] tick=5 -> Expire("A") [] tick=6 -> EndWait
  [] tick=7 -> Receive("A",0,"Writer","Completed",TRUE) [] OTHER -> FALSE
ShutdownSpec == Init /\ [][Next /\ ShutdownScript]_vars
ConflictScript == CASE tick=0 -> Offer("A") [] tick=1 -> Launch("A")
  [] tick=2 -> Receive("A",0,"Writer","Completed",TRUE)
  [] tick=3 -> Receive("A",0,"Writer","Completed",FALSE)
  [] tick=4 -> Receive("A",0,"Writer","Completed",TRUE) [] tick=5 -> Read [] OTHER -> FALSE
ConflictSpec == Init /\ [][Next /\ ConflictScript]_vars
Rejected == {a \in Attempts: gate.status[a] \in {"Denied","StaleBinding","StaleEpoch"}}
RejectedRefund == Rejected \cap admission.held = {}
ResolutionCoherent == admission.resolved=gate.resolved
ControlCoherent == \A a \in Attempts:
  /\ (gate.status[a]="Queued" <=> admission.status[a]="Queued")
  /\ (gate.status[a]="Active" <=> admission.status[a]="Active")
  /\ (gate.status[a]="Stopped" <=> admission.status[a]="Stopped")
  /\ (gate.status[a]="Finished" <=> admission.status[a]="Finished")
  /\ (gate.status[a]="Expired" <=> admission.status[a]="Expired")
  /\ (gate.status[a]="Cancelled" => admission.status[a]="Cancelled")
CaptureCoherent == \A a \in Attempts: gate.capture[a] # Absent => admission.captured[a]=gate.capture[a].payload
KernelSafety == /\ A!TypeOK /\ A!QueueLedger /\ A!ReservationLedger /\ A!CostBound /\ A!CompletionBound
  /\ A!SingleFlight /\ A!FIFO /\ A!ExactDispatch
  /\ G!TypeOK /\ G!AuthorityAtLaunch /\ G!BindingAtLaunch /\ G!EpochAtLaunch
  /\ G!ExactTrustedEvidence /\ G!ValidEvidenceRetained /\ G!SafeResolution /\ G!SingleFlight
AckEvidence == \A t \in acks:
  [tuple |-> t,issuer |-> "Writer",kind |-> "Completed"] \in gate.receipts
BaselineAcknowledged == baseline=Absent \/ baseline \in acks
BaselineUncontested == baseline=Absent \/ baseline.attempt \notin failures
BaselineAttributed == baseline=Absent \/ baseline.binding=gate.binding
LatestKnownAck == baseline=Latest(acks,failures,gate.binding,admission.order)
ReadSafe == readSafe
AdmissionSteps == [][A!Next]_admission
GateSteps == [][G!Next]_gate
EvidenceMonotone == [][acks \subseteq acks' /\ failures \subseteq failures' /\ gate.receipts \subseteq gate'.receipts]_vars
ReleaseProjection == [][(~readDone /\ readDone') =>
  output'=IF readGranted THEN [kind |-> "Data",data |-> Public(gate,baseline)] ELSE Unavailable]_vars
AdmissionClosed == [][(~accepting => ~accepting') /\
  (\A a \in Attempts: admission.status[a]="New" /\ admission'.status[a]="Queued" => accepting)]_vars
WaitIsNotQuiescence == [][(~waitEnded /\ waitEnded') =>
  admission'=admission /\ gate'=gate /\ acks'=acks /\ failures'=failures /\ baseline'=baseline]_vars
ReportStable == [][waitEnded => unknownReport'=unknownReport]_vars
NotSuccessfulSave == acks={}
NotOldCaptureAcknowledged == ~\E t \in acks: t.payload # gate.text
NotProgressHeld == ~progressHeld
NotBadHeld == ~badHeld
NotRejectedRefund == ~(Rejected # {} /\ Rejected \cap admission.held={})
NotQueuedTogether == ~(admission.status["A"]="Queued" /\ admission.status["B"]="Queued")
NotStoppedHeld == gate.stopped \cap A!Outstanding \cap admission.held = {}
NotDeniedLateComposite == ~(readDone /\ output=Unavailable /\ gate.stopped \cap gate.resolved # {}
  /\ admission.held={} /\ gate.receipts # {} /\ baseline=Absent)
NotOrderedDuplicateSafe == ~(readDone /\ baseline # Absent /\ baseline.attempt="B" /\ Cardinality(acks)=2)
NotProjectedKnownAck == ~(readDone /\ output.kind="Data" /\ acks # {} /\ output.data.matchesKnownAck)
NotQueuedExpiredRefund == ~\E a \in Attempts: gate.status[a]="Expired" /\ a \notin admission.held
NotQueuedCancelledRefund == ~\E a \in Attempts: gate.status[a]="Cancelled" /\ a \notin admission.held
NotActiveExpiredHeld == admission.expiredActive \cap A!Outstanding \cap admission.held = {}
NotWaitReportsUnknown == ~(waitEnded /\ unknownReport # {} /\ unknownReport \subseteq admission.held)
NotLateAfterWait == ~(waitEnded /\ unknownReport # {} /\ unknownReport \subseteq gate.resolved /\ admission.held={})
NotConflictRetained == ~(readDone /\ acks # {} /\ failures # {} /\ baseline=Absent)
=============================================================================
