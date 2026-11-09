-------------------------- MODULE SaveExecution -------------------------
EXTENDS Integers, Sequences, FiniteSets
CONSTANTS Bound, Fuel, UnchargedTick, ForgetCleanup, QuiescenceAsFailure
VARIABLES admission, gate, acks, failures, baseline, readGranted, readDone, output,
  readSafe, badHeld, progressHeld, accepting, waitEnded, unknownReport, tick,
  control, events, clock
C == INSTANCE SaveComposition WITH Bound <- Bound, EarlyRefund <- FALSE,
  SkipRejectRefund <- FALSE, HistoricalBaseline <- FALSE, QuiescentBaseline <- FALSE,
  ReceiptOrderBaseline <- FALSE, LeakRead <- FALSE, DropOnWait <- FALSE, IgnoreContradiction <- FALSE
K == INSTANCE Execution WITH Docs <- {"R"}, MaxFuel <- Fuel,
  RecheckContext <- TRUE, HonorFuel <- TRUE, state <- control
coreVars == <<admission,gate,acks,failures,baseline,readGranted,readDone,output,
  readSafe,badHeld,progressHeld,accepting,waitEnded,unknownReport,tick>>
vars == <<coreVars,control,events,clock>>
InitialControl == [phase |-> "New",verb |-> "Save",fuel |-> Fuel,
  granted |-> TRUE,current |-> TRUE,applied |-> FALSE,dispatched |-> FALSE,
  released |-> FALSE,pulse |-> FALSE,contents |-> K!Before,evidence |-> {}]
RECURSIVE Run(_,_)
Run(k,es) == IF Len(es)=0 THEN k ELSE Run(K!Step(k,es[1]),Tail(es))
Init == /\ C!Init /\ control=InitialControl /\ events = <<>> /\ clock=0
Work(es) == /\ events'=es /\ control'=Run(control,es) /\ clock'=clock+1
Idle == /\ events' = <<>> /\ control'=control /\ clock'=clock+1
StutterC == UNCHANGED coreVars
AbortC == IF ForgetCleanup THEN StutterC
  ELSE IF gate.status["A"]="Queued" THEN C!CancelQueued("A")
  ELSE IF gate.status["A"]="Active" THEN C!Stop("A") ELSE StutterC
Offer == /\ control.phase="New" /\ C!Offer("A") /\ Work(<<"Admit">>)
Prepare == /\ control.phase="Admitted"
  /\ (IF K!Halted(K!Step(control,"Prepare")) THEN AbortC ELSE StutterC)
  /\ Work(<<"Prepare">>)
LaunchEvents == (IF gate.granted THEN <<>> ELSE <<"Revoke">>) \o
  (IF gate.capture["A"].binding=gate.binding /\ gate.capture["A"].epoch=gate.epoch
    THEN <<>> ELSE <<"DependencyChanged">>) \o <<"Dispatch">>
Launch == /\ control.phase="Prepared"
  /\ LET k == Run(control,LaunchEvents) IN
    (IF k.dispatched \/ k.phase="Rejected" THEN C!Launch("A") ELSE AbortC)
  /\ Work(LaunchEvents)
Tick == /\ ~K!Halted(control) /\ control.phase # "New"
  /\ (IF K!Halted(K!Step(control,"Tick")) THEN AbortC ELSE StutterC)
  /\ events' = <<"Tick">>
  /\ control'=(IF UnchargedTick THEN control ELSE K!Step(control,"Tick")) /\ clock'=clock+1
Surprise == /\ ~K!Halted(control) /\ control.phase # "New"
  /\ AbortC /\ Work(<<"Surprise">>)
Deadline == /\ gate.status["A"] \in {"Queued","Active"} /\ ~K!Halted(control)
  /\ C!Expire("A") /\ Work(<<"Deadline">>)
Cancel == /\ gate.status["A"] \in {"Queued","Active"} /\ ~K!Halted(control)
  /\ (IF gate.status["A"]="Queued" THEN C!CancelQueued("A") ELSE C!Stop("A"))
  /\ Work(<<"Cancel">>)
Receive(v,issuer,kind,stored) ==
  /\ LET settled == v=0 /\ issuer="Writer" /\ kind \in {"Completed","Quiescent"}
         es == IF settled THEN <<IF kind="Quiescent" /\ ~QuiescenceAsFailure THEN "ReceiptQuiescent"
                                ELSE IF kind="Quiescent" THEN "ReceiptFailed"
                                ELSE IF stored THEN "ReceiptOK" ELSE "ReceiptFailed">>
               ELSE IF K!Halted(control) THEN <<>> ELSE <<"Tick">>
     IN /\ (K!Halted(control) \/ settled \/ control.fuel>1)
        /\ C!Receive("A",v,issuer,kind,stored) /\ Work(es)
Environment == /\ (C!Edit \/ C!RevokeWrite \/ C!NewEpoch \/ C!Invalidate \/
                    C!RevokeRead \/ C!Read \/ C!CloseAdmission \/ C!EndWait)
  /\ Idle
Next == /\ clock<Bound
  /\ (Offer \/ Prepare \/ Launch \/ Tick \/ Surprise \/ Deadline \/ Cancel \/ Environment \/
       (\E v \in 0..7, issuer \in {"Writer","Stranger"},
         kind \in {"Completed","Quiescent","Progress","CancelIntent"}, stored \in BOOLEAN:
           Receive(v,issuer,kind,stored)))
Spec == Init /\ [][Next]_vars
LateScript == CASE clock=0 -> Offer [] clock=1 -> Prepare [] clock=2 -> Launch
  [] clock=3 -> Tick [] clock=4 -> Receive(0,"Writer","Completed",TRUE) [] OTHER -> FALSE
LateSpec == Init /\ [][Next /\ LateScript]_vars
ControlType == control \in K!StateSpace
ReceiptGrades == /\ ("Failed" \in control.evidence => "A" \in failures)
  /\ ("OK" \in control.evidence => acks # {})
NoOrphanControl == /\ (gate.status["A"]="Queued" => control.phase \in {"Admitted","Prepared"})
  /\ (gate.status["A"]="Active" => control.phase="Waiting")
  /\ (control.dispatched <=> "A" \in gate.dispatched)
CompositeSafety == /\ C!KernelSafety /\ C!ResolutionCoherent /\ C!ControlCoherent
  /\ C!RejectedRefund /\ C!BaselineAcknowledged /\ C!BaselineUncontested /\ C!BaselineAttributed /\ C!LatestKnownAck
KernelCorrespondence == [][control'=Run(control,events')]_vars
RankDecreases == [][(Len(events')>0 /\ ~K!Halted(control)) => K!Rank(control')<K!Rank(control)]_vars
NoRefill == [][K!Rank(control')<=K!Rank(control)]_vars
TerminalStable == [][K!Halted(control) => control'.phase=control.phase]_vars
CompositeSteps == [][C!Next]_coreVars
LateFactsMonotone == [][control.evidence \subseteq control'.evidence]_vars
NotSucceeded == ~(control.phase="Done" /\ "OK" \in control.evidence /\ acks # {})
NotBudgetBeforeDispatch == ~(control.phase="Stopped" /\ ~control.dispatched /\ admission.held={})
NotStoppedUnknown == ~(control.phase="Stopped" /\ "A" \in C!G!Outstanding /\ "A" \in admission.held)
NotLateBudgetEvidence == ~(control.phase="Stopped" /\ "OK" \in control.evidence /\ admission.held={})
NotFaultRefund == ~(control.phase="Fault" /\ admission.held={})
NotDeniedLaunch == ~(control.phase="Rejected" /\ gate.status["A"]="Denied")
NotQuiescentWithoutSave == ~("Quiescent" \in control.evidence /\ acks={} /\ failures={} /\ admission.held={})
=============================================================================
