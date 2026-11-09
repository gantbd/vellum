# Annotation query products and snapshot pagination

Specification §13.8 selects F03's three native semantic products in
annotation-queries.json. These are not wire encodings, Rust layouts, new verbs or
a mandatory transport. §§12.8–12.9 and read-contexts.md subsequently select native
acquisition/stop and selector lifecycle products at existing F02/F19 boundaries.
Their registry reference composes the query functions here; it is not native
implementation certification. A scoped context selects one coherent retained annotation
projection. Enumeration, inspection and target-location read that same context;
they do not silently join independently current body, placement or source roots.

## Precedents and selected behavior

[Neovim extmark retrieval](https://neovim.io/doc/user/api/#nvim_buf_get_extmarks())
provides explicit buffer/namespace selection, ordered retrieval, a result limit
and optional details. Its limit bounds returned marks, not necessarily traversal;
Vellum must also bound preparation work. Extmarks are not authored comments or
an authorization model. [W3C Annotation Pages](https://www.w3.org/TR/annotation-model/#annotation-page)
provide ordered members within a collection and next-page relations. Vellum uses
the collection/page relationship, not HTTP, JSON-LD, global totals, public IRIs or
automatic retrieval. Neither precedent establishes snapshot consistency or safe
recipient projection; those follow Vellum's existing §12 obligations.

The base profile keeps one authorized ordered domain for a read context and
invalidates continuation when relevant policy/continuity changes. Refiltering each
page against an independently current collection would mix revisions and risk
skipped/duplicated records and hidden global offsets. Retaining a context costs
bounded storage and can become unavailable; it is not a lease on rights or memory.
The engine may implement equivalent indexed/retained projections instead of
materializing a separate full vector for every reader.

## Conditional arguments

Assume a truthful bounded coherent snapshot and host-established recipient,
destination/purpose, projection, current policy and continuity facts. Project
existence and fields before deriving ordering, page positions and continuation.
Equal authorized ordered projections then yield equal functional page and record
results for the same public query/selector schedule. Hidden records cannot create
visible gaps, global counters or totals. Withheld content is not an empty body;
withheld target data is not detached placement. A permitted detached result states
detachment without fabricating coordinates. Location describes the retained source
context, not an assertion of current placement or semantic applicability.

Every handoff rechecks current scope, grant, retained availability, lifecycle,
continuity and bounded transfer readiness. Tokens only select retained state.
Revocation, changed recipient/purpose, replacement incarnation or lost retention
cannot become an empty successful page or a newer snapshot under the old selector.
Regrant does not heal a visibility gap. A broader grant cannot expand the captured
ceiling; establish a new authorized context. The same unavailable public product
covers absent, hidden, stale, expired and conflicting causes in this base profile;
internal explanations and independently authorized diagnostics remain truthful.

Within a retained context, let N be the number of visible rows and k the private
next position. A successful nonterminal page with positive bounded limit advances
k by at least one and at most that limit. Thus N-k strictly decreases along a
forward traversal until its end. Each normalized reference call has finite bounded
loops. This does not prove termination of callers that deliberately replay pages,
arbitrary policy code, real registry operations or a global scheduler. Admission
must bound actual scan/preparation, retained bytes, row/result bytes and selector
issuance; a result limit alone is insufficient. Failure to establish complete
coverage is unavailable, never a false end. Operational interference allowed by
§12 remains observable; timing, contention and constant-time privacy are not proved.

## Executable evidence and boundaries

annotation-queries.mjs checks exact variant agreement with F03, 96 release cases
(three variants times 32 gate combinations), 512 paired query comparisons
(16 visible field masks times eight hidden states times four first/continued-page
and individual query observations), and 18 complete traversals over zero through
eight visible records with limits one and two.
Whole expected successful results and unavailable results are compared, with
capture/request/current input frames. Fifty-one controls cover forward/end/empty pagination,
scope and incarnation substitution, regrant gaps, cursor ambiguity and altered
limits/positions, missing/hidden/withheld results, detached placement, closed
request fields, owned input/output copies and preparation bounds.

The fixture uses normalized trusted rows, representative point/detached locations,
safe integer versions, string selectors and a supplied finite registry. Capture
projection, references/order and host current facts are premises, not policy truth,
entropy/non-forgeability, actual identity resolution or physical retention proofs.
JSON.stringify byte charges measure only the fixture rows, not an adopted wire
format, native heap charge or complete output envelope. Numeric bounds are test
parameters, not product defaults. Its private `next` is selector-issuance intent;
it must never be released as a raw offset. The executable does not mint final
public tokens or prove serialized registry issuance/owned handoff. Native schemas,
all target alternatives, actual source/annotation snapshot acquisition, historical
lookup, current-policy integration, adapter projection and complete family
refinement remain gates. No new SMT/TLC theorem or independent certification is
claimed by these same-author reference checks.

## Instrumentation

INS-03 observes bounded query/release dispositions, context availability,
continuation invalidation and resynchronization. INS-01/12 retain preparation,
capacity and diagnostic-loss facts separately. RED/golden signals distinguish
query service, wait, refusal and useful delivered rows; USE measures actual scan,
retained context and cursor pressure with declared units. No body, location,
selector, document identity or hidden count becomes a metric label. STRIDE and
LINDDUN cover forged selectors, stale grants, cross-recipient correlation and
hidden offsets; STPA/FMEA cover false completion, mixed snapshots, resource
exhaustion and misleading detached/withheld presentation. Actual fields, charging,
sink delivery and workloads require integration evidence.
