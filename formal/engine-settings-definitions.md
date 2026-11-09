# Native settings definition and association activation

Section 22.3.1.4 separately adds bounded layer lifecycle without letting the
definition product replace authored layers. See engine-settings-layers.md for
monotonic incarnation issuance, current grants, coherent reassociation and retained
interpretations. Layer lifecycle is no longer an unrepresented reference path;
complete consumer/public/host/physical/concurrent/crash/independent gates remain.

Specification §22.3.1.3 supplies private `engine-settings-definition/1` requests
through F16 `supported-registry`. Definitions and associations use the existing
settings job, owner, preparation pool and atomic root/outcome publication. They
do not replace authored layers or introduce another configuration authority.

## Declaration updates and authority

The exact normalized product separates definition updates from layer set/remove/
reset. VS Code's primary [configuration registry](https://github.com/microsoft/vscode/blob/main/src/vs/platform/configuration/common/configurationRegistry.ts)
likewise distinguishes configuration/default registry deltas and schema/update
notifications; its specialized schema notification permits enum changes only.
That is precedent for distinct responsibilities, not Vellum's mutation vocabulary,
atomicity, retention or authorization proof. Vellum's contracts derive from
requirements §11.2 and its existing bounded owner model.

The proposal replaces schemas, value restrictions and target associations while
preserving the actual current authored layers and their generations. Re-resolution
must succeed for every target without orphaning even unused authored contributions.
Changing a facet requires its own exact-true current write grant as well as ordinary
settings/authority guards. Unchanged facets require no unrelated write right.
Value restrictions never stand in for actual host authorization or revocation.

Normalization bounds request bytes/nodes/depth/identifiers; declaration/state/type/
target and plan limits apply separately. Set-value quotas belong to the layer
request product, not an implicit new per-definition quota. Unknown callbacks,
properties, cycles/accessors and unsupported values cannot enter normalized owned
data. The six closed types remain a private settings vocabulary, not a universal
JSON Schema implementation or document-content schema provider.

## Preparation and publication

Preparations capture all current declaration, association and authored-layer
dependencies. Current service cannot reinterpret an old preparation against new
schemas or newly edited layer values. Layer preparations now also capture the
association epoch, closing an ABA hole before live association changes are added.
Mandatory and additional dependencies remain conjunctive; unrelated source-root
progress alone does not stale this configuration footprint.

The candidate advances configuration and changed-facet epochs without resetting
them. Existing target/domain counters change only when their effective key/value
mapping changes; schema membership changes count even when surviving values are
equal. New target counters are contextual to the association/interpretation epoch.
Old leases keep their captured interpretation. A failing or exhausted transition
publishes nothing; a bounded valid no-op needs no unused counter capacity.

The common settings path preserves source journals, authored document/annotation
state, history and effect obligations. Retry, protected cancellation, parent
reservation, retirement and spent issuance use the existing owner mechanisms.
No new job kind, network component, filesystem mechanism or frontend dependency
is added. Layer creation/removal and actual applicability remain separate work.

## Recipient observation

The actual F07 reader observes changed permitted values/declarations and preserves
its captured ceiling when new definitions appear. Native target withdrawal requires
a separate owner-state check: an absent target cannot produce a current visibility
packet. After binding the recognized request/context/scope and passing current
read gates, validated actual absence invalidates only that captured target context.
Restoration cannot revive it; foreign and transient queries do not assert loss.
Delivery invalidation does not refund issuance or discharge retained obligations.

## Executable evidence and symbolic obligations

Run `node formal/engine-settings-definition-checks.mjs`. It retains 24 existing
activation and 22 reader controls, and adds 27 controls including 480 native scalar
resolution cases checked against an independent precedence oracle. It checks
23 malformed requests, six budget cases and seven version cases; refusal leaves
the root and preparation/parent accounting unchanged after invocation issuance.
Five mutants must fail assertions after a coherent four-module baseline. They
remove association fencing, authored-layer dependencies, registry grants,
definition publication and absorbing target-withdrawal detection. Import, syntax
or unrelated exception failures do not count as kills.

Shared `invocation-control-checks.mjs` now exercises 23 forms across twelve kinds:
368 gate, 184 scope, 46 race cases and 5,957 traces/16,698 edges, with all twelve
existing control mutants retained. These are bounded controlled interleavings,
not physical concurrency evidence.

Nine BASE-DEF conditions in baseline-contracts.smt2 define a symbolic transition
over opaque authored data, contextual epochs and a selected effective-domain
counter. Seven positive obligations first establish satisfiable premises, then
exclude violations of authored frames, safe ranges, facet authority, exhausted
no-op behavior, association ABA fencing, native absence detection and absorbing
loss. Two satisfiable weakened-rule witnesses demonstrate value-only ABA and
packet-only loss failures. The symbolic transition is not extracted from JavaScript
and does not prove Rust/JavaScript refinement. Totals are 73 baseline conditions,
287 across fourteen manifests. Full frozen-input verification remains required.

## Observation and remaining integration

INS-01/02/09 distinguish preparation, current grant/dependency refusal and common
publication; INS-03/08 cover recipient target loss; INS-12/13 cover status, retained
contexts and discharge. MELT/golden/RED/USE use bounded reason/count/occupancy
populations, not raw setting keys, values, targets, handles or epochs as ordinary
labels. STRIDE/LINDDUN cover forged preparations, stale interpretations, authority
substitution, metadata linkage and exhaustion at those same interfaces.

Actual association/policy truth, all consumer adoption, new-layer lifecycle,
all-family discovery/events/registry integration, public codecs/Rust packaging,
measured limits, physical/concurrent/crash refinement and independent assurance
remain gates. This narrows selected C1/C2/C3 integration gaps, not whole F16 or v1
closure. The initializer remains an initializer, not a live activation authority.
