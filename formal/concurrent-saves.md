# Concurrent saves: evidence contracts and coordination comparison

Status: the universal attribution/evidence constraints follow existing
requirements §§3.6–3.7, 5.4 and 5.9. Subsequent user residual review selected
single-flight plus bounded FIFO of exact admitted captures, explicit capacity/
expiry outcomes and no implicit coalescing (§5.10). The alternatives below remain
a model comparison, not proof-derived approval or implementation. Numeric budgets,
deadlines, service and host storage capabilities remain unspecified.

## Conclusions supported by the model

1. Request/capture order, editor revision order, host write order and receipt
   delivery order are different relations. A late receipt remains attributable
   to its actual immutable snapshot; it is not permission to relabel current state.
2. Serialization/profile changes can alter output without changing text revision.
   Choosing a baseline by maximum text revision alone cannot resolve that case.
3. Cancellation stops control, not necessarily storage activity. A conservative
   single-flight slot stays occupied until the effect has a known safe disposition.
4. Concurrent conditional writes from one **non-reused version** can have only
   one successful consumer. A successful write must advance that version even
   without a foreign writer. This assumption was not exercised by the earlier
   one-attempt-per-artifact model; this extension checks it explicitly.
5. Conditional equality validation is not causal-order evidence by itself. Only
   under a declared non-ABA token contract can a successful write's output token
   matching another write's input token establish that causal edge. Token values
   remain opaque; lexical/numeric sorting is not justified.
6. Trusted host ordering evidence can resolve receipt reordering, but is scoped
   to the resource/authority/binding epoch and actual write event. A trace ID,
   client timestamp or callback sequence is not that evidence.
7. Missing order evidence must remain missing. Retain all exact receipts and an
   unresolved set of maximal acknowledged candidates rather than inventing a
   unique latest acknowledgement. This is an internal reference construction,
   not a required public container/API shape.
8. Even a unique maximal **acknowledged** write does not prove current external
   contents: other attempts can be unresolved and external writers may intervene.
   Equality with a historical baseline, current external freshness, pending work
   and durability remain separate dimensions.

The design distinction between causal and total order is informed by
[Lamport's ordering model](https://lamport.azurewebsites.net/pubs/time-clocks.pdf).
The editor-specific relations and policy recommendations here are our derivation,
not a claim that the paper specifies this persistence protocol.

## Capability separation

| Host evidence/capability | What it can support | What it does not imply |
|---|---|---|
| Opaque equality token | Equality check under its declared identity/validity contract | Total order, per-write uniqueness, durability or freshness after the check |
| Atomic conditional write | Compare the expected state and perform the write at the provider boundary | A token never repeats; arbitrary external writer isolation beyond the provider's contract |
| Non-reused write-version token | Safe causal token-chain inference within its advertised identity/retention epoch | Discovery of unobserved intervening writes or a globally ordered authority |
| Attested per-resource write order | Order known successful writes independently of delivery | Current external content, cross-resource atomicity or true author identity |
| Completed/quiescent attempt evidence | Safely release the relevant write-ownership obligation | Successful content persistence, crash durability or safe redirection to another binding |

[RFC 9110 §8.8.3](https://www.rfc-editor.org/rfc/rfc9110.html#section-8.8.3)
defines opaque entity tags; [§13.1.1](https://www.rfc-editor.org/rfc/rfc9110.html#section-13.1.1)
describes conditional If-Match evaluation to prevent lost updates. Those are
useful analogies, not a mandatory HTTP transport. A representation validator is
not automatically a unique per-write sequence or a non-ABA history identifier.
Returning to an earlier representation can make an earlier content-derived
validator applicable again. Do not infer a stronger provider capability merely
from the label ETag/CAS/atomic replacement.

## Reference model and checked scope

`ConcurrentSaves.tla` has two attempts, Old and New, on one fixed resource,
incarnation and binding. Initially unsaved live text is 1 while external text is
0. A later source edit returns live text to 0 at a fresh relative revision; in
the profile-only variant text stays 1 and only output profile changes. Relative
revision zero is the start of this slice, not an identity reset in production.

Snapshots capture exact attempt/context/revision/profile/payload and the last
explicitly observed host token. The separate Probe event models an explicit host
read outside local publication; it is not hidden I/O or a token-refresh-and-retry
loop. Captures are immutable and conflicts do not automatically redispatch.

HostWrite performs conditional comparison and replacement as one host action.
Fresh-token profiles advance on every successful local or foreign write;
reusable-token profiles permit ABA. Token integers are model representatives:
library inference compares them for equality only. Host positions are proof
oracles, exposed as ordering evidence only in the declared ordered profile.

Success and conflict receipts are exact immutable atoms; duplicate union is
idempotent. `Before` derives only permitted token-chain/host-order edges between
acknowledged successful attempts. `Maxima` is the undominated acknowledged set.
Receipt evidence grows monotonically; this derived set need not grow monotonically
as later evidence establishes a new ordering. The evidence lattice and its
derived decision projection must not be conflated.

This is intentionally a conservative, incomplete inference relation. For example,
a foreign write breaks a direct token-chain edge, but accepted local Probe
chronology plus non-reuse might still establish an order using a stronger proof.
The gap witness shows what this reference algorithm leaves unresolved; it is
**not** an information-theoretic impossibility theorem for fresh tokens. The
separate two-world reusable-token countermodel below establishes a genuine
ambiguity for its narrower observation/capability profile.

`Unresolved` is based on dispatched attempts without accepted result evidence,
not the model oracle's knowledge that storage has finished. Stop preserves it.
The candidate single-flight guard conservatively blocks other dispatches until
that evidence arrives. A separately proven quiescence signal could permit a
different contract, but this model does not invent one or guarantee progress.

Nineteen TLC cases cover:

- Four eight-step exhaustive profiles: overlapping fresh tokens, single-flight,
  reusable opaque tokens, and host-attested ordering.
- Two positive scripted runs: reversed successful receipts in ten steps and an
  intervening foreign write breaking the direct token chain in eleven steps.
- Four negative controls: receipt-arrival selection, cancellation freeing an
  unresolved slot, violating declared version advancement, and inferring causal
  order from reused tokens.
- Nine witnesses: overlapping outstanding attempts, stopped-but-unresolved work,
  blocking behind it, a newer capture winning the conditional race, reversed
  receipt delivery handled safely, same-revision/different-output saves,
  incomplete order across a foreign-write gap, host ordering resolving that gap,
  and reusable-token order remaining unresolved.

The ABA script returns storage to its initial representation with two writes,
while an unsafe token-chain inference fabricates the reverse edge. Generic
reusable-token exploration is an overapproximation, not a formal HTTP/ETag
implementation. The model does not implement codecs, resource aliasing, retries,
queue fairness, arbitrary attempt counts, recovery or a concrete filesystem.

Four additional SMT conditions check the non-reused-version single-consumer
argument, associative/commutative/idempotent maximum merge of trusted order
positions, a satisfiable ABA countermodel, and two storage executions with equal
accepted observations/receipts but different final content. These are trusted-solver VCs,
not automatic translation/refinement of the TLA+ model. Max-order merge is valid
only after same-scope trusted order evidence is established, never over opaque
token spellings or document revisions.

In the two-world countermodel, external state begins at 0. In world A, Old
conditionally writes 1, Probe observes token/content 1, and New writes 0. In world
B, an unobserved foreign write produces 1, Probe observes the same state, New
writes 0, and the still-outstanding Old writes 1. Tokens denote representation
values and can repeat. Both worlds accept the same Old/New snapshots and
successful receipt tokens delivered New then Old, yet end with different bytes.
Without additional evidence, those observations cannot determine actual final
write order/content. This is not an argument that all providers or all possible
observation profiles are ambiguous.

## Policy alternatives — pending decision

| Candidate | Pros | Cons/tradeoffs | Recommendation |
|---|---|---|---|
| Single-flight per resolved resource identity | Smallest overlap/unknown-outcome state space; straightforward attribution and retention | Head-of-line waiting behind unknown effects; does not itself resolve external writers or choose queue order | Recommended conservative v1 direction, pending approval |
| Overlapping conditional writes | Can exploit provider concurrency and latency; one stale precondition need not block unrelated work | Conflicts can select an unexpected winner; stronger evidence/ordering and reconciliation required; does not promise latest-intent wins | Capability-gated alternative, not an implicit v1 promise |
| Single-flight with coalescing of undispatched work | Reduces burst/agent save traffic | Must explicitly distinguish superseded requests from persisted exact revisions; cannot coalesce already dispatched unknown writes | Defer until admission/supersession semantics are decided |

The recommended direction is per **host-resolved resource identity**, not path,
workspace or serialization profile. Distinct profiles can still overwrite the
same destination. Binding/incarnation/attempt fencing remains necessary, and an
unknown old-binding write cannot be ignored during rebind. This relies on the
identity contract already required by §3.7; no universal alias discovery is added.

Single-flight is local coordination, not disk isolation: retain conditional-write
capability checks for any requested conditional guarantee, handle external
conflicts explicitly, and do not refresh a token and retry an old snapshot as an
automatic force overwrite. Editing and independent resources continue through
the normal publication path; this is not another editing mode or global lock.

Adoption still needs a decision on busy versus pending admission, queued request
ordering, bounded retention/wait, supported provider profiles, safe-disposition
evidence and any superseded outcome. None is silently selected here. All
observations retain recipient projection and bounded content-safe instrumentation
at IF-04/IF-10/IF-13, rather than exposing source, opaque tokens or private IDs in
default logs/metric dimensions. Concrete schemas and measured overhead remain open.

The follow-on [admission comparison](save-admission.md) checks candidate busy and
bounded FIFO policies while leaving adoption, budgets, expiry and coalescing open.
