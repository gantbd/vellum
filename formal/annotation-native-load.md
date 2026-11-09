# Owned reads and initial native annotation loading

Current integration: §11.2.18 composes owned initial restoration and separate
baseline adoption without nested root publication; see
[annotation-load-invocations.md](annotation-load-invocations.md). Actual read
authority and completed-read cancellation semantics remain distinct from writer
inspection and effect authority. Earlier counts/exclusions retain their slice scope.

Current integration: §16.5.16 separately retires original annotation reads and
ordinary acknowledged-write producers after actual native provenance release,
under actual work/known-consumer/retention checks; see
[disposed-annotation-producers.md](disposed-annotation-producers.md). Remaining
all-profile lifecycle/export/rebind/history and contested-recovery gates persist.
Earlier exclusions below retain their original slice scope.

Current integration: §16.5.15 separately adds superseded-read discharge behind an
actual settled checkpoint and known-consumer checks, plus bidirectional charged
attachment detach; see [annotation-read-discharge.md](annotation-read-discharge.md).
Earlier exclusions below describe this document's original slice. Full lifecycle,
uncheckpointed/contested recovery and host/physical/production assurance remain open.

Specification §16.5.13 connects the owned annotation-read transcript to initial
native restoration. The coordinator reads the actual native domain, publishes
only the private task's received bytes and retains its producing read afterward.
This initial slice does not adopt a loaded baseline, replace nonempty annotations
or discharge provenance pins. Section 16.5.14 separately adds initial loaded-baseline
adoption/comparison and qualified writer replacement; see annotation-loaded-baseline.md.

## Native authority and private ownership

One confined read coordinator can be created per healthy actual invocation domain,
under exact owner-control, resource and preparation facts. Its raw read-registry
owner never escapes. Caller-supplied roots and cloned/foreign coordinator/task
handles cannot enter the native path through the standalone read API. The
standalone normalized-root reference remains independently testable and does not
acquire native publication authority.

Admission, submission and candidate inspection obtain the current actual domain
root. Existing read and ordinary-pair checks bind the original source revision,
annotation set, full binding/pair and independent read registration. A native fault
fences new admission/submission/use, but does not prevent authenticated late
receipt intake or safe cancellation/retirement of an unconsumed read. Owner/domain
identity is local ownership evidence; host identity/policy/coverage truth remains
an explicit premise, not an identity credential inferred from a document.

Restoration accepts no caller replacement bytes or parsed snapshot. It retrieves
the actual owned current candidate, requires the exact captured snapshot profile,
and passes those received bytes to the existing boundary-owned restoration path.
The caller still supplies independently attested current identity/source/history
associations and grants. Parsing cannot manufacture them. The engine consumer
still receives typed records and has no snapshot parser/schema dependency.

## Publication and retained provenance

Before any native publication, construct and bound a private provenance record
containing the original complete candidate, context, association map, profile and
before-publication anchor. Admission has already charged one additional
maxCandidateBytes companion reservation per task, alongside its transcript,
witnesses and original candidate reservation. This is a composed logical charge
using an existing bound, not an uncharged post-publication allocation or a new
numeric default. Refusal leaves actual root, task issuance, charge and pins intact.

The existing native helper then enforces known-empty managed inventory/history,
exact current target dependencies, identities/targets/version floors, complete
root frames, attached baseline reservation floors and parser-source receipt carry.
Unrelated peer edits survive. A changed restore publishes once; an empty validated
restore publishes nothing, including at exhausted generations. Success records
the producing task once; a consumed task cannot restore again, even for an empty
snapshot. Historical scoped result inspection reports restored/unchanged separately
from uncontested/contested provenance and explicitly not-adopted baseline status.

Native installation and the final retained-record assignment form one serialized
reference transition with all expected allocation/size checks before installation.
This is not production concurrent/crash atomicity, allocator-failure refinement or
a promise that an external supervisor can reconstruct the transition after a crash.

Once consumed, a task is pinned regardless of later stop, native fault, content
revocation, quiescence or delivery drain. The retained original candidate and
association survive cancellation invalidating the ordinary candidate slot.
Authenticated late contradictions contest the producing read and change provenance
knowledge, never undo published annotations, erase later edits or fabricate a
baseline. Historical evidence inspection has its own current eight-fact release
gate; it is not a current live annotation read or external freshness assertion.

This slice has no pin-discharge facet. Pressure refuses more admission rather than
evicting required provenance. A complete later transfer/release must account for
loaded baselines, history, disposal/rebinding and all remaining consumers; a caller
boolean, timeout, cancellation or producer quiescence cannot substitute. Unconsumed
reads retain the existing safe retirement rules and never recycle issued origins.

## Conditional preservation arguments

Assume confined factories, immutable actual native roots, truthful current host
facts, finite natural profiles and serialized reference transitions:

1. The coordinator's private reader/domain association and actual-root lookup
   prevent a supplied fake root or foreign task from authorizing a native load.
2. The original owned receipt stream supplies the only restoration bytes. Exact
   current read dependencies and independent native association checks compose;
   neither a released candidate copy nor decoded identity text grants authority.
3. Every admitted task reserves its possible retained provenance before effects.
   The complete retained record fits that companion bound before publication.
   Success adds no unreserved charge, reissues no origin and spends no extra native
   generation beyond the existing restoration transition.
4. Existing native frame/floor laws preserve peers, source, history, bindings,
   baselines and other promises. Expected refusal publishes neither native state
   nor a provenance record. Empty success consumes only the read application.
5. A consumed task remains in both private reader and retained-provenance stores.
   Its retirement fence survives cancellation/quiescence/fault; late contradictions
   alter knowledge only. Retained publication facts are not rewritten as failure.
6. Each call traverses bounded records/maps and the existing bounded decoder/
   restoration path, without a provider callback or await in final installation.
   This proves no provider liveness, service-time bound or whole-engine termination.

These are authored conditional composition/trace arguments with executable finite
witnesses, not proof-assistant induction or independent Cleanroom certification.
No new SMT condition, global bounded-lattice closure or categorical-completeness
claim is introduced.

## Executable evidence and precedent

annotation-native-load-checks.mjs covers 42 actual native variant/capture/undo/redo
stages through owned receipt assembly and re-capture of the complete restored
snapshot. It checks all 65,536 restoration-gate vectors and 256 historical-evidence
release vectors, fourteen controls and thirteen assertion-rejected semantic mutants
after a passing rewired baseline. Controls include actual-root acquisition,
foreign/cloned handles, immutable copies, target versus peer edits, independent
association/profile checks, stop/conflict/fault behavior, pinned original evidence,
late knowledge without rollback, exact companion capacity, retained-record bounds,
empty exhausted-generation use and actual attached baseline promise preservation.
Read admission/submission/candidate grants are independently challenged as well.
Import/syntax/runtime failures are test failures, not counted as semantic kills.

The direct established reference is the existing source read/activation composition:
actual current owner lookup, complete captured input, final revalidation, producing
read retention and late contradiction without rollback. Annotation hydration keeps
the existing document owner rather than creating another owner, and does not
inherit source-loaded baseline semantics without a separate contract. The accepted
editor distinction between file reading, authored editing and persistence reporting
continues to apply; no speculative storage location or new external transport is
selected by this composition.

## Instrumentation and remaining gates

PATH-05/12 and INS-01/02/03/04/06/07/08/09/13 distinguish native owner admission,
provider submission/receipts, syntax completion, association validation, actual
restored/unchanged publication, producer pinning, historical result release, late
contestation and eventual independent discharge. These are separate success and
knowledge milestones. MELT/RED/golden signals cover phase/useful completion latency,
traffic, refusals, errors and unresolved age. USE separately covers task/work,
origin issuance, transcript/witness/candidate/companion capacity and pinned producer
occupancy. STRIDE challenges supplied-root/foreign-owner substitution, stale targets,
payload swapping, quota theft and false savedness; LINDDUN covers annotation,
provenance and cross-role activity linkage. Bodies, snapshots, paths, aliases,
identities, origins, tokens, handles and generations are not ordinary metric labels.

The separate §16.5.14 path supplies initial loaded-baseline adoption/comparison
and qualified writer replacement. Producer-pin transfer/discharge, nonempty
replacement/missing-history profiles, all disposal/rebind/history
routes, public invocation/outcome/events, actual provider/policy/identity/history
truth, package privacy, global/temporary/physical costs, installed telemetry,
concurrent/crash refinement and independent assurance remain gates. Source and
annotation files do not become one externally atomic write or one resource owner.
