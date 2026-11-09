# Proofs of the adopted abstract editor contracts

Scope: PA-01–09 in specification §27, not correctness proofs of Emacs, Neovim,
Helix, CodeMirror, actual Rust code or native platforms. These are authored
mathematical arguments with solver-checked local preservation lemmas in
`precedent-laws.smt2` (29 conditions: 19 positives and 10 controls). Existing
`parametric.smt2` and finite TLA+ slices supply additional, distinct support.
The report records the two SMT manifests separately; neither is silently
counted as a TLC case or a new whole-system assurance claim.

## Method and limits

For each safety invariant I: establish I initially; show every admitted abstract
transition preserves I; conclude I after every finite sequence by induction on
transition count. Idle/reject transitions preserve state. This is an authored
induction argument, not a proof-assistant-checked induction or automatic
translation from specification prose. The SMT solver checks the stated local
formulas over arbitrary integers, arrays and finite sequences, with no fixed
document length, alphabet size, participant count or execution horizon.

Every positive condition first checks SAT premises, then UNSAT of the negated
conclusion. Controls require SAT of the faulty alternative. This rules out
inconsistent positive premises and demonstrates selected fault sensitivity,
not exhaustive failure coverage. No `unknown`, timeout, parser error, missing
condition, duplicate output or extra solver diagnostic is accepted. Z3 4.15.4
is trusted; no proof object is independently replayed. The correspondence between
the formulas and these mathematical arguments is reviewed reasoning, not a
mechanically checked refinement.

The abstraction has unbounded mathematical integers. Actual checked arithmetic,
allocation, finite IDs and identity reuse/expiry require implementation refinement.
Read/write authority and currentness inputs must be supplied truthfully. Content
arrays are not concrete allocation graphs. Claimed snapshots are retained and
fresh root identities are not overwritten; proving reclamation/lifetime is a
separate obligation. No new concrete schema, namespace capability, endpoint
profile, coalescing timeout, grammar, setting catalog or provider is selected.

## PA-01: functional roots and retained snapshots

Represent immutable root contents by R: Root → (Participant → Value). A staged
publication supplies complete candidate vector C and a fresh root r', not any
already-published/retained root. Publish by R' = R[r' ↦ C].

Theorem: every old retained root r keeps exactly its prior vector, and every
participant of r' equals C's corresponding participant. Array store/select laws
give R'(r)=R(r) for r≠r' and R'(r')=C. `PRE-SNAPSHOT-PUBLISH` checks both pointwise
and whole-vector equalities. Starting from an immutable initial root, induction
over fresh publications preserves every retained root. Stage/view/reject actions
do not write R by this abstract transition definition, so they preserve it too.

`PRE-SNAPSHOT-REUSE-CONTROL` exhibits overwritten snapshot contents if freshness
is removed. This proves the value-level store discipline, not actual COW, alias
safety, reclamation or absence of illicit implementation mutation paths.

## PA-02: arbitrary finite original-snapshot batches and inverse order

Let T be a finite sequence. A canonical finite list E consists of replacements
(lo,hi,I), in ascending original coordinates with 0≤lo≤hi≤|T|. Source interiors
do not overlap. Adjacent nonempty ranges are allowed. Coincident/touching
insertions enter only after the separately declared ordering/combination resolves
them; no resolution policy is inferred here. These premises imply that the
cursor below never goes backwards. Expected source slices/current authority are
separate admission premises, not implied by the text theorem.

Define a reference scan, not a production algorithm choice:

    B := ε; c := 0
    for (lo,hi,I) in E:
        G := T[c:lo]; D := T[lo:hi]; S := T[hi:|T|]
        l := |B| + |G|
        h := l + |D|
        record inverse (l, l+|I|, D)
        B := B ++ G ++ I; c := hi
    return B ++ T[c:|T|]

Invariant after k edits: the sequentially updated document is B ++ T[c:|T|];
B is precisely the first k original gaps and inserted sequences. The processed
source prefix ends at c; everything later is still an original T slice.

Base k=0: ε ++ T[0:|T|]=T. For the next edit, the current document factors as
B ++ G ++ D ++ S because the next range starts at or after c. Replacing the
current range [|B|+|G|, |B|+|G|+|D|) yields B ++ G ++ I ++ S. This is exactly
the new B and c invariant. `PRE-BATCH-PREFIX-STEP` checks the arbitrary-sequence
factorization and length delta, including empty segments. Thus structural
induction on any finite E gives exactly specification §4.2's original-snapshot
concatenation and |result|=|T|+Σ(|I|−(hi−lo)). No two-edit or fixed-list bound
is needed for this authored theorem.

For the inverse, at each step replace [l,l+|I|) by the captured D. The same
factorization yields the immediately previous document; checked by
`PRE-BATCH-INVERSE-STEP`. Starting with the last recorded inverse and inducting
backwards recovers T. Earlier inverse coordinates become valid only after all
later effects are reversed. This proves content recovery for the exact staged
chain, not permission to selectively undo across changed frontiers.

The strict-gap/adjacent two-edit SMT laws supply independent composition checks.
`PRE-BATCH-UNSHIFTED-CONTROL` exhibits differing results from obsolete prefix
coordinates despite valid slice bounds. Sorting/tie resolution, arbitrary
input normalization, multi-selection lowering and actual retained-inverse storage
are not verified code. Scalar-boundary/valid-UTF8 admission remains a premise;
these sequence laws do not prove grapheme navigation or all Unicode profiles.

## PA-03: namespace frame and conditional range-order preservation

For attachment map M: (Namespace,LocalID) → Position, an update of key (a,i)
preserves every key (b,j) with a≠b, even if i=j. Datatype constructor injectivity
and store/select prove this; `PRE-NAMESPACE-FRAME` checks arbitrary namespaces
and IDs. Induction over updates restricted to namespace a preserves all other
namespace values. An ID-only store fails this frame (`PRE-NAMESPACE-ID-ONLY-CONTROL`).
This theorem is storage isolation, not an authorization or namespace-secrecy theorem.

For insertion at p of length k≥0, map q to q before p, q+k after p, and at p
choose p or p+k according to its declared affinity. For range endpoints a≤b,
mapped order and bounds hold if a<b OR left endpoint is left-affine OR right
endpoint is right-affine. `PRE-ANCHOR-RANGE-ORDER` checks the implication. With
a=b=p, left endpoint right-affine and right endpoint left-affine, k>0 inverts
the endpoints (`PRE-ANCHOR-OPPOSED-AFFINITY-CONTROL`). A supported profile must
exclude that configuration or explicitly define a normalization/detachment rule;
the theorem does not select which. Replacement/deletion identity and mapping
of a whole attachment map through source edits remain open profile/refinement work.

## PA-04: atomic root observation and abort frame

With PA-01's store discipline, local publication changes the live root from r to
r' at one abstract transition, after complete candidate validation/staging.
An observation captures one retained root once and projects every participant
from it. Thus it returns either R(r) or C, never a mixture. This is checked
pointwise for an arbitrary participant by `PRE-ATOMIC-ROOT-READ`. The participant
domain need not be finite for this law; actual publication requests remain finite.
If validation fails, live root stays r (`PRE-ATOMIC-ABORT-FRAME`). Private staging
or unreferenced candidate storage is not authored publication or a recipient release.

Induct on root swaps: between swaps reads pin the current or a permitted retained
root; a swap creates a new coherent choice and preserves old roots. Therefore
any finite interleaving of these abstract reads/swaps is coherent. Each read's
linearization point is its root capture; multiple independent reads are not
one snapshot. `PRE-ATOMIC-TORN-READ-CONTROL` constructs a vector equal to neither
old nor candidate if participant values are read/written independently.

This is a general abstract construction theorem. It does not prove that the
implementation stages all required companions, uses atomic/synchronized root
capture, retains ownership safely or mediates current recipient releases. Existing
bounded transaction/bridge checks cover selected gate interactions. External
effects, file writes and callbacks cannot be inserted into this atomic step.

## PA-05: explicit group lifecycle and unrelated-plan isolation

Use an abstract compatibility key (author,plan,context), not a chosen public
schema. It stands for all grouping-relevant identity, including unrelated-plan
distinction. A new group starts empty with fixed key and open status. Append
is possible only on explicit/profile-authorized join, open status and equal key.
Close preserves entries and prevents further append to that group. Rejection
preserves entries; creating another group does not reopen the closed one.

Invariant: every entry in a group carries its fixed key. Empty initialization is
vacuous. On append, an arbitrary index is either in the old prefix, preserved by
the induction hypothesis, or the new last index containing the matching key.
`PRE-GROUP-APPEND-STEP` checks both cases with arbitrary group size. Close/reject
preserve the array and size; `PRE-GROUP-CLOSE-STABLE` checks the closed case.
Induction gives isolation for all finite append/close sequences. Groups are not
merged by time alone. `PRE-GROUP-PLAN-OMITTED-CONTROL` shows author and context
may match while plan differs, so those fields alone are insufficient.

This proves grouping identity/closure, not inverses, arbitrary linked-frontier
graphs, retention/pins/floors, authorization to join or concrete typing policy.
Those remain separate gates; finite History models do not become unbounded
history-refinement proofs through this result.

## PA-06: revision-gated commands and conditional surface equivalence

For a canonical request define admissibility to include registered contract,
current revision/dependencies and present authority, in addition to its other
required semantic checks. Abstract dispatch changes root to its validated
candidate only when admissible, otherwise leaves it unchanged. Unknown,
stale or denied requests therefore cannot mutate; `PRE-COMMAND-REJECT-FRAME`
checks this frame. A discovered descriptor is not an admissibility input.
`PRE-COMMAND-DISCOVERY-AUTHORITY-CONTROL` shows stale discovery can wrongly permit
mutation if substituted for current authority.

If UI and agent lowering yield the same canonical request/candidate and current
context, deterministic shared dispatch yields the same state/outcome meaning.
`PRE-COMMAND-SURFACE-EQUIVALENCE` checks the state equality. Induction extends
this to equal lowered request sequences under equal intervening context events.
The identical lowering/context premise is essential; this is not proof of
actual adapters, native event ordering, public outcome schemas or discovery
field noninterference. Descriptors remain subject to mediated release. Closed
vocabulary/totality for actual schemas still requires D33; finite Execution's
declared vocabulary is not the final public API.

## PA-07: independent regions, delimiters and source maps

For ordered disjoint half-open source intervals [a,b),[c,d) with b≤c, a content
point cannot belong to both (`PRE-REGION-UNIQUE-MEMBERSHIP`). Apply this pairwise
to any finite disjoint region list: each byte has at most one such region; gaps
and delimiter intervals remain host-owned by the supplied partition. A local
offset x in [0,b−a) maps to a+x and back by subtracting a. The existing affine
map theorem includes endpoint domains; content membership here is half-open.

A point update within the first region preserves every other region and every
outside delimiter (`PRE-REGION-INDEPENDENT-FRAME`). A length-changing replacement
inside that region preserves the entire original suffix, but shifts its source
origin by the edit delta (`PRE-REGION-SPLICE-SUFFIX`). Thus later disjoint regions
retain their byte content under correspondingly shifted coordinates, not under
stale offsets. PA-02's finite-list induction extends this framing to resolved
nonoverlapping edits. Derived/parser context still needs invalidation/revalidation;
unchanged bytes do not imply unchanged language interpretation or authority.

If two disjoint fences are concatenated and treated as one contiguous source
interval, offsets for the second fence hit the gap or wrong source location
(`PRE-REGION-CONCATENATION-CONTROL`). Independent mode must retain separate maps.
This proves consequences of a valid partition, not correctness/completeness of
actual fence discovery, nesting/crossing/precedence, aliases or parser assets.
No delimiter reassignment or combined-injection policy is adopted here.

## PA-08: typed composition, dependency frame and activation

Let C(k) be a configuration value, U the changed-key set, and F_k each declared
setting-specific combiner. Define C'(k)=F_k(C(k),input(k)) for k∈U and C(k)
otherwise. Unrelated settings are unchanged (`PRE-CONFIG-UNRELATED-SETTING-FRAME`).
No associativity, commutativity, idempotence or override order is assumed for
arbitrary F_k; actual typed combiners need their declared laws and validation.

If the dependency relation is complete, a consumer declared unaffected cannot
use a changed key. Its used values therefore remain equal pointwise, checked
by `PRE-CONFIG-DEPENDENCY-FRAME`. This extends to every finite sequence of
changes that leaves that consumer unaffected, by induction. Omitting an actual
dependency admits an affected value while the consumer is marked unaffected
(`PRE-CONFIG-MISSING-DEPENDENCY-CONTROL`). Dependency completeness is an explicit
supplier/refinement obligation, not proved by this implication.

Existing `SMT-CONTEXT-COHERENT-ACTIVATION` proves compatible old or validated
compatible candidate context remains compatible under a single whole-context
switch. Induct from an initially valid context; reject/stage preserve it, accepted
switch installs a valid one. The partial-activation control exhibits failure if
components are selected independently. PA-01 supplies value-level old-context
preservation; DerivedRetention supplies bounded lifetime/control examples.
Actual validators, retained-generation capacity, exhaustion/fallback, keymap
conflict rules and concrete activation synchronization remain unproved.

## PA-09: synchronization evidence, not physical durability

Fix one captured effect attempt and its identity domain. Maintain observed facts
W (accepted write acknowledgment), Q (accepted synchronization acknowledgment),
F (accepted sync failure). Initially all false. Accepted WriteReceipt adds W;
accepted SyncReceipt adds Q even if W has not arrived; accepted SyncFailure adds
F without retracting W or Q. Other events add none. Define the corroborated
synchronization projection S = W AND Q. Facts accumulate. This is a proof
relation, not the chosen concrete receipt schema or final disposition policy.
Combined evidence can be interpreted as two observations in either order.

Invariant S⇒W and monotonicity of W/Q/S/F hold initially and under each event,
checked by `PRE-SYNC-EVIDENCE-STEP`; induction gives all finite event traces.
An observed sync failure after W preserves W without asserting S
(`PRE-SYNC-FAILURE-PRESERVES-WRITE`). A sync acknowledgment arriving first remains
in Q and joins a later accepted write acknowledgment; it is not dropped
(`PRE-SYNC-OUTOFORDER-RETAINED`). Reporting synchronization solely from W is
unsound (`PRE-SYNC-PREMATURE-ACK-CONTROL`). A prior S and later F may coexist as
observations; they do not grant a new safe-disposition success or erase conflict.

This proves evidence accounting only. Truth/authenticity/attempt attribution of
accepted receipts, actual write/sync execution, error behavior, attributes,
hardware durability and failure recovery require provider contracts/tests. It
does not establish universal storage CAS, crash recovery or durable group saves.
The supported-sync default is a policy backed by precedent, not a theorem that
turns unsupported hosts into durable storage. Local cancellation does not clear
past effect facts; existing bounded storage/dispatch models cover selected late
and contradictory-evidence paths.

## Proof disposition and next boundary

All nine adoptions now have explicit general abstract arguments and named local
solver support, with assumptions and fault controls. This closes the request
for abstract construction/preservation arguments, not each adoption's full
assurance gate. Unselected attachment/API/configuration/provider profiles must
be decided before their concrete specifications and refinements can be proved.
Production/native claims require production/native evidence; none exists yet.
No goal-completion or independent certification follows from this document.
