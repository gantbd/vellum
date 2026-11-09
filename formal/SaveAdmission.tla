-------------------------- MODULE SaveAdmission --------------------------
EXTENDS Integers, Sequences, FiniteSets
CONSTANTS Bound, QueuePolicy, Capacity, ForgetStopped, Recapture, DropQueued, Bypass,
          Lifecycle, BudgetUnits, CompletionSlots, IgnoreCost, IgnoreSlots, ReleaseUnknown
Requests == {"A","B","C","D"}
Resources == {"R","S"}
Resource(a) == IF a="D" THEN "S" ELSE "R"
Cost(a) == IF a="A" THEN 2 ELSE 1
Used(h) == 2*Cardinality(h \cap {"A"}) + Cardinality(h \ {"A"})
Remove(q,a) == SelectSeq(q,LAMBDA x: x # a)
VARIABLE s
Advance(t,label) == [t EXCEPT !.tick=s.tick+1, !.last=label]
Init == s=[status |-> [a \in Requests |-> "New"],
  queue |-> [r \in Resources |-> <<>>], captured |-> [a \in Requests |-> -1],
  payload |-> [a \in Requests |-> -1], live |-> 0,
  dispatched |-> {}, resolved |-> {}, stopped |-> {}, order |-> <<>>,
  waited |-> {}, frozen |-> FALSE, bypassed |-> FALSE,
  held |-> {}, reason |-> [a \in Requests |-> "None"],
  expiredQueued |-> {}, expiredActive |-> {}, reclaimed |-> FALSE,
  tick |-> 0, last |-> "Init"]
Outstanding == s.dispatched \ s.resolved
GuardOutstanding == IF ForgetStopped THEN Outstanding \ s.stopped ELSE Outstanding
Active(r) == {a \in Outstanding: Resource(a)=r}
GuardActive(r) == {a \in GuardOutstanding: Resource(a)=r}
Submit(a) == /\ s.status[a]="New"
  /\ LET r == Resource(a)
         busy == GuardActive(r) # {} \/ Len(s.queue[r])>0
         reason == IF ~QueuePolicy /\ busy THEN "Busy"
                   ELSE IF Len(s.queue[r])>=Capacity THEN "Resource"
                   ELSE IF Lifecycle /\ ~IgnoreCost /\ Used(s.held)+Cost(a)>BudgetUnits THEN "Budget"
                   ELSE IF Lifecycle /\ ~IgnoreSlots /\ Cardinality(s.held)>=CompletionSlots THEN "Slots"
                   ELSE "None"
         result == IF reason="Busy" THEN "Busy"
                   ELSE IF reason # "None" THEN "Full" ELSE "Queued"
     IN s'=Advance([s EXCEPT !.status[a]=result,
       !.reason[a]=reason,
       !.held=IF result="Queued" THEN @ \cup {a} ELSE @,
       !.reclaimed=@ \/ (result="Queued" /\ s.expiredQueued # {}),
       !.captured[a]=IF result="Queued" THEN s.live ELSE @,
       !.waited=IF result="Queued" /\ busy THEN @ \cup {a} ELSE @,
       !.queue[r]=IF result="Queued" THEN Append(@,a) ELSE @],"Submit")
Dispatch(a) == /\ s.status[a]="Queued" /\ GuardActive(Resource(a))={}
  /\ (Bypass \/ s.queue[Resource(a)][1]=a)
  /\ s'=Advance([s EXCEPT !.status[a]="Active",
       !.queue[Resource(a)]=Remove(@,a), !.dispatched=@ \cup {a},
       !.payload[a]=IF Recapture THEN s.live ELSE s.captured[a],
       !.order=Append(@,a), !.frozen=@ \/ s.live # s.captured[a],
       !.bypassed=@ \/ s.queue[Resource(a)][1] # a],"Dispatch")
Cancel(a) == /\ s.status[a] \in {"Queued","Active"}
  /\ s'=Advance([s EXCEPT
       !.queue[Resource(a)]=Remove(@,a),
       !.status[a]=IF @="Queued" THEN "Cancelled" ELSE "Stopped",
       !.held=IF s.status[a]="Queued" THEN @ \ {a} ELSE @,
       !.stopped=IF a \in s.dispatched THEN @ \cup {a} ELSE @],"Cancel")
\* Accepted exact-attempt safe-disposition evidence, not a caller timeout.
\* May mean confirmed completion or confirmed no-future-effect, not necessarily saved.
Resolve(a) == /\ a \in Outstanding
  /\ s'=Advance([s EXCEPT !.resolved=@ \cup {a},
       !.held=@ \ {a},
       !.status[a]=IF @="Stopped" THEN @ ELSE "Finished"],"Resolve")
\* Abstract observation of a deadline, not a wall-clock/scheduling proof.
Expire(a) == /\ Lifecycle /\ s.status[a] \in {"Queued","Active"}
  /\ s'=Advance([s EXCEPT
       !.queue[Resource(a)]=Remove(@,a),
       !.status[a]=IF @="Queued" THEN "Expired" ELSE "Stopped",
       !.expiredQueued=IF s.status[a]="Queued" THEN @ \cup {a} ELSE @,
       !.expiredActive=IF s.status[a]="Active" THEN @ \cup {a} ELSE @,
       !.stopped=IF s.status[a]="Active" THEN @ \cup {a} ELSE @,
       !.held=IF s.status[a]="Queued" \/ ReleaseUnknown THEN @ \ {a} ELSE @],"Expire")
Change == /\ s.live=0 /\ s'=Advance([s EXCEPT !.live=1],"Change")
SilentDrop(a) == /\ DropQueued /\ s.status[a]="Queued"
  /\ s'=Advance([s EXCEPT !.queue[Resource(a)]=Remove(@,a)],"Drop")
Next == /\ s.tick<Bound
  /\ ((\E a \in Requests: Submit(a) \/ Dispatch(a) \/ Cancel(a) \/ Resolve(a) \/ Expire(a) \/ SilentDrop(a)) \/ Change)
Spec == Init /\ [][Next]_s
TypeOK == /\ s.status \in [Requests -> {"New","Queued","Active","Stopped","Finished","Cancelled","Expired","Busy","Full"}]
  /\ s.dispatched \subseteq Requests /\ s.resolved \subseteq s.dispatched
  /\ s.stopped \subseteq s.dispatched /\ s.live \in {0,1}
  /\ \A r \in Resources: s.queue[r] \in Seq(Requests)
QueueLedger == \A a \in Requests:
  (s.status[a]="Queued") <=> a \in {s.queue[Resource(a)][i]: i \in 1..Len(s.queue[Resource(a)])}
CapacityBound == \A r \in Resources: Len(s.queue[r])<=Capacity
SingleFlight == \A r \in Resources: Cardinality(Active(r))<=1
ExactDispatch == \A a \in s.dispatched: s.payload[a]=s.captured[a]
FIFO == ~s.bypassed
Queued == {a \in Requests: s.status[a]="Queued"}
ReservationLedger == s.held = Queued \cup Outstanding
CostBound == Lifecycle => Used(s.held)<=BudgetUnits
CompletionBound == Lifecycle => Cardinality(s.held)<=CompletionSlots
ExpiredNotDispatched == s.expiredQueued \cap s.dispatched = {}
CaptureImmutable == [][\A a \in Requests:
  s.captured[a] # -1 => s'.captured[a]=s.captured[a]]_s
EvidenceMonotone == [][s.resolved \subseteq s'.resolved]_s
StoppedStable == [][\A a \in s.stopped: s'.status[a]="Stopped"]_s
ExpiredStable == [][\A a \in s.expiredQueued: s'.status[a]="Expired"]_s
NotBusyRejected == ~\E a \in Requests: s.status[a]="Busy"
NotCapacityRejected == ~\E a \in Requests: s.status[a]="Full"
NotQueuedCancelled == ~\E a \in Requests: s.status[a]="Cancelled"
NotFrozenDispatch == ~s.frozen
NotStoppedBlocks == ~\E a,b \in Requests:
  a \in Outstanding \cap s.stopped /\ s.status[b]="Queued" /\ Resource(a)=Resource(b)
NotLateRelease == ~\E a,b \in Requests:
  a \in s.resolved \cap s.stopped /\ b \in Outstanding /\ Resource(a)=Resource(b)
NotIndependentProgress == ~(Active("R") \cap s.stopped # {} /\ Active("S") # {})
NotFIFOProgress == ~(s.order = <<"A","B">> /\ "A" \in s.resolved /\ "B" \in s.waited)
NotQueuedExpiry == s.expiredQueued = {}
NotActiveExpiryHeld == s.expiredActive \cap Outstanding \cap s.held = {}
NotLateExpiredResolution == s.expiredActive \cap s.resolved = {}
NotBudgetRejected == ~\E a \in Requests: s.reason[a]="Budget"
NotSlotsRejected == ~\E a \in Requests: s.reason[a]="Slots"
NotReservationReused == ~s.reclaimed
NotUnknownBlocksBudget == ~(s.expiredActive \cap Outstanding # {} /\
  \E a \in Requests: s.reason[a]="Budget")
=============================================================================
