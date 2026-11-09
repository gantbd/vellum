-------------------------- MODULE ConcurrentSaves -------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC
CONSTANTS Bound, SingleFlight, OrderedEvidence, FreshTokens, ProfileOnly,
          ReceiptOrderSelection, ForgetStopped, FreezeToken, UnsafeTokenOrder
Attempts == {"Old","New"}
Absent == [absent |-> TRUE]
VARIABLE s
Advance(t,label) == [t EXCEPT !.tick=s.tick+1, !.last=label]
NextToken(t) == IF FreezeToken THEN t ELSE IF FreshTokens THEN t+1 ELSE (t+1)%2
\* The slice starts with unsaved live text 1 and external text 0. A later edit
\* returns to 0 at a fresh revision; the reusable-token script is a real ABA
\* representation-return witness, not merely a token collision on different data.
Initial == [generation |-> 0, revision |-> 0, profile |-> 0, text |-> 1]
Snapshot(a) == [attempt |-> a, document |-> "A", incarnation |-> 1, binding |-> 0,
  revision |-> s.live.revision, profile |-> s.live.profile,
  payload |-> <<s.live.text,s.live.profile>>, expectedToken |-> s.observedToken]
Init == s = [live |-> Initial, captured |-> [a \in Attempts |-> Absent],
  dispatched |-> {}, stopped |-> {}, result |-> [a \in Attempts |-> "None"],
  resultToken |-> [a \in Attempts |-> 0], position |-> [a \in Attempts |-> 0],
  actualToken |-> 0, actualPayload |-> <<0,0>>, observedToken |-> 0,
  writes |-> <<>>, foreign |-> FALSE, receipts |-> {}, arrivals |-> <<>>,
  tick |-> 0, last |-> "Init"]
Received == {r.attempt: r \in s.receipts}
SeenOK == {r.attempt: r \in {r \in s.receipts: r.outcome="OK"}}
Unresolved == s.dispatched \ Received
DispatchPending == IF ForgetStopped THEN Unresolved \ s.stopped ELSE Unresolved
Before(a,b) == a # b /\ a \in SeenOK /\ b \in SeenOK /\
  ((OrderedEvidence /\ s.position[a]<s.position[b]) \/
   ((FreshTokens \/ UnsafeTokenOrder) /\ s.resultToken[a]=s.captured[b].expectedToken))
Maxima == {a \in SeenOK: ~\E b \in SeenOK: Before(a,b)}
Selected == IF ReceiptOrderSelection /\ Len(s.arrivals)>0
            THEN {s.arrivals[Len(s.arrivals)]} ELSE Maxima
Capture(a) == /\ s.captured[a]=Absent
  /\ s.live.generation=(IF a="Old" THEN 0 ELSE 1)
  /\ s' = Advance([s EXCEPT !.captured[a]=Snapshot(a)],"Capture")
ChangeSource == /\ s.live.generation=0
  /\ s' = Advance([s EXCEPT !.live=[generation |-> 1,
       revision |-> IF ProfileOnly THEN 0 ELSE 1,
       profile |-> IF ProfileOnly THEN 1 ELSE 0,
       text |-> IF ProfileOnly THEN 1 ELSE 0]],"ChangeSource")
Probe == s' = Advance([s EXCEPT !.observedToken=s.actualToken],"Probe")
Dispatch(a) == /\ s.captured[a] # Absent /\ a \notin s.dispatched \cup s.stopped
  /\ IF SingleFlight /\ DispatchPending # {} THEN s'=Advance(s,"Blocked")
     ELSE s'=Advance([s EXCEPT !.dispatched=@ \cup {a}],"Dispatch")
HostWrite(a) == /\ a \in s.dispatched /\ s.result[a]="None"
  /\ LET ok == s.captured[a].expectedToken=s.actualToken
         token == IF ok THEN NextToken(s.actualToken) ELSE s.actualToken
     IN s' = Advance([s EXCEPT !.result[a]=IF ok THEN "OK" ELSE "Conflict",
       !.resultToken[a]=token, !.position[a]=IF ok THEN Len(s.writes)+1 ELSE 0,
       !.actualToken=token, !.actualPayload=IF ok THEN s.captured[a].payload ELSE @,
       !.writes=IF ok THEN Append(@,a) ELSE @],"HostWrite")
ForeignWrite == /\ ~s.foreign
  /\ s' = Advance([s EXCEPT !.foreign=TRUE, !.actualToken=NextToken(@),
       !.actualPayload = <<9,9>>, !.writes=Append(@,"Foreign")],"ForeignWrite")
Receipt(a) == [attempt |-> a, snapshot |-> s.captured[a], outcome |-> s.result[a],
  token |-> s.resultToken[a], order |-> IF OrderedEvidence THEN s.position[a] ELSE 0]
Receive(a) == /\ s.result[a] # "None"
  /\ s'=Advance([s EXCEPT !.receipts=@ \cup {Receipt(a)},
       !.arrivals=IF s.result[a]="OK" THEN Append(@,a) ELSE @],"Receive")
Stop(a) == /\ a \in s.dispatched \ s.stopped
  /\ s'=Advance([s EXCEPT !.stopped=@ \cup {a}],"Stop")
Next == /\ s.tick<Bound
  /\ ((\E a \in Attempts: Capture(a) \/ Dispatch(a) \/ HostWrite(a) \/ Receive(a) \/ Stop(a))
      \/ ChangeSource \/ Probe \/ ForeignWrite)
Spec == Init /\ [][Next]_s
\* Both writes succeed with independently refreshed preconditions, but receipts
\* arrive New then Old. Receipt order must not replace storage-order evidence.
ChainScript == CASE s.tick=0 -> Capture("Old") [] s.tick=1 -> Dispatch("Old")
  [] s.tick=2 -> HostWrite("Old") [] s.tick=3 -> Probe [] s.tick=4 -> ChangeSource
  [] s.tick=5 -> Capture("New") [] s.tick=6 -> Dispatch("New")
  [] s.tick=7 -> HostWrite("New") [] s.tick=8 -> Receive("New")
  [] s.tick=9 -> Receive("Old") [] OTHER -> FALSE
ChainSpec == Init /\ [][Next /\ ChainScript]_s
GapScript == CASE s.tick=0 -> Capture("Old") [] s.tick=1 -> Dispatch("Old")
  [] s.tick=2 -> HostWrite("Old") [] s.tick=3 -> ForeignWrite [] s.tick=4 -> Probe
  [] s.tick=5 -> ChangeSource [] s.tick=6 -> Capture("New") [] s.tick=7 -> Dispatch("New")
  [] s.tick=8 -> HostWrite("New") [] s.tick=9 -> Receive("New")
  [] s.tick=10 -> Receive("Old") [] OTHER -> FALSE
GapSpec == Init /\ [][Next /\ GapScript]_s
ExactReceipts == \A r \in s.receipts:
  /\ r.snapshot=s.captured[r.attempt] /\ r.outcome=s.result[r.attempt]
  /\ r.token=s.resultToken[r.attempt]
  /\ r.order=IF OrderedEvidence THEN s.position[r.attempt] ELSE 0
OneWinnerPerToken == FreshTokens => \A a,b \in Attempts:
  a # b /\ s.result[a]="OK" /\ s.result[b]="OK"
    => s.captured[a].expectedToken # s.captured[b].expectedToken
EvidenceOrderSound == \A a,b \in Attempts: Before(a,b) => s.position[a]<s.position[b]
SelectionRespectsEvidence == Selected=Maxima
SingleFlightBound == SingleFlight => Cardinality(Unresolved)<=1
ReceiptsMonotone == [][s.receipts \subseteq s'.receipts]_s
CaptureImmutable == [][\A a \in Attempts: s.captured[a] # Absent => s'.captured[a]=s.captured[a]]_s
StopPreservesObligations == [][s'.last="Stop" =>
  /\ s'.dispatched=s.dispatched /\ s'.receipts=s.receipts /\ s'.result=s.result]_s
DuplicateSelectionStable == [][s'.last="Receive" /\ s'.receipts=s.receipts => Selected'=Selected]_s
\* The actual world is an oracle for checking soundness, not accessible ordering
\* evidence unless OrderedEvidence is part of the declared host profile.
ConcurrentOutstanding == Cardinality(Unresolved)=2
StoppedOutstanding == "Old" \in s.stopped \cap Unresolved
StoppedBlocksNew == SingleFlight /\ s.last="Blocked" /\ "Old" \in s.stopped \cap Unresolved
NewWinsRace == s.result["New"]="OK" /\ s.result["Old"]="Conflict"
ReverseDeliverySafe == SeenOK=Attempts /\ Len(s.arrivals)>=2 /\ s.arrivals[Len(s.arrivals)]="Old"
  /\ Selected={"New"} /\ Before("Old","New")
SameRevisionDifferentOutput == SeenOK=Attempts
  /\ s.captured["Old"].revision=s.captured["New"].revision
  /\ s.captured["Old"].payload # s.captured["New"].payload
TokenChainGapUnresolved == ~OrderedEvidence /\ s.foreign /\ SeenOK=Attempts /\ Maxima=Attempts
OrderedGapResolved == OrderedEvidence /\ s.foreign /\ SeenOK=Attempts /\ Maxima={"New"}
ReusableTokensUnordered == ~FreshTokens /\ ~OrderedEvidence /\ SeenOK=Attempts /\ Maxima=Attempts
  /\ s.actualPayload = <<0,0>>
NotConcurrentOutstanding == ~ConcurrentOutstanding
NotStoppedOutstanding == ~StoppedOutstanding
NotStoppedBlocksNew == ~StoppedBlocksNew
NotNewWinsRace == ~NewWinsRace
NotReverseDeliverySafe == ~ReverseDeliverySafe
NotSameRevisionDifferentOutput == ~SameRevisionDifferentOutput
NotTokenChainGapUnresolved == ~TokenChainGapUnresolved
NotOrderedGapResolved == ~OrderedGapResolved
NotReusableTokensUnordered == ~ReusableTokensUnordered
=============================================================================
