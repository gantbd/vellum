--------------------------- MODULE Disclosure ----------------------------
EXTENDS Naturals, Sequences, FiniteSets, TLC
CONSTANTS Bound, LeakCache
Ids == {"Visible","Hidden"}
World(h) == [body |-> [a \in Ids |-> IF a="Hidden" THEN h ELSE 0],
  version |-> [a \in Ids |-> 0], present |-> [a \in Ids |-> TRUE],
  attached |-> [a \in Ids |-> TRUE]]
Projection(w,rev,rich) == [sourceRevision |-> rev,
  records |-> {[id |-> a, body |-> w.body[a], version |-> w.version[a], attached |-> w.attached[a]]:
    a \in {a \in Ids: w.present[a] /\ (a="Visible" \/ rich)}},
  count |-> Cardinality({a \in Ids: w.present[a] /\ (a="Visible" \/ rich)})]
Envelope(kind,payload) == [kind |-> kind, payload |-> payload]
EmptyProjection == [sourceRevision |-> 0, records |-> {}, count |-> 0]
Unavailable == Envelope("Unavailable",EmptyProjection)
VARIABLE s
Advance(t) == [t EXCEPT !.tick=s.tick+1]
Init == s = [left |-> World(0), right |-> World(1), source |-> 0,
  grant |-> TRUE, epoch |-> 0, disposed |-> FALSE, queued |-> FALSE, queueEpoch |-> 0,
  outputL |-> <<>>, outputR |-> <<>>, last |-> "Init", hadRevocation |-> FALSE,
  queueDenied |-> FALSE, released |-> FALSE, tick |-> 0]
Output(w) == IF s.grant /\ ~s.disposed THEN Envelope("Data",Projection(w,s.source,LeakCache)) ELSE Unavailable
HiddenBody(side) == /\ s[side].version["Hidden"]<2
  /\ s' = Advance([s EXCEPT ![side].body["Hidden"] = 1-@,
    ![side].version["Hidden"]=@+1, !.last="Hidden"])
HiddenExistence(side) == s' = Advance([s EXCEPT ![side].present["Hidden"]=~@, !.last="Hidden"])
VisibleBody == /\ s.left.version["Visible"]<2
  /\ s' = Advance([s EXCEPT !.left.body["Visible"]=1-@, !.right.body["Visible"]=1-@,
    !.left.version["Visible"]=@+1, !.right.version["Visible"]=@+1, !.last="Visible"])
DeleteTarget == /\ s.source=0
  /\ s' = Advance([s EXCEPT !.source=1,
    !.left.attached=[a \in Ids |-> FALSE], !.right.attached=[a \in Ids |-> FALSE], !.last="Delete"])
Request == /\ ~s.queued /\ s.grant /\ ~s.disposed
  /\ s' = Advance([s EXCEPT !.queued=TRUE, !.queueEpoch=s.epoch, !.last="Request"])
ReleaseAllowed == s.grant /\ ~s.disposed /\ s.queueEpoch=s.epoch
Release == /\ s.queued
  /\ s' = Advance([s EXCEPT !.queued=FALSE,
    !.outputL=Append(@,IF ReleaseAllowed THEN Output(s.left) ELSE Unavailable),
    !.outputR=Append(@,IF ReleaseAllowed THEN Output(s.right) ELSE Unavailable),
    !.queueDenied=@ \/ ~ReleaseAllowed, !.released=@ \/ ReleaseAllowed, !.last="Release"])
Query == s' = Advance([s EXCEPT !.outputL=Append(@,Output(s.left)),
  !.outputR=Append(@,Output(s.right)), !.last="Query"])
\* Hidden existence is not disclosable: both internal causes share one projection.
ProbeHidden == s' = Advance([s EXCEPT !.outputL=Append(@,Unavailable),
  !.outputR=Append(@,Unavailable), !.last="Probe"])
Diagnostic == s' = Advance([s EXCEPT !.outputL=Append(@,Envelope("Health",EmptyProjection)),
  !.outputR=Append(@,Envelope("Health",EmptyProjection)), !.last="Diagnostic"])
Revoke == /\ s.grant /\ s.epoch=0
  /\ s' = Advance([s EXCEPT !.grant=FALSE, !.epoch=1, !.hadRevocation=TRUE, !.last="Revoke"])
Regrant == /\ ~s.grant /\ s.epoch=1 /\ ~s.disposed
  /\ s' = Advance([s EXCEPT !.grant=TRUE, !.epoch=2, !.last="Regrant"])
Dispose == /\ ~s.disposed /\ s' = Advance([s EXCEPT !.disposed=TRUE, !.last="Dispose"])
Next == /\ s.tick<Bound
  /\ ((\E side \in {"left","right"}: HiddenBody(side) \/ HiddenExistence(side))
     \/ VisibleBody \/ DeleteTarget \/ Request \/ Release \/ Query \/ ProbeHidden
     \/ Diagnostic \/ Revoke \/ Regrant \/ Dispose)
Spec == Init /\ [][Next]_s
FunctionalNoninterference == s.outputL=s.outputR
EqualPublicState == Projection(s.left,s.source,FALSE)=Projection(s.right,s.source,FALSE)
BodySurvivesDeletion == [][s'.last="Delete" =>
  /\ s'.left.body=s.left.body /\ s'.right.body=s.right.body
  /\ s'.left.version=s.left.version /\ s'.right.version=s.right.version]_s
HiddenPreservesSource == [][s'.last="Hidden" => s'.source=s.source]_s
NoUnauthorizedRelease == [][s'.last="Release" /\ Len(s'.outputL)>Len(s.outputL) =>
  (s'.outputL[Len(s'.outputL)].kind = "Data" => ReleaseAllowed)]_s
SuccessfulRelease == s.released
RevokedQueuedDenied == s.queueDenied /\ s.epoch=1
RegrantGapDenied == s.queueDenied /\ s.epoch=2 /\ ~s.released
DetachedBodyRetained == s.source=1 /\ ~s.left.attached["Visible"] /\ s.left.present["Visible"]
HiddenDifferenceUnobservable == s.left.body["Hidden"] # s.right.body["Hidden"] /\
  Len(s.outputL)>0 /\ s.outputL=s.outputR
NotSuccessfulRelease == ~SuccessfulRelease
NotRevokedQueuedDenied == ~RevokedQueuedDenied
NotRegrantGapDenied == ~RegrantGapDenied
NotDetachedBodyRetained == ~DetachedBodyRetained
NotHiddenDifferenceUnobservable == ~HiddenDifferenceUnobservable
=============================================================================
