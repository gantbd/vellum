# Native annotation field publication and invocation replay

Specification §13.6.1 composes F13 update-body/reattach requests with actual
source/annotation compound publication, authored-field history and the shared
invocation ledger. Document, explicit-profile point and nonempty range targets
use the existing native target algebra. Separate §13.9.1 compound membership
evidence is in [annotation-membership.md](annotation-membership.md); this field-only
checker does not prove it. Section 13.6.2 separately exercises native create/delete
on this same producer; see [annotation-membership-editing.md](annotation-membership-editing.md).
Actual identity issuance/reservation and decoded gateway,
point-profile persistence and complete F13 implementation remain open.

## Precedent and adopted contract

The [W3C Web Annotation Data Model](https://www.w3.org/TR/annotation-model/#web-annotation-principles)
separates bodies, targets and constrained source resources. That supports the
existing conceptual separation, not Vellum byte coordinates, current authority,
live version issuance or undo identity. No JSON-LD/RDF exchange, network dereference
or remote content execution follows from this native bridge.

[Neovim extmarks](https://neovim.io/doc/user/api/#extmarks) distinguish positional
undo restoration from extmark creation/deletion, which do not create buffer undo
states. Their undo_restore setting concerns positions around deleted/restored text.
It cannot establish authored-comment create/delete history or identity restoration.
Vellum continues to distinguish automatic placement from explicitly authored
body/target choices and retains the membership proof gap.

## Owned native preparation

Use the existing exact annotation-authoring/1 fields and update-body/reattach
variants. The normalized LiveSourceTarget binding is exactly document, sourceRevision
and target. Its target is the existing closed document/point/range sum. A point
retains explicit position/content mapping profile and before/after affinity; a
range is nonempty, half-open and scalar-boundary aligned. Current source, owner,
incarnation, bounds and accepted interpretation are validated without clamping,
focus lookup, quote matching or implicit profile substitution. This is not a
selected public token encoding, Rust layout or hostile-object decoder.

Require one actual live document and one matching annotation identity/incarnation,
finite current root/document/record/context/participant coverage, valid field
versions, current annotation-write authority and required companion/history/stop/
interpretation eligibility. Body size is UTF-8 bytes; empty body remains authored
data, not deletion. Preserve literal scalar text, including a leading BOM character.
Reject duplicate current record identities, even when their incarnations differ.

Preparation copies/freezes the original request, profile, recipient, finite limits
and mandatory dependency set. Retain the actual opaque owner binding separately.
Only the producer registry resolves the resulting owned capsule. Its bounded
complete descriptor and original capsule identity join the existing ledger's closed
annotation job case; small or equal-looking wrapper fields are not complete intent.
The ledger bounds the full envelope as well. An incoming decoded retry must still
resolve the old invocation before preparing another capsule; upstream canonical
routing and wire/native decoding are not proved here.

Mandatory and caller-supplied extra dependencies form the existing bounded current
conjunction. Equal duplicate conditions are idempotent; conflicting or stale current
conditions refuse. At execution the mandatory scope must still match the captured
set; it cannot silently expand or shrink behind an accepted preparation. Context
truth, actual grants and reservations remain host premises, not caller authority.
Numeric checker capacities are fixtures, not selected product defaults. Preparation
ownership/retention must remain bounded by the actual host; per-value logical checks
do not prove a global producer registry quota or physical allocation/copy peaks.

## Independent field effects and current publication

Body update checks the current content version and changes only the authored body
and required actual companions. It does not depend on source revision, old target
freshness, source read/write permission or an expected whole annotation-set value.
Later source mapping and compatible target edits are preserved from the current
root. Actual explicit/policy dependencies can still require source-related evidence.

Reattachment checks the current target generation, exact requested source and
current target eligibility, then lowers the original native target through the
existing before-target compound product. It preserves a compatible later body
update. Another document's edit does not stale that target; a changed required
source refuses instead of mapping the old request to a newer revision. An explicit
reattachment can repair detached placement even when the authored coordinates are
equal: complete semantic identity, not field equality alone, decides no-op.

Use the actual publishAnnotationPlan, not a local replacement record constructed
from old data. Source bytes/revisions, unrelated documents, current views and
external effects remain framed for these standalone field requests. Real authored
changes create the existing per-record linked history participant and advance
changed field/set/publication versions coherently. Complete validated no-op does
not allocate unused versions or history. Overflow or producer/outer-root pressure
refuses the whole candidate with the prior root unchanged.

The shared invocation ledger installs root and reserved terminal facts once.
Stop, result forgetting, retirement or a later field change cannot renew an old
no-op's eligibility. Its current outcome release and captured ceiling remain the
existing contract; private full roots/descriptors are never recipient replies.
Actual native F14 undo/redo consumes the resulting body/target journals, restores
authored values with fresh changed versions and does not rewind source or views.
General target restoration after source-context changes remains a separate gate;
this bridge does not turn targetRestorationValid into a general mapping proof.

## Conditional arguments and executable evidence

Assume normalized finite roots, actual owner/scope/retention/reservation truth and
serialized non-reentrant trusted publication. Original owned identity plus stable
bounded descriptor prevents a retry from selecting another producer candidate.
Final current field/version/dependency checks precede actual compound publication.
Body-only lowering has no source/target wish; reattachment has no body wish. The
existing compound merge therefore preserves current independent fields and source/
view frames. Complete candidate/root checks precede shared root/fact installation;
existing ledger induction supplies at most one eligible start and spent no-op.

Root/request/record/context/dependency bounds and two fixed native branches bound
the new traversals. Existing finite compound/history traversals supply the remaining
conditional per-run halting argument. No extension evaluator, callback or provider
work is launched in publication. These authored composition arguments are not a
proof-assistant refinement, open-load progress theorem, new SMT law, whole-engine
categorical completeness or bounded-lattice closure.

Checks cover 1,280 admission, 1,280 current publication and 640 release combinations;
five representative body/target forms each run all 720 six-action permutations:
3,600 traces/21,600 edges, plus 118 independent UTF-8 target-oracle cases.
Sixteen controls and seventeen assertion-rejected semantic
mutants cover actual resulting roots, copied versus original capsules, complete
intent/owner/recipient/profile binding, current mandatory/extra dependencies, scalar
and byte bounds, duplicate/disposed identities, producer/outer capacity, fresh
versions, empty bodies, explicit placement repair, source/field independence and
actual native history round trips. The original four editing, clipboard, compound
and base ledger suites remain separately revalidated. Finite traces are not
unbounded implementation exploration or independent Cleanroom certification.

## Instrumentation and remaining gates

PATH-01/03/04 and IF-01/03/08/12 distinguish native preparation, first binding versus
retry, current field/target/dependency validation, local disposition/history and
current outcome release. INS-01 measures bounded preparation/admission/replay;
INS-02 separates applied/unchanged/refused; INS-03 protects release; INS-06 records
actual linked history; INS-07 separates authored field/placement changes from save
milestones; INS-09 tracks interpretation/profile changes; INS-12/13 cover
fault capture and held preparations. MELT/RED/golden signals separate preparation,
service, useful local completion and observation latency. USE tracks declared root,
descriptor, dependency and history pressure. STRIDE covers forged identity/owner,
stale destructive targets, lost dependencies and false field-only no-ops; LINDDUN
covers private body/target existence, descriptor leakage and cross-context linkage.
Only bounded phase/budget categories are metric labels, never bodies, coordinates,
private IDs, paths, query/dependency text or per-attempt tokens.

Separate §§13.6.2/13.9.1 native/membership evidence and §11.2.4 lifecycle evidence
extend this field-only checker. Section 13.6.3 now checks actual native collection
events and independent writer outcomes in annotation-observation.md. Its original
false scope flags remain confined to its own tests, not absence of those slices.
Section 13.6.4 separately checks normalized F13 result receipts in annotation-results.md.
Actual identity issuance, public wire/alias result-reference integration, full retirement
and general target restoration remain open. Deletion, retained absence and final
retirement cannot be conflated to invent that proof.
Point representation is approved in §29.4.1; native persistence/restoration remains
separate. Decoded gateway,
other event/lifecycle/effect profiles, actual host/provider/physical/native/concurrent/
production and independent assurance remain gates. No operation family/variant,
transport, engine component, presentation requirement or deployment default is added.
