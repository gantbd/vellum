# Owned preparation and required evidence boundaries

Specification §§26.5–26.6 binds an immutable owned candidate to its retained
descriptor/claims, scoped selector, current source/context validation and bounded
required-report owner. Definitions are explicitly supplied and their relevant
import closure is captured, not silently fetched from names. This refines D03/
D09/D26/D33/D35/D46 without adding a new editor mode, service or schema runtime.

## Direct sources and adopted limits

| Primary source | Mechanism used | Not imported |
|---|---|---|
| [Roslyn workspace model](https://learn.microsoft.com/en-us/dotnet/csharp/roslyn-sdk/work-with-workspace) | Immutable solution/document snapshots and explicit application of a changed solution support prepared-value versus publication separation | C#/VB compilation model, a required workspace compiler or Vellum atomicity proof |
| [Neovim API metadata](https://neovim.io/doc/user/api/#api-metadata) | Typed signatures, API levels and discoverability support one retained executable contract per operation | Neovim public identifiers, RPC, permissive extension policy or authorization guarantees |
| [IntelliJ threading model](https://plugins.jetbrains.com/docs/intellij/threading-model.html) | Recheck object validity across preparation phases; keep protected mutation work short | UI-thread writes, application-wide RW lock or a copied scheduling model |
| [Rust Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html) | Shared ownership can retain immutable values | A particular pointer type, absence of interior mutation or physical resource accounting |
| [JSON Schema 2020-12 core §8.2.3](https://json-schema.org/draft/2020-12/json-schema-core#section-8.2.3) | Schema references identify definitions and do not imply automatic network retrieval | A core JSON Schema dependency, actual dialect/version choice or safe validator implementation |
| [SHACL 2017 §3.4.1](https://www.w3.org/TR/2017/REC-shacl-20170720/#failures) | Validation execution failure is distinct from a conformance result | SHACL as the required content schema or a claim that partial reports prove global conformance |

These are mechanism precedents, not evidence of equivalent upstream transaction,
security or refinement guarantees. Definition/URI adapters remain separate from
the authoritative editor core. Sources were consulted for this goal pass; linked
versions/status matter, and a newer draft is not silently substituted for a
published Recommendation or an adopted product profile.

## Conditional arguments

If preparation takes immutable ownership of every relevant semantic field and
selectors never alias a different live value, a resolved selector has one fixed
candidate meaning for its retained lifetime. Capturing the descriptor interpretation
and deriving required claims from it prevents caller mutation or metadata discovery
from lowering obligations. Subject matching then refers to the owned candidate,
not a provider's replacement label. Final current dependency/authority checks and
existing exact publication are still necessary; immutability is not permission.

Required intake checks authentic/admissible issuer and exact required key before
it can affect that candidate. An equivalent duplicate adds no new fact; a reused
issuer-scoped identity with different payload cannot overwrite the first record.
Contradiction, protocol uncertainty or retention exhaustion prevents established
satisfaction from authorizing publication. This is a safety rule, not a promise
of unlimited report storage or guaranteed external delivery. Actual owners must
retain responsibility or invalidate affected preparation without losing other
held obligations. Unrelated/forged records cannot obtain this blocking authority.

For a finite authorized definition graph, every newly visited node is added once;
every processing step consumes bounded work. An exhausted budget produces no
complete closure. On completion the visited set contains the root and all imports
of its members; any import-closed superset of the root contains every node added
by induction. Thus it is the least reachable definition closure. Capturing each
included definition's sound immutable identity/version, with validation options
separately bound in the assertion/context, detects changed imports without depending
on unrelated registry entries. The fixture assumes truthful definition versions;
it returns IDs/versions, not actual definition bytes/import adjacency/options.
This does not prove recursive schema evaluation,
URI resolution, dynamic reference semantics or provider validation termination.

## Executable scope

`preparation-boundary.mjs` checks 256 current-admission combinations through owned
preparation, required report satisfaction and the source-only exact-edit reference.
Thirty controls cover immutable capture, caller mutation, authority/session/retired
selectors, descriptor redefinition/removed claims, changed context, bounded issue/
non-wrap, ordinary editing independence, issuer-scoped duplicates, authenticated
but inadmissible/unrelated records, contradictory intake, identity faults and
capacity blocking publication. Input/rejected-state frames are checked on the
matrix and exercised rejection paths. Literal successful source edits prevent
reject-all vacuity. Report identity comparison occurs only after normalized key/
issuer validation; hostile shape/authentication remains a supplier gate.

`definition-closure.mjs` checks all 512 three-node directed graphs and three roots:
1,536 cases. Its independent oracle intersects every import-closed superset rather
than repeating traversal. Six controls cover cycles, work exhaustion, missing or
unauthorized imports, changed imported versions and unrelated definitions.

Fixtures use owned frozen JavaScript objects, session labels, finite serial/capacity
limits, normalized trusted descriptors/reports and simple static import graphs.
They do not choose public Rust/token encodings, crypto/hash algorithms or numerical
product profiles. Normalization, shape decoding, authentication, issuer-policy
generation coherence, actual compatible-descriptor relations, allocator/lifetime
and full companion identity/eligibility are still suppliers. Source-only companion
assumptions come from source-edit-contracts.mjs; D03 dedup/status and real concurrency
remain separately scoped. No new machine-checked theorem or automatic whole-engine
composition follows from these tests or authored arguments.

## Instrumentation and remaining work

INS-01 distinguishes descriptor resolution, preparation, selector lookup and final
admission. INS-05 records required intake applicability, duplicate/fault/conflict,
closure unavailable/exhausted and retained responsibility; INS-02 records actual
publication/no-op/refusal; INS-03 gates candidate/report release; INS-09 covers
descriptor/definition activation and retained interpretation. Account candidate,
worker, definition and evidence lifetimes under the existing resource profiles.
MELT/RED/USE/golden measures must not conflate preparation, validation and commit,
or turn missing retention/coverage into zero/healthy status.

STRIDE/LINDDUN/STPA/FMEA cover selector substitution, report-ID collisions, forged
intake poisoning, URI/definition substitution, hidden subject/provenance leakage,
retention floods, wrong-candidate validation and false publication after dropped
negative evidence. Concrete fields/units/capture costs and real supplier/native
fault/privacy/resource evidence remain integration gates.
