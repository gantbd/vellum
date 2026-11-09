# Transaction publication clause traceability

The global source fingerprint is reconciled to the modular-substrate requirement
change after exact comparison with the adopted source: all ten selected §5.3
units and their 41 fragments are unchanged. Only the fingerprint changes; owners,
support, scenarios, residuals and sixteen sensitivity controls remain intact.
The earlier failed full report is not reclassified as passing or semantically
complete by this traceability repair. Current full qualification remains required.

Requirements §5.3's ten source bullets are partitioned into 41 review clauses in
[publication-clauses.json](publication-clauses.json). Each clause retains its exact
source fragment, owning specification sections, selected positive case/property
references, a conformance scenario, residual decisions/evidence and an integrity,
authority or disclosure risk. This makes separate obligations visible inside the
existing source-unit census without changing the requirements or choosing API
schemas. The clause IDs are audit-local, not engine verbs or public identifiers.

## Selected obligations and evidence

| Source unit | Clauses | Review focus |
|---|---|---|
| 5.3/b1 | TXC-01–10 | Explicit snapshot changes, common semantics, representation/adapters, partial mappings, staged companions, typed contracts and callback/runtime boundaries |
| 5.3/b2 | TXC-11–13 | Per-document revisions, original half-open coordinates and sequential lowering |
| 5.3/b3 | TXC-14–15 | Unresolved overlap/order rejection and selection/read-only validation |
| 5.3/b4 | TXC-16–19 | Preparation, final gates, stale rebuilding/rejection and no validation/publication state mismatch |
| 5.3/b5 | TXC-20–23 | Non-text dependencies, distinct freshness, recomputation and no obsolete selection restoration |
| 5.3/b6 | TXC-24–26 | Local atomic publication, unchanged failure and nonauthoritative diagnostics |
| 5.3/b7 | TXC-27–32 | Operation/generation identity, source revision rules and explicit cross-authority scope limits |
| 5.3/b8 | TXC-33–35 | Internal attribution, postpublication delivery and outcome stability after callback/effect failure |
| 5.3/b9 | TXC-36–38 | External token freshness, no invented isolation and required host guarantee or refusal |
| 5.3/b10 | TXC-39–41 | Publication versus history grouping, conditional typing grouping and indivisible linked identity |

Twenty-two clauses reference at least one configured positive bounded property;
nineteen have no direct property listed. These are reference counts, not proved
clauses, percentages or independent evidence. Some unlisted clauses are scope
constraints; others need actual representations, adapters, runtime call graphs,
public metadata, provider evidence or implementation refinement. The complete
selected source text is covered, not every possible semantic subclause of the
whole requirements document. A clause can still contain a coupled predicate with
multiple participants; textual partition does not prove semantic indivisibility.

Each listed property supports only the explicitly limited relation. For example,
Transactions models abstract grant/context/revision validation, not full resource
or capability contracts; EditorContracts has one representative boundary edit;
its batch TLC excludes adjacency. Separate byte-edit oracles/SMT arguments supply
additional adjacent-range support, not a complete canonicalizer. The diagnostic
context-handoff property does not prove arbitrary native callbacks or effects.

## Instrumentation and security at the boundary

IF-03 / INS-02 / the existing `publication` profile remains the common placement.
Per-clause risk descriptions identify what enforcement or conformance must
challenge; they do not make diagnostics an enforcement path.

- Preparation, final validation and publication have separate local timing
  boundaries; use actual available monotonic observations, not inferred display,
  external-write completion or missing-clock zeros.
- Count one logical operation at authoritative disposition and participant work
  separately. A rejected participant is not a partially committed transaction.
  Offered/admitted/rejected/faulted, pending age and resource saturation retain
  the existing MELT/golden-signal and USE/RED distinctions.
- Internal operation/generation/before-after revision evidence is authoritative;
  optional events/logs/traces are not the commit receipt. Sampling, off/full/loss
  or sink failure cannot turn Applied into Rejected or erase authoritative facts.
- Keep callback formatting/export waits outside publication. Capture only the
  existing bounded allowed primitives; no raw candidate/source/annotation content,
  protected topology, paths, tokens or high-cardinality object/principal labels.
- Observe delivery through IF-02/IF-08 / INS-03 under current recipient policy;
  diagnostic drain uses IF-09 / INS-12. Metadata required internally is not
  automatically authorized for a recipient. External effects retain IF-04 /
  INS-04 attribution and unknown-outcome limits.

The per-clause risks refine the existing STRIDE/LINDDUN review: source/operation
spoofing, stale/torn-state tampering, misattributed outcomes, metadata disclosure,
unbounded/reentrant work and authority escalation. They add no new threat-control
implementation or numeric latency/resource claim. Actual field schemas, overhead,
clock/profile choices and sink/provider behavior remain their existing gates.

## Validation and completion boundary

Run `node formal/publication-clauses.mjs --self-test`. The full verifier runs it
from captured source bytes. The checker requires the current source fingerprint,
exact ordered reconstruction of all ten source units, 41 unique ordered IDs,
existing contract owners/decisions and exact configured positive case/property
references. It also checks the declared publication observation placement.
Sixteen controls reject stale/missing/altered records, invalid owners/evidence,
control/witness promotion, missing residuals, wrong observation placement and
unsupported evidence-status promotion.

These are partition/reference checks, not verification of prose meaning or
conformance scenario execution. The whole suite must establish actual execution
and freshness of the referenced cases. All concrete refinements remain open;
independent review must still challenge clause granularity, correspondence and
missing scenarios. No D-row closes through this ledger alone.
