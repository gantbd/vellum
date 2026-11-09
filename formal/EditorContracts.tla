------------------------- MODULE EditorContracts -------------------------
EXTENDS Integers, Sequences, FiniteSets, TLC
CONSTANTS Bound, SourceLimit, InsertLimit, BatchInsertLimit,
          UnsafeScope, SplitState, LeakRelease, SkipGap, BadMap,
          TrustDiscovery, IgnoreEventGrant, IgnoreEventRetention
VARIABLE s

\* The alphabet is a representative valid-boundary domain, NOT all Unicode.
Alphabet == {"a", "b"}
Texts(n) == UNION { [1..k -> Alphabet] : k \in 0..n }
Prefix(t,n) == SubSeq(t,1,n)
Suffix(t,n) == SubSeq(t,n+1,Len(t))
Edits(t) == { e \in { [lo |-> l, hi |-> h, ins |-> x] :
  l \in 0..Len(t), h \in 0..Len(t), x \in Texts(InsertLimit) } : e.lo <= e.hi }
Apply(t,e) == Prefix(t,e.lo) \o e.ins \o Suffix(t,e.hi)
Inverse(t,e) == [lo |-> e.lo, hi |-> e.lo+Len(e.ins),
                 ins |-> SubSeq(t,e.lo+1,e.hi)]
\* Representative left-affinity point profile. Deleted interiors detach.
MapPoint(p,e) == IF p <= e.lo THEN p
  ELSE IF p >= e.hi THEN p+Len(e.ins)-(e.hi-e.lo)
  ELSE -1
Mapped(p,e) == IF BadMap THEN p ELSE MapPoint(p,e)
AlgebraInit == \E t \in Texts(SourceLimit): \E e \in Edits(t):
  \E p, cut \in 0..Len(t):
  s = [source |-> t, edit |-> e, point |-> p, cut |-> cut]
AlgebraSpec == AlgebraInit /\ [][FALSE]_s
SpliceLaw == LET t == s.source e == s.edit IN
  /\ Len(Apply(t,e)) = Len(t)-(e.hi-e.lo)+Len(e.ins)
  /\ Prefix(Apply(t,e),e.lo) = Prefix(t,e.lo)
  /\ Suffix(Apply(t,e),e.lo+Len(e.ins)) = Suffix(t,e.hi)
InverseLaw == Apply(Apply(s.source,s.edit),Inverse(s.source,s.edit))=s.source
PointLaw == LET p == s.point e == s.edit q == Mapped(p,e) IN
  /\ q \in {-1} \cup 0..Len(Apply(s.source,e))
  /\ ((e.lo < p /\ p < e.hi) => q = -1)
  /\ (p <= e.lo => q = p)
  /\ (p > e.lo /\ p >= e.hi => q = p+Len(e.ins)-(e.hi-e.lo))
ChunkLaw == LET t == s.source IN
  /\ Prefix(t,s.cut) \o Suffix(t,s.cut) = t
  /\ Apply(Prefix(t,s.cut) \o Suffix(t,s.cut),s.edit)=Apply(t,s.edit)
  /\ \A l \in 0..Len(t): \A h \in l..Len(t):
       SubSeq(Prefix(t,s.cut) \o Suffix(t,s.cut),l+1,h)=SubSeq(t,l+1,h)

\* Canonical start-snapshot batch, excluding ambiguous touching insertions.
Delta(e) == Len(e.ins)-(e.hi-e.lo)
Shift(e,n) == [e EXCEPT !.lo=@+n, !.hi=@+n]
BatchInit == \E t \in Texts(SourceLimit): \E e1, e2 \in Edits(t):
  /\ e1.hi < e2.lo /\ Len(e1.ins)<=BatchInsertLimit /\ Len(e2.ins)<=BatchInsertLimit
  /\ s=[source |-> t, first |-> e1, second |-> e2]
BatchSpec == BatchInit /\ [][FALSE]_s
BatchResult == Apply(Apply(s.source,s.second),s.first)
BatchComposition == BatchResult = Apply(Apply(s.source,s.first),
  Shift(s.second,IF BadMap THEN 0 ELSE Delta(s.first)))
BatchInverse == Apply(Apply(BatchResult,
  Inverse(Apply(s.source,s.second),s.first)),Inverse(s.source,s.second))=s.source

\* Actual finite source update drives text, attachment and conditional inverse.
Base == <<"a","b","a">>
Edit == [lo |-> 0, hi |-> 1, ins |-> <<"b">>]
InitialRoot == [rev |-> 0, text |-> Base, body |-> "authored",
  point |-> 2, history |-> <<>>, owner |-> "local"]
Publish(r) == [rev |-> r.rev+1, text |-> Apply(r.text,Edit),
  body |-> r.body, point |-> MapPoint(r.point,Edit),
  history |-> Append(r.history,Inverse(r.text,Edit)), owner |-> r.owner]
Advance(t) == [t EXCEPT !.tick=s.tick+1]
Init == s = [root |-> InitialRoot, frozen |-> InitialRoot,
  expected |-> 0, scope |-> {0}, phase |-> "New", staged |-> {},
  grant |-> TRUE, retained |-> TRUE, readGrant |-> TRUE,
  discovered |-> FALSE, accepted |-> FALSE, rejection |-> FALSE,
  delivered |-> FALSE, releasedGrant |-> FALSE, releasedRetained |-> FALSE,
  releasedRev |-> -1, gap |-> FALSE, needsSync |-> FALSE, synced |-> FALSE,
  eventApplied |-> FALSE, eventAfterSync |-> FALSE,
  eventGrant |-> FALSE, eventRetained |-> FALSE, tick |-> 0]
Components == {"source","attachment","history"}
Prepare == /\ s.phase="New"
  /\ s'=Advance([s EXCEPT !.phase="Building", !.expected=s.root.rev])
Stage(c) == /\ s.phase="Building" /\ c \in Components \ s.staged
  /\ s'=Advance([s EXCEPT !.staged=@ \cup {c},
       !.phase=IF s.staged \cup {c}=Components THEN "Ready" ELSE "Building"])
ScopeOK == Edit.lo \in s.scope
Valid == (s.grant \/ (TrustDiscovery /\ s.discovered)) /\
         s.expected=s.root.rev /\ (ScopeOK \/ UnsafeScope)
Commit == /\ s.phase="Ready"
  /\ IF Valid THEN
       s'=Advance([s EXCEPT !.root=IF SplitState THEN [@ EXCEPT !.text=Apply(@,Edit)]
                                 ELSE Publish(@),
            !.phase="Committed", !.accepted=TRUE])
      ELSE s'=Advance([s EXCEPT !.phase="Rejected", !.rejection=TRUE])
Narrow == /\ s.scope={0} /\ s'=Advance([s EXCEPT !.scope={}])
Revoke == /\ s.grant /\ s'=Advance([s EXCEPT !.grant=FALSE])
\* Focus is deliberately not a targeting input; source/context changes are.
Discover == /\ ~s.discovered
  /\ s'=Advance([s EXCEPT !.discovered=s.grant])
RevokeRead == /\ s.readGrant /\ s'=Advance([s EXCEPT !.readGrant=FALSE])
Evict == /\ s.retained /\ s'=Advance([s EXCEPT !.retained=FALSE])
Release == /\ s.phase="Committed" /\ ~s.delivered
  /\ ((s.readGrant /\ s.retained) \/ LeakRelease)
  /\ s'=Advance([s EXCEPT !.delivered=TRUE, !.releasedGrant=s.readGrant,
       !.releasedRetained=s.retained, !.releasedRev=s.root.rev])
Gap == /\ ~s.gap /\ s'=Advance([s EXCEPT !.gap=TRUE, !.needsSync=TRUE])
Resync == /\ s.needsSync /\ s.readGrant /\ s.retained
  /\ s'=Advance([s EXCEPT !.needsSync=FALSE, !.synced=TRUE])
Consume == /\ s.gap /\ ~s.eventApplied /\ (~s.needsSync \/ SkipGap)
  /\ (s.readGrant \/ IgnoreEventGrant) /\ (s.retained \/ IgnoreEventRetention)
  \* A new event release, not just local processing of already disclosed bytes.
  /\ s'=Advance([s EXCEPT !.eventApplied=TRUE, !.eventAfterSync=s.synced,
       !.eventGrant=s.readGrant, !.eventRetained=s.retained])
Next == /\ s.tick < Bound
  /\ (Prepare \/ (\E c \in Components: Stage(c)) \/ Commit \/ Narrow
      \/ Revoke \/ Discover \/ RevokeRead \/ Evict \/ Release \/ Gap \/ Resync \/ Consume)
Spec == Init /\ [][Next]_s
TypeOK == /\ s.tick \in 0..Bound
  /\ s.phase \in {"New","Building","Ready","Committed","Rejected"}
  /\ s.staged \subseteq Components /\ s.scope \subseteq {0}
Coherent == s.root=InitialRoot \/ s.root=Publish(InitialRoot)
Frozen == s.frozen=InitialRoot
BodyAndOwner == s.root.body="authored" /\ s.root.owner="local"
SafeRelease == s.delivered =>
  (s.releasedGrant /\ s.releasedRetained /\ s.releasedRev=1)
GapSafe == s.eventApplied => s.eventAfterSync
EventReleaseSafe == s.eventApplied => (s.eventGrant /\ s.eventRetained)
ScopeAtPublish == [][(s'.accepted /\ ~s.accepted) =>
  (s.grant /\ s.expected=s.root.rev /\ ScopeOK /\ s.staged=Components)]_s
RejectedUnchanged == [][(s'.rejection /\ ~s.rejection) => s'.root=s.root]_s
DiscoveryAuthorized == [][(~s.discovered /\ s'.discovered) => s.grant]_s
Committed == s.phase="Committed"
ScopeRejected == s.phase="Rejected" /\ s.scope={}
DeniedAfterDiscovery == s.phase="Rejected" /\ s.discovered /\ ~s.grant
Handoff == s.delivered
GapRecovered == s.eventApplied /\ s.eventAfterSync
RetainedSnapshot == s.phase="Committed" /\ s.frozen=InitialRoot
NotCommitted == ~Committed
NotScopeRejected == ~ScopeRejected
NotDeniedAfterDiscovery == ~DeniedAfterDiscovery
NotHandoff == ~Handoff
NotGapRecovered == ~GapRecovered
NotRetainedSnapshot == ~RetainedSnapshot
=============================================================================
