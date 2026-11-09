--------------------------- MODULE StorageBridge ------------------------
EXTENDS Integers, Sequences, FiniteSets
CONSTANTS Bound, HostCapable, ForgeStored, FakeEarly, IgnoreConflict
VARIABLES admission, gate, acks, failures, baseline, readGranted, readDone, output,
  readSafe, badHeld, progressHeld, accepting, waitEnded, unknownReport, tick,
  store, clock
C == INSTANCE SaveComposition WITH Bound <- Bound, EarlyRefund <- FALSE,
  SkipRejectRefund <- FALSE, HistoricalBaseline <- FALSE, QuiescentBaseline <- FALSE,
  ReceiptOrderBaseline <- FALSE, LeakRead <- FALSE, DropOnWait <- FALSE,
  IgnoreContradiction <- IgnoreConflict
P == INSTANCE Persistence WITH s <- store, Bound <- Bound,
  ConditionalCapable <- HostCapable, IgnoreBinding <- FALSE,
  IgnoreCapability <- FALSE, LooseReceiptIdentity <- FALSE
coreVars == <<admission,gate,acks,failures,baseline,readGranted,readDone,output,
  readSafe,badHeld,progressHeld,accepting,waitEnded,unknownReport,tick>>
vars == <<coreVars,store,clock>>
StutterC == UNCHANGED coreVars
Init == /\ C!Init /\ P!Init /\ clock=0
Offer == /\ C!Offer("A") /\ P!Capture("Source") /\ clock'=clock+1
HostEligible == HostCapable /\ store.captured["Source"].binding=store.binding
  /\ store.captured["Source"].token=store.externalToken
Launch == /\ P!Dispatch("Source")
  /\ (IF HostEligible THEN C!Launch("A") ELSE C!CancelQueued("A"))
  /\ clock'=clock+1
HostWrite == /\ P!HostWrite("Source") /\ StutterC /\ clock'=clock+1
Forward == /\ P!Receive("Source")
  /\ C!Receive("A",0,"Writer","Completed",ForgeStored \/ store.host["Source"]="OK")
  /\ clock'=clock+1
Conflict == /\ P!Conflict("Source") /\ C!Receive("A",0,"Writer","Completed",FALSE)
  /\ clock'=clock+1
BadReceipt == /\ P!BadReceipt("Source") /\ C!Receive("A",1,"Writer","Completed",FALSE)
  /\ clock'=clock+1
Early == /\ FakeEarly /\ store.host["Source"]="None"
  /\ C!Receive("A",0,"Writer","Completed",TRUE) /\ UNCHANGED store /\ clock'=clock+1
Stop == /\ C!Stop("A") /\ P!Stop("Source") /\ clock'=clock+1
Edit == /\ C!Edit /\ P!EditSource /\ clock'=clock+1
ExternalChange == /\ P!ExternalChange /\ StutterC /\ clock'=clock+1
Rebind == /\ P!Rebind /\ C!Invalidate /\ clock'=clock+1
Environment == /\ (C!Read \/ C!RevokeRead \/ C!CloseAdmission \/ C!EndWait)
  /\ UNCHANGED store /\ clock'=clock+1
Next == /\ clock<Bound
  /\ (Offer \/ Launch \/ HostWrite \/ Forward \/ Conflict \/ BadReceipt \/ Early \/ Stop \/
       Edit \/ ExternalChange \/ Rebind \/ Environment)
Spec == Init /\ [][Next]_vars
ConflictScript == CASE clock=0 -> Offer [] clock=1 -> Launch [] clock=2 -> HostWrite
  [] clock=3 -> Forward [] clock=4 -> Conflict [] clock=5 -> Forward [] OTHER -> FALSE
ConflictSpec == Init /\ [][Next /\ ConflictScript]_vars
LiveAgreement == gate.text=store.text /\ gate.binding=store.binding
CaptureAgreement == (gate.capture["A"]=C!Absent <=> store.captured["Source"]=P!None) /\
  (gate.capture["A"] # C!Absent =>
    /\ gate.capture["A"].payload=store.captured["Source"].payload[1]
    /\ gate.capture["A"].profile=store.captured["Source"].profile
    /\ gate.capture["A"].binding=store.captured["Source"].binding
    /\ gate.capture["A"].incarnation=store.captured["Source"].incarnation)
AckBackedByWrite == \A t \in acks:
  /\ store.written["Source"] # P!None
  /\ store.written["Source"].snapshot=store.captured["Source"]
  /\ t.payload=store.written["Source"].snapshot.payload[1]
  /\ t.binding=store.written["Source"].snapshot.binding
  /\ t.profile=store.written["Source"].snapshot.profile
OutcomeAgreement == /\ (acks # {} <=> "OK" \in store.facts["Source"])
  /\ ("A" \in failures <=> "Failed" \in store.facts["Source"])
BaselineAgreement == IF store.baseline["Source"]=P!None THEN baseline=C!Absent ELSE
  /\ baseline # C!Absent
  /\ baseline.payload=store.baseline["Source"].snapshot.payload[1]
  /\ baseline.binding=store.baseline["Source"].snapshot.binding
  /\ baseline.profile=store.baseline["Source"].snapshot.profile
DispatchAgreement == ("A" \in gate.dispatched <=> "Source" \in store.dispatched)
ResolutionBackedByQuiet == "A" \in gate.resolved => "Source" \in store.quiet
CompositeSafety == /\ C!KernelSafety /\ C!ResolutionCoherent /\ C!ControlCoherent
  /\ C!CaptureCoherent /\ C!BaselineAcknowledged /\ C!BaselineUncontested /\ C!BaselineAttributed /\ C!LatestKnownAck
PersistenceSafety == /\ P!BaselineAttribution /\ P!NoUndispatchedEvidence
  /\ P!StrongAdmission /\ P!ConditionalWrite /\ P!MalformedReceiptsRejected
CompositeSteps == [][C!Next]_coreVars
PersistenceSteps == [][P!Next]_store
EvidenceMonotone == [][acks \subseteq acks' /\ failures \subseteq failures' /\
  store.facts["Source"] \subseteq store'.facts["Source"]]_vars
SafeRebind == P!SafeRebind
NotSaved == acks={}
NotUnsupportedRefused == ~(~HostCapable /\ store.phase["Source"]="Rejected" /\ admission.held={} /\ gate.dispatched={})
NotRaceFailed == ~(store.host["Source"]="Failed" /\ "A" \in failures /\ baseline=C!Absent)
NotPrelaunchRefused == ~(store.phase["Source"]="Rejected" /\ gate.dispatched={} /\ admission.held={})
NotLateStopped == ~(acks # {} /\ gate.status["A"]="Stopped" /\ admission.held={})
NotHistoricalBinding == ~(acks # {} /\ gate.binding=1 /\ baseline=C!Absent)
NotOldSnapshot == ~(acks # {} /\ store.captured["Source"].revision<store.textRev /\ baseline # C!Absent)
NotMalformedHeld == ~(store.rejectedReceipt /\ "A" \in admission.held /\ gate.resolved={})
NotConflictRetained == ~(acks # {} /\ "A" \in failures /\ baseline=C!Absent)
=============================================================================
