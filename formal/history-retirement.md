# Linked history retirement candidates

Specification §14.8 refines adopted D14 with the actual normalized group/frontier
registry. The helper stages complete root and retention-inventory candidates; it
does not install a native root, create another mutation interface or reclaim every
copy. Public retirement routing, real pins/promises/permissions/reservations and
native companion/lifetime transfer remain separate gates.

## Registry and dependency interpretation

Validate complete groups, known source/body/target/membership journal kinds,
participant scopes and coherent undo/redo stacks. Every valid group appears once
in every actual participant scope, in one consistent direction. Invalid groups
are absent from active stacks. Stack order agrees with retained group order, and
undo entries precede remaining redo entries in that order for each scope.
Duplicate, unknown, partial-linked and malformed inventories refuse, not empty.
Inverse correctness and historical mapping truth are not re-proved by this check.

Within each stack, the top is last. To reach a lower entry one must traverse the
entry immediately above it. Add an edge from the higher prerequisite to the lower
dependent in both undo and redo stacks. Removing a middle entry therefore removes
the unreachable prefix below it, not automatically the still-reachable suffix
above it. Shared group identities propagate closure through every linked scope.
The construction follows the existing actual compound publisher's redo-prefix
invalidation rule, extended here to retirement of either direction.

The host supplies additional dependency edges under complete-coverage/current-
obligation facts. Frontier adjacency is not assumed to discover every mapping,
snapshot, admitted reversal or semantic retention dependency. Cycles are permitted
in this extra dependency relation; the finite closure removes their complete
reachable component or refuses. Unknown endpoints and duplicate supplied rows
refuse rather than dropping obligations.

Current retention facts include explicit protected group identities (`pins`),
per-group retirement permission (`allowed`) and frontier promises (`floors`). A
floor identifies a scope/direction and a `through` group: protect that entire
top-connected stack segment, including the named group. This is the reference
floor interpretation, not a duration, byte quota or universally selected pin API.
Pins count distinct protected groups, not the actual number of pin owners/copies.
Host completeness, authorization and discharge truth remain premises.

## Preparation and final candidate

Requests name the authority and explicit seed groups. Preparation validates current
exact-true grants/coverage, finite limits and all registry/retention data, computes
the least closure, checks every affected group's permission and all pins/floors,
and preflights the complete plan/candidate. It returns only an opaque owned plan.
Copied immutable intent, full group/frontier basis, normalized retention facts,
limits and recipient/profile bind later use. There is no public graph/count reply.

Final candidate construction requires the original plan owner and unchanged
history/retention/interpretation basis, with current grants and resources again.
Whole-history binding is conservative: even a changed unrelated pin inventory
needs new preparation. Unrelated current source/view/settings fields are not
replaced with a prepared old root. A refused candidate returns the current root
unchanged. An empty validated removal needs no unused publication generation.

For a nonempty admissible closure, remove whole group rows and all their frontier
entries in one new publication generation. Preserve current documents, authored
source/comments, version floors, views, bindings and unrelated state. Return the
matching retention companion: remove incident extra edges and retired permission
entries; preserve surviving pins and promises. The root and this companion must
be installed coherently by the real coordinator, not by two independent writes.

Logical removal is not physical destruction, evidence discharge, producer stop
or source/annotation deletion. In particular, retained-absent membership remains
retained until its separate actual `retireMembership` gates pass. Prepared plans,
snapshots and workers may retain copies. Weak parser-journal/native ownership
handoff is not established by copying the JSON-like root; actual preservation or
declared optional unavailability must be composed at installation.

## Finite closure laws and termination

For the fixed finite group set G and complete relation E, use the powerset lattice
P(G), ordered by inclusion, with bottom empty and top G. Define
F(X) = seeds ∪ X ∪ successors_E(X). F is monotone and inflationary. Starting at the
seeds, each newly discovered group strictly decreases |G minus discovered|.
The worklist terminates after at most |G| node visits and |E| reachable edge probes;
the explicit per-phase `maxClosureSteps` may refuse sooner without a partial plan.
Other validation/copy scans have their separately bounded finite domains.

Every discovered node belongs to every E-closed superset of the seeds: initially
this holds for seeds, and a closed superset contains each successor of its members.
At worklist exhaustion the result is itself closed, hence is the least such set.
Closure is extensive, monotone and idempotent. E-closed subsets are closed under
union and intersection, giving a bounded distributive lattice for this fixed
relation. These are authored conditional arguments, not proof-assistant induction.

If that least set intersects a protected group or contains a group without current
retirement permission, every closed superset contains the same blocker. Refusing
the complete retirement is therefore necessary under these fixed facts; retaining
only a convenient visible subset is not another valid closure. Changing a promise,
permission or edge requires fresh authorized evidence, not an algorithmic fallback.
This lattice concerns group sets only, not all editor states, arbitrary evidence
joins, provider behavior or categorical completeness of the whole engine.

Each retained frontier keeps a suffix after closure. No surviving entry appears
reachable across a removed gap; complete linked groups remain coherent. Native
records and authored versions are framed, while the publication counter advances
once for nonempty history metadata change. Actual concurrent installation and
all-owner retention accounting must refine this candidate relation separately.

## Executable evidence

The checker uses actual group rows produced by the compound publisher, not a
second abstract bitset implementation. It enumerates 32,768 cases over all 64
loop-free directed three-group edge relations, eight seed sets, eight pin sets and eight
floor selections. Its oracle intersects all closed seed supersets rather than
following the worklist. Permission coverage has separate linked-scope controls;
the matrix is not an enumeration of every runtime/history graph.

Forty-two native annotation capture/undo/redo states exercise actual retained
registries and unchanged authored state. Ninety single-missing/nonboolean checks
cover fifteen gates at preparation and final candidate construction. Eighteen
controls cover stack direction, linked hidden participants, pin/floor/permission
changes, owned intent and current-basis revalidation, resource limits, cycles,
empty counter-exhausted identity, candidate companion coherence, existing history
queries and actual membership retirement after group release.

The source-only survivor comparison uses the actual compound and reversal helpers
on paired unretired/retired roots. Enumerate all 27 three-command participant
sequences over document A, document B and their linked pair. Each participating
source toggles between `a` and `b`, deliberately including equal-byte ABA histories.
For each sequence, take all four global undo prefixes and all eight retirement seed
sets: 108 history states and 864 candidates. Try each surviving group in both
directions, comparing availability and identity-on-refusal; successful edges keep
complete document/version frames equal, surviving group rows identical and
frontiers equal to the original filtered by surviving IDs. Publication advances
once independently on each successful side. Input roots remain unchanged.

The exploration has depth six and deduplicates visited frontier/source signatures;
it visits 1,962 selected paired states and checks 6,576 attempts. Source versions
are compared at every traversed edge, but are not in the deduplication key. This
is explicitly not exhaustive over version/counter states, arbitrary trace lengths,
new edits, annotations, target mapping or native installation. The native annotation
cases above test retirement frames, not this source-only reversal equivalence.

For source-only roots without annotations or activated membership inventories,
the authored preservation argument strengthens the
frame with two conditions: surviving journals are unchanged, and each original
stack has a removed prefix followed by its retained suffix. Closure establishes
that condition in both directions for every participant scope. A surviving group
is at all required tops on one side exactly when it is on the other; a removed
top has no retained entry below it. Equal current source/version frames and equal
forward/inverse journals give the same deterministic reversal or refusal under
equal current gates. A successful reversal moves the same surviving top between
stacks, leaving removed entries below retained entries and preserving the relation.
Conditional induction therefore preserves the relation along source-only reversal
traces while roots, gates, finite counters and inverse preconditions stay valid.
This is an authored conditional argument, not a proof-assistant derivation or an
all-kind theorem. Membership/native companions need their own refinement.

A direct counterexample shows why simply unlinking selected entries is unsound:
after intervening source changes return to equal bytes, ordinary reversal can
accept older history exposed across the gap. Closing the retirement prefix removes
that false eligibility while preserving newer reachable history and authored text.
Eighteen assertion-detected mutations follow a passing dynamically rewired baseline,
including corruption of a surviving source inverse.
Syntax/import failures are not detections. The existing 223 solver conditions in
thirteen manifests remain unchanged; these checks add no new solver theorem or
independent Cleanroom certification.

## Precedent and observations

[Neovim undo blocks and retained history](https://neovim.io/doc/user/undo/) provide
direct precedent for whole command blocks and states becoming unreachable when
undo data is cleared. Vellum retains frontier-only linked history rather than
adopting Neovim's branch navigation or persistent-undo file behavior. Complete
cross-scope closure and pin/promise refusal derive from Vellum's adopted D14, not
an inherited proof or claim that the editors have identical retention semantics.

Reuse PATH-04, IF-03/12 and INS-01/02/03/06/08/12. MELT, RED/USE and golden signals
distinguish offered/prepared/refused/candidate retirements, closure work/capacity,
retained pressure and installation/reclamation evidence. A candidate is not an
installed retirement or reclaimed memory. STRIDE includes forged plans, partial
participants, stale permission/pin inventories, missing edges and premature release;
LINDDUN includes private group/scope graphs and pin/blocker linkage. Raw identities,
group graphs, journal contents and private blocker reasons are not ordinary labels.
Current recipient projection and measured installed telemetry remain required.

No public verb, service, runtime, native job kind, numeric default or automatic
eviction trigger is selected. The helper accepts trusted normalized inputs, not
hostile executable objects. Real host facts, preparation/global/physical accounting,
native root/weak-journal transfer, full retirement/invocation/result/event lifecycle,
production/concurrency and independent assurance remain. The ordinary annotation
destination-binding decision is unaffected. No whole D14/F14 or goal closes.
