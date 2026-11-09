-------------------------- MODULE DispatchGate ---------------------------
EXTENDS Naturals, FiniteSets
CONSTANTS Bound, ProviderSafe, IgnoreAuthority, IgnoreBinding, IgnoreEpoch,
          TrustAnything, MatchAttemptOnly, PromoteProgress, IgnoreCapability,
          RequireCurrentReceipt
Attempts == {"A","B"}
Kinds == {"Progress","Completed","Quiescent","CancelIntent"}
Absent == [absent |-> TRUE]
VARIABLE s
Advance(t,label) == [t EXCEPT !.tick=s.tick+1, !.last=label]
Init == s=[status |-> [a \in Attempts |-> "New"],
  capture |-> [a \in Attempts |-> Absent], text |-> 0,
  binding |-> 0, epoch |-> 0, granted |-> TRUE,
  dispatched |-> {}, stopped |-> {}, resolved |-> {},
  receipts |-> {}, goodSeen |-> {}, badObserved |-> FALSE,
  authorityLaunch |-> TRUE, bindingLaunch |-> TRUE, epochLaunch |-> TRUE,
  safeResolved |-> {}, frozenLaunch |-> FALSE, tick |-> 0, last |-> "Init"]
Outstanding == s.dispatched \ s.resolved
Tuple(a) == [attempt |-> a, resource |-> "R", incarnation |-> 1,
  binding |-> s.binding, epoch |-> s.epoch, profile |-> 0, payload |-> s.text]
Capture(a) == /\ s.status[a]="New"
  /\ s'=Advance([s EXCEPT !.status[a]="Queued", !.capture[a]=Tuple(a)],"Capture")
Dispatch(a) == /\ s.status[a]="Queued" /\ Outstanding={}
  /\ LET denied == ~IgnoreAuthority /\ ~s.granted
         staleBinding == ~IgnoreBinding /\ s.capture[a].binding # s.binding
         staleEpoch == ~IgnoreEpoch /\ s.capture[a].epoch # s.epoch
         reject == denied \/ staleBinding \/ staleEpoch
         reason == IF denied THEN "Denied" ELSE IF staleBinding THEN "StaleBinding" ELSE "StaleEpoch"
     IN s'=Advance([s EXCEPT !.status[a]=IF reject THEN reason ELSE "Active",
       !.dispatched=IF reject THEN @ ELSE @ \cup {a},
       !.authorityLaunch=@ /\ (reject \/ s.granted),
       !.bindingLaunch=@ /\ (reject \/ s.capture[a].binding=s.binding),
       !.epochLaunch=@ /\ (reject \/ s.capture[a].epoch=s.epoch),
       !.frozenLaunch=@ \/ (~reject /\ s.capture[a].payload # s.text)],"Dispatch")
Revoke == /\ s.granted /\ s'=Advance([s EXCEPT !.granted=FALSE],"Revoke")
\* Environment invalidation, NOT permission to activate a new binding while unknown.
InvalidateBinding == /\ s.binding=0 /\ s'=Advance([s EXCEPT !.binding=1],"InvalidateBinding")
NewEpoch == /\ s.epoch=0 /\ s'=Advance([s EXCEPT !.epoch=1],"NewEpoch")
Edit == /\ s.text=0 /\ s'=Advance([s EXCEPT !.text=1],"Edit")
Stop(a) == /\ s.status[a]="Active"
  /\ s'=Advance([s EXCEPT !.status[a]="Stopped", !.stopped=@ \cup {a}],"Stop")
EndQueued(a,why) == /\ s.status[a]="Queued" /\ why \in {"Cancelled","Expired"}
  /\ s'=Advance([s EXCEPT !.status[a]=why],"EndQueued")
\* Finite representatives of wrong routing/tuple fields, not all malformed bytes.
Alter(t,v) == CASE v=0 -> t
  [] v=1 -> [t EXCEPT !.resource="Other"]
  [] v=2 -> [t EXCEPT !.incarnation=2]
  [] v=3 -> [t EXCEPT !.binding=@+1]
  [] v=4 -> [t EXCEPT !.epoch=@+1]
  [] v=5 -> [t EXCEPT !.profile=1]
  [] v=6 -> [t EXCEPT !.payload=2]
  [] OTHER -> [t EXCEPT !.attempt=IF @="A" THEN "B" ELSE "A"]
Receive(a,v,issuer,kind) == /\ a \in s.dispatched
  /\ LET tuple == Alter(s.capture[a],v)
         msg == [tuple |-> tuple, issuer |-> issuer, kind |-> kind]
         exact == tuple=s.capture[a]
         good == issuer="Writer" /\ exact
         accepted == (issuer="Writer" \/ TrustAnything) /\
           (IF MatchAttemptOnly THEN tuple.attempt=a ELSE exact) /\
           (~RequireCurrentReceipt \/
             (s.granted /\ tuple.binding=s.binding /\ tuple.epoch=s.epoch))
         safeKind == kind \in {"Completed","Quiescent"}
         resolves == accepted /\ (safeKind \/ PromoteProgress) /\ (ProviderSafe \/ IgnoreCapability)
         actuallySafe == good /\ safeKind /\ ProviderSafe
     IN s'=Advance([s EXCEPT
       !.goodSeen=IF good THEN @ \cup {msg} ELSE @,
       !.receipts=IF accepted THEN @ \cup {msg} ELSE @,
       !.badObserved=@ \/ ~good,
       !.resolved=IF resolves THEN @ \cup {a} ELSE @,
       !.safeResolved=IF actuallySafe THEN @ \cup {a} ELSE @,
       !.status[a]=IF resolves /\ @="Active" THEN "Finished" ELSE @],"Receive")
Next == /\ s.tick<Bound
  /\ ((\E a \in Attempts: Capture(a) \/ Dispatch(a) \/ Stop(a) \/
        (\E why \in {"Cancelled","Expired"}: EndQueued(a,why)) \/
        (\E v \in 0..7, issuer \in {"Writer","Stranger"}, kind \in Kinds: Receive(a,v,issuer,kind)))
       \/ Revoke \/ InvalidateBinding \/ NewEpoch \/ Edit)
Spec == Init /\ [][Next]_s
TypeOK == /\ s.status \in [Attempts -> {"New","Queued","Active","Stopped","Finished","Denied","StaleBinding","StaleEpoch","Cancelled","Expired"}]
  /\ s.dispatched \subseteq Attempts /\ s.resolved \subseteq s.dispatched
  /\ s.stopped \subseteq s.dispatched
AuthorityAtLaunch == s.authorityLaunch
BindingAtLaunch == s.bindingLaunch
EpochAtLaunch == s.epochLaunch
ExactTrustedEvidence == \A m \in s.receipts:
  /\ m.issuer="Writer" /\ m.tuple.attempt \in s.dispatched
  /\ m.tuple=s.capture[m.tuple.attempt]
ValidEvidenceRetained == s.goodSeen \subseteq s.receipts
SafeResolution == s.resolved \subseteq s.safeResolved
SingleFlight == Cardinality(Outstanding)<=1
EvidenceMonotone == [][s.receipts \subseteq s'.receipts /\ s.resolved \subseteq s'.resolved]_s
CaptureImmutable == [][\A a \in Attempts:
  s.capture[a] # Absent => s'.capture[a]=s.capture[a]]_s
StoppedStable == [][\A a \in s.stopped: s'.status[a]="Stopped"]_s
TerminalStable == [][\A a \in Attempts:
  s.status[a] \in {"Finished","Denied","StaleBinding","StaleEpoch","Cancelled","Expired"}
    => s'.status[a]=s.status[a]]_s
NotAcceptedDisposition == s.resolved={}
NotDeniedLaunch == ~\E a \in Attempts: s.status[a]="Denied"
NotStaleBinding == ~\E a \in Attempts: s.status[a]="StaleBinding"
NotStaleEpoch == ~\E a \in Attempts: s.status[a]="StaleEpoch"
NotFrozenAllowed == ~s.frozenLaunch
NotBadIgnored == ~(s.badObserved /\ s.receipts={} /\ Outstanding # {})
NotProgressHeld == ~(Outstanding # {} /\ \E m \in s.receipts: m.kind="Progress")
NotLateBindingEvidence == ~\E m \in s.receipts: m.tuple.binding # s.binding
NotLateRevokedEvidence == ~(~s.granted /\ s.receipts # {})
NotStoppedLateCompletion == s.resolved \cap s.stopped = {}
=============================================================================
