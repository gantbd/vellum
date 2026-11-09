# Managed comment fields and mutation frames

Specification §13.5 makes the existing D19 comment record/mutation products
explicit: one document-associated identity/incarnation, plain valid UTF-8 body,
authored content version, explicit target/generation and separate mapping evidence.
This reference does not select D20 persistence, public field names, optional
metadata keys or a full attachment implementation. The subsequent §29.4 default
selects reference-host JSON/empty metadata; actual codecs, schema and host binding
are separate from these authored-field checks.

[W3C Web Annotation](https://www.w3.org/TR/annotation-model/) distinguishes bodies,
targets, selectors and source states. Its
[text-position guidance](https://www.w3.org/TR/annotation-model/#text-position-selector)
warns that position-only selectors are brittle under edits. Vellum therefore keeps
its already adopted incarnation/revision/expected-target and explicit reattachment
rules; it does not import character offsets as UTF-8 coordinates, JSON-LD as a
mandatory storage format, or the standard's broader multi-body/multimedia scope.
This is a mechanism comparison, not Web Annotation conformance evidence.

Content update captures the authored content version. Reattachment captures the
authored target generation plus relevant new source context. Each merges its
actual change with the immediately preceding other fields, not an old whole-record
copy. Deletion destroys both authored components and must validate both current
generations plus actual policy/history/disposal dependencies. Source-driven
attachment evidence and source revisions retain their separate meanings.
Empty bodies remain records; deleting a target cannot silently delete the body.

Under fixed independent permissions/constraints and no extra cross-field semantic
dependency, the pure authored-field functions commute: content update changes only
body/content version, reattachment changes only target/generation, and each reads
the same unaffected expected version. This is not a theorem that full annotation
commands, histories, policies or emitted events commute. Those may create real
ordering/dependency requirements and must use the shared publication contract.

`annotation-records.mjs` checks three mutation shapes over 16 supplied current
gate combinations (48 cases), preserving source and framed refusal/input state.
Eighteen controls cover empty create/update, body/target independence, stale
versions, whole-record deletion preconditions, unchanged fields, invalid scalar
input, retained deletion evidence, incarnation reuse and unknown mutation tags.
Useful creation/update/deletion outcomes prevent a reject-all fixture. Equal-body/
target controls compare the returned/input state, not only unchanged status;
a deliberately changed version must trip that frame check.

This reference has one logical record, body and target versions, a retained list
and a source field that must not change. Counters starting at one, strings and
fixture targets are not product encodings or capacity choices. Target validity/
current mapping, permissions, resource/profile checks and complete companions are
supplied, not decoded/proved. It does not implement automatic mapping, actual
source/annotation history, modified-state baselines, metadata schemas, physical
retention, serializer or recipient projection. In particular its equal-target
test covers no additional attachment/review/history intent; real no-op classification
must include all actual semantic effects under §6.4.

INS-01/02 distinguish input resolution/admission and actual authored publication;
INS-03 protects body/target/provenance/counts; INS-06 tracks actual history;
INS-07 tracks independent annotation persistence. MELT/RED/USE/golden signals use
bounded categories and actual clocks/coverage, not raw bodies, authors, annotation
IDs or quotes as labels. STRIDE/LINDDUN/STPA/FMEA cover stale deletion, target/body
overwrite, false attribution, hidden existence and lost retained obligations.
Actual field schemas, retention limits, persistence associations and both adapters'
accessible projections remain specification/integration gates.
