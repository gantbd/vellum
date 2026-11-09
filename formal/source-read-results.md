# Closed source read results

The selected F02 range/line paths preserve their existing native products while
checking closed result shapes before release. The checks add no storage, parsing,
presentation or transport mechanism. Acquisition and annotation result codecs
remain separate work; discovery /1–/5 is unchanged.

## Product fields and validation layers

`source-products.mjs` exports immutable declarations consumed by its result check.
Content has exact `status/ref/range/unit/source/context` fields. Line has exact
`status/context/ref/unit/line/body/delimiter` fields. Unavailable has only `status`.
References have `authority/document/incarnation/revision`; spans have
`lo/hi/source`. Units remain UTF-8 bytes, byte arrays are dense integer sequences
from zero through 255, range/line values agree with the original request, span byte
counts agree with extents, body and delimiter meet, and delimiters are empty, LF,
CR or CRLF. These are native value checks, not a hostile decoder or a wire schema.

| Boundary | Checks | Authority it does not establish |
| --- | --- | --- |
| Source-product construction | Closed fields and domains, original context and actual captured source reference | Current grants or independent correctness of the source/line algorithm |
| Coherent member reply | Original native member alias and reference derived from the retained member root, before publication | Authority from a reply's own revision or a private string selector |
| Northbound source facade | Shape, original request/document/context and data-free refusal | An independent captured-revision assertion; the fixed native owner supplies it |

The core producer rejects an unexpected internal result with the exact
`NATIVE_SOURCE_RESULT_INVARIANT` code. The coherent owner faults and withholds a
malformed rebound source reply. The decoded facade fences owner admission and
returns only unavailable for malformed native data or refusal products. Its
protected finalizer still refunds exactly its existing temporary workspace.
Selected pre-release faults preserve native registry/issuance and authored roots;
they do not establish post-release crash recovery or containment of arbitrary
native code.

## Resources and observation

The checks scan already admitted result bytes and inspect fixed metadata without
copying another payload or retaining another context. Existing native reply and
decoding reservations remain their owners' obligations. Constant-space byte scans
and successful logical headroom tests do not prove physical/global allocation or
measured latency limits.

Observe construction, coherent alias binding and northbound validation separately
along IF-01/02, PATH-02 and INS-01/03/05/08/12/13. Track finite result/refusal/fault
classes, validation duration and admitted reply-byte buckets; do not log source
content, references, paths or opaque handles as ordinary metric labels. A typed
internal invariant, normal unavailable and unavailable caused by owner fencing
must not be combined into one operational success population. Assurance-tool
outcomes remain separate from runtime health.

## Scoped evidence

- `node formal/source-read-result-mutations.mjs`: 24 current/retained native
  route/form cases, 63 shape refusals, six controls and 24 assertion-detected
  semantic mutations.
- `node formal/source-read-result-fault-mutations.mjs`: eight fixed probes, each
  across 24 cases, with fault counts `0/24/24/24/8/8/8/8`; three additional
  mutations challenge coherent native guards and the northbound guard.
- `node formal/source-read-result-substrate-checks.mjs`: 143 post-import callable
  exports intercepted, no selected baseline calls, and three injected call types
  detected across 24 cases each, with protected refund and unchanged native/root
  state.

Coherent private string selectors and public native aliases are deliberately
different roles. Cloning a string does not alter its identity; the probe instead
changes that private value and separately clones the actual native object alias.
Coherent final alias/revision probes also exercise the private owner directly,
so northbound rejection cannot mask removal of the lower-level check.

The existing independent source oracle still checks 1,869 line and 10,819 range
cases. Of its seven old mutations, five fail independent assertions and two are
now rejected earlier by the exact typed native invariant. The report records both
dispositions; arbitrary exceptions, syntax errors and tool faults do not count as
that invariant. The original native-source, coherent-query, argument, gateway and
engine/UI mutation banks pass. Four old gateway mutation construction targets and
two probe seams are updated for the local request/result binding without changing
their challenge.

These finite checks are not unbounded/categorical completeness or independent
assurance. Actual host/policy truth, complete public codecs/events/selectors,
other operation variants, physical/global/concurrent/crash and measured refinement
remain gates. Full frozen verification and endpoint reconciliation are required
before adoption; no whole completion row closes.
