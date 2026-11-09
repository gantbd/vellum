--------------------------- MODULE Transactions ---------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC
CONSTANTS Bound, LeakStage
Docs == {"A", "B", "R"}
Ops == {"X", "Y"}
Contexts == {"Membership", "Config", "Selection", "Binding", "Incarnation"}
Writes(o) == IF o = "X" THEN {"A", "B"} ELSE IF o = "Y" THEN {"B"} ELSE {"R"}
Reads(o) == IF o = "X" THEN Docs ELSE {"B"}
Value(o) == IF o = "X" THEN 1 ELSE IF o = "Y" THEN 2 ELSE 3
InitialRoot == [gen |-> 0, rev |-> [d \in Docs |-> 0], text |-> [d \in Docs |-> 0]]
Apply(r, o) == [gen |-> r.gen + 1,
  rev |-> [d \in Docs |-> r.rev[d] + IF d \in Writes(o) THEN 1 ELSE 0],
  text |-> [d \in Docs |-> IF d \in Writes(o) THEN Value(o) ELSE r.text[d]]]
RECURSIVE Replay(_)
Replay(xs) == IF Len(xs) = 0 THEN InitialRoot
              ELSE Apply(Replay(SubSeq(xs, 1, Len(xs)-1)), xs[Len(xs)])
VARIABLE s
Advance(t) == [t EXCEPT !.tick = s.tick + 1]
Modes == {"Off", "OK", "Full", "Failed"}
CaptureState(t,mode) == [t EXCEPT !.diagnostic=mode]
Authoritative(t) == [root |-> t.root, log |-> t.log, phase |-> t.phase,
  built |-> t.built, expected |-> t.expected, captured |-> t.captured,
  context |-> t.context, grant |-> t.grant, reason |-> t.reason,
  observed |-> t.observed, readChanged |-> t.readChanged]
Init == s = [root |-> InitialRoot, log |-> <<>>, published |-> {InitialRoot},
  phase |-> [o \in Ops |-> "New"], built |-> [o \in Ops |-> {}],
  expected |-> [o \in Ops |-> InitialRoot.rev],
  captured |-> [o \in Ops |-> [c \in Contexts |-> 0]],
  context |-> [c \in Contexts |-> 0], grant |-> [o \in Ops |-> TRUE],
  reason |-> [o \in Ops |-> "None"], observed |-> InitialRoot,
  readChanged |-> FALSE, diagnostic |-> "Off", tick |-> 0]
Valid(o) == s.grant[o] /\ s.captured[o] = s.context /\
            \A d \in Reads(o): s.expected[o][d] = s.root.rev[d]
RejectReason(o) == IF ~s.grant[o] THEN "Denied"
                  ELSE IF s.captured[o] # s.context THEN "Context" ELSE "Revision"
Prepare(o) == /\ s.phase[o] = "New"
  /\ s' = Advance([s EXCEPT !.phase[o] = "Building", !.expected[o] = s.root.rev,
                    !.captured[o] = s.context])
Build(o, d) == /\ s.phase[o] = "Building" /\ d \in Writes(o) \ s.built[o]
  /\ s' = Advance([s EXCEPT !.built[o] = @ \cup {d},
        !.phase[o] = IF s.built[o] \cup {d} = Writes(o) THEN "Ready" ELSE "Building"])
Commit(o) == /\ s.phase[o] = "Ready"
  /\ IF Valid(o) THEN
       s' = Advance([s EXCEPT !.root = Apply(@, o), !.log = Append(@, o),
                      !.published = @ \cup {Apply(s.root, o)}, !.phase[o] = "Committed"])
     ELSE s' = Advance([s EXCEPT !.phase[o] = "Rejected", !.reason[o] = RejectReason(o)])
Stop(o) == /\ s.phase[o] \in {"New", "Building", "Ready"}
  /\ s' = Advance([s EXCEPT !.phase[o] = "Stopped"])
Revoke(o) == /\ s.grant[o] /\ s' = Advance([s EXCEPT !.grant[o] = FALSE])
Invalidate(c) == /\ s.context[c] = 0 /\ s' = Advance([s EXCEPT !.context[c] = 1])
ChangeRead == /\ ~s.readChanged
  /\ s' = Advance([s EXCEPT !.root = Apply(@, "R"), !.log = Append(@, "R"),
      !.published = @ \cup {Apply(s.root, "R")}, !.readChanged = TRUE])
Observe == s' = Advance([s EXCEPT !.observed =
  IF LeakStage /\ s.built["X"] = {"A"}
    THEN [s.root EXCEPT !.text["A"] = 1] ELSE s.root])
Capture(mode) == s' = Advance(CaptureState(s,mode))
BaseNext == (\E o \in Ops: Prepare(o) \/ Commit(o) \/ Stop(o) \/ Revoke(o))
  \/ (\E o \in Ops, d \in Docs: Build(o,d))
  \/ (\E c \in Contexts: Invalidate(c)) \/ ChangeRead \/ Observe
\* Diagnostic capture is a separate bounded field; no callback or required I/O.
Next == /\ s.tick < Bound
        /\ BaseNext \/ (\E mode \in Modes: Capture(mode))
Spec == Init /\ [][Next]_s
SerialRoot == s.root = Replay(s.log)
NoPartialObservation == s.observed \in s.published
DiagnosticIsolation == \A mode \in Modes: Authoritative(CaptureState(s,mode))=Authoritative(s)
TypeOK == /\ s.tick \in 0..Bound /\ s.root.gen = Len(s.log)
          /\ \A o \in Ops: s.phase[o] \in {"New","Building","Ready","Committed","Rejected","Stopped"}
PublishValidated == [][(s'.root # s.root /\ Len(s'.log)>Len(s.log)) =>
  (LET o == s'.log[Len(s'.log)] IN o = "R" \/ (s.phase[o] = "Ready" /\ Valid(o)))]_s
RejectedUnchanged == [][(\E o \in Ops: s.phase[o] # "Rejected" /\ s'.phase[o] = "Rejected")
                          => s'.root = s.root]_s
SuccessfulX == s.phase["X"] = "Committed"
BothCommit == \A o \in Ops: s.phase[o] = "Committed"
StaleLoser == s.phase["X"] = "Rejected" /\ s.phase["Y"] = "Committed"
ReadDependencyRejected == s.reason["X"] = "Revision" /\ s.readChanged /\ s.phase["Y"] # "Committed"
ContextRejected == s.reason["X"] = "Context"
DeniedRejected == s.reason["X"] = "Denied"
StoppedBeforePublish == s.phase["X"] = "Stopped" /\ "X" \notin {s.log[i]: i \in 1..Len(s.log)}
PartialStagePrivate == s.built["X"] = {"A"} /\ s.observed = InitialRoot
SuccessfulSaturated == SuccessfulX /\ s.diagnostic="Full"
SuccessfulFailedCapture == SuccessfulX /\ s.diagnostic="Failed"
NotSuccessfulX == ~SuccessfulX
NotBothCommit == ~BothCommit
NotStaleLoser == ~StaleLoser
NotReadDependencyRejected == ~ReadDependencyRejected
NotContextRejected == ~ContextRejected
NotDeniedRejected == ~DeniedRejected
NotStoppedBeforePublish == ~StoppedBeforePublish
NotPartialStagePrivate == ~PartialStagePrivate
NotSuccessfulSaturated == ~SuccessfulSaturated
NotSuccessfulFailedCapture == ~SuccessfulFailedCapture
=============================================================================
