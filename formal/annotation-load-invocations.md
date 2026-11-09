# Owned annotation restoration and baseline adoption

Specification §11.2.18 connects the existing initial annotation restoration and
loaded-baseline adoption to the owned lifecycle invocation. These remain separate
local operations after an annotation read has completed. Neither operation issues
provider work, makes a disk read atomic with publication, replaces a nonempty
inventory, nor establishes current external freshness or write acknowledgment.

## Authority and the two phases

The native annotation-read owner lends one opaque, stable task authority under a
healthy actual domain and exact-true owner-control, preparation and resource facts.
The handle binds the actual manager, reader, task record, restoration/consumption
stores and domain. Copies, foreign owners/domains and incomplete candidates refuse.
Before restoration, the candidate must remain the same submitted, unstopped,
uncontested, unrefused task. After restoration, the retained consumed-read record
supplies provenance: cancellation does not erase a completed restoration. A late
contradiction still invalidates provenance. Actual task retirement revokes the
handle; existing native consumer pins determine whether retirement is permitted.

The private reference profile is `native-annotation-load/1`. It uses the existing
lifecycle wrapper and job kind, not a new family, public command or wire encoding.
The nested request is exactly one of:

- `{phase: 'restore', context, binding}` uses the existing host source/alias
  association evidence and exact native/snapshot/membership profile. Its writer
  inspection must be null. Reuse actual transcript/pair/candidate inspection,
  native restoration preparation and final target/history/proof validation.
- `{phase: 'adopt-baseline'}` uses the previously retained restoration and its
  exact profile. Supply an actual read-only shared-writer inspection and existing
  charged read-provenance attachment. The confined native baseline factory checks
  the actual writer ledger, destination conflicts, current ordinary pair/read
  registration, known prior baseline, original bytes and native capacity.

The writer inspection cannot dispatch work or acquire read-producer authority.
Restoration and adoption share the existing private candidate rules with their
direct native facades; neither calls a nested root publisher. Only source-save
constructs the loaded-read writer guard and only annotation-native-load seals the
loaded-baseline token. Existing confinement tests retain those boundaries.

## Publication and release

An immutable plan binds owner/domain/ticket/reference, authority, inspection,
request and profile. Current service checks exact interpretation/profile/facet
identity and stages against the actual current root. Restore maps restored to
applied and empty unchanged to unchanged. Adoption maps installed to applied and
current to unchanged. A bounded receipt contains only invocation, common status,
annotation-load operation, phase, native disposition and document identity.

Construct retained restoration evidence, candidate, receipt and native completion
before committing producer state. The common invocation owner checks actual root,
future-baseline promise and outcome bounds, freezes the candidate and carries its
actual source journal. Final stage validation checks exact plan/owner/domain/
ticket/before-root/result/receipt and current native evidence/guard. Commit the
restoration map plus consumption flag, or baseline-adoption record, then install
root/outcome/receipt/terminal phase through bounded trusted assignments in the
same synchronous turn. No await, provider call or user callback intervenes.

Queued invocation cancellation does not cancel, refund or retire the completed
read. Cancellation before restoration prevents restoration; cancellation after
restoration retains baseline eligibility under the existing provenance rules.
Conflicts, current pair/registration changes, active writer work and capacity or
promise failure refuse without partial publication. An empty restoration still
consumes and pins the producer. Invocation retirement forgets its plan, not its
read evidence. Replay does not repeat either operation.

Baseline adoption after authored edits preserves current source, annotations,
versions, history and parser-journal identities. It records the historical loaded
snapshot, not a claim that newer content is saved. Result release requires the
captured identifier ceiling, current invocation context, permitted summary and
existing strict lifecycle-result grants. A receipt is not read authority.

## Conditional arguments

Assume the existing trusted private module boundary, serialized non-reentrant
publication, finite validated profiles and sound supplied identity, policy,
coverage, retention and writer-uniqueness premises.

1. **Native reduction.** Restore uses the same candidate inspector, restoration
   preparer and final target proof as direct native restoration. Adoption uses
   the same pure baseline candidate and confined guard/token factory as direct
   adoption. Thus a supported owned success has the native root and disposition;
   comparisons exercise both phases and current-baseline repeats.
2. **Frame and producer consistency.** Before final commit, all candidate and
   receipt construction is private. Root, promise and outcome failures discard
   stages. Final exact identities and revalidation prevent stage substitution.
   Successful bounded producer assignments and terminal tuple installation form
   one serialized reference transition. A stopped/refused invocation leaves both
   producer and authoritative root unchanged. This is not a concurrent/crash
   implementation theorem or a transaction combining restoration with adoption.
3. **Lifetime and knowledge.** Task identity and revocation prevent resurrection.
   Native retained restoration and shared-writer attachment keep original read
   attribution; completed cancellation is not contradictory evidence. A late
   conflict changes provenance without rolling back authored state or fabricating
   write evidence. Native retirement/attachment charges remain independently owned.
4. **Finite closure and termination.** The wrapper and two phase products are
   closed; unknown fields/phases refuse. Every invocation moves issued → pending
   → done or faulted, with explicit retirement, no retry or backward transition.
   Each turn uses finite profile-bounded scans, validation and assignments. Phase
   rank and those finite bounds establish conditional turn termination, not
   production latency, fair scheduling or external task termination. No universal
   category, bounded-lattice completeness or hostile-decoder proof follows.

## Executable evidence and remaining gates

`annotation-load-invocation-checks.mjs` checks 42 annotation capture/undo/redo stages
and 252 typed/dynamic publications against direct native roots and dispositions.
It checks 256 admission vectors, 150 strict service refusals, 24 release masks and
twenty controls, including actual writer competition, delayed source edits/journal
identity, empty restoration, stop/conflict/retirement, staged substitution, result
capacity and an actual outstanding baseline promise. Thirty-two semantic mutants
are required in four exact eight-case banks, each after a passing rewired baseline.
The reference pass and every bank are mandatory, not optional coverage sampling.

Shared cancellation now exercises nineteen forms/ten native kinds, 304 gate cases,
152 context cases, 38 races, 4,921 traces/13,794 edges and the existing twelve
mutants. The direct native restoration and loaded-baseline mutation suites remain
mandatory, including their actual writer and factory-confinement controls.

Nonempty replacement/reload/merge, missing-history restoration, contested recovery,
all binding/history/legacy/host profiles, complete F17 events/registration/remote
products, actual host identity/global writer/coverage/retention truth, physical
accounting, concurrent/crash implementation, installed measurements, packaging,
frontends and independent assurance remain gates. No full family or goal closes.

## Instrumentation and threat coverage

Reuse PATH-01/05/10/12, IF-01/03/04/07/08/10 and
INS-01/03/04/05/07/08/09/12/13. Distinguish read completion, restoration, adoption,
current baseline and receipt release, not one combined save/load success. MELT
captures bounded phase/outcome and refusal reasons; RED/golden signals separate
admission, waiting, service and release; USE tracks invocation slots, read pins,
charged attachment/candidate capacity and shared-writer pressure. These are
instrumentation contracts, not installed or measured signals.

STRIDE checks copied/foreign authority, stale stage substitution, late contradiction,
unauthorized result release and capacity exhaustion. LINDDUN checks linkability
and disclosure across phase/read/document associations. Do not label metrics with
raw handles, document/resource identities, versions, source, comments or transcripts.
Trusted supervision and recipient observations retain separate disclosure rules.
