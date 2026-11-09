# Workspace membership read dependencies

Section 13.9.5 additionally composes membership with source/annotation and
configuration predicates in one owner. Fixed-instance lattice checks, native
interleavings and routed advice retain the whole conjunction and read-only frame;
see mixed-reads.md. Membership still proves neither member source stability nor
complete dependency discovery.

Specification §13.9.4 lets an edit depend on one declared workspace membership
set without conflicting on unrelated workspace activity. The predicate consumes
the generation already maintained by native lifecycle operations; it does not
introduce another workspace owner, directory model or global edit mutex.

## Contract and footprint

`owned-annotation-plan/5` preserves `/4`'s exact fields, source write scopes and
configuration kinds, adding the closed `workspace-membership` product with
`workspace` identity/incarnation and `expectedWorkspace`. The expected generation
is a safe natural integer. Explicit zero is valid under existing workspace row
contracts; missing generation never defaults to zero. Earlier profiles reject the
new kind. The unknown-version control moves from `/5` to `/6` because only `/5`
is explicitly adopted. Existing argument semantics and bounds do not widen.

The actual owner supplies the starting root and complete invocation context.
Require one exact live matching workspace row and current generation, plus one
exact read/presence/equality grant for the full predicate in
`native-read-authority/2`. Scope/identity knowledge and document permission are not
membership observation authority. Unknown, stale, ambiguous, malformed, denied
or over-budget reads refuse before no-op classification or candidate mutation.
Canonical read ordering never reorders/coalesces authored edit records.

Native membership addition/removal advances the workspace generation only when
the set changes. Disposal removes an actual member and advances affected workspace
generations; container retirement removes its exact row without disposing member
documents. Add/remove ABA restores values but not the old generation. Other
workspace changes do not stale this target solely through global publication.
Read-only targets add no history writer. Empty read conjunctions need no unused
workspace/configuration service. Replay observes the original outcome, not a
freshly interpreted operation.

The footprint is this declared membership set. Member contents, the workspace
registry, filesystem inventory, configuration applicability and semantic discovery
completeness need their own evidence/dependencies. An unjoined resource is not
silently part of a logical workspace because it shares a directory or spelling.

## Precedent and conditional laws

[FoundationDB conflict ranges](https://apple.github.io/foundationdb/developer-guide.html#conflict-ranges)
provide primary precedent for targeting read assumptions rather than conflicting
on unrelated changes. This is a design analogy, not adoption of its runtime,
distributed isolation or automatic retry. Vellum's native lifecycle already supplies
the workspace counter; no dependency is added merely to obtain that pattern.

Four BASE-WSREAD solver cases check the declared guard, two-step membership ABA,
disjoint-counter preservation and a satisfiable value-only weakened guard. Three
positive cases first establish satisfiable premises, then reject the negated law;
the control exhibits a stale version despite restored membership values. Safe
counter ranges are explicit. These are conditional scalar equations, not proof
that every native/host producer implements them. Totals are 278 conditions across
fourteen manifests, 64 baseline; no new solver manifest or state-machine family.

## Native reference evidence

Run `node formal/workspace-read-checks.mjs`. Fifteen controls cover eight permission
masks, complete context/predicate binding, exact identities, explicit zero, malformed
and duplicate products, actual live uniqueness and stale generations. Both typed
and dynamic native publication and F10 advisory routes use the actual common owner.
The actual annotation lifecycle profile is supplied when validating imported managed
annotation state; missing companion limits are not worked around by dropping it.

Six native interleavings cover membership change, add/remove ABA, disposal after
a real membership publication, container retirement, another workspace's progress
and idempotent membership. The disposal case writes an independent live document,
so rejection is not merely an attempt to edit a disposed document. Current/stale
no-ops, historical replay and empty conjunctions retain their normal meanings.
Two mixed configuration/workspace cases begin with accepted initial declarations
in one actual owner and check conjunctive canonicalization and metadata frames.
They do not establish real configuration-target applicability.

Nine predicate mutants omit generation/liveness/uniqueness, expand a legacy profile,
accept truthy grants, unbind context, drop count/duplicate limits or substitute
permission for another expected version. An unmodified dynamically loaded predicate
module passes before assertion failures count as kills. Native integration cases
use the unchanged actual module graph; these predicate mutants are not a mutated
whole-graph or production/independent refinement proof. Legacy read/configuration/
source-scope/advisory suites remain separately checked. Full frozen verification
and live/frozen input matching are required for a completed batch claim.

## Instrumentation and remaining gates

INS-01/02 cover bounded admission, validation and actual publication; INS-03 covers
predicate observation permission; INS-08/09/12/13 cover retained work, interpretation,
replay and history boundaries. MELT/golden/RED distinguish useful applied/unchanged
outcomes from stale/denied/refused work and queue/service time; USE tracks predicate,
intent, staging and retained-context pressure. STRIDE/LINDDUN cover forged scopes,
wrong incarnations, version replay, missing grants, hidden membership inference and
cross-workspace linkage. Use safe bounded codes, not raw workspace IDs, member lists,
counts, expected/current versions or permission facts as ordinary labels.

Actual policy and external membership/identity truth, automatic complete dependency
discovery, other provider/registry domains, all public products/consumers, physical
allocation/concurrency/crash behavior and independent assurance remain separate
gates. This adds no filesystem/network/identity service, UI behavior, public wire
format, job kind or production default. Instrumentation sites are specified, not
claimed installed or measured by these checks.
