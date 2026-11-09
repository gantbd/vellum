# Native semantic configuration layer lifecycle

Specification §22.3.1.4 adds bounded layer creation/removal and complete target
reassociation to the existing F16 semantic-settings job. It preserves the same
owner, staged preparation, atomic root/outcome publication and retained
interpretation contracts. It introduces no filesystem-backed preference service,
frontend keymap state, callback execution or new native job kind.

## Identity and atomic activation

The private engine-settings-layers/1 request binds invocation, execution profile,
expected configuration generation and layer-incarnation high-water, create/remove
vectors, the complete proposed target associations and additional read dependencies.
Each created row carries an exact layer identity, legal scope kind and authored
values; its generation is initialized by the engine to one. Each removed row
names an exact current identity and expected layer generation. Existing layers
not removed retain their identities, generations and values unchanged.

The accepted initial configuration seeds its high-water from the maximum accepted
layer incarnation, or zero for no layers. Ordered creation rows must use exactly
high-water+1 through high-water+count, within the safe integer domain. Their labels
do not grant authority. The high-water advances only on successful publication
and never decreases on removal. Thus every newly issued incarnation exceeds all
previously accepted incarnations in this live owner lineage. Exhaustion refuses
creation; removal and valid no-op need no unused identity. Recompiling initial
declarations is not an allowed live reset. Cross-process restoration and a fresh
owner's namespace continuity remain separate identity premises.

Creation/removal and reassociation are one proposal. A removed layer cannot remain
referenced by any target; missing, duplicate, incompatible or ambiguous inputs
reject the whole candidate. Unused created layers are still type/scope checked.
Requests cannot replace surviving authored values or weaken schema/restriction
facets. Actual document/workspace applicability is a host premise, not inferred
from these declared associations.

Current ordinary settings gates apply to every request. Real layer changes also
require exact-true layerRegistryWritable and write permission for every created
or removed identity. Changed associations require associationRegistryWritable;
unchanged facets require no unrelated grants. Readability of a configuration
source does not authorize its activation. Trusted grant lists are not caller
request fields. No activation suspends real revocation.

## Versions and retention

Preparation captures the actual declarations, restrictions, layers, associations
and issuer high-water, plus mandatory/additional predicates. Current service
checks the same context, profile, expected generation, current permissions and
dependencies. Concurrent settings progress refuses this complete-set lifecycle
proposal; unrelated source-root progress is preserved. Retrying preparation does
not refresh the captured intent or dependencies.

A real change advances configuration generation. Association generation advances
only when the target list changes. Surviving target-domain generations advance
only for changed effective values; new target counters are contextual to the
association epoch. An unattached layer change therefore need not invalidate
unrelated source consumers. Only counters actually changed need spare capacity.
Bound total creation/removal rows, aggregate serialized created-value objects,
request/tree/identifier/target/layer/state/plan bytes and the parent reservation.
A valid no-op still validates bounded input and current authority, but consumes
no publication, generation or identity allocation.

Removing a live layer does not rewrite old interpretation leases. They retain
their originally charged data until independently discharged. Pending cancellation,
faults and replay use the existing settings invocation lifecycle; stop alone is
neither retirement nor a refund of lifetime issuance. Source, annotations, views,
history and unresolved effects remain outside this configuration mutation.

## Precedent and evidence

[CodeMirror compartments](https://github.com/codemirror/state/blob/main/src/facet.ts)
support replacing a selected configuration portion through a transaction, and
configuration resolution rejects duplicate compartments. That supports staged
coherent reconfiguration, not Vellum identity issuance, permissions, reclamation
or host applicability. The monotonic bounded issuer follows Vellum's existing
non-reuse and lifecycle direction rather than importing a global naming service.

Run engine-settings-layer-checks.mjs: 22 controls, 25 malformed products,
22 grant cases, five budget cases, ten version cases and six argument cases,
with seven assertion-killed source mutations after a coherent baseline.
Twenty-four prior activation and 27 definition controls remain. Reused-incarnation
and wrong-removal-generation cases carry otherwise sufficient permissions and
coherent associations so unrelated refusal cannot mask the guard being tested.
Common cancellation covers 25 forms/twelve kinds: 400 gate/200 context cases,
50 races and 6,475 traces/18,150 edges, ten controls/twelve mutations.

Eight BASE-LAYER cases separately check the declared scalar issuer transition:
six nonvacuous obligations for range, freshness, nonrefund, exhaustion, no-op and
disjoint-domain frames, plus two satisfiable weakened-rule witnesses. They bring
the baseline manifest to 81 conditions and all fourteen manifests to 295. These
conditional laws are not source-extracted refinement proofs. Complete consumer,
registry/result/event products, host policy/applicability, public codecs, Rust,
physical/concurrent/crash and independent assurance remain separate gates.

## Observation and threat coverage

IF-01/03/07 and PATH-01 retain admission, interpretation and publication ownership.
INS-01/09 distinguish malformed/unsupported admission and context/version refusal;
INS-02/03 distinguish unchanged/applied/nonpublication and historical outcomes;
INS-08/12/13 cover plan/state cost, identity exhaustion, held leases and protected
retirement. MELT/golden/RED/USE remain design contracts, not installed telemetry.
STRIDE addresses forged identities/grants, stale reassociation and exhaustion;
LINDDUN addresses configuration provenance and linkage. Ordinary labels use bounded
reason/profile codes, not keys, values, identities, high-water marks or raw paths.
