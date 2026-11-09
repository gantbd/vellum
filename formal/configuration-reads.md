# Configuration read dependencies

Section 13.9.5 additionally checks all three configuration domains and whole-state
conditions conjunctively with source/annotation and workspace predicates. See
mixed-reads.md for the fixed-instance finite product, native interleavings and
advisory agreement. This adds no setting consumer or automatic footprint discovery;
other provider/host/physical/independent gates remain.

Section 13.9.4 explicitly inherits these configuration products into native `/5`
alongside workspace-membership reads. Native mixed-conjunction cases preserve
configuration/workspace state and canonical intent in one actual owner; see
workspace-reads.md. `/4` semantics and `native-read-authority/2` remain unchanged.
Inheritance supplies no proof that a logical settings target applies to a document
or workspace, and does not discover missing consumer dependencies.

Specification §13.9.3 extends the native compound read conjunction with exact
settings assumptions. A source edit can depend on a semantic configuration domain
without treating every configuration metadata change as a conflict. An explicitly
metadata-sensitive assumption instead selects the whole configuration counter.

## Contract and ownership

`read-dependencies.json` closes two added read products under
`owned-annotation-plan/4`. The plan preserves `/3` source scopes and field set.
Each read binds schema, restriction and association generations plus either the
target/domain generation or authority-wide configuration generation. Positive
safe integers, finite bounds, exact fields and unique subjects are mandatory.
Existing `/1`–`/3` meanings do not expand implicitly.

`declaredReadsCurrent` consumes the actual starting root supplied by the common
owner. `validatedEngineSettingsState` checks bounded data and resolved-value
coherence; it does not establish ownership, real target membership or permission.
The actual compound path supplies the owned root and actual invocation context.
`native-read-authority/2` carries exact current permission premises for the entire
predicate. Whole-state equality needs its own authorized observation, not merely
permission to inspect one contributing value.

Configuration validation precedes candidate mutation and no-op classification.
Disjoint target/domain or origin-only changes preserve an unaffected semantic
counter, while metadata changes advance the whole-state counter. A value changing
and returning still has a newer domain counter. Configuration reads do not create
authored history participants. Empty conjunctions preserve the minimal engine
without requiring a settings service. Advisory validation uses the same candidate
rules without publishing state or allocating an invocation.

## Precedent and proof scope

[FoundationDB conflict ranges](https://apple.github.io/foundationdb/developer-guide.html#conflict-ranges)
provide primary precedent for declaring targeted read assumptions rather than
conflicting on every unrelated change. Vellum adopts that design principle, not
FoundationDB's distributed isolation guarantee, automatic retry or runtime.
Version checks here are local reference behavior; existing conditional settings
counter laws do not establish production refinement.

Run `node formal/configuration-read-checks.mjs`. The checker covers sixteen
current/stale predicate cases across four kinds and two routes, 32 permission
masks and five actual settings interleavings. Ten controls cover interpretation
versions, target identity/incarnation, exact context and authority profile,
missing/over-budget configuration, stale no-ops, mixed source/configuration reads,
canonical retry intent, historical replay, optional empty conjunctions and F10
advisory agreement. The source publication preserves settings; settings activation
uses the actual common owner before the waiting compound resumes.

Five mutants independently omit schema, restriction, association, whole-state or
domain version fences. A dynamically loaded unmodified predicate module passes
before each weakened condition is required to cause an assertion failure. These
predicate mutants supplement actual native integration cases; they are not a
mutated whole-module-graph or independent implementation proof. The full frozen
suite includes this checker. Its report is authoritative only for identical
captured inputs and a successful terminal run.

## Instrumentation and remaining obligations

Use existing INS-01/02 admission/publication, INS-03 disclosure, INS-08 bounded
retention, INS-09 configuration and INS-12/13 lifecycle/history observation points.
Distinguish malformed, denied, stale, exhausted and published work with safe bounded
codes. MELT/golden/RED/USE observe queue/service latency, rejection/error classes,
publication and occupancy; do not label metrics with target identities, setting
names/values, generations, predicates or permission facts. STRIDE/LINDDUN apply
to predicate forgery, stale interpretation, missing grant and metadata inference.
No instrumentation is claimed installed by these reference checks.

Automatic complete dependency discovery, actual engine consumer adoption,
mutable definition/policy/association activation, target applicability truth,
all read families/events, public wire/Rust products, real host policy, physical
allocation/concurrency/crash behavior and independent Cleanroom assurance remain
separate obligations. No new UI, filesystem, network or external-effect ownership
is introduced.
