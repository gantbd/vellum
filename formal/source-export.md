# Explicit source destinations and binding intents

Current integration: §11.2.9 adds native source binding-history release and actual
native source peer/queued-consumer retirement fences without refunding producers
or changing current bindings/baselines; see
[native-source-history.md](native-source-history.md). All source-read cleanup and
remaining persistence/host/public/physical/independent gates remain. Earlier
exclusions below retain their original slice scope.

Current integration: §16.3.3 adds native source export admission, reservation release
and qualified SaveAs activation with private codec preparation, actual root/pair/
writer/promise checks and atomic serialized local publication; see
[native-source-destinations.md](native-source-destinations.md). Remaining persistence/
registration/history/recovery/public/host/physical/independent gates remain.
Earlier exclusions below retain their original slice scope.

Current integration: §11.2.8 connects the existing pure source-rebind product to
the actual native root with final pair/writer checks, exact retained binding/baseline
history, source journals and promise floors; see
[native-source-rebind.md](native-source-rebind.md). Native export/SaveAs admission,
registration/history release and host/physical/public/independent gates remain.
Earlier exclusions below retain their original slice scope.

Specification §11.2.3 adds normalized F17/F21 subproducts to the existing owned
source-save coordinator. `source-export.json` declares the closed request and gate
inventory; `source-export-checks.mjs` exercises the actual read/codec/write/lifecycle
references. This is design/reference evidence, not production filesystem code,
whole-family refinement or independent Cleanroom certification.

## Precedent and chosen boundary

[Neovim's editing contract](https://neovim.io/doc/user/editing/) distinguishes an
explicit write to another file from SaveAs, which also changes the current filename.
Use that separation, not its entire overwrite/backup/dirty-flag policy. An explicit
destination must not implicitly replace an existing document's ownership or make
an unrelated current binding saved. Pure rebinding changes a local relationship;
it neither renames a disk file nor writes or reads destination contents.

[Rust OpenOptions::create_new](https://doc.rust-lang.org/std/fs/struct.OpenOptions.html#method.create_new)
provides precedent for exclusive creation, including failure on an existing dangling
symlink and avoiding a separate existence-check/create race. This motivates an
independent `exclusiveCreate` provider capability, not a selected production Rust
version, filesystem procedure or established provider guarantee. A prewrite probe
alone is not exclusive create, compare-and-write, durability or external freshness.

## Closed normalized products

`source-export/1` / `captured-source-export` requires exact current document/source/
serialization/source-binding and target registration, a fresh operation/effect/attempt,
purpose `copy-export` or `save-as`, independent protection/acknowledgment, and pinned
condition `{presence,resourceVersion}`. `absent` requires null version, explicit
create grant and exclusive-create capability. `present` requires the original exact
version and explicit overwrite grant. Conditional existing writes independently
require conditional-write support; synchronized acknowledgment independently requires
synchronization support. No force-overwrite mode, refreshed condition or fabricated
absence token exists. Scratch first-write uses the same explicit destination path;
it does not invent a loaded baseline.

Admission captures the actual immutable serialized source once, while reserving the
destination key in the actual root under that effect origin. Existing source and
all comments/history/views remain unchanged. A foreign/ambiguous/unknown-role owner
refuses. An unowned reserved key blocks another alias activation rather than being
reported as known absence. Multiple same-owner intents retain separate origins and
share the ordinary source writer's exact FIFO, outstanding limits, faults, receipts,
issued-origin set and immutable artifacts. There is no second writer or Busy-based
replacement/coalescing policy. Current coverage and supplied host identity truth
remain premises, not consequences of matching path strings.

Dispatch rechecks rights, full registration, original source binding, reservation,
captured target condition and safe preceding effects. Intervening source/settings
edits do not change the payload. Provider handoff excludes the original source
binding and baseline/private export state; the target sees its own required context.
All ordinary application/synchronization/disposition/quiescence/stop/contradiction
rules apply. Unknown or contested effects do not become known failures or safe
release. Cancellation can stop the binding intent while retaining a late write fact.

SaveAs activation is an explicit current-root transition after a qualified full write
and safe no-future-effect disposition. Revalidate current authority, original source
binding, target registration/owner, old and new destination barriers, and attested
postwrite presence/version/continuity. A null acknowledged version remains null;
it permits a byte baseline, not a fabricated next-save token. Atomically within the
normalized root, activate the captured target relationship and acknowledged artifact
baseline, release this intent's reservation and preserve current authored data and
newer edits. Repeat activation is unchanged only for the same current binding.
Copy-export never activates a relationship or publishes another binding's baseline.
A rejected activation leaves an inspectable written-but-not-bound fact; there is no
automatic external rollback or claim of atomic external write plus local activation.
Actual concurrent private-ticket/root installation remains a refinement obligation.

`source-rebind/1` / `activate-source-binding` requires exact current binding and target
registration plus current rebind/destination authority. Refuse unresolved effects,
pending export reservations, conflicting owners and exhausted generations. Same-context
success is unchanged after validation; otherwise advance only binding/publication,
set baseline to unestablished, and preserve document identity/source/settings/comments/
history/views. A later save needs a real baseline or another explicit destination
request. Rebind is not a hidden save, disk rename or content adoption.

## Reservation and provenance retention

Release removes only this owned origin after stopped predispatch intent or known
application plus safe disposition. Preserve other reservations and the current active
owner. Lifecycle key release and task retirement cannot orphan an export reservation.
Release of a key/reservation does not free outstanding work or artifacts.

An actual relation change appends the old binding/baseline to bounded private root
`bindingHistory`. Producing read and acknowledged-write evidence stays pinned there
as with live/retained-disposed provenance. Explicit internal history release requires
independent history/snapshot/read/write/provider discharge and current authority;
peer quiescence, delivery drain and other pins still govern actual retirement.
History/root capacity can reject activation, including after an external write:
retain written-not-bound evidence and allow explicit safe reservation cleanup.
No physical space, secure erasure, universal companion transfer or future capacity
guarantee is inferred. The normalized transition requires serialized installation.

## Conditional arguments and executable evidence

Assume a valid serialized root, authentic bounded envelopes, complete identity/owner/
writer/companion coverage, actual provider capability/continuity and correct supplied
authorization/discharge facts. Then:

1. Reservation checks admit no second live document owner for an occupied key;
   per-origin cleanup frames other intents and current ownership. Alias inspection
   cannot turn an occupied pending key into an unowned initial-load opportunity.
2. Shared private tasks, actual FIFO and safe-disposition barriers prevent this
   exporter from bypassing preceding unknown effects or creating a parallel writer.
   They do not serialize arbitrary external actors or prove provider behavior.
3. Immutable codec capture and exact target correlation attribute written bytes to
   the original source/settings and destination. Later edits and probes cannot
   substitute current bytes, another effect or an unrequested condition.
4. SaveAs requires both historical qualified evidence and current local intent/
   ownership/continuity. Writing alone never authorizes rebinding; stopped or stale
   intent never acts. Root refusal does not falsify or compensate an external fact.
5. Copy and pure rebind frame authored state and unrelated baseline meaning. Old
   evidence pins move to explicit bounded history, not disappearance at rebinding.
   Actual concurrent installation and premise truth are not proved by these laws.

Checks enumerate 131,072 admission, 131,072 dispatch and 16,384 activation gate
combinations; 21 controls include scratch absent create, overwrite authority,
alias FIFO and reservations, later source/settings edits, unknown/sync failure/
contradiction, cancel/late write, stale binding/disposal, target token/procedure,
copy cleanup, pure rebind, actual historical read/write retirement pins, normal
post-SaveAs save, and tokenless baseline refinement. Sixteen semantic mutants are
rejected by assertions, not import failures. These finite matrices/controls and
authored conditional arguments are not general proof-assistant induction, new SMT
theorems or a categorical completeness certificate. Earlier standalone lifecycle/
continuity `rebindOrSaveAsProved:false` flags keep their original scope; this new
ledger names only the explicit normalized products actually checked here.

## Instrumentation and remaining gates

INS-08/02 distinguish reservation, relation publication and historical-pin transfer;
INS-04/07 distinguish capture, dispatch, application, acknowledgment and safe effect
disposition; INS-03 protects status and provenance release; INS-06/13 retain history,
capacity, unknown effects and work/quiescence responsibilities. MELT, RED and golden
signals track each phase's rates/errors/latency and unknown/disposition age. USE
tracks queue/slot/artifact/reservation/history saturation in declared logical units.
STRIDE covers forged ownership/receipts, stale-binding or registration substitution,
ABA, unauthorized overwrite and reservation bypass. LINDDUN covers original-path
disclosure and historical cross-destination linkage. Use protected correlation and
bounded reasons, not raw paths, opaque tokens, document/provider identities or source
as metric labels. Actual clocks/collectors, privacy noninterference and measured
physical service/capacity remain assurance gates.

Open: actual host identity/exclusive-create/conditional-write/continuity/durability
and support matrix; annotation and combined persistence/point-policy storage; native
companions and retained consumer ownership; complete public invocation/replay/events;
retry/reconciliation; actual concurrent root/private-ticket install; hostile decoders,
physical accounting, production refinement and independent assurance. No new runtime,
network transport, public family/variant label, HTTP service or engine per file is
introduced. Existing F17 rebind and F21 source-write products carry these subproducts.
