# Annotation authoring products and publication contract

Specification §§13.6–13.7 fix F13's four native request products and separate
persisted-data staging, current activation and effect evidence. The machine-readable
field catalog is annotation-operations.json. It does not add a family/variant,
wire command language or Rust ABI. Operation-families.json retains its 28 families
and 85 semantic variants; source/comments share the existing local coordinator.

## Precedents

[Neovim API metadata and buffer updates](https://neovim.io/doc/user/api/#api-metadata)
support typed discovery, explicit handles and revision-aware checks. This does not
import current-buffer shorthand, clamping, RPC or permissive executable extension.
[CodeMirror transactions](https://github.com/codemirror/state/blob/main/src/transaction.ts)
keep starting state, changes, selection and effects in one transaction; Vellum
retains explicit complete participants rather than importing arbitrary effects or
transaction callbacks into publication. CodeMirror transaction annotations are not
Vellum's authored comments. [Roslyn workspaces](https://learn.microsoft.com/en-us/dotnet/csharp/roslyn-sdk/work-with-workspace)
separate immutable candidate models from explicit application. None supplies proof
of Vellum authorization, history, multi-object atomicity or persistence guarantees.

## Conditional argument

The closed request products make required version fields unavoidable: body update
reads content version, reattachment reads target generation and exact new source,
and deletion reads both. The owner/document/incarnation checks prevent retargeting
by equal local IDs. Body and target updates merge their changes into the current
other fields; under the stated independent constraints they do not overwrite a
concurrent unrelated field. Real semantic/policy/history dependencies still apply.

Mandatory captured dependencies are combined with caller-declared additional
conditions, not replaced by them. Equal duplicate conditions add no new requirement;
conflicting expected or current generations refuse. Under a coherent actual
registry this checks the same required conjunction independently of vector order.
The finite lookup loops terminate on bounded vectors/registries; this authored
argument does not prove actual lookup ownership, time cost or domain truth.

If a complete validated change is applied, changed authored fields receive their
required new live versions; the annotation snapshot and publication generations
advance coherently. Deletion retains the removed record's prior version evidence.
The authored source projection is framed for standalone annotation authoring.
If the complete semantic effect is identity, no such counters advance and no unused
new counter is required. Current authority, bounds, stop and dependency checks still
apply. Failure before publication preserves the immediately preceding state, not
an older snapshot. Provider diagnostics cannot confer authority or rewrite facts.

These are known terminal decision facts, not the complete execution lifecycle;
pending or unavailable historical status is not evidence of not-published.
Persisted snapshots carry data. Only the supported owned activation plan, current
host identity/source/history evidence and shared final publication can change live
records. A stored incarnation or version is not proof of continuity. An absent
record is not deletion authority. Actual load/save integration must discharge these
obligations; no executable restoration planner is claimed by this slice.

## Executable evidence and limits

annotation-operations.mjs checks the exact four catalog variant labels against F13,
then explores 1,024 first-admission gate cases: four requests times 256 common gate
combinations. Each success is compared with its entire expected resulting state,
including the actual authored change, preserved fields, retained deletion evidence
and bookkeeping. Refusal and all input frames are checked. Eighty-six controls cover
missing/unknown products, cross-owner/incarnation errors, relevant stale versions,
invalid scalar bodies, actual body-byte/vector bounds, fresh identity and target
proof, retired identities, missing/conflicting dependencies, independent fields,
complete no-ops, counter availability, diagnostic nonauthority and file-as-command
rejection. A body update after a target change preserves the new target.

The reference composes annotation-records.mjs with normalized point targets and one
live logical record. It uses private safe integer counters, a two-dependency/64-byte
fixture profile and supplied host current/authority/allocation/companion facts.
These numbers are not product defaults. It has no general document/range target
decoder, hostile native-object boundary, actual registry/version allocator, complete
placement/history/retirement model, compound publication, restoration planner,
Rust/wire adapter or physical lifetime accounting. Its state/results are internal
fixtures, not released reply schemas. Equal-field companion-only transitions are
deliberately outside this fixture and cannot be mislabeled unchanged. Required v1
capabilities still need complete handlers, not a waiver via this limited reference.

Invocation replay precedes/reuses first admission under the separately scoped D03
contract; this evaluator is not its dedup store. No actual scheduler, queue,
authentication, source-byte target oracle, historical reply release or unbounded
implementation/refinement proof is added. These are authored arguments and
same-author checks, not new TLC/SMT theorems or independent certification.

Section 13.6.1 and annotation-editing.md separately add the owned native body/
reattachment bridge with document/explicit-profile point/range targets and actual
compound/history/shared invocation publication. The limitations above describe
this first-admission fixture, not an absence of that later scoped composition.
Create/delete identity/membership restoration, general mapping history, full public
gateway/events, load planning, point storage and actual host/implementation evidence
remain required; no complete F13 claim follows from either slice alone.

## Instrumentation and remaining obligations

INS-01 records descriptor/admission/version/dependency disposition with bounded
categories; INS-02 records actual coherent publication/no-op/refusal; INS-03 gates
reply/status/snapshot disclosure; INS-06/07 separate history and persistence;
INS-12 records capture loss/fault without changing authoritative outcomes.
MELT/golden/RED/USE reuse useful outcomes, queue/service phases and actual retained
pressure, not raw body/IDs/dependency tokens as metric labels. STRIDE/LINDDUN/
STPA/FMEA cover forged owner/version, destructive stale delete, implicit restoration,
hidden existence, duplicate-intent publication, counter confusion and lost obligations.
Actual event fields/reply projection, handler totality, all companions and host/
provider/resource conformance remain integration/specification gates.
