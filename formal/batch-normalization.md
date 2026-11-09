# Unambiguous batch normalization

D02 now adopts one shared admission normalization path: accept unambiguous
unsorted original-snapshot edits, order them canonically, retain complete edit
records/boundaries and reject unresolved ambiguity. This refines requirements
§5.7 and specification §4.2.1, not a new editing mode, storage structure or public
request schema. D01 still governs semantic identity after required validation;
normalization itself neither drops identity-looking records nor creates commits.

## Contract and general argument

For one identified per-document starting context, consider a finite collection E
of complete edit records with valid byte ranges, expected source contents and
associated semantic metadata. Its admissible domain has distinct starting
positions, nonoverlapping interiors, and only nonempty replacements when two
ranges touch. Insertion inside a replacement, coincident insertions and insertion/
replacement endpoint ties remain rejected until the caller supplies an explicitly
resolved unambiguous request. Arrival order, correlation tags or author labels
do not resolve ambiguity.

There is exactly one permutation of E in strictly increasing start order.
One construction sorts complete records; another assigns each record the rank
equal to the number of records with a smaller start. For a finite set with
distinct starts, remove its least element: its rank is zero, and ranks of all
remaining elements decrease by one. Induction on collection size establishes
that ranks are a bijection onto `0..n−1` and reconstruct the unique sorted list.
Thus normalizing a permutation of the same complete records yields that same
list, and normalizing the result again changes nothing.

Because this is only a permutation, every record's expected contents, context,
payload and metadata remain associated; no edit is split, joined, removed or
duplicated. Consumers of the common ordered change semantics therefore see the
same source, mapping and conditional inverse inputs. This preserves the existing
profile's semantics; it does not select an endpoint profile or prove that an
actual implementation supplied every required semantic participant.

The finite-list argument is authored mathematics, not proof-assistant replay or
verification of a production sorting algorithm. Local SMT lemmas and finite
executable comparisons below provide separate scoped checks.

## Boundary preservation counterexample

For source `abcd`, replace `[0,1)` with `X` and `[1,2)` with `Y`. Both separate
edits and a merged `[0,2)` replacement produce `XYcd`. Under the existing
representative left-affinity/deleted-interior-detaches rule, a point at shared
boundary 1 survives the separate edits at 1, but lies inside the merged replaced
range and detaches. That profile is a reference case, not a new production default.

Consequently byte equality alone cannot justify merging adjacent edits. D02's
normalization preserves their boundaries. A compact storage representation may
still refine the full logical contract while retaining equivalent boundary,
mapping, inverse and metadata evidence; normalization does not authorize loss.
Existing byte-oracle coalescing comparisons establish only byte equality, not
whole-operation or mapping equivalence.

## Executable reference checks

`batch-normalization.mjs` compares copy/sort/ordered-validation against pairwise
ambiguity validation plus rank assignment. Both share fixture shape/source/
expected-byte checks and trusted byte-oracle helpers. They are same-author,
algorithmically different reference constructions, not independent Cleanroom
expectations or production engine code.

The finite domain has all 40 strings of zero through three scalars over `a`, `é`
and `🙂`, insertion choices empty/`X`/`🙂`, empty batches, every single insertion
boundary, and all nonoverlapping nonempty range batches of up to three edits.
It contains 3,067 canonical batches and 8,008 permutation evaluations: 40 empty,
1,002 one-edit, 2,592 two-edit and 4,374 three-edit evaluations. Each checks both
constructors, idempotence, complete record preservation, expected forward bytes,
reverse inverse recovery and unchanged source/request/metadata. Reference identity
checks are stronger than value preservation; production representations need not
share JavaScript object references.

Fifteen invalid-domain controls require their intended private diagnostics:
overlap, coincident/start/end/interior insertion ambiguity, duplicate edits,
expected-byte mismatch, wrong context despite equal bytes, invalid scalar/insert
bytes, sparse insert/expected/list arrays, invalid bounds and exhaustion. Rejecting
these examples preserves the source/request data; it does not prove the real
coordinator's publication frame. Eight controls expose sparse comparison, input-
order dependence, in-place sorting, omitted/swapped metadata, empty-record
elision, container-order tie resolution and silent adjacent coalescing.

One additional fixture normalizes equal-offset edits independently in two document
contexts. It checks context separation only, not atomic multi-document publication
or grouping discovery. Fixture context/tag/expected/meta fields and explicit
three-edit limit are private test notation, not selected public fields, IDs,
capacity defaults or a complete resource envelope. Source validation/comparison,
sorting and metadata retention still need actual byte/work/memory/time profiles.

`normalization-laws.smt2` adds four local positive conditions (SAT premises then
UNSAT negated conclusion): two-record idempotence, permutation independence,
whole-record framing and adjacent-map separation. Three SAT controls exhibit
input-order dependence, independently swapped metadata and tied-start ordering.
Integer payload/metadata/context symbols are opaque stand-ins, not encodings or
hash-equality guarantees. The exact ordered `normalizationCases` manifest uses
trusted Z3 4.15.4; it is not arbitrary-list code refinement or certificate replay.

## Authority observations and remaining refinements

Normalization is private preparation at IF-01/03, PATH-01 and INS-01/02. It grants
no authority, repairs no stale revision and does not publish per-document partial
results. The coordinator still validates all participants/current dependencies
before atomic publication; any required failure preserves current state rather
than rolling back unrelated work. Existing PS-TX/BR/NO evidence remains separate,
not a newly established complete composition proof.

Observe bounded input size, normalization cost, ambiguity/refusal/exhaustion and
final disposition separately from commit counts. Preserve logical requests versus
edits/participants/attempts and unknown versus zero; no raw metadata, private
targets or identity-valued metric labels are introduced. STRIDE/LINDDUN cover
context/provenance swapping, accidental coordinate mixing, ambiguous intent,
hidden participant disclosure, adversarial sort/fan-out pressure and diagnostic
interference. Telemetry neither resolves ties nor supplies authorization.

Public schemas/correlation, actual finite budgets, caller-resolved tie lowering,
all mapping/grouping profiles, owned metadata/alias safety, host services and
concrete canonicalizer/coordinator refinement remain open. No new origin label,
network service, generic callback or implicit no-op elision policy is adopted.
