# Bounded engine semantic settings arguments

The subsequent §22.3.1.2 native slice consumes this normalizer; see
[engine-settings-state.md](engine-settings-state.md). It adds accepted initial
declarations, type/scope resolution, actual preparations/leases and common layer
publication. The argument normalizer itself still establishes only the structural
claims below. Mutable registries, real consumers and recipient projections remain
separate gates.

Specification §22.3.1.1 selects an engine-only normalized argument product for
F16 `semantic-settings`. It creates no settings store or application preference
service. Successful normalization returns owned data, not an accepted key/type,
permission, issued preparation, invocation admission or publication result.

## Product and boundaries

[semantic-settings.json](semantic-settings.json) records exact fields.
The envelope contains `route` and `request`. Typed requests use family index 15;
dynamic requests use `activate-context`. Both name variant `semantic-settings`
and version 1, with `arguments`. The engine ownership projection applies before
this product can match. Presentation verbs and the older combined product fail.

| Argument | Normalized meaning |
| --- | --- |
| contract and operation | `semantic-settings-activation/1` and `semantic-settings` |
| invocation and profile | Nonempty invocation label and exact captured execution-context profile |
| changes | Ordered bounded records naming layer identity, expected generation, registered-key candidate, operation and value |
| layer | Exact authority/id/incarnation tuple; authority must match the supplied context, not a self-declared scope kind |
| operation and value | Set carries a structured value; remove/reset carry explicit null, with distinct operation meaning |
| dependencies | Extra typed domain/identity/positive-generation predicates; required engine dependencies are added and checked later |

Duplicate layer/key changes, conflicting expectations for one layer and repeated
dependency keys reject. Order is preserved; normalization does not reorder or
merge an authored intent. Labels use scalar-valid text and UTF-8 byte caps.
Private reference versions are positive safe integers, not selected public Rust
or serialized version encodings. Context identity equality is not host/policy truth.

The seven explicit limits bound change/dependency counts, compact-JSON request
bytes, aggregate set-value bytes, identifier bytes and per-tree input nodes/depth.
Request and context trees are bounded separately. Empty changes/read sets need no
unused set-value capacity; remove/reset do not charge a supplied set value.
No numeric product defaults are selected. Finite native numbers are copied as
provided, including negative zero; no source number lexemes, decimal precision
or injective wire encoding is claimed. Registered setting schemas still determine
legal numeric ranges and value types.

## Direct precedent and separation

[RFC 8259 §§8.2 and 9](https://www.rfc-editor.org/rfc/rfc8259.html#section-9)
supports explicit input limits and identifies interoperability problems with
unpaired surrogates. This is not adoption of JSON as a required engine transport.
[Roslyn's workspace model](https://learn.microsoft.com/en-us/dotnet/csharp/roslyn-sdk/work-with-workspace)
separates immutable changed values from explicit application; it is not a proof
of Vellum authority or local atomicity.

```mermaid
flowchart LR
  A[Typed or dynamic arguments] --> B[Bounded owned normalization]
  B --> C[Actual engine key, type and scope resolution]
  C --> D[Current authority and required dependencies]
  D --> E[Owned staging and atomic publication]
```

Only B is supplied by this slice. An unknown key can be structurally normalized
without being registered, engine-owned or writable. Later resolution must reject
unknown/wrong-owner/ill-typed keys and current scope failures. UI state cannot be
admitted by calling it an engine setting. Existing line commands already carry
explicit semantic options; a general preference store is not required to decode
this product. The normalizer never changes source, history, configuration or UI.

## Conditional bounds and evidence

[semantic-settings.mjs](semantic-settings.mjs) walks a plain normalized data tree
with an explicit enter/exit stack. Each enqueued value consumes a finite node
reservation; active ancestors detect cycles, while repeated noncyclic values are
copied independently. Output containers freeze on exit. Compact JSON string-byte
charges handle escapes and scalar UTF-8 without allocating escaped strings.
The algorithm does not recursively copy, freeze or stringify nested containers.

[semantic-setting-checks.mjs](semantic-setting-checks.mjs) checks 24 typed/dynamic
products, ten independent JSON byte/node/depth oracle cases and 46 controls,
including a 10,000-level value, aliases/cycles, accessors, literal prototype keys,
nonfinite numbers, exact bounds, conflict identities, order and UI-field refusal.
The string-byte oracle additionally covers all single BMP code units: 63,488
scalar cases succeed and 2,048 lone surrogates refuse, plus four astral boundary/
representative cases (63,492 successful byte cases total). Each successful case
also tests its one-byte-too-small cap. This is byte-charge evidence, not a new
whole-Unicode segmentation or raw JSON parser conformance claim.
Eleven mutations must load and fail assertions after an unchanged module baseline.
The engine static-import boundary now includes this normalizer: 65 local modules
exclude the relocated frontend helpers. No new solver condition is claimed.

Native property enumeration and descriptor allocation occur before some size
refusals. Logical accepted-copy bounds are not pre-allocation, RSS or hard-time
guarantees. Proxy/native-code containment, hostile raw-byte decoding and duplicate
JSON-member handling are outside this normalized-data profile. Actual key/schema
producers, permissions, currentness, retention, publication and complete native
gateways remain integration obligations, not facts supplied by this decoder.

## Observation and security

INS-01 separates input normalization/refusal from admission; INS-09 distinguishes
later registry/type resolution and activation; INS-02 remains actual publication.
INS-08 accounts staged/retained values and INS-12 diagnostic loss. MELT/RED/golden
populations must not count a normalized request as admitted or applied. USE must
measure actual allocation/work separately from declared byte/node charges.
STRIDE/LINDDUN/STPA/FMEA cover owner-label substitution, stale scope versions,
unknown-key authority claims, deep/wide payloads, accidental callbacks and private
value disclosure. Raw keys, values, identity tuples and invocation labels are not
ordinary metric labels. Rejection timing/cardinality noninterference, real policy,
physical/concurrent/crash refinement and independent assurance remain unproved.
