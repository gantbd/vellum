# Native semantic-root accounting v1

The user selected **Native**, rather than canonical reference JSON bytes. The
engine computes `native-semantic-root/1` from its actual current owned state.
`SourceCoordinator::semantic_root_usage` exposes a trusted-owner breakdown; it
does not accept host-provided sizes, retain a root or authorize a recipient.

Measurement, shared settings admission/publication checks and compound editing
publication checks are implemented. Annotation preparation also admits spent
identity slots against the configured root ceiling before issuance.
Source birth/initial resource load, logical container birth, membership/selection
replacement and provider/encoder/restorer registration now use that ceiling too.
Persistence baseline replacement, initial annotation binding and restoration now
complete the audited growth routes of the current trusted native owner. Configured
admission also preserves pending ordinary-baseline root-slot obligations. This is
a scoped implementation invariant, not a universal refinement/physical-memory
proof or completion of future facade/profile families. No default ceiling is
chosen. Existing source/configuration/history/retention limits remain distinct,
and scoped lifecycle lowering/service now uses the same native candidate guards.

## Cost rules and projection

Managed-search product guards use borrowed original metadata and constant auxiliary
state. Candidate validation walks only the existing bounded match/scope sequences;
release/navigation/occurrence validation compares existing retained identities and
products. No result-sized validation allocation, retained pool or additional
authored-root incidence is introduced. The shared first-cause health code is
bounded owner metadata, as before; the new injected fault field is test-only and
explicitly excluded by the exhaustive root-field census. Failed candidates release
only their unpublished original hold, not spent issuance. Late delivery faults
retain the published result and hold until protected retirement. Native matching,
copy/reply and hold reservations remain the existing separately funded obligations.

Units are native inline record sizes (`size_of`, including inline handles and
padding), plus logical sequence lengths and actual UTF-8/blob payload lengths.
Vector/string spare capacity, allocator metadata, Arc control blocks and process
heap peaks are not units. Units depend on the native build/ABI; this is not a
portable wire encoding, stable on-disk byte count or JSON equivalence. A ceiling
must be interpreted against the same native build/profile as its measurement.
The profile is provisional alongside the native facade, not a cross-build ABI.

Each owning semantic incidence is charged, even where immutable payload storage
is shared. Referenced provider/encoder/restorer registration bodies are counted
once in their owner registry, not again for every handle. Settings use the
compiler's existing checked logical-state measure, once for the current root.
All size additions and sequence-size multiplication are checked; overflow refuses
with a typed result rather than wrapping or saturating into a valid measurement.

| Component | Included current state |
| --- | --- |
| Header | Owner identity handle, publication generation, fault flag, document sequence header and optional current settings handle. |
| Documents | Issued slots (including empty tombstones), inline source/format/policy/lifecycle state, actual source UTF-8, source history frontier links and archived binding payloads. BOM intent is inline metadata, not three extra encoded-source bytes. |
| History | Current group/member/replacement/journal records, forward/inverse UTF-8 and authored annotation journal bodies. Source/annotation frontier payloads are charged with their owning records, not twice here. |
| Annotations | Issued slots, separately prepaid retirement slots and issuer-document links; retained present/absent records, actual bodies, restored identity keys and history frontier links. Retired slots keep identity/version floors and restored keys, not body/target payloads. |
| Views | Issued slots, logical attachment/stamp/selection headers and actual positional selections. No UI geometry, gestures or keymaps. |
| Workspaces | Issued slots, logical membership/generation records and actual membership handles. |
| Persistence | Provider/encoder/restorer registrations (including declared destination capabilities), current source/annotation bindings, active source-export reservation rows and their key incidences, identity keys, exact baseline bytes/validators/versions, baseline provenance and read/writer attachment metadata. Disposed bindings' inline headers and the source association-history vector header are in document slots; history row headers and all archived payloads are charged with their owner document. |
| Settings | Current compiled configuration's checked logical bytes, including its owned arenas, identities and generations. |

Component metadata headers include the stated subsystem's native profile/counter
fields where present. The explicit projections exclude co-located unrelated work;
they do not charge `size_of::<SourceCoordinator>()` wholesale. Exhaustive owner,
document and mixed-state inventories force reconsideration when fields are added.

Annotation final retirement moves the original identity/floors and optional restored
key into its prepaid slot, then drops the absent record's body/target/frontiers.
Both authoring issuance and initial restoration reserve/account the retirement
slot before installation. Its header is not allocated at cleanup, and the key is
moved, not copied or silently released. Root accounting checks parallel inventory
lengths and rejects live/retired overlap. Retired slots are never reused; current
record/body capacity is released without refunding lifetime issuance. Host-owned
copies of previously delivered values are outside this root and are not erased.

## Native current-root identity

Current-root identity is separate from this byte gauge. Discovery cuts now retain
a fixed native identity stamp and compare it before acquisition/query consumption:

| Identity input | Current mutation coverage |
| --- | --- |
| Original owner and checked publication generation/fault state | Source, policies, format, lifecycle, annotations/history, installed settings, view/workspace rows, bindings/baselines and export reservations. |
| Exact view/workspace profile values | Logical-view, line, Unicode and workspace profile installation, including same-size None/Some changes; no-op calls stay equal. |
| Append-only immutable registration positions plus exact persistence headers | Provider/encoder/restorer registration; continuity/save-operation configuration, read/save admission fences and native dispatch serial. |
| Validated read/writer attachment incarnation and private lifecycle generation | Present and absent attachment ABA, including equal-size cycles; a presence bit or cast address is insufficient. |

The three resource registries have no public replacement/removal transition and
retain their immutable issued prefixes. Prefix length is an exact position in that
non-reused owner history, not a general collection-count identity. Replacement or
removal would require revisiting this rule. Exhaustive owner and mixed-state
censuses force field classification; future mutation routes must preserve the
publication-or-header law explicitly. Current native transition coverage is an
implementation audit, not an unbounded proof about future code or memory faults.

Attachment admission checks space for two private lifecycle steps before installing
any endpoint credit. Successful attach advances once; the admitted detach owns the
remaining step exclusively. Detach advances even when returning to absence, so
None → Some → None cannot recreate an old root. No-op calls do not advance it.
Protected detach needs no global publication unit or new allocation, including at
global exhaustion/fault. An impossible active exhausted local generation faults
without erasing endpoints or credit. The local generation has its own explicit
u64 incidence in the native persistence header; it is not an authored revision or
new global engine counter.

The stamp copies fixed headers and original Arc identity markers, not root bodies,
schemas or callbacks. Its inline cost is in the actual Packet header; existing
remaining-packet admission and declared slot accounting include it. Retaining an
incarnation marker does not retain endpoint credit, root data or authority. Capture/
comparison creates no parent reservation, authored revision, new global counter or
provider I/O. It does not stamp excluded private work/admission/diagnostic state,
an external command registry, filesystem state, canonical JSON, allocator/stack/RSS
or physical/crash behavior. Those remain separate contracts.

## Separate budgets, not missing root payload

Save As activation moves an existing current binding into source association
history without dropping or duplicating its baseline/key charge. A changed context
adds the new current binding/header/key/baseline and removes only its original
reservation row/key. Same-context activation replaces baseline payload and removes
that reservation without adding history; scratch first-binding adds no archive.
Actual root/promise and explicit archive-row bounds precede all installation.
Historical anchors are actual producer consumers across live/disposed owners,
until independent provenance discharge. This archive is not user edit history and
is not automatically evicted. Explicit source association-history release now
removes the native rows and payload incidences only after independent current
discharge facts and actual row/producer checks. The vector header remains in its
document slot; no physical-capacity refund is claimed. Root and future-baseline
byte/publication floors still apply to a shrink. Writer/artifact charges are not
released by this control; other consumers, quiescence and delivery drain still
govern producer retirement. A full archive refuses further rebinding rather than
silently losing evidence.

Pure source rebinding moves the exact old binding/optional baseline into this same
archive and adds a new current binding/header/key with no baseline payload. It
does not subtract the old baseline's charge while that archive still owns it.
Scratch first-binding adds no archive; same-context requests add no root bytes or
publication unit. Optional baseline absence remains distinct from a zero-byte
baseline: native row headers include the Option representation, and only an actual
Some baseline contributes bytes/validator incidences. Exact native preflight tests
cover both scratch and previously bound roots below/at/above the required ceiling.

Queued source-condition pins do not add root rows or a new retention pool. The
actual existing task captures retain their original baseline references, and
retirement scans bounded writer inventory for exact origins and required written
records after older/equal anchors. Root-baseline replacement cannot refund those
producer/artifact obligations. Stop/dispatch can end a queue consumer, but producer
retirement still independently checks other consumers, safe evidence, quiescence
and delivery drain. Retained task/artifact/held-byte gauges continue to represent
the actual unreleased producers; this is not an extra physical-allocation charge.

Typed source versions add a separate optional epoch incidence to actual loaded,
written and archived baseline payloads. Native row sizes account for the new Option
header and provider capability fields; source-load preflight counts actual epoch
bytes before birth. Baseline replacement/enrichment counts both epoch and token
changes. Pinned capture/receipt retention remains outside the semantic root, under
existing task/live/lifetime and per-field byte bounds. The accepted causal edge,
fixed-role result witnesses and shared two-contradiction typed witness slots are
bounded task metadata, not an unbounded union store or a new worker/root pool.
Epoch and token fields independently use the explicit max_validator_bytes envelope;
that is not a complete wire-receipt/derivation or physical peak-memory budget.
Native own-chain metadata credits now separately preflight minimum and expanded
derivation/receipt projections. They are not the public wire-envelope budget.

The captured save after-result profile is fixed task/dispatch metadata, not a new
root payload or capability. Explicit typed-after absent-create tasks use the same
bounded result/epoch/token and shared two-conflict slots; no nonexistent before
version, causal derivation edge, separate evidence pool or worker is allocated.
Existing source preparation refusal releases its original charge before any
artifact/task issuance, including profile or independent capability mismatch.

Own-chain setup's fixed optional profile header belongs to the persistence root
projection. Its active per-task credits, immutable proof vectors and actual launched
conditions remain pending writer metadata, not authored root payload. Shared
continuity credit equals actual live ticket charges; drift faults without erasing
debt. Minimum funding precedes artifact/task issuance. Expanded proof/receipt growth
replaces only that ticket's original credit after successful final preflight;
stopped tickets keep credit until independent retirement. Source artifact payloads,
condition byte snapshots, accepted effect witnesses and their existing bounded
obligations are not silently transferred into this metadata quota. These explicit
native projections may overlap logical receipt contexts and are not RSS, a physical
preallocated region, wire bytes or a guarantee of allocator success.

Writer operation parents remain private metadata. SaveOperationLimitsV1's fixed optional configuration
header belongs to the root projection; its preallocated bounded private row
inventory, original context/intent/task links and spent operation labels do not.
Configuration requires a checked independent maxIssued × maxCaptureBytes plus
store-header reservation, including full row headers before inventory allocation.
Per-request capture includes original origin/context/target-key incidences and a
prepaid tombstone. The envelope remains after actual child retirement; zero codec
held bytes does not imply zero replay storage. Existing task/artifact/condition/
continuity budgets remain separately required. This is conservative native logical
accounting, not physical/global/wire-byte allocation proof.

The shared parent row now uses a closed inline source/annotation role. Its fixed
header (including the widest variant) must fit before preallocating lifetime row
inventory. Annotation capsules additionally charge the pinned before-resource
bytes/version and both annotation/source association key incidences, including
duplicate logical key incidences backed by shared Arcs. Actual native annotation
artifact/task and baseline-capacity promise funding stays separate; parent metadata
recovery cannot move or release a claim. Guarded child retirement replaces only
the co-owned parent payload with its spent origin; still-retained annotation
artifacts and their snapshots follow existing separate discharge.

Native save observation pools are deliberately outside the authored-root projection.
The actual shared retention parent separately charges the fixed lifetime context
inventory (state/pool headers plus maximum native row slots) and each whole child
maxHeldBytes ceiling before exposure. Checked declared native demand is
5R + 2C + 3X + (W+1)B + (I+4)N, subject to minimum native context/reply/reference/
wait/evidence frame sizes. The declaration and reserved ceiling are distinct
gauges. Context/signature/version metadata, ordinary and four protected reference
credits, and active/ended waiter slots remain bounded; no producer snapshot/body
or extra producer pin is acquired. Closing observation, scope loss or exhausted
ordinary issuance refunds nothing. Protected waiter/context/pool retirement checks
actual retained slots and independent control/retention/drain facts before returning
only its own reservation. This is native logical accounting, not wire/RSS/allocator
measurement, clock truth or a concurrent/crash theorem.

The same pool/context/wait accounting now covers actual owned annotation readers.
The closed target and receipt sums replace save-only slots; fixed native record
and reply projections therefore use the new widest variants. Admission reserves
at least the larger save-evidence/read-receipt fixed frame plus two validator
envelopes inside the complete wait/reply frame. Read projection retains only up
to two unique original end/failure witnesses and counts, not chunk payloads or
restoration snapshots. Version Arcs do not acquire producer/capture pins. Query
permission withholding, local receipt quarantine and observer stop return no
shared credit; all existing protected retirement/drain rules remain required.

Owned annotation-read parents are also private pending work, not authored-root
payload. Their actual shared-retention reservation is checked before exposure:
maxHeldReadBytes + 2 * maxIssuedReads * maxCaptureBytes + native Store/State headers.
Origin and complete parent capsules have separate bounded native projections;
fixed replies have a separate ceiling. Lifetime origin/parent vectors are prepaid.
Child retirement spends the origin but returns no sponsorship. Only irreversible
protected reader-store retirement, after admission fencing and actual reader/held-
byte drain, releases that reservation. Native reader work limits still apply;
this conservative logical sponsorship is not wire bytes, RSS, allocator or crash
qualification. Existing unowned reader work remains independently accounted.

Source-export destination reservations ARE semantic-root ownership, unlike private
codec preparations. Admission checks the row/header plus key incidence against
the actual root ceiling and baseline-promise floors before codec/task issuance.
The table is bounded by existing lifetime task issuance, with exactly one active
row per original export task. Release removes only that origin's row and advances
publication once; it does not refund producer/artifact/outstanding-work charges.
Task retirement and disposed-provenance release cannot orphan an active row.
Provider destination capabilities are inline registration metadata, accounted by
native registration sizes. No separate writer, unbounded origin table or physical
memory guarantee is introduced. Faulted-root ownership publication remains fenced.

The optional native resource metadata catalog is semantic-root payload, not a new
provider port or actual capability. The ResourceState header includes its optional
limits/root reference, floor and local fence. Active Root/row headers and all
identity/configuration/procedure string payloads are counted once in persistence
bytes. Replacement checks the actual root ceiling and existing promise floors.
Its original marker/profile/floor participate in current-root identity; bootstrap
stales older cuts without changing authored revisions, while exact idempotent
configuration and unchanged batches preserve the root marker and do not reset the
clock. Owned preparations are outside the CURRENT-root projection. Their explicit
native maxPlanBytes bounds the original request, native headers and two candidate/
result frames before allocation/reconstruction; request/result/label/row ceilings
are independent. This is per-attempt logical staging, not allocator capacity, RSS,
shared-parent sponsorship or a global cap on privileged caller-held preparations.
Registration jobs now retain only bounded original request/context/profile and a
result ceiling in the existing finite invocation inventory; the admission candidate
is discarded and reconstructed at service. Native intent bytes include complete
context labels, ticket/capture headers and copied request payload. Actual receipt
bytes must fit both native/facade limits and the invocation byte ceiling before
publication. The retained immutable value/seal share one result payload and bind
the original capture; delivery checks the complete copied receipt before allocation.
This adds no second task registry or caller-selected publisher. Direct caller-held
preparations are still not globally sponsored; catalog-backed consumer lifetimes,
public/physical and complete all-family accounting remain open.

Catalog-backed source-read targets retain an immutable full row, original native
registration/identity and catalog profile. Binding bounds the logical capture
header, full row payload and native key against its explicit per-call ceiling and
the native plan ceiling before copying. Host-held targets are not CURRENT-root
payload or globally sponsored caller storage. Admission adds this complete capture
to the existing reader task's sponsored held charge; owned operations also include
it in their independently bounded logical intent envelope (shared immutable Arc,
not a claim of duplicated physical allocation). Static validation recomputes this
charge without requiring the row still be current. Withdrawal cannot refund task
charges, release outstanding native capabilities or prevent original late evidence
and protected retirement. Other catalog-backed consumers remain separate work.

Annotation reads now share that immutable closed-kind capture. Their native held
charge includes the entire catalog capture, and static task validation recomputes
the full charge against the original association and native limits without looking
up a current row. Owned annotation operations also charge the full logical capture
inside their existing bounded intent envelope and preserve it across child copies/
replay. This adds no second producer registry or current-root payload. Original
external captures, dispatches and restoration provenance retain their existing pin
rules; catalog withdrawal neither discharges these nor refunds issuance.

Catalog-backed ordinary source/annotation writes retain one immutable complete
write-row/profile/native-identity capture in the existing writer task and artifact.
Its header, full row payload and native key are bounded at binding before copying.
The existing artifact-held byte ceiling charges that capture once alongside payload
and any loaded condition; all checks precede artifact/task issuance. The task and
artifact share the same capture and static validation checks this correlation
without requiring a current catalog row. Source task retirement releases its owned
artifact/capture through the existing path; withdrawal alone releases nothing.
Host-held binding handles remain per-call bounded, not globally sponsored. No
second task registry or CURRENT semantic-root payload is introduced. Owned catalog
source/annotation parents also retain that capture inside the existing finite
maxCaptureBytes envelope, checked before child/origin issuance. The intent and
artifact envelopes conservatively each count the logical capture, although their
immutable Arc is shared; this is not a physical-allocation total. Matching recovery
does not consume a second parent, artifact or baseline hold. Own-chain retains its
existing independent native credit law; declaration refusal precedes any credit
installation.
Source export/Save-As transfer adds catalog bytes to the already prepaid source
payload only after verifying the extra shared-artifact headroom. All fallible checks
precede preparation transfer/task/reservation installation; expected refusal
discards only the owned preparation and spends no writer/artifact issuance.
Catalog annotation exports create one separately bounded artifact envelope per
ticket, sharing immutable payload/snapshot/provenance with the reusable input.
Both payload incidences and all retained condition/catalog bytes are logical charges,
not a claim of duplicated physical allocation. Preflight covers artifact live and
lifetime slots as well as byte headroom, including pending source credits. Export
task retirement releases only its own envelope; the caller's original artifact
retains its separate lifetime. Neither path adds CURRENT-root capture bytes.

The document generated-newline birth default is a fixed-size closed enum in the
existing Document header, counted by the actual Option<Document> root projection
at birth. No extra registry, heap payload, mutation generation or retained pool is
introduced. Host/catalog policy selection occurs before the ordinary birth/root
preflight; an alias join preserves the existing header unchanged. Exact current
default queries copy only that enum. Default-newline preparation retains the
resolved explicit delimiter inside the existing bounded line-edit intent, not a
second root or text batch; its actual source/view/companion/history costs remain
governed by the existing transaction machinery.

Passive native registration normalization shares the scoped closed configuration
parser. Its independent inert capture bounds and joint peak envelope cover retained
inert nodes plus the complete typed change headers and provider/resource/configuration
payloads before output allocation. The original invocation role is borrowed, not
cloned as authority. These are per-call native logical bounds, not CURRENT-root
payload, allocator/RSS or global caller-ingress sponsorship. Decoded submission
rechecks the native profile and uses the existing owned registration job's separate
bounded capture and receipt envelope; normalization reserves no job or root state.

Passive native/scoped serialization normalization bounds inert profile/format/
generation capture and the joint retained-inert plus normalized-output envelope
before allocating the copied execution-profile string. Original invocation,
document and dependency roles are immutable borrows; their referent storage,
caller ingress, stack and allocator/RSS are not included in this per-call logical
bound. Dependency cardinality is checked at normalization; full request/plan
accounting remains the existing typed handler's responsibility. No root, job or
global caller-memory reservation is minted by normalization.

Scoped settings and scoped lifecycle normalization budget iterative value-conversion
workspace with the retained inert and normalized output envelopes before typed
allocation. Its conservative output envelope counts value-root header overlap;
the conversion workspace reserves an Option<SettingValueV1> header per captured
node. Borrowed invocation/dependency/basis referents, caller ingress, stack and
allocator/RSS are excluded explicitly. Native set-value node/depth and job/plan
budgets retain their existing meanings; envelope fields do not spend set-value
nodes, including empty/remove/reset requests. No root or job is reserved here.
Lifecycle's output envelope additionally includes every creation/contribution,
removal identity, target and reference header/payload. Its lifecycle basis is
separately supplied, not copied into normalized data. Native created-value and
candidate/root/plan admission remain independent existing handler checks.

Scoped definition normalization preflights type traversal workspace before even
allocating that workspace: per captured node, two usize traversal/order slots plus
Option<SettingTypeV1> and Option<SettingValueV1> conversion slots. The joint bound
also includes retained inert capture and the complete conservative normalized
declaration output, including schema/type/value-root header overlap, restriction
choices and every target/identity payload. These are transient logical bounds,
not allocator/RSS, caller ingress, borrowed roles or native candidate/plan charges.
The existing compiler remains responsible for native schema/value/compatibility
and retained-state limits. No root, basis or job is reserved by normalization.

Fact-bearing native ordinary settings uses the existing invocation ledger, with
no new registry or conditional-basis issuance. Its complete logical capture counts
owner/context headers and UTF-8 labels, native request/value capture and all caller,
required and combined read envelopes. Complete intent plus native preparation and
wrapper headers must fit both parent intent and native plan ceilings before binding.
The retained capture charges its weak preparation-identity header; the pending
wrapper charges strong pointer headers and the native preparation envelope. The
weak identity keeps no old settings snapshot alive after service/stop. Physical
Arc/Weak allocation/control-block memory and allocator/RSS are not logical root or
global sponsorship claims. Unexpected extra preparation ownership faults rather
than publishing. Native envelope normalization separately bounds inert/output/
conversion workspace, excluding original invocation/dependency referent storage.
Native values and definitions now share a closed carrier with boxed payloads.
Logical native capture includes the complete enum header plus the value capture
or declaration capture; the enum header is subtracted before copying that payload.
Preparation accounting likewise includes the boxed payload's complete native
envelope and its enum/wrapper headers, never subtracting an inline payload that
is actually boxed. Definition input includes its explicit epoch triple and the
shared declaration envelope. This changes no CURRENT-root projection and promises
no physical allocation/control-block/RSS bound. The same immutable preparation
identity and weak post-settlement lifetime apply to both participants.
The same carrier now includes native lifecycle. Its condition pair and complete
layer capture use the same enum/payload/plan accounting and grant-comparison work
ceiling. Native/scoped lifecycle share contribution copying; native targets reuse
the closed definition target codec. Normalization charges every creation/removal/
target header and payload plus joint inert/value conversion workspace. Optional
created-reference holds retain their independent existing shared-parent reservation;
they cannot be added to an already admitted native job, and failed funding leaves
the ticket issued. Result staging/publication and protected discharge reuse existing
holds rather than another result table or a second identity allocator.

Conditional registration bases are separate retained authority, excluded explicitly
from CURRENT semantic-root bytes and root identity. The issuer reserves its entire
maxHeldBytes from the existing shared parent before installation. Its checked
declared envelope is maxProfileBytes + maxBases × (2 × maxBasisBytes + maxUseBytes +
2 × maxReplyBytes). Native profile/header, scope, typed slot, full captured row,
host-copy and reply bounds are checked before issuance; native headers and UTF-8
payload bytes are logical units, not JSON/allocator/RSS sizes. maxRootBytes checks
the actual current native semantic-root projection without retaining a root copy.
Active handles use original opaque markers and the coordinator namespace; equal
active recapture spends no issuance, removal spends no refund. No lifetime-sized
tombstone vector is needed: the monotonic issued count and permanent Closed issuer
state prevent budget reset. Retirement discharges only this pool's reservation,
never producer work or another pool's debt. External handle aliases and privileged
host copies are not globally sponsored physical memory.

Scoped registration invocations reuse the existing finite job/capture inventory.
Both the original scoped ingress envelope and complete retained native/context/
scope/reference/index envelope must fit maxIntentBytes before copies. Lowering
first borrows current private basis slots to validate the complete permutation,
duplicate typed slots, table/configuration shape and native request payload cost;
only then does it allocate a normalized request. Original actions/configurations
are shared with that request, not duplicated into another mutable request tree.
Issuer/handle aliases retain no whole-root snapshot or extra job registry. Service
temporary lowering is individually bounded by the existing intent ceiling; this
does not claim a global peak/RSS temporary budget. The sealed native outcome stays
trusted-owner evidence; the independent scoped copied receipt is sized as its
native header plus fixed index/state rows under captured/current result limits.
No native stamp/identity/configuration payload enters this scoped receipt. Forget/
retire drops job references, not the shared authority pool or its issuance count.

Passive scoped-registration normalization is a per-call caller-owned native
envelope, not CURRENT-root payload or an issuer/retention reservation. Inert request
capture bounds bytes/nodes/depth. A schema-only borrowed pass sizes the complete
typed header/change rows/configuration string payload before copying that output.
The independent normalization maximum bounds the larger of inert traversal peak
and simultaneous retained inert plus typed output. Capture and output limits do
not stand in for the existing maxIntentBytes checks at actual scoped admission.
Original invocation references are borrowed, not serialized/copied capability
payloads; basis handles remain a separate native array validated by the common
admission path. This profile excludes caller ingress, program stack, allocator/RSS
and globally sponsored caller-held normalized values. No second job/authority pool
or filesystem/transport allocation mechanism is introduced.

Pending source codec preparation is not semantic-root payload. Its one fixed inline
owner slot is separate from the root projection, as are existing artifacts/tasks.
The resource profile reserves one live/issued artifact credit and payload bytes in
the existing shared held-artifact budget before encoding. Other admissions preserve
that credit and the prepaid vector slot. Retention moves the charge without adding
it twice; discard checks actual aggregate occupancy and refunds only the original
preparation. `prepared_source_bytes` overlaps `held_artifact_bytes`, not a second
total. No persistent artifact issuance is spent until retention. Native inline-slot
and payload bounds do not establish allocator overhead, transient Arc conversion
peaks, global parent funding or RSS. No source, binding or publication root changes.

Source saved-state comparison checks a per-call encoded-output cap and a checked
current-plus-baseline cap before temporary encoding. The caller-owned explicit
budget, or compatibility limits derived from the configured resource blob ceiling,
does not become semantic-root state. Temporary output drops at return without
artifact/task issuance or a retained reservation. These bounds are not global or
shared-parent temporary funding and do not count allocator overhead or peak RSS.

The caller-owned diagnostic recorder is not a semantic-root field or a coordinator
retention reservation. Its opt-in measurement profile bounds fixed native state
(four series, four generation references and four counters) plus one four-point
snapshot frame before allocation. Event queue capacity remains a separate existing
reservation. Disabled/default recorders allocate no measurement state. Recording
and successful drain only update fixed bounded counters; no record-time growth,
subscriber, clock or I/O occurs. Returned snapshot copies and caller-created value
series remain caller-owned, not globally funded registry entries. Native logical
units exclude Arc control blocks/allocator metadata/RSS. The public value helpers
do not establish general measurement-series/provenance/recipient retention budgets.

Optional native service timing adds an inline recorder profile and an ephemeral,
private `Instant` episode, not a coordinator field, pool, retained source or new
retention reservation. Observations already contain optional durations; queue
storage is unchanged. Off, disabled and initially full capture skip clock reads;
record/drain remain clock-free and existing loss accounting is unchanged. These
logical bounds do not establish clock overhead, stack/RSS or physical-memory
guarantees. Timing neither grants authority nor discharges an obligation.

The actual source/annotation transaction publisher's `publication_measurements`
field is separately parent-funded, not caller-owned or part of the semantic root.
Its fixed native pool reservation includes six immutable series, six packed
bounded atomic counters and one complete six-point snapshot workspace. The probe
only clones the private count-owner Arc; no source/body/journal/root is copied or
pinned. Default-off publication allocates nothing, and active probes allocate
nothing at publication time. Exclusive coordinator borrowing serializes private
probe updates and snapshots; atomic storage preserves native Send/Sync without a
mutex or retry loop. Profile retries cannot reset counters/generations/ordinals.
Stop keeps the reservation and yields off points. Protected stopped-pool retirement
checks actual probe holds, delivery drain and retention release, discharges the
original reservation once and leaves a closed nonrearming state. Actual parent
corruption still fences the owner; ordinary metric profile/snapshot refusal does
not. Costs are logical native units, not allocator/RSS, caller-retained reports or
a complete measurement/provenance/exporter budget.

The optional native read-release collector is also separately parent-funded, with
six inline series/counters, protected reservation metadata and one complete six-point
snapshot frame. It retains no source or context. The exhaustive owner inventory
explicitly excludes it from semantic-root payload, and activation leaves semantic
root usage unchanged. Recording at the shared gate uses no allocation, clock,
callback or retained probe; exclusive synchronous borrowing serializes updates,
snapshots, stop and retirement. Default-off and retired paths allocate nothing.
Profile activation needs actual additional parent headroom; exact/minus-one profile
and parent tests enforce it. Absorbing stop retains funding, protected retirement
refunds once, and neither step discharges native read/context/workspace obligations.
Returned points retain caller-owned generation references rather than pool ownership.
Arc control blocks, allocator/RSS and caller-retained reports are not these units.

Protected native observation-stop composition owns no new retention charge or
ordinary read workspace. The four closed first-party roles borrow original typed
handles/control scopes under exclusive Rust ownership and invoke the existing
bounded native metadata methods. Roster lengths and scope comparisons are bounded
by original captured owner profiles; malformed lengths cannot create a new scan
population. Stop does not refund contexts, holds or lifetime issuance, and does not
clear unrelated temporary debt. Protected retirement remains a separate operation.
Ordinary parent exhaustion and an editing fault do not become stop-admission gates.
This is not a budget/proof for a hostile decoder, external protected ingress or
panic/abort/physical memory. No new pool or current-root inventory field is added.

The passive native stop envelope adds no copying/frame reservation: every ordinary
accepted value is a closed constant, validated through exactly three fixed-width
record shapes (2/4/3 fields). It rejects unknown nested data without traversal and
canonicalizes directly to an original borrowed typed target. No caller strings or
new capability wrappers are retained. This intrinsic native grammar bound does not
fund caller-owned packet memory, wire parsing or external protected ingress; those
remain separate. It must not be generalized to free-form fields without revisiting
their capture/control funding and lifetime contracts.

Passive native source argument binding retains at most one normalized output frame
alongside the source adapter's existing staging. Its whole-envelope, seven-field
actor and canonical shadows are independently bounded and released on normalization
return; fixed templates have an explicit bound. Before native facts staging the
source adapter now explicitly discharges its already-validated actor/query/scope
shadows. This preserves the existing classification-plus-seven-effective-frame
reservation, including full coherent-peer facts, without nested admission or a new
pool. Effective bytes remain min(caller, settings-parent ceiling). Document and
source/combined/member capabilities remain original borrowed references, not inert
copies or serialized identities. Logical bounds do not qualify physical resources.

Passive native annotation arguments use that same effective-byte ceiling and
classification/seven-frame guard. Envelope/actor/canonical shadows are independently
bounded and released on normalization return; fixed schema templates and normalized
owned output have explicit ceilings. Original snapshot/document/record/continuation
references stay borrowed outside the inert graph. Native lowering is a separately
size-checked fixed enum, cloning only original opaque identity wrappers. The
annotation adapter explicitly drops validated actor/query/scope shadows before
native facts staging; normalized output and lowering fit within the existing seven
frames. There is no second admission, selector pool or authority inferred from
normalization. Existing selector publication, nonrefunding issuance, native pool
funding, protected refund and retirement are unchanged; late refusal is not rollback.

Passive native acquisition arguments share the existing classification/eight-frame
guard, with min(caller, settings-parent ceiling) bytes. Native participant rows and
class enums are typed; original documents/retained references remain borrowed.
Fixed schema templates, combined dynamic participant templates and owned normalized
actor/profile/participant rows each have explicit native ceilings; three inert
captures are independently bounded. Normalization temporaries drop on return.
The acquisition adapter now drops its validated actor shadow before facts staging,
making room for the normalized frame without another reservation or pool. Port,
class, profile, ordered-document and original-reference checks precede native loss
and cut consumption. Installed contexts/prepared transfers keep their original
funding and nonrefunding issuance, including late facade faults; not rollback.

The fixed native search adapter shares that same read busy fence and an eight-frame
family. Classification and 8*min(caller bytes, settings-parent ceiling) use actual
parent reservations, not a settings intent allocation or fake gauge. Native actor,
port, scope, command query, independently captured current query/flags and complete
peer facts each have explicit native byte/node/depth/template/owned-copy bounds.
Query/current validation shadows discharge before later staging. A fixed original-
product binding fits a separately checked native ceiling; it copies no match vector
and adds no retained pool. Existing managed-search pools fund native results/holds
and release products. Protected refund releases only the guard's own obligations,
then owner health gates delivery; late failure does not erase results or refill
issuance. Private first-party facts restoration is bounded by the existing copied-
facts row/member-frame allowance, including the selected non-first coherent scope.
These logical native bounds do not measure allocator/RSS, caller-retained replies,
public decoder/identity work or physical/concurrent/crash behavior.

Native and passive-decoded read adapters now share a private execution/cleanup
gate and explicit native error delivery. The change adds no retention claim:
classification, seven/eight-frame workspace, native context/result storage and
nonrefunding issuance keep their existing owners and ceilings. Optional-result
compatibility wrappers invoke that same gate once, not nested admission. A late
fault still withholds delivery without refunding native installed obligations.

Passive decoded history arguments use the same history read guard and seven-frame
workspace ceiling; they do not call native facade admission a second time. The
normalizer bounds the complete envelope shadow, eight-field actor and canonical
record independently, including original three-field limits and a null opaque
placeholder. All original handles remain borrowed outside the inert graph. Fixed
schema templates have an explicit native-unit bound; owned normalized query/profile/
actor/limit output has its own one-frame ceiling. Normalization shadows drop before
native query staging, leaving at most that one normalized frame alongside the
shared adapter's existing bounded copies/workspace. No new retained context or
alias pool is created. Native result storage retains existing history pool funding.
Refusal/fault uses the same protected temporary refund and final-health gate;
late native loss is not rolled back. These are native logical envelopes, not JSON,
caller input memory, reflection/preallocation, RSS or hostile-parser guarantees.

`invocation_measurements` is another optional non-semantic-root pool funded by its
own actual parent reservation. It covers seven native series/counters, a complete
admissions/dispositions snapshot and one declared native bool unit per lifetime
invocation capacity. The private slot cohort flag is inline native ledger metadata,
not a source/root hold or a metric dimension; its lifetime-capacity surcharge is
checked before collector admission, including late activation. Captured intent,
pending preparation, created-result holds and source journals retain their original
independent budgets. The collector neither copies nor pins them. First-bind and
known terminal installers do constant bounded updates; the fault snapshot scans
only actual bounded native slots and does not allocate a retained ticket set.
Snapshot preflights all seven ordinals before constructing the owned report.
Saturation/collection stop/exhaustion does not alter execution. Protected observer
retirement discharges only its own reservation and leaves a closed nonrearming
state after clearing its private cohort marks, not a claim that retained invocations
are retired. Caller-retained snapshots,
full measurement/provenance/recipient ownership and physical costs remain separate.

Annotation/combined preparation is separately parent-funded retained work, not a
current semantic-root payload. Its fixed native envelope funds lifetime cut headers,
retained permitted facets and independent request/copy/scope workspace even at zero
prepared capacity. Annotation-only cuts contain no source root or source-product
profile. Combined cuts retain only one admitted source projection with its profile.
Current-root measurement bounds admission but is not copied into a cut. Protected
cut discard does not refund the fixed reservation or spent issuance; empty-pool
retirement releases its own reservation once. Installed annotation/combined ownership
has a distinct fixed actual-parent reservation for slots, retained projections/scope
and independent request/copy/reply workspace. Cut transfer moves values, not a root
copy or another pool's charge. Preparation retirement refunds only preparation;
installed stop/row retirement refunds no fixed funding/issuance, and empty installed
pool retirement refunds its own charge once. The installed profile now additionally
funds F03 record/continuation descriptors and page-staging workspace, with explicit
row/page/selector caps. Context and F03 descriptor slots share spent lifetime
issuance; equivalent retries create no duplicate obligation. Complete native enum
reply headers, vector row headers and admitted body payloads bound disclosure before
registration. Protected context retirement removes descriptors without refunding
their spent IDs or the fixed reservation. Combined search obligations remain future
work, not funded claims inferred from successful annotation/source reads. All of
this is retained work, not semantic-root
growth or a physical/global allocation proof.

Pending/prepared commands, invocation/result/replay slots, held source/annotation
captures, search results, Unicode boundary tables, interpretation/basis pools and
history pin controls remain separately bounded. Pending read/save transcripts and
held artifacts are likewise outside this root; once bytes are elected as a current
persistence baseline, that baseline is inside it. Host-retained returned snapshots
do not alter the current root's measure. Host admission/profile limits and these
retention pools are not editable semantic configuration in this projection.

The traversal allocates no intermediate root or JSON, invokes no callbacks/I/O
and changes no publication generation or authority. It is linear in current
bounded semantic records; string/blob lengths do not scan their contents.
`SemanticRootAccounting` emits only bounded completed/refused diagnostics without
content, identities, byte counts or clock calls. Recorder saturation/disablement
cannot change the measurement. Neither a successful measure nor a diagnostic
establishes owner health, admission, persistence, physical memory or crash safety.

## Implemented settings admission

`configure_semantic_root_budget(max_bytes, recorder)` installs one immutable,
embedding-selected ceiling after checking that the current actual root fits.
Underflow/overflow and an undersized ceiling refuse before installation. Repeating
the same ceiling is an unchanged configuration; a different ceiling is unavailable.
Configuration publishes no semantic generation and provides no recipient authority.
`semantic_root_byte_ceiling` is a trusted-owner query, not an admission certificate.

Ordinary value, value-with-reads, declaration and authored-layer preparation now
require this explicit ceiling. An absent budget is unavailable, not unlimited.
The compiler computes the known candidate's logical settings bytes during its
existing semantic preflight. The owner replaces only the current settings
contribution in the measured whole root, using checked arithmetic, and refuses
before returning/retaining a prepared plan if it does not fit. It does not charge
the old root retained by that plan to the current semantic root. Plan, wrapper,
parent-intent, configuration-state and separate retention budgets still apply.

The common publisher recomputes the actual settings candidate against the current
root and remeasures unrelated components before installation. It never trusts the
preparation-time byte count for final service. Even no-op proposals require a
configured ceiling and a fitting current root; exact-boundary no-ops consume no
unused publication counter. Retained capacity refusal is a terminal historical
outcome; replay cannot rearm it. Scoped value and declaration preflight use the
same checks before observing absorbing conditional-basis loss.

Initial trusted settings installation may bootstrap before the budget is selected;
budget configuration subsequently measures that actual initial state. If the
ceiling was already installed, initial settings installation checks its candidate
before publication. No later settings preparation silently bootstraps a ceiling.
Existing fixtures explicitly select their test ceilings; those are not production
defaults. `SemanticRootBudgetConfiguration` and `SemanticRootAdmission` diagnostics
are bounded, optional and content-free, with no byte values or clock callbacks.

## Implemented compound publication and annotation issuance

When the ceiling is configured, the common compound publisher measures one
complete staged root before any source, annotation, view, history or generation
installation. It overlays actual source candidates, retained present/absent
annotation values, mapped selection sets, journals and replacement frontier links.
Unchanged settings, workspaces, persistence and other documents remain in the
whole-root measure. Current and staged measurements share payload-cost rules;
no cloned owner, fabricated host size or trial installation/rollback is used.

The check uses final component contributions rather than adding only positive
growth. Thus selection normalization can reclaim records to fund source growth
in the same atomic candidate. Undo/redo retains absent annotation bodies where
the engine actually retains them and accounts transferred frontier links. A new
history branch counts actual replaced links, not a monotonic estimate. Service
remeasures later unrelated documents and their retained history; it does not
borrow preparation-time headroom or refresh the original mutation conditions.

Capacity refusal leaves all publication participants and generations unchanged.
Retained invocation refusal remains terminal and retry cannot allocate again.
At the exact ceiling, an ordinary compound no-op needs no unused publication
counter. Checked candidate totals do not trust a cached total and cannot wrap.
The existing bounded optional `SemanticRootAdmission` diagnostic observes the
actual check; disabled/saturated capture cannot change the result.

Annotation creation spends owner-local identity slots during preparation, before
authored publication. Each admitted slot adds an actual `Option<Record>` and its
issuer-document handle. The engine checks that cost before spending the slot.
Bodies retained only in prepared work are not yet current-root annotation bodies;
the final publisher checks the actual authored candidate. Previously admitted
slots remain spent if later preparation or publication refuses; no lifetime refund
or rollback of issued identities is implied. Refusal tests distinguish this spent
issuer state from an impermissible partial authored publication.

## Implemented birth, logical state and registration admission

When the ceiling is configured, source birth checks the decoded UTF-8 payload plus
the new nonrefundable document slot before issuance or publication. Initial strict
decoding remains observable even if later capacity or publication-counter checks
refuse; decoding/admission completion is not a publication receipt. Refusal spends
no document incarnation, so a smaller fitting candidate can still be admitted.

Initial resource loading adds its actual binding record, identity key, exact
baseline bytes and validator to the same birth candidate. This prevents a source
owner being installed before discovering that its baseline/binding does not fit.
The binding vector is reserved before birth; its final insertion adds no fallible
step after source publication. An existing alias joins its current owner unchanged
without another birth/baseline charge or fresh decode.

Logical view/workspace birth checks its actual issued-slot cost at preparation,
staging and installation, including retained invocation service after unrelated
growth. Membership replacement measures old versus proposed document-handle
payload. Selection attachment, detachment and the shared direct/retained selection
publisher measure old versus normalized proposed selection payload. Replacement
uses checked subtraction/addition, not an unverified net-growth estimate. No frame,
selection, membership or generation prefix is installed on refusal. Existing
Unicode/endpoint/occurrence consumers retain their common selection publisher.

Removing membership/selection payload reclaims its actual logical bytes. Retiring
an empty view/workspace preserves its spent slot charge; retirement cannot refund
issuer high-water. Relation retirement itself is non-growing: record headers remain
charged, and only membership/selection payload can disappear. Retained capacity
refusal is terminal even when later release creates new headroom.

Provider, annotation-encoder and annotation-restorer registration check the actual
new registry handle plus native registration body before minting/pushing a new
registration. Dropping a returned host port does not erase its owner-held registry
record or refund root capacity. Profile/authority/context eligibility checks remain
independent. These are trusted engine seams; no filesystem/network/UI mechanism is
introduced. All added checks use optional bounded content-free admission capture,
with no clocks, identifiers, payloads, byte counts or diagnostic authority.

Settings preparation still requires an explicitly configured ceiling; the other
trusted primitive paths preserve their existing bootstrap behavior when this
profile is absent. No numeric production default or unlimited sentinel is added.

## Persistence and held root-slot capacity

Source baseline replacement, annotation binding activation, written-baseline
publication, loaded-baseline adoption and sealed initial annotation restoration
check their actual candidate before installation. Imported records charge spent
slots, issuer-document links, bodies and imported identity keys. A refused transient
restoration package can release its own witness capture on drop, but spends no
record identity and publishes no authored/read-adoption prefix. Explicit exports
remain unreserved: later baseline adoption may legitimately refuse capacity.

Ordinary source and annotation saves promise future baseline installation. Their
independent retained-byte and publication-unit reservations now also preserve a
logical root-slot floor under specification §§16.3.4/16.5.4–16.5.5. Let `C` be a candidate's
current logical-root size and `b(d,k)` its current baseline payload size for source
or annotations. For each document/artifact kind with pending holds, use the maximum
of `b(d,k)` and all corresponding captured future payload bounds, then add only
the positive difference to `C`. These differences sum across documents and
artifact kinds. Each hold still pays its own complete retained descriptor/
bookkeeping charge and publication unit, including multiple holds for
one document. Native inline optional-slot headers are already paid at binding
activation; they are not counted twice as future payload.

Future payload bounds use exact captured artifact and association-key lengths plus
the original provider's bounded maximum validator bytes.
Source bounds include the captured encoded payload, maximum token bytes and,
for the epoch/token profile, maximum epoch bytes as a separate field. Source and
annotation claims share the configured future-baseline byte ceiling; neither can
spend the other's reserved publication unit or root-slot demand. Fixed descriptor/counter
widths are native units, not JSON estimates. Admission and floor validation share
the sizing law and check the complete captured retention charge. Ordinary save
admission validates prospective demand before issuing artifact/task identities.
Configuring a ceiling after holds already exist must preserve those holds too.

All growing candidate guards, including settings preflight, preserve the current
floor. Baseline installation substitutes its actual new slot contribution and
consumes only its own pending credit, preserving all others. Removing or disposing
a held binding/document remains blocked by the existing current obligation guard.
Cancellation/stop of a dispatched unknown effect does not release headroom;
protected discharge requires actual disposition/quiescence evidence. Capacity is
not write authority, acknowledgment, current freshness or permission to refresh an
original write condition. Corrupt private promises fault admission rather than
being treated as ordinary low capacity; mutable admissions latch that fault and
protected stop remains available.

`semantic_root_capacity_usage` reports trusted-owner current-root bytes, pending
baseline growth, aggregate admission demand and optional configured ceiling.
The original root gauge still measures current payload only. Neither retains a
root nor grants recipient release. Bounded `SemanticRootCapacityInspection` and
admission diagnostics carry only disposition, not byte values or identifiers.

## Current route audit and remaining scope

The configured invariant starts when actual current root plus outstanding demand
fits the explicit ceiling. The audited current mutation routes preserve it:

| State | Growth gate / non-growing case |
| --- | --- |
| Source/document slots and initial source bindings | Shared birth candidate; text changes use compound publication. Disposal removes payload; policies/serialization change fixed-width fields. |
| Annotations and identity slots | Standard preparation admits spent slots; compound publication admits authored values; restoration admits complete imported records/keys. |
| Views and workspaces | Birth validation/service/installation; actual selection/membership replacement; compound view mapping. Retirement/disposal removes payload while preserving slot charges. |
| History | Complete compound journal/frontier candidate; retirement removes groups/links; pins are separately bounded controls. |
| Persistence | Registration admission, birth/binding activation and actual source/written/loaded baseline replacement. Archival moves already-charged payload into existing document slots while removing active binding headers. |
| Settings | Initial installation, shared value/declaration/layer candidate preflight and final recomputation, including pending slot floors. |
| Pending ordinary promises | Prospective admission, per-document maximum root-slot demand, per-hold retention/counter demand, own-credit consumption and protected discharge. |

This audit covers actual production mutations, not test-only corruption fixtures.
Fixed-width metadata updates do not grow native bytes; existing publication-counter
guards still preserve pending units. Legitimate lifecycle/binding release cannot
invalidate an outstanding promise because its actual obligation checks refuse it.
Other retained captures, results and held work remain in their separate budgets.
The source-context field is likewise explicitly excluded from semantic-root
measurement. Its fixed pool reserves native context/hold tombstones, capture
envelopes and independent copy/query/reply workspace against actual parent
retention before any context can issue. Current-source acquisition checks the
actual semantic-root gauge against its own root ceiling, but retains only the
admitted source projection. Retained-source acquisition copies only an already
admitted available projection,
not a newer whole root. Each copy pays ordinary context/issuance limits in the
same reserved pool and never pins its original context implicitly.
The shared source/combined/version-set managed search pool is separately parent-funded,
not part of semantic-root size or silently absorbed into source-context funding.
Its fixed native reservation includes lifetime result slots, bounded retained
request/scope/matches and independent evaluation/copy/reply workspace, even with
zero result capacity. Each published result owns an actual original-context hold;
this spends the source pool's own shared hold issuance but copies no source root.
Result retirement discharges that hold, not fixed pool funding or either issuer.
Its own retained domain labels are charged explicitly. Bootstrap configuration
copies the actual source or installed combined domain; subsequent source-pool
retirement does not close unrelated combined results. Empty search-pool retirement
refunds its reservation once and preserves its own spent issuance metadata.
Combined holds use a separate preallocated actual ledger in the installed owner;
its fixed funding covers lifetime hold slots before configuration succeeds.
Contexts, F03 selectors and F05 holds share a nonrefilling issuance ceiling, while
live holds have an independent cap. A result retains its original issuer and hold;
protected retirement cannot discharge a peer issuer or refund lifetime issuance.
Retained installed-context acquisition uses the same fixed copy/reply workspace
and ordinary context/selector issuance, without a hold on the original or a new
pool reservation. An explicit admitted-class tag and narrowed native byte census
exclude unrequested raw source/profile or annotation rows. Complete copied value,
scope and reply precede nonallocating installation; refusing or faulting staging
installs no context and refunds no spent identity. These are native logical units,
not allocator/RSS or independent global bounds.
Coherent version-set preparation has its own parent-funded retained pool, explicitly
excluded in the exhaustive current-root inventory. It funds measurement workspace
against the actual immutable parent semantic-root ceiling, not merely a smaller
local input threshold. Lifetime cut slots, complete ordered member/projection
frames, aggregate capture/scope and independent request/copy staging are reserved
before configuration. Native member accounting conservatively includes inline
member and projection headers; it is not serialized JSON or measured heap. Private
staging registers/spends one cut only after every member fits. Discard preserves
fixed funding and spent issuance; empty-pool retirement releases only its own
reservation once. Installed parent/member ownership uses a distinct reservation,
not this preparation charge. Its fixed native units cover lifetime parent/member
tombstones, complete alias staging and request/copy/scope/reply workspace before
configuration succeeds. Each installation spends one parent plus every member
alias together; value/scope frames move only after complete reply/alias staging
and current checks. The exhaustive root inventory excludes this retained pool
from authored-root size. Preparation-pool retirement leaves installed funding/
values intact; group retirement removes aliases without refunding lifetime issuance
or fixed funding. Empty installed-pool retirement refunds its own reservation once.
Member source range/line replies now use this installed workspace with independent
complete inner source and outer parent/member request/reply ceilings. Context facts
count the actual installed selector header rather than assuming a prepared-ID width.
These value-only reads mint no alias/hold, spend no issuance and add no retained
engine charge; already delivered copies do not become engine-held leases or a
global recipient-memory guarantee. F03 queries now fund lifetime descriptor slots
and complete `(page rows + continuation)` staging before installed configuration.
Their actual native record/reply/selector types determine copy/row/reply limits.
Contexts, member aliases and descriptors share spent issuance/live-selector caps;
reusing an equivalent descriptor spends nothing. Complete candidate keys and reply
ownership precede nonallocating registration, so refusal installs no prefix.
Parent retirement removes query lookups but not spent IDs or fixed funding.
Whole-parent search holds now have preallocated lifetime slots in the installed
reservation, an independent live-hold cap and shared nonrefilling issuance. Each
result retains its actual original member/parent ledger entry, not another source
copy. Actual counts fence held-parent retirement. Complete protected parent controls
release one published result hold; private unpublished cleanup spends issuance
without publishing a result or refunding fixed funding. The shared search owner
funds its original issuer/hold header. Full fresh peer frames have complete native
request bounds independent of the inner selected-source envelope. Invocation
captures explicitly retain bounded peer scopes for attribution after retirement;
the full request/capture envelope charges those copies without a new read lease.
Retained version-set copies use that same installed reservation and shared admission/
alias/reply staging, not another pool or a hold on the original. Before copying,
complete original plus requested-subset input, narrowed native values/scopes and
new context/alias issuance fit their own bounds. Fresh actual native reply/registry
frames are recomputed from the requested member count and next aliases, not assumed
equal to old frames. Excluded members/classes retain no private payload inventory.
Refusal or staging fault leaves every original, charge and spent counter unchanged;
successful copies own their values/aliases independently and share no stop/hold state.
Native source result validation adds constant auxiliary metadata only: the expected
captured reference is a stack/Arc header copy, not another source payload or context.
Shape checks inspect already admitted reply fields/lengths and at most the delimiter
bytes; they neither allocate a payload nor extend a lease. Existing source/reply
profiles fund construction. The bounded sticky invariant cause is explicitly
classified as owner-health control metadata in the exhaustive inventory, not
authored semantic-root bytes or capture-buffer authority. Selected fault injection
changes no registry/issuance/root/funding; protected cleanup keeps existing rules.
Native F03 result validation similarly borrows already staged reply/retained rows
and selector candidates, without copying bodies, retaining a context or growing
another registry. Duplicate-page checking uses constant auxiliary state and a
bounded quadratic scan; the original page/reply limits remain authoritative.
Annotation invariant faults use the same fixed first-cause owner-health slot.
Prepublication refusal leaves actual selector issuance and funding unchanged;
this is not the separate postpublication facade fault frame.
Native acquisition validation adds only fixed stack/borrowed metadata, not another
source/body copy, retained participant set or lease. It compares existing candidate
and final reply fields before consuming a cut or installing the whole alias set;
bounded quadratic uniqueness checks use no side collection. The same first-cause
health slot holds acquisition invariant faults. Existing source/context/participant/
reply envelopes fund staging; preinstallation refusal drops staged copies and
preserves original funding, registry and monotonically spent issuance.
The standalone native inert-data copier is caller-owned, not authored root data or
an implicitly funded invocation. Its explicit byte ceiling includes capture and
traversal-Vec headers, inline flat nodes, child-edge headers, text/key UTF-8 payload
and live traversal frames; its usage separates retained output from peak logical
copy workspace. Edge/key obligations are charged before reservation/copy, even
when not all children have yet been constructed. Nodes and root-one depth have
independent ceilings. Actual allocator capacity, caller input, physical peak and
global reservations are excluded. A decoded gateway must reserve this workspace
against its actual parent before invoking capture; this utility does not do so.
The native fixed-port source adapter now does reserve its classification frame and
seven maximum-sized frames against that actual coordinator retention parent, with
maximum min(caller capture bytes, invocation intent bytes). Scope templates drop
before facts staging, avoiding an eighth live copy/template frame. Complete peer
scopes/flags, owned native facts and producer-frame headers have independent
preflight bounds; opaque identities remain outside inert payload copying. A private
linear finalizer refunds only its own actual classification/workspace reservations,
never native context pools, holds or lifetime issuance. Busy/workspace control
fields are explicitly excluded from authored-root accounting; actual parent debit
remains visible in retention usage. Outstanding-debt surprises fence admission
without erasure; discharge failure cannot report that debt as zero. Native result
buffers remain separately funded by their existing producer profiles. This is not
physical/global accounting, exact JSON-byte equivalence or panic-abort recovery.
The native F03 adapter reuses this same owner guard and seven-frame envelope,
including complete coherent peer scopes/flags even for an annotation-only selected
member. No separate facade context/alias pool or source lease is created. Native
page bodies and fresh record/continuation aliases retain their existing producer
funding. Facade validation borrows reply metadata and uses constant auxiliary state
for bounded quadratic uniqueness checks. A pre-native fault spends no selectors;
a post-native/finalizer fault preserves the entire installed selector set and spent
issuance, refunds only temporary frames and fences release. Native protected
retirement can discharge those retained obligations without refunding issuance.
Native acquisition uses the same guard with a closed eight-frame family, not a
caller-controlled frame multiplier. Ordinary requested scopes/classes and separate
current/prepared/installed fact packets have independent native byte/node/depth
checks before ownership copies. Temporary actor, request-envelope, requested-row,
fact and producer-reference buffers remain within that declared envelope; native
payload capture/installation retains its own pool funding. Private cut matching
returns only a boolean and introduces no projection copy or retained identity set.
Pre-handoff refusal preserves preparation/native state. A late facade failure does
not refund installed contexts, aliases, spent issuance or consumed cuts. Undisclosed
native contexts remain bounded owned obligations, not proof of complete public
owner-shutdown or undelivered-result recovery. Native versus JSON/physical/global
accounting distinctions and unwind-only cleanup assumptions remain unchanged.
Managed replacement/selection invocation captures remain separate bounded work in
the existing ledger: complete original scope/query/command, selected ranges and
fixed native plan headers are charged against the invocation capture envelope,
alongside existing source/view staging limits. Native selection-request copies are
counted explicitly. No extra source-context hold or current semantic-root copy is
created. Retiring a search result removes eligibility, not those retained invocation
values or their own charges. Actual source/selection publication still uses the
existing candidate semantic-root admission and companion checks.
Internal holds do not create another root copy; their actual ledger prevents
premature stopped-snapshot/row discharge. Context
stop or row retirement never refunds the pool reservation or spent issuance.
Optional created-layer result holds likewise use actual parent retention, not the
semantic-root gauge: complete native context/identity capture, worst qualified
reply and settings max-state staging allowance are reserved before queue admission.
Their weak plan-identity witness owns no source/settings root. The existing layer
stage still checks candidate semantic-root capacity and pending baseline floors
before private result staging. Protected discharge releases the separate hold;
it cannot reduce semantic-root demand or refund invocation/basis issuance.

Native history metadata bases are caller-owned outputs, not an added authored-root
field, coordinator context pool or parent reservation. Their bounded native logical
charge includes basis/group/scope headers, complete participant-union workspace and
both frontier vectors, including absence and lost-floor state. It excludes journal
payloads and does not pin inverse history. Global metadata traversal is separately
bounded by declared group/scope/participant limits. Trusted-owner usage gauges do
not authorize recipient count disclosure. These native units do not establish
JSON-byte or physical-memory equivalence.

`history_inspection` is a separately retained, non-authored-root owner field. Its
actual parent reservation includes the pool, copied exact scope, `maxIssued`
optional-entry slots, `(maxContexts + 1)` native capture-plus-basis ceilings and two
reply ceilings. All arithmetic is checked before admission. Projection capture
includes scope/group row headers and alias payloads plus temporary group/scope-ID
lists and their headers; basis capture retains its existing union/stack charge.
Reply admission accounts for native variant headers and every disclosed alias/detail
row before copying. No new payload-sized validation shadow or journal pin is added.
Invalidation/stop preserves retained entries and charge. Entry retirement requires
owner control, delivery drain and retention discharge, removes the entry but not
spent issuance or the fixed pool reservation. Only stopped, empty owner retirement
releases that actual reservation, preserving a closed issuance tombstone. Protected
cleanup works under the global native fault fence; it never heals that fence.

The fixed native history query adapter shares the existing read gateway's linear
temporary reservations, not another authored-root or snapshot pool. It first bounds
and reserves the three-usize native classification frame, then seven caller-byte
frames. Their total must fit both the history owner's `maxHeldBytes - baseCharge`
and the actual parent ledger. History does not use the settings-intent clamp or its
history capture ceiling as a request-byte default. Independent actor/scope/query/
facts shadows, ownership copies and finite template/producer work fit the declared
seven-frame envelope; each inert shadow has independent byte/node/depth admission.
Opaque identities remain borrowed original handles, never copied label graphs.
Actual native history reply bytes retain the pool's existing two-reply funding;
retained alias validation adds no payload-sized shadow or new alias pool. Refund
releases only the guard's two linear reservations. Late faults preserve native
entries/issuance/base charge and any observed invalidation. Unowned workspace drift
is fenced, not erased. These bounds are native logical units, not JSON, caller-
retained reply memory, allocator/RSS, crash or complete global-memory guarantees.

Native source-reader storage is private pending work, not authored-root payload.
Its shared-retention parent reserves maxHeldBytes + maxIssued*sizeof(Option<Task>)
+ Store/State headers before exposing the reader. A task conservatively charges
its fixed header, two key incidences (reader and possible activation association),
maximum decoded source, total input, bounded chunk geometry, two maximum chunk
conflict payloads and eight validator fields (four possible epoch/token pairs).
Each live charge must fit the configured held ceiling; validation recomputes the
actual sum before new issuance or discharge. Lifetime slots do not recycle.
Source/annotation budgets are explicit separate read inventories, with one shared
outstanding-worker gate and shared actual retention budget. Retirement returns no
pool sponsorship; irreversible protected store retirement requires admission
fencing, zero actual tasks/held bytes and independent control/retention/drain facts.
Activation separately funds the document, binding and baseline through existing
root accounting; EOF/candidate preparation changes no authored root. These are
conservative native logical units, not transient allocator/RSS, wire, host-worker
or physical/global/crash guarantees.

Owned source operations add a separate actual shared-retention reservation:
2*maxIssued*maxCaptureBytes + Registry/State headers, checked before installation.
Each retained origin and complete native parent capsule has an independent bound;
the entry includes a prepaid tombstone and actual child identity. Raw labeled
history consumes the same bounded origin inventory without acquiring an owned
parent. Native source slots and operation slots never recycle. Parent/child drift
or payload-bound violations fault before removal/refund. Guarded child retirement
atomically drops only the parent payload and retains its spent identity; returning
the registry reservation requires source admission stopped, all actual source work
and pins drained and strict control/retention/drain facts. The underlying reader
cannot retire while that registry is active. These are separately sponsored
metadata and work pools, not a new publication root or physical-memory theorem.

Source-load observation reuses the shared observation ledger and fixed native
target/receipt sum. Admission requires a complete source-read receipt frame plus
four maximum validator fields inside the complete wait/reply frame, covering two
typed end witnesses without assigning epochs to opaque versions. Actual reply
accounting sums only retained epoch/token bytes, never raw chunks or candidates.
The existing 5R+2C+3X+(W+1)B+(I+4)N declaration, whole-child sponsorship and
independent protected discharge remain unchanged; widened fixed native variants
are included by their current size projections.

Optional native F06 decoding is separately sponsored inside the actual native
observation context: 8*input.max_bytes + Decoder/State headers, checked against its
explicit decoder-held ceiling and actual shared parent before activation. The
existing native pool/context size projections include the widened fixed record.
Queries borrow that prepaid workspace and reserve no new admission/byte credit.
Whole-envelope, actor and canonical shadows are each bounded by native inert copy
usage; opaque roles are null in those copies. Policy/fact shadows, normalized
ownership and fixed templates fit the remaining declared frames. Classification
checks include the largest 32-field template plus two lossless u64 measurement
words; this is not JSON or a selected public epoch encoding. Current policy/fact
copies are bounded before native service. Native reply/continuation envelopes keep
their independent existing funding. A decoder fault, native receipt fault, stop
or scope loss returns no workspace credit. Busy ownership prevents recursive
service/retirement; guarded actual context retirement releases both reservations
only after native waiter cleanup and independent protected drain facts. These are
native logical bounds, not wire bytes, allocator/RSS, a normalized admission-pool
refinement or hostile/proxy/concurrent/crash proof.

The optional discovery reader is also outside authored-root payload. Its actual
shared parent reserves the configured maxHeldBytes, after checked declared demand:
X + P*B + C*(X+2A+2R) + 2S*N plus native Store/State and packet/context slot headers
and the fixed builtin-source staging envelope. The latter prepays four X-bounded
identity strings, owner-inspection row headers and the complete six-variant
metadata frame (checked native `builtin::workspace_bytes`). The private source
marker is part of the native Packet/Context headers, including capture-byte checks.
X is scope, P/B preparation slots/packet bytes, C context slots, A capture bytes,
R reply bytes and S/N selector slots/record bytes. Native queries stage only new
selectors, bound complete replies (including exposed inert metadata) and reserve
the destination selector capacity before installing any selector or spending
issuance. Live selector records and tentative duplicates fit the prepaid 2S*N
envelope. Each native record must fit N; native handles have separate role-bearing
reply cost. Iterative metadata equality bounds pair-work by the capture ceiling.
The pre-install native result guard borrows the actual reply, original context and
tentative map. It adds no metadata allocation, new parent reservation, serializer
or schema execution. Checked native reply/selector counts bound its loops; immutable
copier-owned metadata supplies the finite native usage bounds and approved payload
identity. It does not establish JSON costs, host truth, allocator/stack/RSS or
physical safety. Guard fault keeps the consumed cut spent and preserves parent
sponsorship and previous issuance; no tentative selector is installed.
It does not allocate/clone a full raw-registry projection. Complete row/schema/presentation metadata
and packet projections are checked against remaining native ceilings before each
copy/allocation; context projection is sized before materialization. Inert-copy
limits include nodes/depth/traversal workspace, not exact JSON bytes or allocator
capacity. Only the visible captured domain is installed. Native current-root
identity now covers the audited payload projection; it does not prove external
accepted-registry/schema truth or normalized/native/physical correspondence.
The fixed builtin /1 source discharges declaration origin for the handler-owned
F13/F14 inventory only. Generic input still requires independent external origin
attestation. Builtin and generic cuts cannot replace one another even when their
projected metadata is identical; source recognition precedes context loss effects.
Request-role/scalar/cursor recognition also precedes loss effects and cut
consumption. It borrows original bounded tables without allocation, staging or
issuance; foreign/mismatched requests leave existing obligations intact. Current
page capacity and delivery readiness remain later checks, so valid recognized
loss is still absorbing. Optional query timing adds no identity-bearing labels or
new retained reservation and is not a queue, transport or physical-memory measure.
Stopping/retiring contexts or consuming packets does not refund parent credit or
lifetime issuance. Actual global drain, independent protected facts and irreversible
registry retirement release only that original reservation. Integrity failure
preserves uncertain funding and fences discovery separately from the editor root.

Each explicitly enabled F01 native decoder has separate original parent sponsorship:
8 × input.max_bytes plus native decoder state/record headers, bounded by its explicit
maxHeldBytes. Fixed template admission, copied node/depth/byte limits and caller
ceilings precede handoff. The three native inert copies and owned normalized meaning,
fifteen current-query facts and fixed templates borrow these conservative native
frames; reply/packet/selector/context obligations retain their existing sponsorship.
This is logical credit, not preallocated buffers, canonical JSON or physical/stack/RSS
proof. Opaque roles never enter inert traversal. The full policy epoch stays u64;
its two-u32 budget shadow is not a wire identity/epoch encoding.
The context's inline decoder-state header is included in actual Context sizing;
active decoder metadata/workspace has the separate once-reserved charge. Query
success/refusal/fault/stop returns no decoder credit. Busy contexts cannot retire;
actual stopped-context retirement validates and releases only that original decoder
reservation before removing the context. Uncertain declaration/funding drift does
not erase debt. Late delivery failure cannot undo native selector installation or
issuance. Existing global packet/context drain still controls registry retirement.

Future mutation paths must compose their actual candidate with the root-slot floor,
not check only the old root or rely on allocation sharing. Scoped lifecycle private
materialization/service uses the shared native layer kernel; public facade assembly
and the other engine acceptance-plan families remain open. The route audit and tests are not unbounded induction,
host-policy truth, physical memory/crash safety or independent Cleanroom assurance.
