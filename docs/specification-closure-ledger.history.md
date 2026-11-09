# Specification closure ledger

This ledger controls completion of the approved v1 design and scoped-reference
review. It separates unfinished contracts from implementation and release evidence.
It does not replace requirements, select new behavior, waive acceptance gates or
certify existing contracts. The previous approximate 50 percent estimate is withdrawn.

## Baseline and reporting

### Provisional baseline and implementation-first direction

The user accepts the current design as a provisional baseline, not a completed
specification or production implementation. The previous goal has been cleared.
Future progress must be measured by working Rust engine behavior and closed
deliverables, not accumulated reference checks or snapshot counts. The historical
71% below measures accepted design items only; it is not engine completion.

The later inherited 95–96% implementation estimates are also withdrawn: they
were not based on a measured denominator. Current engine completion is governed
by the seven remaining acceptance gates C1–C7 in
[the engine test plan](engine-test-plan.md#fixed-remaining-v1-acceptance-gates).
Historical passing counts and per-slice estimates below do not close those gates.
This reporting reset does not waive required engine behavior or expand scope.

Executable contract and regression tests belong with the Rust implementation.
Port selected JS reference invariants as behavior is implemented; do not require
a wholesale mechanical translation before building. Retain useful independent
models and counterexamples as design references, without treating their passes
as implementation evidence. Instrument the actual Rust execution and substrate
boundaries; instrumentation complements tests and does not prove correctness.

The first delivery target is a narrow open → validated atomic edit → query →
local-filesystem save path, preserving lossless content and explicit failure
results. UI remains outside the engine; clipboard workflows and host handlers
belong to integration, and filesystem mechanisms belong behind the modular
substrate boundary. Provisional placement is not qualification of the remaining
public contracts. Resolve only ambiguities that block the slice, explicitly;
do not silently promote remaining choices or unsupported platform guarantees.

The three native settings inner argument declarations now directly drive their
decoders' exact field/contract/operation guards, and normalized products expose
the same static descriptions. This closes that native declaration/decoder binding,
not complete discovery schemas, accepted registry assembly or discovery /2.
Verification passes 319 focused Rust tests (21 native unit and 298 public settings
scenarios), formatting and strict engine all-target/all-feature Clippy. No observed
red-first cycle, new formal proof or full-workspace checkpoint is claimed for this
slice. The broader debug batch began before these edits and remains separate;
release/Unicode qualification is pending. Overall estimate stays approximately
95%, low confidence, and the engine goal remains active.

The Rust implementation now lives in `crates/vellum-engine`, with the root `Makefile`
and `README.md` defining repeatable checks and explicit coverage limits. It
provides optional bounded callback-free diagnostic capture, a test-only
scripted substrate seam, pure source preparation, trusted-owner coordination,
bounded invocation replay, linked source history with pins/retirement and pinned
source saves with separate application/control evidence and baseline/delivery holds.
Trusted exact literal search/replacement now has bounded session/result retention,
explicit navigation/wrap, current/all replacement and invocation replay through
ordinary atomic source/history publication, including final supersession checks.
Owned search-occurrence selection now uses the common selection invocation ledger
with explicit replace-all-selections/replace-primary/add-primary, direction and
affinity. The original result, generation, source and view context are checked at
preparation and publication. Known partial occurrences may select or replace
explicitly; navigation and replace-all require completeness. Normalization never
changes the original replacement range. Bounded descriptors charge retained result
data, and optional content-free occurrence-resolution capture does not evaluate a
query. Recipient/async and complete retained selection-result products remain open.
Native `bounded-source-scope/1` now supports exact participant/revision restrictions
with independent replacement ranges and inclusive insertion-site intervals. The
ordinary source/compound publication path validates the retained restrictions,
including empty/no-op requests, without changing authored edits or history policy.
Scoped source and compound invocations use the common one-shot ledger, full bounded descriptors
and original restriction/work-profile comparison. Ten BDD scenarios and three
native regressions cover finite cell/gap oracles, malformed/foreign/stale scopes,
compound authority/history, bounds, replay/stop/forget and capture integrity.
Content-free scope preparation/validation observations remain optional. This does
not close recipient policy, mapped scopes, complete result assembly or
the complete command/discovery facade.
Native compound declared reads now cover source revision, annotation-set version,
live body version and live authored-target generation. Original-root equality and
presence validate before publication/no-op classification under explicit finite
predicate/work bounds. Duplicate subjects reject; canonical order does not rewrite
authored edits. Read-only subjects gain neither mutation nor authored history.
Read-enabled compound invocation/replay composes with source scopes in the shared
ledger/publisher. Advice is current read-only checking, not admission or a future
reservation. Seven BDD scenarios include 512 finite predicate-selection/mismatch/
changed-no-op cases; three native regressions cover descriptor bounds and captured
read integrity. Content-free preparation/advice/final-validation boundaries remain
optional. Configuration predicates, recipient grants, semantic dependency
discovery and complete F10/F11 qualification remain open.
Native optional logical workspaces now have bounded non-reused containers, shared
membership and exact generation checks. Retirement removes only the addressed
container/edges; membership does not transfer source ownership or choose settings.
Both document disposal paths stage all actual membership removals and counter
advances before one publication with source liveness and view detachments. Six BDD
scenarios include 1,728 finite membership traces; three native tests cover counter/
work exhaustion and accounting contradiction without partial mutation. A separate
v2 read profile adds live workspace membership predicates, leaving v1's enum closed.
Four more BDD scenarios include 2,048 mixed predicate cases, ABA/retirement/disposal,
disjoint frames, zero/foreign/duplicate subjects, profile-bound replay, scoped
composition and optional instrumentation. Complete lifecycle recipient/event and
host-policy qualification, durable/imported identity and settings
associations remain open; no complete F17/workspace facade is claimed.
Native retained view/workspace create/retire now uses the common invocation table,
profile-bound immutable meaning and direct controls' actual owned staging. Current
roots/caps and exact retirement expectations revalidate at service; creation never
reserves an identity at admission. Typed trusted-owner receipts are prepared before
publication. Known refusals stay known; altered captures/kinds or interrupted
publication fence Unknown without a fabricated rejection or identity receipt.
Eight BDD scenarios and four native regressions cover all four commands, shared
frames, fresh service-time allocation, stale targets, stop/forget/retirement,
admission/current capacity, absent/changed profiles, descriptor thresholds,
optional diagnostics and interruption after actual root installation. This does
not qualify dynamic adapters, recipient result ceilings or the complete F17 family.
Native semantic-settings compilation now validates the adopted six closed types,
explicit scopes/defaults, finite restrictions, logical layer/target identities and
associations. Resolution includes absent inheritance, explicit library reset,
replacement, declared top-field merge, precise winning origins and equal-rank
conflict rejection even under a higher override. Effective equality treats numbers
mathematically, lists in order and maps by fields, while native number bits survive
copying. Iterative traversal and flat arenas bound deep data without recursive engine
copy/drop. Initial owner installation and trusted reads are executable and preserve
source/history semantics; initial compilation cannot overwrite live settings.
Nine BDD scenarios include 750 independent scope-oracle cases, all types, deep data,
limits, metadata/origins and diagnostic equivalence. Two native tests cover exhausted
publication and foreign/forged target handles. Mutation invocation/replay, funded
interpretation retention, configuration read predicates, actual consumers and full
recipient F07/F16 qualification remain open, not closed by this compiler slice.
Native direct set/remove/reset activation now rechecks exact written-layer and
affected-target contributor generations plus immutable interpretation context.
It rebuilds from current state rather than replacing it with an old candidate;
disjoint changes survive, common-contributor changes stale. Layer/configuration
metadata and per-target effective domain epochs advance independently. Mathematical
no-ops preserve original number bits and require no unused publication counter.
Request/plan logical bytes, nodes, depth and work are bounded; flat rebuilding
compacts overwritten and derived values. Nine further BDD scenarios include 125
independent set/remove/reset traces, disjoint/stale/ABA cases, all domains, stable
repeated-update space, bounds and capture equivalence. Four native regressions
cover exhausted counters, final publication refusal and signed-zero identity.
These are trusted direct preparations, not shared invocation replay or funded
interpretation leases; arbitrary caller-held plan counts are not parent-funded.
Mutable schemas/restrictions/associations/layer lifecycle, additional caller read
conjunctions, command consumer binding and recipient products remain open.
Activation checkpoint: 505 default workspace tests pass in debug and release,
with formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference tests passing in each profile. The rough implementation planning
estimate is 82%; this is not measured coverage or complete F16 qualification.
Native settings jobs now use the shared one-shot ledger and the same actual
staging/publisher as direct activation. Structural admission bounds precede binding;
an explicit activation profile is required. Independent flat intent captures
preserve numeric bits, map meaning and ordered changes without recursive engine
copy/drop. Known original native preparation refusals remain original after later
state changes. Service revalidates intent/preparation integrity, current profile
and original dependencies, then installs root and typed historical outcome under
one fault fence. Stop/forget/retire retain common spent-identity semantics; private
capture drift or interrupted installation yields Unknown, not a known rejection.
Five further BDD scenarios and six native regressions cover retry, all operations,
signed zero/map/order equality, disjoint/stale service, source frames, unavailable
preparation, descriptor/structural limits, profile/capture drift, post-install
interruption and diagnostic equivalence. The independent finite trace test now
runs 125 traces through each direct and retained route (250 total).
This closes native mutation replay, not dynamic/recipient F16, extra caller reads,
funded interpretations, definition/layer lifecycle or command consumer binding.
Retained-settings checkpoint: 516 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates passing in each profile. Rough planning estimate: 83%,
not measured coverage. The full engine implementation goal remains open.
Native interpretation retention now reserves an explicit worst-case pool envelope
against actual owner-local logical retention headroom. Private owner-bound funding
tokens reject foreign/double refunds; failed bounds/headroom/allocation checks do
not install a partial pool. Non-reused opaque lease IDs retain actual immutable
roots across activation; cloned IDs do not mint storage. Release restores live
capacity, not lifetime issuance or the prepaid envelope. All real settings
preparations retain a counted hold, including pre-pool and invocation-held plans.
Retirement requires no leases/plans, refunds once and leaves a closed marker.
Protected discharge remains available after fault without healing the owner.
Six BDD scenarios and four native regressions cover exact funding/size thresholds,
overflow, competing actual reservations, historical roots, stale/foreign/forged
handles, issuance/live capacity, pre-pool preparations, stop/discharge, foreign
funding-token drift and optional instrumentation. Existing source/history/effect
budgets and arbitrary embedding-held plan memory are not claimed parent-funded;
configured consumers, extra configuration reads, other F16 products and complete
retention/lifecycle/recipient integration remain open.
Interpretation-retention checkpoint: 526 default workspace tests pass in debug
and release, plus formatting, strict all-feature Clippy, documentation tests and
both strict Unicode reference integration gates in each profile. Rough planning
estimate: 84%, not measured coverage. The full implementation goal remains active.
Native configured-line jobs now resolve an exact target/key-role map against
actual registered source-domain values. Explicit bounds cover copied request and
inner meaning before charged lease issuance. Independent retained intent and
prepared witnesses bind actual interpretation, epochs, source-domain generation,
context and work profiles; matching retries never resolve again. The common
source publisher checks those original dependencies and values together with
ordinary source/view/annotation/history companions. Metadata-only equal-value,
unrelated target/domain progress survives; source-domain ABA and selection/profile
changes refuse even on no-ops. Stop/forget/fault do not discharge leases. Protected
release/retirement does, without reopening execution or refunding issuance.
Eight BDD scenarios include all three actions, 32 independent prefix oracles,
history restoration, disjoint/stale/no-op frames, malformed/foreign configuration,
quota/structural bounds and optional diagnostics. Five native regressions cover
outer/inner descriptor thresholds, exhausted no-op counters, lost/substituted
captures, tightened profiles and interrupted actual source installation followed
by protected discharge. This closes trusted native line consumption, not recipient
read/use authority, dynamic F11 arguments, all settings consumers or full F16.
Configured-line checkpoint: 539 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates in each profile. Rough planning estimate: 85%, not measured
coverage. The full engine implementation goal remains active.
Recipient-bound conditional layer bases now have native capture, availability-only
inspection, stop and protected retirement. Full seven-field contexts, fixed profile
and actual contributor/interpretation dependencies bind to the owner. The engine
checks normalized trusted facts without claiming authenticated policy truth.
Foreign/malformed/unknown facts do not establish loss; recognized authority/write
or actual footprint loss closes a retained basis. Budget/readiness refusal alone
does not. Interpretations and bases share prepaid live capacity, tombstone storage
and nonrefundable issuance, but distinct handle types and row kinds. Context/root/
footprint bytes are included in the bounded capture envelope. Closed/stopped rows
remain charged until independent retention/delivery discharge, including after
fault without healing the owner. Nine BDD and five native regressions cover
contexts/guards/frames, absorption, funding/bytes, variant/slot integrity, actual
work-budget recovery and optional redacted instrumentation. Scoped first-party
lowering, whole-proposal validation, service/replay and recipient codecs remain
open; this does not close §22.3.1.5 or raise the rough 89% planning estimate.
Conditional-basis checkpoint: 622 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates in each profile. The full implementation goal remains
active; scoped mutation lowering, service revalidation and replay are next.
Typed scoped ordinary mutation now has direct and retained execution through the
existing native normalizer/resolution kernel and common publisher. Unique fully
used basis vectors replace requester native conditions. Whole-proposal semantic,
read and complete plan/parent wrapper preflight precedes stateful loss observation;
invalid later input cannot poison an earlier basis or bind a slot. Original
immutable context/vector/native intent/read captures and independent seals reject
refresh, profile reinterpretation or valid-plan substitution. Current host facts,
gateway, original bases and required/additional reads are rechecked at service.
The generic runner cannot execute a pending scoped job without fresh facts. Its
fact-bearing service port shares the same owner/ledger/root/outcome fence; no new
transport or engine mode is introduced. Canonical host/caller overlap does not
permit dropping original mandatory requirements. Success exposes changed/unchanged
only. Cancellation/forget/retirement never rearm work or implicitly release bases;
interrupted actual installation stays Running/Unknown. Eleven BDD and six native
tests cover this behavior, full independent wrapper thresholds, private drift and
bounded optional signals. Dynamic codecs, recipient result release, scoped
registry/lifecycle variants, real policy and physical/refinement gates remain open.
Observed admission-time integrity faults also latch the owner, without registering
an invocation or leaving a preparation hold. Ordinary refusals do not fault it.
Ordinary admission authorization/readiness and layer grants precede semantic/read
queries; data-dependent grant scanning has an explicit work ceiling and cannot
close a basis merely because that budget is exhausted.
Scoped-mutation checkpoint: 639 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates in each profile. Rough planning estimate: 90%, not measured
coverage. That checkpoint preceded the intrinsic serialization slice below.
Native intrinsic document serialization now has a separate positive-safe generation
and closed strict-UTF-8/BOM output intent, without settings-registry installation.
Trusted direct/retained commands bind exact target/context/profile, mandatory and
caller reads and the existing publisher/one-shot ledger. Complete native logical
request/plan/parent bounds apply; identity needs no unused counter. Metadata does
not mutate source revisions/bytes/history or undo meaning. Source save artifacts
and tasks retain matching original format state, checked before dispatch/baseline
publication. Current saved-state comparison encodes current intent, while old
acknowledgments remain historical and cannot release writer ownership. Unencodable
intent refuses capture before issuance/I/O. Ten BDD plus nine native tests cover
those paths, actual counter/budget thresholds, private corruption and interrupted
installation staying Running/Unknown. Optional bounded format signals expose no
content. This is native implementation evidence, not new universal formal proof,
wire/refinement equivalence or physical crash guarantees. The full goal remains
active: at that checkpoint scoped format bases, registry/lifecycle variants and
public recipient contracts remained open. Rough planning estimate then: 91%, not measured
coverage; the historical 90% checkpoint above is not the current estimate.
Serialization checkpoint: 658 default workspace tests pass in debug/release,
alongside formatting, strict all-feature Clippy, documentation tests and both
strict Unicode reference tests in each profile. Frozen formal evidence and its
hashes/censuses remain unchanged; this checkpoint does not replace their scope.
Native metadata-only format bases and private scoped serialization lowering are
now implemented. Their dedicated profile funds the native pool/frame, lifetime
tombstones and context envelopes against actual shared owner headroom. No source,
old root, registry or baseline is retained. Exact context/profile/target facts bind
before recognized absorbing loss; transient/unknown/foreign facts cannot poison a
basis. Source progress commutes, while actual format change/disposal invalidates.
Scoped commands derive target/generation privately and retain original basis
identity through the common native plan, publisher and exclusive invocation slot.
Complete semantic/read/whole-wrapper preflight precedes stateful observation;
fresh basis-resolution permission does not replace native mutation authority.
Raw service/commit cannot bypass scoped validation. Historical replay, spent
issuance, stop-held charge and protected fault-time discharge remain independent.
Eight BDD plus thirteen native tests cover budget boundaries/competition, binding,
loss, current reads, exclusivity, private substitution and interrupted actual
publication remaining Running/Unknown. Bounded lifecycle/scoped signals emit no
content/conditions/identities. The shared prepared format plan's Debug output is
redacted, including its native condition and reads; its private fields are not
exposed through scoped formatting. Public codecs/recipient projections, scoped
registry/lifecycle variants, host-policy truth and physical/refinement gates remain open.
Rough planning estimate: 92%, low confidence, not measured coverage. The full
engine goal remains active; frozen formal archives/censuses/hashes are unchanged.
Scoped-serialization checkpoint: 679 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference integration tests in each profile. The gates include the shared
prepared-plan Debug redaction and its scoped disclosure assertion.
Native declaration-role bases and scoped definition activation are now implemented.
The distinct typed basis shares the funded settings pool and spent issuer, retaining
only context/profile and the exact interpretation-facet triple. Ordinary layer/read
permissions do not grant this role. Complete binding precedes recognized absorbing
loss; transient/unknown/foreign facts cannot poison a basis. Source and ordinary
layer progress commute with the condition; actual facet progress invalidates it.
Scoped first-party lowering derives the expected tuple privately and preserves the
native full preparation footprint, semantics and changed-facet grant independence.
Complete request/read/plan/actual-parent wrapper preflight precedes stateful loss
observation. Current required/read-count checks prevent copying changed requirements
before refusal. Raw/scoped slots, original basis identity, historical replay, typed
changed-only success, redacted plans and Running/Unknown publication fencing remain
intact. Stop/forget do not retire the independent basis; protected cleanup works
after fault. Ten BDD and six native regressions cover those behaviors, exact budget
boundaries, private corruption and actual post-install interruption. Bounded optional
basis/scoped signals contain no conditions/content/identities. Scoped authored-layer
lifecycle, public recipient products and broader async/provider/control work remain
open. Rough planning estimate: 93%, low confidence, not measured coverage. The full
goal remains active; frozen formal archives/censuses/hashes are unchanged.
Scoped-declaration checkpoint: 695 default workspace tests pass in debug/release,
alongside formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference integration tests in each profile. Native implementation evidence
does not replace frozen reference/formal claims or close their remaining gates.
The independent conditional layer-lifecycle basis controls are now native, sharing
settings-pool funding, record bounds, context capacity and spent issuance. Distinct
typed handles/rows retain context/profile and only the configuration/incarnation
pair, not payload or an old root. Source-only progress commutes; actual settings
progress invalidates. Exact binding precedes absorbing recognized loss, without
poisoning on unknown/transient/foreign facts. Observation grants no mutation.
Stop holds charge; protected drain-based retirement works after faults and counter
exhaustion. Six BDD and four native tests exercise these controls, exact record
bytes, valid/corrupt issuer pairs, wrong-role/slot forgery and bounded optional
content-free signals. This is not private first-party resolution or scoped
materialization/service, which remain open.
An explicit prerequisite remains: §22.3.1.8's whole-owner-root candidate ceiling
has no native measure/budget yet. Coordinator document/participant/staging/source,
search capture, configuration state/plan and parent intent ceilings are not that
aggregate guarantee. It has not been waived, replaced by a host claim or silently
identified with settings bytes. Implement its actual root guard and tests before
claiming full scoped lifecycle closure. Rough planning estimate remains 93%, low
confidence; the full goal and other open families remain active. Frozen formal
archives/censuses/hashes are unchanged by this implementation slice.
Lifecycle-basis checkpoint: 705 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference tests in each profile. Zero-high-water evidence uses fresh
empty compiled settings; the counter-only inconsistent fixture was replaced before
this checkpoint. No scoped lifecycle or aggregate-root closure is claimed.
Aggregate-root audit: reference settings preparation bounds serialized candidate
root JSON, whereas the native coordinator co-locates semantic/history/persistence
state with separately bounded pending/retained work. The missing native profile
is therefore an admission-contract choice, not just a number. A user selection
is requested between a versioned engine-computed native logical semantic-root
measure (recommended, separately budgeted held work) and exact canonical reference-
root JSON bytes (requires compatible projection/codec). No option, root-field set,
default budget or equivalence claim is silently adopted. The full goal remains
active and the 93% low-confidence estimate is unchanged pending that selection.
Independent read-bound hardening: ordinary scoped settings and native/scoped
serialization now refuse changed mandatory counts before fresh descriptor copying;
serialization retries reject changed caller counts without reinterpreting input.
Three observed red-to-green native regressions preserve full same-count validation;
one BDD scenario verifies no publication/basis loss with optional/saturated capture.
No aggregate-root profile or budget is selected by this fix; the goal stays active.
Read-bound checkpoint: 709 default workspace tests pass in debug/release, with
formatting, strict all-feature Clippy, documentation tests and both strict Unicode
reference tests in each profile. Frozen formal evidence is unchanged.
Managed source codec edges now emit bounded optional completed/refused signals at
actual document/new-load decoding and save capture/current-comparison encoding.
Metadata, existing-binding reuse, pinned dispatch and earlier admission refusals
do not fabricate conversions. Source fidelity/results remain unchanged, and codec
completion is not publication or persistence evidence. Five red-to-green BDD
scenarios plus one native publication-exhaustion case exercise these paths without
content/identities/offsets/raw errors or clock callbacks. Standalone pure codecs,
physical I/O and recipient-release policy retain their separate boundaries. This
selects no aggregate-root profile; the goal and existing 93% estimate remain open.
Codec-edge checkpoint: 715 default workspace tests pass in debug/release, with
formatting, strict all-feature Clippy, documentation tests and both strict Unicode
reference tests in each profile. Frozen formal archives/hashes/censuses are unchanged.
The user subsequently selected **Native** logical-root accounting. The pending
profile choice is resolved and work resumes; no numeric ceiling is inferred.
At selection time, engine-computed semantic-root measurement, candidate admission/
publication checks and scoped lifecycle lowering/service remained unimplemented.
The decision itself closes none of those obligations or other roadmap families
and establishes no heap/crash bound.
The first resumed slice now implements `native-semantic-root/1` current-root
measurement and bounded content-free diagnostics. Actual current semantic records,
history/frontiers and persistence baselines are counted separately from held work;
checked arithmetic refuses overflow and spare allocation capacity is excluded.
See [cost projection](native-root-accounting.md). Native/BDD checks cover actual
component growth and held-root/save/snapshot exclusion. Aggregate admission and
publication enforcement still remain open; no numeric ceiling is inferred and
the existing 93% estimate is unchanged. Frozen formal evidence is not modified.
Root-measurement checkpoint: 723 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference integration tests in each profile. Eight native/BDD tests were
added; no universal refinement, global-capacity or physical-memory proof is claimed.
Shared settings root-preflight/publication integration is now native. The explicit
immutable ceiling checks actual current-root fit on configuration, then value,
additional-read value, declaration and layer preparation check their known compiler
candidate against the actual whole root before plan retention. An absent ceiling
is unavailable, not unlimited. Initial installation checks a preconfigured ceiling
or bootstraps before ceiling configuration measures the current root. Service
recomputes the candidate and remeasures current unrelated state; refusal installs
no prefix. No-op counter behavior, terminal retained refusal/replay and preflight
before scoped ordinary/declaration basis-loss observation are tested. Six native
and six BDD tests cover the slice, with bounded optional/saturated diagnostics.
Other root-growing pathways still need capacity integration; neither a global
invariant nor scoped lifecycle-family closure is claimed. Estimate remains 93%,
low confidence, and frozen formal evidence remains unchanged.
Settings root-admission checkpoint: 735 default workspace tests pass in debug/
release, with formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference integration tests in each profile. No global-capacity,
physical-memory, crash or universal refinement claim is added.
Compound publication now checks one complete staged root over source, annotation
values, mapped selections, retained journals and actual frontier replacements.
Unrelated current state is remeasured; reclaimed selection records may fund source
growth. Capacity refusal precedes every participant/generation installation and is
terminal under retained replay. Standard annotation creation admits its spent slot
cost before issuance; later refusal never refunds already admitted identities.
Eleven native/BDD tests cover exact optional/saturated boundaries, multi-document
atomicity, slot refusal, retained retry, undo/redo/branching, late peer history,
selection reclamation, arithmetic and no-op counter floors. Two BDD regressions
first failed on the old code and passed with the guards. Source birth, standalone
view/workspace paths, persistence, scoped lifecycle and other roadmap obligations
remain open. Estimate stays 93%, low confidence; no global capacity invariant or
new universal/physical/refinement proof is claimed.
Compound root-admission checkpoint: 746 default workspace tests pass in debug/
release, with formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode integration tests in each profile. Frozen formal archives, hashes
and censuses remain unchanged.
Source birth now checks its decoded payload and spent slot before issuance;
initial resource loading checks the combined source/binding/baseline/validator
candidate before either owner state is installed. Known aliases join unchanged.
Logical view/workspace birth revalidates headroom through retained service and
installation; membership and the shared direct/retained selection publisher check
actual old/proposed payloads. Release reclaims payload without refunding spent
slots. Provider/encoder/restorer registration admits actual registry handle/body
growth before issuance. Retained refusal cannot rearm when headroom later returns.
Twelve native/BDD tests cover exact/no-prefix/no-spend/replay/reclamation boundaries,
optional/saturated diagnostics and malformed internal subtraction as a fault. Five
BDD regressions first failed on the prior code and passed with the guards. Other
persistence baseline/binding/activation paths, scoped lifecycle and roadmap
families remain open. Overall estimate is roughly 94%, low confidence; no global
capacity, universal refinement or physical-memory/crash proof is claimed.
Birth/logical-state/registration checkpoint: 758 default workspace tests pass in
debug/release, with formatting, strict all-feature Clippy, documentation tests and
both pinned Unicode integration tests in each profile. Frozen formal archives,
hashes and censuses remain unchanged.
Persistence candidate admission now covers source/written/loaded baseline
replacement, initial annotation binding and sealed restoration. Complete record/
key/slot payload is checked before installation. Ordinary saves additionally
reserve logical root-slot demand (maximum per document, additive across documents),
separately from each hold's retention/counter charge. Competing candidate guards,
settings preflight and late ceiling configuration preserve that floor; installation
validates/consumes only its own credit. Unknown effects retain headroom until actual
protected discharge. Corrupt promises fault mutable admission without issuance;
protected stop works, and consumption cannot launder an underreserved charge.
Trusted-owner demand gauges distinguish current/future bytes without adding them
to diagnostic labels. Thirteen new native/BDD tests cover these paths, including
five observed red-to-green behavioral regressions. The native mutation-route audit
maps current gates and non-growing cases; this closes the current native capacity
prerequisite, not universal/physical/crash/refinement proof or independent assurance.
Scoped lifecycle, public facade and the other full-goal families remain open.
Overall estimate stays roughly 94%, low confidence; frozen formal evidence remains
unchanged.
Persistence/root-slot checkpoint: 771 default workspace tests pass in debug/
release, with formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference integration tests in each profile. Thirteen tests were
added. Frozen formal archives, hashes and censuses remain unchanged.
Scoped lifecycle lowering/service is now implemented for the typed trusted owner.
Create authority/incarnations and removal generations are privately derived from
the held configuration/issuer pair. Complete associations preserve exact native
identities and local create-index forms. Actual changes require fresh registry,
association and full-identity layer grants; no-op avoids unused grants/counters.
Complete semantic/read/root/plan/parent-wrapper preflight precedes stateful loss.
Original context, basis, reads and reference intent remain sealed through replay.
Direct and retained paths use the existing native layer publisher and one-shot
ledger; generic service cannot bypass current facts. Protected basis retirement
survives faults, and source/history progress is preserved subject to declared reads.
Thirteen BDD scenarios and six native tests exercise boundaries, permissions,
replay, no-op exhaustion, history, sealing, corrupt state and post-install Unknown.
Bounded optional phase diagnostics remain payload/identity/epoch-free. Public
codecs, created-reference result delivery and recipient integration remain open;
this is not complete public facade closure or universal refinement. Overall
planning estimate is roughly 95%, low confidence, not measured catalog coverage.
Frozen formal archives, hashes and censuses remain unchanged.
Scoped lifecycle checkpoint: 790 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Nineteen tests were added;
this implementation evidence does not enlarge frozen formal proof claims.

The native lifecycle write-basis selector consumer (§22.3.1.11) is now implemented.
Bounded entirely-used vectors select distinct actual layers, with pure full
footprint checks before stateful permission loss and complete native/root/wrapper
preflight. Fresh selected-basis facts remain independent of registry/lifecycle/
association grants. Original opaque handles, private identities, removal indices
and raw reference forms stay charged/sealed through replay. Both forms share
admission, participant, runner, outcome and publisher. Source-only progress
commutes; stopped bases fence queued work, and invocation retirement never retires
independent bases. Nine BDD scenarios and two native tests exercise selector
validation, source/replay frames, fresh facts, empty no-op, exact parent limits,
bounded optional instrumentation and actual post-install Unknown/protected cleanup.
Optional historical created-reference production/release and public codecs remain
open. Estimate remains roughly 95%, low confidence; frozen formal evidence stays
unchanged. This is implementation evidence, not independent Cleanroom assurance.
Selector checkpoint: 801 default workspace tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference integration tests in each profile. Eleven tests were added.

Optional native created-layer results (§22.3.1.10) now take an owned native or
scoped/basis-selected preparation through a separately funded first-party
admission port into the same invocation/publisher. Complete native capture/reply
bounds and max-state staging allowance consume actual parent headroom before
admission. Durable holds own no root; weak witnesses bind the original plan.
After native/root/promise validation, actual created identities/counts govern
private staging and common successful terminal publication. Ordered opaque
references release only under the original context and current full-envelope
result/outcome/history facts; flags are trusted premises, not host-policy proof.
Fresh basis acquisition rechecks actual full identity/current rights and consumes
the existing funded pool. Same-label recreation cannot retarget references.
Protected forget invalidates mappings/releases the hold once without retiring
independent bases or rearming work. Generic controls cannot bypass discharge.
Nine BDD scenarios and three native tests cover both routes, selectors, source
progress, permissions, replay/fallback, basis independence, reincarnation, no-op,
stale/stop, exact result/parent bounds, plan substitution and private/post-install
faults. Native accounting is not JSON/physical equivalence; early faults preserve
the old root, while post-install Unknown does not assert rollback. Bounded optional
phase signals contain no authored data, identities or native stamps. Public codecs,
other operation results, actual policy and whole-family refinement remain open.
Planning estimate remains roughly 95%, low confidence; frozen evidence is unchanged.
Created-result checkpoint: 813 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Twelve tests were added;
these native results do not enlarge the frozen formal proof claims.

Context-bound pending cancellation (§17.1.1) now validates actual owned ticket,
complete original context, strict independent protected-control facts and original
intent/prepared metadata witnesses. Its monotone inline flag does not settle or
refund. Actual healthy driver/specialized service records stopped-before-publication;
ordinary edit/read permission is not needed. Current scoped ordinary/definition/
lifecycle settings and intrinsic serialization classes support it; other jobs
without execution context refuse, and result context is never promoted. Both
serialization forms, lifecycle selectors and protected result holds compose with
the same path. Independent peer work and basis/result reservations survive until
their actual owning settlement/discharge. Publication-first and Unknown states
cannot be relabelled; wrong-context input cannot poison the owner. Eight BDD and
five native tests cover these frames, all control/context gates, event orders,
replay, metadata substitution and optional content-free request/settlement signals.
The existing privileged immediate stop remains separate. All F18 forms, service
fairness, generic async scheduling, provider quiescence, deadlines, fencing and
shutdown acceptance remain open. Estimate remains roughly 95%, low confidence,
not measured coverage. Frozen formal evidence, hashes and censuses are unchanged.
Cancellation checkpoint: 826 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Thirteen tests were added;
frozen formal proof scope is unchanged.

Funded mediated current-source contexts (§12.8.4) now acquire actual current source/
revision under complete document/context/policy binding and conservative native
parent/pool envelopes. Request/copy/reply workspace is independently funded at zero
context capacity; context/hold lifetime issuance is shared and nonrefilling.
Current semantic-root, scope/capture/request/selector bounds precede copy/issuance.
Queries bind actual original selectors/scope and valid source shape before accepted
loss. Fresh whole-envelope facts control delivery; unknown/transient refusals do
not establish loss, while actual owner disposal/fault and bound read/policy loss
close delivery without resurrection. Publication alone preserves old captures.
Source range/line materialization is shared with trusted queries and preserves
mixed delimiters and literal U+FEFF. Actual hold-ledger/count checks protect stopped
snapshots and parent funding; only empty-pool retirement refunds once, without
reopening issuance. Protected cleanup neither heals faults nor recalls disclosed
copies. Eight BDD/five native tests exercise these paths, exact envelopes, shared
quotas, faults and bounded optional content-free signals. Accounting is native,
not JSON/physical equivalence; host policy remains a premise. Annotation/combined,
retained/version-set/F05 child/discovery/public products and independent refinement
remain open. Estimate stays roughly 95%, low confidence; frozen evidence unchanged.
Current-source context checkpoint: 839 default workspace tests pass in debug/
release, with formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference integration tests in each profile. Thirteen tests were
added. Initial debug/doc and debug Unicode attempts were unqualified by host disk
exhaustion (archive ENOSPC/staging I/O refusal); after scoped Cargo cleanup of
regenerable engine development artifacts, both rebuilt gates passed. No source,
pinned test input or frozen formal archive was removed.

Retained-source acquisition (§12.8.6) now resolves an original available context
through the same native acquisition kernel. Exact scope/profile and complete
extra-selector request checks precede accepted loss, which closes the original
even when new occupancy/issuance is exhausted. Unknown/transient refusal does not.
Copies use stored source/revision, not current bytes or a newer whole root, and
spend one context/issuance unit without holding their original. Independent
stop/retirement and stale-edit refusal remain intact. Seven BDD/three native tests
cover old/new cuts, all loss/exhaustion classes, scope/policy/foreign guards,
ordinary refusal, disposal/fault, held stopped data, shared copy-chain issuance,
exact request bytes and newer-root growth. Bounded optional retained-capture/loss
signals preserve privacy and outcomes. Broader historical/version-set/annotation
products, managed search/consumer revalidation, discovery/public products and
independent refinement remain open. Estimate stays roughly 95%, low confidence;
frozen formal evidence/hashes/censuses remain unchanged. Checkpoint: 849 default
workspace tests pass in debug/release, plus formatting, strict all-feature Clippy,
documentation tests and both pinned Unicode integration tests in each profile.
Ten tests were added; this does not enlarge frozen formal proof claims.

Managed source-only literal matches now have an actual parent-funded child result
pool (§15.4.2, partial F05), original-context holds before shared-kernel evaluation,
fresh separately attributed source/query release facts and protected one-time hold
discharge. Full request/scope/shape checks precede loss; query loss does not revoke
independent source reading. Original revisions survive publication without granting
stale edits. Result/pool retirement cannot refill either issuer; active child pools
block source-pool retirement, which preserves closed child issuance. Seven native
and five BDD tests cover exact native envelopes/funding, zero-result workspace,
loss/temporary matrices, original/foreign binding, literal/Unicode/partial results,
hold corruption and fault cleanup, with optional bounded content-free signals.
Managed next/previous navigation, F11/F12 consumer preparation/service validation,
annotation/combined/version-set issuers, public products and independent refinement
remain open. Estimate stays roughly 95%, low confidence; frozen evidence unchanged.

Managed literal-result ownership checkpoint: 861 default workspace tests pass in
debug/release, with formatting, strict all-feature Clippy, documentation tests and
both pinned Unicode reference integration tests in each profile. Twelve tests
were added; this does not enlarge frozen formal proof claims.

Managed next/previous navigation now uses the actual retained source-search owner,
shared trusted ordering/wrap kernel and common fresh source/query/hold release
checks. Complete request/scalar/current-occurrence checks precede loss; incomplete
result and complete reply refusals cannot hide recognized loss. Valid empty results
remain distinct from unavailable; cross-result or retired occurrences cannot bind
an identical peer. No new holds, issuance, source generation or history are created.
Five external and two native tests cover these rules, exact profile intersections,
finite directional-walk oracle and bounded optional redacted diagnostics. F11/F12
preparation/service consumer validation, other issuers and public products remain
open. Estimate remains roughly 95%, low confidence; frozen formal evidence unchanged.

Managed navigation checkpoint: 868 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Seven tests were added;
these finite native checks do not enlarge frozen formal proof claims.

Native source-only managed F11/F12 replace-all/current and match-selection consumers
now freeze original result/scope/query/command into the shared invocation ledger,
without an independent read lease or extra source issuance. Existing native source
and selection publishers retain intrinsic revision/policy/view/companion checks;
fresh independently attributed mutation authorization is additional to source/query
eligibility. Generic execution refuses absent fresh facts, replay does not refresh,
and loss/retirement settles not-applied without cancellation or charge refund.
Checked partial-prefix occurrences support current replacement, not exact navigation
or incomplete replace-all. Context-bound stop remains intent until actual service.
Twelve BDD/three native tests cover these paths, exact capture limits, mismatch/
retirement/stale/no-op semantics, optional bounded redacted observations, private
candidate drift and post-install Unknown. Other issuers, public products, host policy
truth and independent refinement remain open; frozen formal evidence is unchanged.
Overall estimate stays roughly 95%, low confidence, not measured coverage.

Managed consumer checkpoint: 883 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Fifteen tests were added;
these native checks do not enlarge frozen formal proof claims.

Native annotation/combined preparation (§12.8.5, not installed contexts) now has a
separate actual-parent reservation, bounded current-root facet capture and opaque
spent cut identity. Complete actual visibility rows choose facets rather than data;
hidden/unrequested values are not retained, and annotation-only cuts retain neither
source nor source-product profile. Combined capture is coherent. Full identity/scope
preflight precedes nonconsuming current whole-facet handoff validation; accepted
loss, actual retained disposal or fault invalidates, while unknown/temporary refusal
does not. Protected cleanup cannot refill issuance or heal faults; empty-pool refund
is one-time. Six BDD/five native tests cover exclusions, exact native/local/parent
bounds, original values, spoofing/loss, private location-integrity faults and
optional bounded content-free signals.
Installed funding/one-time transfer, F03 selectors/queries, combined F02/F05 and
retained/version-set/public products remain open. Frozen evidence unchanged;
overall estimate remains roughly 95%, low confidence, not measured coverage.

Annotation-preparation checkpoint: 894 default workspace tests pass in debug/
release, with formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference integration tests in each profile. Eleven tests were
added; these native checks do not enlarge frozen formal proof claims.

Installed annotation/combined ownership and combined F02 range/line reads now use
an independently parent-funded immutable owner. Full cut/scope/request checks and
fresh whole-projection authority precede bounded, one-time, nonallocating ownership
transfer. Refusal does not consume; precommit interruption/private registry drift
fences without a partial context. Preparation retirement does not erase installed
data or refund its obligations. Original source/version and both-class current
rights govern combined source reads; shape/input preflight precedes accepted loss,
while delivery scan/reply ceilings cannot hide loss. Annotation-only contexts do not
gain source. Protected stop/retirement and independent one-time pool refund retain
spent issuance. Six BDD/four native tests cover these paths, exact funding/envelopes,
faults and bounded optional redacted signals. F03 selectors/pages/queries, combined
F05 holds/consumers, retained/version-set/public products and independent refinement
remain open. Frozen evidence unchanged; estimate remains roughly 95%, low confidence.

Installed-ownership checkpoint: 904 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Ten tests were added; these
native checks do not enlarge frozen formal proof claims.

Native current annotation/combined F03 enumeration, inspection and location now
resolve original context-bound opaque record/cursor selectors. Typed scope/request/
kind/context/cursor shape precedes loss; fresh whole-projection facts govern release.
Original facet values never widen or refresh from the root. The installed profile
funds selector/staging envelopes, shares spent context/selector issuance and bounds
whole rows/pages before registration. All candidates and owned replies finish before
nonallocating handoff; retries reuse IDs, and a later oversized row/refusal/private
precommit fault registers no prefix. Empty pages need no selector; finite positive
limits advance cursors. Protected context retirement removes dependent selectors,
not spent issuance/funding or disclosed copies. Eight BDD/five native tests cover
these rules, exact bounds/facts, identity guards, faults and optional redacted signals.
Combined F05, retained/version-set/public products and independent refinement remain
open. Frozen evidence unchanged; estimate remains roughly 95%, low confidence.

F03 current-reader checkpoint: 917 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Thirteen tests were added;
these native checks do not enlarge frozen formal proof claims.

Native shared F05 now admits source-only and installed combined contexts through
a closed first-party issuer/facts sum. One matcher/navigation/consumer path retains
the exact original issuer and actual hold. The independently funded search owner
owns its domain labels and survives source bootstrap retirement. Combined hold
slots are funded/preallocated at installed configuration; live holds are capped
and share nonrefilling issuance with contexts/F03 selectors. Both admitted classes
and the whole projection are rechecked at release, navigation, occurrence lookup,
and F11/F12 preparation/service. Wrong-issuer and annotation-only packets refuse
before loss; held stopped contexts cannot retire. Protected result retirement
discharges its original hold once without a new content grant or issuer refund.
Seven BDD/two native tests cover this composition, coexistence, direct combined
bootstrap, actual replacement/selection publishers, loss/identity guards, bounds,
faulted protected cleanup, corrupt ledgers and bounded redacted instrumentation.
These are scoped native implementation checks, not independent/universal proof.
Retained annotation/coherent-version-set/public products and independent refinement
remain open. Frozen formal inputs/hashes/censuses are unchanged. Overall estimate
remains roughly 95%, low confidence, not measured coverage.

Shared-search checkpoint: 926 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Nine tests were added;
no frozen formal artifacts, hashes or censuses were regenerated.

Native §12.8.7 retained annotation/combined acquisition now copies only an original
available installed projection. Explicit admitted classes allow non-widening
source-only/annotation-only/combined copies; excluded values/profile and query
projections are absent. Complete original scope/identity/subset/request preflight
precedes accepted whole-original loss, which precedes new capacity/preparation
refusal. Stage owned value/scope/reply before one nonallocating context installation.
Copies have independent stop state, fresh F03 selectors and no hold on the original;
spent issuance/funding is not refunded. Search consumes the old cut, while stale
mutation still refuses through the actual publisher. Six BDD/four native tests
exercise narrowing/isolation, original retirement, selectors, fact matrices,
independent preparation, request boundaries, fault/staging behavior, old-cut search/
stale replacement and bounded optional content-free instrumentation. The first
BDD scenario failed on missing APIs; other checks cover composition, not ten
independent red-green cycles. Coherent version sets, arbitrary-history/public
products, actual host policy and independent/universal refinement remain open.
Frozen formal evidence is unchanged. Overall estimate remains roughly 95%, low
confidence, not measured catalog coverage.

Retained installed-context checkpoint: 936 default workspace tests pass in debug/
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference integration tests in each profile. Ten tests were added;
frozen formal artifacts, hashes and censuses were not regenerated.

Native coherent version-set preparation now stages ordered explicit unique members
from one serialized root using the shared projection builder. It issues no child
contexts, retains only admitted classes/facets, and registers no partial roster on
refusal. Fixed parent-root measurement workspace uses the actual immutable root
ceiling independently of a smaller local input limit. Private whole-cut handoff
validation binds the exact ID/scopes and checks all members before deciding loss;
later peer loss cannot hide behind earlier unknown/resource pressure. Discard and
pool retirement preserve nonrefilling issuance/actual reservation ownership.
Five BDD/five native tests cover original values/common-cut provenance after newer
publication, class isolation, later-member refusal, strict current facts and scope/
identity guards, exact request/context/local/parent funding, zero-capacity workspace,
staging faults, private provenance drift and bounded optional content-free signals.
The first BDD failed on missing APIs; other checks cover composition, not ten
independent red-green cycles. Parent/member installation, member queries, coherent
search/consumer holds, retained set copies and public products remain open. This
is preparation progress, not whole-version-set or independent proof closure.
Frozen formal artifacts/hashes/censuses are unchanged. Overall planning estimate
remains roughly 95%, low confidence, not measured coverage.

Coherent-preparation checkpoint: 946 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Ten tests were added;
frozen formal artifacts, hashes and censuses were not regenerated.

Native coherent installation now stages the whole ordered reply and alias batch
before transferring an original preparation once into a separately funded parent.
Current whole-preparation validation precedes new target capacity/refusal. Explicit
parent/member/selector/shared-issuance and value/scope/row/request/reply bounds apply;
private staging or capacity surprises fault without consuming the preparation or
publishing a prefix. Parent/member selectors cannot cross groups or become standalone
read leases. Installed validation reuses the all-member gate and actual liveness/
fault checks. Preparation-pool retirement leaves installed values/funding intact.
Protected parent/member stop closes the whole set; complete controls govern removal
of all aliases without refunding spent issuance or peer obligations. Five BDD/five
native tests cover one-time transfer/order, current fact matrices, capacity/loss
ordering, exact native frame/funding bounds, faults, private alias drift, original
values/common-cut provenance, actual peer disposal/fault, protected cleanup and
optional bounded content-free signals. First BDD failed on missing APIs; other
tests cover composition, not ten independent red-green cycles. Member queries,
actual parent F05 holds/consumers, retained-set copies and public products remain
open. Frozen formal artifacts/hashes/censuses are unchanged; these native checks
are not whole-family or independent/universal refinement. Overall estimate stays
roughly 95%, low confidence, not measured coverage.

Coherent-installed-owner checkpoint: 956 default workspace tests pass in debug/
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference integration tests in each profile. Ten tests were added;
frozen formal artifacts, hashes and censuses were not regenerated.

Native version-set member source ranges and lines now resolve original member/
parent/scope identities, reject out-of-class/malformed packets before loss, and
revalidate all required peers before disclosure. The existing source-context shape
resolver and canonical reply constructors supply old UTF-8/line values after newer
publication or preparation-pool retirement. Complete inner source and outer native
query/reply bounds are independent; accepted loss precedes tighter delivery scan/
reply refusal. Installed facts account their actual context-selector width. Reads
issue no selector/hold, spend no issuance and create no authored publication.
Five BDD/four native tests cover original values, Unicode/newline fidelity, class/
identity/scalar/shape guards, fresh false/unknown/pressure facts, exact inner/outer
boundaries, actual peer disposal/fault at first handoff, protected cleanup and
optional bounded redacted instrumentation. First BDD failed on missing API; other
checks cover composition, not nine separate red-green cycles. Two of five member
query forms are native; the three F03 forms, parent F05 holds/consumers, retained-set
copies and public products remain open. Frozen formal evidence is unchanged;
native tests do not establish independent/universal refinement. Overall planning
estimate remains roughly 95%, low confidence, not measured coverage.

Coherent-source-read checkpoint: 965 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Nine tests were added;
frozen formal artifacts, hashes and censuses were not regenerated.

Native coherent F03 enumeration, inspection and target-location now reuse the
standalone reader's pure paging/facet kernel, with distinct member-bound opaque
record/cursor types and owner-specific all-peer/current/native-frame checks.
Installed funding covers lifetime descriptors and whole-page staging. Contexts,
member aliases and query descriptors share spent issuance/live-selector caps.
Complete shape precedes loss; whole-parent current validation precedes delivery/
new-selector pressure. All keys and owned reply values finish before nonallocating
registration. Later-row refusal and private staging/capacity faults register no
prefix. Equivalent full-budget retries reuse IDs; empty pages need none. Parent
retirement removes query lookups without refunding spent IDs or disclosed copies.
Seven BDD/five native tests cover all three forms, original facets, finite paging,
identity/cursor/kind/class guards, exact native bounds, later-row atomic refusal,
fresh peer facts, actual peer disposal/fault, protected cleanup, injected private
faults and optional bounded content-free signals. First BDD failed on missing APIs;
other checks cover composition, not twelve independent red-green cycles. Two
native fixtures initially exceeded the existing body-creation cap before reaching
the reader; corrected bodies still exceed the tested read-row cap. All five
member-query forms are native; parent F05 holds/consumers, retained-set copies and
public products remain open. Frozen formal evidence is unchanged; these checks
are not independent/universal refinement. Overall estimate stays roughly 95%, low
confidence, not measured coverage.

Coherent-member-query checkpoint: 977 default workspace tests pass in debug/
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference integration tests in each profile. Twelve tests were
added; frozen formal artifacts, hashes and censuses were not regenerated.

Native coherent-member F05 now joins the existing closed issuer sum, shared matcher/
navigation/occurrence path and original F11/F12 publishers. Original member/parent
and stable selected scope are distinct from complete fresh current-peer facts.
Preflight bounds full host frames and inner source requests before loss; every
release/use rechecks all required peers and actual hold ownership. The installed
pool funds lifetime whole-parent hold slots, separately caps live holds and shares
nonrefilling issuance with context/member/query selectors. Held parents cannot
retire. Complete issuer-specific protected control frames discharge one original
published result hold, while private unpublished cleanup adds no public result.
Immutable bounded peer-scope attribution survives parent/result retirement without
creating an independent lease; malformed packets cannot consume pending work.
Eight BDD/four native tests cover all three F05 forms, actual replacement/current/
selection publishers, peer loss after preparation, malformed attribution and known
not-applied retirement, shared caps/spent issuance, protected denials, independent
source/set results and source bootstrap retirement, class/parent guards, exact three-
coordinate full-frame boundaries, private ledger drift/unpublished cleanup, actual
disposal/fault and bounded optional redacted signals. First BDD failed on missing
APIs; other checks cover composition, not twelve independent red-green cycles.
Retained-set copies and full public products remain open. Frozen formal artifacts/
hashes/censuses are unchanged; tests are scoped native coverage, not host-truth,
independent/universal refinement. Overall estimate stays roughly 95%, low confidence.

Coherent-search checkpoint: 989 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference integration tests in each profile. Twelve tests were added;
frozen formal artifacts, hashes and censuses were not regenerated.

Native §12.8.10 retained sets now independently copy ordered original member/class
subsets from an available parent. Identity/scope/selection/request preflight rejects
unknown members, duplicates and widening before loss; whole-original currentness
precedes new capacity/preparation refusal, including omitted peers. Shared narrowed
projection and current-install admission/alias/reply staging preserve old values,
facets and common-cut provenance without current-root/provider fallback. Complete
staging precedes one registry publication; failure spends/installs nothing. Copies
borrow no original hold/stop state; aliases/selectors are fresh and excluded data
cannot be restored by a copy chain. Existing query/search/consumer paths apply;
old reads do not authorize stale replacement. Six BDD/four native tests cover order,
class/data exclusion, original retirement, omitted-peer/unknown facts, malformed
scope/member/widening/preparation gates, fresh selectors, old-cut search/stale edit,
four-coordinate exact admission, independent held-parent/cut ownership, private
staging/capacity faults, actual disposal/fault and bounded optional redacted signals.
First BDD failed on the missing API; other checks cover composition, not ten separate
red-green cycles. Full public read/result products and other roadmap families remain
open. Frozen formal artifacts/hashes/censuses are unchanged; this is scoped native
implementation coverage, not host truth or independent/universal refinement.

The inherited approximate 95% aggregate is withdrawn as an engine-completion
measure: the roadmap still has whole unfinished facade/control families and no
complete command-level acceptance denominator. Completed slice percentages and
specific remaining behavior take precedence; whole-engine completion is unmeasured.

Retained-set checkpoint: 999 default workspace tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference integration tests in each profile. Ten tests were added; frozen formal
artifacts, hashes and censuses were not regenerated. Next native work is the closed
read-result contract (§§22.1.14–22.1.17), not UI or transport implementation.

The native source-producer portion of §22.1.14 now guards final range/line products
from source-only, combined and coherent-member owners. One validator compares
constructed kind/stamp/geometry/length/delimiter and final alias to independent
captured expectations before release. It copies no payload and adds no lease.
Malformed construction returns unavailable and latches owner fault plus a bounded
trusted-owner NATIVE_SOURCE_RESULT_INVARIANT cause, independently of capture.
One BDD/four native tests cover old references after publication, 36 actual
postconstruction fault injections over all three owners, unchanged root/registry/
issuance/funding frames, fault persistence/protected cleanup and disabled/on/full
redacted diagnostics. A control intentionally accepts same-length altered payload
as structurally valid: actual byte/line correctness remains an independent oracle
obligation, not a claim of this guard. First BDD failed on missing API; remaining
checks cover composition, not five separate red-green cycles. Test-only compile/
lint issues were fixed before qualification; they are not semantic mutation kills.
Decoded facade and late-fault frames, public
metadata/discovery/codecs and operational/independent gates remain open. Frozen
formal artifacts/hashes/censuses are unchanged. Slice scope is source producers,
not the complete closed-read-result family or whole-engine completion.

Source-producer-result checkpoint: 1,004 default workspace tests pass in debug and
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference tests in each profile. Five tests were added. This native
source-producer slice is complete (100%); whole-engine completion remains
unmeasured rather than inferred from test totals.

Native F03 producer-result guards (§22.1.15) now validate final page/record/location
replies in both installed annotation/combined and coherent-member owners before
selector commit. A shared borrowed validator binds final aliases to actual staged/
existing selectors and exact requested slots, checks page cardinality/fullness,
continuation and alias uniqueness, and preserves independently optional facets.
Five native tests exercise 32 actual postconstruction corruptions with unchanged
root/registry/issuance/funding, sticky first-cause owner fault, protected cleanup,
optional/full bounded capture, absent facets and body-oracle/geometry controls.
These are composition/fault-injection tests, not five red-green cycles or universal
refinement. No body copy or lease is added; validation's auxiliary live state is
constant. Native prepublication faults cannot be conflated with late facade faults,
which cannot undo already installed selector sets. Acquisition-facade/decoded/
metadata/discovery/public-codec and independent operational gates remain open;
frozen formal artifacts, hashes and censuses remain unchanged.

Native F03 producer-result checkpoint: 1,009 default workspace tests pass in debug
and release, plus formatting, strict all-feature Clippy, documentation tests and
both pinned Unicode reference tests in each profile. Five native tests were added;
this native prepublication slice is complete (100%). Whole-engine completion stays
unmeasured, and the full implementation goal remains active.

Native acquisition producer guards (§22.1.16) cover current/retained paths for
source-only, annotation/combined and coherent owners before installation/issuance
or cut consumption. Final identities/classes/ordered documents and staged alias
bindings are checked against actual candidates; retained copies additionally
preserve requested meaning and distinct original parents. Ten native tests cover
23 malformed constructions, unchanged root/native/prepared/funding/issuance,
protected cleanup and 36 capture-off/on/full accepted/malformed combinations.
One source test first failed before refusal wiring; other checks are composition,
not ten independent red-green cycles. Test fixture/lint corrections are not mutation
kills. Validation adds no payload/participant set or lease, and diagnostics cannot
decide health. Existing source acquisition stays an opaque ID/Option API, without
inventing the future facade status/data wrapper. Native role separation is typed,
not numeric slot inequality. Decoded/facade/late-fault/refund frames, discovery,
complete public products and operational/independent gates remain open. Frozen
formal artifacts/hashes/censuses remain unchanged.

Native acquisition producer-result checkpoint: 1,019 default workspace tests pass
in debug and release, plus formatting, strict all-feature Clippy, documentation
tests and both pinned Unicode reference tests in each profile. Ten native tests
were added. This native preinstallation slice is complete (100%); the full engine
implementation goal remains active, with overall completion unmeasured.

Native inert-data capture now supplies the nonrecursive plain-value copying
prerequisite for decoded facade work. Nine public BDD scenarios and two unit tests
cover owned UTF-8/scalars/signed zero, explicit byte/node/depth ceilings, cycles,
duplicate fields, nonfinite refusal, independent shared-value copies, optional/full
redacted capture and arithmetic/capacity refusal. One initial scenario failed on
missing API; remaining checks are composition, not eleven red-green cycles or an
actual allocator-OOM experiment. A flat arena avoids recursive output cleanup.
Native logical output/live-frame units are not frozen-model JSON-byte equivalence,
physical/global bounds or invocation-parent reservation. Capability binding, full
typed/dynamic normalization, current facts, parent workspace/busy/protected refund
and native/facade result composition remain open. This is not decoded-facade closure.
No frozen formal artifacts/hashes/censuses were regenerated.

Native inert-capture checkpoint: 1,030 default workspace tests pass in debug and
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference tests in each profile. Eleven tests were added. Copier
prerequisite scope is complete (100%), not the decoded facade; the full engine
goal remains active and whole-engine completion is unmeasured.

The native fixed-port source-read adapter now composes capture, actual coordinator
retention headroom and native source/combined/coherent-member range/line queries.
Two public BDD scenarios and thirteen native tests cover capture/facts/bounds/loss/
reentry and original-root ownership, including 36 selected fault frames through
protected finalization. One finalizer regression failed before the post-cleanup
release check; composition checks are not fifteen independent red-green cycles.
Classification plus seven native-byte frames reserve/refund only their own actual
parent charges. Native contexts/issuance remain independent; unexplained debt is
fenced, not erased. Final release signals follow refund, without payload labels.
This native typed seam does not close exact decoded envelopes, public document/
profile projections, JSON-safe result shadows, discovery/selector/event codecs,
hostile ingress, physical/global accounting or host panic-hook/abort/crash and
independent refinement. Frozen formal artifacts/hashes/censuses remain unchanged.

Native source-adapter checkpoint: 1,045 default workspace tests pass in debug and
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode reference tests in each profile. Fifteen tests were added. Native
typed adapter scope is complete (100%); full decoded-facade and engine completion
remain unproven, and the implementation goal stays active.

The native annotation facade adapter now composes bounded copies, the shared
source/annotation read guard and actual parent temporary reservations with both
single-context and coherent-member F03 ports. Two public BDD scenarios and twelve
native tests cover annotation-only/retained ownership, fresh selector round trips,
independent facets, full peer loss/bounds/reentry, nonpromotion, structural identity
controls, separate event populations and native versus facade fault causes.
Thirty-six selected faults distinguish pre-handoff unchanged-registry frames from
late complete-installed-selector/nonrefunding-issuance frames. Protected refund
touches only temporary reservations; native retirement remains separate. Checks
are composition/fault injection, not fourteen red-green cycles or universal
refinement. Exact decoded envelopes, JSON-safe shadows, public profile/document/
selector/event codecs, discovery and operational/independent gates remain open;
frozen formal artifacts, hashes and censuses were not regenerated.

Native annotation-adapter checkpoint: 1,059 default workspace tests pass in debug
and release, plus formatting, strict all-feature Clippy, documentation tests and
both pinned Unicode reference tests in each profile. Fourteen tests were added.
Selected native typed adapter scope is complete (100%), not the full decoded
facade or engine; the implementation goal remains active.

The native read-acquisition facade adapter now covers six current/retained source,
annotation/combined/coherent ports with the shared guard and an eight-frame family.
Read-only private cut matchers bind full scope/class/ordered roster before native
validation can apply loss or consume a preparation. Two public BDD scenarios and
eleven native tests cover selected-root fidelity, independent retained ownership,
matching/nonpoisoning, full facts, actual-parent/copy limits, reentry, fault causes
and optional bounded instrumentation. Thirty-six selected faults preserve the
pre-/post-install distinction: late contexts, issuance and consumed cuts remain
native obligations. A binding-guard removal was assertion-detected and restored;
other checks are composition, not thirteen red-green cycles. Source-current's
opaque-only bad-product probe is an unwind surprise, not a JSON-shape mutation.
No public preparation/cleanup verb or wire codec is invented. Complete decoded
profiles/document projections, JSON-safe shadows, owner shutdown/undelivered-result
recovery and operational/independent gates remain open. Frozen formal artifacts,
hashes and censuses remain unchanged.

Native acquisition-adapter checkpoint: 1,072 default workspace tests pass in debug
and release, plus formatting, strict all-feature Clippy, documentation tests and
both pinned Unicode reference tests in each profile. Thirteen tests were added.
Selected six-port native adapter scope is complete (100%); the full decoded facade,
owner recovery/shutdown and engine goal remain open and active.

The scoped history-basis prerequisite is now native: selected actual group identity
and validity, complete source/annotation participants, metadata presence, both
ordered stacks and lost-floor flags. Thirteen native tests and two public BDD
scenarios cover hidden-participant staleness versus unrelated work, set order,
limits, corruption, optional observations and independence from reversal permission
and inverse pins. One hidden-participant comparison mutant was assertion-detected
and restored. Caller-owned native accounting is not pooled F04 funding or JSON-byte
equivalence. Endpoint equality does not detect unobserved ABA; absorbing retained
context invalidation, recipient projection/current grants and delivery remain open.
Checkpoint: 1,087 default tests pass in debug/release, plus formatting, strict
all-feature Clippy, documentation tests and two pinned Unicode reference tests in
each profile. Fifteen tests were added.
Selected metadata-basis prerequisite: 100%; full engine goal remains active and
unmeasured. Frozen formal artifacts, hashes and censuses are unchanged.

The native owned-history slice now adds a parent-funded registry, exact scope,
copied unique aliases, complete private basis, monotone identities, absorbing
invalidation, protected stop/retirement and closed owner tombstones. Native F04
frontier/availability/permitted detail and current private F14 alias bridges enforce
complete actual blocking without leaking hidden aliases. Advice remains independent
of actual reversal permission, inverse pins and admitted plan lifetime. Twenty-three
native tests and two public BDD scenarios cover gates, privacy, bounds, retention,
ordering and ownership. An exact endpoint-return test killed mismatch-invalidation
removal; a real result-guard failure corrected structural `Arc<()>` equality to
allocation identity. Successful products use retained operation/context/alias/
order/count/byte validation; twelve selected malformed-result injections preserve
retained obligations under a sticky native fault fence and protected cleanup.
Optional bounded events/gauges distinguish advice, refusal, fault and discharge.
Exact decoded/public history profiles, discovery/identity/control codecs and
host/physical/global/crash/concurrent/independent gates remain open. Frozen formal
artifacts, hashes and censuses are unchanged; the full engine goal remains active.

Native history-owner checkpoint: 1,112 default tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference tests in each profile. Twenty-five tests were added. Selected native
owner/lifecycle/advice/result-guard slice is complete (100%); decoded/public F04 and
whole-engine completion are not claimed.

The fixed native history facade now composes all three query meanings with the
shared read busy/finalizer guard. Classification plus seven caller-byte frames must
fit actual history-owner and parent headroom; neither settings intent nor history
capture limits silently replaces the request ceiling. Independently bounded exact
actor/scope/query/facts copies preserve opaque original handles, unknown/false
meaning and native absorbing loss. Fourteen native tests and two public BDD
scenarios cover current advice, scoped staleness, nonpoisoning, copy/owner/parent
bounds, reentry, optional diagnostics and protected cleanup. Eighteen selected
stage/operation faults and three stale-context fault probes preserve retained
entries/issuance/base reservation and any already-observed invalidation. Removing
actor binding was assertion-detected when foreign false facts revoked a valid
entry; the binding was restored before qualification. Facade result invariants do
not replace native diagnosis or establish history/evidence truth. Exact decoded
envelopes, JSON-safe shadows, public history codecs/discovery and operational/
independent gates remain. Frozen formal artifacts/hashes/censuses are unchanged;
the full engine goal stays active and completion remains unmeasured.

Native history-adapter checkpoint: 1,128 default tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference tests in each profile. Sixteen tests were added. The selected three-port
native adapter is complete (100%); exact decoded/public history and whole-engine
completion are not claimed.

Native measurement values now implement the adopted 32 descriptors and finite
dimension dictionaries, typed tagged units, absent-versus-zero state, bounded
ordinals/counters, sticky inexact saturation, conditional exact deltas and pure
same-domain/epoch clock conversion. No actual clock or effect-attempt truth follows.
Four opt-in Recorder series instrument actual diagnostic offers, captures, drains
and drops with separately bounded native state/snapshot workspace and no record-
time allocation/callback/clock. Fifteen native tests and three public scenarios
cover catalog conformance, value laws, bounds, generations, saturation, atomic
snapshot refusal and unchanged real edits with metrics off/available/saturated.
Misclassifying dropped as captured was assertion-detected and restored. Other
probe populations, full provenance/collection/recipient release/retention ownership,
physical/clock/workload and independent gates remain open. No boundary-marker/1
widening, exporter, UI metric, bucket or numeric default is adopted. Frozen formal
artifacts/hashes/censuses remain unchanged; full engine completion is unmeasured.

Native measurement checkpoint: 1,146 default tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference tests in each profile. Eighteen tests were added. Selected value-kernel/
four-stage diagnostic slice is complete (100%); full instrumentation/provenance
and whole-engine completion are not claimed.

The actual shared source/annotation transaction-publication kernel now owns a
separately parent-funded disposition pool. Applied/no-op/refusal/fault counts derive
from one real final-phase result or unwind, not participant/event volume. Replay,
preparation refusal and stopped pending jobs do not inflate this population. Thirteen
native tests and three public BDD scenarios cover mixed/derived/reversal routes,
disabled/saturated events, native owner isolation, pool/profile/parent bounds,
counter/ordinal exhaustion and protected nonrearming teardown. Returned and unwind
faults preserve the engine fence; a partial-assignment probe expressly rejects any
rollback inference. No-op-as-success was assertion-detected and restored. Probe
updates allocate nothing and use no clock/callback/mutex/retry loop. Other publisher
populations, general provenance/current recipient projection, actual durations,
physical/overhead/workload and independent gates remain open. Frozen formal
artifacts/hashes/censuses are unchanged; full engine completion is unmeasured.

Native transaction-measurement checkpoint: 1,162 default tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference tests in each profile. Sixteen tests were added. Selected native
transaction-kernel disposition slice is complete (100%); other populations,
protected recipient measurement assembly and whole-engine completion remain open.

Native invocation measurement now uses shared installers at all 24 current native
first-binding and sixteen known terminal-install sites. Actual cohorts distinguish
issuance, first admission, replay, no-op, success, refusal and stop; late activation
excludes already-admitted work. Twelve native tests and three public BDD scenarios
cover active source/settings/relation paths, replay/forget/retirement, limits,
parent/root accounting, saturation, ordinal exhaustion and independent observer
teardown. A cohort-bypass mutation was assertion-detected and restored. Bounded
fault census relies on actual retained nonretirable/nonreexecuting consumed slots;
future uncertain-job reclamation must preserve/transfer that history. Returned,
unwind/partial-install and nonconsuming control-fault tests keep original-job
terminal facts separate and never invent nonpublication or quiescence. No clocks,
callbacks or allocation enter the transition updates. Full provenance/current
recipient projection, ingress offers, durations, other populations, physical/
overhead/workload and independent gates remain open. Frozen formal evidence is
unchanged; full engine completion remains unmeasured.

Native invocation-measurement checkpoint: 1,177 default tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference tests in each profile. Fifteen tests were added. Selected native
ledger-cohort/disposition slice is complete (100%); other populations, protected
recipient measurement assembly and whole-engine completion remain open.

Passive native F04 arguments/dispatch now preserves exact typed/dynamic tuple,
contract/variant/field/undo-redo semantics and the original borrowed opaque context.
Owned ordinary data outlives the packet. Independent whole/actor/canonical native
capture plus tighter caller limits compose with one actual classification/seven-
frame gateway reservation and finalizer. Ten native tests and two public BDD
scenarios cover equality, ownership, malformed/unsupported inputs, exact boundaries,
current loss, native empty/hidden blocking/permitted detail, reentry, optional
diagnostics and eighteen selected faults. Profile-gate removal was assertion-
detected when an unsupported request poisoned a valid context; it was restored.
No arbitrary JS object/reflection/hostile parser, JSON/wire handle/result codec,
public acquisition/control/discovery, physical/global/crash/concurrent or independent
closure is claimed. Frozen formal evidence is unchanged; full engine completion
remains unmeasured.

Native decoded-history checkpoint: 1,189 default tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference tests in each profile. Twelve tests were added. Selected passive-native
F04 arguments/dispatch slice is complete (100%); wire/result/discovery products,
other facade families and whole-engine completion remain open.

Native source argument binding adds actual typed family 1/dynamic `read-document`
dispatch over the existing two F02 variants and three native context roles.
Original snapshot/document capabilities stay borrowed; canonical ordinary values
are owned, bounded and route-independent. Native document binding deliberately
does not implement the reference serialized identity triple; codec/refinement and
full public products remain open. Exact vocabulary/numeric checks, separate actor
attribution, pre-loss document/profile binding, complete peer facts and shared
result/cleanup guards are preserved. Classification and seven effective-byte frames
use the original actual parent ledger, with early validation-shadow discharge and
no nested admission. Thirteen internal and two public BDD tests exercise the native
slice, including eighteen fault combinations and an assertion-detected profile-gate
mutation (restored). Frozen formal evidence is unchanged.

Native source-binding checkpoint: 1,204 default tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference tests in each profile. Fifteen tests were added. Selected native F02
argument-binding/dispatch slice is complete (100%); serialized document identity,
remaining facade families and whole-engine acceptance remain open. The goal stays
active; engine-wide completion remains unmeasured.

Native annotation argument binding adds actual typed family 2/dynamic
`read-annotations` dispatch over the three existing F03 operations for single and
coherent-member owners. Original snapshot/document/record/continuation roles remain
native, borrowed and distinct; ordinary actor/profile/limits are owned and bounded.
Native document binding does not close serialized identity or full wire/reference
decoder refinement. Exact forms, positive safe-integer limits, pre-loss scope/profile
binding, native selector authentication, complete peer facts, current facet grants
and original-request result checks compose through one existing seven-frame guard.
Early validation-shadow discharge admits normalized output and bounded fixed native
query lowering without another pool or reservation. Late faults retain published
selectors/issuance and protected cleanup. Fifteen internal tests and two public BDD
scenarios include normalizer/dispatch red-to-green acceptance, 36 injected faults,
optional facet withholding and an assertion-detected profile-gate mutation
(restored). Frozen formal evidence is unchanged.

Native annotation-binding checkpoint: 1,221 default tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference tests in each profile. Seventeen tests were added. Selected
native F03 argument-binding/dispatch slice is complete (100%); serialized identity,
remaining facade families and whole-engine acceptance remain open. The full goal
stays active; overall implementation coverage remains unmeasured.

Native acquisition argument binding now dispatches typed/dynamic snapshot-handoff
over all six existing current/retained ports. Native document/reference capabilities,
closed class enums and ordered typed participant rows are not wire codecs or full
reference-decoder refinement. Ordinary data and templates are bounded; target,
class, profile and original-reference correspondence precede native loss/transfer.
Actual prepared-cut matching, complete facts, once-only installation, independent
retained ownership, result guards and nonrefunding lifetime rules remain controlling.
Thirteen internal and two public BDD tests include red-to-green acceptance,
36 injected faults, original-capture/retirement behavior and an assertion-detected
profile-gate mutation (restored). One existing eight-frame guard/actual-parent
reservation is preserved through early actor-shadow discharge. Frozen formal
evidence is unchanged.

Native acquisition-binding checkpoint: 1,236 default tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference tests in each profile. Fifteen tests were added. Selected native
snapshot-handoff argument-binding/dispatch slice is complete (100%); wire class/
identity codecs, remaining facade families and full engine acceptance remain open.
The full goal stays active; overall implementation coverage remains unmeasured.

Managed-search product guards now cover candidate creation, retained literal
release, occurrence and navigation for source/combined/coherent-member issuers.
Borrowed bounded shape/identity checks add no result-sized validation allocation or
pool. They do not prove literal equality/completeness or rerun matching/navigation.
The shared sticky search-result invariant withholds malformed products and fences
the owner. Unpublished holds discharge without result installation or refunded
issuance; published results/holds remain until protected retirement. Eight internal
and two public BDD tests include 45 injected corruption/issuer combinations,
semantic release red-to-green, an assertion-detected release-guard bypass (restored),
empty/exhausted-prefix distinctions and bounded optional observations. This is a
native F05 facade prerequisite, not completed facade/argument/wire or independent
refinement. Frozen formal evidence is unchanged.

Native search-product checkpoint: 1,246 default tests pass in debug/release, plus
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference tests in each profile. Ten tests were added. Selected native managed-
search product-guard prerequisite is complete (100%); the F05 facade/arguments,
wire products and full engine acceptance remain open. The goal stays active;
overall implementation coverage remains unmeasured.

The fixed native managed-search adapter now wraps existing evaluation/release/
occurrence/navigation under one real eight-frame parent guard and shared read busy
fence. It independently copies command/current-query and scope/full peer facts,
preserves original issuer/result/occurrence identities, restores selected non-first
member scope, validates guarded native products and checks final health after exact
temporary refund. No requery/navigation rerun or result-sized validator is added.
Thirteen internal and two BDD tests include semantic acceptance red-to-green,
72 fault/port/issuer combinations, explicit late result/hold/issuance retention and
protected cleanup, non-poisoning, loss/stop, exact parent boundaries and optional
observations. Final-guard bypass was assertion-detected and restored. Test-only
privileged lookup is not public undelivered-result recovery. These native ports
are not new catalog verbs or complete decoded F05/profile/generation/wire/discovery
assembly. Frozen formal evidence is unchanged.

Fixed native search-adapter checkpoint: 1,261 default tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference tests in each profile. Fifteen tests were added. Selected fixed
native managed-search adapter slice is complete (100%); decoded F05/public assembly,
other facade families and whole-engine acceptance remain open. The full goal stays
active; overall implementation coverage remains unmeasured.

Native search outcome delivery now preserves the existing closed error vocabulary
through all four native ports via `try_search_facade_v1`; the optional API forwards
to it. Native invalid-input/limit errors no longer collapse to `Unavailable`, while
authority premises deliberately remain indistinguishable. Late faults retain native
obligations, faulted admission emits `Unknown`, and the existing protected refund
and invariant guards are unchanged. Five unit tests and one public BDD scenario,
plus strengthened exact-admission and 72-fault cases, pass. Acceptance failed against
a refusal-only scaffold, and an error-collapse mutation was detected and restored.
Checkpoint: 1,267 default tests pass in debug/release, formatting, strict Clippy,
documentation and two pinned Unicode reference tests in each profile. This native
outcome slice is complete (100%), not decoded F05/public assembly or full engine
acceptance. No formal artifacts or new transport/command meanings are introduced.

The eight native/decoded source, annotation, history and acquisition adapter paths
now expose explicit result-returning APIs, and join search under one private
admission/unwind/protected-refund/final-health executor. Native source/annotation
read and acquisition errors survive the handoff; history's semantic unavailable
does not become a guessed detailed failure. Compatibility optional APIs execute
the same route once. Nine unit tests plus one public BDD scenario check typed
refusal/retry, both decoded routes, all six acquisitions, native context ceilings,
original retained reads after mutation and independent retirement. Existing facade
fault, authority, disclosure and accounting tests continue to exercise that shared
gate. Deliberate native-error collapse and late-health-gate bypass were assertion-
detected and restored. No formal artifacts, pools, clocks, commands or transport
meanings are added; broader public assembly and the full engine goal remain open.

Read-outcome checkpoint: 1,277 default tests pass in debug/release, formatting,
strict all-feature Clippy, docs and both pinned Unicode reference tests in each
profile. This native slice is complete (100%). The user-requested overall estimate
is approximately 66% (+1 point from 65%, low confidence), not measured implementation
coverage; the full goal stays active.

Native `release.dispositions` now observes whole attempts at the shared read gate,
after protected cleanup and final-health classification, for the nine supported
native/decoded read adapter paths. The optional fixed parent-funded pool captures
a prospective cohort with closed content-free dimensions, saturating inexact
counters, atomic six-series snapshot preflight, absorbing stop and protected
one-time retirement. Metrics do not settle requests, grant reads, infer latency or
discharge native context/workspace debt. Direct trusted reads, other release
families, general metric-recipient delivery and clock/span/physical work remain open.
No frozen formal artifact, catalog verb, transport, UI or substrate is changed.
Ten unit tests and one public BDD scenario pass; eight existing facade tests also
assert actual totals. The no-op collector failed semantic acceptance and an injected
double count was detected and restored. Checkpoint: 1,288 default tests pass in
debug/release, formatting, strict all-feature Clippy, docs and both pinned Unicode
reference tests in each profile. This counter-cohort slice is complete (100%).
Overall working estimate: approximately 67% (+1 point from 66%, low confidence),
not measured implementation coverage. The full engine goal stays active.

Native coherent-context stop no longer reuses retirement's discharge predicate.
Original ordered scopes and exact-true owner controls remain required, while
drain/retention facts no longer prevent stopping. Retirement and whole-parent hold
release still require those facts, and retirement still waits for zero actual holds.
Five unit tests and one public BDD scenario cover parent/member targets, all 81
two-member drain/release vectors, full pressure/faulted owners, denied/foreign scopes,
live holds, independent copies and already delivered data. The first three tests
failed on the original code; a weakened-retirement mutation was detected and restored.
No protocol, pool or formal artifact changes. Checkpoint: 1,294 default tests pass
in debug/release, formatting, strict Clippy, docs and both pinned Unicode reference
tests in each profile. This native defect is closed; broad F19/control/lifecycle
closure remains open. Overall working estimate stays approximately 67% (unchanged
at whole-percentage resolution, low confidence), not measured coverage.

The selected native F19 stop composition now routes original typed source,
annotation, coherent parent/member and history controls through one closed facade.
It preserves actual native errors, works under ordinary pressure and existing
editing faults, and never enters the data-read gate or refunds/retire contexts.
Eight unit tests and one public BDD scenario cover local/foreign scopes, idempotence,
independent copies, protected cleanup, diagnostics and early/late faults without
rollback. Initial scaffold acceptance failed; a global-health admission mutation
was detected and restored. No new pool, command, transport or formal artifact is
introduced. Checkpoint: 1,303 default tests pass in debug/release, formatting,
strict Clippy, docs and both pinned Unicode reference tests in each profile.
This four-owner native interface is closed, not full F19 decoding/attachment/
stream/revocation or full engine acceptance. Overall working estimate: approximately
68% (+1 point from 67%, low confidence), not measured coverage; the full goal is active.

The selected native F19 stop path now has exact passive typed/dynamic arguments:
original family index 18 / `control-observation`, version 1, `stop-observation` and
`read-context/1`. Closed 2/4/3-field records canonicalize without allocation to an
original borrowed target; trusted lowering must bind that exact role/reference
before native control. Caller owner/scope fields, malformed tuples and arbitrary
nested data are rejected, not copied or traversed. This intrinsic native grammar
bound does not claim wire/ingress funding or a general-purpose protected decoder.
Seven unit tests and one public BDD scenario pass, including ordinary pressure,
faulted owners, original packet lifetime separation, all current target roles,
foreign history, capture modes and 160 malformed tuple/target/route combinations.
Initial scaffold acceptance failed; a target-binding bypass was detected and
restored. Checkpoint: 1,311 default tests pass in debug/release, formatting, strict
Clippy, docs and both pinned Unicode reference tests in each profile. No formal
artifacts, pools, verbs, transports or UI/substrate roles are added. This native
slice is complete, not full F19 or whole-engine acceptance. Overall working estimate:
approximately 69% (+1 point from 68%, low confidence); the full goal stays active.

Selected native synchronous service timing is now explicitly opt-in through the
caller's recorder, default off. The nine read adapter paths, native protected stop
and passive stop normalization attach local elapsed durations to existing boundary
markers. Normalization/control remain separate; missing, regressed or panicking
clock reads produce absence, not zero. Disabled or initially full capture skips
sampling. No coordinator pool, public clock callback, authority or retention claim
is added. Five new unit tests, one public BDD scenario and eight strengthened tests
cover the selected behavior; scaffold failure and a restored saturation mutation
establish focused regression sensitivity. Checkpoint: 1,317 default tests pass in
debug/release, formatting, strict Clippy, docs and both pinned Unicode reference
tests in each profile. Formal artifacts are unchanged; native tests do not establish
universal timing/platform correctness. Duration-series provenance/recipient delivery,
other pathways, async spans and overhead qualification remain open. Overall working
estimate: approximately 70% (+1 point from 69%, low confidence); the goal stays active.

All seven native invocation service entries now use that opt-in timing profile,
with distinct execution/replay markers and once-only compatibility delegation.
Returned service disposition is not the retained job outcome; unwinding creates
no completed sample and does not clear publication fences. Three new tests and
nine strengthened tests cover original binding, cohort/retention preservation,
both serialization forms, scoped mutations, search, faults and queue loss after
entry. The initial BDD test failed before implementation. Checkpoint: 1,320 default
tests pass debug/release, formatting, strict Clippy, docs and both pinned Unicode
reference tests in each profile. This slice introduces no formal-artifact or
contract changes. Overall estimate: approximately 71% (+1 point from 70%, low
confidence); remaining public contracts and broader instrumentation stay open.

Native §13.9.1 absent-annotation final retirement is now implemented, not just
modeled. Exact current gates/floors plus actual journal/capture discharge precede
payload movement into a prepaid identity/floor/restoration-key tombstone. No
lifetime reuse, source/set revision rewrite, automatic history eviction, public
cleanup verb or erasure of delivered copies is introduced. Authoring/restoration
and root accounting fund the new slots before issuance. Five public scenarios,
three unit tests and a strengthened actual-restoration test cover refusal,
invalidated history, clones, capacity, metadata preservation, faults and bounded
diagnostics; the initial positive scenario failed on the refusal-only scaffold.
Checkpoint: 1,328 default tests pass debug/release, formatting, strict Clippy, docs
and both pinned Unicode reference tests in each profile. Formal artifacts/hashes
are unchanged; whole-document reclamation and full engine acceptance remain open.
Overall engineering estimate: approximately 72% (+1 point from 71%, low confidence).

Source saved-state comparison now preflights encoded-output and checked combined
current/baseline size before serialization. An explicit native budget and the
compatibility resource-blob-derived caps share one producer; larger comparison
does not authorize larger saves. Six BDD scenarios and one arithmetic unit test
cover boundaries, encoding fidelity, exact equality, undo, ambiguity, no writer
issuance and diagnostics. The positive acceptance test failed on the refusal-only
scaffold. Checkpoint: 1,335 default tests pass debug/release, formatting, strict
Clippy, docs and both pinned Unicode reference tests in each profile. This closes
per-call native preflight, not scoped lineage/result assembly or global temporary
funding. Formal artifacts/hashes are unchanged. Overall estimate remains about
72% (low confidence; unchanged rounded estimate for this supporting slice).

Prepared source artifacts now implement the export codec prerequisite: exact
source/serialization capture, reserved shared byte/live/lifetime credit, single
retention without re-encoding and original-capability protected discard. Annotation
admissions preserve the credit and vector slot; stale/disposed/foreign/spent
handles refuse. Aggregate-charge drift fences without erasing unrelated debt.
Five BDD scenarios and three unit tests pass; initial acceptance failed on the
refusal-only scaffold. Checkpoint: 1,343 default tests pass debug/release, formatting,
strict Clippy, docs and both pinned Unicode reference tests in each profile.
Destination authority, writer-task admission, Save As activation and full public
composition remain open. Formal artifacts/hashes are unchanged. Overall estimate:
approximately 73% (+1 point from 72%, low confidence); the full goal stays active.

Native source Copy/Save As admission, shared dispatch and per-origin destination
reservations now compose the codec with the existing writer. Explicit present/
absent conditions use no fabricated absence token; independent host capabilities
default off. Current gates, structured pair evidence, root/artifact/task capacity,
original binding intent and immutable output are checked before issuance/launch.
Reservation release is not acknowledgment, quiescence or activation; root faults,
role drift, birth, retirement and disposed provenance cannot bypass ownership.
Twelve public BDD scenarios, three unit tests and one real macOS existing-file
copy test cover this slice; initial acceptance failed on the refusal-only scaffold.
Checkpoint: 1,359 default tests pass debug/release, formatting, strict Clippy, docs
and both pinned Unicode reference tests in each profile. Save As activation/archive,
full public invocation/replay/recipient assembly and physical exclusive-create/
conditional-write procedures remain open. Formal artifacts/hashes are unchanged.
Overall estimate: approximately 75% (+2 points from 73%, low confidence), not a
test-count fraction; the full implementation goal stays active.

Native acknowledged Save As activation now changes the local source association
only after actual qualified/safe noncancelled task evidence, original binding,
current attested postwrite probe, final pair and both destination effect checks.
Explicit archive-row and root/promise envelopes precede publication. Changed
contexts retain the exact old binding/baseline and producer pins; same-context
activation preserves identity/history and scratch first-save invents no old row.
Later edits survive, only the named reservation is released, and retained outcomes
distinguish local activation from write/release/quiescence. Activated producers
participate in ordinary baseline election, late version refinement and actual
current/history retention. Ten public scenarios, two unit tests and one actual
macOS write/probe/activation test cover this slice; the initial acceptance failed
on the refusal-only scaffold. Explicit history release, pure rebinding, full
public invocation/replay/recipient assembly and stronger filesystem procedures
remain open. Formal artifacts/hashes are unchanged.

Save As checkpoint: 1,372 default workspace tests pass debug/release, formatting,
strict all-feature Clippy, documentation tests and both pinned Unicode reference
tests in each profile. Thirteen tests were added. Overall estimate: approximately
77% (+2 points from 75%, low confidence), not measured coverage. The native local
activation/archive slice is complete; the full implementation goal stays active.

Native explicit source association-history release now removes only the live
owner's known archive under the exact expected binding, bounded row profile,
current structured ordinary pair and twenty-four strict current control/discharge
facts. It neither requires file-write/content-disclosure rights nor performs I/O.
Native unbound expectations are distinct from loaded generation zero; unexpected
archive/producer drift fences rather than fabricating empty history. Current
baselines, edit/annotation history, task facts, writer reservations and artifact
charges survive. Empty archives need no publication unit; nonempty root shrink
still honors actual capacity and pending baseline byte/counter floors. Five public
scenarios and two unit tests cover the slice; initial acceptance failed on the
refusal-only scaffold. Pure rebinding, own-chain progression and full public
invocation/replay/recipient assembly remain open. Formal artifacts/hashes are
unchanged. Checkpoint: 1,379 default workspace tests pass debug/release, formatting,
strict all-feature Clippy, documentation tests and both pinned Unicode reference
tests in each profile. Seven tests were added. Overall estimate: approximately
78% (+1 point from 77%, low confidence), not measured coverage. This native control
slice is complete; the full implementation goal stays active.

Native pure source rebinding now changes only the local association after twenty
strict current facts, exact binding/registration/ownership and structured pair,
both destination effect domains, pending export intentions and archive/root/promise
checks. A changed context archives the exact old optional baseline and leaves the
new baseline absent; same-context is unchanged, including at counter exhaustion.
Scratch first-binding invents no old row. The explicit Option representation does
not turn absence into empty bytes or a validator: compatibility comparison and
ordinary pinned saves return MissingEvidence until an explicit qualified baseline
exists. Source edits/undo, annotations, logical relations and producer ledgers survive;
no writer, load, rename or file write is performed. Seven public scenarios and
three unit tests cover the slice; initial acceptance failed on the refusal-only
scaffold. Queued source-condition origin retention, own-chain save progression,
full public invocation/replay/recipient assembly and broader host/physical assurance
remain open. Formal artifacts/hashes are unchanged. Checkpoint: 1,389 default
workspace tests pass debug/release, formatting, strict all-feature Clippy, docs
and both pinned Unicode reference tests in each profile. Ten tests were added.
Overall estimate: approximately 80% (+2 points from 78%, low confidence), not
measured coverage. This native association/baseline-absence slice is complete;
the full implementation goal stays active.

Native source-writer retirement now checks actual nonstopped queued consumers.
Exact captured baseline origins and same-destination written records after older/
equal anchors remain retained despite a newer root baseline. Strictly newer
anchors do not pin older records; stop ends only the queue consumer. Original
anchor task/document/binding/whole registration, artifact bytes and issued launch
serials are validated; drift fences rather than deleting evidence. Existing root/
history, safe disposition, fault, quiescence and delivery-drain guards remain.
Four public BDD scenarios and one unit regression cover the slice; initial
acceptance failed because the old producer incorrectly retired with a queued
baseline-origin consumer. The actual optional bounded SaveProducerRetirement
boundary is content-free and claims no measured timing or external effects.
No new pin registry, root charge or worker is introduced. Own-chain advancement,
typed epoch/causal receipt capabilities and full public persistence composition
remain open. Formal artifacts/hashes are unchanged. Checkpoint: 1,394 default
workspace tests pass debug/release, formatting, strict all-feature Clippy, docs
and both pinned Unicode reference tests in each profile. Five tests were added.
Overall estimate: approximately 81% (+1 point from 80%, low confidence), not
measured coverage. This native retention gap is closed; the full implementation
goal stays active.

Native present-source causal evidence now preserves explicit bounded epoch/token
read/result/edge facts through pinned dispatch, baselines, Save As and actual root
accounting. Causal and non-reuse capability declarations default off independently;
opaque validators are not parsed or promoted. The v2 dispatch view preserves typed
versions; the fallible legacy v1 view refuses loss rather than stripping epochs.
Protected original intake separates actual before/after causal edges from results,
disposition and quiescence. Incompatible facts fault in either order; typed result/
edge contradictions share a fixed two-fact cap while compatible fixed-role result
metadata remains bounded. Full witness storage cannot partially rewrite first facts.
Nine public save scenarios include all 24 receipt orders; another public scenario
covers typed Save As postwrite epochs. Two unit tests cover downgrade/root-fault
intake and exact epoch-funded birth; one real macOS test proves pre-write refusal
with preserved non-write/disposition/quiescence facts despite a fixture capability
overclaim. Initial acceptance failed on an Unsupported factory scaffold. Own-chain
derivation/expanded-envelope budgets, absent-create causal receipts, full public
profiles and actual provider causal/non-reuse truth remain open. Formal artifacts/
hashes are unchanged. Checkpoint: 1,407 default workspace tests pass debug/release,
formatting, strict all-feature Clippy, docs and both pinned Unicode reference
tests in each profile. Thirteen tests were added. Overall estimate: approximately
83% (+2 points from 81%, low confidence), not measured coverage. This present-source
causal evidence slice is complete; own-chain advancement and the full engine
implementation goal remain open/active.

Native own-chain source admission/launch now share the ordinary codec/writer with
pinned saves. Original typed conditions and verified anchors remain immutable;
separate launched conditions/receipts derive only from retained matching actual
write edges, safe disposition, current permission and both capabilities. Known
nonapplication is skipped. Epoch/equal-token/multi-edge reuse, missing facts,
foreign contexts, legacy bypass and bad final probes refuse without refresh or
serial issuance. Actual causal ingress uses the launched head. One-time explicit
native edge/derivation/receipt/aggregate metadata credits preflight minimum before
artifact issuance and expansion before launch; stop is not refund, actual drained
retirement is. These are logical native projections, not wire or physical memory
guarantees. Typed current root anchors also retain required prefixes until a newer
eligible baseline is actually published. Same-context copies retain real source
generations without becoming baseline producers. Eleven public scenarios, three
unit tests and one strengthened existing typed Save As/copy scenario cover this
slice; initial acceptance failed on an Unsupported scaffold. Optional bounded
setup/admission/derivation signals and actual credit usage remain content-free.
Public wire/invocation/replay/recipient profiles, absent-create causal receipts and
host/physical/concurrent/crash truth remain gates. Formal artifacts/hashes are
unchanged. Checkpoint: 1,421 default workspace tests pass debug/release, formatting,
strict all-feature Clippy, docs and both pinned Unicode reference tests in each
profile. Fourteen tests were added, and the strengthened existing Save As/copy
case was additionally rerun in debug/release. Overall estimate: approximately 85%
(+2 points from 83%, low confidence), not measured coverage. This native present-
source own-chain/metadata-credit slice is complete; the full implementation goal
stays active.

Native absent-create after-results now use an explicitly selected Opaque/EpochToken
schema independent of the Absent before condition. V1 absence stays opaque; present
destinations cannot switch schema. Capabilities/create grants remain independent,
and safe admission refusal consumes preparation without issuing artifact/task work.
Typed after-results can refine an unavailable version and establish a qualified
Save As baseline/zero-edge own-chain anchor. Copy remains unbound. A concrete
before-version edge contradicts absence and is retained in the existing fixed
two-fact conflict store, in either result order, without erasing known writes or
manufacturing a creation edge. Five public scenarios and one internal profile-drift
test cover this slice; initial acceptance failed at protected intake. Existing
bounded admission/evidence diagnostics carry no content or invented time. Physical
exclusive-create and full public invocation/replay/recipient/wire composition remain
gates; no formal artifact/hash or independent-proof claim changes.

Typed-create checkpoint: 1,427 default workspace tests pass debug/release, formatting,
strict all-feature Clippy, docs and both pinned Unicode reference tests in each
profile. Six tests were added. Overall estimate: approximately 86% (+1 point from
85%, low confidence), not measured coverage. This native after-result receipt slice
is complete; the full engine implementation goal remains active.

Native ordinary source-save operation parents now bind the complete bounded
origin/request and existing native execution context in the shared writer with
one actual serialized child. Matching current authorized lookup returns that same
task, without recapture, dispatch, authored-root publication or terminal-result
replay. Changed intent/scope or retired operations refuse. First admission checks
all twenty strict native premises and existing actual source/format/binding/
registration/pair/codec/continuity guards. Failed child admission spends no parent;
raw tasks cannot be adopted. Guarded actual child retirement installs the prepaid
operation tombstone while releasing only independently discharged child payloads.
Unknown effects, stop, quiescence and metadata recovery do not waive obligations.
One-time explicit label/capture/reply/lifetime-store bounds reserve conservative
native logical metadata independently of codec payloads; no physical/wire bound
is claimed. Parent/child/reservation drift faults without erasing debt. Nine public
scenarios and two internal tests cover this slice; initial matching acceptance
failed on an Unsupported scaffold. Actual operation gauges and bounded optional
configuration/admission/recovery/retirement markers contain no private labels or
invented clocks. Annotation parents, full public codec/invocation/observer/event
assembly and permission-filtered terminal receipts remain gates. Frozen formal
artifacts/hashes are unchanged; no whole F21/goal or independent-proof closure.

Owned-source operation checkpoint: 1,438 default workspace tests pass debug/release,
formatting, strict all-feature Clippy, docs and both pinned Unicode reference tests
in each profile. Eleven tests were added; final full gates include the unconfigured-
store linkage guard/regression. Overall estimate: approximately 87% (+1 point from
86%, low confidence), not measured coverage. This native source-parent slice is
complete; annotation parents and the full engine implementation goal remain open.

Native ordinary annotation save parents now share the source operation namespace,
lifetime envelope, actual codec/writer/FIFO and spent tombstones. The original
parent captures source/set, encoding-context identity, complete annotation/source
association and pinned destination bytes/version. First admission installs the
existing actual artifact/task/baseline-capacity claim with its parent; failed
schema, producer, root/claim or host checks spend none. Matching recovery needs
annotation disclosure rather than source-read permission, preserves the exact
original claim/task and ignores replacement candidates/technical profiles after
edits or an ordinary admission fence. Typed causal and annotation own-chain refuse.
Source absence is explicit; scratch annotations do not invent a source binding.
Actual claim/effect/quiescence/nonce/artifact/consumer discharge remains independent
of metadata recovery and parent tombstoning. Native capsule projections fund the
widest inline role header, pinned condition and both association-key incidences;
role/context/condition drift faults without erasing obligations. Nine public
scenarios and two internal tests cover this slice, including strict twelve-fact
release/twenty-nine-fact fresh admission, shared namespace, exact capsule bounds,
root refusal, unknown effects, post-edit recovery and off/zero/full diagnostics.
Initial matching acceptance failed on an Unsupported scaffold. Standard producer,
JSON, external policy/alias/provider truth and physical/concurrent/crash guarantees
are not proved by issuer fixtures. Full public codec/invocation/observer/events and
permission-filtered retained terminal receipts remain open; formal artifacts/hashes
are unchanged. No whole F21/engine-goal or independent-proof closure is claimed.

Owned-annotation operation checkpoint: 1,449 default workspace tests pass debug/
release, formatting, strict all-feature Clippy, docs and both pinned Unicode
reference tests in each profile. Eleven tests were added. Overall estimate:
approximately 88% (+1 point from 87%, low confidence), not measured coverage. This
native annotation-parent slice is complete; public facade/retained-result
integration and the full engine implementation goal remain open/active.

Native owned save observation now implements the source-save/annotation-save
portion of §23.1.11 through original private writer parents, not publication
receipt selectors or a second executor. Captured/current policy intersection,
separate source/annotation disclosure, ordinary status/evidence signatures and
independent stop/quiet-only protected progress govern actual retained facts.
Late evidence never dispatches, adopts baselines or implies quiescence/stronger
acknowledgment. Zero ordinary issuance and local receipt quarantine preserve
protected control. Raw/foreign targets refuse; observers add no producer pin.
Explicit scope loss closes absorbingly; temporary capacity/receipt denial does
not. Bounded owned waits consume nonrenewable strictly advancing clock-epoch
steps, not wall time; ended slots require actual protected discharge and observer
stop never stops a producer. The shared retention parent reserves the lifetime
inventory plus every whole child ceiling before exposure under checked native
5R + 2C + 3X + (W+1)B + (I+4)N bounds and minimum frame sizes. Native gauges and
static optional markers distinguish queries, permitted snapshots, local faults,
active/ended waits and actual cleanup without private labels or invented time.
Nine public scenarios and three internal tests cover this slice; initial acceptance
failed on an unavailable scaffold. Unwinding result faults quarantine only the
receipt facility; process/allocator abort, physical/wire/clock/concurrent truth and
independent certification are not proved. Native versions/actual captured strength
remain lossless; portable/default version/acknowledgment mapping, read-parent
observation, public codecs/discovery, publication receipt and event/shutdown assembly
remain open. Frozen formal artifacts/hashes are unchanged; no full F06/goal closes.

Native-save observation checkpoint: 1,461 default workspace tests pass debug/release,
formatting, strict all-feature Clippy, docs and both pinned Unicode reference tests
in each profile. Twelve tests were added. Overall estimate: approximately 89%
(+1 point from 88%, low confidence), not measured coverage. This native save-
observation/wait slice is complete; the full implementation goal and remaining
read/public-profile/receipt/event/shutdown deliverables remain open/active.

Native annotation-load operation ownership now executes against actual reader
tasks. Exact authorized retry returns the same task without submission/issuance;
changed origin intent, scope, association or reader registration refuses. Reader
registration is not writer identity. Current release facts remain mandatory on
recovery, while fresh launch facts and the admission fence apply only to new work.
Ordinary, superseded and disposed retirement spend the same co-owned parent only
after their actual quiescence/capture/provenance guards. Origins remain spent.
Shared retention prepays bounded lifetime inventory and native work sponsorship;
only irreversible protected retirement after actual drain returns that credit.
Nine public and three internal scenarios cover this slice, including native
integrity/boundary/drain tests and bounded private instrumentation. No source-read
job is fabricated from the annotation reader. Load observation, source async-read
execution and full public-profile/receipt/event/shutdown deliverables remain open.
Frozen formal artifacts/hashes are unchanged; no full F06 or engine goal closes.

Native annotation-load ownership checkpoint: 1,473 default workspace tests pass
debug/release, formatting, strict all-feature Clippy, docs and both pinned Unicode
reference tests in each profile. Twelve tests were added. Overall estimate:
approximately 90% (+1 point from 89%, low confidence), not measured coverage.
The ownership slice is complete; load observation, source async-read execution,
public-profile/receipt/event/shutdown deliverables and the full goal remain active.
No JavaScript qualification rerun or new formal proof is claimed.

The annotation-load branch of native operation observation now executes through
the same funded scope/policy/progress/wait/protected-control lifecycle as saves.
The provisional shared Rust API/markers use `NativeOperation...`; closed target
and receipt sums preserve distinct native save/read products. Only actual owned
annotation readers attach. Counts, phase, stop/quiescence, conflict/refusal and
at most two unique original end/failure witnesses release under current read and
annotation grants, never source or disposition substitutes. Chunk-conflict bodies,
restoration and baseline results are excluded. EOF does not publish; observers
do not pin or revive retired producers. Ten public scenarios and two internal
checks cover the new branch and shared-bound/fault behavior; existing save tests
exercise the same kernel. No new formal proof/JavaScript qualification is claimed.
Source async-read execution/observation, portable version/acknowledgment mapping,
complete public codecs/discovery/receipts and event/shutdown remain open.

Native annotation-load observation checkpoint: 1,485 default workspace tests pass
debug/release, formatting, strict all-feature Clippy, docs and both pinned Unicode
reference tests per profile. Ten public and two internal tests were added. Overall
estimate: approximately 91% (+1 point from 90%, low confidence), not measured
coverage. This native branch is complete; the full engine goal remains active.

The native raw source reader now owns a distinct funded task/transcript/candidate
lifecycle and explicit initial-load activation. It does not borrow an annotation
task or activate at EOF. Original provider/nonce, bounded ordered chunks, exact
versions/end and strict UTF-8 preserve split signatures/scalars and original bytes.
Candidate success, stop and quiescence remain distinct. Activation creates a fresh
owner or joins an existing dirty owner unchanged; repeated activation checks the
actual writer association, independently of reader registration. A regression
reproduced and fixed that latter distinction. Source and annotation inventories
have separate explicit funding and one shared worker ceiling/aggregate gauge.
Twelve public and two internal scenarios cover native behavior, bounds, integrity,
publication exhaustion, actual pin drain and private instrumentation. Pool credit
returns only after irreversible protected actual-drain retirement. Atomic snapshot
strength refuses rather than downgrading. This is not source-load owned-parent
retry/observation, recipient/current grant assembly, portable profiles, a native
filesystem worker, reload/replacement or full event/shutdown qualification. Frozen
formal artifacts/hashes remain unchanged; no new formal proof is claimed.

Native source-reader checkpoint: 1,499 default workspace tests pass debug/release,
formatting, strict all-feature Clippy, docs and both pinned Unicode reference tests
per profile. Twelve public and two internal tests were added. Overall estimate:
approximately 92% (+1 point from 91%, low confidence), not measured coverage.
The raw-reader/initial-activation slice is complete; source-load owned-parent
recovery/observation, public-profile/receipt/event/shutdown work and the full engine
goal remain active. No JavaScript qualification rerun or new formal proof is claimed.

Owned source-load admission/recovery now runs against the actual native source
reader. Original request/scope binds one child; matching current release returns
that exact task without issuance, submission, decoding, activation or replay.
Raw labeled histories cannot be adopted; unlabeled legacy tasks are never adopted.
Changed intent/scope refuses. Parent/child consistency and capsule/ledger bounds
are checked; actual child retirement spends the parent in the same serialized
transition, preserving its origin and registry sponsorship. Protected global drain
must retire that registry before reader sponsorship can return. Eight public and
two internal scenarios cover this slice; initial acceptance failed on an unsupported
configuration scaffold. Recovery under a root fault remains metadata-only.

The source-load observation branch now completes the fixed four-kind native
dispatch through the existing funded scope/policy/progress/wait/control kernel.
It releases only safe read counts and original terminal metadata, not chunks,
candidates, activation or baseline results. Source-read/disclosure grants do not
borrow annotation/write-disposition permission. Two typed end witnesses preserve
all four bounded validator fields; opaque versions acquire no invented epochs.
Seven public and two native tests cover this branch, exact wrapper bounds and local
result quarantine with independent minimal control and preserved producer/debt.
Public typed/dynamic profile/registry descriptions, portable mapping, generic
publication receipt facets, complete event/shutdown and host/physical/concurrent
qualification remain open. Frozen formal artifacts/hashes remain unchanged;
no new formal proof or JavaScript qualification is claimed.

Owned source-load/four-kind observation checkpoint: 1,518 default workspace tests
pass debug/release, formatting, strict all-feature Clippy, docs and both pinned
Unicode reference tests per profile. Nineteen tests were added: eight public/two
native ownership checks and seven public/two native observation checks. Both slices
qualified together. Overall estimate: approximately 94% (+2 points from 92%, low
confidence), not measured coverage. These native slices are complete; public-
profile/receipt/event/shutdown work and the full engine goal remain active.
No JavaScript qualification rerun or new formal proof is claimed.

Native F06 typed/dynamic input binding now runs over actual four-kind observation
contexts. Exact version/contract/field/kind routing and bounded ordinary capture
preserve original opaque task/context/progress roles out of band; canonical shadows
use null for those roles. Target/profile/facet/scope mismatch refuses before native
loss handling, and no rejected native envelope falls back to publication receipt
decoding. Explicit owner-controlled activation prepays eight input frames plus
native bookkeeping in the actual shared parent. Queries consume neither another
workspace reservation nor context admission. Busy ownership/private native handoff
and guarded context retirement preserve live frames and native waiter obligations.
Unexpected decoder faults retain funding and leave direct native protected control
available; native receipt faults remain separate. Seven public and three internal
scenarios cover this slice, including 32 four-kind/form/route deliveries and exact
byte/node/depth/funding/fault checks. Initial normalization acceptance failed on an
unavailable scaffold. No new formal proof or JavaScript qualification is claimed.
Accepted-registry standard schemas/discovery, portable result normalization, the
complete normalized admission-pool relation, generic publication receipt facets,
event/shutdown and host/physical/concurrent qualification remain open.

Native F06 gateway checkpoint: 1,528 default workspace tests pass debug/release,
formatting, strict all-feature Clippy, docs and both pinned Unicode reference tests
per profile. Seven public and three internal tests were added. Overall estimate:
approximately 95% (+1 point from 94%, low confidence), not measured coverage.
Native input binding/prepaid decoding is complete; accepted-registry schemas/
discovery, portable result normalization, generic receipt facets, event/shutdown
and the full engine goal remain active. No JavaScript rerun or new formal proof
is claimed.

Discovery now has a native separately sponsored cut/context owner, not executable
registration. Attested bounded metadata/complete visibility/dense order feeds
single-use original-scope/publication-bound cuts. Acquisition captures permitted
rows/facets and independently observable targets without publication or authority
inference. All fallible copies/bounds precede context/issuance installation.
Wrong scope/foreign/stale handles refuse; consumed valid cuts do not resurrect
after later capacity refusal. Protected stop clears payloads, retirement preserves
issuance, fresh live-slot markers prevent aliasing and irreversible global drain
returns only the original parent reservation. Integrity faults fence discovery,
not the editing domain, and preserve uncertain debt/original protected cleanup.
Eight public and two native scenarios cover this prefix, including explicit base-
profile selection and unsupported/later-profile refusal. Native queries now add
fresh-cut list/inspect/advice, context-scoped lazy opaque selectors, exact cursor
limit binding, captured disclosure ceilings and complete bounded reply staging
before issuance. Explicit original-scope authority/retention loss or changed
permitted descriptors stops and clears payload/selectors; unknown facts withhold.
Foreign scopes cannot apply loss. Positive advice remains non-authoritative and
requires original variant/target plus independent current truth. Parent sponsorship
and lifetime issuance survive stop/refusal. Thirteen additional public scenarios
and one bounded iterative metadata-comparison scenario cover these behaviors.
The actual result guard now runs before native selector/issuance installation,
checking original request/context/role correlation, captured page/variant bounds,
approved metadata identity/facets/caps and tentative map use. A red/green private
producer-version regression and four corruption/control tables cover 36 cases in
five additional internal scenarios. Malformed producer results fence discovery,
not the editor root; prior issuance/debt/original cleanup remain, while tentative
selectors stay unpublished. A separate content-free marker records the actual
validation boundary. This is not an independent policy/schema or wire proof.
Native cuts now bind owner/publication plus exact administrative profiles,
append-only immutable registration positions, persistence fence/dispatch headers
and private attachment lifecycle generation/original incarnation. Seven public scenarios and three native
test cover non-publication registration, same-size header changes, attachment ABA,
idempotent controls, stale nonconsumption, queued-work exclusion, actual dispatch,
original-owner separation and fresh continuation without authored revision changes.
Private attachment bounds reserve the detach step before credit; global publication
exhaustion cannot block it. Corrupt local exhaustion retains endpoints/debt and
faults. Registration and absent-ABA regressions failed before their fixes;
identity is not a size gauge or
hash. Its fixed inline packet cost is included by existing native boundaries.
Exhaustive censuses and the current publication-or-header mutation audit do not
prove arbitrary future mutation paths, memory faults, host policy or physical state.
F01 native typed/dynamic query handoff now uses shared actual closed field/role
declarations and a separately once-funded context decoder. Bounded whole/scope/
canonical copies preserve original opaque roles and full native epochs; fifteen
current facts are copied before existing native query/result guards. No query
acquires another context or new workspace credit. Stop/fault preserves the decoder
reservation until actual protected retirement. Early malformed input leaves cuts
unconsumed; later refusal follows native consumption, and late faults retain spent
issuance. Eleven public and three native scenarios exercise bounds, all forms,
cursor continuation, malformed inputs, known/unknown loss, original scope, busy and
uncertain cleanup, post-native faults and private diagnostics. No wire/JSON, schema
truth, all-family registry or independent proof follows from these native tests.
Whole F01 is not closed: schema/accepted-registry release and complete facade
integration remain. External accepted-registry truth/currentness and normalized
JSON/native accounting remain open. No formal proof or
JavaScript qualification is claimed. Overall estimate remains 95% pending discovery
integration; this is not measured coverage.

Native scoped-discovery checkpoint: 1,581 default workspace tests pass debug/
release, formatting, strict all-feature Clippy, docs and both pinned Unicode gates
per profile. Discovery has 39 public and eleven internal scenarios, plus one native
root-identity and two attachment-generation tests. The decoder slice adds eleven
public and three native tests. Native typed/dynamic query handoff, shared actual
field declarations, separately prepaid workspace ownership and protected retirement
are implemented. External registry/schema/complete facade integration, full F01 and the engine goal remain
incomplete. Overall estimate
remains approximately 95% (low confidence), not measured coverage. Schema release,
complete facade assembly, accepted-registry truth/currentness, receipt/event/shutdown and
other acceptance-plan gates remain. No new formal proof or JavaScript rerun is
claimed.

Native dependent-reversal handler prerequisite: native owner submissions now retain
original group or expected source/annotation frontier plus direction and canonical
V3 dependencies in the shared invocation ledger. Service checks exact history
kind/group/direction/read capture; final validation and publication reuse the
existing linked source/annotation publisher. The initial red regression proved
that reclassifying an undo plan as an ordinary edit previously succeeded. Eight
native and three public scenarios cover the fix, bounded admission, no-refresh
replay, stop, all-or-none late-read refusal, annotation restoration and optional
diagnostics. This does not close F14 recipient/wire/registry integration or revise
formal artifacts/hashes. Overall estimate stays approximately 95% (low confidence).
This handler checkpoint passes 1,592 default workspace tests in debug and release,
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference tests in each profile. The full engine goal remains active.

Native selected builtin declaration source (§22.1.6) now derives the six F13/F14
logical products from handler-owned immutable declarations and exhaustive variant
mapping. Bounded trusted-owner inspection and fixed-source preparation use the
existing discovery owner, shared sponsorship and scoped queries. Private source
origin prevents generic/builtin substitution before consumption or loss effects;
visibility/authority/resource/advice premises remain independent. Eight public
and two native scenarios cover declaration release, origin, native limits, stale
roots, captured ceilings, protected lifetime and optional bounded diagnostics/
local service timing. This closes the selected declaration-origin implementation
slice, not schema/value/recipient/all-family registry correspondence. Formal
artifacts/hashes are unchanged; no new formal proof or JavaScript qualification is
claimed. Overall estimate stays approximately 95% (low confidence).
Builtin-source checkpoint: 1,602 default workspace tests pass in debug and release,
plus formatting, strict all-feature Clippy, documentation tests and both pinned
Unicode reference tests in each profile. Discovery has 47 public and thirteen
internal scenarios. The full engine implementation goal remains active.

Native discovery request-recognition correction: an actual red regression showed
that foreign selectors could apply known-loss facts to a healthy original context.
Original role/kind/correlation and scalar/cursor bindings now precede loss effects
and cut consumption. Valid recognized loss still absorbs before current page
capacity and transient delivery refusal. Six public and one native test cover the
guard, direct/typed/dynamic paths, all loss facts, protected debt, local corruption
and optional diagnostic/timing behavior. Malformed scalar/cursor admission now
preserves its cut; two previous later-consumption assertions were strengthened.
Frozen formal/reference ordering is not reconciled or newly proved. /2 remains
unavailable pending its native resource-registration ports and complete descriptor/
handler correspondence; no partial profile or unsupported metadata is substituted.
Overall estimate stays approximately 95% (low confidence); full goal remains open.
Role-recognition checkpoint: 1,609 default workspace tests pass in debug and
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode gates per profile. Discovery has 53 public and fourteen native
tests; the full engine implementation goal remains active.

Native resource-catalog activation prerequisite is now implemented: bounded known
read/write metadata and closed typed batches, explicit absent/present (including
zero) expectations, one seeded/non-reused registration floor, complete owned
request/candidate/result capture and fresh owner/root/profile/floor reconstruction.
Publication, root capacity and exact root identity include the catalog without
changing actual provider ports or document bindings. Ten native, four public and
one discovery-root scenario cover atomicity, clocks/exhaustion/no-ops, independent
limits and facts, original certificate/corruption, frames and optional bounded
instrumentation/local timing. Catalog-backed consumer captures/staleness, owned
invocation/scoped bases/receipts/events, recipient products and discovery /2 remain
open. This is not whole §22.3.3–§22.3.6 qualification, a provider-truth claim or global
caller-held-plan accounting. Formal artifacts/hashes remain unchanged. Overall
estimate stays approximately 95% (low confidence); the full engine goal is active.
Native catalog checkpoint: 1,624 default workspace tests pass debug/release,
formatting, strict all-feature Clippy, documentation tests and both pinned Unicode
reference gates in each profile. The full engine implementation goal remains open;
this does not qualify provider truth, registered consumer lifetimes or invocation/
recipient/outer-schema integration.

Owned native registration metadata jobs now use the existing invocation machine:
complete original request/context/profile/ticket, no old-root retention as a service
authority, fresh reconstruction, exclusive native publication plus sealed retained
outcome, protected stop and nonexecuting replay. Typed retained delivery intersects
captured/current permission and bounds, preserving unsafe native stamps behind
explicit trusted-host classification. Twelve native and three public scenarios
cover scope/protocol/limits/faults/lifecycle and actual optional diagnostics/timing
plus once-only installed admission/terminal counters. Unexpected staging/publication
interruptions remain Faulted/Running, never known nonpublication or retry. Consumer
registration lifetimes, scoped bases, recipient/wire/events and /2 remain open;
this is not full F16, concurrency/crash or new formal refinement. Frozen formal
artifacts/hashes remain unchanged. Estimated overall progress remains approximately
95% (low confidence); the full engine goal remains active.
Owned-registration checkpoint: 1,639 default workspace tests pass in debug and
release, plus formatting, strict all-feature Clippy, documentation tests and both
pinned Unicode gates per profile. This does not close catalog-backed consumers,
scoped registration bases, recipient/wire or discovery/schema integration.

Conditional registration basis authority (§22.3.5) now has a native finite pool:
opaque original handles, complete scope/policy/native-profile binding, full present
row and absent-predicate currentness, independent observation/private-host grants,
shared-parent sponsorship and protected nonrefunding retirement with a spent issuer
marker. Nine native tests and three public scenarios exercise actual Rust behavior,
including envelope limits, ABA, authority, root/provider frames and bounded optional
instrumentation. Verification is focused on these and adjacent catalog/invocation/
root/retention paths, formatting and engine all-feature Clippy; the last full debug/
release qualification remains 1,639. Scoped mutation composition (§22.3.6), consumer
registration lifetimes and public wire/discovery integration remain open. No frozen
formal artifact/hash changed and no new formal refinement is claimed. Estimated
overall progress remains approximately 95% (low confidence); the full goal is active.

Typed native scoped registration now composes those bases with the existing
registration job and native publisher. Original issuer/scope/reference/index meaning
and separate captured/current result rights bind both service and historical
delivery. Native/scoped entry points refuse each other's plan flavor; only scoped
index/state receipts leave the scoped result facet. Thirteen native tests and two
external BDD scenarios cover authority/staleness/limits/replay/protected lifetimes,
unknown faults and actual bounded instrumentation/cohorts. The first public API
and fresh-companion replay regressions failed before their fixes. Passive/dynamic
scoped admission, registered consumers, public wire and discovery /2 remain open;
this is not full §22.3.6 or formal refinement. Frozen formal artifacts/hashes are
unchanged. The full goal remains active; approximately 95% is a low-confidence
estimate, not measured acceptance coverage.

Scoped-registration verification: affected registration/catalog/basis/control unit
filters and four external BDD targets pass, along with formatting and strict engine
all-target/all-feature Clippy. No new full-workspace/release/Unicode/formal run is
claimed; 1,639 remains the last full debug/release checkpoint.

Passive native scoped registration envelopes now normalize exact closed schemas
into the existing typed admission path with bounded inert/typed workspace. Original
invocation roles remain borrowed and actual basis references stay outside inert
data; normalized values confer no authority. Six native tests and a 72-combination
BDD scenario cover field/profile/Boolean/index integrity, exact limits, typed intent
equivalence and fresh current admission. The existing optional diagnostic/cohort
scenario also covers decoded refusal/replay and actual timed boundaries. No wire
role codec, global ingress sponsorship, public discovery /2 or formal refinement
is claimed; frozen formal artifacts/hashes remain unchanged. Registered consumer
currentness and the full goal remain open. The approximate 95% overall estimate
is low confidence, not a measured acceptance percentage.

Passive-admission verification: 31 registration unit tests, three external scoped
BDD scenarios, formatting and strict engine all-target/all-feature Clippy pass.
This is focused verification, not a new full-workspace/release/formal checkpoint.

The approved catalog/provider boundary now has a native source-read implementation:
host-established correspondence becomes an engine-issued immutable full-row binding
to the original opaque native target. Admission/dispatch/activation enforce row
currentness without treating unrelated publication as invalidation. Identity-known
activation and actual native guarantees remain independent. Withdrawal preserves
original late evidence, owned recovery, issuance and outstanding handle retention;
unbound replay cannot replace a bound intent. Reader-held and owned-intent logical
envelopes include the full capture. Optional bounded content-free binding/currentness
markers support opt-in timing. Native-only targets remain a separate privileged
profile. Annotation/writer and insertion-policy composition, recipient/wire and
discovery /2 remain open. No new formal proof/refinement or hash update is claimed.
The approximate 95% overall estimate remains low confidence, not measured coverage.

Focused source-read binding verification passes 24 catalog/basis/consumer unit
tests, four adjacent source-reader/owned-operation unit tests and 37 public source
read scenarios (six new binding, 31 existing). No new full-workspace, release,
Unicode or formal checkpoint is claimed; the last full checkpoint remains 1,639.
Formatting and strict engine all-target/all-feature Clippy pass.

Catalog-backed annotation reads now use the same immutable capture with a closed
source/annotation kind and independent annotation grant. Currentness gates native
admission, dispatch, new capture and restoration; known identity gates restoration.
Late evidence and previously returned capture bytes survive withdrawal, completed
restoration outcomes remain historical, and owned recovery cannot drop or replace
the original catalog dependency. Full capture bytes are bounded in task/intent
charges; static task-charge validation does not consult current rows. Optional
bounded binding/currentness observations support native timing and off/saturated
equivalence. A public scenario reproduced stale admission before consumer wiring.
Writer consumers and insertion-policy consumption remain open. No new formal
refinement, whole-suite checkpoint or overall completion claim is made; the rough
overall estimate remains approximately 95%, low confidence, not measured coverage.

Annotation-read binding verification: 97 focused tests pass (46 catalog/native
reader unit tests, seven new public catalog-annotation scenarios and 44 adjacent
public scenarios), with formatting and strict engine all-target/all-feature Clippy.
No full-workspace/release/Unicode/formal suite was rerun; 1,639 remains the last
full checkpoint. Frozen formal inputs and hashes are unchanged.

Catalog-backed ordinary source/annotation writes now use an engine-issued immutable
writer binding separately from the native document association and pinned condition.
Original full-row currentness and known identity gate admission, dispatch and new
baseline installation. Metadata cannot grant a missing native guarantee. Original
receipts and installed historical baselines survive withdrawal. The task/artifact
share one full capture charged by the existing artifact hold; source retirement
releases it through the existing owned-artifact path. Wrong native destinations
are expected refusals, and native-only annotation export cannot strip a catalog
artifact's dependency. Optional bounded binding/currentness markers support timing
and disabled/saturated equivalence. The first public test failed on missing APIs
before implementation. Owned catalog jobs, own-chain and export/Save-As remain
separate open composition; no complete writer/facade or new formal refinement is
claimed. The rough overall estimate remains approximately 95%, low confidence,
not measured coverage.

Ordinary writer verification passes 237 focused tests: 100 resource unit tests,
eight new catalog-writer scenarios, 75 save scenarios, five source-artifact scenarios
and 49 owned-operation/observation scenarios. Formatting and strict engine all-target/
all-feature Clippy pass. No full-workspace/release/Unicode/formal run was performed;
1,639 remains the last full checkpoint and frozen formal artifacts/hashes are unchanged.

Owned catalog source/annotation saves now retain full original writer bindings in
the existing parent ledger. Parent byte ceilings include the complete capture
before child/origin issuance. Static task/artifact/parent correlation is separate
from currentness: matching historical recovery survives withdrawal without launch,
new candidate or baseline reservation. Bound/unbound or different captures cannot
share an origin; a separately issued semantically identical full binding may
recover it. Fresh work retains all current native and catalog premises.
Catalog own-chain saves require explicit continuity/causal/nonreuse declarations
as well as actual native support and the existing chain/probe/permission/credit
law; conditional write and synchronization are not inferred. Actual optional
admission/recovery/currentness observations cover this path, including saturated
capture. The initial public scenario failed on missing APIs before implementation.
Catalog export/Save-As composition remains open; no new registry, wire codec,
formal refinement or whole-engine completion is claimed. The rough overall
estimate remains approximately 95%, low confidence, not measured coverage.

Owned catalog/own-chain verification: 79 focused tests pass (three writer unit,
four parent/child unit, twelve catalog writer, 49 existing operation/observation and
eleven continuity scenarios), plus formatting and strict engine all-target/
all-feature Clippy. Exact parent capture bounds precede issuance, and the catalog
chain follows an actual acknowledged predecessor edge. No full-workspace/release/
Unicode/formal suite was rerun; 1,639 remains the last full checkpoint and frozen
formal inputs/hashes are unchanged.

Catalog source copy/Save-As (v1 opaque and v2 explicit after-result profiles) and
annotation exports now retain original writer captures through the existing writer.
Source transfer checks extra catalog headroom before issuance; expected refusal
discards only the owned preparation. Save As rechecks currentness before binding
activation, while original receipts, historical activation/outcome/baseline facts
and protected reservation release/retirement survive withdrawal. Exclusive create,
atomic conditional write and typed causal after-results require independent
declarations and actual native support; token nonreuse is not inferred from a typed
result. Annotation exports, including captured ordinary-association exports, own a
separately bounded artifact envelope sharing immutable payload/snapshot/provenance.
Reusable inputs are not retargeted; a prior catalog dependency cannot be stripped
or changed. Exact payload/condition/catalog and slot checks precede issuance, and
task retirement releases only the export-owned envelope. The initial public export
scenario failed on missing API before implementation. No new provider I/O, task
registry, recipient/wire enablement or formal refinement is claimed.

Export/Save-As verification passes 241 focused tests: 103 resource unit tests,
18 catalog-writer BDD scenarios (six export scenarios), 40 existing source-export
scenarios, 75 save scenarios and five source-artifact scenarios. Exact transfer and
clone bounds have under/exact/above cases. Save As checks off/saturated/available
timed capture at admission, dispatch and activation. Formatting and strict engine
all-target/all-feature Clippy pass. No full-workspace/release/Unicode/formal rerun
was performed; 1,639 remains the last full checkpoint and frozen formal hashes are
unchanged. The full goal remains active; approximately 95% is still a low-confidence
planning estimate, not measured coverage. Insertion-policy consumption and broader
facade/recipient/lifecycle acceptance remain open.

Generated-newline policy now reaches native document state and execution: birth
stores the explicit closed LF/CRLF/CR default in the fixed document header;
compatibility create/load select LF and host-aware birth/load accept another
policy. Catalog source activation consumes the captured strict-UTF8 default before
birth; alias joining preserves the existing owner/default/text without publication.
The exact-current-version query and default-newline facade reuse existing bounded
explicit line derivation, companion mapping, publication and history. Literal and
explicit/configured delimiters are unchanged. No policy setter/generation or new
command/wire variant is introduced. Optional bounded policy-read/default-newline
observations support timing and off/saturated equivalence. The initial public
scenario failed on missing APIs before implementation (and a fixture constructor
typo, subsequently corrected); remaining cases are composition checks.

Insertion-policy verification passes 70 focused tests: 13 root-accounting unit,
four native line unit, four source-reader unit, four new policy BDD, seven catalog
source-read BDD, seven line-edit BDD and 31 existing source-read BDD scenarios.
The catalog activation case now covers all three policies; a new join case and
native explicit-policy load case preserve the original owner/default. Formatting
and strict engine all-target/all-feature Clippy pass. No full-workspace/release/
Unicode/formal rerun or formal refinement is claimed; 1,639 remains the last full
checkpoint and frozen formal inputs/hashes are unchanged. The full goal remains
active; approximately 95% is a low-confidence planning estimate, not measured
coverage. Remaining work needs reconciliation against the actual engine acceptance
matrix before broad closure or another milestone qualification claim.

The native closure worklist is now reconciled in the current section of
docs/engine-test-plan.md: facade/discovery, retained results, lifecycle/retained
targets, observation/events, effect-control/shutdown, instrumentation completeness
and milestone qualification remain distinct open bundles. They are not equally
sized or a new count-derived percentage. Typing coalescing is conditional under
§14.1, not mandatory merely because the old README listed it; UI/clipboard
integration and additional filesystem platforms are outside the selected target.
Optional facilities must not be advertised before their engine contracts exist.
This corrects scope bookkeeping without dropping required native engine behavior.

Inspection found a concrete discovery prerequisite: native/scoped typed registration
handlers existed, but only scoped registration had a passive envelope path. The
native registration-invocation/1 normalizer and decoded submission now implement
§22.3.4's exact constants, borrowed original invocation role and complete ordered
table/provider/resource/expectation/configuration schema. Native identity/stamp
numbers are safe noncoercing naturals, zero-present differs from absence, duplicate
slots and wrong-table/extra/shadow products refuse before typed output allocation.
The existing scoped configuration parser is shared, preserving all Boolean/profile/
line-break meanings. Inert and joint inert/typed bounds are separate from native
job/root admission; normalized data grants no authority. Decoded submission checks
the original profile and delegates fresh native context/intent/authority guards.
Optional bounded timed normalization/admission observations cover the actual path.

Verification passes 43 focused tests: 34 registration unit tests and nine native/
scoped public scenarios, including three new unit/three public scenarios and all
72 complete configuration products. Formatting and strict engine all-target/
all-feature Clippy pass. No full-workspace/release/Unicode/formal suite was rerun;
1,639 remains the last full checkpoint and frozen formal artifacts/hashes are
unchanged. Discovery /2 still needs its complete actual handler-envelope/declaration
and schema producer/guard set; this slice does not enable it. The full goal stays
active; approximately 95% remains a low-confidence planning estimate, not measured
coverage.

Native/scoped serialization now also have passive Rust normalization and decoded
submission for §22.3.1.6/9. Exact eight-/six-field schemas preserve original borrowed
roles and a separately supplied scoped basis; inert data cannot mint authority.
Closed Boolean/format and safe positive generation validation, shadow rejection,
dependency cardinality and exact inert/joint bounds precede typed output allocation.
Borrowed dependency referent storage is explicitly excluded from the transient
bound. Decoded submission rechecks profiles and delegates existing authority,
basis/currentness, service and replay guards. Actual optional content-free timed
normalization/admission observations cover disabled, saturated and available capture.
Verification passes 57 focused tests (31 unit, 26 public save scenarios), formatting
and strict engine all-target/all-feature Clippy, including three new unit and three
public scenarios. No full-workspace/release/Unicode/formal rerun or refinement is
claimed; the last full checkpoint remains 1,639 and frozen formal hashes are
unchanged. This closes these two passive paths, not discovery /2 or the full goal.

The scoped-semantic-settings/1 Rust adapter now implements §22.3.1.5's exact passive
envelope and indexed set/remove/reset changes. Original invocation/dependency roles
remain borrowed and bases separately supplied. Schema/duplicate/null/index checks
and joint inert/output/conversion bounds precede iterative typed allocation;
native set-value node/depth and request/plan accounting remain typed-handler checks.
An executable regression first rejected empty/non-set envelopes under zero native
value-node/depth limits; separating normalization budgets fixes that mismatch.
Decoded profile checks delegate original reference/loss ordering, current service
permissions and once-only replay; timed optional content-free diagnostics cover the
actual normalization/admission paths. Four new unit and three public scenarios
participate in 26 passing focused tests (eleven unit, fifteen public). Formatting
and strict engine all-target/all-feature Clippy pass. No full-workspace/release/
Unicode/formal rerun or refinement is claimed; 1,639 remains the full checkpoint.
Frozen formal inputs/hashes are unchanged. Discovery /2 and other closure bundles
remain open; the goal stays active at the inherited approximate 95% low-confidence
planning estimate, not measured acceptance coverage.

The scoped-engine-settings-layers/1 Rust adapter now implements §22.3.1.8's exact
eight-field passive envelope, original invocation/dependency roles and separately
supplied lifecycle basis. Exact creation/contribution/removal/target-reference
schemas, safe numeric conditions, duplicate/shadow guards and joint inert/output/
conversion bounds precede typed allocation. It shares the iterative inert-value
converter with scoped ordinary settings. Repeated creation labels remain distinct
proposed incarnations, never inferred identities or grants. Decoded profiles gate
the original conditional job; current native candidate/value/root/plan checks,
validation-before-loss, service grants, source framing and once-only allocation/
replay remain authoritative. Optional content-free timed normalization/admission
observes actual paths. The first public scenario failed on missing APIs; four new
unit and three public scenarios are included in 63 passing focused lifecycle/
adjacent settings tests (23 unit, 40 public). Formatting and strict engine all-target/
all-feature Clippy pass. No full-workspace/release/Unicode/formal rerun or refinement
is claimed; 1,639 remains the full checkpoint and frozen formal hashes are unchanged.
Six of ten required F16 outer forms now have passive Rust paths, not a complete
schema/discovery /2 assembly. The full goal remains active; approximately 95% is
still only the inherited low-confidence overall planning estimate.

Scoped definition now has §22.3.1.7's passive six-field Rust adapter, with original
borrowed roles, separately supplied basis and iterative closed type/value/schema/
restriction/target conversion under inert and joint output/workspace bounds.
Decoded profile checks delegate compatibility, candidate/root/plan bounds, current
facet grants, loss ordering and publication/replay to the existing conditional job.
Four new unit and three public scenarios, initially red on missing APIs, participate
in 36 passing focused tests (23 unit/compiler, thirteen public), including timed
refusal/success and disabled/saturated equivalence. Formatting and strict engine
all-target/all-feature Clippy pass. No full-workspace/release/Unicode/formal rerun
or refinement is claimed; 1,639 remains the full checkpoint and no formal artifacts
were edited. Seven of ten F16 envelope paths now exist; /2 assembly and the overall
goal remain open. Inspection additionally records that the raw native ordinary-
settings port is privileged, not complete recipient-context admission/service;
inert metadata or profile-string equality must not erase that boundary. Overall
approximately 95% remains a low-confidence planning estimate, not measured coverage.

Native ordinary settings now has §22.3.1.1's exact passive six-field adapter and a
context-bearing admission/service path for §22.3.1.2, distinct from the unchanged
privileged legacy API. The existing ledger/publisher retains complete context,
immutable native conditions and caller/required/combined reads; fresh service
grants and dependencies remain decisive. No layer basis or observation grant is
issued/inferred. Generic service cannot bypass facts, and raw/scoped/native intent
substitution refuses. A failing equal-plan substitution test drove an immutable
preparation identity/weak seal; broken provenance/aliasing and post-install panic
remain unknown under the owner fault fence, never fabricated clean rejection.
Complete parent/plan and normalization envelopes are independently bounded; weak
identity retains no old settings root after settlement. Protected context stop
and non-rearming cleanup use the common ledger. Nine new unit and five public
scenarios participate in 452 passing focused tests (163 invocation unit, 289
settings-activation public). Formatting and strict engine all-target/all-feature
Clippy pass. No full-workspace/release/Unicode/formal rerun or refinement is claimed;
1,639 remains the full checkpoint and no formal artifacts were edited. Eight of ten
F16 envelope paths now exist; native lifecycle/definition, schema/discovery assembly
and the other closure bundles remain open. The goal stays active; approximately
95% is still an inherited low-confidence overall estimate, not measured acceptance.

Native definition now has §22.3.1.3's seven-field epoch-conditioned Rust envelope
and context-bearing execution path. It reuses the scoped declaration codec and
native values' sealed context/dependency/cancellation carrier via a closed native
participant branch. Current changed-facet grants remain independent; no conditional
basis or unrelated write grant is inferred. Complete native enum/boxed payload and
preparation envelopes are charged, with enum headers preflighted before capture.
Original preparation identity, required-read non-erasure, protected stop, source
framing and post-install unknown fencing remain common. Five new unit and four
public scenarios participate in 461 passing focused tests (168 invocation unit,
293 settings-activation public), including timed refusal/success and off/saturated
diagnostics. Typed/decoded cases initially failed on missing APIs; large-enum lint
findings drove compact boxing with corrected header/payload costs. Formatting and
strict engine all-target/all-feature Clippy pass. No full-workspace/release/Unicode/
formal rerun or refinement is claimed; 1,639 remains the full checkpoint and no
formal artifacts were edited. Nine of ten F16 envelope paths now exist; native
layer lifecycle, schema/discovery assembly and other closure bundles remain open.
The full goal stays active at approximately 95% (low confidence, not measured
acceptance coverage).

The target-identity prerequisite for live definition changes is now native:
opaque handles bind owner-local issued registrations, not physical vector rows.
Initial compilation assigns checked registrations; existing value-activation
copies preserve identities and the allocator. Queries, configuration-domain reads
and configured-line interpretation resolve against the particular current or held
root, so storage ordering cannot silently retarget a handle. Minting/lookup reject
corrupt duplicate bindings. Handles do not retain a settings snapshot or spend
interpretation quota. One BDD scenario covers repeated actual value rebuilds; five
native regressions cover storage permutation, old versus renewed/missing bindings,
historical roots, duplicate identities, exhausted allocation during ordinary value
changes and configured-line current/retained-root resolution. Removal/renewal are
fixture-only here: full live definition/association/layer activation remains open.
This prerequisite does not raise the rough 87% planning estimate or close F16.
Target-identity checkpoint: 567 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates in each profile. The full implementation goal stays active.
Native live declaration/association activation now replaces only the declaration
facets, not authored layers or initializer generations. Bounded flat original
captures bind expected interpretation, complete settings basis, fixed limits and
additional v3 reads. Validation preserves every authored contribution (including
unused layers), checks schemas/scopes/references/resolution and stages the actual
candidate before shared publication. Facet epochs and effective key/value-domain
maps advance separately. Existing registrations survive reorder/reassociation;
removed registrations never revive, and new targets start contextual counters at
one. Valid complete identity requires no unused counter and preserves number bits.
Direct and shared-ledger paths retain old interpretations, preparation holds,
historical retry/refusal meaning and the Running/Unknown publication fence.
Sixteen BDD scenarios and seven native tests cover the transition, all closed types,
constraints/counters/ABA, additional reads, replay, charged hold discharge,
retirement, whole descriptor thresholds, private capture substitution and actual
post-install interruption. Optional definition boundaries are content-free.
Malformed expected epochs refuse before binding an issued invocation; valid
identity still executes with exhausted unused publication capacity.
Recipient registry-role enforcement, dynamic F16/F07 codecs, authored-layer
lifecycle and remaining engine families remain open. This closes the privileged
native definition/association transition, not full F16 or a universal proof.
Definition/association checkpoint: 590 default workspace tests pass in debug and
release, plus formatting, strict all-feature Clippy, documentation tests and both
strict Unicode reference gates in each profile. Rough planning estimate: 88%, not
measured coverage. The full engine implementation goal remains active.
Native authored-layer lifecycle now uses explicit complete create/remove/target
proposals, expected configuration/high-water and additional v3 reads through the
same direct publisher and retained ledger. Compiler initialization records all
accepted layer incarnations; successful publication alone advances contiguous
issuance, and removal never refunds it. Surviving identities/generations/values and
old interpretations remain intact. Compact arena copying drops removed values;
target/domain counters share the definition transition rules. Unattached changes
preserve unaffected domains/association epochs. Ordinary value plans now remap
captured identities rather than relying on storage indices after unrelated removal.
Twelve BDD scenarios and six native regressions cover lifecycle/issuance, bounds,
counter exhaustion, staleness/frame behavior, replay/refusal, exact numeric bits,
retention, private capture drift, actual publication interruption and optional
content-free instrumentation. Native logical created-value/request bounds do not
select a serialized encoding. Recipient registry grants, dynamic codecs, general
parent funding, host/process restoration and other engine families remain open.
Layer-lifecycle checkpoint: 608 default workspace tests pass in debug/release,
plus formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates in each profile. Rough planning estimate: 89%, not measured
coverage. The full engine implementation goal remains active.
Native declared configuration dependencies now add only the specified domain and
whole-state kinds under a new v3 read profile. Both require schema/restriction/
association epochs and positive safe expected versions. Actual owner-bound target/
registry handles replace caller identity labels; these are not recipient grants.
The same canonical bounded conjunction serves advice, direct/scoped compound plans
and shared invocation replay/publication. Domain counters preserve unrelated and
origin-only progress; whole-state counters intentionally invalidate broader metadata
assumptions. ABA and changed no-ops refuse before any candidate installation. Reads
remain non-authoring; empty conjunctions require no unused configuration/workspace
participant. V1/v2 input vocabularies and single-predicate work budgets stay intact.
Seven BDD scenarios include 2,304 independent conjunction/domain/metadata cases,
all interpretation epochs and safe-version edges, mixed source/workspace/configuration
scopes, ABA, replay/profile separation, bounds/foreign/duplicate subjects and optional
capture. Four native regressions cover full descriptor thresholds, malformed-version
admission, older-profile smuggling, private capture drift and legacy work bounds.
Recipient observation authority, actual mutable-definition/association producers,
other consumer integration and the complete public F10/F11 facade remain open.
Configuration-read checkpoint: 550 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates passing in each profile. Rough planning estimate: 86%,
not measured coverage. The full engine implementation goal remains active.
Native settings-with-reads now has a separate v2 opaque preparation, job kind and
typed refusal profile. Caller v3 source/annotation/workspace/configuration reads
conjunct with, never replace, automatic settings contributors. One actual staging
boundary validates both before no-op classification and current-root candidate
installation. Outer intent/read envelope bounds precede copying; complete wrapper,
retained read rows and native core are covered by the plan ceiling. The two finite
work profiles remain explicit. Private count/profile seals prevent lost footprints
from becoming unrestricted commands. Native and additional-read refusals remain
distinct; matching retries keep original intent and never resample dependencies.
Six BDD scenarios include 1,792 independent mixed read-set/producer/changed-no-op
cases, mandatory/disjoint frames, replay/profile separation, protected source reads,
foreign/duplicate refusals and capture equivalence. Five native regressions cover
full intent/plan thresholds, early bounds, hold accounting, capture/profile/count
loss or valid-plan substitution, exhausted no-op counters and interrupted actual
settings installation without a fabricated receipt. V1 remains without extra reads.
Recipient grants/dynamic routes, mutable settings definition/association/layer
lifecycle and remaining engine families are still open.
Dependent-settings checkpoint: 561 default workspace tests pass in debug/release,
with formatting, strict all-feature Clippy, documentation tests and both strict
Unicode reference gates in each profile. Rough planning estimate: 87%, not measured
coverage. The full engine implementation goal remains active.
Bounded source range/line queries and typed byte/line/scalar/UTF-16 conversions now
execute against exact live native source revisions. Replies preserve LF/CRLF/CR,
trailing empty lines and scalar boundaries without normalization, clamping or
incidental document-wide metadata. Scan/content limits refuse complete products,
not partial prefixes. An exhaustive v1 conversion interface and content-free
query/conversion boundaries are tested with an independent line oracle and
native-index inverse laws. Opt-in logical-view registration now supplies fresh
typed identities, detached birth and exact attachment/selection lifecycle fences.
Bounded explicit positional normalization preserves direction/primary/affinity;
line/document endpoints run without source-write permission. Source publication
and reversal capture/revalidate all affected view signatures and stage their
current positional maps with actual source/annotation/history companions. Equal-
byte view effects do not fabricate source revisions or undo entries. Disposal
detaches views atomically, without UI/focus/rendering policy. Unicode 18 boundary
conformance, remaining ordinary-edit/selection invocation assembly, protected footprints and
complete recipient/discovery products remain open.
Literal selection replacements now execute with explicit mapped/carets-after
policies in that same atomic source/annotation/view/history path. Preparation
freezes exact source/view/selection intent and bounds the complete expected-plus-
inserted payload before copying. Final validation rejects stale peers and policy,
while owned intent drift fences the owner. Equal-byte caret effects do not create
source revisions or undo entries. Literal invocation admission now retains the
original exact context/text/policy and preparation or refusal in the common ledger;
retries do not reacquire selections, forgotten results remain spent and execution
compares the retained capture before current validation. Private capture loss or
policy drift fences Unknown. Selection-only set/endpoint requests now use the same
one-shot ledger with exact original raw-list/primary or endpoint/budget intent,
frozen candidates and typed retained native update/refusal results. They share the
direct selection publisher, preserve compatible later comments/history and require
no source-write permission. Stale targets refuse; private intent/candidate drift
fences Unknown. Complete retained selection snapshots, other selection forms and
ordinary-edit/discovery facade remain open; no UI input behavior moved into core.
Explicit simple line edits now lower indent/outdent and chosen generated newlines
into shared source/annotation/view/history publication. Target lines, prefix bytes,
actual tab columns, unit bytes, native descriptors/payloads and coordinate scan
extents have finite bounds. Exact line endpoints, minimal suffix replacement and
per-origin newline carets preserve the adopted semantics. Frozen request/profile
drift is revalidated before publication; no keymap or grammar dependency is added.
Explicit line invocation/replay now uses the common source runner with retained
original settings, work-profile snapshot and preparation/refusal. Exact replay
precedes state acquisition; execution checks the independent capture before source
publication. Typed line refusals retain preparation meaning, including an absent
profile that is activated later; unknown integrity failures still fence the owner.
Configured settings/interpretation leases, recipient discovery and production Unicode provider selection
remain open. This is trusted-owner implementation, not full command qualification.
The next native slice now owns exact-source Unicode boundary captures and retained
tables under finite live/lifetime/count/byte caps. It validates canonical grids and
significant intervals, rejects stale/foreign/retired contexts, preserves provenance
and permits protected local cleanup without source/history publication. It does
not segment or establish Unicode conformance.
Provider semantic truth remains a declared host premise; native production-provider
qualification and recipient release are not counted as closed.
Direct native command consumers now implement strict directional grapheme/word
motion and deletion over that retained evidence. Collapse/extension and explicit
versus implicit deletion preserve the adopted semantics; finite work and final
source/view/evidence checks precede publication. Movement shares the selection
publisher; deletion shares source/annotation/all-view/history publication and
carets-after mapping. Closed single-entry typed capture dispatch replaces separate
optional family slots. This is conditional command execution, not segmentation
qualification or completion of the authorized command/discovery/replay facade.
Unicode command replay now has native shared-ledger implementation for both
operations. Original evidence identity and all command/work intent stay bound;
matching retries cannot reacquire or substitute equivalent tables. Typed motion
updates/refusals and source commit facts survive stop/forget/undo/later source
changes with one execution only. Current eligibility is still checked at the one
eligible run. A demonstrated private kind/job mismatch regression tightened the
source runner to explicit legal pairings; detected drift fences Unknown. Full
recipient/discovery/result contracts and provider conformance remain open.
The separate `vellum-unicode-reference` Rust adapter now verifies jar/tool pins and
runtime versions, bounds/stages the reference worker protocol, and passes every
pinned grapheme/word corpus plus native capture/command/replay/deletion/undo
composition. The corpus exposed eight valid combined tables whose significant
word vectors can split graphemes: eligibility now refuses only the incompatible
requested word-start/end command, not the table or grapheme operations. No data
is intersected or dictionary-tailored. Reference qualification is a strict explicit
gate, not production/incremental provider selection, a sandbox or independent proof.
Shared attachment candidate preparation now implements the existing typed mapping
rules with bounded fanout/work and separate source/placement change classification.
The owner now acquires actual authored records and publishes source/annotation/
history companions together, with independent body/target versions, membership
restoration and retained compound invocation facts. This is trusted-owner Rust
implementation coverage, not closure of recipient release, annotation persistence,
final retirement, general retained target mapping or the full family gates.
Coherent immutable annotation capture and a separate strict v1 reference-JSON
codec now execute with bounded parsing/output and explicit complete host aliases.
Both point meanings survive; decoded counters/identities are data, not edit tokens.
Explicit export now uses the same destination writer/evidence ledger as source
saves, with bounded shared retained artifacts and original encoding contexts.
Source baselines cannot be published or superseded by annotation exports, and
stopping artifact delivery does not cancel admitted effects or release their holds.
Native codec-to-filesystem export is tested. Initial ordinary annotation binding
now enforces typed ownership and exact-basis known distinction under one provider
registration. Source admission/dispatch/baseline publication independently recheck
the pair. Native comparison catches introduced hardlinks without refreshing the
pinned condition; it remains best-effort host evidence, not an alias oracle.
This does not close cross-provider comparison, annotation rebind/unbind,
generalized annotation save obligations, native load/restoration,
before-codec funded capture, final lifecycle or recipient release.
Explicit annotation baseline adoption/comparison now runs against the real Rust
writer/artifact ledger, with original association, newest eligible dispatch serial,
separate token enrichment, current exact-byte comparison and producer retention.
Native codec/filesystem composition is tested. Native descriptor sizing is not
the reference JSON root-envelope reservation. Ordinary native save now atomically
admits a current transient capture, artifact, shared-writer task and worst native
descriptor/counter claim. All current Rust publications preserve held counter units;
prepaid installation transfers the claim without a second capacity demand. Safe
cleanup releases only the promise, preserving independent writer/receipt retention.
Native filesystem composition, counter-floor interference and fault-safe cleanup
are tested. Transient codec preparation remains host-funded; before-codec funding,
wire/root-envelope laws and full public/lifecycle contracts remain open.
Required acknowledgment follows the captured request rather than provider
capability. These are implementation slices, not new design acceptance or full
persistence/public-facade closure.
The missing owned-read prerequisite now executes in Rust: independent registered
reader and original ordinary binding, bounded ordered receipts and conflict
witnesses, explicit stop/quiescence/delivery drain, shared read/write launch capacity
and immutable retained byte captures. The macOS adapter and strict codec compose
without source/comment/baseline publication; invalid input remains intact. Typed
decoded data is not restoration authority. Initial hydration now uses a separately
provisioned typed restorer, explicit associations and managed source witnesses. It
verifies the original read and lifetime-empty inventory, installs records/floors
without authored history and pins original provenance. Unused slots retain their
document ownership. Native read/re-export/save, late contradiction, imported
incarnation preservation and counter/integrity refusals are tested. Explicit loaded
baseline adoption now retains the actual consumed read, original linked encoding
context and historical restored set, with no fabricated write acknowledgment.
Same-owner attachment and baseline descriptors are bounded; admission/dispatch
preserve original loaded evidence even after written-baseline replacement. Exact
raw-byte comparison distinguishes semantically equal noncanonical JSON. Producer
discharge after an actual current written checkpoint now executes atomically with
read retirement. Original delivery holds, actual loaded references, unresolved
destination work and checkpoint contradiction block release. Both attachment
endpoints are charged before assignment and detached together, with a fresh
incarnation on reattachment. These protected controls preserve writer evidence,
authored history, publication counters and spent origins. Uncheckpointed disposed
producer discharge, generalized historical-source import, public read
invocation/release and atomic-resource read profiles remain open. This does not
qualify the historical read/load
reference as a complete Rust implementation.
Retained native disposal and independent source/annotation binding release now
execute with exact-root preparations, current promise checks, lifecycle/pair
counters and the common publication floor. Current content/edit/capture/launch
paths reject disposed owners; protected history retirement and historical
artifacts/receipts remain usable. Keys remain reserved until actual safe writer
disposition permits release. Complete bindings/baselines move into fixed retained
document slots, preserving producer pins and spent incarnations. Explicit native
provenance release now requires absent keys, discharged history and safe actual
writer disposition; it is not inferred from key release or discard. Disposed read
retirement scans actual peers and delivery holds; claimed ordinary writer cleanup
removes task/claim/artifact atomically under its independent safety/drain guards.
Both retirement orders, no-save-checkpoint cleanup, cold archive framing and
fault/counter-safe controls execute in Rust. Final storage reclamation, live
rebind, contested recovery and recipient/lifecycle companions remain open.
The adopted ambiguous absent-BOM output refusal is now enforced before
source save issuance, without rejecting the valid in-memory edit.
The user selected staged replacement with safety refusals, without in-place
fallback. A separate macOS local-filesystem module now executes that narrow path,
including verified bounded xattrs, explicit unsupported-metadata/link refusals,
best-effort conflict checks and native service-time instrumentation. This is not
atomic conditional writing, universal crash durability or platform-wide conformance. It
does not implement the complete editor command facade, qualify the
production substrate interface, or change any design-item acceptance below.

Current adopted baseline: specification 0.224/model 0.288. Full frozen verification
passes 802/802 checks over 1,018 inputs, with source/worker audit, exact root byte
comparison and an integrity-checked retained archive. The one-shot publication
mechanically updated 49 input files (13 new), removed none, and preserved the
previous 0.219 baseline at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-6RS49n`.
The adopted archive is `.verification-evidence.YovcKv`; its manifest SHA-256 is
`eaf3fbf326efd058039f93e348d13f646a2e247f282a7303b345fb7cf1d62bd3`.
The exact root report SHA-256 is
`7f1eb33a925df255a9d2925307a6f7e8afa8e63b82fa898f4152889712e1a189`.
This adopts the qualified predecessor's facade/retention/funding corrections,
not the later inspection/control drafts. Publisher
`/private/tmp/vellum-v0224-adopt.mjs` is consumed and must not be replayed.
The separately frozen 0.227 successor is terminal and failed, not adopted. The
corrected 0.230 candidate is still under full frozen verification (session
71453), but has reported shutdown-composition and ordinary-source-save command
timeouts. It cannot qualify as a passing full run. Let the same run finish; do
not restart it or treat partial passes as adoption.
Whole-deliverable acceptance remains twelve of seventeen (71% by count).

Current staged successor: 0.233/model 0.297, frozen for selected verification at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-xeZ0IC`
(1,020 matching inputs; editable source ends `vellum-proof-inputs-vx4ChR`).
The same original cut now includes registration bases, reads and event owners,
including actual parent funding, context/entry/continuation selectors, original
commands, receipt pins and technical deliveries retaining old queue heads. It
returns no registration rows, descriptors, locators or visibility packets.
Three added lifecycle groups and the complete 54-group reference composition pass;
the exact 104-mutation registration passes (terminal 25528, 140,719 ms within its
180-second deadline), with retained output
`/private/tmp/vellum-v0233-registration-HEgojp`. Seven exact
selected regressions pass on this corrected snapshot (terminal 60360), with
outputs retained in cEe4sM, XmGWb5, DYXsRT, dUQFuD, NTz5aj, HJFmuf and akigC4
under `/private/tmp/vellum-v0233-registration-`. No qualification/adoption or
whole L09 closure is claimed. Cold initialization passes all 143 entries and both
orders (terminal 50009). Composition fixture registry ceilings are derived from
their existing exact charge functions, rather than unrelated 64-MiB test ceilings;
engine funding/refusal rules and existing regressions are unchanged.
The retained EIkXJF predecessor failed its mutation construction (terminal 13249):
the basis test dereferenced an absent row before asserting original identity.
The corrected test asserts both rows exist before reading their conditions;
neither the TypeError nor the failed run counts as a semantic detection. Its
seven regressions passed separately (terminal 30435), but that predecessor is
not the current qualification candidate. Do not poll terminal 13249/30435/60360/25528.
Six owner families remain: discovery, history, analysis, diagnostic capture/drain
and publication observers. Complete prepared-plan dependencies and shutdown
service/cutoff/report/control retirement/public F18 also remain open.
Next engine-lifetime work must address the existing missing whole-owner retirement
ports in history inspection, analysis registries and native publication observers,
not merely add more metadata. Their current code retires individual contexts/
tasks/outcomes, while native publication usage explicitly reports whole-owner
retirement unsupported. Follow existing native registry discharge patterns and
preserve independent retained read contexts, effects and historical evidence;
stopped/empty child counts alone are not a parent discharge certificate.

Staged 0.232/model 0.296 is retained for selected verification at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-P83Krv`,
with all 1,020 captured input hashes matching. Its editable source is
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-C5mal9`.
Do not edit the retained selected-test source. Three additional integrated groups
pass for settings read/write/definition/lifecycle leases and scoped prepared plans,
settings-read preparations/context/target/origin selectors, and serialization
conditional bases with actual slot-rooted scoped plan edges. They preserve
independent stop, protected retirement and spent issuance without returning
configuration values, declarations, visibility packets or raw origin identifiers.
The complete reference composition passes 51 groups. The extended exact predicate
and all 84 semantic mutations pass (terminal 57212, 109,220 ms within its bounded
180-second deadline); retained output is
`/private/tmp/vellum-v0232-registration-F4lvZF`. Eight exact selected settings/
serialization/initialization/boundary registrations also pass (terminal 47815):
ordinary settings bases, scoped settings, settings activation/inspection,
serialization bases/scoped serialization, cold initialization and engine boundary.
Their retained output directories are nKBVSx, LRIEQd, N4qW7w, 61GtGd, Lp4If8,
bSG82I, EklpEH and x7XIgQ under `/private/tmp/vellum-v0232-registration-`.
Do not poll either terminal session again. The runner
`/private/tmp/vellum-v0232-registration.mjs` preserves generated output and checks
all 1,020 frozen input hashes before/after each selected execution. This closes
the selected settings/serialization cut slice, not a full candidate or whole L09.

Initial reverse imports into the coordinator exposed eager declaration cycles in
selection/ordinary arguments (terminal failures 78993/85274). A first lazy-selection
repair merely moved the failure. That argument edit was reverted; the three new
helpers instead read the original exact owner-specific frame from their actual
private parent, which is already their store key. No caller-chosen budget, new
registry, authority or engine module was added, and the original argument source
and module graph are preserved. All 143 isolated entries and both aggregate orders
pass after the correction (terminal 65951); construction failures are not semantic
mutation evidence. Settings preparation fixture refusals were missing existing
companions/current-dependency facts; the fixture supplies those facts explicitly,
without weakening engine gates.

The scoped pool sections do not inventory every invocation plan: unscoped source
serialization without a basis pool and other prepared-plan families remain
explicitly unproved. Registration bases/reads/events, discovery, history, analysis,
diagnostic capture/drain and publication-observer owner sections remain unresolved,
as do service/cutoff/current-authorized report/control retirement/public F18.
No whole L02/L09 closure, full successor qualification or adoption is claimed.

Staged 0.231/model 0.295 is editable at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-IQ5qHE`.
It extends the same original-control cut to actual source contexts,
annotation preparations/contexts, version-set preparations/parents/member aliases
and source-search children, plus original invocation-slot metadata and other
attached writers. No new pool modules or public APIs are introduced. Three
integrated lifecycle groups pass: independent prepared/installed annotation
lifetimes; version-set parent/member identity; and source worker/search holds
surviving stop until protected discharge. Captured old cuts remain immutable,
new cuts require protected retirement, and spent issuance is not refunded.
The combined reference/mutation run is terminal (session 87711, exit zero):
48 integrated groups and 68 semantic mutations, with construction failures not
counted. Its exact extended registered predicate also passes (terminal session
26679, 86,423 ms), with generated stdout/stderr/result retained at
`/private/tmp/vellum-v0231-shutdown-registration-150K28`. Do not poll either
terminal session again. The frozen 0.230 runner's 60-second deadline is unchanged.
The measured successor duration exceeds that deadline, so staged 0.231 allows
explicit bounded per-registration deadlines (maximum 180 seconds) and records
them in the report. Only shutdown composition and ordinary source save select
180 seconds; every other registration retains its original 60 seconds. No
semantic assertion, mutation, count, engine limit or required tool is removed.
The source-save exact registration passes in 81,676 ms with all 27 mutations
(terminal 67329; retained output
`/private/tmp/vellum-v0231-shutdown-registration-0veich`). Do not poll it again.
Both measured registrations exceed the old 60-second deadline without weakening
their semantic predicates, and source save passes within the new finite deadline.
The boundary
exact registration passes in 1,966 ms (terminal 79140; retained output
`/private/tmp/vellum-v0231-shutdown-registration-KyjcOk`), preserving the 147-module
engine/UI/integration boundary and all ten mutations. Cold initialization passes all
143 isolated entries and two aggregate orders (terminal 50506). These selected
executions do not qualify or adopt a full candidate.
Full verification/adoption and complete owner inventory/
service/cutoff/report/control-retirement/F18 remain open. The staged model and
specification describe these roles and retain
`completeOwnerObligationsProved=false`; this scoped check does not establish
complete invocation-plan dependencies, all selector/issuer variants or shutdown.

Full 0.227 finished 802/803 checks over 1,020 unchanged inputs with complete tool
execution. Its only failed check is the F08 facade aggregate predicate, which
still expected 146 engine modules although its actual output and the dedicated
boundary registration both report 147 after the canonical funding leaf was added.
The corrected predicate retains every other assertion and accepts that existing
semantic output in an audit-only evaluation; the failed report remains failed.
Its report SHA-256 is
`2cf394fd53e1da8b0913b5403f5846492fa6c0ed5bda5c91ac065d43db6ce5bb`;
worker `/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-9ejDPz`.
Do not poll terminal session 49790 or adopt that report.

Corrected 0.230/model 0.294 is now frozen at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-H3VvVg`
with 1,020 matching captured inputs and the F08 aggregate expectation repaired to
147. Its full pinned run is live in session 71453; expected inventory remains 803
checks and 301 SMT conditions in fifteen manifests. Do not edit those frozen
inputs or restart the run because an observation yields. This supersedes the
unqualified retained Caegaa candidate below, whose aggregate expectation is stale.
Full qualification/adoption remain unproven until terminal success and endpoint
audit. The adopted root remains the separately qualified 0.224 baseline.
The 0.230 endpoint/manifest/SMT and thirteen selected-registration auditor is
prepared at `/private/tmp/vellum-v0230-audit.mjs`; its preparation checks source
identity and evaluator syntax only, not terminal qualification. It includes the
previously missed F08 aggregate predicate.

Working 0.230/model 0.294 at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-w1TwtG`
extends the original obligation cut to source/annotation readers, baseline
reservations and codec descendants. It preserves actual stream/candidate/artifact/
preparation/claim/reservation/task identities, annotation restoration and writer
provenance, and independent stopped/quiescent/conflict/refusal facts. Baseline
installation can consume a hold while retaining a producer pin; later legitimate
claim deletion can leave a marker-only pin. That pin remains explicit, with no
invented deleted producer metadata. Eight persistence groups pass, including
actual cancellation followed by authentic quiescence and producer retirement,
qualified baseline installation followed by superseded task retirement, pending
codec preparation discard and a small cut refusing before enumerating 24 imports.

Read-only challenge identified premature full-map identity arrays before bounded
copy refusal. The correction iterates private maps directly and retains tuples
only after bounded row acceptance, including connected observation/admission and
writer dependency helpers. Called usage/funding reductions now use constant-
scratch loops with unchanged returned accounting. Reviewer readback found the
reported construction issue resolved at this scope; it ran no tests. The declared
control envelope is conservatively `4I + 5R + sum(M) + S(I + R)` plus the existing
fence, not a physical allocator/RSS claim. Established failed codec imports are
retained separately from complete and aborted states.

The construction issue also affects staged 0.228/0.229 cut predecessors. Retain
them as historical scoped evidence, not adoption candidates without this
correction and its bounded-enumeration control. The adopted 0.224 baseline has no
such cut extension and remains unchanged.

The first integrated attempt (terminal 15428) failed on a non-unique quiescence
mutation anchor; the next (terminal 31001) rejected a mutation that produced a
cloning API TypeError rather than a semantic assertion. Both construction defects
are corrected without changing the intended mutations or their assertions. Neither
failure is counted as detected semantics or successful qualification. A third
attempt (terminal 59504) correctly refused to count a cloned-owner mutation whose
test dereferenced null before asserting actual owner membership. The test now
asserts that membership explicitly before reading usage, preserving the intended
identity requirement without counting a TypeError. The exact
fifty-nine-mutant composition predicate now passes on the corrected graph:
eleven readiness, ten consent, seven writer-cut, nine observation-cut and eight
persistence-cut groups (forty-five total). Thirteen selected persistence,
observation/admission/decoder, boundary and cold-initialization regressions pass
across terminal batches 82337 (five passing rows before its documented stale-anchor
failure) and 40702 (eight remaining regressions plus the exact composition
predicate). Do not poll either terminal batch again. No full 0.230 success,
qualification, adoption or whole L09
acceptance is claimed. Other owner obligations, service/cutoff/current-authorized
reports, control retirement and public F18 remain open.
The persistence regression batch (terminal 82337) passed boundary/initialization,
native source save, ordinary annotation save and loaded-annotation checks, then
stopped on a stale attachment-charge mutation anchor after the constant-scratch
reader-usage rewrite. That mutation now removes the same attachment charge from
the new accumulator initialization; its assertions and semantic target are
unchanged. A follow-on batch covers that check and the not-yet-run dependent
regressions plus the corrected integrated shutdown predicate. Do not count the
failed construction as a detected mutant or complete regression run.

The corrected 0.230 candidate is retained unchanged at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-Caegaa`,
with all 1,020 input hashes matching both that endpoint and the tested working
tree. No full run has started there.

Working 0.229/model 0.293 at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-DefklF`
extends the same original retained cut to actual execution/native observation
contexts and observation-admission pool/reservation roles, rather than adding a
second snapshot lane. Internal cut names now describe obligations. The original
owner rows distinguish absent attachments from retained/uninspected obligations.
Private producers preserve actual ticket, producer/task, progress/wait/after,
pool/reservation/context identities, unknown setup faults, ended retained waits,
decoder workspace and independent occupancy/spent accounting. All rows compete
for the one aggregate result ceiling through an original owner-specific active
frame. Nine added control groups and twelve new semantic mutants pass in the
selected composition's exact predicate: eleven readiness, ten consent, seven
writer-cut and nine observation-cut groups, forty-four mutants total. Read-only
challenge found no concrete defect at that scope and ran no tests.

Cold initialization then exposed two previously eager declaration lookups through
the newly connected import cycle (policy declarations and input forms). Both now
resolve at call time without removing declaration validation. The 143 isolated
entrypoints, both aggregate import orders and existing startup negative control
pass after the fixes. Startup failures were not counted as semantic mutants.
The exact composition predicate and all nine selected ordinary-save, retained-
lifecycle, observation/admission/decoder/argument, initialization and boundary
regressions pass against the corrected coherent graph (terminal session 86081;
do not poll again). No full qualification, adoption or whole
deliverable acceptance is claimed; other actual owner obligations, service/cutoff/
report/control retirement and public F18 remain open.
The corrected candidate is retained unchanged at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-nqWVYY`,
with 1,020 captured inputs matching their hashes. No full run has started there.
The adopted root still exactly matches its separate qualified 0.224 input report.

Working 0.228/model 0.292 at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-9pu7PR`
adds the actual shared writer's retained obligation cut under the original native
shutdown control. Seven groups preserve actual opaque task/domain/read-owner
identities, independent outcome/disposition/quiescence facts, local/peer/unattributed
ownership, bounded refusal, immutable retry and protected cut retirement. Ten new
semantic mutants are assertion-detected; the exact registered composition predicate
passes eleven readiness, ten consent and seven writer-cut groups plus all thirty-one
mutants. This includes no module-construction failures as semantic detections.
The control envelope is now `4I + 3R + sum(M) + S(I + R)` plus the existing fence,
funding the retained cut and separate bounded construction/lowering workspace.
Read-only challenge subsequently identified an omitted primary source-reader
dependency (annotation readers alone were insufficient). The working correction
includes that actual reader and native domain with an artifact discriminator,
asserts both source and annotation identities, and adds an omission mutant. Its
updated thirty-two-mutant exact predicate passes (terminal session 51907).
Read-only reviewer readback confirms the omission is resolved, with no new concrete
finding; it did not run tests or provide independent certification. No unsafe
teardown was reproduced or implemented by the omitted edge.
Specification §17.4.5 and the model describe this as internal control metadata,
not a recipient report or all-owner inventory. Other owner-role/dependency cuts,
service/cutoff/current-authorized reports, control retirement and public F18 remain
open. Boundary, isolated-entrypoint, ordinary-save parent and native retained-
lifecycle exact predicates pass. No full 0.228 qualification or adoption
is claimed. The frozen 0.227 run remains separate; do not modify its inputs.
The corrected 0.228 candidate is retained, with all 1,020 input hashes matching,
at `/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-UmHupz`.
No full run has started for that candidate. Keep it unchanged for subsequent
qualification; the source working directory above is not the frozen endpoint.

Working 0.227/model 0.291 at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-lVvdsO`
repairs two reproduced cross-path held-byte defects in the 0.225/0.226 inspection
extension. A pre-prepared native write committed with peak 524,288 against ceiling
524,287; an annotation-read provenance attachment reached 196,608 against 196,607.
Both duplicated final-admission calculations omitted the live inspection lease.
Writer usage, native commit and provenance attachment now share one pure
`source-save-funding.mjs` ledger, including links/tasks/claims/inspection. Eleven
readiness groups, ten consent/disposition groups and twenty-one assertion-detected
whole-tree mutants pass, with exact below/equal/above controls and post-unwind retry
for both paths. Loaded-annotation regression passes nineteen controls/seventeen
mutants; the updated 147-module boundary and 143 isolated entries pass. Read-only
challenge found no concrete remaining defect in the fixes. Earlier 0.225/0.226
candidates must not be adopted on a green suite lacking these cases; frozen
running inputs are unchanged. Complete owner-obligation/dependency inventory,
service/cutoff/report/control retirement and public F18 remain open. No whole
deliverable acceptance changes: twelve of seventeen (71% by count).

Full 0.224 has now passed (terminal session 38273): 802/802 checks, 1,018
unchanged inputs and 301 SMT conditions in fifteen manifests. The retained worker
is `/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-yzlK6p`;
report SHA-256 is `7f1eb33a925df255a9d2925307a6f7e8afa8e63b82fa898f4152889712e1a189`.
The current 0.224 source/worker endpoint, manifest/SMT and ten selected actual
registration audit pass. Before publication, root exactly matched adopted 0.219;
the guarded 0.224 adoption has since completed as recorded above. It does not contain
the later inspection extension's known defects. Do not poll session 38273 again.

Full 0.226 is also terminal, but failed (session 87778): 802/803 checks, 1,019
unchanged inputs and complete tool run. Its sole failing check is the ordinary
save parent request's explicit writer-envelope expectation: the new 43-byte
maximum-width inspection metadata field was absent from that expected frame.
The 0.227 checker now includes that actual field without changing the production
charge or weakening the equality assertion. The report SHA-256 is
`645004f4376d52c5ed5ad174ef6299be18d92779b6736858bcae06e947e16454`;
worker `/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-ZIuZI5`.
That failed run is retained, not repaired in place. Independently of its stale
expectation, the two reproduced inspection-credit defects prohibit adopting 0.226.
Do not poll session 87778 again. No full verification process is currently live;
0.227 must receive its own coherent frozen full run.

The corrected 0.227 batch is now frozen at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-9NAubT`:
1,020 inputs, expected 803 checks/301 SMT conditions. Full session 49790 is live
with the pinned tools and local TLC listener permission. Its twelve-registration
endpoint auditor `/private/tmp/vellum-v0227-audit.mjs` prepares successfully but has
not audited a full report. Focused source-save regression passes thirty controls/
twenty-seven mutations; the corrected ordinary-save parent suite passes eleven
controls/fifteen mutations. Exact shutdown, 147-module ownership and 143-entry
initialization predicates pass. This is the current full run, not a restart of
any terminal predecessor. No passing/adoption claim follows from its initial log.

The older full 0.223 run is now terminal exit 1 (session 32980): 801/802 checks,
1,018 unchanged inputs, complete tool run. The only failure is bank 10's known
`registration-event-retirement-parent-refund-omitted` construction anchor, repaired
in 0.224. The report SHA-256 is
`f162c057dc3e0b8ec134340f7024b3bcabce81d0f2715a1b1abd3f3109fa23ab`;
the retained worker is
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-JIRQWn`.
This is a complete failed run, not an adopted pass. Do not poll session 32980 again.

Working 0.226/model 0.290 at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-1PY9KI`
connects readiness to an actual parent-funded original consent and fence, then
to a dedicated native F17 disposition installer. The generic post-fence disposal
refusal is unchanged. Exact original control/writer/intent, current independent
save/discard authority, actual qualified output, native writer/certificate guards,
root constraints and once-only progress are composed rather than represented by
the draft's wrapper flag. Nine readiness groups, ten consent/disposition groups,
eighteen assertion-detected whole-tree mutations and the exact registered predicate
pass, including both source/annotation completion orders on one actual authority,
writer and control. Fence, source-save, ownership and cold-entrypoint regressions
pass. Existing read-only second-agent challenge found no concrete defect in this
delta; this is not independent certification. A final-guard-only identity mutation
survived because the request lookup independently enforces identity; it was not
counted as detection. The coordinated removal of both guards is detected.
Complete owner-obligation/dependency inventory, service/cutoff/report assembly,
control retirement, public F18 and full frozen qualification/adoption remain open.
This closes the focused native consent/disposition bridge, not all L09. The fixed
acceptance count remains twelve of seventeen (71% by count).

The consolidated 0.226 candidate is separately frozen at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-HFU6V5`:
1,019 inputs, expected 803 checks and 301 SMT conditions. Its endpoint auditor
`/private/tmp/vellum-v0226-audit.mjs` compiles and prepares eleven selected actual
registrations, but has not audited a full report. The full 0.226 frozen launcher
is now running as session 87778 with the pinned tools and required local TLC
listener permission; initial construction/census checks pass. No full 0.225 run
is queued: it is consolidated rather than verified as a redundant standalone batch.
The current 0.226 focused regressions include sixteen retained-lifecycle controls/
eleven mutations, thirty source-save controls/twenty-seven mutations, the original
twelve fence controls/fourteen mutations, the 146-module boundary and 142 isolated
entrypoints. The new control block was mechanically placed after established
native functions to preserve older exact mutation anchors; no predicate or assertion
was weakened. The current exact shutdown predicate passes again after relocation.
Frozen 0.223 and 0.224 verification remain separate from this candidate; their
results cannot qualify changed 0.226 inputs.

Working 0.225/model 0.289 now supplies the actual native shared-writer readiness
prerequisite for shutdown at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-EjlrUw`.
Nine control groups and nine assertion-detected whole-tree semantic mutations,
including the exact registered suite predicate, pass. The relation independently
derives source/annotation qualification, matching pre-admitted saves and conflicting
destination work; it cannot accept a caller's ready flag as evidence. Its temporary
workspace is debited against the actual writer and refunded on every exit.
Read-only challenge exposed two concrete defects, both now corrected and covered:
supplier reentry could fault the domain without changing the root, and source
serialization could allocate beyond a smaller comparison ceiling before refusal.
Actual domain health is now revalidated after suppliers and before returning;
exact source byte length, including BOM, is preflighted before suppliers or
serialization. Removing either safeguard is assertion-detected. This is not full
qualification/adoption, accepted shutdown, a complete owner inventory or a
post-fence disposal capability. L09 still requires the integrated original-control,
consent/disposition and bounded service/cutoff/report composition. Accepted
deliverables remain twelve of seventeen (71% by count); a helper is not another
deliverable.

Adopted specification 0.219/model 0.283 implements the user-selected typed retained-outcome
receipt facet and reconciles the exhaustive versioned executable-enum policy.
The full frozen suite, source/worker/root endpoint audit and retained archive
integrity check pass. Focused receipt/legacy checks and read-only second-agent challenge are complete;
the returned-invocation correlation guard and stale U01 status are reconciled.
This accepts N02 for the eight selected native receipt kinds, not all future
operation results or all of L02. L13's design acceptance review can now
use the resolved enum policy, without inventing a production package/support matrix.

A separate staged 0.220/model 0.284 fixes a reproduced L09 diagnostic-drain
admission gap. New task issuance and first submission now serialize with the
actual ordinary fence; matching retries and pre-admitted service/discharge retain
their semantics. All original controls/mutants and the targeted additions pass;
read-only challenge found no concrete defect. It leaves the adopted 0.219 inputs
unchanged. Its full qualification and endpoint audit now pass as recorded below;
root adoption remains separate. This closes no whole L09 claim.

That staged batch also reuses the existing F17 outcome/event reference through
the actual-domain observer and retained-receipt facet, covering fourteen existing
forms on both routes. The existing cases/unsafe alternatives and exact registered
check pass; full qualification and endpoint audit now pass, with adoption still
separate. It adds no engine mechanism and
does not claim a complete F19 context sum or whole L02 closure.

The combined 0.220 candidate's first frozen qualification terminated unsuccessfully:
453 of 795 checks passed; all 342 failures were TLC local RMI listener denials
under the sandbox, with unchanged source inputs and no other failed checks.
The failed report is retained separately (SHA-256
42a9009d3b900d62b0fca2b70367634281c477f9739a7d3f0de13204180164c5).
The pinned Evidence model passes with local listener permission. The second full
run of the unchanged 1,006-input candidate used that permission and terminated
successfully (session 67748): 795/795 checks, 299 SMT conditions in fifteen
manifests, 168 catalog elements and thirteen interfaces. Exact endpoint and
registration audit passes against source and retained worker
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-o32vgj`.
The report SHA-256 is `f53f23bef07a92fa46cc0e44d1fcfbb640f5821496520d9cd3d5f8cf9336552f`.
The audit wrapper needed two portability/range repairs: its inherited one-check
range now stops before the separately validated new event check, and an extracted
dynamic import uses a file URL when evaluated in a data module. No suite input,
expected semantic predicate or passing report was changed. Adopted 0.219 remains
unchanged; the qualified predecessor is retained, not promoted over its successor's
known spent-origin funding repairs. Do not poll terminal session 67748 again.

A separate 0.221/model 0.285 candidate now binds F20 source-load native
effect-attempt requests to their actual task across both routes. Matching requests retain
the task without recapture/submission; changed intent, foreign scope and retired
origins refuse. It also repairs a reproduced source-reader lifetime funding gap:
128 admit/cancel/retire cycles retained 386,432 bytes of spent origin keys beyond
a 350,000-byte live-task envelope. The parent now separately reserves the safe
maxIssued × maxCaptureBytes lifetime bound; invalid operands and overflow refuse
without parent mutation. Nine controls/ten targeted mutations, the exact harness
registration, native-read regression, owner-attachment controls/selected mutation,
127 cold entrypoints, 131-module boundary and conditional SMT induction/negative
control pass. Read-only challenge's operand-validation finding is repaired.
This is unqualified staged work, not a whole F20/L02 or physical-accounting proof.
The adopted 0.219 report and running 0.220 candidate retain the known funding gap;
their earlier passing checks do not establish the newly tested lifetime bound.
The analogous annotation-reader lifetime store is now audited below;
the shared writer already has an explicit spent-origin reservation. No unrelated
accepted deliverable is silently reopened or signed off by this finding.

That same staged 0.221 candidate now supplies §16.1.4's annotation-load original
request/task relation. Eight controls/ten guard mutations, eight cross-route
deliveries, 94 denials and both original transcript/native-load regression suites
pass. A real source edit invalidates the original candidate's applicability while
leaving its task retrievable without refreshed intent or submission. A baseline
counterexample confirms the analogous lifetime gap: 898,432 origin-key bytes
remain after 128 retirements against a 760,000-byte task envelope. The validated
separate parent reservation fixes it using the same conditional SMT law.
Read-only challenge found no concrete defect at this scope. Full qualification/
adoption, dynamic discovery, provider dispatch and receipt/event/control assembly
remain; both ordinary-save retry bindings are still open. No all-F20 acceptance
or change to the twelve-accepted/five-open deliverable count is claimed.

Review clarified a material scope limit for the earlier staged load adapters: this
relation recovers an existing native effect-attempt task. It does not establish
ownership/admission of the originating parent operation, parent-command
deduplication, or authorization/reconciliation of another attempt. A changed
attempt label can still admit fresh native work under ordinary gates. Those
parent/effect/retry composition obligations remain open under requirements §7.4;
neither task recovery nor a shared origin string closes them. No homogeneous
global issuer is required. The unregistered F21 source-save prototype had the
same limit and was removed from the working candidate, with its scratch copy
retained outside the formal inventory. No save binding is accepted by that removal.

The working 0.221 source now addresses that finding for F20 load commands:
first native admission atomically owns a parent-operation record with complete
original context, immutable request and actual child task. The native owner is
the sole retry owner; the selected profile admits one attempt per parent, not an
automatic retry loop. Matching requests retrieve that child; changed attempt/
effect labels refuse, including through raw admission. Prior raw attempts cannot
be retroactively adopted. Child retirement also retires its co-owned parent into
a spent tombstone. Each bounded parent record has a separate lifetime reservation;
the parent funds maxHeldBytes + 2 × maxIssued × maxCaptureBytes for live payloads,
spent origins and parent records. Source thirteen controls/sixteen mutations and
annotation twelve controls/sixteen mutations pass with coherent baselines.
Bounded read-only review found no concrete defect in the selected parent relation.
This is a proposed reference/design delta pending qualification/adoption, not
whole F20, downstream result/event/discovery or general retry/reconciliation closure.

The same working candidate now supplies §16.1.5's two ordinary-save counterparts.
Its actual shared writer owns original domain/context, immutable request and
actual child; annotation intent additionally retains the full validated artifact
context and identity/source-association binding. Matching requests cannot
recapture, migrate pools/profiles, dispatch or attest storage. One attempt per
parent, raw bypass/adoption refusal, real claim discharge and spent retirement
are checked. The sponsor separately funds the bounded parent store once for the
shared node. Bounded private input/scope capture precedes current-fact reads.
Eleven controls/fifteen coherent-graph mutations, thirteen shared-writer
controls, native source thirty controls/twenty-seven mutations and annotation
nineteen controls/twenty mutations pass; bounded review found no concrete defect.
Exact registration, cold-boundary reconciliation and full qualification/adoption
still govern acceptance. This implements the four selected load/save parent
relations, not the remaining downstream discovery/receipt/event/control products
or all of L02. The fixed twelve-accepted/five-open deliverable count is unchanged.
Review's P2 finding—the outer routing grant read preceding private capture—is
repaired: routing uses catalog metadata only, with strict current permission
checks after capture. The control now mutates caller inputs at both authorized
and writeAllowed reads; the targeted outer-grant mutant is detected. Read-only
fix verification confirms that finding is closed. This is not a grant bypass.

The same input-capture ordering defect was subsequently reproduced on both
load paths: current-fact suppliers could mutate the caller's request/context
before private capture. Both native owners now bound and copy the complete
semantic input before reading facts; outer catalog routing reads no permission
supplier. New controls cover typed/dynamic/native routes, fresh/matching calls
and mutation at authorization/read/annotation-disclosure suppliers. Both old
paths fail the controls; repaired paths and coherent baselines pass, and separate
capture-omission/outer-grant mutations are detected. This repair is 100% complete
at focused-reference scope, pending full qualification/adoption. It closes no
whole deliverable; the twelve-accepted/five-open count remains unchanged. Earlier
review covers the prior parent relation, not this subsequent repair.

The working 0.221 candidate now also supplies §23.1.11's separately versioned
native-operation observation profile. All four actual F20/F21 parent/task kinds
bind to normalized F06 status, retained native-evidence facets, protected minimal
control and bounded observation-only waits. Native completion never becomes a
publication outcome; execution-receipt/1's eight existing kinds are unchanged.
Read rows/chunk-conflict bytes and save export/baseline workflow state are not
exposed. Actual context/task/issuer identity and captured/current grants remain
mandatory. Observer stop/expiry does not stop a producer; known producer retirement
makes later observation unavailable without another execution or issuer.

Seventeen controls/fifteen facade mutations, 24 typed/dynamic deliveries and
the exact registered validator pass. They exercise actual reads, late chunk
conflict without content release, actual write acknowledgment without baseline
adoption/quiescence, native stop, foreign/raw/copied roles, grants, waits, busy/
admission reentry, local receipt faults and protected progress under ordinary
exhaustion. The existing actual parent funds separate bounded context/input,
signature/reply, waiter and ordinary/protected reference envelopes before exposure.
The static boundary covers 134 modules with unchanged 71 engine/five integration/
eleven presentation variants; 130 isolated cold entrypoints pass. The finite
argument is conditional on native monotone stop/quiescence, bounded native
records and returning trusted ports, not physical/concurrent/crash refinement.
Separate read-only challenge found no concrete defect in this selected relation;
the reviewer inspected but did not execute the checker or full qualification.
The normalized result/control/wait slice is 100% complete at focused design/
reference scope, not accepted at full-batch or whole-deliverable scope. Full frozen
qualification/adoption, accepted-registry schema/decoder, F19 event/context and
F18 shutdown assembly remain. This is not whole L02/L09 acceptance; the fixed
twelve-accepted/five-open deliverable count is unchanged.

The consolidated 0.221/model 0.285 batch has entered full frozen qualification
with the pinned toolchain and required local TLC listener permission (session
19013; expected 799 checks/1,013 inputs, 301 SMT conditions). Its source is now
held unchanged. Initial checks pass; no terminal qualification, endpoint audit
or adoption is claimed for that batch. The predecessor 0.220 run (67748) has since
completed successfully and passed audit as recorded above; only 19013 remains
live. Do not restart it merely because observation times out.

A successor 0.222/model 0.286 now closes the selected native observation decoder/
accepted-registry relation in §23.1.12. Explicit native-operation-observation/1
selection reuses the existing actual-domain pool and its eight prepaid frames;
older pools keep their original branch. Actual parent/domain/task/scope, native
grants and waiter/context retirement remain authoritative. Two records describe
four closed argument/result products and private wait continuations under existing
metadata bounds; structural validity and positive advice grant no execution.

Eleven controls/twelve coherent three-module mutations, 24 route deliveries,
45 standard-schema value cases and the exact registered validator pass. Existing
pool checks (twelve controls/fifteen mutations), decoder checks (twelve controls/
twenty-three mutations, twelve faults/twenty-four reentries), selected substrate
and eight-facet receipt regression pass. The actual retirement mutation anchor
was rebound to the same guard after native/local dispatch sharing; no semantic
assertion was dropped. Static boundary covers 137 modules; 133 isolated cold
entrypoints pass, including the existing admission/capacity modules now explicitly
included in this boundary. Ownership remains 71 engine/five integration/eleven UI.

The decoded handoff reproduced a pre-existing normalized native-profile gap:
its input-size check traversed an unowned opaque progress marker. This successor
sizes null-shadow ordinary data while preserving task/context/progress identity;
the test now refuses without invoking the marker or faulting the receipt facility.
Neither the frozen 0.221 candidate nor adopted 0.219 is silently modified. A
separate bounded read-only challenge found no concrete defect in the selected
new relation and did not execute tests or qualification. This slice is 100% at
focused design/reference scope; full batch qualification/adoption, complete F19
event/context and F18 shutdown integration remain. No whole deliverable closes;
the fixed twelve-accepted/five-open count is unchanged. No extra UI, substrate,
host registry placement, network service or production numeric default is selected.

The completed 0.222/model 0.286 decoder/discovery batch is now also in full frozen
qualification (session 94270; expected 800 checks/1,015 inputs and 301 SMT
conditions). Its staged source is held unchanged from launch. The preceding
0.221 session 19013 remains active; 0.220 session 67748 is terminal successful
and audited. No 0.221/0.222 terminal qualification or root adoption is claimed.

The live 0.222 run has now reported two mutation-construction failures in the
source/annotation original-load request checks. Their short scope-guard anchors
occur twice after addition of native observation state seams. Both checks refuse
construction before semantic mutation execution; this is not a passing batch or
evidence that a runtime guard failed. The unchanged frozen run remains live for
its complete report. The editable 0.223/model 0.287 successor binds each anchor
to the exact original request-admission guard, preserving both scope checks,
all original cases and mutation names. Focused reruns govern this repair; no
qualification input or result is patched in place.

Both repaired focused suites now pass: source thirteen controls/sixteen original
mutations, annotation twelve controls/sixteen original mutations. Passing coherent
baselines precede mutation execution; construction failures are not counted as
semantic kills. The repair changes harness anchors only, not engine guards.

That successor also implements §12.3.4's closed first-party F19 stop composition
across eleven existing native classes. Seven controls, six detected guard-removal
mutations, 22 typed/dynamic deliveries and 165 refusals pass. Actual domain/owner
attachment identity, full original scope and strict current protected control
precede one fixed native stop branch. History remains entry-local; coherent
members close their parent; publication claim binding precedes trusted stop.
No registry, issuer, producer cancellation or policy mutation is added. Static
boundary covers 146 modules and 142 isolated entrypoints pass, with the same
71 engine/five integration/eleven presentation variants. Seven adjacent native
regression suites pass. Bounded read-only challenge found no concrete defect
and did not execute checks. This dispatcher slice is 100% at focused staged
reference scope, not public lookup/hostile-wire, whole F19/L02, shutdown, full
qualification or adoption. Overall acceptance stays twelve of seventeen (71%
by count, not effort); the five open deliverables remain unchanged.

The same editable 0.223 successor now closes selected local opaque stop-target
resolution as well. A single finite pass over actual prepaid attachment metadata
uses pure issuer-owned context membership and then exactly one native stop. The
caller supplies neither owner nor target kind; copied/foreign/retired references
and record aliases cannot become control targets. Ten controls/nine detected
semantic mutants now cover 22 bound plus 22 resolved deliveries and 165 bound
plus 57 resolved refusals. Bounded read-only follow-up found no concrete defect
and did not execute tests. This local resolution slice is 100% at staged focused
scope; wire encoding, stream-command retry/revocation, F18, full qualification and
adoption remain. No new issuer, registry, policy service or shutdown inventory
completeness is claimed; accepted deliverables remain twelve of seventeen (71%).

Working 0.223 §12.3.5 now adds the two original registry-observation command
bindings. The existing registry owns distinct command references, not aliases of
technical preparations; complete command/result frames reuse prepaid packet slots
and nonrefundable issuance. Exact original root/scope/resync generation, one native
call, matching control-only lookup and both typed/dynamic routes are checked.
Thirteen controls/seventeen coherent native-plus-facade mutations cover four routes
and twenty refusals. Post-native-mutation fault injection and four unsafe
alternatives preserve actual generation advancement, fence new registry work,
retain the unknown command and keep protected stream discharge. Unexpected faults
are not ordinary unavailable or restart. Read-only review's pre-lookup fault
classification finding is repaired and verified by readback. Post-fact revalidation
also prevents retired-command lookup, retirement erasing a reentrantly faulted
record, and registry retirement releasing a newly issued child's funding. This original-command
slice is 100% at staged focused scope, not historical baseline payload disclosure,
discovery, explicit revocation, F18, full qualification or adoption. No host-defined
command-registry placement choice or new owner/global issuer is selected. The
fixed twelve-accepted/five-open count remains unchanged.

The command slice's exact full-harness predicate passes. Adjacent registry
regression retains twenty-five controls and all forty original mutations across
the five banks; the 146-module boundary and 142 isolated entrypoints pass after
the command/fault changes. These results do not substitute for the still-required
full frozen qualification or historical-result disclosure/discovery binding.

The final post-fact reentry guards now pass the exact registered predicate
(thirteen controls/seventeen mutations/four fault alternatives), the complete
existing registry suite (twenty-five controls/forty mutations), the 146-module
ownership boundary and 142 isolated entrypoints. The consolidated 0.223/model
0.287 candidate is frozen at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-Yy1Cm5`
with 1,018 inputs. Full qualification is running in session 32980 with the pinned
toolchain and local TLC-listener permission (expected 802 checks/301 SMT
conditions). Do not edit that source or restart on an observation timeout.
The predecessor 0.222 session 94270 remains separately live; its known three
mutation-construction failures are repaired in this successor. Neither running
suite is an accepted report, and root adoption remains 0.219.

The separate editable 0.224/model 0.288 successor at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-3hbXaC`
now specifies §12.3.6's original registry-baseline receipt relation. It follows
the selected typed retained-outcome route and existing historical metadata release
precedent: separate receipt-context revocation, retained/funded original slot
associations, exact current historical authorization even for removed slots, and
complete-original-frame or unavailable rather than a silently refreshed/partial
baseline. Existing receipt profiles and stream queues remain unchanged. The model
marks the new release edge explicitly unverified. Native/decoder/discovery,
retention/funding and reentry composition remain required; the running frozen
0.223 candidate cannot qualify later 0.224 edits. No deliverable acceptance changes.

The frozen 0.223 run has now exposed an additional mutation-construction failure
in owner-attachment bank 10: registry retirement captures current facts as `f`,
but its parent-refund mutation still targets the previous call using `g`. The
0.224 harness repair changes only that specific construction operand; it still
removes the actual parent release and requires the same semantic assertion to
detect the missing refund. The native retirement guard and frozen 0.223 inputs
are unchanged. Focused bank verification passes in terminal session 29586:
the coherent baseline and all ten existing bank mutations, including omission of
the real registry-parent refund, are detected as required. Do not poll 29586 again.
The full 32980 run remains live and its final report must retain its actual failure.

The predecessor 0.222 run is terminal (session 94270, exit 1); do not poll it
again. Its unchanged 1,015-input worker completed all 800 registered commands:
797 passed, with exactly the source-load/annotation-load original-scope and
load-activation retirement mutation-construction failures already repaired in
0.223. Report SHA-256 is
`90b685a03f866f65a58b6bd82ece29d8a15b2c91ce7da49084ce87b0a0635b9e`,
retained at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-WDcSnu/formal/results.json`.
It is a complete failed run, not an adopted pass. At that recorded point, only
the newer full session 32980 remained live; the subsequently launched 0.224 run
is recorded separately below. This historical statement is not a current run
inventory.

Working 0.224 §12.3.6 now has actual native receipt ports, not only a prose
contract. The registry funds and retains original historical entry-to-slot
associations, owns each receipt context as a real command-retention claim in its
existing bounded packet inventory, and binds historical delivery certificates to
the original context/command/result and current root. Narrowed authorization never
filters/refreshes the original baseline or consumes stream events. Live-stream
retirement does not erase independent retained command history; receipt stop or
fault does not erase its claim or execution settlement. Protected context
retirement requires pending certificate discharge. Eight native control groups,
ten targeted coherent-graph unsafe variants, seventeen existing command mutations,
four fault alternatives and the exact registered predicate pass. The complete
legacy registry suite (twenty-five controls/forty mutations) and existing stop
facade (ten controls/nine mutations) also pass. Second-agent read-only challenge
found no concrete reachable defect at the selected native scope; no independent
certification is implied. The selected public F06 facet now binds both native
forms on four typed/dynamic routes; original result and strict opaque-role
validation quarantine malformed provenance without changing command settlement.
The integrated predicate requires ten receipt control groups, ten native/three
facade mutations, two schema-valid identity fault probes/three unsafe alternatives,
two accepted-registry descriptors/three variants and twenty-five standard schema
cases. Actual authorized discovery and denied-receipt checks establish that advice
cannot substitute for native identity or grant. F19 stop routing now covers twelve
classes including receipt contexts (twenty-four deliveries per route, 180 bound/
62 local refusals, ten controls/nine existing mutations); the old scope mutation
anchor is disambiguated without weakening its semantic assertion. The 146-module
ownership boundary and 142 isolated entrypoints pass. Read-only second-agent
challenge found no concrete defect in the public delta. A private funded native
authorization-basis seam avoids reconstructing removed slots or retaining an
unaccounted host copy; it is not recipient output or a grant. Full frozen
qualification/adoption, explicit F19 grant revocation and whole L02/L09 acceptance
remain. No new mode, owner/global issuer or widening of an old profile is selected.
The fixed accepted count is still twelve of seventeen (71% by count).

The consolidated 0.224/model 0.288 candidate is now frozen at
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-djMxiC`
with 1,018 inputs. Full qualification is running in session 38273 using the pinned
toolchain and local TLC-listener permission (expected 802 checks/301 SMT
conditions). The source contains the bank-10 mutation-construction repair as well
as the integrated registry command/receipt/discovery/stop binding. Do not edit
that frozen source or restart its process on a timeout. The older 32980 run remains
separately live with its known bank-10 construction failure; neither a partial
passing log nor its eventual failed report can substitute for 0.224 qualification.
Root remains adopted 0.219; no complete deliverable is accepted by launching a run.

The 0.221 frozen run is now terminal (session 19013, exit 1); do not poll it again.
Its unchanged 1,013-input worker completed all 799 registered commands, with
796 passing and three mutation-construction failures: the two known duplicated
load-request scope anchors and load-activation bank 2's obsolete adjacency anchor
for retirement revocation/task deletion. The third anchor is now rebound in the
editable successor to the revocation call itself, preserving parent retirement,
all original assertions and mutation names. The repaired bank now passes all ten
original semantic mutations with its coherent baseline; engine predicates are
unchanged. The old failed report stays unqualified in
`/private/var/folders/19/z642v6s947n2swz2pv39v1nh0000gn/T/vellum-proof-inputs-hu9KD3`
with SHA-256 `607a448512bc6989b8e04f7155af7b7760827721a906e1fb7359cf21c22482e8`.
All registered tools ran and endpoints stayed unchanged; construction failures
mean those commands did not complete all their intended semantic cases. No
passing batch or adoption follows. Only session 94270 remains live; its known
load-anchor failures are likewise not patched in the frozen source.

The audited baseline is specification 0.219/model 0.283: 794 successful checks,
1,005 inputs and matching source/worker/root endpoints. The requirements fingerprint is
`6807f406fc8e32dc2f83b8076ec0928b0940dbd2b01cc9aece6d124b80e4f590`.
The exact report SHA-256 is
`ab45ed98efc5bb8e6561b4f3e4d59ceb27a67d596882908a2318018c4800984d`.
The byte-exact qualified worker/archive is retained in `.verification-evidence.3Yr0Kv`;
its read-only integrity check passes. Previous 0.218, 0.216, 0.215, 0.214, 0.213, 0.212, 0.211, 0.210 and 0.207 evidence remains retained.

The unqualified 0.217/model 0.281 attempts consolidated the selected native owner attachment,
new-obligation versus retained-service, root-installation and dependency-discharge
changes described below. Its first complete frozen-input qualification finished
with 26 failed checks, complete tool execution and unchanged input endpoints.
Every failure maps to an investigated repair; none remains unclassified. The
second qualification finished with one failed check out of 791, complete tool
execution and unchanged endpoints. The failure is the retained-source lookup
fault mutation: new admission and later query guards masked that original
mutation. The editable harness now requires a passing
contextual baseline with those guards disabled before mutating the lookup guard;
all thirteen original mutations pass its actual registration predicate in focused
verification. No engine predicate changed. This correction is outside the second
run's frozen inputs, so neither run qualifies the candidate. The complete second
report has been inspected; no other failure remains unexplained. Its report hash
is `ba500bda744729b9785ad71cc5ff0b03173fc06da2838ebf4b4753c979ea2a9c`.
This qualification does not close shutdown consent,
controller composition, complete dependency/factory coverage or the five open
design deliverables.

Adopted [specification 0.218](../specification.md) includes the owner attachment,
admission/dependency fixes, retained-source test correction and F08 query facade
over the actual analysis owner. Both existing query variants preserve original
scope/snapshots, overlapping decoder/native reservations, fault cleanup and protected
evidence; generic accepted-registry discovery adds no builtin profile or command
receipt. The complete frozen suite, source/worker/root endpoint audit and retained
archive integrity check pass. Targeted read-only second-agent review found no
material defect at this selected F08 scope; it is not independent certification.
This closes the selected F08 argument/discovery binding gap, not F16 activation,
F24/F28 invocation/task/evidence assembly or complete shutdown. L02/L09/L13/L17/L18
remain open; twelve accepted deliverables and five open deliverables are unchanged.

Focused repairs now cover the direct and ordinary configured-line settings-pool
attachments and their retained cleanup, admission-fence mutation construction,
metadata reservations in settings/read/annotation boundary expectations, and
the preserved discovery mutation partitions. The annotation substrate checks pass
with native pool and ownership-edge charges accounted for separately. Source-read
and annotation-export verification split the original scenarios and mutation names
into smaller processes under the unchanged command deadline; they do not weaken
engine guards or remove cases. These focused results and the reconciled first-run
failure list permitted the second full qualification attempt; they do not replace it.

The existing source census maps 1,121 units to 1,121 review rows; it explicitly
does not establish semantic closure. The decision ledger has 48 baseline entries:
25 baseline-specified and 23 baseline-specified-profile-gated. The catalog has
71 engine variants across 23 families, five integration variants across three
families and 11 presentation variants across six families; families can span
ownership classes. These are inventories,
not completion percentages or proof that every variant is implemented.

Report progress as named deliverables closed, their remaining acceptance criteria,
and decisions or external gates. Do not divide test counts, variants, paragraphs
or the unequally sized deliverables below into a whole-goal percentage. No
deliverable is newly signed off merely by appearing in this ledger.

The qualified baseline has twelve accepted deliverables and five open deliverables:
L02, L09, L13, L17 and L18. The approved clipboard ownership change temporarily
reopened L01's inventory/disposition acceptance; the three-owner catalog is now
qualified and adopted, so L01 is reaccepted. The current approved review has
twelve accepted and five open engine deliverables. These are cited dispositions,
not an effort percentage. L14 and L15 remain accepted following qualification
and adoption of their corrected contracts.
L11's presentation work is tracked separately, not counted as an engine blocker.
The combined editor ledger currently has twelve accepted and six
open rows including L11; no new closure is claimed by moving a UI responsibility. These are unequal work packages,
not an effort estimate. Design/reference closure is the goal of this pass;
production implementation, measurement, governance and independent-assurance
gates remain separately visible and do not silently expand this pass. Reuse
adequate accepted evidence. Reopen a closed item only for a concrete contradiction,
changed contract or unmet acceptance condition; record that reason explicitly.
Each update must name a closure or a specific remaining acceptance gap. Additional
tests, documents and repeated reviews alone do not count as deliverable closures.

This is a working control artifact, outside the current verifier's root-Markdown/
formal-file input inventory. Its structural crosswalk is checked separately; it
is not part of the frozen qualification or evidence for semantic completion.
Normative contract changes still require the normal frozen qualification process.

## Scope and ownership

[Requirements §§1.1–1.2](../requirements.md) govern inclusion; later conditional
requirements do not promote deferred capabilities into v1. Both iced and ratatui
adapters and both thin applications remain required. Optional parsing still needs
the approved working mixed-language reference integration. Exact-revision agent
plans, explicit declared dependencies, atomic local multi-document publication,
lossless fidelity, annotations and bounded cleanup remain required.

The engine owns semantic behavior and mutation authority. Integration owns
headless-capable workflows across authorized engine and provider contracts. UI
support owns keymaps, chords, interaction, geometry/presentation and native-input
translation. Resource modules own concrete filesystem or other supported resource
mechanisms; separate host/provider adapters supply clipboard, clocks and execution.
Their evidence does not manufacture semantic authority. Follow the existing
[single-user release classification](../v1-release-gates.md).

No mandatory transport, federation, workflow service, scripting runtime, semantic
rename/discovery engine or universal schema/graph engine is added. Additional
provider/profile generality is not a prerequisite unless an existing v1 contract
actually advertises it. A known defect on a supported path cannot be deferred.

### Engine completion excludes UI completion

The engine is the presentation-independent behavior facade and its authoritative
semantic state/effect contracts. Keymaps, painting, layout/decoration producer
interfaces, geometry providers, visual navigation/scrolling policy and native
input/accessibility presentation belong to adapters/examples. They cannot become
engine-completion conditions merely because the wider editor release needs them.
Engine-facing logical targets, selection mapping, validated commands, effects,
outcomes and authorized data release remain in L02 and their owning engine slices.

L11's useful UI work is retained in the [reference UI design backlog](reference-ui-design.md),
including the ratatui example, with applicable shared iced integration obligations.
Its geometry/decoration choices and UI framework/platform matrix are separate from
the engine pass. L13 still needs the engine package/feature/type exposure and API
evolution contract, but not a selected UI framework version. U03's engine Rust/
package/support choices and its UI support choices must be reviewed separately.
L17/L18 checks this boundary without requiring UI completion. Existing UI references
may remain in the combined verification suite as boundary/integration evidence;
their presence does not import their responsibilities into the engine.

### Modular resource substrates and first target

The user selected modular resource substrates with the local filesystem as the
first target module. The [substrate module design](substrate-module-design.md)
records engine-owned ports versus provider-owned mechanisms, explicit capability/
identity profiles and retained effect responsibility. S3 is a possible later module,
not a v1 delivery or filesystem-equivalence promise. This is separate from the
engine's in-memory text backend and needs no runtime plugin/hot-swap framework.
The adopted 0.207 batch includes the requirements/package/model reconciliation;
L13 retains unresolved API policy, and L17/L18 retain final consistency/handoff.
E01/E02 retains real local-filesystem
implementation and OS/procedure conformance. The existing L12 port contracts remain
accepted at their stated scope; this decision does not establish provider truth.

### Engine package and type boundary review

[The API boundary review](engine-api-boundaries.md) records permitted exported
contract roles, provider/UI/host/serialization edges, feature-combination rules
and procedural compatibility/migration obligations. These are architectural roles,
not invented crate names, tested feature combinations or an MSRV promise. The
review now crosswalks requirements §16 and specification §§25/28.43–44/29.5:
separation/type exposure, headless/substrate boundaries, feature activation,
behavioral evolution, retained interpretation/migration and asset/support obligations
are already specified. No duplicate reference model is required for these roles.
The user selected exhaustive authority-bearing executable enums within explicitly
versioned profiles. Unknown execution remains rejected; named Unknown knowledge/
effect states are not an extensible executable payload. Adding executable meaning
requires an explicit version/API compatibility decision and downstream migration.
Adopted 0.219 §25 reconciles that selection. L13's stated design criterion requires
recording the support choices needing U03, not asserting an unimplemented matrix.
Its crosswalk covers the listed design facets. Subsequent U05 review exposed the
host-command registry placement ambiguity, which must also be reconciled before
acceptance; full 0.219 qualification and endpoint/adoption audit pass. Actual package/MSRV/
platform choices and conformance remain U03/E01–E02, with accountable commitments
in U04/E04. L13 is not yet signed off. This working record does not change frozen
qualification inputs.

### L14 accepted engine instrumentation contracts

L14's engine scope stops at semantic/effect/control interfaces; UI input/layout/
render/presentation signals belong to the separate example stream. The adopted
observation-intent index and bounded marker queue do not alone close the semantic
measurement contract. Adopted 0.207 §23.3.1 supplies engine-measurement/1:
32 descriptors across eleven engine sites with closed point/dimension shapes,
seconds and explicit byte/count units, populations, phase/cutoff endpoints, tagged
resource coordinates, continuity, coverage/freshness and restricted correlation.
This contract—not the old marker queue—addresses the schema gap.

Focused evidence covers nine controls, 804 point cases, 24 coordinate cases,
286 bounded counter steps and ten semantic mutations. The actual harness predicate
passes from a separate working directory. Separate review found and verified
corrections for pre-admission refusals, derived ordinary refusals and the once-only
first-result endpoint, then found no remaining integration contradiction. The exact
registered checks pass in the complete 765-check report; source/root endpoint and
archive integrity audits pass. L14 is accepted at the design/scoped-reference level.
E02/E03 retains verification of installed probes, provider classification,
runtime episode tracking, actual clock/capture costs, physical accounting and
numerical workload/SLO claims; design acceptance does not establish those runtime facts.

## Catalog ownership inventory

The following is transcribed from operation-families.json and engine-ui-ownership.json.
Required denotes a required surface contract, not unconditional availability or
proof of a working implementation. Conditional operations retain explicit
capability/currentness/unsupported behavior. In particular, optional parsing still
has the required reference integration, and conditional host facilities do not
create a mandatory timer, listener or native-input service.

| Family | Catalog surface | Engine-owned variants | Integration-owned variants | Presentation-owned variants |
| --- | --- | --- | --- | --- |
| F01 discover-contracts | required | list, inspect, advisory-enablement | None | None |
| F02 read-document | required | bounded-range, line, snapshot-handoff | None | None |
| F03 read-annotations | required | enumerate, inspect, target-location | None | None |
| F04 inspect-history | required | frontier, availability, permitted-blocking-detail | None | None |
| F05 search-matches | required | literal-matches, next-match, previous-match | None | None |
| F06 inspect-execution | required | status, bounded-observation-wait, retained-outcome, minimal-control-status | None | None |
| F07 inspect-context | required | effective-configuration, permitted-origin, capabilities | None | None |
| F08 read-derived-analysis | conditional | supported-analysis, supplied-evidence-inspection | None | None |
| F09 read-geometry | conditional | None | None | caret-hit-conversion, bounded-layout, visual-navigation-target |
| F10 validate-proposal | required | advisory-validation | None | None |
| F11 change-source | required | exact-batch, ordinary-edit, replace-current, replace-all, accepted-composition-commit | accepted-paste, validated-cut-deletion | None |
| F12 change-selection-and-navigation | required | set-selection, logical-movement, select-search-match | None | supported-visual-movement |
| F13 author-annotations | required | create, update-body, delete, reattach | None | None |
| F14 reverse-history | required | undo, redo | None | None |
| F15 change-viewport | required | None | None | scroll, viewport-intent |
| F16 activate-context | required | semantic-settings, supported-registry, compatible-analysis-or-provider-context | None | None |
| F17 change-document-lifecycle | required | create-scratch, activate-loaded-candidate, attach-detach-view, membership, rebind, dispose, view-container, workspace-container | None | None |
| F18 control-execution | required | request-cancellation, shutdown-disposition | None | None |
| F19 control-observation | required | establish-bounded-stream, stop-observation, authorized-resynchronization, revoke-observation-grant | None | None |
| F20 request-resource-read | required | source-load-candidate, annotation-load-candidate | None | None |
| F21 request-resource-write | required | captured-source-save, captured-annotation-save | None | None |
| F22 request-clipboard-read | required | None | plain-or-supported-structured-read | None |
| F23 request-clipboard-export | required | None | copy, cut-export-stage | None |
| F24 request-analysis-work | conditional | enabled-parse-query-work | None | None |
| F25 request-geometry-work | conditional | None | None | enabled-provider-measurement |
| F26 request-host-facility | conditional | timer, task-cancellation, authorized-diagnostic-drain | None | dialog, native-input-accessibility, host-preference |
| F27 ingest-provider-evidence | required | progress, acknowledgment, failure, quiescence, unknown, contradictory-or-duplicate-evidence | None | None |
| F28 accept-derived-result | conditional | enabled-analysis-result | None | enabled-geometry-result |

## Approved capability crosswalk

The eleven inclusion bullets in requirements §1.1 route as follows. This is a
scope/ownership check; individual contract/evidence completion belongs to the
named deliverables. The explicit §1.1 exclusions remain excluded, and §1.2's
local, transport-independent boundary applies to every row.

| Scope bullet | Capability | Closure owners and principal family boundary |
| --- | --- | --- |
| V1-01 | Editing, selections, navigation, indentation, read-only and undo/redo | L06–L07; F11/F12/F14. |
| V1-02 | Multiple documents/views, mapping and lifecycles | L04/L06/L07; F12/F17. |
| V1-03 | Document-associated comments and annotations | L07–L08; F03/F13. |
| V1-04 | Anchored agent proposals, declared dependencies and conflicts | L03; F02/F10/F11. No built-in semantic rename/discovery requirement. |
| V1-05 | Typed facade, identifiers, metadata/discovery and host registration | L02/L05; F01/F16. UI owns bindings/chords. |
| V1-06 | Literal search/navigation, replacement and atomic replace-all | L06; F05/F11/F12. |
| V1-07 | Optional parsing/highlighting with required mixed-language reference | L10; F08/F16/F24/F28. No universal semantics or rendering. |
| V1-08 | Both adapters' scrolling, selection, clipboard and basic decorations | L11; presentation rows above and F11/F12/F22/F23. |
| V1-09 | Typed effects/completions, capabilities, control, bounds and file integration | L08/L09/L12/L15; F18/F20/F21/F26/F27. |
| V1-10 | Local identities/capabilities and applied versus persisted truth | L03/L04/L08/L16; publication, lifecycle and evidence boundaries. Cross-authority coordination stays explicitly conditional. |
| V1-11 | Digital twin, tests/conformance, threats, optional telemetry and budgets | L14–L18; actual implementation/measurement acceptance stays E01–E05. |

L01 scope and ownership inventory is audited: all eleven inclusion bullets and
all 87 existing catalog variants have routes without moving UI or substrate work
into the semantic engine. The existing qualified engine/UI boundary evidence
checks the 76/11 partition. This closes L01's inventory/disposition criterion,
not any downstream behavior, unsupported capability, product catalog or release
gate. L17 still audits the finer normative clauses and may reopen L01 only for a
specific missing or contradicted v1 scope obligation.

## Fixed design deliverables

L01's inventory/disposition and L03/L04/L05/L06/L07/L08/L10/L12/L16's semantic design/scoped-reference
criteria are accepted. Other deliverables still need closure audits. Existing adequate contracts
and evidence should be accepted by inspection, not reproduced with another model.
A closure record must cite the exact contract/profile, evidence, assumptions,
unresolved exceptions and separately retained release gates.

| ID | Deliverable and governing requirements | Acceptance criterion for this design pass | Existing evidence to assess first |
| --- | --- | --- | --- |
| L01 | Supported v1 scope and ownership; §§1–3, 16–19 | Every required capability and catalog variant has an engine, integration, UI or substrate owner and an explicit base/conditional/excluded disposition. Historical aspirations cannot create new work. | Specification §§1, 25, 28–29; operation catalog and engine/integration/presentation partition. |
| L02 | Complete northbound products; §§3.3, 4, 12.3–12.4 | Every enabled engine variant has exact arguments, results, applicable outcomes/events, authority/dependencies, bounds/control, typed/dynamic agreement, discovery and version/refusal rules. Record genuine not-applicable facets; do not invent events or network codecs. Resolve U01 where needed. | Operation-family catalog; native argument/result/gateway/discovery references; specification §§6, 12, 22–23. |
| L03 | Exact proposals and atomic publication; §§3.4, 5.1–5.3, 5.7 | An end-to-end supported multi-object plan covers original revisions, declared dependencies, preparation, current validation, publication/no-op/conflict and companion/history frames. No automatic semantic footprint discovery is required. | Specification §§4–9, 13.9; source/edit contracts, compound publication, mixed-read and proposal-validation evidence. |
| L04 | Identity, lifetimes and mediated release; §§3.7, 15.4–15.6 | Current and retained read/observation contexts have exact ownership, disclosure, invalidation, handoff and protected-discharge contracts, including document/view/workspace lifetimes. Host policy truth remains an external premise with an explicit interface. | Specification §§11–12; source/annotation/history/discovery context, lifecycle and observation references. |
| L05 | Configuration and activation; §§11–11.2 | The supported engine settings/registration catalog has typed scope/composition, actual consumers and dependencies, activation/refusal, retained interpretation and result semantics. UI binding catalogs stay separate. No arbitrary future-provider completeness obligation remains. | Specification §22; native settings, definitions, layers, scoped bases, registration and created-result evidence. |
| L06 | Ordinary editing, coordinates and search; §§5, 5.6–5.7, 17.3 | Supported text/selection/movement/indent/search/replacement commands have complete endpoints, Unicode/line profiles, read-only/no-op/conflict behavior and shared semantics. Optional folding/tailoring is explicitly classified. | Specification §§4, 15, 26, 28.17–18/45, 29.2; text, line, selection, search and Unicode references. |
| L07 | Annotations and linked history; §§3.8, 5.8, 15.6 | Authoring, mapping/detachment, both point profiles, source/annotation identity/version companions, grouping, undo/redo and dependency-closed retirement form one coherent contract. Storage restoration is reconciled with L08. | Specification §§13–14, 29.3–4.1; annotation, compound/history, frontier and snapshot evidence. |
| L08 | Load/save and saved-state lifecycle; §§5.4, 5.9–5.10, 10 | Required source/annotation load, ordinary save and supported export/rebind paths define captures, independent outcomes, external-change protection, saved baseline, failures/unknowns and retained obligations. Distinct ordinary destinations and lossless storage stay fixed. Any advertised in-place reload needs its own contract; initial activation does not imply that optional surface. | Specification §§11.2.1/11.2.17, 16/29.4.1; codecs, read/write owners, binding-pair, ordinary-save, SaveAs, annotation-load and baseline evidence. |
| L09 | Execution, control and historical evidence; §§3.6, 7 | Supported work has closed admission/service/settlement, cancellation, deadlines, shutdown, duplicate/late/conflicting evidence and fault/reentry relations. Separate stop intent, settlement and quiescence; name scheduler/clock assumptions rather than asserting service. | Specification §17; invocation, F18 control, effect evidence, observation, queue and supervision references. |
| L10 | Optional language and supplied evidence; §§6–6.2, 5.2 | The approved Markdown/child pipeline and supplied-evidence contracts specify pinned profiles, maps, context/generation changes, bounded work, exact coverage and fallback. Optional providers cannot become core authority or introduce deferred structural operations. | Specification §18/26.3–6; pinned assets/licenses, parser bridge, mixed-language, native-analysis and report evidence. |
| L11 | Separate UI example/adapter stream; §§8–9.5, 10.1 | Track presentation/input/layout/decoration/accessibility and UI support choices in reference-ui-design.md. Not an engine-completion blocker. Only semantic facade/data/effect boundary obligations remain in the corresponding engine rows; no UI completion is asserted. | Specification §§19–21; existing ownership partition and frontend/geometry references retained as UI/editor-integration evidence. |
| L12 | Substrate procedures and guarantees; §§3.4, 5.9–5.10, 7.6, 10 | Required host ports specify identities, procedures, evidence fields/strength, capability failures, alias/continuity, synchronization, clocks and cleanup. Concrete platform guarantees await E02; stronger absent capabilities do not disable ordinary best-effort saves. | Specification §§11, 16–17, 21, 25; identity, source-write/read, provider and substrate boundary evidence. |
| L13 | Package and compatibility design; §§16–16.2 | Record library/provider/adapter/app boundaries, features/types crossing each interface, profile/version evolution and the support choices needing U03. Do not infer actual MSRV/platform compatibility from a supplier manifest or reference tool. | Specification §§25, 28.43–44, 29.5; package/host decision register. No Cargo manifests currently establish a production matrix. |
| L14 | Fault and instrumentation contracts; §§12–12.8 | Supported pathways/interfaces map to closed dispositions, minimized schemas, units/populations, correlation, timing, loss/freshness, pressure and grant/retention rules. MELT/golden/RED/USE share signals; STRIDE/LINDDUN/hazard perspectives identify risks without duplicate counters. | Specification §23; model interfaces/pathways, observability catalog, outcome projection and diagnostic capture/drain evidence. |
| L15 | Resource profiles and protected discharge; §§7.5–7.6, 12.7, 17.10–11 | All supported obligations have units, owner, admission/retention/refusal and once-only discharge rules; profile validity and protected control are defined. Physical allocation costs and numerical production limits stay E03, not missing design theorems. | Specification §§17.2, 28.35, 28.39–28.41; reservation, capacity, lifecycle, preparation and decoder evidence. |
| L16 | Local security and privacy contracts; §§15–15.6 | Capability provenance/currentness, inert authored data, boundary validation, minimal disclosure and fault isolation cover supported paths. Specify supply-chain and maintenance acceptance requirements; real ownership/support commitments remain U04/E04. | Specification §§11–12, 23–24, 28.47; STRIDE/privacy/hazard and evidence-trust coverage. |
| L17 | Requirement and evidence traceability; §§13–14, 17–19 | Reconcile every applicable normative source clause with a supported contract, condition or justified exclusion and exact evidence role. Resolve stale historical residual wording. Census coverage alone cannot sign off semantics. | Current clause audit, D01–D48 ledger, specification coverage and scoped evidence limitations. Audit row counts describe inventory, not semantic closure. |
| L18 | Final model reconciliation and adversarial handoff; §§13–14, 17.14–15 | Reconcile L01–L17, requirements/model/contracts/interfaces/instrumentation and all adopted input hashes; run the frozen suite and a separate challenge review. Deliver remaining user decisions and E01–E05 with no hidden design ambiguity. | Qualified reports/retained snapshots, digital twin and risk-based acceptance rules. Independent Cleanroom certification is not implied. |

L01 and L17 establish the closure baseline. L02 integrates the public contract;
L03–L10 and L14–L16 supply its semantic obligations. L11–L13 can proceed in parallel
where they do not depend on an unresolved operation/profile. L18 is last.

### Northbound audit disposition

The [northbound contract audit](public-contract-matrix.md) maps every engine
variant to existing native contracts and six finite integration work items.
Its inventory/assembly is complete; L02 semantic closure is not. The catalog is
explicitly routing-only, several native products still lack a complete facade
binding, and F18 cancellation does not establish shutdown. U01 is resolved;
its selected 0.219 receipt binding is qualified and adopted at N02's stated scope.
Existing sufficient contracts and evidence are to be reused rather than recreated.
This working audit does not change the qualified normative baseline.

N03's [working lifecycle reference](lifecycle-outcome-event-reference.mjs) now
checks native outcomes, original-invocation/per-form receipt correlation and
recipient projections together: 28 success cases, 28 pending-stop cases, fourteen
withholding cases and two permitted annotation-facet deliveries. Six controlled
unsafe composition alternatives fail their assertions. This addresses the review
finding that receipt shape alone admitted a foreign invocation. The remaining
acceptance gap is cross-family outcome/event assembly and complete F19 context/
control binding, plus qualification of the selected receipt route. This working
reference is outside the frozen suite and closes no whole L02 deliverable.

F16's eleven selected semantic-settings/supported-registry forms are accepted for
their existing N01/N06 logical input/execution bindings in the
[northbound audit](public-contract-matrix.md#accepted-settings-and-registration-input-execution-bindings).
The adopted 0.207 native plans, scoped bases, current service checks, shared
publication and handler-bound /2 discovery supply the evidence; no new envelope
or proof program is needed. Fresh focused settings/scoped/discovery reruns pass,
and second-agent readback found no missing logical binding blocker. This closes
that integration task, not full F16 or L02: compatible analysis/provider-context,
N02/U01 result assembly and complete N03/F19 relations remain open. The detached
registration-result validator is not treated as installed native validation.

Adopted 0.210 accepts the [standalone annotation/reversal and mixed-authoring bindings](public-contract-matrix.md#accepted-annotation-reversal-and-mixed-authoring-bindings)
for N01/N06, plus the internal coordinator-owned receipt vector at its native
retained-release scope. This resolves the corresponding argument/execution and
internal receipt-ownership tasks using 768/768 qualified checks, 970 matching
source/worker/root inputs and the retained archive. It does not select U01's
public receipt route or close complete N03/F19/other-family assembly; L02 remains
open. Independent host/issuer truth and E01–E05 remain separate gates.

F19's [read-context stop bindings](public-contract-matrix.md#accepted-protected-read-context-stop-bindings)
are accepted at N01/N04/N06 native-reference scope for source, annotation/combined
and coherent parent/member contexts, including retained copies under those same
issuers. The adopted exact request, original owner/scope, strict protected-control,
whole-parent stop and separate hold/retirement contracts suffice; focused reruns
and second-agent readback confirm that scope. No new facade wrapper or proof
program is added. The same audit now accepts existing protected native stops for
history, settings, discovery, resource-catalog and registration-stream contexts;
fresh focused runs pass fourteen, twenty-two, twenty-three, twenty-eight and
twenty-five controls respectively. History context close checks its original
recipient while its actual owner retains the scope; whole-owner history stop is
a distinct operation. Registration establishment/resync native products suffice,
but reused preparation refusal is not original-command matching-result lookup.
Heterogeneous dispatch, unsupported classes, that command-retry lookup,
revoke-grant and N03 remain the specific F19 integration gaps. No whole
L02/L09 deliverable closes from this disposition.

Adopted 0.215/model 0.279 passes the complete 772-check suite over 981 inputs,
with matching source/worker/root and archive audits. It binds the existing analysis owner to its actual
invocation domain/current root and supported inventory. Ten focused controls,
117 ordinary and 36 protected strict-grant cases, twelve coherent module mutants
and adjacent regressions pass; review found the missing-inventory refusal gap,
which was corrected and rechecked. This does not close the complete analysis
audience/context/result facade, provider activation, global funding or L02.

F20/F21's four [native load and ordinary-save request bindings](public-contract-matrix.md#native-load-and-ordinary-save-request-bindings)
are accepted at exact native request/admitted-task execution scope using the
qualified 0.211 evidence. Actual domain/root, private issuer, current binding,
immutable codec capture and annotation baseline-pool relations already exist;
no new generic envelope is needed. Fresh shortened reference runs pass 27 source-
read, fourteen annotation-load, thirty source-save and nineteen annotation-save
controls; separate code review confirms that scope. Complete N01/N06 remains
open because reused origins refuse rather than compare the original invocation/
request and retrieve its task. A changed attempt label is not matching retry.
N03, F18 shutdown, U01 and E01–E05 remain separate. This resolves the native
binding inventory question, not a full family or L02 deliverable.

F26/authorized-diagnostic-drain is accepted for its existing N01/N04/N06 logical
facade binding; see the [variant acceptance](public-contract-matrix.md#accepted-diagnostic-drain-binding).
The qualified gateway already binds the real queue/domain/scope, issued task,
exact typed/dynamic request, immutable matching retry, once-only service,
separate result/control release and actual parent-hold retirement. Its sixteen
controls/forty semantic mutants, queue ownership suite and eight BASE-DRAIN
conditions are reused. A fresh shortened run and separate code review confirm
that scope. No invented publication event or generic U01 receipt route is needed
for this local diagnostic operation. Timers, general task cancellation, host/
exporter truth, installed instrumentation and all-F26/L02 closure remain outside
this acceptance. No whole deliverable count changes.

The remaining F19 publication-observer binding now has an explicit
[attribution prerequisite](public-contract-matrix.md#publication-observer-attribution-prerequisite)
and [working reference](observer-attribution-reference.mjs). Caller scope/role
binding must precede the trusted /2 helper's absorbing current-context loss path.
Twenty foreign claims/eight wrong-role cases preserve the valid queue; eight real
loss cases fence it; two unguarded examples demonstrate the unsafe adaptation.
Second-agent readback confirms only that ordering claim. The adopted helper is
unchanged; installed guarded assembly, exact heterogeneous products and frozen
qualification remain L02 work. No new policy service or user decision is implied.

Adopted 0.212/model 0.276 implements original-claim binding in actual
native event/minimal-outcome release ports. Five controls/eight semantic mutants
and six adjacent suites pass, as do the exact registered predicate, unchanged
124-module engine boundary and 124 cold entrypoints. Bounded review found no
must-fix under normalized inert-input and independently trusted current-port
premises. This closes the native release-attribution gap only;
full public issuer/domain/context assembly remains open.
All 770 checks pass with matching source/worker/root and retained archive integrity.
The 976-input fingerprint is
`5a5f50da70acdc0d828b74072b5bfa342fa2d832b6bc999a811a64401b003ed1`;
session 91050 is terminal exit 0 and must not be polled again. The exact report
and archive are recorded at the top of this ledger. Its publication/archive-copy
scripts are consumed and must not be replayed. No whole F19/L02 deliverable closes.

The northbound audit also accepts N01/N06 for F12's four existing native forms
implementing set-selection, logical-movement and select-search-match. Exact
arguments, domain/ticket replay, view-only authority, retained native evidence,
qualified terminal pairs, protected cleanup and discovery /11 are covered by
the adopted 0.202 evidence. No operation-specific selection receipt is invented;
U01 does not block these bindings. Cross-family F06/F19 observation/event assembly
remains N03. This closes that specific binding task, not another whole L02 slice.

F11/ordinary-edit's four existing native forms are also accepted for N01/N06:
literal selection, explicit simple line, Unicode boundary deletion and configured
simple line edits. The qualified native declarations, actual owner gateway,
shared publication/terminal path and discovery /3 supply that binding. Remaining
F11 ingress and cross-family observation assembly stay open. The two existing
replacement bindings are now accepted separately through the generic accepted-
registry metadata contract: F11/version-1 identity, exact operation/index/replacement/
invocation fields, separate original owned result, current eligibility and native
handler/common publication path. The northbound matrix records their full scope.
A separate bounded review confirms that requiring another fixed builtin discovery
version was unnecessarily strong; profiles 1–13 are not asserted to contain these
variants. Registry/handler truth and cross-family observation remain distinct.
Adopted 0.207 closes F01's result-guard composition exception and accepts source-only
exact-batch plus primary-only composition. Accepted paste and validated cut still
require their original-task admission/dispatch/replay binding dispositions.

The former composition gap has an executed native counterexample in the
[northbound audit](public-contract-matrix.md#accepted-composition-commit-binding).
Ordinary editing correctly replaces both selections (`XbX`); the accepted
primary-only composition requirement would replace only the primary (`abX`).
The separate composition-commit/1 binding now supplies that behavior, including
mapped secondary selections, through the adopted 0.207 coordinator path. Preedit
and native input ordering stay in the UI stream. The working counterexample is
not qualified evidence; the adopted sixteen-control/six-mutation binding is.
This closes that specific N01/N06 task, not a whole deliverable.

F06's four existing bindings are accepted against actual ledger/native result,
prepaid decoder and /8 discovery contracts, including trusted host-driven bounded
wait continuation. Pooled admission returns the original funded context/reservation;
native poll/tick/expire/stop binds that context and owned waiter, while shared
lifetime checks require waiter drain/native retirement before refund. Continuation
uses funded native bounds directly, not an invented shared decoder busy wrapper.
Separate review confirmed this existing coupling under serialized inert host-port
premises. U01 still controls operation-specific receipt access; complete F19/event
assembly and real synchronization/clock/service truth remain separate. No new
recipient poll verb, event transport or proof model is required for this binding.

L03 is accepted below following complete frozen qualification, source/root
endpoint audits and adoption of the shared unsorted-edit admission bridge.
It is not generic unfinished work requiring another publication model or proof library.

The F02/F03 audit now accepts six existing logical query bindings: source range,
line and single/coherent snapshot handoff; annotation enumeration, inspection
and target location. The northbound audit records their exact product index,
all L02 criterion dispositions and existing qualified integration/fault/discovery
evidence. No new API behavior or proof machinery is introduced. L02 as a whole,
cross-family lifetime closure and production/host/measurement/assurance gates
remain open; these six bindings are no longer generic "needs binding" work.

The same audit accepts F04 frontier, availability and permitted-blocking-detail
bindings using their existing exact arguments/results, actual owner gateway,
current alias/detail release, protected lifetime and explicit discovery /7.
Nine logical query bindings across F02/F03/F04 are now accepted by inspection of
current qualified contracts/evidence. This is not nine new operations or a goal
percentage; full-facade L02 and the separate lifetime/history audits remain open.

F07's existing configuration-query/1 profile is also accepted for its three
operations, with exact inputs/results, actual parent-funded pool/decoder and
discovery /9. This is profile-specific, not acceptance of every capabilities
alternative. The resource-catalog profile's actual-parent funding gap is now
resolved by qualified/adopted 0.202. Its exact resource-catalog capabilities
binding is now accepted separately through generic accepted-registry metadata,
not by extending configuration semantics. The northbound matrix records the five
outer fields, list/inspect alternatives, original context/selector roles, exact
handler/result/fault/lifetime relations and trusted registry correspondence.
This is not acceptance of every conditional F07 domain or full L02/L15.

Qualified/adopted draft 0.202/model 0.266 connects registry funding and
decoded workspace to the actual parent. Nineteen selected actual registrations
pass, including existing resource argument/result/fault/substrate regressions,
49 reader mutations and the extended invocation checks. Four decoder fault stages
retain native obligations; the earlier issuer-reset variant is explicitly
guard-blocked, not counted as detected. The full repaired batch passed 760 checks
over 946 unchanged inputs, with source/worker/root endpoint audits and an
[integrity-checked archive](../.verification-evidence.XyK7Im/manifest.json).
The existing fixed F07 descriptor still describes configuration-query/1, not
resource-catalog-query/1. The latter's logical discovery binding is accepted
through the already supported generic registry contract; another builtin profile
is not required. Separate review confirmed that scope, retaining actual
registry/schema/handler truth as a premise. Funding completion alone was not the
binding argument and does not close all conditional domains or L02/L15.

The original failed full run had 759 passing checks and one stale aggregate
expectation: 21 legacy reader controls rather than the retained 21 plus seven
funding controls. Its repaired successor changes that expectation to exactly 28;
the invalidation controls/mutations and 5/4/10 gap/transient/unknown cases pass.
The failed run remains diagnostic history, not qualified evidence or a pass.
The separately executed, audited successor is the accepted 0.202 baseline.

The successor also clarifies §15.4.4's existing synchronous F05 cancellation
boundary, with the matching model and reference argument. A whole admitted,
finite non-reentrant call is one cooperative quantum; entry and subsequent
release/use checks enforce accepted stop, without claiming mid-scan interruption
or native preemption. Five existing actual search registrations pass against this
successor, including native ownership/consumer and decoded issuer composition.
No search implementation, operation, profile, new checker or solver law changes.
This resolves L06's narrow crosswalk exception. Full qualification and endpoint
reconciliation are complete; the acceptance record below preserves its scope.

The same adopted successor's §22.2.3 supplies the missing trusted host command
binding contract: immutable descriptor/handler incarnation, exact activation
expectations, explicit executor/affinity and serialized base calls, owned capture,
ordinary facade admission, withdrawal and code/asset retirement only after actual
exit/discharge. It is above the authoritative coordinator, not UI keymaps, the
resource registry or a new executable core vocabulary. The conditional preservation
argument in formal/context-handoff.md reuses existing capture/retention/currentness
laws without claiming a native command registry or closing PH-09. Concrete binding
challenge scenarios are explicit, not counted as executed. The separate bounded
review found no must-fix issue in either the host binding or search-cancellation
clarification and confirmed that the prior L05/L06 semantic exceptions are resolved
in this successor. It did not run tests or establish native binding ownership.
The repaired successor's full frozen qualification, endpoint adoption and evidence
archive are complete. The original failed run is not reclassified as a pass.

### L03 exact proposals and atomic publication audit

The supported design is a finite, exact-revision plan published by one local
owner. The following criteria accept existing contracts and scoped evidence;
they do not certify a production engine. L03 is closed at this level. The adopted
0.201 report is complete, passes all 758 checks and matches all 946 current inputs.
Its SHA-256 is
`69754044a560fab7eca7daa3e5a1473a0229ec38d8b8895eca606513a6e2520a`.

| Acceptance facet | Disposition and exact existing evidence |
| --- | --- |
| Original targets and coordinates | Accepted: specification §§2–4 and [source/edit contracts](../formal/source-edit-contracts.md) bind authority/document/incarnation, exact starting revision, UTF-8 byte extents and expected bytes. The report's `Source read to exact edit semantic boundary checks` passes. Structural/line resolution must lower to that same source contract; spelling or parser handles cannot establish persistent identity. |
| Shared normalization | Accepted: §4.2.2 and [invocation publication](../formal/invocation-publication.md) connect one shared helper before immutable intent capture and F10 candidate evaluation, preserving original request/root observation checks. The adopted 0.201 full report passes the 60 permutation/profile/route cases and all 28 invocation mutations, including ordering, ambiguity and empty-record sensitivity. Source/worker/root input matches and report validation pass. The ordered private evaluator is unchanged; normalization preserves whole records, boundaries and expected-content validation. |
| Declared cross-object reads | Accepted: §§13.9.2–13.9.5 and [mixed reads](../formal/mixed-reads.md) bind source/set/body/target, configuration and workspace predicates to the actual owner root and invocation context. The declared-read, configuration-read, workspace-read and mixed-conjunction report checks pass, including intervening changes that retain the intervening root and refuse the whole queued edit. Reads neither grant writes nor create history participants. |
| Preparation and final validation | Accepted: §§5, 6.5.1 and [invocation publication](../formal/invocation-publication.md) stage bounded candidates and validate current rights, dependencies, companions, stop state and capacity before installation. The report's `Owned invocation and actual local publication composition checks` passes. Matching retries retain owned intent; they cannot refresh preconditions or execute again. |
| Current annotation and view companions | Accepted for the declared profiles: §§6.4.1, 13.9 and [current-view mapping](../formal/history-views.md) stage actual annotations and current selections with the source candidate. The compound, view-aware reversal and equal-byte companion report checks pass. Private companion dependencies are not caller visibility grants; absent required inventories refuse rather than mean empty. |
| Linked history and unrelated-state frames | Accepted: §§13.9.1 and 14 and [membership/history composition](../formal/annotation-membership.md) retain one linked group across actual authored participants. Compound/history, retained-membership and native reversal checks pass. Reversal requires all participant frontiers and fresh versions; current unrelated bodies, selections and peer work are not replaced with stale snapshots. General retained target restoration requires supported evidence or whole-group refusal, not invented reattachment. |
| Applied, unchanged, refusal and fault | Accepted: §§6.1–6.5, [no-op publication](../formal/noop-publication.md) and [equal-byte companions](../formal/equal-byte-companions.md) distinguish complete semantic identity from equal source bytes. The report's no-op models, companion classifier and invocation checks pass. Refusal frames the immediately current root; unexpected owner faults preserve conservative Unknown and fence admission instead of claiming confirmed nonapplication. |
| One local installation and later observation | Accepted at design/reference level: §6.5.1 pairs the staged root, reserved terminal facts and done phase in one serialized trusted decision. Invocation checks exercise one publication/history/event across retries. Current release is separate; notification failure cannot undo publication. Concurrent/crash refinement and complete public result/event assembly remain E01/E05 and L02/L09, respectively. |
| Scope, effects and instrumentation | Accepted: §§1, 5, 16–17 and the publication contracts explicitly exclude cross-authority transactions, atomic multi-file persistence and saga recovery from this local guarantee. IF-01/03/08, PATH-01/02 and INS-01/02/03/06/13 distinguish preparation, publication, history, protected discharge and observation. L14 owns exact signal schemas; diagnostics cannot grant authority or determine publication. |

The [41-clause transaction crosswalk](../formal/publication-clauses.md) covers all
ten requirements §5.3 source bullets. Its passing partition/reference check is
traceability evidence only; the behavioral findings above rely on the named
native checks and conditional arguments, not on clause counts or green links.

The normalization bridge adds no command, profile, module or solver law. Full
frozen qualification terminated successfully; source and adopted-root endpoint
audits passed. The byte-exact qualified report and original worker provenance are
retained in [the evidence archive](../.verification-evidence.RchZuq/manifest.json),
whose integrity audit passed. Republication and archiving are not new executions.

Host identity/policy truth, physical resource costs and serialized production
installation remain explicit supplier/refinement assumptions under E01–E05.
Automatic semantic-footprint discovery, a universal mapping engine and arbitrary
provider guarantees are not missing L03 features. Public command/outcome access
and external effects retain their own deliverables; accepting this supported
publication contract does not close those interfaces or waive their gates.

### L04 identity lifetime and mediated release acceptance

L04 is accepted at the semantic design and scoped-reference level against the
adopted 0.201 baseline. The existing contracts cover its lifetime obligations;
no new owner, lease, timer, public command or proof model is required. Public
cross-family routing remains L02, shutdown composition L09, complete resource
accounting L15 and actual host/production evidence E01–E05.

| Acceptance facet | Existing contract and passing qualified evidence |
| --- | --- |
| Logical ownership and lifecycle | Specification §§11.1–11.3.1 distinguish document incarnation, resource binding, views, workspace membership and retained obligations. Last-view closure or membership removal does not dispose an owner; disposal cannot resurrect an old incarnation or erase undischarged effects. `Native retained lifecycle publication checks`, `Native view workspace container lifecycle checks` and the owned lifecycle invocation checks pass. Host alias/continuity truth and safe native issuer construction remain explicit premises. |
| Capture, disclosure and narrowing | §§12.8.4–12.8.10 and [current source contexts](../formal/native-source-contexts.md), [retained annotation projections](../formal/native-retained-annotation.md) and [retained version sets](../formal/native-retained-version-set.md) bind capture to the actual original owner/root or available original context. A retained copy never refreshes from latest; narrowing validates the whole original before removing classes. Source and annotation grants remain independent. Current/retained source, retained-projection and retained coherent-set ownership checks pass. Already disclosed raw snapshots cannot be recalled. |
| Current checks and absorbing invalidation | §§12.8.3–12.9 and [read invalidation](../formal/read-invalidation.md) bind recognized request, original handle and exact scope before applying profile-specific accepted loss. False, unknown and transient refusal have distinct meanings; regrant cannot revive an invalidated identity. Actual source/annotation/coherent owner fault or disposal defeats stale positive host facts. All four `Owned … read absorbing invalidation checks`, native installed lifecycle and source actual-disposal checks pass. This is observed invalidation, not guaranteed eager notification or detection of unobserved ABA excursions. |
| Coherent parent ownership | §12.10 and [version-set contexts](../formal/version-set-contexts.md) require all required members at the same handoff boundary. A member is not an independent lease; subset selection needs a fresh copy. Parent stop and actual lifecycle checks propagate through native search and dependent editing without granting writes. Native coherent-set member, lifecycle, search-disposal and dependent-consumer checks pass. |
| Historical advice and independent commands | §14.5.1 and [history contexts](../formal/history-contexts.md) bind actual selected groups and complete participant frontiers, not a global publication counter. Scoped basis loss invalidates advice; unrelated changes need not. Admitted reversal retains its own intent after inspection closure and independently revalidates authority/currentness. `Owned native history context and reversal composition checks` and history absorbing-invalidation checks pass. Historical receipt release uses its own current grant; disposal is not a universal erasure rule. |
| Configuration and registration contexts | §§22.2–22.3 and [settings reads](../formal/engine-settings-reads.md) retain captured interpretation/ceilings while checking current eligibility. [Resource acquisition](../formal/registration-acquisition.md) retains late installed obligations under the original cut's cleanup receipt; it never infers rollback, refunds issuance or reopens the issuer. Settings lifecycle, resource invalidation and native registry observation checks pass. Pending 0.202 funding and resource-profile discovery are L15/L02, not accepted here. |
| Observation, protected stop and discharge | §§12.3/12.9 and [observation admission](../formal/observation-admission.md), [publication observations](../formal/publication-observation.md) and [invocation observations](../formal/invocation-observation.md) separate observer closure, producer cancellation, settlement, delivery drain and retained ownership. Stop needs actual owner control, not a still-valid content grant or spare ordinary capacity. Resynchronization uses a new authorized frontier; notification failure cannot undo publication. Shared observation admission/retirement, actual invocation observation/wait, collection and recipient publication checks pass. Unknown setup retains reservations; actual empty/drained retirement refunds only its own charge once. |
| Bounds and evidence scope | §§12.8–12.10 distinguish live capacity, monotonically spent issuance, internal holds and protected discharge. Base contexts are bounded and evictable, without a mandatory TTL or renewal/pin API. `Mediated read context registry and retirement checks` and `Native source-context finite lifetime trace checks` pass. Finite traces and logical accounting do not establish physical erasure, global memory ceilings, secure wire tokens, crash refinement or independent assurance. L14 retains exact telemetry schemas for these distinct populations. |

The accepted evidence is the existing qualified execution, not a rerun or an
all-context categorical-completeness claim. Older reference files describing
later-added profiles as future work do not reopen those profiles; L17 must
reconcile that historical wording against the current normative sections.

### L05 L06 and L09 bounded review exceptions

The separate review of these three deliverables found the following specific
exceptions. Their otherwise adequate contracts should be reused, not replaced
with broader provider frameworks, asynchronous search or a new scheduler.

| Deliverable | Existing coverage accepted for reuse | Remaining technical acceptance criterion |
| --- | --- | --- |
| L05 configuration/activation | Six closed settings, typed scope/composition, definitions/layers, scoped activation, configured-line and serialization consumers, registration read ownership and current result release. | Resolved and accepted in 0.202: §22.2.3 defines host-command descriptor/handler lifetime, activation conflict, execution boundary and scheduling, with a conditional preservation argument. Concrete facade/discovery/native binding remains L02/E01, not arbitrary plugin loading or a claim that the resource registry implements command binding. |
| L06 editing/search | Existing editing, Unicode/line profiles, selections, indentation, literal search, navigation/replacement, read-only and no-op/conflict contracts. | Resolved and accepted in 0.202: §15.4.4 defines the bounded call's cooperative cancellation quantum and before/later-release checkpoints. It does not imply mid-scan interruption, native preemption or an implemented async mode. |
| L09 execution/control | Owned admission, replay/publication, stop intent versus settlement/quiescence, late/duplicate/contradictory evidence, protected control and explicit service/clock premises. | Compose the supported F18 shutdown relation across affected owners: independently acknowledged source/annotation versions, admission fence, queued/issued work, bounded drain/cutoff and retained obligations or supported transfer. Specification §§17.4/28.22 already govern policy; the two-save close-admission slice is not a complete shutdown transition. Public syntax stays L02; actual service/crash execution stays E02/E03/E05. |

This is a bounded second-agent document/contract challenge, not independent
Cleanroom certification. L05/L06's exceptions are now resolved and qualified;
L09's distinct composition exception remains open.

### L05 configuration and L06 editing acceptance

Both deliverables are accepted at the design and scoped-reference-evidence level
against specification 0.202/model 0.266. All 760 registered checks pass over 946
unchanged inputs; original worker, source and adopted workspace endpoints match.
The qualified report SHA256 is
`a30c00a421300742d8fc0e41fc7d112b7cc9fe1638050d242b183767cc33740e`.
The archive linked above retains the exact worker/report/input provenance.

| Deliverable | Contract and acceptance evidence | Retained boundary |
| --- | --- | --- |
| L05 | Specification §22 settings, definitions/layers, scoped bases, registration and created results; §22.2.3 command binding contract and the conditional argument in context-handoff.md. Qualified checks include native settings inspection, configured-line consumers, scoped configuration/lifecycle/serialization and current created-result release. The bounded reviewer found the previously identified host-binding contract exception resolved. | Concrete Rust command binding and complete facade/discovery are L02/E01; PH-09, actual policy/executor/asset lifetime and E02–E05 are not proved by the conditional argument. UI keymaps and arbitrary plugin loading do not enter the engine. |
| L06 | Specification §§4–9, 15, 26 and 29.2, including §15.4.4's explicit cooperative quantum. Qualified checks cover pinned Unicode 18 boundaries, logical movement/deletion, simple indentation/newline, ordinary/selection/replacement gateways, synchronous search and handler-bound search products/discovery. The existing five actual search registrations and broader full regression pass. | Whole-call bounded cooperation is not mid-scan preemption or a measured response SLO. Optional tailoring/folding stays explicitly conditional; complete cross-family facade assembly remains L02 and actual adapters/host/measurement remain E01–E05. |

This closes the two recorded semantic exceptions without selecting a new mode,
runtime or provider framework. Reuse this evidence unless a concrete contradiction
or changed contract requires reopening it.

L09 now has a [working shutdown composition target](shutdown-composition-review.md),
outside qualified inputs. Separate challenge found three finite issues, now
corrected: ordinary saves must be qualified/already admitted before fencing;
scope explicitly binds one authority/engine incarnation and future admission;
one shutdown identity preserves the original wait budget/progress across retries.
The reviewer confirmed those corrections without finding a remaining contradiction.
The target also separates post-fence baseline/evidence installation, partial
document dispositions, cutoff/retained ownership and forced-close refusal.
The linked working reference now passes 26 source-reader/writer/F17 controls and
rejects five controlled unsafe alternatives. It covers positive Save/discard
closure, retained save witnesses and the review's queued/absent/failed/incompatible
write counterexamples. The separate native annotation leg passes ten controls/two
unsafe alternatives for actual save readiness and disposal. A new
[joint authority reference](shutdown-joint-reference.mjs) passes eight controls/four
unsafe alternatives using one actual root and writer, with source and
annotation saves on distinct documents. It checks independent prerequisites, both
completion orders, real baselines/disposal, cutoff holds and late contradiction.
Review reproduced and prompted corrections for an incompatible captured write
hidden by historical equality and a loaded annotation baseline compared under the
write context. Both reproduced cases now pass regression controls; read-only review
confirmed the corrections without a remaining must-fix in this scope. The same
reference now additionally passes three same-document controls/two unsafe variants:
actual source/annotation ordinary saves, independent baselines, both completion
orders, one native disposal and cutoff/late evidence. Setup uses existing explicitly
authorized same-destination SaveAs before shutdown, not a cleanup exception; bounded
read-only review found no must-fix. Four source-reference controls also cover forced
wait termination under original control/current strict host authority, unchanged
owners/effects/budget, immutable cutoff and idempotent normal completion. The
retained-owner path supplies no successor transfer or right to free the engine;
read-only review found no must-fix. Ordinary/protected fence integration and complete protected owner inventory
remain specific composition tasks; actual native routes, complete host inventories
and clock/service truth remain separate implementation evidence. These working
references are not adopted and do not close L09. No new generic
cancellation or two-save proof is required merely to repeat accepted coverage.

### L07 annotations and linked history acceptance

L07 is closed at the goal's semantic-design and scoped-reference-evidence level
against specification 0.199/model 0.263. The supported contract is plain-text
authored annotations with document, explicit-profile point and nonempty range
targets; resource-scoped, linked frontier-only history; and bounded, dependency-
closed retention. This does not sign off the complete F13/F14 public facade,
production implementation or release.

| Acceptance facet | Accepted contract and scoped evidence |
| --- | --- |
| Record identity and authoring | Specification §§13.1/13.5–13.6.4; [field authoring](../formal/annotation-editing.md) and [membership authoring](../formal/annotation-membership-editing.md) bind create/update/delete/reattach to document and annotation incarnations. Body and authored-target versions are independent; deletion validates both. Empty body is not absence. The native field/membership publication checks pass through the shared invocation owner. Fresh private allocation is captured once; authentic issuance, reservation and non-reuse are explicit host/refinement obligations, not claims proved by supplied IDs. |
| Mapping and both point profiles | §§4.3.1/6.4.1/13.2 and [equal-byte companions](../formal/equal-byte-companions.md) distinguish position from content, explicit affinity, original-byte survival, affected applicability and detachment. Target deletion retains authored body; equal bytes cannot silently restore identity. The mapping-profile and actual companion-classification checks pass. Automatic mapping changes evidence, not authorship or authored history. |
| Atomic source/comment companions | §13.9 and [compound publication](../formal/annotation-compounds.md) stage source, current annotation placements, authored field/membership journals and current view companions together. Source/annotation/version domains remain distinct. The compound, retained membership, native authoring and view-aware reversal checks pass; a failed required participant preserves the immediately current root. L03 separately qualifies unsorted source ingress; no F13 authoring request gains an independent publication path. |
| Grouping and frontier eligibility | §§14.1–14.7 and [native reversal](../formal/history-reversal.md) freeze an explicit group or expected scope frontier, complete participant set, original journals and mandatory-plus-extra dependencies. Every actual frontier must match. Reversal has its own invocation and fresh changed versions; it is not another ordinary group. Compound/history, native admission, history-context and view-aware checks pass, including linked blocking and unrelated-scope progress. Optional typing coalescing is not an implicit timer or a prerequisite for this explicit-group baseline. |
| Preservation and restoration | Source reversal maps current records/views and preserves later bodies and explicit targets unless they are eligible authored participants. It never restores a stale whole row or historical peer selection. Retained create/delete restoration preserves entity identity with fresh field-version floors. Supported target restoration requires retained evidence; unavailable evidence refuses the whole group without guessing coordinates or reattaching equal text. Native membership/reversal and later-comment preservation cases pass. General semantic or arbitrary mapping-history restoration is not advertised. |
| Redo and retirement closure | §§14.3/14.4/14.8 and [history retirement](../formal/history-retirement.md) define complete linked redo invalidation and least dependency-closed removal under current permissions, pins, floors and bounded work. Protected closure refuses as a unit; surviving journals/frontiers and authored state are framed. `Linked native history retirement candidate checks` and the history invalidation/retention cases pass. Retirement evidence is candidate construction plus scoped preservation arguments—not an installed native retirement, a physical reclamation receipt or proof of complete host dependency discovery. |
| Queries, historical decisions and disclosure | §§12.8/13.8/14.5–14.6.1 and the accepted F03/F04 bindings retain exact current contexts, original selectors, independent ceilings and protected discharge. [Annotation receipts](../formal/annotation-results.md) and [reversal receipts](../formal/reversal-results.md) preserve original decisions while current grants govern release. Their native receipt/current-release and query integration checks pass. U01 still selects public receipt retrieval in L02; receipt existence is not that decision or permission. |
| Persistence and lifecycle reconciliation | §29.4.1 and [snapshot staging](../formal/annotation-snapshot.md) explicitly preserve both point profiles; [initial restoration](../formal/annotation-restoration.md) and [owned annotation loading](../formal/annotation-native-load.md) require exact source/identity associations and a known-empty inventory. Decode/native/capture round trips, initial loaded-baseline and disposed binding/producer checks pass. Annotation/source savedness and outcomes remain separate. Nonempty replacement, merging and reload are not silently inferred. L08 owns the complete persistence audit. |
| Bounds, trust and ownership | §§13–14/23 and the referenced profiles bound bodies, records, targets, journals, inverses, linked fan-out, closure and retained identities. Pressure may refuse work; it cannot evict unresolved authored state or invent empty history. Host-established actor/grant evidence is separate from claimed provenance; comment text remains inert data. Source read/write, annotation authoring and visibility are separate grants. INS-01/02/03/06/07/08/12/13 distinguish authoring, mapping, history, release, persistence and protected retention. L14/L16 retain the complete signal/security audits. |

All 37 existing check names referenced by this audit were found exactly once and
passed in the original 0.199 report. Complete 0.201 qualification retains every
prior check name and passes against the matching 946 current inputs.
The current report fingerprint is recorded in the L03 audit above. This is reuse
of existing executed evidence, not 37 new executions or a semantic proof supplied
by counting tests. The accompanying frame, induction and finite-closure arguments
retain their stated assumptions and bounded domains.

L02 retains public command/result/event assembly and U01. L04's cross-family
identity/lifetime acceptance is recorded above; L08 records the accepted persistence
contracts below; L11 retains both adapters' annotation indication and accessibility.
L14/L15/L16 retain system-wide instrumentation, resource and security audits.
E01/E05 retain actual issuer/retirement integration, coherent native installation,
parser-journal ownership transfer and production/concurrent/crash refinement;
E02/E03 retain actual host policy, retention, physical costs and measurements.
No opaque selector, stored version or successful inverse proves those premises.

A concrete contradiction or missing supported annotation/history behavior can
reopen L07. An older slice's generic "full family remains open" does not by itself
reopen its accepted semantic contract or require a universal restoration profile.
Final cross-deliverable consistency and adversarial challenge remain L17/L18.

### L08 load save and saved state acceptance

L08 is closed at semantic-design and scoped-reference-evidence level for the
selected v1 profiles in specification 0.199/model 0.263: strict-UTF-8 source load,
initial annotation load, ordinary source/annotation saves, explicit source rebind/
copy-export/SaveAs and supported annotation export. The acceptance does not imply
in-place reload, arbitrary encoding, nonempty annotation replacement, automatic
merge or unadvertised force-overwrite/recovery support.

| Acceptance facet | Accepted contract and scoped evidence |
| --- | --- |
| Lossless source and immutable output | §§16.1–16.2 and [source codec](../formal/source-codec.md) preserve accepted scalar UTF-8, loaded BOM choice, mixed newline octets and final-newline absence without normalization. File/document byte coordinates are explicitly different across a consumed signature. The ambiguous absent-BOM/literal-leading-FEFF output tuple refuses before writing without changing valid authored source. Streaming fidelity and native load/save checks pass; their conditional round-trip laws concern the declared domain, not arbitrary binary or other encodings. |
| Read completion and activation | §§11.2.1/11.2.17 and [source resource reads](../formal/resource-read.md) require an owned ordered attributed transcript, exact EOF and successful staged codec output. Birth creates one fresh owner; known aliases join an independently authorized existing owner unchanged, preserving dirty source/comments/history. Captured-read and atomic-resource-snapshot are different requested guarantees. Native read/activation and invocation checks pass; neither token equality nor codec success proves external snapshot truth. |
| Annotation snapshots and initial loading | §29.4.1 and [annotation reads](../formal/annotation-resource-read.md) preserve the accepted semantic snapshot, both point profiles and original received JSON bytes. [Owned native loading](../formal/annotation-native-load.md) independently validates exact document/source/identity association and known-empty inventory; file data is not mutation authority. Snapshot/native/capture, read-to-load, initial baseline and invocation checks pass. Canonical annotation output can differ from loaded JSON spelling; semantic annotation fidelity is not source-file byte fidelity. |
| Distinct ordinary destinations and one writer | §§16.2.3–16.3 and [ordinary bindings](../formal/ordinary-bindings.md) require a current complete source/annotation pair and known resolved-resource distinction. Different locators or roles cannot establish it. Source saves, annotation saves and separately authorized exports share the destination writer/FIFO and unresolved-effect barriers. Binding-pair, ordinary admission and export integration checks pass. Explicit exports do not activate an invalid ordinary pair or infer annotation disclosure from source permission. |
| Frozen captures and conflict protection | [Source saves](../formal/source-save.md), [native source saves](../formal/native-source-save.md) and [ordinary annotation saves](../formal/ordinary-annotation-save.md) capture actual current content, serialization/binding/registration and required acknowledgment before admitted dispatch. Later edits cannot alter queued bytes. Final authority, full registration, captured binding and declared condition/probe checks remain necessary. Known external conflict blocks ordinary overwrite. Best-effort checking does not become universal compare-and-write; stronger requested guarantees require separately advertised capability or refusal. |
| Evidence and savedness | §§16.4–16.5 and [save continuity](../formal/source-save-continuity.md) separate application, observed bytes, synchronization, safe disposition, quiescence, causal/version evidence and current release. Baseline election uses qualified actual producer/artifact evidence, never receipt arrival or maximum source revision. Exact current-output equality is separate from external freshness and the revision named by a historical receipt. Native source and [annotation baseline installation](../formal/annotation-baseline-installation.md) checks pass. A source-save success cannot clear annotation changes; combined persistence has per-artifact outcomes unless an actual joint guarantee exists. |
| Failure unknown and retained obligations | Failed required synchronization can coexist with a completed write; unknown or contradictory effects remain owned and block incompatible dispatch/rebind. Cancellation/expiry/wait cutoff is not nonapplication, quiescence or refund. Late authenticated evidence remains attributable without rolling back authored state. The native producer/baseline/discharge checks and partial-save, active-expiry, wait-unknown and contradiction scenarios pass. New work cannot silently coalesce away an admitted save, refresh pinned intent or erase required provenance. |
| Explicit destinations and local lifecycle | [Source export/rebind](../formal/source-export.md) and [native SaveAs](../formal/native-source-destinations.md) distinguish copy-export, local rebinding and acknowledged binding activation. They check destination authority/identity, current ordinary pair, old/new effect barriers and captured intent. Native rebind, export, SaveAs and binding-history checks pass. A completed write followed by refused activation remains written-but-not-bound; no external rollback or atomic file-write-plus-local-publication guarantee is invented. |
| Bounds and observability | Finite profiles bound transcripts, chunks, candidates, artifacts, queued tasks, witnesses, comparison work and retained consumers. Preparation, effect occupancy and baseline promises have distinct admission/discharge rules. IF-04/10/12 and INS-01/02/03/04/07/08/12/13 separate encoding, dispatch, acknowledgment, savedness and retained pressure. Unknown is not zero, failure or safe reuse. L14/L15 retain system-wide signal/accounting review and E03 physical measurements. |

The audit located 43 exact existing check names once each in the complete adopted
report and confirmed they passed against the matching current inputs. This is
existing executed evidence plus its scoped contract/argument review, not a new
filesystem execution or proof of the host's actual receipts. The audited baseline
and report fingerprint remain unchanged.

L02 retains the public effect/task/result/event assembly; L04's cross-family
lifetime design is accepted above; L09 retains shutdown, scheduler/deadline and control
integration; L12 substrate-contract review. E01–E05 retain production Rust and
package visibility, actual filesystem/clipboard/platform procedures, resource/
clock/retention truth, physical costs, concurrent/crash refinement and assurance
execution. Required acknowledgment must describe the real provider procedure;
neither this sign-off nor a local mutex establishes external-writer isolation or
crash durability. No automatic retry, migration, uncheckpointed recovery or
general producer-transfer capability is inferred.

The reload scope disposition below removes an artificial prerequisite, not any
supported-path defect. A concrete contradiction in an advertised load/save path
reopens L08; optional unsupported profiles do not. Final model, clause and
cross-deliverable consistency remain L17/L18.

### L10 language and supplied evidence acceptance

L10 is closed at semantic-design and scoped-reference-evidence level against
specification 0.199/model 0.263. It covers the approved optional language service
with its required working Markdown/embedded-language reference, plus the shared
supplied-evidence boundary. It does not add structural commands, a universal
schema/semantic engine or language-specific mutation authority.

| Acceptance facet | Accepted contract and scoped evidence |
| --- | --- |
| Optional service and required reference | §§18/26.3–26.6 keep ordinary editing independent of parsers, grammars, schema evaluators and provider availability. The [pinned native reference](../formal/tree-sitter-reference.md) executes Markdown block/inline parsing and JavaScript child parsing/highlighting, not fallback alone. Its adopted native check passes. Diagram rendering, recursive injections, LSP and automatic semantic discovery remain outside required v1. |
| Assets and activation compatibility | [The reference manifest](../formal/tree-sitter-reference.json) pins Tree-sitter 0.26.0, Markdown 0.5.4 and JavaScript 0.25.0, their query/native/wrapper/license fingerprints and tested environment. Native checks reject mismatched fingerprints and unsupported registrations/query generations rather than claim compatibility. Query-only work reuses the same native forest; relevant grammar/discovery/registry/configuration changes cannot borrow an incompatible parse fact. Pins identify tested bytes, not publisher authentication, reproducible builds or the production platform matrix. |
| Committed source and incremental input | §18.5 and [parser source](../formal/parser-source.md) bind actual owned committed transformations, exact source revisions and LF-row/byte-column coordinates. Missing/gapped/incompatible/budget-exhausted journals require an independently admitted fresh snapshot or unavailable result, not an invented diff. Source-input and pinned incremental-versus-fresh checks pass; old published trees are copied before mutation and remain unchanged. Native coordinate bounds do not set the authoritative core's document-size limit. |
| Independent regions and source maps | Each fence remains part of one authoritative document and is parsed independently; same labels do not concatenate programs. Included ranges preserve original byte/point mappings and explicit host-prefix holes. Host delimiters/prefixes remain host-owned; derived edit targets crossing a hole or fence refuse instead of clipping. Native cases cover prose, multiple fences, fence/label changes, stale context and mapped captures. Ordinary exact-byte editing can still cross language boundaries through the common engine contract. |
| Fallback and coverage | The fixed host-controlled registry recognizes its JavaScript aliases; labels cannot name executable code or downloads. Unknown/unavailable labels, including unsupported Mermaid, incomplete fences and exhausted child work have explicit fallback/coverage dispositions. One child failure does not disable peers or host styling. Lexical query predicates needing an absent locals service are explicitly disabled, not assumed true. Native mixed-language and engine acceptance checks pass; display precedence is deterministic within the selected profile, not a theme or rendering proof. |
| Work ownership and current release | §18.7–18.8, [analysis work](../formal/analysis-work.md) and [native binding](../formal/native-analysis.md) bind actual source/context/provider/task and one submitted parser lease per analysis identity. Matching views share accepted results; independent contexts do not unify by equal bytes. Cancellation disables use without inventing quiescence or refund; base holds and pending native drop remain charged. Actual native process-death/drop/close controls and current shared-view release checks pass. Engine F08 reads never parse or disclose native handles; empty ranges return no captures under the adopted positive-span guard. |
| Supplied assertions and candidate binding | §§26.3–26.6, [validation reports](../formal/validation-reports.md) and [owned preparation](../formal/preparation-boundary.md) distinguish source/candidate subjects, named interpretation/definition/options, coverage, conformant/nonconformant/indeterminate verdicts and insufficient/established/refuted/contested satisfaction. Accepted descriptor claims cannot be removed by a caller. Authenticated/admissible required-report intake, identity conflicts, bounded retained responsibility and explicit authorized definition closure precede evidence use. Their existing scoped checks pass. URI names cannot fetch code; syntax/shape is not provider truth or complete semantic discovery. |
| Bounds threats and instrumentation | Profiles bound source/tree/query work, regions, captures, tasks, contexts, bases, native records/messages and definition/evidence closure. Expected malformed source is editable input, not an invariant violation. INS-01/02/03/05/07/08/09 distinguish asset admission, source handoff, parser lease/work, region/query coverage, current acceptance/release and native retirement. STRIDE/LINDDUN/hazard coverage includes forged contexts/maps, unsafe native loading, tiny-fence floods, stale evidence and retained-source leakage. L14/L15/L16 retain the complete system-wide audits. |

Nine exact existing report entries were checked: parser source, native binding,
analysis/shared-view release, pinned native mixed-language execution, structural
targets, named assertions, owned required-evidence preparation, definition closure
and half-open query-range solver conditions. Each occurs once and passes in the
complete adopted report, whose current inputs and fingerprint remain unchanged.
Finite native comparisons and conditional arguments retain their own scopes;
neither package checksums nor a parser result prove grammar completeness.

L02 retains complete conditional F08/F24/F28 public products, discovery and result/
event assembly, including any pending value-guard integration. L05 retains the
system-wide configuration/registry audit; L11 both frontend renderings; L13 the
actual package/support matrix. E01–E05 retain production Rust, trusted provider/
asset loading and authentication, actual policy/retention/clock truth, physical
native allocation/reclamation, concurrency/crash/fairness/preemption and independent
assurance. A verification process deadline is not a native halting guarantee.
Child parsers may reparse freshly; incremental child reuse and a locals service
are not advertised by the accepted reference. Broader providers need their own
explicit profiles rather than inheriting unsupported guarantees from this one.

The shared supplied-evidence contract is accepted; automatic schema evaluation,
arbitrary content-model completeness and new structural verbs are not required
to close it. A concrete missing supported language behavior or contradictory
evidence may reopen L10. Final clause/model/interface consistency and separate
adversarial review remain L17/L18.

### L12 accepted substrate procedure and guarantee contracts

L12 is accepted at the design/scoped-reference level using adopted specification
0.202/model 0.266. The engine-facing contracts specify what evidence is required
and what it means; they do not assert that a concrete provider supplies it.
Accepted L04/L08/L10 lifecycle, persistence and language contracts are reused.

| Acceptance facet | Existing sufficient contract |
| --- | --- |
| Port ownership and procedure | Specification §§1.2/17.1 assign semantic capture, validation and evidence adoption to the engine, and resource resolution, byte conversion, replacement, synchronization and attribute/security preservation to the substrate. Explicit effect requests name provider/capability, exact origin/attempt, required completion meaning, limits and trusted supplier. Ordinary edits perform no hidden storage I/O. |
| Identity, aliases and continuity | §11.1.1 and [source resource reads](../formal/resource-read.md) separate document incarnation, binding and provider continuity; declare comparison domain/validity; refuse unknown evidence as known distinction; serialize final alias joining without replacing the live authored owner. The exact read/write registrations retain full bodies/generations and captured evidence. A locator, content equality or opaque token is not identity, ordering or permission. |
| Read/write evidence and synchronization | §§16.1–16.5 and [source saving](../formal/source-save.md) distinguish captured bytes from an atomic physical snapshot, application from synchronization, safe disposition from quiescence, and historical acknowledgment from current savedness/freshness. Requests/receipts bind actual task, origin, provider/resource and immutable capture; schema/attestation checks precede contradiction handling. Ordinary best-effort saving remains valid without stronger optional CAS/atomicity; explicitly stronger requests cannot degrade silently. |
| Annotation and destination roles | §§16.2.3–16.3.5/16.5.9 and L08 require known-distinct ordinary source/annotation destinations and one shared destination writer, exact captures, independent artifact outcomes and separately qualified baselines. Export and SaveAs do not acquire ordinary-save authority by renaming a role. Local publication does not promise atomic multi-file persistence or distributed writer exclusion. |
| Clipboard capabilities and completion | §21.1 and [clipboard products](../formal/clipboard-products.md) define named destinations/registrations, literal capture, current dispatch/release, exact receipt correlation, declared acknowledgment and independent retention/drain. Export acceptance precedes renewed source validation for cut; failed local deletion is copied-not-cut. No universal physical transfer, retention or durability follows from a profile string. |
| Clock/service and cleanup | §§17.4/28.22 select same-epoch monotonic duration, declared phase/suspend behavior, inherited nonrenewable remaining limits, final expiry checks and refusal/stopping when required clock evidence is invalid/unavailable. Conditional F26 facilities do not create a mandatory timer service. Stopped queued work, issued unresolved effects, quiescence, delivery drain, retained provenance and spent issuance keep independent discharge rules. L09 still owns complete shutdown composition; deadline expiry is not native preemption. |
| Required native reference and guarantee honesty | L10 retains actual pinned mixed-language integration and native handle lifecycle evidence under its scope. §25 requires explicit supported profiles; negotiation grants no authority or weaker guarantee under the old name. [Substrate boundary evidence](../formal/engine-substrate-boundary.md) checks selected post-import paths and injected hidden calls, not static host-free packaging, every API, arbitrary code containment or actual provider truth. |

The adopted 760-check/946-input report and all current qualified inputs match
the fingerprint recorded in L16 below. Relevant existing resource-read, source-
save, annotation-save, clipboard, substrate and native-language checks pass.
A separate reviewer independently matched nine relevant normative/reference
artifacts to that report and found no unmet L12 design criterion. No new
procedure, platform default, model or test program was introduced for this audit.

L02 retains incomplete public/evidence assembly; L09 shutdown; L11 UI integration;
L13/U03 supported package/platform commitments; L15/E03 complete accounting and
physical measurements. E01/E02/E05 still requires actual provider procedures,
alias/coherence/attestation truth, rename/synchronization/attribute guarantees,
clipboard ownership, clock/service, native cleanup, concurrent/crash integration
and production conformance. A mock, receipt validator or scoped interception
cannot close those gates. L17/L18 retains final correspondence and handoff.

### L15 accepted resource contracts and logical accounting

L15 is accepted at the design/scoped logical-accounting level against adopted
0.207, including the query-workspace fix. A separate bounded review found no further
unresolved resource-policy choice in the existing design contracts:

- §§17.2.1–17.2.2 distinguish occupancy from consumed work, identify independent
  accounting coordinates and require conservative demand and valid finite profiles.
- §§17.2–17.2.4 require an owner, admission, retention, refusal and justified
  discharge for supported obligations. Preparation includes all required companions;
  cancellation cannot refund another obligation or replenish consumed issuance.
- Protected control and retained lifecycle contracts preserve cleanup under ordinary
  pressure. Activation cannot confiscate existing commitments; §28.35 retains captured
  interpretation or refuses incompatible activation.

The former resource reference-evidence exception was F08 query construction: 0.202
did not separately reserve its normalized query workspace. Adopted 0.207 funds
two construction-envelope frames independently of the delivery ceiling and checks
actual results before release. The exact registered checks now pass in the complete
765-check qualification, with matching source/root endpoints and verified archive.
The failed 0.204 report itself remains unadopted.
The 3,113-byte construction versus 64-byte reservation witness belongs to the first
unadopted 0.204 draft, not adopted 0.202; that draft was corrected before freezing.

Acceptance covers the design and scoped logical-accounting
evidence, not actual heap usage, measured production limits, scheduling fairness or
complete L02 facade assembly. Physical charging and measurements remain E03.
Private-helper wrapper symmetry is not an additional acceptance criterion; complete
ownership and accounting on supported paths is.

### L16 accepted local security and privacy contracts

L16 is accepted at the semantic-design/scoped-reference level by reuse of the
adopted specification 0.202/model 0.266. Requirements §§15.1–15.6 have the following
contract dispositions. No new behavior, supported profile or proof machinery is
introduced by this acceptance.

| Acceptance facet | Existing contract and evidence disposition |
| --- | --- |
| Threat scope and trusted authority | Specification §24.3 and model TB-01–07/ASM-01–07/THR-01–07 identify supported boundaries, assets, attacker-controlled inputs, trusted suppliers, prevention/containment and acceptance obligations. All six STRIDE classes have explicit security-test requirements. Deferred network identity/federation has no active v1 pathway; optional host exporters/providers require their declared profiles before use. A threat inventory is not evidence that every threat has been discovered or mitigated in code. |
| Capability provenance and currentness | §§12.1–12.2/12.6 bind the actual recipient, incarnation, data scope, purpose/destination, captured ceiling and accepted current policy at a serialized owned-value handoff. Discovery, caller labels, membership, correlation and retention grant no authority. L04's accepted ownership/invalidation/discharge composition is reused; host policy truth and actual synchronization remain premises, not Boolean-guard theorems. |
| Inert source and annotations | §§13.1/13.3 distinguish provenance from permission and prohibit authored instruction execution, implicit URL fetching and rich-body evaluation. Source, annotation, historical and outbound grants stay distinct. Accepted L07 covers annotation authoring/mapping/history; trusted native assets are not an untrusted-code sandbox. |
| Boundary validation and fault isolation | §§17.1/23.1/24.3 require validation at actual request, context, persisted-data, provider-evidence, asset and diagnostic boundaries. Authentic evidence is separate from current applicability. Known malformed input is not supplier contradiction; quarantine is justified and scoped. Ordinary failure, cancellation, fault, unknown effect and quiescence retain different meanings. No native/process-fault containment or crash recovery is inferred. |
| Minimal disclosure and observation | §§12.4/12.6/13/23.1.1/23.2–23.4 cover whole-report projection, hidden counts/tokens/cache reuse, current queued release, collection allowlists, independent authoritative outcomes and honest diagnostic loss. Already disclosed snapshots/copies are not revocable; constant-time/resource privacy, host inspection and secure erasure are excluded. ACS-02 proves only its fixed bounded diagnostic projection under its stated schedule/assumptions, not whole-engine noninterference. |
| Supply chain and maintenance acceptance | §§24.2–24.3/25/28.47 require reviewable/replaceable parsing/executing dependencies, asset provenance/license review, material-boundary review, synthetic regression for confirmed vulnerabilities, real reporting/triage/update ownership and a supported-fix policy before release. No contact, response deadline, platform guarantee, audit cadence or indefinite support promise is invented. |

The current adopted report contains 760 passing checks over 946 matching inputs;
its SHA-256 is `a30c00a421300742d8fc0e41fc7d112b7cc9fe1638050d242b183767cc33740e`.
Fifteen relevant existing release/projection/annotation/diagnostic/privacy entries
were inspected and each passes, including mediated context retirement, native
annotation queries, reversal receipt release, diagnostic capture/drain, paired
disclosure and late privacy regressions. This is reuse of executed scoped evidence,
not fifteen new tests or a complete product-security proof. A separate reviewer
found no unmet L16 design criterion and independently matched the relevant current
source hashes; that review is not independent Cleanroom certification.

L02/U01 retains missing facade/result/event assembly. L11/L12/U03 retains concrete
UI/platform profiles; L14 retains exact measurement schemas and installed probes;
L15/E03 retains complete resource accounting and physical measurements. U04/E04
still requires real maintenance/reporting/support commitments. E01/E02/E05 retains
production implementation, actual host truth and executed unit/fuzz/concurrency/
integration/exhaustion security assurance. Requirements §15.2 is not declared
implemented by this design acceptance. L17/L18 retains clause/model reconciliation
and final adversarial handoff. A concrete security-contract contradiction may
reopen L16; generic historical assurance wording alone does not.

### Bounded challenge of the recent acceptance records

The separately authorized assurance reviewer challenged L07/L08/L10 against the
current requirements, selected contracts and adopted 0.199 evidence. They found
no material must-fix: conditional reload, conservative history restoration/refusal,
retirement candidate versus native installation, and actual JavaScript child parsing
with truthful Mermaid fallback preserve the required scope. Public assembly and
cross-family lifetime work remain L02/L04 rather than being hidden in release gates.
The review checked report flags/counts and matching hashes for requirements,
specification, model and nine relevant reference documents. It did not execute
the suite or exhaustively inspect code, and is not independent Cleanroom
certification. L18 still requires the final whole-specification challenge.

### Reload scope correction

U02 was incorrectly treated as a mandatory v1 design blocker. Requirements
§§1.1–1.2 require reference file loading/saving, explicit lifecycle and external-
change protection; §5.9 makes reload a path "where supported" and calls it
optional. Neither that conditional wording nor editor precedent adds a required
in-place replacement capability. File watching and automatic external merging
remain excluded.

The selected source-load contract in specification §§11.2.1/11.2.17 and
[load invocations](../formal/load-invocations.md) is explicit: a new resource
creates an owner, while a known alias joins the independently authorized existing
owner without replacing dirty source, comments, history or views. F17's
`activate-loaded-candidate` label does not advertise a general reload. Required
external-conflict refusal and supported save-elsewhere/rebind paths remain in L08;
loading file bytes cannot bypass them. Safe disposal followed by a fresh open is
a different lifecycle, not a continuity-preserving reload substitute.

The precedents support different reload tradeoffs, not a universal default:
[Neovim](https://neovim.io/doc/user/options/#'undoreload') has configurable
whole-buffer undo retention; [Helix's current source](https://raw.githubusercontent.com/helix-editor/helix/master/helix-view/src/document.rs)
applies a diff transaction and appends history; the
[Emacs manual source](https://raw.githubusercontent.com/emacs-mirror/emacs/master/doc/emacs/files.texi)
describes undoable reversion and a finer-grained preservation alternative whose
configured time is not a whole-operation deadline. These are current primary
references, not pinned Vellum dependencies or proofs of annotation continuity.

No reload algorithm, dirty-discard permission, history-loss fallback or new
operation is adopted. U02 is removed from mandatory technical closure. If a
public/reference surface actually advertises in-place reload, its supported
contract must be completed before that surface is accepted; the conditional
requirement cannot be evaded by calling a reload ordinary activation. User
selection is needed only to add that capability, not to finish the existing
initial-load design. L08's selected load/save contracts are accepted above.

## External evidence retained for release

Design completion requires specifying each applicable acceptance procedure and
assumption, not claiming that its future execution already happened.

| ID | Gate retained after design closure | Required evidence |
| --- | --- | --- |
| E01 | Production implementation and package conformance | Actual Rust APIs/manifests/builds, both adapters/apps, dependency/license inventory and implementation-to-contract tests. |
| E02 | Real provider and platform behavior | Named filesystem/identity/synchronization/clock/clipboard/geometry/IME/accessibility procedures with supported-platform integration and failure evidence. |
| E03 | Physical limits and measured service/reliability | Actual allocation/charging, representative workloads, hardware, clocks, saturation, loss/censoring and uncertainty before numerical defaults, budgets or service claims. |
| E04 | Accountable operational commitments | Real security reporting/triage/update owners, supported-fix policy, release communications and exercised maintenance procedures. |
| E05 | Executed assurance processes | Actual CI and review-role assignments, implementation/concurrency/crash acceptance and any separately promised independent certification. The design's own adversarial review remains L18. |

These gates remain mandatory for corresponding release claims. Moving their
execution out of a design item does not satisfy or waive them.

## Decisions and questions for user review

| ID | Question and disposition | Affected deliverable |
| --- | --- | --- |
| U01 | Resolved by user approval: a typed, permission-filtered facet of the existing retained-outcome query. Adopted 0.219 §23.1.10 implements eight native facet kinds with bounded replies, captured/current native release and independent protected control. Focused checks, selected second-agent challenge, full frozen qualification, source/worker/root audit and retained archive integrity check pass. N02 is accepted for those eight kinds, not unspecified future results or whole L02. | L02; settings and lifecycle result integration. |
| U02 | Scope disposition: not a mandatory v1 decision. The selected load profile is initial birth/join and cannot overwrite an existing owner. In-place reload remains conditional and unadvertised by that profile; adding it would require an explicit continuity/dirty-state/history contract and user scope choice. See the reload scope correction above. | L08; no blocker for initial-load closure. |
| U03 | Exhaustive versioned authority-bearing executable enums are selected by user approval. Actual Rust/package/MSRV and host support choices still need concrete review before promises freeze. Modular substrates/local filesystem first are selected; the exact OS/procedure matrix is not. UI support choices belong to the separate UI stream and are not engine blockers. | Engine L13; separate UI L11; E01–E02. |
| U04 | Real reporting/maintenance ownership and security-support commitments require accountable user choices. Their absence does not block unrelated technical contract drafting. | L13/L16; E04. |
| U05 | Pending user choice: place the trusted host-defined command descriptor/handler registry in the headless integration adapter above the engine (recommended), or host it in the engine facade outside coordinator guards. Requirements §11 and specification §22.2.3 require its semantic contract but do not settle registry placement. Handler execution/affinity is adapter-side either way. No relocation or exclusion is adopted yet; PH-09's exact context/handler correspondence, authorized discovery, retained lifetime and validation of emitted engine requests remain required. | L02/L13/L17/L18 ownership and handoff reconciliation; concrete Rust/host execution remains E01/E02. |
| U06 | Pending user choice: F19 revoke-observation-grant targets one exact engine-owned observation context (recommended for local single-user v1), or an explicitly modeled grant and every dependent context. Requirements §15.5 defines release/revocation safety but not this target granularity. A context-local action cannot claim grant-wide revocation, and existing stop cannot silently implement external policy mutation. [Options, tradeoffs and proof obligations](observation-revocation-decision.md) record the decision. No schema, authority gate, stop alias or new grant/cohort registry is adopted. | L02/F19 and L09 release ordering; L17/L18 reconciliation. |

This records currently evidenced questions, not a claim that no further ambiguity
can exist. A newly discovered question must name the affected requirement and
observable tradeoff. Previously adopted defaults, both point profiles and distinct
ordinary destinations must not be reopened as undecided.

## Decisions and residual package crosswalk

| Existing topics | Closure owners |
| --- | --- |
| D01–D03 | L02–L03, L09 |
| D04–D06 | L02, L06–L07, L11, L14–L15 |
| D07–D12 | L04, L12, L14–L16 |
| D13–D14 | L07, L15 |
| D15–D16 | L08, L12 |
| D17–D18 | L06 |
| D19–D21 | L07–L08 |
| D22–D23 | L09, L12, L14–L15 |
| D24–D26 | L05, L10, L13 |
| D27–D30 | L06, L11, L13 |
| D31–D32 | L02, L11–L12 |
| D33–D35 | L01–L02, L05, L11, L15 |
| D36–D38 | L14–L16 |
| D39–D41 | L15, L18; E03/E05 |
| D42 | L04, L16 |
| D43–D44 | L13; E01/E04 |
| D45–D46 | L03, L06, L10 |
| D47–D48 | L13, L16–L18; E04/E05 |

| Existing package | Design closure owners |
| --- | --- |
| C1 Configuration | L02, L05 |
| C2 Dependencies | L03, L05, L10 |
| C3 Public products | L02, L04, L08–L09 |
| C4 Editing and interaction | L06, L11 |
| C5 Annotations and history | L07–L08 |
| C6 Assets | L10, L13 |
| C7 Hosts and packages | L11–L13 |
| C8 Resources and observation | L14–L16 |
| C9 Assurance | L17–L18 |

L01 governs all packages. A whole C row no longer stays design-open merely
because a separately retained E gate has not executed; its actual technical
criteria above must still be met and reviewed.

## Native shutdown fence counterexamples

The [joint shutdown reference](shutdown-composition-review.md#selected-executable-reference)
now has executable negative evidence, not just a fence assumption. Four actual
native paths remain possible after its wrapper accepts shutdown: invocation
issuance, source-save admission, annotation-save admission and publication of an
already-admitted compound edit. Both save admissions can add obligations without
changing the root, so checking root installation alone cannot close L09. The
existing eight joint and three same-document controls, with four and two unsafe
variants respectively, still pass; their save-readiness evidence is not withdrawn.

The next closure must bind admission and final publication to the same actual
authority-owned fence, preserve only the already-qualified original save duties
and protected service, and account for future owner creation. A domain fault is
not that fence. No whole L09 closure, new shutdown policy or frozen qualification
is claimed by this negative working audit.

Unadopted candidate 0.216/model 0.280 now implements the selected actual-domain
ordinary-admission primitive behind those four gaps. Twelve controls and fourteen
coherent copied-module mutations pass, with strict original context/root/control,
exact parent headroom, one retained absorbing fence, observational matching retry,
prepared-token crossing, reentry exclusion and exception unwinding. Both original
source/annotation save completion orders and protected stop/retirement remain valid.
Six adjacent baselines and the shared-invocation, source-read, source-save and
annotation-save mutation banks pass. One old source-save fault injection was
retargeted to remove the new redundant section guard and the actual installer
guard; all twenty-seven original mutation identities and assertions remain.
Read-only review found no must-fix at this primitive's scope. Normal census,
ownership/dependency, isolated entrypoint and exact registered-predicate checks
pass. The F18 cancellation regression also passes all 38 forms/twelve native kinds,
9,842 traces/27,588 edges and twelve original mutants. The corrected 984-input
batch passes all 773 checks and is adopted as 0.216/model 0.280 after source,
worker, root and retained-archive endpoint/integrity audits.
The original run exposed a stale annotation-export fault-injection anchor. The
corrected frozen source retains all thirty controls and twenty-nine original
mutant identities; it removes the redundant admission fault guard together with
the actual installer fault guard. The original failed run is not qualified by
this correction and remains separate from the new execution.
This does not accept shutdown consent, owner/eligible-dispatch
inventory, all native entrypoints or L09; it does not change UI/integration ownership.

The [actual owner inventory](shutdown-composition-review.md#actual-owner-inventory-and-shared-attachments)
now distinguishes separately bounded registries, parent-funded pools and shared
attachment edges. The source writer is authority-bound and can serve several
actual domains; source-read registries and history owners can multiply within one
domain, while invocation observation can bypass its admission pool through direct
context acquisition. Whole-owner release cannot be inferred from task retirement.
The [negative inventory probe](shutdown-inventory-probe.mjs) reproduces two new
source-read registries, two history owners and two direct observation contexts
after the candidate's actual fence, without increasing domain charge accounting.
It validates the retained frozen input hashes before the probe. This changes the
next integration step: bind actual factory/attachment acquisition and aggregate
inventory funding to the domain fence; do not accept a flat caller-provided list
or claim complete shutdown from the four selected refusal checks. These working
sources do not close L09 or withdraw the selected fence's existing scoped evidence.

## Historical qualification notes

The following records preserve the state at their named candidate revisions.
References to running sessions, pending adoption, unresolved U01 and older
deliverable counts are historical, not current instructions or live-process
claims. The baseline/reporting and northbound disposition above govern current
status. In particular, 0.210, 0.218 and 0.219 are adopted; U01's route is selected
and its eight-kind receipt binding is accepted at N02's stated scope.

The 0.214/model 0.278 ownership/discovery reconciliation is adopted after all
771 checks over 978 inputs, source/worker/root endpoint audits and byte-exact
archive verification. Its thirteen profiles retain 303 profile/variant occurrences,
43 current engine metadata variants and all original family indices; six
oracle-sensitivity controls challenge the ownership check. The complete catalog
still has 71 engine, five integration and eleven presentation variants. This closes
the discovery ownership reconciliation, not a further whole deliverable. The
source report and retained archive are identified at the top of this ledger.
The analysis-owner binding is now adopted as 0.215/model 0.279 after 772 checks
over 981 inputs and matching endpoint/archive audits. Its publication audit
isolates the inserted analysis registration from the preceding result-guard
registration and retains both validators. Session 40131 is terminal success and
must not be polled or restarted. The original 0.216/session 72072 is terminal
failure: 772 of 773 checks pass; the annotation-export fault-injection construction
anchor is stale. Its source and worker report remain retained and unqualified.
The corrected frozen successor completed successfully as session 68759 and is
adopted as 0.216/model 0.280. Its original worker is retained byte-for-byte with
the original capture manifest and both reports in `.verification-evidence.CekvNe`.
Neither terminal handle may be polled or restarted. The original failed 0.216
source remains unqualified; the successful successor does not retroactively
qualify that execution.

The editable owner-inventory successor now connects twenty-four actual factory/pool
types. Focused composition controls and coherent attachment mutations pass;
these do not close the all-owner shutdown relation. Baseline reservations fund their complete local ceiling at the actual
parent, retain their edge on stop, and reject reentrant duplicate creation after
limit capture. All twelve original baseline controls, sixteen publication-floor
route cases and twenty original mutants pass; the duplicate-pool mutant removes
both redundant singleton guards. Catalog reads retain their existing native parent
lease and add metadata-only attachment funding. Whole retirement captures strict
discharge facts once and rechecks actual owner identity and retained children
before releasing either reservation. All twenty-eight original catalog controls,
forty-nine original mutants and four decoder-fault cases pass; the previously
guard-blocked reopening variant remains excluded from mutation detections.
Serialization retains an actual private pool identity while preserving the
boolean facade and its native charge. Eight focused controls cover first creation,
strict protected discharge, retained bases, distinct domains and combined parent
headroom. Settings retains its actual lazy pool identity across nine fixed ports;
twenty-eight controls cover its five first-creation forms, shared funding, retained
products, strict discharge and a native exception after pool creation. A pool that
survives such an exception remains attached and funded rather than being mistaken
for an aborted acquisition. Both extensions pass adjacent settings, scoped
definition/lifecycle, serialization, disposal and ownership-boundary regressions;
read-only review found no must-fix at their first-creation scope.
Observation admission now attaches its pool and actual reservations separately;
decoder workspace is reserved before product capture, without charging the native
context ceiling twice. Known setup refusal releases that workspace; an unknown
setup exception retains it rather than inventing discharge. Strict protected
retirement snapshots discharge facts and rechecks the original reservation after
possible reentry. Fourteen focused controls, the original admission/decoder cases
and mutation banks, decoder fault cases and native observation regressions pass.
This candidate is not frozen or qualified.
The [shared-writer funding refinement](shutdown-composition-review.md#candidate-shared-writer-funding-relation)
now fixes the candidate accounting owner: one immutable actual-domain sponsor
funds the shared node once, while each attached domain funds its own metadata.
The sponsor remains retained; no whole-writer retirement or transfer is invented.
Actual writer records and conservative native-ledger funding now exist in the
editable reference. Native source-reader construction, native destination
preparation and validated annotation provenance establish actual attachments;
ordinary annotation admission requires an existing actual binding. Original
legacy work cannot be retroactively adopted as funded native work. Standalone
supplied-root annotation export remains a legacy helper, not covered native ingress.
Review reproduced and closed the unbound ordinary-annotation ingress and failed
provenance staging seams. Twelve writer controls include actual peer tasks,
sponsor fencing, overflow/refusal and eligible source-save drain/baseline adoption;
seven new semantic mutations and the retained save mutation banks pass. The
reviewer found no further must-fix at this scope. No Rust implementation or
whole-writer retirement is established.
The native publication observer now has an actual-domain wrapper, parent-funded
local ceiling and metadata, original-scope-before-loss checks, and roots obtained
only from its actual domain. Its local publication summary comes from an actual
terminal invocation ticket; copied/foreign tickets and caller facts cannot
substitute. Rich reasons/participants, generic effects and recovery evidence are
not imported from another recipient's classification. Oversized prepared outcomes
refuse before copying. Stop and protected outcome retirement keep the owner and
its reservation retained. Twelve focused controls and ten new semantic mutations
pass; the complete focused owner run passes 174 controls and 84 mutations.
Selected review found no must-fix; no full F19/N03 or physical claim follows.
The editable successor now separates new obligations from retained service on
existing owners. Selected source/annotation reads, analysis tasks, history,
discovery, catalog contexts, registration establishment, source/search/settings
contexts and annotation/coherent captures reject new work after the actual
ordinary-admission fence. Existing reads, registration resynchronization,
authenticated analysis evidence and protected discharge remain available. The
first combined check passed 207 controls and 104 coherent mutations.

Two additional mixed cases are corrected in the successor: registration capture
may return an identical retained basis after fencing but cannot issue a new one;
an existing settings pool cannot allocate another context or write/definition/
lifecycle basis after fencing. Existing settings products remain usable and
releasable. Focused controls and the original 32 registration-basis mutations
pass; that combined successor run passed 216 controls and 106 coherent mutations.

New plan creation and exact retry are now separated for the five settings-plan
forms and direct/scoped source serialization. After fencing, a matching retained
plan is returned through its existing validation path without another allocation;
new plans and fresh serialization bases refuse. New baseline promises also require
open admission. Retrieving an existing write hold uses a lookup-only path that
cannot allocate if caller facts reenter and change its claim. The preceding plan/
reservation run passed 231 controls, and eleven targeted coherent mutations detect
removal of these new guards or substitution of allocating lookup. This selected
run does not claim that all previous mutations were rerun on these latest edits.

Ordinary annotation baseline preparation, validation and commit now require the
original actual-domain admission-section token through their confined producer.
Selected root installers exclude fence installation during fact reads and commit.
New source/annotation activation, live lifecycle changes, catalog changes and
container changes cannot cross a fence; qualified baseline adoption and the three
disposed binding/provenance releases retain their original guards and can proceed.
Fourteen focused controls cover these distinctions, including prepared tokens,
unchanged owner frames, fact reentry and exception unwind. Generic disposal does
not substitute for the still-required shutdown-consent controller.

Existing regressions exposed two earlier candidate defects. The writer now accepts
genuine previously exposed historical roots using a weak identity witness, without
accepting copied roots or bypassing their actual domain's current fence. Native
source-destination fact preflight now excludes fence installation before selecting
or funding a first writer attachment. Historical source-read consumers and the
original twelve fence controls pass again. The consolidated owner/root baseline
passes 246 controls. Dependency review also reproduced and corrected recursive
source-hold decrement and search lost-capture/fault defects. Six composed release
cases span source, combined annotation and coherent version-set issuers; strict
discharge facts are captured once and the original hold is rechecked after fact
capture. Existing context/search mutation banks and the established bounded search
traces pass, with attachment-frame charges/refunds kept distinct from native cost.
Other preparation paths, complete dependency/tombstone closure and shutdown
acceptance remain required. These edits
are unqualified reference-model
changes, not Rust engine implementation or a change to the adopted 0.216 baseline.
No whole L02 or L09 deliverable closes.

The full-suite pass exposed a configured-line consumer that could create the
native settings pool without its attachment. The editable repair routes that
creator through the existing transaction and distinguishes new plans from exact
retry. Its targeted mutation and the commit-section-release mutation pass against
a 249-control owner baseline; all 9,842 original cancellation traces also pass
after test-only parent headroom is supplied for requested observers. Delaying a
measurement fixture until its test runs keeps module construction distinct from
semantic fault detection. Settings-query refund oracles are being reconciled with
metadata frames, and source-read mutations are partitioned into bounded processes
without removing scenarios. These repairs still require focused validation and
a new frozen qualification; the first capture cannot qualify them retroactively.

Adopted 0.211/model 0.275 addresses the F17 temporary capture
exception without changing its existing products: the actual parent reserves four
M-sized serialized frames before either lifecycle-envelope normalizer and refunds
only those temporary units under the existing slot guard. Fourteen forms/two routes,
42 headroom boundaries, fourteen refusals/fourteen faults and four semantic mutants
pass. Both prior envelope publication/fault suites and nine further adjacent suites
pass, including 124-module ownership/entrypoint checks and the exact new registration.
Read-only review found no must-fix in the logical-frame scope. An older resource
mutation's non-unique target was made specific to its original funding function;
its original semantic assertions still pass. Physical/hostile-input, full F17 event/
result assembly and F18 fence integration remain separate.

The 973-input snapshot fingerprint is
`a952d8cc7f981fbdc688d1e9329170830bb884081bc70f58d8d0f2d56855b142`.
Its complete suite terminated successfully with 769/769 checks on session 1759;
that terminal session must not be polled again. Source/worker and root endpoint
audits pass, including 299 conditions/fifteen manifests/81 baseline conditions.
All thirteen changed files (three additions, no removals) were hash-guarded into
the root. Exact report publication retained the prior 0.210 report. Report SHA-256
is `369fd0beac3d3a4c5b1a0d222b39f24ae5db03cd6483d6b8795f0da91a02e56d`;
worker report SHA-256 is
`15b2c8f7751a418b5636d7ac9bfbdb06c08f828049bfd1def142f0baa2cadb5e`.
The `.verification-evidence.shfO7F` read-only archive integrity verifier passes.
Its initial manifest lacked the verifier/version metadata; adding those fields
fixed archive validation without changing any qualified source or report byte.
Publisher/archive-copy scripts are consumed and must not be replayed. This closes
the actual-parent temporary-workspace gap for the fourteen selected F17 forms,
not full F17/L02 or a whole engine deliverable. The current adopted root is
0.213/model 0.277 after the clipboard ownership qualification recorded below;
the frozen 0.214 and 0.215 batches remain running and unadopted.

Clipboard ownership is clarified in the [UI/integration boundary](reference-ui-design.md#clipboard-integration-boundary).
Gestures are UI work; native clipboard access is provider work; clipboard-specific
workflow orchestration belongs above the semantic kernel. The behavior facade
retains explicit captures, exact edits, authority, atomic publication, linked
history/companions and truthful external-effect contracts. Existing F22/F23 and
F11 obligations are not removed or accepted merely by assigning mechanisms to an
integration module. The unadopted draft proposing an invocation-owned clipboard
registry is held, not registered, qualified, kernel-integrated or adopted. Its
argument-only checks cannot establish the missing original task/issuer/retry
assembly. L17/L18 still need the final normative/package/model reconciliation.

The user subsequently selected the headless integration facade for clipboard
workflow verbs and clipboard-specific task state. Adopted 0.213/model 0.277
updates requirements §§3.1/3.3/10.1, specification §§1.1/6.5.3/21.1, CMP-10/PATH-06,
the ownership catalog and separate engine/integration projections. All 87 labels
and original family indices remain: 71 engine, five integration and eleven
presentation variants. No transfer behavior or engine authority safeguard is
removed. This is not a UI reclassification or acceptance of the held core draft.

Focused evidence passes 44,544 gate cells per projection, eighteen controls, ten
semantic mutants, 121 UI frames, ten undispatched integration frames, the 28-family
catalog and 124 isolated native/reference entrypoints. Separate read-only review
found no must-fix. Immutable integration descriptors are checked against tampering
into another control profile. These tests explicitly do not establish actual
integration execution or a clipboard-free production kernel/package graph.

The source census is reconciled to 1,121 rows/units: changed §3.1/p5 and §3.3/b9,
new §10.1/p1 ownership text and the prior reference paragraph shifted to p2.
Eight census sensitivity controls pass. All ten §5.3 units and 41 publication
fragments remain byte-for-byte semantically unchanged; only the global source
fingerprint changes there, and all sixteen partition controls pass. The new
requirements SHA-256 is
`6807f406fc8e32dc2f83b8076ec0928b0940dbd2b01cc9aece6d124b80e4f590`.
The existing graph generator still yields 168 elements; its CMP-10 label now
records integration ownership. Census/graph coverage is not semantic closure.
Full frozen qualification and endpoint adoption pass; the current matrices and
ownership notes are reconciled. L01 is reaccepted for this approved ownership refinement;
L02/L09/L13/L17/L18 remain open, with integration-specific residuals retained
outside engine completion and still required for the wider editor integration.

Adopted 0.213 is frozen at 977 inputs and all 770 checks pass.
The exact registered ownership predicate and adjacent native/schema discovery
regressions pass. The full process terminated successfully; that terminal session
must not be polled again. Source, worker and root endpoint audits pass, with 299
conditions/fifteen manifests/81 baseline conditions. All sixteen changed files
(one addition, no removals) were hash-guarded into the root. The previous report
is retained separately. Qualified report SHA-256 is
`fdeb965ce59216c07b738bf71870bdd38db5624c9b0f45c99faa0c10c85f6186`;
the byte-exact `.verification-evidence.SEONBp` archive integrity verifier passes.
Its publisher/archive-copy scripts are consumed and must not be replayed. This
closes the reopened ownership reconciliation, not integration task/retry/effect
conformance, production kernel/package separation or L02.

A separate 0.214/model 0.278 follow-up corrects stale current discovery
and boundary prose while explicitly preserving historical qualification scopes.
Its new native metadata/ownership crosscheck passes all thirteen existing
profiles: 303 profile/variant occurrences, 43 current metadata variants, all
within the 71-variant engine projection. None advertises one of the five
integration workflows as engine behavior. Six explicitly named oracle sensitivity
controls and the exact registered predicate pass; these are not implementation
mutant kills or handler-execution proof. The unchanged 1,121-unit census passes,
and graph regeneration retains the same 168 elements and graph bytes. This
follow-up is frozen at 978 inputs and its 771-check full run is active; it has
not entered full qualification or adoption and closes no additional deliverable.
Read-only code/proof-scope review found no must-fix. The reviewer's focused launch
lacked the pinned ICU4J environment and stopped before checking, so only the
main pinned-environment execution is a focused pass. The running 0.214 and 0.215 batches
are left untouched.

The repaired 0.207/model 0.271 batch is now qualified and adopted: 765/765 checks,
957 matching inputs, source/root endpoint audit and byte-exact evidence archive
integrity verification. The original failed 0.207 report remains separate historical
evidence; terminal session 56093 must not be polled again. The previous adopted
0.202 report/archive are preserved. That batch's retained archive is
`.verification-evidence.pEYfnW`; report SHA-256
`90c304982e30330d9ad8c4ac73660e196183b52b9ff7a83661e5c94e7504548f`.
L14/L15's conditional acceptance is now satisfied. L02/L09/L13/L17/L18 remain open;
0.210's complete qualification and source/worker/root endpoint audits pass; adoption
and `.verification-evidence.5uR1DG` archive integrity are complete. Session 26639
is terminal exit 0 and must not be polled
again. Successors 0.211–0.213 are adopted above; 0.214 and 0.215 remain active. The history below
records prior candidates and does not override this current disposition.

The adopted baseline is unchanged by this control ledger. The following existing
batches are retained, not newly invented tasks:

| Batch | Existing work | Current disposition |
| --- | --- | --- |
| Specification 0.198/model 0.262 | F01 query self-description and discovery integration | Qualified and adopted; preserved by the current baseline. |
| Specification 0.199/model 0.263 | Empty F08 range correction and conditional interval proof | Qualified and adopted; preserved by the current baseline. |
| Specification 0.200/model 0.264 | Existing F08 reply-value shapes | Consolidated into qualified/adopted 0.201; no separate full run is needed. |
| Specification 0.201/model 0.265 | Shared compound normalization and F08 value checks | Qualified/adopted and preserved by 0.202; archived 758-check/946-input evidence. L03 accepted. |
| Specification 0.202/model 0.266 | Actual-parent resource registry funding, decoded workspace, existing F05 cancellation clarification and trusted host-command binding contract | Current qualified/adopted baseline: 760 checks/946 unchanged inputs, matching source/worker/root and integrity-checked evidence archive. L05/L06 accepted. The earlier single stale aggregate-validator failure remains failed historical evidence; its corrected successor is the accepted run. Resource-profile discovery remains a separate L02 exception. |
| Specification 0.203/model 0.267 | Existing F01 result validator connected before native selector/issuance publication | Failed full qualification, terminal exit 1: 724 passed/37 failed out of 761 checks, unchanged frozen inputs. All failures identify obsolete discovery mutation construction anchors. Retained report SHA-256: `9564d7897463c6650c8ca8587dac1a133fbb66824626d986dd669297dc402764`. No adoption or semantic detection is claimed for failed construction. |
| Specification 0.204/model 0.268 | Repair the existing discovery mutation anchors; connect the F08 actual query result guard and temporary workspace | Full qualification completed with exit 1: 761 passed/one failed out of 762 checks over 946 unchanged inputs. `copied-target-reference` survived native discovery mutation bank 2. Source and worker match the report; retained report SHA-256 `c799f0a332bb374930dca4580ed0cc91c2cc6bd3f8b093cfd32b2c8ff7781f43`. Not adoptable. In the mutated producer, result validation masks the return-only oracle while incorrectly faulting the owner for known invalid caller input. Never poll its terminal session again. |
| Specification 0.205/model 0.269 | Modular substrates/local filesystem first, copied-target oracle and closed engine measurements | Failed full qualification, terminal exit 1: 762/763 checks pass; publication-clause global source fingerprint is stale. Worker matches its report; source retains the freeze fingerprint, but generated architecture.ttl was not copied from the failed worker, so report inputsUnchanged/current-match are false. Report SHA-256 36127508b7cba65710aad1650fb48406dd457d595c53574d7c5abf479625604a. No adoption; never poll terminal session 3893 again. |
| Specification 0.206/model 0.270 | Primary-only composition and repeated-source grouping | Frozen retained candidate, consolidated into 0.207; no standalone full run. Its inherited stale publication-clause fingerprint is corrected in 0.207 after exact selected-text comparison. Composition sixteen controls/six mutants and owner twenty-seven controls/thirty-seven mutants pass focused registrations/review; no full qualification/adoption claimed. |

The frozen 0.206 snapshot is retained (953 inputs; fingerprint
`1616756a468bdb19f75a7f3c02c5e9f8db61d904beb7f849419a8c7af10c5c06`)
and consolidated into 0.207, not separately qualified with its known-stale clause
ledger. The original 0.207/model 0.271 consolidated full run terminated with exit 1:
763 passed and two failed out of 765 checks over 957 unchanged frozen inputs, fingerprint
`b59334f192a861729ede074c274179872c02531639e8e1bcf8fccd8a80487bef`.
It retains all prior checks and adds source-only exact-batch binding. Its current
verifier generated the architecture artifact before the fresh capture. Clause
fingerprint reconciliation preserves all ten selected units/41 fragments and
sixteen sensitivity controls. The successor auditor compiles but has not audited
a report. The failures are the exact-batch and composition aggregate scope-marker
checks; both child checks exited successfully. Source and worker match their report
inputs. The retained source report SHA-256 is
`c2811e28d6883a65d48c3846be13f8cbd90f0a7bf95977c2b289af31618879b3`.
Session 85665 is terminal and must not be polled again. No adoption is claimed.

The repaired 0.202 successor is qualified and adopted. Preserve its frozen source
and worker; do not replay its consumed publisher/archive copy. The newer 0.203
through 0.205 failed candidates, retained 0.206 and unqualified 0.207 candidates are not adopted.
L01/L03/L04/L05/L06/L07/L08/L10/L12/L16 are accepted at their stated levels.
Continue L02's remaining operation products and U01 integration boundary. Use
the matrix to select the largest unresolved end-to-end contract rather than
another disconnected checker. L17 reconciles stale registers alongside that work.

The [mixed authoring design](atomic-authoring-design.md) now explicitly inherits
§13.9's independent body/target versions and duplicate-field refusal, existing
create/delete identity exclusivity, and refusal-versus-invariant-fault handling.
The [native target probe](mixed-target-binding-counterexample.mjs) passes with
asserted unchanged body/body version, one publication and one linked two-participant
history group. This closes the working design review findings, not the public
mixed binding or L02. The original 0.207 run failed its aggregate checks
for exact-batch and composition: the checker outputs lack their required
scope markers. An editable successor adds those markers without changing the
semantic controls, mutants or aggregate requirements. Those focused checks alone
do not establish full qualification or adoption.

Both corrected registrations now pass the unchanged aggregate validators. The
repaired 0.207/model 0.271 source is frozen separately with 957 inputs and fingerprint
`3f0b4828e883173a3da9292d025d2fa6cd792519c009802cb300a4ad2d886e60`.
It changes only the two checker output scope markers relative to the failed
original; no requirement, model, production predicate or aggregate requirement
changes. Its full frozen run remains active on session 56093; the original session
85665 terminated with the two scope-marker failures. This is qualification of a separate
repaired snapshot, not a timeout restart. All 946 qualified root inputs and the
original frozen fingerprint are unchanged; 160 local
documentation links resolve. Working design review fixes and marker repair do not
close a whole deliverable: ten engine deliverables remain accepted and seven open.

### Standalone annotation and history invocation candidate

Specification 0.210/model 0.274 adds the shared invocation binding for existing
standalone F13/F14 products, retaining the unqualified 0.208/0.209 compound receipt
and mixed-authoring work. It captures original native preparation and rechecks
role-specific authority, allocation, dependencies and limits; retries cannot follow
a newer history frontier. No UI behavior or resource-module mechanism enters this
engine binding, and the U01 public receipt-retrieval decision remains open.

The second-agent review found that incomplete history limit profiles could admit:
present-value validation missed absent coordinates. The repaired capture requires
all declared finite coordinates. Construction funding now accommodates a native
ceiling above the parent intent ceiling while enforcing the actual retained intent
bound, rather than refusing otherwise valid small edits solely for that difference.
The extra reservation is checked and debited against actual parent headroom before
native preparation. These are logical serialized frames, not physical allocation
measurements or provider-truth proofs.

Focused evidence passes twelve controls, eight variant/target cases, seventy-nine
omissions, eight permission cases, fourteen semantic mutants, ten adjacent suites,
the exact aggregate registration, the 124-module engine/UI boundary and 124 clean
entrypoints. Read-only review found no remaining must-fix in these changes.
The retained 970-input snapshot has fingerprint
`90486c3d8799109b6dc1fb42dc93da47391eaf55b40a073ca6783efaf51a215b`;
its complete suite terminated successfully with 768/768 checks and unchanged
970-input source/worker endpoints. This consolidates unrun 0.209 without dropping
its checks. The corrected endpoint audit passes all expected registrations,
outputs, 299 conditions/fifteen manifests/81 baseline conditions and pinned-tool
evidence. The first prepared auditor used one stale mutant-name literal; its
preserved failure was corrected in an outer auditor against the exact frozen
source/result name, without changing proof inputs or predicates. The source report
SHA-256 is `9652501b5e501ec19bd618f62305bc09bd4321630807341851fea1feff667412`;
the worker report SHA-256 is
`2d868ced8adae374118db7c6e572eeefa5698ea97d216f18899ae03e2b191962`.
Session 26639 is terminal exit 0. All nineteen changed files were hash-guarded
into the root, with no deletions. Exact report publication preserved the previous
report. Root endpoint audit and `.verification-evidence.5uR1DG` archive integrity
pass. This retained 0.210/model 0.274 baseline had 970 matching root inputs;
current 0.211 supersedes it without dropping its evidence. No whole deliverable
or goal closes solely from that adoption. The archive
is an integrity-checked evidence copy, not another verifier execution or independent
certification. Consumed publisher/archive scripts must not be replayed.

The [combined native authoring reference](atomic-authoring-reference.mjs) passes
seven controls: one shared-owner publication containing source plus all four
annotation operations, five atomic conflict refusals and one Unknown/faulted host
protocol case. Four receipt-policy checks consume the same published root with
individual pre-execution ceilings and current release guards. They are not a
coordinator-owned, aggregate-funded receipt vector. The adopted 0.210 native
binding supplies that vector and mixed ingress separately; the working reference
alone does not prove them. Public U01/event assembly remains L02 work; no whole
deliverable or actual allocation-issuer claim is promoted.

Read-only second-agent review of this combined reference found no material
correctness or masking issue. Its terminology now distinguishes pure receipt/frame
bound preflight and disclosure-ceiling capture from actual memory reservation.
This review is not independent Cleanroom certification.

Specification 0.208/model 0.272 now contains an internal native component-receipt
binding (§13.9.6), actual-parent logical funding, precommit vector installation,
historical/current release, retention/drain-gated vector eviction and owner-
controlled once-only hold refund. Eleven controls/nine assertion-detected semantic
mutants, seven adjacent regression suites, the exact registered validator and
121-module boundary/clean-entrypoint checks pass. Read-only review found no
publication/binding/funding defect; the cleanup distinction follows existing
§13.8, with strict Boolean retention/drain checks on the new atomic path.

The 0.208 source is separately frozen: 961 inputs, expected 766 checks, fingerprint
`9344edb93beede8e36ffbb9ffdd8932c8ef95f602d86f24b3f9a6f151368d9ad`.
Its endpoint auditor compiles but has not audited a full report. Full qualification
has not started; both predecessor sessions remain live. Adopted 946 root inputs
and both running snapshots were rechecked unchanged before this capture. Public
mixed ingress/per-participant authoring authority and U01 remain open; no whole
deliverable is accepted by this focused internal receipt evidence.

Specification 0.209/model 0.273 supplies the public mixed logical binding in
§6.5.2.5 and incorporates the 0.208 receipt path. Sixteen controls/thirteen semantic
mutants, eight field omissions, 26 current authority cases, twelve adjacent
regression suites, the exact registered validator and 123-module boundary/clean-
entrypoint checks pass. A read-only bounded reviewer found no must-fix. The binding
preserves original public/native intent, before/candidate source meaning, separate
F11/F13 authority, current per-component owner/limits/dependencies, original create
allocation, private stamps, one whole publication and historical receipt ceilings.

The consolidated 0.209 source is frozen: 966 inputs, expected 767 checks, fingerprint
`0ba75746b0c04697b7fcae8e47b8c488952cc44017bfbd325e78a220b788beb4`.
The endpoint auditor compiles but has not audited a report. No full run has started;
0.208 is retained and consolidated rather than scheduled for a redundant standalone
run. Root 946 adopted inputs, both running snapshots and frozen 0.208 were verified
unchanged before capture. Public discovery/enablement, standalone-family completeness,
cross-family event/result assembly and U01 remain L02 work. Full qualification/
adoption is required before accepting this candidate; accepted/open counts do not
change on focused evidence alone.

A final consistency sweep replaced the inherited receipt section's stale “public
mixed ingress remains open” wording with its actual §6.5.2.5 dependency. The earlier
pre-sweep capture is retained but superseded before any full run; no frozen source
was edited. This reconciles the design record without changing production predicates
or promoting scoped evidence to whole-facade completion.

After each closure, report the item, acceptance evidence, remaining items and
any decision needed. Add work only for a cited unrepresented v1 obligation,
demonstrated defect or approved scope change. A new test or proof helper is not
itself a new requirement or reason to reopen an adequate contract.
