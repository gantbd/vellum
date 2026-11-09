# Private annotation capture preparation

Current integration: §16.3.5 adds actual native explicit annotation-export
admission/release through the existing codec and destination owner; see
[native-annotation-export.md](native-annotation-export.md). Annotation disclosure
and source-write capabilities remain separate, and ordinary ownership/baseline/
binding intent is preserved. Earlier counts/exclusions retain their slice scope;
host, public, recovery, physical/concurrent/crash and independent gates remain.


Specification §16.5.8 supplies a shared-codec prerequisite for atomic ordinary-save
admission. The existing export path issues an artifact/ticket before a separate
baseline claim and hold step. Private preparation allows a later coordinator to
validate admission before issuing those products. This slice supplies the codec
primitive, not the coordinator or the complete admission transaction.

## Ownership and state transitions

One codec owner may hold one private preparation. The producer uses the actual
native artifact projection, including explicit point profiles and historical
source associations. Current capture/control grants must be exactly true. The
preparation privately retains the original immutable root identity and exact
payload. Its handle contains no artifact, bytes or executable write authority.

The bounded state machine is absent → prepared → committed or discarded. A
refusal leaves the current state unchanged. Committed/discarded handles are dead;
there is no revival, partial artifact or asynchronous external work. A refused
commit can be discarded or retried while the original root remains current, but
it never silently refreshes the payload after a root change.

Preparation charges the shared held-byte ledger for its full logical record,
including payload, result metadata, nominal owner/domain/root references and
counter envelope. It reserves one artifact slot and one future issuance unit
without incrementing issued or installing an artifact. The envelope must fit
maxArtifactBytes and remaining held capacity. Existing source/annotation captures
and source imports account for those reservations. A second private preparation
refuses; its construction cannot replace a live reservation.

Commit rechecks actual owner/handle/domain identity, current exact root identity,
native health, all fourteen current exact-true gates and historical-source
attestation when required. It installs the original private artifact, increments
issued once, replaces the preparation charge with the retained payload charge,
and consumes the preparation. Existing artifacts/imports remain untouched.
The reserved slot/unit allows other codec work only within remaining capacity.

Discard requires exact-true owner control, not ongoing content disclosure or a
healthy native domain. It removes unpublished preparation storage and its future
reservations without decrementing lifetime issued. This distinction does not
authorize refund of an issued artifact or an outstanding external write.
Commit/refusal/discard do not change the native root or publication counter.

## Conditional preservation arguments

For finite normalized inputs, immutable native roots, confined trusted codec
facets, truthful current grant/evidence facts and serialized transitions:

1. A preparation originates in the accepted lossless native capture and owns its
   private payload. Copied, foreign and type-confused handles cannot select it.
2. With A issued artifacts, I consumed lifetime units and P∈{0,1} preparations,
   competing captures require A+P < maxArtifacts and I+P < maxIssued; imports
   preserve the latter. Commit consumes P, adds one artifact and increments I,
   preserving both ceilings. No successful capture can steal its reserved unit.
3. If H is held bytes and C the staged record charge, prepare requires C ≤ L−H
   and changes H to H+C. Commit changes H to H+C−C+B where B is the smaller
   retained payload charge. Discard changes H+C back to H. Neither decrements I.
4. Exact-root identity and native-health/current-grant checks prevent a stale or
   faulted authority from committing even when captured bytes remain historical.
   Historical-source evidence is reattested, not inferred from current location.
5. Removing the owner slot and private handle record after commit/discard makes
   either terminal transition single-use. Refusal changes neither record nor
   held/reserved capacity. Issued artifact retirement remains a separate path.

These are authored conditional finite accounting/ownership arguments plus actual
reference-code checks, not proof-assistant induction, new SMT conditions, arbitrary
concurrency proof or independent Cleanroom certification.

## Executable evidence

annotation-preparation-checks.mjs covers fourteen native operation variants over
capture, undo and redo: 42 stages. Exact output bytes, shared charges, single
commit, unchanged native roots and normal artifact retirement are checked.
All 16,384 fourteen-gate combinations are checked independently at preparation
and commit, with exact refusal frames and positive final siblings.

Twelve controls cover unpublished discard versus lifetime issuance; one active
stage; foreign/cloned handles; stale roots; competing source/annotation artifact
slots; imports/captures competing for lifetime units; permitted unrelated capture;
exact envelope/held limits; strict current commit/discard grants; historical-source
reattestation; failed native preparation; and actual native fault followed by safe
discard. Fifteen assertion-detected mutants follow a passing dynamically rewired
codec baseline. They remove stage charges, replace versus double-charge accounting,
preserved issuance, commit issuance, root identity, single-stage control, discard
refund, competing issuance/slot reservations, current prepare/commit grants,
historical evidence, native-health fencing, envelope limits or exact payload.
Import, syntax and infrastructure failures do not kill semantic mutants.

## Integration and instrumentation gates

The prepared payload currently stays within the codec primitive. A later ordinary
save coordinator must compose writer-ticket, origin, destination, claim, native
baseline-hold and root admission before exposing any success; it must not mistake
this prepared handle for that complete reservation. Existing export semantics
remain intact. Atomic ordinary-save admission is still open.

One preparation per codec owner bounds this private retained record, not all engine
preparation work or physical memory. Construction peaks and retention of the shared
root referent remain parent/global accounting obligations; nominal reference sizes
are not allocator sizes. Numeric production limits, actual host/policy/alias truth,
package-private enforcement, public invocation/events, concurrency/crash durability,
hostile object decoding and independent assurance remain gates.

PATH-05/12 and INS-01/02/03/04/07/08/13 distinguish requested, prepared, reserved,
committed, refused and discarded work. RED/USE/golden questions cover preparation
latency/errors, occupied slot/issuance headroom, held bytes and abandoned private
preparation age. MELT at the codec boundary and STRIDE/LINDDUN cover stale-root
reuse, forged handles, stolen quota, content release and activity correlation.
These are observation obligations, not installed telemetry or numeric SLOs.
Raw payloads, root values, paths, contexts, identities and handles are not labels.
