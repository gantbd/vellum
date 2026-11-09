# Native annotation decision receipts and current release

Specification §13.6.4 closes the selected normalized F13 identity/version result
products. Actual create/body/reattach/delete decisions capture a small immutable
receipt at the same trusted serialized root/fact boundary. Current release meets
the captured disclosure ceiling with an invocation/record-bound current grant;
later edits, undo/redo and eviction never reconstruct the receipt from a newer row.
Public wire/alias encoding, actual grants and production refinement remain separate.
[Native current-read composition](annotation-reads.md) separately checks §12.8.2:
current query captures, old writer receipts and stream identities remain independent,
without direct receipt/event-to-selector conversion. The receipt checker below
retains its own narrower scope; actual query integration is not inferred from it.

## Precedent and adopted boundary

[SQLite RETURNING](https://www.sqlite.org/lang_returning.html) supplies a useful
precedent for returning directly affected record values, including generated
identifiers, without a later lookup; deletion reports the removed values. SQLite
explicitly distinguishes mutation processing from transaction commit. Vellum does
not adopt SQLite, SQL syntax, trigger semantics, storage or durability. Its local
root/fact/receipt installation follows its own existing publication protocol.

[Cedar authorization](https://docs.cedarpolicy.com/auth/authorization.html) evaluates
a principal/action/resource/context request and defaults to denial without a
permit. This supports explicit scoped disclosure rather than deriving read access
from a successful write. The reference does not implement Cedar, authenticate a
principal or prove the host's current grant facts.

## Closed result products

The observation wrapper is exactly annotation-result plus a result, or the common
unavailable product. Inside a known result, the existing three F13 decisions remain:

| Decision | Mandatory facts | Conditionally disclosed facts |
|---|---|---|
| applied | status, original invocation/operation, operation-bound publication reference, native annotation reference, historical membership | contentVersion and targetGeneration under their respective ceilings/grants |
| unchanged | status, original invocation/operation, native annotation reference, historical membership | contentVersion and targetGeneration; no new publication reference |
| not-published | status and original invocation/operation | none; no candidate identity/version or target-existence facts |

Applied/unchanged record products require identity disclosure at both admission and
release. Otherwise detailed observation is unavailable; independently permitted
coarse F06 facts can still be observed. Pending, unknown/faulted, forgotten or
retired result bindings are unavailable, never inferred as not-published.

The native reference has the already selected owner document/id/incarnation
semantics. Its normalizer representation is not a selected public wire encoding,
globally broadcast identifier, event reference or stored-JSON tuple. Its explicit
disclosure grant permits this native identity product; possession grants neither
read nor write authority. Any later command still needs current owner/identity,
authority, interpretation and authored-version checks. A historical reference may
be stale or absent now. Query/observer aliases are not automatically converted.

In this finite reference, the publication reference is the original once-only
invocation's causal anchor, not the authority root counter or a read selector.
Applied identifies its one coherent local publication; unchanged issues no new
publication anchor. Membership is historical present or retained-absent at that
decision. Authored delete is not final identity retirement. No body, target value,
source bytes, annotation-set counter or raw root snapshot is retained in a receipt.

## Capture and bounded ownership

Owned native preparation identifies the actual original target, including the
already-issued create allocation. Admission checks recipient/selected profile and
authority consistency and an optional exact trusted annotationResultGrant:
invocation, native annotation reference and three Boolean disclosure capabilities.
Scope mismatch, unknown fields, non-Boolean values or invalid capability dependence
refuse admission. Missing detailed authority captures an all-false ceiling, not a
write grant and not an inferred permission. The captured ceiling is copied/frozen.
Matching retry cannot change it.

Reserve the complete private receipt and largest permitted public frame against
maxResultBytes before pending execution. The synthetic bound uses the owned target,
longer membership label and maximum safe private counters; no body/location size
is guessed. The shared declared charge is now two root maxima plus context bytes
and, per slot, four intent maxima and five result maxima. The two added result
units cover the native receipt and native release frame beside the existing coarse
policy/result/frame reservation. This is explicit logical accounting, not heap/RSS
measurement or a new public numeric default.

After actual candidate/outer-root validation, capture the resulting record's native
identity and both authored versions. A body edit sees an independently advanced
current target generation, and reattachment sees an independently advanced body
version. Delete uses the actual retained absent payload/version floors. Unchanged
uses validated current facts without incrementing source/publication/history.
Known refusal or stop stores only the minimal not-published decision.

Validate the closed receipt vocabulary and bound before installation. A surprise
or oversized private receipt faults/fences with conservative Unknown and no native
receipt, not fabricated nonpublication. Trusted final assignments install root,
coarse facts, receipt and done phase together. The receipt contains copied scalars/
native identities only, not references pinning old document or native-parser roots.
Result eviction clears both caches under retention/drain checks. Retirement removes
the owned ticket; neither operation restores eligibility or resets issuance.

Receipt/ceiling fields are declared in the issued slot, not added structurally at
final publication. This keeps the final reference step to the existing trusted
assignments; it does not prove that a VM allocator, concurrent native runtime or
physical memory behaves as the logical reservation declares.

## Current disclosure and finite lattice

Let i mean identity disclosure, b content-version disclosure, and t target-generation
disclosure. Versions require identity, giving exactly five admissible capabilities:
empty, i, i+b, i+t and i+b+t. Their order is set inclusion, meet is intersection,
join is union, bottom is empty and top is all three. Join is a mathematical upper
bound, never the release rule. They are the downsets of the
finite prerequisite order i below b and t; hence a bounded distributive lattice.
Malformed prerequisite combinations are rejected, not silently interpreted.

Current release requires the exact retained invocation context, current common
release gates, original coarse publication ceiling and epoch, and matching detailed
grant invocation/native identity. Identity and each version field meet captured
and current capabilities. A later broader grant cannot amplify the captured
ceiling. The coarse D05 details flag still controls its own participant envelope;
native result identity/version grants are a distinct explicitly scoped product,
not a way to enable raw F06 details. Missing or unbound grants release no record.

Returned values are copies. Receipt membership/versions stay facts about the
original decision after later work, deletion, restoration, disposal or a later
domain fault. They are not an assertion that the annotation remains live. Current
policy must still permit historical result observation; valid old facts do not
revive a withdrawn grant or old command precondition.

## Conditional arguments and solver scope

Under the existing serialized owner, authentic current grant/identity/retention
facts and finite budgets, admission reserves a bounded copied ceiling and complete
receipt/frame capacity. Native ownership and actual candidate validation select
exactly one matching live or retained-absent record, or a minimal known refusal.
Closed receipt validation and final installation establish agreement of one local
root decision, coarse fact and receipt. Fault/refusal paths cannot install a
candidate-only identity or falsely successful receipt.

After installation there is no row lookup in result release. An induction over
later edits/history/observation/forget/retire transitions preserves the original
receipt or explicitly removes its availability; it never makes a spent invocation
fresh. Scoped current release and capability meet establish non-amplification
conditionally on host truth. The five-element disclosure lattice says nothing
about categorical completeness or a lattice of all editor operations.

Six additions in scoped-release.smt2 check four positive local conditions and two
SAT controls: capability/bounds closure, meet non-amplification, scoped decision
binding and epoch exclusion; controls witness unsafe current-only grant use and
later-row version substitution. At this receipt batch the scoped manifest had
fifteen conditions (nine original plus six), and the suite had 205 over thirteen
manifests. Sections 14.5.1/14.6.1 separately add six history and six reversal-receipt
conditions, bringing current totals to 27 scoped/217 suite conditions over those
same thirteen manifests.
Trusted solver results concern these declared predicates, not automatic code/
TLA+/Rust refinement or independent certificates. Finite reference calls use bounded
record/slot/field loops and existing native operators, not callbacks or user code;
hostile decoding, actual native scheduling and physical halting remain unproved.

## Executable evidence and instrumentation

annotation-result-checks.mjs composes the actual native producer/adapter/shared
ledger and native history path: eight capability candidates, 100 captured/current
projection combinations, 512 release combinations, fourteen controls and eighteen
rejected mutations, with a passing unmodified dynamic baseline. Controls distinguish
pending/Unknown/no-op/refusal, source-independent field merging, historical receipts,
fresh-check reference use, context/epoch/grant scope, preadmission capacity, outer-root
refusal, copy isolation and eviction/retirement. Mutation detection requires actual
assertion failures, not load errors. These are selected finite cases, not exhaustive
reachability, a concurrent implementation or all operation-result profiles.

PATH-01/03/04/12 and IF-01/03/08/12 retain admission, local publication, history and
release boundaries. INS-01/02 track reservation/current validation/disposition and
paired fact/receipt installation; INS-03 current release, denial and eviction;
INS-06 retained history; INS-07 obligations; INS-12 diagnostic loss and INS-13 useful
completion. Trusted invocationUsage adds retained receipt count/serialized bytes;
those are private supervision data, not ordinary F06 fields or physical memory.
MELT/RED/golden signals distinguish admission capacity, committed facts, withheld/
lost observations and phase latency; USE covers receipt/intent/root/slot pressure.
STRIDE challenges substituted grants, stale scopes, false receipt publication and
replay; LINDDUN challenges identity linkage, protected versions and root/body leakage.
Raw identities, versions, invocations and source data never become ordinary labels.

Remaining: public wire/alias/query-selector integration, real issuer/PARC/grant/
retention truth, complete retirement/general target mapping/point storage, other
family/effect/lifecycle result products, actual concurrent/physical/decoder/native/
production and independent refinement. No whole family, D-row or all-sections goal
is closed by this result-profile evidence alone.
