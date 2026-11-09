# Owned annotation resource reads

Current registration integration: §22.3.3 adds atomic native resource metadata
activation and removal/reinstall generation fencing; see
[resource-registration.md](resource-registration.md). Current-row checks stale future
work without rewriting captured late evidence, bindings or producer obligations.
Public registration products, imported lifetimes and host/physical/independent gates
remain; earlier exclusions retain their original slice scope.

Current integration: §16.5.15 separately adds superseded-read discharge behind an
actual settled checkpoint and known-consumer checks, plus bidirectional charged
attachment detach; see [annotation-read-discharge.md](annotation-read-discharge.md).
Earlier exclusions below describe this document's original slice. Full lifecycle,
uncheckpointed/contested recovery and host/physical/production assurance remain open.

Specification §16.5.12 supplies the provider-read prerequisite for annotation
loading. An owned, bounded transcript composes existing ordinary binding-pair
checks with the actual version 1 snapshot decoder. It produces a syntax-valid
captured bundle, not native annotation state, a loaded baseline or savedness.

## Captures and current authority

Admission captures the full actual ordinary annotation binding/pair, source
revision, annotation set, read registration and requested read guarantee. Require
an explicitly active annotation role, complete current owner and write-registration
coverage, known-distinct source/annotation destinations and independent resource/
annotation read permission. No path, default sidecar, new document owner or source
role is inferred. Admission and submission enforce twelve exact-true facts;
candidate release enforces eight independent current facts and the whole original
target/binding/read-registration basis. Unrelated peer changes are not target
dependencies. The supplied current normalized root and policy/identity/coverage
truth remain trusted inputs; actual native-root acquisition is a later integration.

Read registrations describe read capabilities and use their own expected generation.
They do not pretend that a write registration declares atomic reads. The resolved
provider/resource/domain/key/locator must match the active annotation binding;
its write-registration generation remains separately checked by the existing
pair guard. This reference does not choose a public registration or request wire
schema. Internal contract discriminators prevent source-task or writer-ticket
substitution; they are not new public verbs.

## Ordered receipt state

The reference reuses resource-read.json's start/chunk/end/failure/quiescence
vocabulary, read guarantees, origins and bounded witness policy. An authority-bound
registry issues non-confusable task handles and permanently spends each admitted
origin within its finite issuance profile. Only submitted tasks accept exact
task/origin/provider/resource envelopes under current authenticity/evidence/ingress
facts. Shape, octets and scalar bounds are validated before duplicate/conflict
classification, so malformed input cannot fabricate a trusted contradiction.

Start captures an attributed resource version and coherence strength. Chunks must
have the exact next sequence and received-byte offset; short chunks are ordinary
progress, not EOF. End repeats start's exact version/strength and received counts.
No reordered delivery, missing bytes or mismatched EOF is repaired. Only actual
complete decoder success produces a candidate, preserving original JSON bytes,
including whitespace/escape spellings. BOM-prefixed JSON remains invalid under
the adopted snapshot profile; no source decoder silently strips it.

Exact retained duplicates are idempotent. Contradictory well-formed witnesses
contest the task and disable its candidate while preserving original rows and up
to two bounded terminal/conflict witnesses. Further distinct witnesses return
evidence-full. Transcript exhaustion or invalid/unsupported snapshots disables
candidate use, without publishing partial state or asserting engine failure.
These ordered, bounded states are not arbitrary-union lattice closure.

Captured-read attributes the received bundle, not a single physical resource
revision or current external truth. Atomic-resource-snapshot additionally requires
declared capability and current trusted read attestation; token equality is not
that attestation. Syntax validation does not prove the embedded document/source/
identity association. A syntactically valid foreign snapshot may remain a read
candidate, but §16.5.11 must still independently refuse invalid restoration.

Cancellation/supersession disables candidate use, not submitted effects. A ready
stopped task can retire without provider quiescence; a submitted task holds work
until independently authenticated attested quiescence. EOF/failure/cancel/wait
cutoff do not establish quiescence. It can precede buffered receipts without
claiming delivery drained. Late pure chunks may be discarded after stop; retained
contradictions and terminal evidence still have their own handling. Retirement
also requires independent retention release and delivery drain. It never recycles
issued origins or erases published annotations: this slice publishes none.

## Conditional preservation and termination

For finite natural profiles, owned registry handles, truthful normalized roots and
current host facts, and serialized reference transitions:

1. Admission establishes an owned immutable complete capture and reserves its
   logical size plus the maximum transcript, two witness envelopes and candidate.
   Submission/release require the same complete target and registration basis.
2. Induction over accepted ordered chunks gives received sequence/count equality
   and original concatenated bytes. Exact EOF and the actual bounded decoder bind
   a candidate to those bytes. Parsing supplies shape only, never edit authority.
3. Every successful conflict permanently disables candidate release; malformed
   envelopes cannot enter that transition. Stop likewise disables release without
   changing the quiescence or retained-byte obligation.
4. Work occupancy, retained task charge and lifetime issuance have separate
   transitions. Only checked retirement removes the task charge, after work and
   retained-consumer gates. It cannot release another task's charge.
5. Each call terminates over bounded arrays/maps, closed event cases and the
   bounded snapshot parser. The local FSM has terminal refusal states; arbitrary
   providers are not proved to return, and no fairness or wall-clock bound follows.

The reservation is a logical reference bound, not temporary allocation, parent/
engine-global/RSS or physical reclamation. Authored conditional arguments and finite
tests are not proof-assistant induction, implementation refinement or independent
Cleanroom certification. No new solver condition is claimed.

## Evidence and precedent

annotation-resource-read-checks.mjs covers all 4,096 admission and 4,096 submission
fact vectors, 256 release vectors, twenty controls and eighteen assertion-rejected
semantic mutants after a passing dynamically rewired baseline. Actual native
artifact bytes, ordinary pair activation and snapshot decoding compose in the
tests. Controls cover current full dependencies, distinct read/write generations,
ownership/copies, invalid data, short chunks, exact counts, unsupported guarantee
promotion, duplicate/conflict retention, cancellation/quiescence/drain, resource
dimensions, lifetime issuance and parsing without restoration authority. Import/
syntax/runtime errors are test failures, not killed semantic mutants.

The direct implementation precedent is the existing owned source-resource-read
contract, adapted without its source decoder or new-owner activation. The
[Node filesystem read contract](https://nodejs.org/api/fs.html#fsreadfd-buffer-offset-length-position-callback)
also distinguishes actual bytes read from requested length and qualifies EOF under
concurrent modification. It supports explicit counts, not a claim of atomic reads.
Vellum's identity, authority, transcript and retention laws remain its own contracts.

## Instrumentation and remaining integration

PATH-05/12 and INS-01/02/03/04/07/08/09/13 distinguish read admission/submission,
protected receipt ingress, bytes received, complete syntax candidate, association
refusal, native restoration, baseline adoption, contestation, stop, quiescence and
actual discharge. Candidate completion is not useful loaded-state publication.
MELT/RED/golden signals cover phase latency, traffic, errors/refusals, useful
completion and unresolved-work age. USE separately covers tasks, outstanding work,
issued origins, capture/transcript/witness/candidate/reply budgets and pinned
provenance. STRIDE challenges forged task/role/provider, stale registration,
guarantee promotion and exhaustion; LINDDUN covers content and owner/alias/activity
linkage. No bodies, raw identifiers, origins, handles, paths, tokens or offsets are
ordinary metric labels. Installed signals and timing noninterference remain gates.

Section 16.5.13 separately composes actual native-root acquisition, typed
restoration and producing-read pins; see annotation-native-load.md. This standalone
supplied-root helper does not itself prove that native composition. Section 16.5.14
separately supplies initial loaded-baseline adoption/comparison and qualified writer
replacement. Complete pin transfer/discharge remain open. Nonempty
replacement/missing-history profiles, all rebinding/history
routes, host/provider truth, package privacy, shared/global/physical accounting,
public invocation/events, production concurrency/crash and independent assurance
remain open. No storage convention, external transport or numeric default is chosen.
