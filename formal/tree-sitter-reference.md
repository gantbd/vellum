# Pinned native language reference

The incremental reference now asserts that its editable tree is a distinct copy
before any edit or native reparse. The existing structural, change-flag and capture
checks still verify preservation of the published tree afterward. The aliased-tree
mutant therefore fails at an explicit ownership boundary before exercising native
code with that invalid alias. Native runner/import/process failures remain failures,
not accepted semantic mutation kills; all thirteen mutation names remain required.

Specification §18.6 exercises actual Tree-sitter parsing/highlight queries over
the committed source bridge and independent Markdown fences. It selects a pinned
verification/reference asset set, not the editor's production dependency graph,
complete language service or supported frontend/host matrix.

## Assets and trust boundary

| Asset | Reference version | Role |
|---|---|---|
| [Python Tree-sitter](https://pypi.org/project/tree-sitter/0.26.0/) | 0.26.0 | Native parser/query API verification tool |
| [Tree-sitter Markdown](https://pypi.org/project/tree-sitter-markdown/0.5.4/) | 0.5.4, block and inline ABI 15 | Host syntax and packaged queries |
| [Tree-sitter JavaScript](https://pypi.org/project/tree-sitter-javascript/0.25.0/) | 0.25.0, ABI 15 | Registered independent child grammar and lexical queries |

The tested tool tuple is CPython 3.13.11 on darwin arm64. The manifest records
publisher source/wheel checksums and enforces installed native binary, Python
wrapper/type, query and MIT-license file fingerprints before and after each worker.
A deliberately mismatched fingerprint must fail. These checks identify the bytes
used, not a reproducible build, publisher authentication, cryptographic attestation
or isolated/certified toolchain. Other platforms/versions need separately reviewed
pins/evidence; they cannot silently pass by matching only package names.

The [Markdown upstream](https://pypi.org/project/tree-sitter-markdown/0.5.4/)
explicitly describes inaccuracies and an editor-highlighting goal. Its packaged
extension profile is therefore derived syntax evidence, not an authoritative
CommonMark validator. JavaScript/JSX coverage is likewise not a universal ECMAScript
conformance theorem. Source stays authoritative, including malformed/incomplete
programs, mixed endings, Unicode and fences. Neither a valid tree nor a highlight
capture grants mutation or establishes program meaning.

Native grammars/scanners are trusted host code. Only fixed installed assets are
loaded; document labels cannot name code, paths, commands or downloads. The worker
process and its deadline are verification mechanisms, not an adopted editor
executor, RPC/network protocol, runtime, preemption or native-crash sandbox promise.
Production integration still needs actual compatibility, packaging/license review,
retention, resource ownership and both required frontend/reference applications.

## Incremental input and immutable results

Cases use actual source publication and tree-sitter-utf8-source/1 rather than
hand-authored parser deltas. Copy the accepted native tree, apply every emitted
TSInputEdit, then parse the captured new source with that edited tree. Compare it
with a fresh tree under the same pinned language. The structural signature includes
named/anonymous/error/missing nodes, field names, byte ranges and LF/byte points;
native queries must produce equal capture names/ranges. Original tree signatures
and captures must remain unchanged. This checks finite incremental consistency,
not grammar semantic completeness or concurrent ownership.

The relation covers single edits, committed coalesced and multi-edit input, an
explicit fresh-source fallback, CR/LF/CRLF and multibyte data, and malformed source.
An observed same-length identifier edit has zero syntax changed ranges but different
lexical captures. Syntax changed ranges cannot serve as the complete highlight
invalidation oracle. Native invalid query construction preserves the prior query/
tree in its control; changing only a query can reuse that immutable tree. These
constructor controls do not prove the engine's full atomic context activation.

## Mixed source and mapping policies

The [upstream split-parser arrangement](https://pypi.org/project/tree-sitter-markdown/0.5.4/)
uses a block tree plus inline parses. The reference discovers inline content from
that tree, excluding named host children but retaining meaningful anonymous inline
punctuation. Native inline queries actually highlight prose; a block-only fallback
does not count as the required integration.

Each fenced block is independently parsed. The fixed host registry recognizes
ASCII case-normalized js/javascript aliases; unavailable JavaScript, unknown
languages including Mermaid, incomplete fences, empty supported bodies and
exhausted child-work budget have distinct outcomes. Mermaid fallback does not
invoke rendering or execute diagram/script content. A failed/unknown region does
not disable an unrelated supported fence or host styling. Label changes rederive
membership/maps in fresh host analyses; cached-result current acceptance remains
the F24/F28 gate, not established by this comparison alone.

Use [native included ranges](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)
in original document bytes/points. Within one fence, exclude identified host
continuation prefixes; these are source-map holes, not deleted source or new
ownership boundaries. Do not concatenate separate same-label blocks into a
program. General multi-fence, arbitrary discontinuous and recursive child injection
plans remain excluded. This narrow host-marker exclusion implements a declared
within-one-fence projection, not a general injection planner.

Child capture fragments are intersected with the exact included ranges for display;
host prefixes/delimiters stay host-owned. A derived contiguous edit target crossing
a hole or fence boundary refuses rather than clipping/widening. This rule does not
restrict ordinary exact-byte authoring across language boundaries or create a new
owner/history. The full proposal/publication/freshness bridge is not supplied by
this internal map helper. Native node identity is not permanent semantic identity.

Display policy gives child lexical captures precedence only within admitted child
ranges, inline captures over host block captures, then shorter spans and the
lexicographically first capture name as deterministic ties. Host literal fallback
remains elsewhere; named prefix
holes are not erased by a child body mask. These are reference semantic capture
layers, not a theme, font, pixels, frontend paint proof or universal editor policy.
Query property-assertion patterns are explicitly disabled in this lexical profile:
two packaged JavaScript patterns require a locals service which is not supplied.
They are not assumed true or advertised as resolved local-variable semantics.

Executing the full packaged highlighter with locals resolution is a valid richer
integration, but needs that actual service and its context/lifetime evidence. The
selected lexical reference keeps pinned upstream query assets and makes the
omission explicit; it does not make that omission a universal product default.

The document's shared child-analysis budget covers inline and fenced child parses;
aliases/tiny fences do not reset it. On exhaustion, retain host styling and explicit
region-budget/partial coverage without claiming all child work completed. The
worker also has finite verification input/source/node/capture/region/output caps.
Query match limits bound in-progress matches, not total captures or process memory;
post-result limits and a process deadline do not prove native halting/preemption,
physical accounting, fairness or product performance defaults.

## Evidence and remaining gates

The suite executes 195 incremental/unchanged cases, one explicit fresh case and
14 mixed-language cases through native binaries. Thirty-two controls plus the
fingerprint-mismatch control and thirteen load-success/assertion-failing mutants
cover immutable tree copy, actual native edit application, fresh equivalence,
host inline/child highlighting, independent same-label fences, source-map holes,
literal fallbacks, query-only behavior, document-wide budget and explicit predicate
exclusion. Parse/import/C-API failures, signals, launch errors and timeouts do not
count as semantic mutation kills. Tool absence does not count as a passing suite.

Python worker/manifest/check sources join frozen capture. Full verification now
requires VELLUM_TREE_SITTER_PYTHON; 49 harness controls include Python capture/
tamper and parser-only nongeneration. The native installation remains external,
with sampled pre/post fingerprints rather than a toolchain-isolation theorem.
Existing TLA+/SMT conditions retain their exact scopes; executing a native parser
does not make their supplier assumptions true.

Full analysis task/context admission, accepted-result sharing across views, actual
parser lifetime/ownership, incremental child reuse, complete grammar/highlight/
locals semantics, arbitrary source coverage, production ABI/dependencies, frontend
conformance, measured bounds and independent Cleanroom certification remain gates.
Requirements and annotation point persistence are unchanged.

## Instrumentation and threat coverage

INS-09 covers asset/ABI/query activation and failed-context disposition; INS-05
separates host parse, inline/region discovery, child work, query, mapping/fallback
and result acceptance. INS-01/03/08 retain interpretation/release and aggregate
capture/parser/tree/region/query/native-resource pressure; INS-10/11 own later
display submission, not parser success. MELT/golden/RED use matching workload and
coverage populations for queue/service/acceptance/fallback; USE retains distinct
native memory, parser slots, snapshots and work units. STRIDE includes unsafe
native loading, incompatible assets, forged labels/maps, stale nodes and query/
tiny-fence floods. LINDDUN includes source and deleted-text retention, language/
structure inference and shared-context reuse. Use bounded outcome/coverage/budget
classes, not source, labels, query text, paths, hashes or document IDs as metric
labels. Actual signals, privacy, overhead and native-host measurements need evidence.
