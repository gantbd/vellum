--------------------------- MODULE UsefulProgress ---------------------------
EXTENDS Naturals, FiniteSets
CONSTANTS Bound, SilentDrop, AcceptStale, CancelDischarges
Jobs == 1..Bound
VARIABLES pending, disposition, cancelled, current, acceptedSafe
vars == <<pending,disposition,cancelled,current,acceptedSafe>>
\* Closed finite backlog; Bound is its cardinality, NOT a transition horizon.
\* Service models a cooperative completion/disposition, not native preemption.
Init == /\ pending=Jobs /\ disposition=[a \in Jobs |-> "Pending"]
  /\ cancelled={} /\ current=TRUE /\ acceptedSafe=TRUE
Service(a) == /\ a \in pending /\ pending'=pending \ {a}
  /\ LET result == IF a \in cancelled THEN "Cancelled"
                   ELSE IF current \/ AcceptStale THEN "Applied" ELSE "Superseded"
     IN /\ disposition'=(IF SilentDrop THEN disposition ELSE [disposition EXCEPT ![a]=result])
        /\ acceptedSafe'=(acceptedSafe /\ (result="Applied" => current /\ a \notin cancelled))
  /\ UNCHANGED <<cancelled,current>>
Serve == \E a \in Jobs: Service(a)
Cancel(a) == /\ a \in pending /\ a \notin cancelled
  /\ cancelled'=cancelled \cup {a}
  /\ pending'=(IF CancelDischarges THEN pending \ {a} ELSE pending)
  /\ UNCHANGED <<disposition,current,acceptedSafe>>
Invalidate == /\ current /\ current'=FALSE
  /\ UNCHANGED <<pending,disposition,cancelled,acceptedSafe>>
Next == Serve \/ (\E a \in Jobs: Cancel(a)) \/ Invalidate
Spec == Init /\ [][Next]_vars /\ WF_vars(Serve)
NoFairSpec == Init /\ [][Next]_vars
Finished == {a \in Jobs: disposition[a] # "Pending"}
DispositionPartition == /\ pending \cup Finished=Jobs /\ pending \cap Finished={}
  /\ disposition \in [Jobs -> {"Pending","Applied","Superseded","Cancelled"}]
CurrentAcceptance == acceptedSafe
TerminalStable == [][\A a \in Finished: disposition'[a]=disposition[a]]_vars
ServiceDecreases == [][Serve => Cardinality(pending')+1=Cardinality(pending)]_vars
EventualDrain == <>(pending={})
AppliedDone == pending={} /\ \E a \in Jobs: disposition[a]="Applied"
SupersededDone == pending={} /\ \E a \in Jobs: disposition[a]="Superseded"
CancelledDone == pending={} /\ \E a \in Jobs: disposition[a]="Cancelled"
CancelStillPending == cancelled \cap pending # {}
NotAppliedDone == ~AppliedDone
NotSupersededDone == ~SupersededDone
NotCancelledDone == ~CancelledDone
NotCancelStillPending == ~CancelStillPending
=============================================================================
