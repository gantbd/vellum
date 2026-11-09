------------------------------ MODULE Bridges -----------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC
CONSTANTS Bound, Fuel, SplitHistory, RestoreBody, LeakCapture, LeakProjection, UnchargedBuild
Documents == {"A","B"}
Ops == {"Edit","Comment","Undo","Save","Read"}
Verb(o) == IF o="Comment" THEN "Annotate" ELSE o
Absent == [absent |-> TRUE]
InitialRoot == [gen |-> 0, text |-> [d \in Documents |-> 0], rev |-> [d \in Documents |-> 0],
  undo |-> [d \in Documents |-> FALSE], redo |-> [d \in Documents |-> FALSE],
  annotation |-> [body |-> 0, version |-> 0, target |-> "Located", mappedRevision |-> 0]]
VARIABLE s
K == INSTANCE Execution WITH Docs <- Documents, MaxFuel <- Fuel,
  RecheckContext <- TRUE, HonorFuel <- TRUE, state <- s.control["Edit"]
KernelInitial(o) == [phase |-> "New", verb |-> Verb(o), fuel |-> Fuel,
  granted |-> TRUE, current |-> TRUE, applied |-> FALSE, dispatched |-> FALSE,
  released |-> FALSE, pulse |-> FALSE, contents |-> K!Before, evidence |-> {}]
RECURSIVE Run(_, _)
Run(k,es) == IF Len(es)=0 THEN k ELSE Run(K!Step(k,es[1]),Tail(es))
Advance(t,o,es,label) == [t EXCEPT !.tick=s.tick+1, !.actor=o, !.events=es, !.last=label]
Source(r) == [document |-> "A", incarnation |-> 1, binding |-> 0, profile |-> 0,
              revision |-> r.rev["A"], payload |-> r.text["A"]]
Public(r) == [source |-> r.text, revision |-> r.rev, annotationCount |-> 0,
  token |-> [text |-> r.text, rev |-> r.rev], ownerGeneration |-> IF LeakProjection THEN r.gen ELSE 0]
Envelope(kind,r) == [kind |-> kind, data |-> Public(r)]
Init == s = [root |-> InitialRoot, published |-> {InitialRoot}, beforeGroup |-> InitialRoot,
  prepared |-> [o \in Ops |-> InitialRoot], built |-> {},
  control |-> [o \in Ops |-> KernelInitial(o)], capture |-> Absent, effect |-> Absent,
  readResult |-> Envelope("Unavailable",InitialRoot), readAttempted |-> FALSE,
  revoked |-> FALSE, tick |-> 0, actor |-> "None", events |-> <<>>, last |-> "Init"]
Admit(o) == /\ s.control[o].phase="New"
  /\ s' = Advance([s EXCEPT !.control[o]=K!Step(@,"Admit")],o,<<"Admit">>,"Admit")
Prepare(o) == /\ s.control[o].phase="Admitted"
  /\ LET k == K!Step(s.control[o],"Prepare")
         r == IF LeakCapture /\ "A" \in s.built /\ ~s.control["Edit"].applied
                THEN [s.root EXCEPT !.text["A"]=1] ELSE s.root
     IN s' = Advance([s EXCEPT !.control[o]=k, !.prepared[o]=s.root,
       !.capture=IF o="Save" /\ k.phase="Prepared" THEN Source(r) ELSE @],o,<<"Prepare">>,"Prepare")
Build(d) == /\ s.control["Edit"].phase="Prepared" /\ d \notin s.built
  /\ s' = Advance([s EXCEPT !.built=@ \cup {d},
       !.control["Edit"]=IF UnchargedBuild THEN @ ELSE K!Step(@,"Tick")],"Edit",<<"Tick">>,"Build")
Valid(o) == CASE o="Edit" -> s.prepared[o].rev=s.root.rev
  [] o="Undo" -> (\A d \in Documents: s.root.undo[d]) /\ s.prepared[o].rev=s.root.rev
  [] OTHER -> s.prepared[o].annotation.version=s.root.annotation.version
EditRoot(r) == [r EXCEPT !.gen=@+1, !.text=[d \in Documents |-> 1],
  !.rev=[d \in Documents |-> r.rev[d]+1],
  !.undo=[d \in Documents |-> ~(SplitHistory /\ d="B")], !.redo=[d \in Documents |-> FALSE],
  !.annotation.target="Detached", !.annotation.mappedRevision=r.rev["A"]+1]
CommentRoot(r) == [r EXCEPT !.gen=@+1, !.annotation.body=1, !.annotation.version=@+1]
UndoRoot(r) == [r EXCEPT !.gen=@+1, !.text=[d \in Documents |-> 0],
  !.rev=[d \in Documents |-> r.rev[d]+1], !.undo=[d \in Documents |-> FALSE],
  !.redo=[d \in Documents |-> TRUE],
  !.annotation=[(IF RestoreBody THEN s.beforeGroup.annotation ELSE r.annotation)
    EXCEPT !.target="Detached", !.mappedRevision=r.rev["A"]+1]]
Commit(o) == /\ o \in {"Edit","Comment","Undo"} /\ s.control[o].phase="Prepared"
  /\ (o # "Edit" \/ s.built=Documents)
  /\ LET es == IF Valid(o) THEN <<"Commit">> ELSE <<"DependencyChanged","Commit">>
         k == Run(s.control[o],es)
         applied == k.applied /\ ~s.control[o].applied
         r == IF ~applied THEN s.root ELSE CASE o="Edit" -> EditRoot(s.root)
              [] o="Comment" -> CommentRoot(s.root) [] OTHER -> UndoRoot(s.root)
     IN s' = Advance([s EXCEPT !.control[o]=k, !.root=r,
        !.published=@ \cup {r}, !.beforeGroup=IF applied /\ o="Edit" THEN s.root ELSE @],
        o,es,IF ~applied THEN "Rejected" ELSE CASE o="Edit" -> "EditPublish"
               [] o="Comment" -> "CommentPublish" [] OTHER -> "UndoPublish")
Dispatch == /\ s.control["Save"].phase="Prepared"
  /\ LET k == K!Step(s.control["Save"],"Dispatch") IN
     s' = Advance([s EXCEPT !.control["Save"]=k,
       !.effect=IF k.dispatched THEN s.capture ELSE @],"Save",<<"Dispatch">>,"Dispatch")
Receipt == /\ s.control["Save"].dispatched /\ "OK" \notin s.control["Save"].evidence
  /\ s' = Advance([s EXCEPT !.control["Save"]=K!Step(@,"ReceiptOK")],"Save",<<"ReceiptOK">>,"Receipt")
Release == /\ s.control["Read"].phase="Prepared"
  /\ LET k == K!Step(s.control["Read"],"Release") IN
     s' = Advance([s EXCEPT !.control["Read"]=k, !.readAttempted=TRUE,
       !.readResult=IF k.released THEN Envelope("Data",s.root) ELSE Envelope("Unavailable",InitialRoot)],
       "Read",<<"Release">>,"Release")
RevokeRead == /\ ~s.revoked
  /\ s' = Advance([s EXCEPT !.revoked=TRUE, !.control["Read"]=K!Step(@,"Revoke")],
                  "Read",<<"Revoke">>,"Revoke")
ControlEvent(o,e) == /\ ~K!Halted(s.control[o])
  /\ s' = Advance([s EXCEPT !.control[o]=K!Step(@,e)],o,<<e>>,e)
Next == /\ s.tick<Bound
  /\ ((\E o \in Ops: Admit(o) \/ Prepare(o) \/ Commit(o)
        \/ (\E e \in {"Tick","Cancel","Surprise"}: ControlEvent(o,e)))
      \/ (\E d \in Documents: Build(d)) \/ Dispatch \/ Receipt \/ Release \/ RevokeRead)
Spec == Init /\ [][Next]_s
\* Scripted witnesses are subsets of the SAME transition relation, not a second model.
BodyScript == CASE s.tick=0 -> Admit("Edit") [] s.tick=1 -> Prepare("Edit")
  [] s.tick=2 -> Build("A") [] s.tick=3 -> Build("B") [] s.tick=4 -> Commit("Edit")
  [] s.tick=5 -> Admit("Comment") [] s.tick=6 -> Prepare("Comment") [] s.tick=7 -> Commit("Comment")
  [] s.tick=8 -> Admit("Undo") [] s.tick=9 -> Prepare("Undo") [] s.tick=10 -> Commit("Undo")
  [] OTHER -> FALSE
BodySpec == Init /\ [][Next /\ BodyScript]_s
SaveScript == CASE s.tick=0 -> Admit("Save") [] s.tick=1 -> Prepare("Save")
  [] s.tick=2 -> Admit("Edit") [] s.tick=3 -> Prepare("Edit")
  [] s.tick=4 -> Build("A") [] s.tick=5 -> Build("B") [] s.tick=6 -> Commit("Edit")
  [] s.tick=7 -> Dispatch [] OTHER -> FALSE
SaveSpec == Init /\ [][Next /\ SaveScript]_s
CaptureScript == CASE s.tick=0 -> Admit("Edit") [] s.tick=1 -> Prepare("Edit")
  [] s.tick=2 -> Build("A") [] s.tick=3 -> Build("B") [] s.tick=4 -> Commit("Edit")
  [] s.tick=5 -> Admit("Save") [] s.tick=6 -> Prepare("Save") [] OTHER -> FALSE
CaptureSpec == Init /\ [][Next /\ CaptureScript]_s
LateScript == CASE s.tick=0 -> Admit("Read") [] s.tick=1 -> Prepare("Read")
  [] s.tick=2 -> Admit("Save") [] s.tick=3 -> Prepare("Save") [] s.tick=4 -> Dispatch
  [] s.tick=5 -> ControlEvent("Save","Cancel") [] s.tick=6 -> RevokeRead
  [] s.tick=7 -> Release [] s.tick=8 -> Receipt [] OTHER -> FALSE
LateSpec == Init /\ [][Next /\ LateScript]_s
KernelTypes == \A o \in Ops: s.control[o] \in K!StateSpace
HistoryTextCoherent == \A d \in Documents:
  /\ (s.root.text[d]=1 <=> s.root.undo[d]) /\ ~(s.root.undo[d] /\ s.root.redo[d])
MappingCurrent == s.root.annotation.mappedRevision=s.root.rev["A"]
CapturePublished == s.capture=Absent \/ s.capture \in {Source(r): r \in s.published}
EffectCaptured == s.effect=Absent \/ s.effect=s.capture
ProjectionErasesHidden == \A a,b \in s.published:
  a.text=b.text /\ a.rev=b.rev => Public(a)=Public(b)
NoCaptureBeforePrepare == s.capture # Absent => s.control["Save"].phase # "New"
KernelRefinement == [][s'.actor \in Ops =>
  /\ s'.control[s'.actor]=Run(s.control[s'.actor],s'.events)
  /\ \A o \in Ops \ {s'.actor}: s'.control[o]=s.control[o]]_s
WorkCharged == [][s'.actor \in Ops /\ ~K!Halted(s.control[s'.actor]) =>
  K!Rank(s'.control[s'.actor])<K!Rank(s.control[s'.actor])]_s
SourcePreservesAuthoredBody == [][s'.last \in {"EditPublish","UndoPublish"} =>
  /\ s'.root.annotation.body=s.root.annotation.body
  /\ s'.root.annotation.version=s.root.annotation.version]_s
CommentPreservesSourceHistory == [][s'.last="CommentPublish" =>
  /\ s'.root.text=s.root.text /\ s'.root.rev=s.root.rev
  /\ s'.root.undo=s.root.undo /\ s'.root.redo=s.root.redo]_s
RejectedRootUnchanged == [][s'.last="Rejected" => s'.root=s.root]_s
CaptureImmutable == [][s.capture # Absent => s'.capture=s.capture]_s
ReleaseAtCurrentBoundary == [][s'.last="Release" /\ s'.readResult.kind="Data" =>
  /\ ~s.revoked /\ s.control["Read"].granted
  /\ s'.readResult.data=Public(s.root)]_s
TerminalAndEvidenceStable == [][\A o \in Ops:
  /\ (K!Halted(s.control[o]) => s'.control[o].phase=s.control[o].phase)
  /\ s.control[o].evidence \subseteq s'.control[o].evidence]_s
CoherentEdit == s.control["Edit"].applied /\ s.root.text["A"]=1 /\ s.root.undo["B"]
CommentSurvivesUndo == s.control["Undo"].applied /\ s.root.annotation.body=1
  /\ s.root.text["A"]=0 /\ s.root.rev["A"]=2 /\ s.root.redo["B"]
FrozenSaveDuringEdit == s.effect # Absent /\ s.effect.payload=0 /\ s.effect.revision=0
  /\ s.root.text["A"]=1 /\ s.root.rev["A"]=1
LateReceiptRetainedDenied == s.control["Save"].phase="Stopped"
  /\ "OK" \in s.control["Save"].evidence /\ s.revoked /\ s.readAttempted
  /\ s.readResult.kind="Unavailable"
BudgetStopped == s.control["Edit"].phase="Stopped" /\ s.control["Edit"].fuel=0 /\ ~s.control["Edit"].applied
SurpriseFaulted == s.control["Edit"].phase="Fault" /\ ~s.control["Edit"].applied
PublishedCapture == s.capture # Absent /\ s.capture.payload=1 /\ s.capture.revision=1
SafeRelease == s.readAttempted /\ s.readResult.kind="Data"
NotCoherentEdit == ~CoherentEdit
NotCommentSurvivesUndo == ~CommentSurvivesUndo
NotFrozenSaveDuringEdit == ~FrozenSaveDuringEdit
NotLateReceiptRetainedDenied == ~LateReceiptRetainedDenied
NotBudgetStopped == ~BudgetStopped
NotSurpriseFaulted == ~SurpriseFaulted
NotPublishedCapture == ~PublishedCapture
NotSafeRelease == ~SafeRelease
=============================================================================
