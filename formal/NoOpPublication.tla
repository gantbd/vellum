------------------------- MODULE NoOpPublication -------------------------
EXTENDS Integers, Sequences, FiniteSets, TLC
CONSTANTS Bound, Kind, DiagnosticMode, SkipValidation, TextOnly, ForceChange,
          BookkeepNoop, RestoreCaptured, DiagnosticWrites, DiagnosticRejects
VARIABLE s
EC == INSTANCE EditorContracts WITH Bound <- Bound, SourceLimit <- 3,
  InsertLimit <- 2, BatchInsertLimit <- 1, UnsafeScope <- FALSE,
  SplitState <- FALSE, LeakRelease <- FALSE, SkipGap <- FALSE,
  BadMap <- FALSE, TrustDiscovery <- FALSE,
  IgnoreEventGrant <- FALSE, IgnoreEventRetention <- FALSE, s <- s
Base == <<"a","b","a">>
InitialRoot == [gen |-> 0, rev |-> 0, text |-> Base, point |-> 1,
  body |-> "comment", groupOpen |-> TRUE, history |-> 0, other |-> 0]
Semantic(r) == [text |-> r.text, point |-> r.point,
  body |-> r.body, groupOpen |-> r.groupOpen]
\* Reference kinds only, not public verbs or default mapping/history profiles:
\* 0 empty; 1 same-text identity; 2 same-text mapped target; 3 selection;
\* 4 annotation; 5 explicit group close; 6 text; 7 effect intent; 8 malformed.
Replacement(r) == [lo |-> 0, hi |-> IF Kind=2 THEN 2 ELSE 1,
  ins |-> SubSeq(r.text,1,IF Kind=2 THEN 2 ELSE 1)]
Candidate(r) ==
  IF Kind \in {1,2} THEN [Semantic(r) EXCEPT
    !.text=EC!Apply(r.text,Replacement(r)), !.point=EC!MapPoint(r.point,Replacement(r))]
  ELSE IF Kind=3 THEN [Semantic(r) EXCEPT !.point=2]
  ELSE IF Kind=4 THEN [Semantic(r) EXCEPT !.body="new comment"]
  ELSE IF Kind=5 THEN [Semantic(r) EXCEPT !.groupOpen=FALSE]
  ELSE IF Kind=6 THEN [Semantic(r) EXCEPT
    !.text=EC!Apply(r.text,EC!Edit), !.point=EC!MapPoint(r.point,EC!Edit)]
  ELSE Semantic(r)
HasEffect == Kind=7
Publish(r,c) == [r EXCEPT !.gen=@+1,
  !.rev=@+(IF c.text # r.text THEN 1 ELSE 0),
  !.text=c.text, !.point=c.point, !.body=c.body, !.groupOpen=c.groupOpen,
  !.history=@+(IF Kind=6 /\ c.text # r.text THEN 1 ELSE 0)]
Checks(t) == [shape |-> Kind # 8, grant |-> t.grant, scope |-> t.scope,
  writable |-> t.writable, capacity |-> t.capacity, capability |-> t.capability,
  revision |-> t.capture.root.rev=t.root.rev, context |-> t.capture.context=t.context]
All(checks) == \A key \in DOMAIN checks: checks[key]
Identity(t) == t.capture.candidate=Semantic(t.root) /\ ~HasEffect
Terminal == {"Unchanged","Applied","EffectRequested","Rejected","Stopped"}
Advance(t) == [t EXCEPT !.tick=s.tick+1]
Init == s=[root |-> InitialRoot, phase |-> "New", context |-> 0,
  grant |-> TRUE, scope |-> TRUE, writable |-> TRUE, capacity |-> TRUE, capability |-> TRUE,
  capture |-> [root |-> InitialRoot, context |-> 0, candidate |-> Candidate(InitialRoot)],
  foreignStage |-> 0, effectRequested |-> FALSE, lateCancel |-> FALSE,
  diagnosticTried |-> FALSE, dropped |-> FALSE, diagnosticFailed |-> FALSE,
  decision |-> [made |-> FALSE, checks |-> [k \in {"shape","grant","scope","writable",
    "capacity","capability","revision","context"} |-> TRUE], identity |-> FALSE,
    before |-> InitialRoot, after |-> InitialRoot, foreignStage |-> 0,
    diagnosticFailed |-> FALSE], tick |-> 0]
Prepare == /\ s.phase="New"
  /\ s'=Advance([s EXCEPT !.phase="Prepared",
    !.capture=[root |-> s.root, context |-> s.context, candidate |-> Candidate(s.root)]])
Invalidate(key) == /\ key \in {"grant","scope","writable","capacity","capability"}
  /\ s[key] /\ s'=Advance([s EXCEPT ![key]=FALSE])
ContextChange == /\ s.context=0 /\ s'=Advance([s EXCEPT !.context=1])
ForeignEdit == /\ s.foreignStage=0
  /\ s'=Advance([s EXCEPT !.foreignStage=1,
    !.root=[@ EXCEPT !.gen=@+1, !.rev=@+1, !.text= <<"b","b","a">>]])
ForeignUndo == /\ s.foreignStage=1
  /\ s'=Advance([s EXCEPT !.foreignStage=2,
    !.root=[@ EXCEPT !.gen=@+1, !.rev=@+1, !.text=Base]])
Unrelated == /\ s.root.other=0
  /\ s'=Advance([s EXCEPT !.root=[@ EXCEPT !.other=1, !.gen=@+1]])
Finish == /\ s.phase="Prepared"
  /\ LET eligible == All(Checks(s)) \/ SkipValidation
         noop == IF ForceChange THEN FALSE ELSE IF TextOnly THEN s.capture.candidate.text=s.root.text ELSE Identity(s)
         phase == IF ~eligible THEN "Rejected" ELSE IF noop THEN "Unchanged"
                  ELSE IF HasEffect THEN "EffectRequested" ELSE "Applied"
         root == IF phase="Unchanged" THEN
                   IF RestoreCaptured THEN s.capture.root
                   ELSE IF BookkeepNoop THEN [s.root EXCEPT !.gen=@+1, !.rev=@+1, !.history=@+1]
                   ELSE s.root
                 ELSE IF phase="Applied" THEN Publish(s.root,s.capture.candidate) ELSE s.root
     IN s'=Advance([s EXCEPT !.phase=phase, !.root=root,
       !.effectRequested=phase="EffectRequested",
       !.decision=[made |-> TRUE, checks |-> Checks(s), identity |-> Identity(s),
         before |-> s.root, after |-> root, foreignStage |-> s.foreignStage,
         diagnosticFailed |-> s.diagnosticFailed]])
Cancel == /\ s.phase="Prepared" /\ s'=Advance([s EXCEPT !.phase="Stopped"])
LateCancel == /\ s.phase \in Terminal /\ ~s.lateCancel
  /\ s'=Advance([s EXCEPT !.lateCancel=TRUE])
Diagnostic == /\ DiagnosticMode # 0 /\ ~s.diagnosticTried
  /\ s'=Advance([s EXCEPT !.diagnosticTried=TRUE,
    !.dropped=DiagnosticMode=2, !.diagnosticFailed=DiagnosticMode=3,
    !.root=IF DiagnosticWrites THEN [@ EXCEPT !.gen=@+1] ELSE @,
    !.phase=IF DiagnosticRejects /\ DiagnosticMode=3 /\ s.phase \in {"Unchanged","Applied"}
      THEN "Rejected" ELSE @])
Next == /\ s.tick<Bound
  /\ (Prepare \/ Finish \/ Cancel \/ LateCancel \/ ContextChange \/ ForeignEdit
    \/ ForeignUndo \/ Unrelated \/ Diagnostic
    \/ (\E key \in {"grant","scope","writable","capacity","capability"}: Invalidate(key)))
Spec == Init /\ [][Next]_s
TypeOK == /\ Kind \in 0..8 /\ DiagnosticMode \in 0..3 /\ s.tick \in 0..Bound
  /\ s.phase \in {"New","Prepared"} \cup Terminal
  /\ s.foreignStage \in 0..2 /\ s.root.point \in -1..3
  /\ s.root.gen \in Nat /\ s.root.rev \in Nat /\ s.root.history \in Nat
SuccessValidated == s.decision.made /\ s.phase \in {"Unchanged","Applied","EffectRequested"}
  => All(s.decision.checks)
NoopMeaning == s.decision.made /\ s.phase="Unchanged" => s.decision.identity
NoopComplete == s.decision.made /\ All(s.decision.checks) /\ s.decision.identity
  => s.phase="Unchanged"
NoopFrame == [][(s.phase="Prepared" /\ s'.phase="Unchanged") =>
  (s'.root=s.root /\ s'.effectRequested=s.effectRequested)]_s
RejectedFrame == [][(s.phase="Prepared" /\ s'.phase \in {"Rejected","Stopped"}) =>
  (s'.root=s.root /\ s'.effectRequested=s.effectRequested)]_s
CandidatePublished == [][(s.phase="Prepared" /\ s'.phase="Applied") =>
  (s'.root=Publish(s.root,s.capture.candidate) /\ s'.root.other=s.root.other)]_s
DiagnosticIsolation == [][(~s.diagnosticTried /\ s'.diagnosticTried) =>
  (s'.root=s.root /\ s'.phase=s.phase /\ s'.capture=s.capture /\
   s'.decision=s.decision /\ s'.effectRequested=s.effectRequested)]_s
TerminalStable == [][(s.phase \in Terminal) => s'.phase=s.phase]_s
OnlyFailed(checks,key) == ~checks[key] /\ \A k \in DOMAIN checks \ {key}: checks[k]
NoopRejected(key) == s.phase="Rejected" /\ s.decision.identity
  /\ OnlyFailed(s.decision.checks,key)
EmptyUnchanged == Kind=0 /\ s.phase="Unchanged"
SameTextUnchanged == Kind=1 /\ s.phase="Unchanged"
MappedApplied == Kind=2 /\ s.phase="Applied"
  /\ s.decision.before.text=s.decision.after.text /\ s.decision.after.point=-1
SelectionApplied == Kind=3 /\ s.phase="Applied" /\ s.decision.after.point=2
AnnotationApplied == Kind=4 /\ s.phase="Applied" /\ s.decision.after.body="new comment"
HistoryApplied == Kind=5 /\ s.phase="Applied" /\ ~s.decision.after.groupOpen
TextApplied == Kind=6 /\ s.phase="Applied" /\ s.decision.before.text # s.decision.after.text
EffectPreserved == Kind=7 /\ s.phase="EffectRequested" /\ s.effectRequested
MalformedRejected == Kind=8 /\ s.phase="Rejected" /\ ~s.decision.checks.shape
GrantNoopRejected == NoopRejected("grant")
ScopeNoopRejected == NoopRejected("scope")
ReadonlyNoopRejected == NoopRejected("writable")
CapacityNoopRejected == NoopRejected("capacity")
CapabilityNoopRejected == NoopRejected("capability")
ContextNoopRejected == NoopRejected("context")
StaleEqualNoopRejected == NoopRejected("revision") /\ s.decision.foreignStage=2
CancelledBeforeNoop == s.phase="Stopped" /\ ~s.decision.made
CancelledAfterNoop == s.phase="Unchanged" /\ s.lateCancel
UnrelatedBeforeNoop == s.phase="Unchanged" /\ s.capture.root.other=0
  /\ s.decision.before.other=1 /\ s.decision.after=s.decision.before
PostNoopFault == s.phase="Unchanged" /\ ~s.decision.diagnosticFailed /\ s.diagnosticFailed
NotEmptyUnchanged == ~EmptyUnchanged
NotSameTextUnchanged == ~SameTextUnchanged
NotMappedApplied == ~MappedApplied
NotSelectionApplied == ~SelectionApplied
NotAnnotationApplied == ~AnnotationApplied
NotHistoryApplied == ~HistoryApplied
NotTextApplied == ~TextApplied
NotEffectPreserved == ~EffectPreserved
NotMalformedRejected == ~MalformedRejected
NotGrantNoopRejected == ~GrantNoopRejected
NotScopeNoopRejected == ~ScopeNoopRejected
NotReadonlyNoopRejected == ~ReadonlyNoopRejected
NotCapacityNoopRejected == ~CapacityNoopRejected
NotCapabilityNoopRejected == ~CapabilityNoopRejected
NotContextNoopRejected == ~ContextNoopRejected
NotStaleEqualNoopRejected == ~StaleEqualNoopRejected
NotCancelledBeforeNoop == ~CancelledBeforeNoop
NotCancelledAfterNoop == ~CancelledAfterNoop
NotUnrelatedBeforeNoop == ~UnrelatedBeforeNoop
NotPostNoopFault == ~PostNoopFault
=============================================================================
