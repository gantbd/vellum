----------------------------- MODULE History -----------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC
CONSTANTS Bound, MixedGroup, PartialUndo
Scopes == {"A", "B", "Comment", "N"}
Groups == {"G", "H", "C", "I", "F"}
Members(g) == CASE g = "G" -> IF MixedGroup THEN {"A","B","Comment"} ELSE {"A","B"}
               [] g = "H" -> {"B"} [] g = "C" -> {"Comment"}
               [] g = "I" -> {"N"} [] OTHER -> {"A"}
Elements(xs) == {xs[i]: i \in 1..Len(xs)}
Pop(xs) == SubSeq(xs,1,Len(xs)-1)
Top(xs,g) == Len(xs)>0 /\ xs[Len(xs)]=g
RECURSIVE Remove(_, _)
Remove(xs,bad) == IF Len(xs)=0 THEN <<>> ELSE
  IF xs[1] \in bad THEN Remove(Tail(xs),bad)
  ELSE <<xs[1]>> \o Remove(Tail(xs),bad)
VARIABLE s
Advance(t) == [t EXCEPT !.tick = s.tick+1]
Init == s = [undo |-> [r \in Scopes |-> <<>>], redo |-> [r \in Scopes |-> <<>>],
  content |-> [r \in Scopes |-> <<>>], rev |-> [r \in Scopes |-> 0],
  used |-> {}, invalid |-> {}, unavailable |-> {}, denied |-> {},
  last |-> "Init", target |-> "None", tick |-> 0]
Dependent(bad) == bad \cup {g \in Groups: \E r \in Scopes: \E i \in 1..Len(s.redo[r]):
  s.redo[r][i] \in bad /\ g \in Elements(SubSeq(s.redo[r],1,i))}
RECURSIVE Closure(_)
Closure(bad) == IF Dependent(bad)=bad THEN bad ELSE Closure(Dependent(bad))
Invalidated(g) == Closure(UNION {Elements(s.redo[r]): r \in Members(g)})
Eligible(g,stack) == /\ g \notin s.unavailable \cup s.invalid
  /\ Members(g) \cap s.denied = {}
  /\ \A r \in Members(g): Top(stack[r],g)
ApplyNew(g) == /\ g \notin s.used /\ Members(g) \cap s.denied = {}
  /\ LET bad == Invalidated(g) IN s' = Advance([s EXCEPT
     !.undo = [r \in Scopes |-> IF r \in Members(g) THEN Append(s.undo[r],g) ELSE s.undo[r]],
     !.redo = [r \in Scopes |-> Remove(s.redo[r],bad)],
     !.content = [r \in Scopes |-> IF r \in Members(g) THEN Append(s.content[r],g) ELSE s.content[r]],
     !.rev = [r \in Scopes |-> s.rev[r] + IF r \in Members(g) THEN 1 ELSE 0],
     !.used = @ \cup {g}, !.invalid = @ \cup bad, !.last="Edit", !.target=g])
Undo(g) == /\ g \in s.used /\ g \notin s.invalid
  /\ IF Eligible(g,s.undo) THEN
       LET participants == IF PartialUndo /\ g="G" THEN {"A"} ELSE Members(g)
       IN s' = Advance([s EXCEPT
         !.undo = [r \in Scopes |-> IF r \in participants THEN Pop(s.undo[r]) ELSE s.undo[r]],
         !.redo = [r \in Scopes |-> IF r \in participants THEN Append(s.redo[r],g) ELSE s.redo[r]],
         !.content = [r \in Scopes |-> IF r \in participants THEN Pop(s.content[r]) ELSE s.content[r]],
         !.rev = [r \in Scopes |-> s.rev[r] + IF r \in participants THEN 1 ELSE 0],
         !.last="Undo", !.target=g])
     ELSE s' = Advance([s EXCEPT !.last="Blocked", !.target=g])
Redo(g) == /\ g \in s.used /\ g \notin s.invalid
  /\ IF Eligible(g,s.redo) THEN s' = Advance([s EXCEPT
       !.undo = [r \in Scopes |-> IF r \in Members(g) THEN Append(s.undo[r],g) ELSE s.undo[r]],
       !.redo = [r \in Scopes |-> IF r \in Members(g) THEN Pop(s.redo[r]) ELSE s.redo[r]],
       !.content = [r \in Scopes |-> IF r \in Members(g) THEN Append(s.content[r],g) ELSE s.content[r]],
       !.rev = [r \in Scopes |-> s.rev[r] + IF r \in Members(g) THEN 1 ELSE 0],
       !.last="Redo", !.target=g])
     ELSE s' = Advance([s EXCEPT !.last="Blocked", !.target=g])
Evict(g) == /\ g \in s.used \ s.unavailable
  /\ s' = Advance([s EXCEPT !.unavailable = @ \cup {g}, !.last="Evict", !.target=g])
Deny(r) == /\ r \notin s.denied /\ s' = Advance([s EXCEPT !.denied=@ \cup {r}, !.last="Deny"])
Next == /\ s.tick < Bound
  /\ ((\E g \in Groups: ApplyNew(g) \/ Undo(g) \/ Redo(g) \/ Evict(g)) \/ (\E r \in Scopes: Deny(r)))
Spec == Init /\ [][Next]_s
NoSplitUndo == \A g \in Groups: \A a,b \in Members(g):
  (g \in Elements(s.undo[a])) <=> (g \in Elements(s.undo[b]))
NoSplitRedo == \A g \in Groups: \A a,b \in Members(g):
  (g \in Elements(s.redo[a])) <=> (g \in Elements(s.redo[b]))
HistoryMatchesContent == s.content = s.undo
NoInvalidRedo == \A r \in Scopes: Elements(s.redo[r]) \cap s.invalid = {}
DisjointPositions == \A r \in Scopes: Elements(s.undo[r]) \cap Elements(s.redo[r]) = {}
BlockedUnchanged == [][s'.last="Blocked" =>
  /\ s'.content=s.content /\ s'.undo=s.undo /\ s'.redo=s.redo /\ s'.rev=s.rev]_s
FreshVersions == [][\A r \in Scopes:
  IF s'.content[r] # s.content[r] THEN s'.rev[r]=s.rev[r]+1 ELSE s'.rev[r]=s.rev[r]]_s
ReversalValidated == [][s'.last \in {"Undo","Redo"} =>
  Eligible(s'.target, IF s'.last="Undo" THEN s.undo ELSE s.redo)]_s
LinkedBlocked == s.last="Blocked" /\ s.target="G" /\ Top(s.undo["B"],"H")
IndependentUndo == s.last="Undo" /\ s.target="G" /\ Top(s.undo["N"],"I")
CommentPreserved == ~MixedGroup /\ s.last="Undo" /\ s.target="G" /\ Top(s.content["Comment"],"C")
CompoundBlocked == MixedGroup /\ s.last="Blocked" /\ s.target="G" /\ Top(s.undo["Comment"],"C")
RoundTrip == s.last="Redo" /\ s.target="G" /\ s.rev["A"]>=3
RedoInvalidated == "G" \in s.invalid /\ "F" \in s.used
DependentInvalidated == {"G","H"} \subseteq s.invalid
RetentionBlocks == s.last="Blocked" /\ s.target="G" /\ "G" \in s.unavailable
RightsBlock == s.last="Blocked" /\ s.target="G" /\ "A" \in s.denied
NotLinkedBlocked == ~LinkedBlocked
NotIndependentUndo == ~IndependentUndo
NotCommentPreserved == ~CommentPreserved
NotCompoundBlocked == ~CompoundBlocked
NotRoundTrip == ~RoundTrip
NotRedoInvalidated == ~RedoInvalidated
NotDependentInvalidated == ~DependentInvalidated
NotRetentionBlocks == ~RetentionBlocks
NotRightsBlock == ~RightsBlock
=============================================================================
