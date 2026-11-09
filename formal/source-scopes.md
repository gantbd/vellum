# Exact bounded source operation scopes

Native `/4` and `/5` explicitly inherit the `/3` source scope fields and predicates;
their added read kinds do not widen write restrictions or acquire mutation rights.
See configuration-reads.md and workspace-reads.md. Older profiles retain their
exact products, and unknown versions still refuse.

Specification §5.2 supplies concrete caller write restrictions in the native
compound `/3` product. Replacement cells and pure-insertion sites are independent
sets at the original document revision. Commands and F10 advice use one predicate;
scope does not create a new owner, grant, presentation mode or external write.

## Precedent and chosen limits

[Emacs narrowing](https://www.gnu.org/software/emacs/manual/html_node/elisp/Narrowing.html)
restricts the accessible region without changing actual buffer positions; saving
still concerns the whole buffer. Its
[source manual](https://raw.githubusercontent.com/emacs-mirror/emacs/master/doc/lispref/positions.texi)
states these distinctions directly. Vellum keeps original coordinates and ownership,
not Emacs's display restrictions, character numbering, implicit clamps or authority.

[Emacs read-only properties](https://raw.githubusercontent.com/emacs-mirror/emacs/master/doc/lispref/text.texi)
make insertion next to protected text depend on boundary stickiness. This supports
explicit insertion semantics rather than assuming that text containment settles
every gap. Vellum does not adopt property inheritance or an implicit endpoint
default: the caller supplies the insertion set separately.

[VS Code ranges](https://code.visualstudio.com/api/references/vscode-api#Range)
provide immutable containment/intersection values. Its enclosing-range union is
not a safe substitute for a permission-set union across forbidden gaps. Vellum
also does not copy automatic endpoint swapping or UTF-16 coordinates.

## Product and execution

`source-scopes.json` defines the exact `/3` header and scope records. `/1` and `/2`
remain exact; no optional unknown field changes their meaning. `/3` retains `/2`'s
four closed read predicates. Each declared source participant has one matching
scope; annotations alone use an explicit empty scope list.

Replacement records `[lo,hi)` are nonempty. Insertion-site records `[first,last]`
include both endpoints and may be singletons. Both use original UTF-8 byte
coordinates with scalar-aligned endpoints. Ordinary canonical batch validation
still rejects any actual edit inside a scalar, even when a site interval spans
that numeric byte position. A nonempty replacement needs coverage of all removed
cells, not a separate pure-insertion permission at its start.

Admission bounds and copies the product, sorts only scope/set records, and rejects
duplicate subjects, overlaps and malformed extents. It does not coalesce adjacent
records or authored edits. Final validation rechecks exact identity/incarnation,
source revision, source-participant correspondence and aggregate scope/range/site/
identifier bounds. Scope absence is not known empty permission. All edits, including
equal-byte replacements and empty insertion records, must satisfy the constraint
before no-op classification or candidate changes. An empty batch has no offending
record but still retains its declared current scope/participant obligations.

The replacement predicate scans the sorted intervals from the edit's start. It
advances covered extent only through overlap/touching coverage and stops at a gap.
The insertion predicate independently checks the explicit closed site intervals.
The existing coordinator then performs its canonical edit, authority, annotation,
view, history and capacity checks. Source scope does not grant those rights or
make read-only context an authored/history participant. No provider I/O, separate
document or shifted local coordinate system is introduced.
The constraint belongs to this forward invocation, not standing document policy
or later undo/redo authority. Reversal retains its own frontier/current-policy
contract; this source profile does not silently implement scoped reversal.

## Conditional arguments

All loops range over bounded native lists. Scope normalization sorts finite
records; coverage advances through each interval at most once per edit. There is
no recursion, extension callback, arbitrary predicate or automatic scope mapping.
This establishes termination for inert finite products, not host scheduling or
physical memory guarantees.

For a normalized interval list, induction over its scan maintains that every
original-source cell between edit start and `covered` belongs to the union.
An interval starting after `covered` cannot cover that gap; sorted later intervals
cannot repair it. Reaching edit end therefore establishes complete removal
coverage. Pure insertions are a separate predicate over valid gaps. These checks
precede candidate cloning in the shared decision path, so a failed scope returns
the exact starting root and cannot publish allowed peers or evade scope as a no-op.
Ordinary canonical edit semantics preserve outside original cells and their order;
offsets may shift, but their owner/coordinate basis is not replaced by a slice.

For a fixed exact snapshot, let U contain one tagged item per scalar cell and one
per insertion gap. P(U) is a finite bounded lattice under union/intersection, with
empty/full bounds. Effective authority must not exceed either caller constraint
or independent current policy. This is a mathematical model, not proof that a host
supplies truthful per-cell policy. Different incarnations/revisions have different
universes and do not acquire an implicit join or remapping operation.

Compact record limits are admission limits, not semantic widening rules. Two
separated singleton cells may each fit a one-interval profile while their union
requires two records. Replacing that union by its hull would add a forbidden cell.
Thus a resource-limited encoding is not automatically closed under every lattice
operation. Refusal preserves the constraint; categorical closure of the engine
does not follow from this powerset model.

## Executable and solver evidence

`source-scope-checks.mjs` enumerates all 128 permissions over three scalar cells
and four gaps in `aé🙂`, all ten valid scalar intervals and both native routes:
2,560 command/query cases, including 1,056 actual publications. The independent
oracle uses cell/gap set membership rather than the implementation's interval
scan. Allowed results are compared with literal prefix/replacement/suffix output;
rejected compounds preserve the exact root, including the other write participant.

Fourteen annotation variants across capture/undo/redo produce 42 actual managed
states and 84 query frames. These are query frames, not additional scoped commands
on every managed history state. Fourteen controls cover boundary independence,
holes/touching, canonical retry identity, strict products, stale/missing subjects,
UTF-8 alignment, empty source/scope, no-op bypass, independent authority/history,
finite limits, copied intent/read independence and actual intervening publication.
Additional frames preserve latest unrelated peer state and allow annotation-only
plans without unused source-scope limits.

A coherent five-module graph baseline precedes sixteen negative mutations.
They weaken insertion endpoints, hole/extent coverage, independent source/scope
revision matching, final scope gating, UTF-8 boundary checks, duplicates/overlaps,
count/interval budgets, canonical ordering and no-op checks. Mutation runs use a
selected permission/native subset. Only semantic assertion failures count as kills;
syntax/import/runtime errors do not. Existing read, advisory, parser, compound and
native invocation mutation suites retain their separate regression scope.

Ten BASE-SCOPE conditions add eight nonvacuous laws and two satisfiable weakened
rules. Insertion/range laws use symbolic integer arithmetic over two normalized
ranges; permission intersection uses a seven-bit model. Grant/current/missing
scope laws are conditional gate abstractions, not compiled-code equivalence or
actual host truth. Baseline totals become 52 conditions, and the full suite 262
across fourteen manifests. Full frozen-input verification remains required.

## Interfaces and instrumentation

Scopes travel with existing command/query intent to the same local coordinator.
The same actual root/context and current policy boundaries apply; no identity
provider, network path or new executor is added. Read dependency, authored scope
and policy provenance remain distinct data roles. File/serialization and UI
adapters continue to act on the original document, not a new scoped owner.

INS-01 measures bounded scope decoding/normalization and admission; INS-02 measures
current containment/refusal and whole publication; INS-03 governs F10 advice release.
MELT/RED/golden measures separate scope conflicts, unsupported shape, pressure and
fault from successful publication. USE accounts temporary interval lists and
candidate/companion work. STRIDE/LINDDUN cover stale-scope substitution, forged
permission, endpoint confusion, hull widening, private extent leakage and floods.
Use bounded operation/profile/outcome categories, not raw document IDs, source
positions, bodies or permission sets as telemetry labels.

Remaining gates include mapped scope profiles, complete workspace/configuration/
semantic-index dependency domains, native privacy/wire products, actual host policy,
production/global/physical accounting, concurrent/crash refinement and independent
assurance. No whole F10/F11/F13, D-row or all-sections goal closes.
