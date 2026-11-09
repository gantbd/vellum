# Owned native annotation artifact retention

Specification §16.2.2 composes the actual §16.2.1 producer with the existing
source codec registry. This is a normalized reference composition, not a new
public wire protocol, production package selection or complete F21 persistence.
The native artifact's approved explicit position/content profiles are unchanged.

## Ownership, boundaries and finite transitions

`captureSerializedAnnotation` uses the actual registry's artifact map, held-byte
ledger, lifetime issuance count and limits. Source imports and source/annotation
artifacts therefore compete for the same held and issuance budgets; both artifact
kinds compete for the same artifact slots. No independent annotation pool bypasses
those ceilings. Reserve worst-artifact headroom before calling the actual native
capture. Check context, output, coordinate, retained-frame and capture-reply bounds
before installing an opaque owned handle. A refused capture installs nothing and
does not consume accepted-object issuance. This is not CPU-work reimbursement:
the parent's actual work reservation remains a separate premise.

The private row contains copied immutable basis/bytes, charge and delivery-stop
state, not an annotation root, historical authority or invocation pin. Its basis
includes the complete original normalized context, document incarnation, source
revision and annotation-set generation. Capture exposes the bounded copied basis
and opaque reference plus exact byte length, not JSON bytes. Basis disclosure itself still requires the
producer's capture grant. Reference possession and basis equality are not grants.

For each installed artifact the closed transition vocabulary is:

- active → active: authorized bounded copied-byte read;
- active → stopped; stopped → stopped: exact-basis protected owner control;
- active/stopped → absent: owner-authorized, retention-released and delivery-drained
  retirement;
- any other input → unavailable, with no installed-state change.

No resume, producer cancellation, source edit, provider write or receipt transition
exists here. Reads against stopped/absent/foreign/cloned references refuse. Source
read, source comparison and source retirement reject annotation rows; annotation
operations reject source rows. Neither artifact kind acquires the other's BOM,
baseline or destination semantics merely because both contain bytes.

Each release checks the complete captured basis and eleven exact-true current
facts in `annotation-retention.json`. It checks an ordered half-open byte range,
chunk bound and complete reply bound. Returned arrays are copies. A chunk can
split a UTF-8 scalar; concatenation reconstructs the original artifact exactly.
An empty EOF chunk is not full-transfer, provider-write or acknowledgment evidence.
Later edits neither mutate old bytes nor rebind their basis to the latest root.
Current permission may allow disclosure of that historical capture; a newer root
does not itself revoke or authorize it.

Stop is absorbing delivery control, independently available without a content-read
or ordinary resource grant. It does not recall delivered copies, refund storage,
retire a handle, stop another artifact or run/cancel the producer. Retirement
requires exact-true owner control, retention discharge and delivery drain. Remove
the row and debit its charge once; do not decrement lifetime issuance. Real parent
holds and provider quiescence must supply discharge truth in later composition;
these booleans do not prove that truth or authorize early release of an effect pin.

## Scoped arguments and executable evidence

Given a valid finite registry and truthful normalized host facts:

1. Successful installation adds one row and one issuance. Preflight headroom and
   the final charge check preserve slot, held-byte and issuance ceilings. A failed
   branch leaves those ledgers unchanged. Existing source paths are not redefined.
2. Read/stop never change held charges or issuance. Retirement subtracts exactly
   the removed row's charge; opaque map membership prevents a second subtraction.
   Lifetime issuance remains monotone, so retirement cannot reset the ceiling.
3. Copied bytes and exact original-basis comparison preserve captured meaning
   across reads and later native edits. Current grant checks remain independent.
4. Finite admitted native capture, finite byte copies and finite range/gate tests
   terminate at this reference boundary. There is no scheduling, host latency,
   physical peak-memory or production encoder-halting proof here.

`annotation-retention-checks.mjs` exercises the actual native producer and codec:
42 native capture/undo/redo cases, 2,048 release masks, fourteen context negatives,
six document/source/set-basis negatives, five chunk widths and thirteen controls.
Fourteen assertion-detected mutations are challenged after a passing dynamically
rewired baseline. Syntax/import failures are not counted as detections. The native
variants include both point profiles. Existing source-codec mutation coverage is
retained; its dynamic loader now resolves the composition's actual imports.

These are finite reference checks plus conditional construction arguments, not
universal implementation refinement or independently certified Cleanroom proofs.
The existing 223 solver conditions/thirteen manifests do not gain new theorems
from these tests. `annotation-retention.json` records positive and negative scope.

## Precedent, instrumentation and remaining work

[Rust shared ownership](https://doc.rust-lang.org/std/sync/struct.Arc.html) is
precedent for shared immutable data and lifetime ownership, not a selection of Arc
or evidence that a reference count alone establishes authority or discharge.
[Rust bounded byte reads](https://doc.rust-lang.org/std/io/trait.Read.html) provides
precedent for composable bounded transfer; it does not establish whole-transfer
completion, provider writes or a chosen runtime/syscall contract. This reference
generates a bounded complete JSON artifact before incremental byte delivery; it
does not claim a streaming encoder.

Reuse PATH-05, IF-01/13 and INS-01/03/07/08/12/13. Observe capture/refusal latency,
release successes/errors/bytes, held occupancy/slot saturation, lifetime issuance,
stop and separately discharged retirement with MELT, RED, USE and golden signals.
STRIDE challenges forged/type-confused references, scope drift, quota bypass and
double refunds; LINDDUN covers content and basis linkability. Do not use raw body,
identity, paths, offsets or basis values as ordinary telemetry labels. Actual
installed telemetry, its cost, availability and privacy enforcement remain gates.

Section 16.3.2 now joins explicit annotation export to the actual shared writer;
see [annotation-write.md](annotation-write.md). Ordinary annotation-save dispatch
and baseline integration must still join that destination writer,
with its baseline/condition policy, exact artifact attribution, current grants,
effect holds, unknown outcomes, receipts and retirement. No separate mutex lane,
annotation-write public protocol, native restoration or source-clean baseline is
invented here. Actual host alias/policy/discharge truth, all-engine admission,
physical accounting, production/concurrent installation and independent assurance
remain open. No whole D20/F21 or all-sections goal closes.
