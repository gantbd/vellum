--------------------------- MODULE Persistence ---------------------------
EXTENDS Naturals, FiniteSets, TLC
CONSTANTS Bound, ConditionalCapable, IgnoreBinding, IgnoreCapability, LooseReceiptIdentity
Artifacts == {"Source","Annotations"}
Attempts == Artifacts \* One unique attempt per artifact in this finite slice.
None == [absent |-> TRUE]
VARIABLE s
Advance(t) == [t EXCEPT !.tick=s.tick+1]
Payload(a) == IF a="Source" THEN <<s.text,s.profile>> ELSE <<s.annotation,s.mapping>>
Version(a) == IF a="Source" THEN s.textRev ELSE s.annotationRev
Snapshot(a) == [artifact |-> a, attempt |-> a, incarnation |-> 1,
  revision |-> Version(a), binding |-> s.binding, profile |-> s.profile,
  payload |-> Payload(a), token |-> s.externalToken]
BadVariants == {"Attempt","Incarnation","Revision","Binding","Profile","Artifact","Payload","Token"}
Malformed(a,v) == LET c == s.captured[a] IN CASE
  v="Attempt" -> [c EXCEPT !.attempt=IF a="Source" THEN "Annotations" ELSE "Source"]
  [] v="Incarnation" -> [c EXCEPT !.incarnation=@+1]
  [] v="Revision" -> [c EXCEPT !.revision=@+1]
  [] v="Binding" -> [c EXCEPT !.binding=@+1]
  [] v="Profile" -> [c EXCEPT !.profile=@+1]
  [] v="Artifact" -> [c EXCEPT !.artifact=IF a="Source" THEN "Annotations" ELSE "Source"]
  [] v="Payload" -> [c EXCEPT !.payload = <<9,9>>]
  [] OTHER -> [c EXCEPT !.token=@+1]
Matches(a,c) == IF LooseReceiptIdentity THEN c.attempt=s.captured[a].attempt ELSE c=s.captured[a]
Init == s = [text |-> 0, textRev |-> 0, annotation |-> 0, annotationRev |-> 0,
  mapping |-> 0, profile |-> 0, binding |-> 0, externalToken |-> 0,
  phase |-> [a \in Attempts |-> "New"], captured |-> [a \in Attempts |-> None],
  dispatched |-> {}, quiet |-> {}, stopped |-> {},
  facts |-> [a \in Attempts |-> {}], host |-> [a \in Attempts |-> "None"],
  baseline |-> [a \in Artifacts |-> None], written |-> [a \in Attempts |-> None],
  rejectedReceipt |-> FALSE, last |-> "Init", tick |-> 0]
Capture(a) == /\ s.phase[a]="New"
  /\ s' = Advance([s EXCEPT !.captured[a]=Snapshot(a), !.phase[a]="Captured", !.last="Capture"])
EncodingFailure(a) == /\ s.phase[a]="Captured"
  /\ s' = Advance([s EXCEPT !.phase[a]="Rejected", !.last="EncodingFailure"])
Dispatch(a) == /\ s.phase[a]="Captured"
  /\ IF (ConditionalCapable \/ IgnoreCapability) /\ s.captured[a].binding=s.binding
        /\ s.captured[a].token=s.externalToken THEN
       s' = Advance([s EXCEPT !.phase[a]="Waiting", !.dispatched=@ \cup {a}, !.last="Dispatch"])
     ELSE s' = Advance([s EXCEPT !.phase[a]="Rejected", !.last="DispatchRejected"])
HostWrite(a) == /\ a \in s.dispatched /\ s.host[a]="None"
  /\ LET ok == ConditionalCapable /\ s.captured[a].token=s.externalToken
     IN s' = Advance([s EXCEPT !.host[a]=IF ok THEN "OK" ELSE "Failed",
       !.written[a]=IF ok THEN [snapshot |-> s.captured[a], tokenAtWrite |-> s.externalToken] ELSE None,
       !.quiet=@ \cup {a}, !.last="HostWrite"])
\* Complete tuple validation precedes accepted evidence. Mismatched candidates
\* represent wrong attempt/incarnation/revision/binding/profile inputs.
Receive(a) == /\ s.host[a] # "None"
  /\ LET fact == s.host[a]
         applicable == IgnoreBinding \/ s.captured[a].binding=s.binding
         base == IF fact="OK" /\ "Failed" \notin s.facts[a] /\ applicable
                 THEN [snapshot |-> s.captured[a], currentBinding |-> s.binding]
                 ELSE IF fact="Failed" THEN None ELSE s.baseline[a]
     IN s' = Advance([s EXCEPT !.facts[a]=@ \cup {fact}, !.baseline[a]=base,
       !.phase[a]=IF a \in s.stopped THEN @ ELSE "Done", !.last="Receive"])
BadReceipt(a) == /\ a \in s.dispatched
  /\ \E v \in BadVariants:
       IF Matches(a,Malformed(a,v)) THEN Receive(a)
       ELSE s' = Advance([s EXCEPT !.rejectedReceipt=TRUE, !.last="BadReceipt"])
\* A contradictory host report remains evidence, not an invented success.
Conflict(a) == /\ "OK" \in s.facts[a]
  /\ s' = Advance([s EXCEPT !.facts[a]=@ \cup {"Failed"}, !.baseline[a]=None, !.last="Conflict"])
Stop(a) == /\ s.phase[a] \in {"Captured","Waiting"} /\ a \notin s.stopped
  /\ s' = Advance([s EXCEPT !.stopped=@ \cup {a}, !.phase[a]="Stopped", !.last="Stop"])
Rebind == /\ s.binding=0 /\ s.dispatched \subseteq s.quiet
  /\ s' = Advance([s EXCEPT !.binding=1, !.baseline=[a \in Artifacts |-> None], !.last="Rebind"])
EditSource == /\ s.textRev<2
  /\ s' = Advance([s EXCEPT !.text=1-@, !.textRev=@+1, !.mapping=@+1, !.last="EditSource"])
EditAnnotation == /\ s.annotationRev<1
  /\ s' = Advance([s EXCEPT !.annotation=1-@, !.annotationRev=@+1, !.last="EditAnnotation"])
ChangeProfile == /\ s.profile=0 /\ s' = Advance([s EXCEPT !.profile=1, !.last="Profile"])
ExternalChange == /\ s.externalToken=0
  /\ s' = Advance([s EXCEPT !.externalToken=1, !.last="ExternalChange"])
Next == /\ s.tick<Bound
  /\ ((\E a \in Attempts: Capture(a) \/ EncodingFailure(a) \/ Dispatch(a) \/ HostWrite(a)
         \/ Receive(a) \/ BadReceipt(a) \/ Conflict(a) \/ Stop(a))
     \/ Rebind \/ EditSource \/ EditAnnotation \/ ChangeProfile \/ ExternalChange)
Spec == Init /\ [][Next]_s
BothSaveScript == CASE s.tick=0 -> Capture("Source") [] s.tick=1 -> Dispatch("Source")
  [] s.tick=2 -> HostWrite("Source") [] s.tick=3 -> Receive("Source")
  [] s.tick=4 -> Capture("Annotations") [] s.tick=5 -> Dispatch("Annotations")
  [] s.tick=6 -> HostWrite("Annotations") [] s.tick=7 -> Receive("Annotations")
  [] OTHER -> FALSE
BothSaveSpec == Init /\ [][Next /\ BothSaveScript]_s
BaselineAttribution == \A a \in Artifacts: s.baseline[a] # None =>
  /\ s.baseline[a].snapshot=s.captured[a]
  /\ s.baseline[a].currentBinding=s.baseline[a].snapshot.binding
  /\ s.baseline[a].currentBinding=s.binding
  /\ "OK" \in s.facts[a] /\ "Failed" \notin s.facts[a]
MalformedReceiptsRejected == \A a \in Attempts: s.captured[a] # None =>
  \A v \in BadVariants: ~Matches(a,Malformed(a,v))
NoUndispatchedEvidence == \A a \in Attempts: a \notin s.dispatched => s.facts[a]={}
StrongAdmission == ~ConditionalCapable => s.dispatched={}
ConditionalWrite == \A a \in Attempts: s.written[a] # None =>
  /\ ConditionalCapable /\ s.written[a].tokenAtWrite=s.written[a].snapshot.token
EvidenceMonotone == [][\A a \in Attempts: s.facts[a] \subseteq s'.facts[a]]_s
CaptureImmutable == [][\A a \in Attempts: s.captured[a] # None => s'.captured[a]=s.captured[a]]_s
StopPreservesEvidence == [][s'.last="Stop" => s'.facts=s.facts /\ s'.dispatched=s.dispatched]_s
BadReceiptNoEffect == [][s'.last="BadReceipt" => s'.facts=s.facts /\ s'.baseline=s.baseline]_s
SafeRebind == [][s'.binding # s.binding => s.dispatched \subseteq s.quiet]_s
SourceDoesNotSaveAnnotations == [][s'.last="Receive" /\ s'.facts["Source"] # s.facts["Source"] =>
  s'.baseline["Annotations"]=s.baseline["Annotations"]]_s
SavedSource == s.baseline["Source"] # None
SavedAnnotations == s.baseline["Annotations"] # None
BothArtifactsSaved == SavedSource /\ SavedAnnotations
OldRevisionReceipt == SavedSource /\ s.baseline["Source"].snapshot.revision < s.textRev
UndoEqualFreshRevision == SavedSource /\ s.baseline["Source"].snapshot.payload=Payload("Source")
  /\ s.baseline["Source"].snapshot.revision < s.textRev
StoppedUnknown == "Source" \in s.stopped \cap s.dispatched /\ s.facts["Source"]={}
LateReceipt == SavedSource /\ "Source" \in s.stopped /\ s.phase["Source"]="Stopped"
HistoricalAfterRebind == s.binding=1 /\ "OK" \in s.facts["Source"]
  /\ s.captured["Source"] # None /\ s.captured["Source"].binding=0 /\ s.baseline["Source"]=None
PartialSave == SavedSource /\ s.annotationRev=1 /\ s.baseline["Annotations"]=None
CASRaceRejected == s.host["Source"]="Failed" /\ s.externalToken=1
ConflictingEvidence == {"OK","Failed"} \subseteq s.facts["Source"] /\ ~SavedSource
UnsupportedRejected == ~ConditionalCapable /\ s.phase["Source"]="Rejected" /\ s.dispatched={}
  /\ s.last="DispatchRejected"
MismatchedReceiptRejected == s.rejectedReceipt
NotSavedSource == ~SavedSource
NotSavedAnnotations == ~SavedAnnotations
NotBothArtifactsSaved == ~BothArtifactsSaved
NotOldRevisionReceipt == ~OldRevisionReceipt
NotUndoEqualFreshRevision == ~UndoEqualFreshRevision
NotStoppedUnknown == ~StoppedUnknown
NotLateReceipt == ~LateReceipt
NotHistoricalAfterRebind == ~HistoricalAfterRebind
NotPartialSave == ~PartialSave
NotCASRaceRejected == ~CASRaceRejected
NotConflictingEvidence == ~ConflictingEvidence
NotUnsupportedRejected == ~UnsupportedRejected
NotMismatchedReceiptRejected == ~MismatchedReceiptRejected
=============================================================================
