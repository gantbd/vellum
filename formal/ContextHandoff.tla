------------------------- MODULE ContextHandoff -------------------------
EXTENDS Integers, Sequences, FiniteSets, TLC
CONSTANTS Bound, CompatibleSemantics, IgnoreCompatibility, ReinterpretAtCommit,
          ForgetContext, RewriteCapture, IgnoreAuthority, IgnoreRevision,
          DiagnosticsOn, DiagnosticFull, DiagnosticFault, DiagnosticWrites, DiagnosticRejects
VARIABLE s

\* Reuse the actual finite text/mapping/inverse operators, NOT the complete
\* EditorContracts temporal relation. One document, one command, two contexts.
EC == INSTANCE EditorContracts WITH Bound <- Bound, SourceLimit <- 3,
  InsertLimit <- 2, BatchInsertLimit <- 1, UnsafeScope <- FALSE,
  SplitState <- FALSE, LeakRelease <- FALSE, SkipGap <- FALSE,
  BadMap <- FALSE, TrustDiscovery <- FALSE,
  IgnoreEventGrant <- FALSE, IgnoreEventRetention <- FALSE, s <- s
Base == <<"a","b","a">>
Resolve(c) == IF c=0 \/ CompatibleSemantics THEN EC!Edit
  ELSE [lo |-> 1, hi |-> 2, ins |-> <<"a">>]
Compatible(a,b) == Resolve(a)=Resolve(b)
Candidate(r,e) == [revision |-> r.revision+1, text |-> EC!Apply(r.text,e),
  point |-> EC!MapPoint(r.point,e),
  history |-> Append(r.history,EC!Inverse(r.text,e)), owner |-> r.owner]
InitialRoot == [revision |-> 0, text |-> Base, point |-> 2,
  history |-> <<>>, owner |-> "local"]
InitialCapture == [context |-> 0, cosmetic |-> 0, revision |-> 0,
  edit |-> Resolve(0), candidate |-> Candidate(InitialRoot,Resolve(0))]
Advance(t) == [t EXCEPT !.tick=s.tick+1]
Init == s=[root |-> InitialRoot, active |-> 0, cosmetic |-> 0,
  retained |-> {0}, grant |-> TRUE, phase |-> "New", capture |-> InitialCapture,
  published |-> FALSE, usedEdit |-> Resolve(0), publishGrant |-> FALSE,
  publishRevision |-> 0, publishContext |-> 0, publishCosmetic |-> 0,
  publishDiagnosticFailed |-> FALSE,
  rejectCompatibility |-> FALSE, rejectRevision |-> FALSE, rejectGrant |-> FALSE,
  diagnosticTried |-> FALSE, dropped |-> FALSE, diagnosticFailed |-> FALSE,
  tick |-> 0]
Prepare == /\ s.phase="New" /\ s.grant
  \* Text-changing profile only; adopted D01 identity is modeled separately in PS-NO.
  /\ Candidate(s.root,Resolve(s.active)).text # s.root.text
  /\ s'=Advance([s EXCEPT !.phase="Prepared",
    !.capture=[context |-> s.active, cosmetic |-> s.cosmetic,
      revision |-> s.root.revision, edit |-> Resolve(s.active),
      candidate |-> Candidate(s.root,Resolve(s.active))]])
Activate == /\ s.active=0 /\ s.grant
  /\ s'=Advance([s EXCEPT !.active=1, !.retained=@ \cup {1},
    !.capture=IF RewriteCapture /\ s.phase="Prepared"
      THEN [@ EXCEPT !.context=1] ELSE @])
Cosmetic == /\ s.cosmetic=0 /\ s'=Advance([s EXCEPT !.cosmetic=1])
Retire == /\ s.active=1 /\ 0 \in s.retained
  /\ (s.phase # "Prepared" \/ s.capture.context # 0 \/ ForgetContext)
  /\ s'=Advance([s EXCEPT !.retained=@ \ {0}])
Revoke == /\ s.grant /\ s'=Advance([s EXCEPT !.grant=FALSE])
ForeignEdit == /\ ~s.published /\ s.root.revision=0
  /\ s'=Advance([s EXCEPT !.root=Candidate(@,EC!Edit)])
Cancel == /\ s.phase="Prepared" /\ s'=Advance([s EXCEPT !.phase="Stopped"])
Eligible == /\ (s.grant \/ IgnoreAuthority)
  /\ (s.capture.revision=s.root.revision \/ IgnoreRevision)
  /\ (Compatible(s.capture.context,s.active) \/ IgnoreCompatibility)
  /\ s.capture.context \in s.retained
Commit == /\ s.phase="Prepared"
  /\ IF Eligible THEN
      s'=Advance([s EXCEPT !.phase="Committed", !.published=TRUE,
        !.root=IF ReinterpretAtCommit THEN Candidate(@,Resolve(s.active)) ELSE s.capture.candidate,
        !.usedEdit=IF ReinterpretAtCommit THEN Resolve(s.active) ELSE s.capture.edit,
        !.publishGrant=s.grant, !.publishRevision=s.root.revision,
        !.publishContext=s.active, !.publishCosmetic=s.cosmetic,
        !.publishDiagnosticFailed=s.diagnosticFailed])
     ELSE s'=Advance([s EXCEPT !.phase="Rejected",
       !.rejectCompatibility=~Compatible(s.capture.context,s.active),
       !.rejectRevision=s.capture.revision # s.root.revision,
       !.rejectGrant=~s.grant])
Diagnostic == /\ DiagnosticsOn /\ ~s.diagnosticTried
  /\ s'=Advance([s EXCEPT !.diagnosticTried=TRUE,
    !.dropped=DiagnosticFull, !.diagnosticFailed=DiagnosticFault,
    !.phase=IF DiagnosticRejects /\ DiagnosticFault /\ s.phase="Committed"
      THEN "Rejected" ELSE @,
    !.root=IF DiagnosticWrites THEN [@ EXCEPT !.text= <<"b">>] ELSE @])
Next == /\ s.tick<Bound
  /\ (Prepare \/ Activate \/ Cosmetic \/ Retire \/ Revoke \/ ForeignEdit
      \/ Cancel \/ Commit \/ Diagnostic)
Spec == Init /\ [][Next]_s
TypeOK == /\ s.tick \in 0..Bound /\ s.active \in {0,1}
  /\ s.cosmetic \in {0,1} /\ s.root.revision \in 0..2
  /\ s.phase \in {"New","Prepared","Committed","Rejected","Stopped"}
  /\ s.retained \subseteq {0,1}
NoDanglingContext == /\ s.active \in s.retained
  /\ (s.phase="Prepared" => s.capture.context \in s.retained)
CaptureConsistent == s.phase # "New" => s.capture.edit=Resolve(s.capture.context)
PublishedAuthority == s.published => s.publishGrant
PublishedRevision == s.published => s.publishRevision=s.capture.revision
PublishedCompatibility == s.published => Compatible(s.capture.context,s.publishContext)
CapturedMeaning == s.published => s.usedEdit=s.capture.edit
AtomicCandidate == s.published => s.root=s.capture.candidate
OwnerStable == s.root.owner="local"
CaptureStable == [][(s.phase="Prepared" /\ s'.phase="Prepared") => s'.capture=s.capture]_s
DiagnosticIsolation == [][(~s.diagnosticTried /\ s'.diagnosticTried) =>
  (s'.root=s.root /\ s'.phase=s.phase /\ s'.capture=s.capture /\ s'.grant=s.grant)]_s
RejectedUnchanged == [][(s.phase="Prepared" /\ s'.phase="Rejected") => s'.root=s.root]_s
TerminalStable == [][(s.phase \in {"Committed","Rejected","Stopped"}) => s'.phase=s.phase]_s
FreshApplied == s.published /\ s.capture.context=0 /\ s.publishContext=0
CosmeticApplied == s.published /\ s.capture.cosmetic=0 /\ s.publishCosmetic=1
CompatibleChangedApplied == CompatibleSemantics /\ s.published
  /\ s.capture.context=0 /\ s.publishContext=1
IncompatibleRefused == s.phase="Rejected" /\ s.rejectCompatibility
StaleRefused == s.phase="Rejected" /\ s.rejectRevision
RevokedRefused == s.phase="Rejected" /\ s.rejectGrant
HeldAcrossActivation == s.phase="Prepared" /\ s.capture.context=0
  /\ s.active=1 /\ 0 \in s.retained
CancelledNoPublication == s.phase="Stopped" /\ ~s.published
DiagnosticDrop == s.diagnosticTried /\ s.dropped
DiagnosticFailure == s.diagnosticTried /\ s.diagnosticFailed
DiagnosticCaptured == s.diagnosticTried /\ ~s.dropped /\ ~s.diagnosticFailed
PostCommitFault == s.published /\ ~s.publishDiagnosticFailed /\ s.diagnosticFailed
NotFreshApplied == ~FreshApplied
NotCosmeticApplied == ~CosmeticApplied
NotCompatibleChangedApplied == ~CompatibleChangedApplied
NotIncompatibleRefused == ~IncompatibleRefused
NotStaleRefused == ~StaleRefused
NotRevokedRefused == ~RevokedRefused
NotHeldAcrossActivation == ~HeldAcrossActivation
NotCancelledNoPublication == ~CancelledNoPublication
NotDiagnosticDrop == ~DiagnosticDrop
NotDiagnosticFailure == ~DiagnosticFailure
NotDiagnosticCaptured == ~DiagnosticCaptured
NotPostCommitFault == ~PostCommitFault
=============================================================================
