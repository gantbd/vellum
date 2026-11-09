-------------------------- MODULE RecipientBridge -----------------------
EXTENDS Integers, Sequences, FiniteSets
CONSTANTS Bound, LeakHidden, LeakEvidence, BypassGate
VARIABLES admission, gate, acks, failures, baseline, readGranted, readDone, output,
  readSafe, badHeld, progressHeld, accepting, waitEnded, unknownReport, tick,
  store, clock, disc, outL, outR, releaseSafe, worldClock
S == INSTANCE StorageBridge WITH Bound <- Bound, HostCapable <- TRUE,
  ForgeStored <- FALSE, FakeEarly <- FALSE, IgnoreConflict <- FALSE
D == INSTANCE Disclosure WITH s <- disc, Bound <- Bound, LeakCache <- LeakHidden
saveVars == <<admission,gate,acks,failures,baseline,readGranted,readDone,output,
  readSafe,badHeld,progressHeld,accepting,waitEnded,unknownReport,tick,store,clock>>
vars == <<saveVars,disc,outL,outR,releaseSafe,worldClock>>
StutterSave == UNCHANGED saveVars
Init == /\ S!Init /\ D!Init /\ outL = <<>> /\ outR = <<>>
  /\ releaseSafe=TRUE /\ worldClock=0
Envelope(e,w) == IF e.kind="Data" \/ BypassGate THEN
  [kind |-> "Data",data |-> [source |-> gate.text,
    annotations |-> IF e.kind="Data" THEN e.payload ELSE D!Projection(w,disc.source,FALSE)],
   internalCount |-> IF LeakEvidence THEN Cardinality(acks) ELSE 0]
  ELSE [kind |-> "Unavailable",data |-> S!C!Absent,internalCount |-> 0]
Emit(allowed) ==
  /\ outL'=Append(outL,Envelope(disc'.outputL[Len(disc'.outputL)],disc.left))
  /\ outR'=Append(outR,Envelope(disc'.outputR[Len(disc'.outputR)],disc.right))
  /\ releaseSafe'=(releaseSafe /\
       (IF disc'.outputL[Len(disc'.outputL)].kind="Data" \/ BypassGate THEN allowed ELSE TRUE))
  /\ worldClock'=worldClock+1
FrameOutput == UNCHANGED <<outL,outR,releaseSafe>>
StorageEvent == /\ (S!Offer \/ S!Launch \/ S!HostWrite \/ S!Forward \/ S!Conflict \/
                    S!BadReceipt \/ S!Stop \/ S!ExternalChange \/ S!Rebind)
  /\ UNCHANGED disc /\ FrameOutput /\ worldClock'=worldClock+1
\* Select only admission close/wait from the actual storage Environment wrapper.
CoreClose == /\ S!Environment /\ readDone'=readDone /\ readGranted'=readGranted
  /\ UNCHANGED disc /\ FrameOutput /\ worldClock'=worldClock+1
SourceDeletion == /\ S!Edit /\ D!DeleteTarget /\ FrameOutput /\ worldClock'=worldClock+1
Hidden(side) == /\ (D!HiddenBody(side) \/ D!HiddenExistence(side))
  /\ StutterSave /\ FrameOutput /\ worldClock'=worldClock+1
Visible == /\ D!VisibleBody /\ StutterSave /\ FrameOutput /\ worldClock'=worldClock+1
Request == /\ D!Request /\ StutterSave /\ FrameOutput /\ worldClock'=worldClock+1
Query == /\ D!Query /\ StutterSave /\ Emit(disc.grant /\ ~disc.disposed)
Release == /\ D!Release /\ StutterSave /\ Emit(D!ReleaseAllowed)
Policy == /\ (D!Revoke \/ D!Regrant \/ D!Dispose)
  /\ StutterSave /\ FrameOutput /\ worldClock'=worldClock+1
Next == /\ worldClock<Bound
  /\ (StorageEvent \/ CoreClose \/ SourceDeletion \/ Visible \/ Request \/ Query \/ Release \/ Policy \/
       (\E side \in {"left","right"}: Hidden(side)))
Spec == Init /\ [][Next]_vars
GapScript == CASE worldClock=0 -> Request [] worldClock=1 -> D!Revoke /\ StutterSave /\ FrameOutput /\ worldClock'=worldClock+1
  [] worldClock=2 -> D!Regrant /\ StutterSave /\ FrameOutput /\ worldClock'=worldClock+1
  [] worldClock=3 -> Release [] OTHER -> FALSE
GapSpec == Init /\ [][Next /\ GapScript]_vars
SavedReadScript == CASE worldClock=0 -> S!Offer /\ UNCHANGED disc /\ FrameOutput /\ worldClock'=worldClock+1
  [] worldClock=1 -> S!Launch /\ UNCHANGED disc /\ FrameOutput /\ worldClock'=worldClock+1
  [] worldClock=2 -> S!HostWrite /\ UNCHANGED disc /\ FrameOutput /\ worldClock'=worldClock+1
  [] worldClock=3 -> S!Forward /\ UNCHANGED disc /\ FrameOutput /\ worldClock'=worldClock+1
  [] worldClock=4 -> Query [] OTHER -> FALSE
SavedReadSpec == Init /\ [][Next /\ SavedReadScript]_vars
SourceAgreement == disc.source=gate.text
FunctionalNoninterference == outL=outR
NoPrivateEvidence == \A o \in {outL[i]: i \in 1..Len(outL)} \cup {outR[i]: i \in 1..Len(outR)}:
  o.internalCount=0
ReleaseSafe == releaseSafe
StorageSafety == /\ S!LiveAgreement /\ S!CaptureAgreement /\ S!AckBackedByWrite
  /\ S!OutcomeAgreement /\ S!BaselineAgreement /\ S!DispatchAgreement
  /\ S!ResolutionBackedByQuiet /\ S!CompositeSafety /\ S!PersistenceSafety
StorageSteps == [][S!Next]_saveVars
DisclosureSteps == [][D!Next]_disc
OutputMonotone == [][Len(outL')>=Len(outL) /\ Len(outR')>=Len(outR)]_vars
NotAuthorizedRead == ~(Len(outL)>0 /\ outL[Len(outL)].kind="Data")
NotSavedEvidenceErased == ~(acks # {} /\ Len(outL)>0 /\ outL[Len(outL)].kind="Data" /\ outL[Len(outL)].internalCount=0)
NotQueuedGapDenied == ~(disc.epoch=2 /\ Len(outL)>0 /\ outL[Len(outL)].kind="Unavailable")
NotHiddenUnobservable == ~(disc.left.body["Hidden"] # disc.right.body["Hidden"] /\ Len(outL)>0 /\ outL=outR)
NotSourceDeletionProjected == ~(disc.source=1 /\ Len(outL)>0 /\ outL[Len(outL)].kind="Data" /\ outL[Len(outL)].data.source=1)
NotRevokedReadDenied == ~(~disc.grant /\ Len(outL)>0 /\ outL[Len(outL)].kind="Unavailable")
=============================================================================
