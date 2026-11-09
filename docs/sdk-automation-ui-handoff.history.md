# SDK automation and UI implementation handoff

Build a thin native Rust consumer layer over supported engine behavior, prove it
with headless consumer tests, then place a minimal UI above it. The northbound
boundary is sufficient for parallel development but is not a frozen published
SDK or complete recipient facade. This handoff adds implementation coordination,
not a new automation language, plugin runtime or engine requirement.

## Starting points

- [Engine public modules](../crates/vellum-engine/src/lib.rs) and
  [coordinator](../crates/vellum-engine/src/coordinator.rs).
- [API boundary and compatibility policy](engine-api-boundaries.md).
- [Public contract design matrix](public-contract-matrix.md).
- [UI ownership and examples backlog](reference-ui-design.md).
- [Current engine closure worklist](engine-test-plan.md#current-engine-closure-reconciliation)
  and [implementation overview](../README.md).
- [Native invocation observation](../crates/vellum-engine/src/coordinator/invocation/observation.rs)
  and [consumer behavior tests](../crates/vellum-engine/tests/invocation_behaviors/observation.rs).
- [Expanded V2 receipt profile](../crates/vellum-engine/src/coordinator/invocation/observation/receipts_v2.rs)
  and [disposal consumer tests](../crates/vellum-engine/tests/retained_disposal_invocation_behaviors.rs).

The contract matrix includes design/reference evidence; it is not a list of
completed Rust handlers. Confirm each advertised capability in the actual owning
implementation and an executable consumer scenario.

For owned source/annotation read and save status/results, use the existing
native-operation observer/decoder, now reachable through
`prepare_native_operation_discovery_cut_v1` and
`query_discovered_native_operation_v1`. The explicit source describes three
`native-operation-observation/1` operations and the separate
`native-operation-receipt/1` retained-outcome operation. Keep the original task,
observer, class-specific facet, scope and current native release facts. Discovery
does not enable decoding, issue a producer, submit I/O or grant receipt access.
Enable and fund the existing context-owned decoder explicitly; protect wait,
context and pool retirement through their existing APIs. Public examples are in
`tests/source_read_behaviors/observation.rs` and
`tests/save_operation_behaviors/observation.rs`. Native forms are not wire IDs.

F01 self-description uses `prepare_native_discovery_query_cut_v1` with explicit
three-variant visibility, then the existing discovery acquisition/list/inspect
APIs. `query_discovered_discovery_v1` checks an original installed operation
selector and current metadata release before calling the existing context-owned
decoder. Supply separate fresh cuts and authenticated facts for discovery-route
release and the target query. Enable and fund the target decoder explicitly with
`enable_discovery_decoder_v1`; descriptions cannot enable it. Both typed and
dynamic routes support list (including original continuations), inspect and
Boolean advisory enablement. Keep original context-local roles and retire each
context/cut at its existing owner. See `tests/discovery_behaviors/self_description.rs`.
Result declarations describe native forms, not serialized opaque identities;
there is no implied recursive schema evaluator or reference /13 wire profile.

Native settings inspection is now available through `configure_settings_reader_v1`,
`prepare_settings_read_v1`, `acquire_settings_read_v1`, `query_settings_read_v1`
and the corresponding protected stop/retirement methods. See
`crates/vellum-engine/tests/settings_behaviors/settings_recipient.rs` for an
executable public consumer. The host supplies authenticated current facts and
complete attested visibility separately from requester arguments. Retain the
original opaque context/target, obtain a fresh actual-root cut for each query,
and discharge ended contexts explicitly. Origin references are context-local,
not layer identities or write authority. Do not expose privileged `settings_read`,
raw projection data or host-owner pool controls as ordinary recipient operations.
Typed/dynamic F07 `configuration-query/1` is now available through
`query_decoded_settings_read_v1`, with `CaptureLimitsV1` describing native logical
input bounds. Budget the fixed `settings_read_decode_classification_bytes_v1()`
plus seven maximum-request frames in actual parent headroom, in addition to the
reader's retained reservation. `SettingsReadReplyV1` carries the original opaque
context/target and requested key with each filtered product. Use
`prepare_native_settings_read_discovery_cut_v1` and
`query_discovered_settings_read_v1` for installed discovery dispatch; copied
descriptors confer no handler or read permission. Discovery release and current
reader release remain independent. Do not reproduce these policy/lifetime owners
in a consumer wrapper, infer a JSON wire codec, or advertise other F07 profiles.

The distinct native resource-catalog recipient path is available through
`configure_resource_catalog_reader_v1`, `prepare_resource_catalog_visibility_v1`,
`acquire_resource_catalog_read_v1` and `query_resource_catalog_read_v1`, with
explicit protected preparation/context/registry retirement. See
`tests/resource_catalog_behaviors/recipient.rs`. Supply complete authenticated
visibility and approved order at the host edge; entry and continuation references
are context-local metadata selectors, not F16 mutation or provider capabilities.
Current cuts must match the actual engine root; stable page retries reuse aliases,
and continuations preserve their original page limit. Catalog inspection is not a
provider probe or resource open. Its typed/dynamic entry is
`query_decoded_resource_catalog_read_v1`; installed dispatch uses
`prepare_native_catalog_read_discovery_cut_v1` and
`query_discovered_resource_catalog_read_v1`. Set the actual
`InvocationLimits.max_intent_bytes` for decoded ordinary requests; native-only
construction leaves that ceiling disabled. Request capture bytes intersect that
ceiling, and actual parent headroom must cover
`resource_catalog_read_classification_bytes_v1()` plus seven admitted request
frames in addition to retained owners. Command issuance may remain disabled.
Use original catalog context, entry and continuation references, never settings
contexts or F19 references. Decoder faults preserve spent aliases and pending
preparations for protected discharge; a temporary refund does not retire the
catalog owner. Capture units are native logical bounds, not a selected wire codec.

F02 source/snapshot discovery now uses
`prepare_native_source_read_discovery_cut_v1`, `query_discovered_source_read_v1`
and `acquire_discovered_read_v1`. These delegate to the existing decoded facades:
range/line reads and all six current/retained single/coherent acquisition ports.
Keep original opaque roles and separately authenticated host facts; metadata
visibility does not grant content access. See
`tests/settings_activation_behaviors/source_context/facade/installed.rs` for
capture → edit → retained read → protected cleanup through public Rust APIs.
Use the existing native retirement APIs rather than adding wrapper-owned leases.

Installed annotation reads use `prepare_native_annotation_read_discovery_cut_v1`
and `query_discovered_annotations_v1`; installed history inspection uses
`prepare_native_history_read_discovery_cut_v1` and `query_discovered_history_v1`.
Each describes its existing three operations and accepts the existing decoded
request with original opaque references and separately attributed native facts.
These adapters use the shared `DiscoveredSourceReadErrorV1` read error sum.
Discovery permission never replaces native content/detail permission, and
discovery retirement does not discharge the reader. Public consumer scenarios
are in `settings_activation_behaviors/source_context/projection/installed/queries.rs`
and `history_behaviors.rs`. No wire codec or new SDK-owned identity map is implied.

For trusted-native F12 dispatch, use `prepare_native_selection_discovery_cut_v1`
and `submit_discovered_native_selection_v1` with the original owning-normalized
selection arguments. The four native forms use their existing invocation runner,
source/view currentness, match/boundary evidence and replay. The host must still
authorize its caller; discovery is metadata, not mutation authority. Scoped F06
selection-result/observation assembly remains open. Consumer checks are in the
selection, search-selection and Unicode invocation argument test modules.

Do not connect the passive logical `literal-search/1` guard to native search by
stringifying document slots or dropping profile/generation fields. Its complete
binding is pending the explicit engine-binding versus integration-edge naming
choice. Working native opaque-context search is unaffected.

Native compound advice is available through `capture_compound_advice_basis_v1`
and `query_compound_advice_v1`. Supply the original typed request, context/policy
and separately authenticated complete predicate-observability facts; basis capture
is not permission. Configure actual parent headroom for the fixed classification
plus eight admitted request frames, and a nonzero native intent ceiling. Advice
returns no prepared transaction, identity, reservation or execution grant. Future
commands independently revalidate. See `annotation_history_behaviors/advice.rs`.
The closed borrowed entry is `query_decoded_compound_advice_v1`; installed dispatch
uses `prepare_native_compound_advice_discovery_cut_v1` and
`query_discovered_compound_advice_v1`. `CompoundAdviceEnvelopeValueV1::Plan` must
carry the same original plan as `Proposal`'s request; the latter retains all
original optional read/scope and context/policy/limit meaning. Both routes still
require the original native host basis and current observability rights. Copied
metadata is not an installed handler or permission. The advertised outer schema
does not parse nested logical-plan JSON; no wire codec is selected.

## Edit scope and coordination

Own new consumer packages/examples, their tests and this handoff. Choose package
layout before implementation and coordinate shared workspace manifest/dependency
edits. The engine thread owns engine implementation and shared semantic contracts;
the filesystem thread owns filesystem mechanisms. Propose shared API changes with
an exact missing behavior and consumer acceptance case before implementing them.
Do not create a second publication path or fill missing semantics with wrapper
state that contradicts engine authority.

## Layer responsibilities

For authored initial text, use the [native logical scratch constructor](../crates/vellum-engine/src/coordinator/birth.rs),
not `create`/file import or a synthetic insert after creation. Scratch preserves
literal leading U+FEFF, mixed endings and explicit output/newline intent without
adding history or selecting/creating a view/workspace. Its specified native birth
generations are one; existing privileged import retains its zero baseline and
one-signature interpretation. File reads still use validated import/activation
paths. The constructor is a privileged kernel API. For owned native execution use
[`submit_scratch_v1` / `run_scratch_v1`](../crates/vellum-engine/src/coordinator/invocation/scratch.rs),
with trusted creation/lifecycle premises and explicit parent-funded limits.
Admission creates no document or preallocated identity. Result release returns
only the original publisher's invocation/document identity under independent
captured/current disclosure rights, not current document state. Forgetting cannot
replay creation; protected retirement releases only that capture hold after
retention release and delivery drain. These premises belong to trusted embedding,
not arbitrary-script credentials. Explicit native F06 V6 now composes scratch
results through `open_invocation_observation_with_receipts_v6` and
`query_invocation_receipt_v6`: the captured lifecycle capability and complete
parent-funded reply wrapper are mandatory, as are common and native current
release rights. Older exhaustive profiles cannot be upgraded; nested grantless
lifecycle requests remain unavailable. Receipt-only faults preserve protected
status and independent cleanup. Complete public F17 decoder/discovery remains
open; do not advertise it yet.

The [trusted-native lifecycle behavior facade](../crates/vellum-engine/src/coordinator/invocation/document_facade.rs)
provides one closed Rust submit/service entry over scratch, live attach/detach/
membership and prepared disposal. Match each command with its operation-specific
admission and fresh service premises; original owners capture intent and enforce
currentness. Results remain separately reauthorized through native facets/F06 V6;
stop, forgetting and cleanup retain existing owner contracts. This does not add an
executor or transport. In particular, the reference string-ticket envelope and its
explicit disposal version/retain/discard fields are not decoded here. The native
prepared-disposal role is not a substitute for those data/authority requirements.

For explicit native disposal expectations use the [explicit disposal profile](../crates/vellum-engine/src/coordinator/invocation/disposal/explicit.rs):
prepare the complete original version/choice request, then submit and service it
with the existing native admission facts plus current applicable coverage. This
uses the same original owner and native result/cleanup ports, not the legacy
facade's `PreparedDisposal` variant. The exhaustive V2 facade exposes it as
`NativeDocumentLifecycleCommandV2::ExplicitDisposal` with matched explicit
admission/service coverage; `Existing` includes unchanged V1 commands. V1 cannot
acquire this operation or bypass an explicit original intent. Both versions share
one outer admission/service observation, not duplicated wrapper timings.
Retain/discard rights cannot be dropped or
changed by replay or legacy service. `binding_generation: None` means unbound;
`Some(0)` preserves a real legacy binding. Neither choice promises physical erasure:
both enter disposed-retained with required evidence preserved. Do not invent the
wire/string identity or legacy counter mapping from this native profile.

For request-driven SDK/automation, use `NativeDocumentLifecycleRequestV1` and
`submit_native_document_lifecycle_request_v1`: the five supported native operations
accept semantic requests rather than a caller-retained disposal preparation.
Service still uses V2's existing operation-specific fresh facts. For disposal the
engine authorizes and reserves the actual parent capture before preparing the
first intent. Matching replay uses only the original captured plan, including after
disposal or result forgetting; it never refreshes against a later root. A failed
first preparation refunds only provisional capacity and leaves the ticket issued.
Independent result/stop/discharge rights remain mandatory. The prepared V1/V2
ports remain available for trusted embedding that already owns a preparation;
these are equivalent inputs to the same owner, not another engine or execution
mode. No string/value/counter codec or transport is added.

The native lifecycle argument guard now accepts exactly the wrapper and five
handler-owned request forms via `NativeDocumentLifecycleEnvelopeValueV1`, then
`submit_native_document_lifecycle_arguments_v1` delegates to the same data-first
owner. Identities, version/lifecycle/view/workspace expectations and binding
presence use native roles; strings are never resolved into handles or rounded into
counters. Source bytes/selections are copied under explicit logical output limits.
The standalone normalized object is caller-owned/funded, not an engine shared
reservation or permission. Native admission still enforces actual funding,
authority, currentness and semantics. These guards do not themselves install the
accepted discovery/registered dynamic route. V1 dynamic invocation is in-process
(requirements §17.4); remote/wire codecs are not v1 engine closure deliverables.

For the installed native lifecycle route, use
`prepare_native_builtin_discovery_cut_v4` with the existing bounded discovery
owner, then obtain opaque entry/variant selectors through list/inspect.
`submit_discovered_native_document_lifecycle_v1` checks those original selectors,
private installed origin and matching operation, revalidates a fresh discovery
cut, and delegates the normalized arguments to the existing native admission.
Scratch, attach/detach, membership and explicit disposal are the selected native
forms; this is not the whole F17 catalog. Generic matching metadata cannot invoke
this route. Discovery visibility/advice is not command permission: supply the
independent original native admission and fresh native service facts, then use
the existing separately authorized retained-outcome facet. Prior `/1`–`/3`
inventories and exhaustive command/receipt profiles remain unchanged. The new
descriptions name native roles; do not treat them as transport codecs.

For the two F17 container variants, use
`prepare_native_container_discovery_cut_v1` with the existing scoped `/1` owner,
then list/inspect its one entry and two variants. This independent fixed source
does not change the existing built-in inventories. Its four create/retire forms
come from `native_relation_argument_declarations_v1` and the existing normalized
relation input. `submit_discovered_native_container_v1` delegates to that owner,
with optional original relation-result admission; service remains `run` and result
delivery remains F06's `Relation` facet. Matching retry retains the same command,
allocation and capture ceiling. An original no-result invocation cannot acquire
results later. These are trusted-host embedding ports, not permission for arbitrary
scripts; metadata and result grants do not authorize execution. Drain the F06
observer and its original policy before retiring the invocation. Logical views/
workspaces are engine containers, not UI windows or filesystem directories.

For disposed source-binding, annotation-binding and provenance release, use
`prepare_native_retained_release_discovery_cut_v1` on the same scoped `/1` owner.
Its one `dispose` variant describes the three existing typed request forms through
`RetainedReleaseOperationV1::argument_declaration_v1`; it is not a new parser or a
widening of the other lifecycle envelopes. `submit_discovered_retained_release_v1`
takes the original invocation, `RetainedReleaseRequestV1` and independent native
admission unchanged. Service still requires `run_retained_release_v1` with fresh
facts; generic `run` cannot bypass them. Use F06 V3's `RetainedRelease` facet and
its current native release facts for historical results. Replay cannot substitute
another substep. Forgetting removes results but not funding: drain the observer,
retire the native release capture and original policy, then retire the invocation.
These operations release retained relationships/provenance, not authored payload
storage, external effects or filesystem resources.

Source rebinding now has native owned invocation/result, installed discovery and
F06 delivery. Its direct `rebind_source_document` API reuses an internal service-time
preparer and guarded commit. Do not cache a prepared replacement root or treat
the native root stamp as a complete writer guard: queued work can change without
that stamp changing. The private commit repeats current writer/barrier checks and
requires fresh authority. No new public preparation API is available to SDKs.

For owned native rebinding use `SourceRebindRequestV1` with the original document,
opaque `SourceBindingTargetV1` and exact native options/pair evidence.
`submit_source_rebind_v1` retains that intent under `SourceRebindAdmissionV1`;
`SourceRebindLimitsV1::required_capture_bytes(context, request)` includes retained
role payload and service-copy staging, not caller ingress or allocator/RSS.
`TrustedSourceRebindInvocationFactsV1` binds a complete context to the existing
native rebind facts. Service is `run_source_rebind_v1` with fresh facts, never a
generic `run` bypass. Current service refusal settles that invocation; freeing a
writer requires a fresh invocation, not retrying a terminal refusal. Native
`source_rebind_result_v1` independently reauthorizes the original historical
document/applied-or-unchanged result under `TrustedSourceRebindResultV1`; it does
not disclose binding/provider identities or savedness. Forgetting does not release
funding; protected `retire_source_rebind_v1` discharges the hold before invocation
retirement. Capture retirement after an unknown execution does not settle it.
These native ports retain the existing trusted-host premises; they are not a
recipient-authentication layer or transport decoder.

For installed rebinding, obtain original entry/variant selectors from
`prepare_native_rebind_discovery_cut_v1` (existing scoped discovery `/1`), then
use `submit_discovered_source_rebind_v1` with the original invocation/request and
native admission. Visibility does not grant execution. Service remains the fresh-
fact native runner. Bind observation policy before original admission, open an
explicit V7 observer with captured lifecycle capability and the full
`InvocationReceiptV7::required_reply_bytes(original_maximum)` envelope, and query
`InvocationReceiptRequestV7::SourceRebind` with independent current native release
facts. V1–V6 observers cannot acquire this facet through a new selector; V7's
`Existing` prefix preserves V6 delivery. Neither discovery nor result queries
rebind, allocate a new document, refresh original intent or prove savedness.
Acknowledged Save As activation remains a separate native operation. Its public
native path now shares private read-only preparation/fenced commit stages and
optional timing. `borrow_source_save_as_authority_v1` lends an opaque
`SourceSaveAsAuthorityV1` for the actual SaveAs task under strict
`SourceSaveAsAuthorityFactsV1`; `source_save_as_authority_current_v1` rechecks
original record/registration identity without activation or provider work. Weak
markers do not override reservation, quiescence or baseline-anchor retirement
obligations. This is not an execution grant, a copy-export authority or a write
dispatch.

For owned activation, use `submit_source_save_as_v1` with that original authority,
`SourceSaveAsRequestV1` and explicit capture/result limits, then
`run_source_save_as_v1` with fresh native and invocation facts. The request retains
the original probe, pair and history bound; it does not submit another write.
`source_save_as_result_v1` separately authorizes release of the original
document/changed result. Later edits remain modified against the captured baseline;
replay does not activate again. `retire_source_save_as_v1` discharges only the
invocation capsule, not the producer's reservation/quiescence/baseline obligations.
For the installed route, obtain selectors with
`prepare_native_save_as_discovery_cut_v1` on the existing scoped discovery /1
owner, and call `submit_discovered_source_save_as_v1` with the original task
authority, request and native admission. Visibility is not execution permission.
Bind observation policy before admission, open an explicit V10 receipt observer
with the original lifecycle capability and full
`InvocationReceiptV10::required_reply_bytes(original_maximum)` envelope, then use
`InvocationReceiptRequestV10::SourceSaveAs` with current native release facts.
V10's Existing prefix delegates V9 results; older observers cannot acquire the
new facet. No query dispatches a write or claims that later edits are saved.

Ordinary saves already have shared-writer parent ownership through
`request_owned_source_save_v1` and `request_owned_annotation_save_v1` (and their
catalog-bound routes). Preserve the original operation/effect/attempt and complete
request/context: matching lookup returns the same child without new capture,
encoding or dispatch. Use the existing native operation observation products for
status/evidence/control; do not fabricate an authored-publication receipt or add
a second invocation/task store. The actual child admission now installs its
artifact/charge, linkage, optional continuity credit and prepared parent under a
single fault fence. This does not make an external write atomic or install its
baseline. Obtain the two installed effect-request selectors using
`prepare_native_ordinary_save_discovery_cut_v1` on the scoped discovery /1 owner,
then call `request_discovered_ordinary_save_v1` with the matching
`NativeDiscoveredOrdinarySaveCommandV1` variant. Preserve the original optional
catalog binding, complete request and current release packet. First admission
still requires its native permissions and candidate/pair evidence; matching
recovery does not need another annotation candidate or permission to recapture.
Visibility is not native execution or result-disclosure permission. Observe the
returned original task with `open_native_save_observation_v1` and the existing
native-operation status/retained-outcome/control products; no new publication
receipt profile is introduced. Observer stop is not producer stop. Native
admission/recovery timing is synchronous engine work, not provider latency or
scheduler wait time.

For the retained lifecycle, the trusted owner can now call
`reclaim_disposed_document_v1` with exact provenance-released state and
`TrustedDocumentReclamationV1` cleanup/disposition premises. The engine recomputes
current native discharge itself and refuses remaining managed targets; do not
use copied diagnostic counts as authorization. Success drops native source and
annotation payload, preserves lifetime identities/floors, and makes that document
incarnation unavailable. Faulted publication stays Unknown and fenced. This is
not a public recipient command, substrate delete, secure erasure, result-holder
revocation or guarantee about unmanaged native copies. Existing constrained
legacy direct disposal retains its own contract.

The trusted native source-stream foundation is now available through
`configure_source_publication_streams_v1`, `open_source_publication_stream_v1`,
`sample_source_publication_stream_v1`, `release_source_publication_event_v1` and
`resync_source_publication_stream_v1`. It uses an exact original
`SourceReadScopeV1` plus separately attributed `TrustedSourcePublicationFactsV1`,
bounded `SourcePublicationLimitsV1`, opaque observer-local generations and
immutable source projection values. Sampling is explicitly host-driven after
publication, not a callback, scheduler or guarantee of every intermediate edit.
Current loss/disposal closes the old stream; gap requires fresh baseline
authority; reply pressure does not substitute a newer payload for a retained head.
Use the separate protected stop/retirement methods and final pool retirement;
stop does not refund funding or lifetime issuance. Native usage is trusted
diagnostic metadata, not permission to expose counts as recipient progress.

These are raw technical source-only ports. Do not bind a recipient's matching
F19 establish/resync retry by calling them again: original-command ownership,
retained-outcome/correlation and heterogeneous routing are still required. No
full publication-observation `/2`, registry stream, explicit revocation choice or
new public catalog capability is inferred. The old exhaustive stop facade remains
unchanged; use this owner's exact native control until its versioned heterogeneous
composition is implemented.

`document_context_pins_v1` supplies `DocumentContextPinsV1` for attached logical
views, workspace memberships, pending/accepted boundary evidence, live issued
annotation snapshots, retained serialization bases, original ledger-slot
document references (`retained_invocations`) and retained native publication
stream rows (`publication_streams`), including stopped rows. Clones share one known
snapshot allocation; the final holder drop drains that incidence. Stale/closed
is not retired. Detached views/empty workspaces are not all-document pins.
This is a label-free optional timed native diagnostic, not a recipient grant,
reclamation receipt. The removal publisher recomputes native owners directly.

`document_read_pins_v1` supplies the separate trusted-owner
`DocumentReadPinsV1` contribution for retained read contexts/preparations/holds,
managed and legacy search products, history groups/pins and inspection bases.
Coherent/member holds and linked history pins retain their full original group;
hidden basis participants are not inferred from the permitted projection. Closed
or invalidated retained records still count, and a closed legacy session can
leave a known live result holder. Counts are per-document incidences, not unique
global allocations or memory charges. Counter/orphan drift is an error, not zero.
The query neither releases holds nor grants reclamation; resources and remaining
engine owners must be accounted independently. Do not export document IDs/paths
as metric labels or treat copied counts as an authoritative discharge receipt.

For trusted-owner resource retention diagnostics, `document_resource_pins_v1`
returns `DocumentResourcePinsV1` for the exact retained native document. These are
resource row incidences, not unique allocations, bytes, physical erasure evidence
or a recipient query grant. Stopped/quiescent work counts until actually retired;
parents, artifacts and reservations can drain independently. Fault/invalid/drift
is an error rather than zero. A zero resource total does not account for retained
document metadata, history, views, snapshots, searches or invocation/results, and
must never be used as authority to free an owner. Do not label exported metrics
with document identities/paths. This query itself releases no hold or producer.

For first-party protected observation stop, use `stop_observation_facade_v1` with
an original `ObservationStopPortV1`: the existing read-stop port, a mutable
invocation observer and control packet, a native-operation observer and control
packet, or a discovery context with its exact scope/current protected facts.
The native owner checks authority; selecting a variant grants nothing. Stop
closes only that observation, remains available under applicable editing faults,
and neither cancels a producer nor releases observer/producer funding. Retire
through the owning native product after its real obligations discharge. Existing
read-stop APIs remain unchanged. This closed SDK composition is not a wire/opaque
attachment resolver or a claim that grant revocation, resync and every stream
owner are implemented. Diagnostics are bounded and optionally timed.

For source-publication streams use the additive `ObservationStopPortV2` with
`stop_observation_facade_v2`; `Existing` delegates the original V1 owners without
widening their exhaustive enum. `stop_decoded_observation_facade_v2` additionally
checks an exact passive typed/dynamic envelope against that first-party lowering.
The opaque `ObservationStopTargetV2` must identify the very same native context;
for a host-owned invocation observer obtain `context_id_v1()` before borrowing it
mutably for control. That identity grants nothing and is not an ordinary progress
or terminal control selector. The fixed logical grammar remains `read-context/1`
and `stop-observation`; Rust V2 is not a new wire format. Malformed or mismatched
requests never reach native stop. Native owner checks still reject foreign-owner
identities. Stop does not retire streams, refund issuance, discharge retention,
cancel producers or implement `revoke-observation-grant`.

`ObservationStopPortV3` now adds `FacetPublication`, `CollectionPublication`,
`ResourceRegistry` and `RegistryReceipt`; `Existing` preserves V2/V1 routing.
Use `stop_decoded_observation_facade_v3` with matching `ObservationStopTargetV3`
for the same fixed typed/dynamic stop grammar, or the original native protected
control edge directly. Each actual owner still validates scope/control. Stopping
a registry receipt leaves its original command claim and sibling/live streams
unchanged; stopping a registry stream does not stop receipt contexts. No discovery
query, ordinary capacity or content grant is introduced into protected stop.
This is not grant revocation, producer cancellation, implicit retirement or the
yet-unimplemented opaque attachment/discovery dispatcher.

Use `release_observation_event_v1` for heterogeneous native event delivery across
source, named facets, collection and registry streams. Match the original
`ObservationEventTargetV1` to the trusted `ObservationEventPortV1`; actual native
scope/current-authority checks still apply. Registry payloads require the exact
historical-head certificate. The closed result preserves each native event's
before/after identities and projection, with Empty and Gap distinct; it creates no
global cursor, callback, new semantic verb or command execution. Configure native
reply ceilings for the complete common reply: its extra enum header is checked
inside the owning release before queue/certificate consumption. Larger caller
limits cannot override parent funding. Pressure withholds the same head/certificate;
it is not permission to drop data or silently resync. This is first-party bound
delivery, not untyped attachment resolution or a wire codec.

Native single-document mixed-facet streams are available through
`configure_facet_publication_streams_v1`, `open_facet_publication_stream_v1`,
`sample_facet_publication_stream_v1`, `release_facet_publication_event_v1` and
`resync_facet_publication_stream_v1`. `FacetPublicationScopeV1` explicitly selects
source, exact named annotation facets and attached semantic views. Host-established
reference strings are recipient references, not raw native IDs or new grants.
`TrustedFacetPublicationFactsV1` supplies exact current annotation/view coverage
separately from shared context gates; source readability is required only when
source is selected. Missing facts refuse; explicit required-facet withdrawal
fences the old stream. Actual view rebinding/disposal also fences release.

Use the native `FacetPublicationControlV1` stop/retirement methods for this new
owner; the exhaustive V2 stop facade remains unchanged. Bound all scope, view,
selection, projection, reply, queue and lifetime dimensions before activation.
Sampling runs outside editing and compares only permitted native state. Full
replacement frames are not a lossless intermediate-edit audit. Matching retries
must not reopen/resync these raw helpers: original F19 command/replay assembly is
still pending. Named observations are not collection discovery; multi-document
observer assembly, registry and grant revocation are not supplied by these APIs.
Stopped rows remain native publication-context incidences until actual retirement;
pool retirement releases its reservation once and does not reset issuance.

Collection visibility preparation is a distinct native technical prerequisite:
configure the existing annotation preparation pool, then use
`prepare_collection_visibility_v1` with complete `TrustedAnnotationCaptureFactsV1`
and explicit `CollectionFacetCeilingV1`. The returned
`CollectionVisibilityPreparationIdV1` binds the exact native root/scope/ceiling;
it cannot be substituted for an ordinary annotation projection. Current validation
and one-use `consume_collection_visibility_v1` require independently attributed
`TrustedCollectionVisibilityFactsV1`, including `projection_current`; unchanged
root identity is not authority. Consumption also requires current reply/transfer
readiness. Root-stale or invalidated packets retain their existing preparation
incidence until `retire_collection_visibility_v1` succeeds under protected cleanup.

`CollectionVisibilityCaptureV1::rows()` is trusted native staging data. Its native
IDs are not northbound collection references, record selectors or grants. Do not
deliver this artifact as a recipient collection API: the separately funded native
collection observer below assigns opaque local references and atomically installs
the complete frame and binding ledger. Returned
technical copies are not managed read contexts or revocable disclosed data. No
new wire encoding, registry, job or advertised F19 descriptor is selected here.

Native annotation-collection streams now use
`configure_collection_publication_streams_v1`,
`open_collection_publication_stream_v1`, `sample_collection_publication_stream_v1`,
`release_collection_publication_event_v1` and
`resync_collection_publication_stream_v1`. Open/sample/resync consume a fresh
root/scope/ceiling-bound visibility packet; abandoned or rejected-before-consumption
packets still need their own protected retirement. `CollectionPublicationScopeV1`
selects the document/context and body/target ceilings. Its stream/reference/
generation issuance and persistent binding bytes/count are independently bounded.
Never cast `CollectionAnnotationReferenceV1` to a native ID, F03 selector or grant.
References persist across visible removal and do not state why membership changed.

For release, `TrustedCollectionPublicationFactsV1::projection_allowed` must attest
the exact whole historical head, not just latest-root membership; current scope,
authority, retention and readiness remain mandatory. Unknown permission withholds;
attributed required-authority loss stops the old observer. Transient reply pressure
retains the exact head, while queue/binding/issuance pressure creates a gap with no
partial ledger installation. Resync requires fresh authorized preparation and
baseline permission. Stop/retirement use the native `CollectionPublicationControlV1`
ports; stopped rows remain C3 incidences until actual discharge. Pool retirement
does not reset spent issuance. Existing exhaustive generic stop facades stay
unchanged. Combined/multi-document assembly, registry streams and original F19
commands/replay/results/correlation are not supplied by these technical APIs.

Native resource-registry observation now uses
`configure_resource_registry_observation_v1` and
`open_resource_registry_observation_v1`, scoped to
`scoped-resource-observation/1`. Baseline input must cover both actual tables with
exact native addresses and separate captured-policy/current membership and
descriptor facts in approved visible order. These are trusted host facts, not
requester grants. The engine returns only local references, classification and
permitted capability declarations. Never treat event references as F07 selectors,
native epochs, resource/provider grants or mutation preconditions.

For updates, `prepare_resource_registry_snapshot_v1` issues a root/scope/stream-bound
packet; `sample_resource_registry_observation_v1` consumes it. Historical delivery
needs `prepare_resource_registry_historical_release_v1` for the exact current head,
then `release_resource_registry_event_v1` with `Some(certificate)`. Passing `None`
can poll empty/gap but cannot release a queued projection. Supplied spent or wrong-
head/root/scope/kind certificates refuse even when the queue is now empty. Current
membership does not authorize an old payload. Whole-head membership/descriptor
denial gaps rather than narrowing the same generation. Readiness can withhold a
valid frame/certificate; preparation itself does not wait for reply/transfer.

`inspect_resource_registry_head_v1` and
`resource_registry_historical_address_v1` are privileged host classification inputs
only, analogous to native raw catalog metadata. Use the exact root-bound head and
borrowed native slot to evaluate current historical policy; do not forward either
product through an ordinary SDK/automation/UI result. The handoff product is
`ResourceRegistryDeliveryV1`, not `ResourceRegistryHistoricalHeadV1`. Host policy,
identity and historical classification truth remain independent premises.

Resync consumes a fresh snapshot packet and exact-true baseline permission.
`ResourceRegistryObservationControlV1` governs separate protected stop and stream/
packet retirement. Abandoned/stale packets remain charged even after their stream
retires; the pool cannot refund until both inventories are empty, and spent issuance
cannot reset. The owner performs no provider I/O, source read, registration mutation
or producer discharge. Existing exhaustive generic stop profiles remain unchanged;
public F19 command/replay/F06 binding, combined/heterogeneous routing and
explicit grant revocation are still pending.

Original registry commands now have a native implementation under
`registration-observation-command/1`. Use `issue_resource_registry_command_v1`
with the closed `ResourceRegistryCommandOperationV1` and original scope/current
admission facts. This reserves intent, not a native execution attempt. The existing
registry packet budget must support at least two slots and the complete command,
reply and historical-binding reservation; smaller event-only packet envelopes may
correctly refuse command issue. No new global issuer/store is introduced.

`prepare_resource_registry_command_snapshot_v1` supplies the distinct technical
preparation. `request_resource_registry_command_v1` binds operation/scope/reference
roles before using its original engine root and resync generation. Stale intent
requires a new explicit command; do not refresh it implicitly. First execution
returns `Settled { command }`, not baseline data. A matching settled retry returns
`ObserveExisting { command }` under protected control without reading current root,
consuming another preparation, reopening a stream or resetting its queue. Native
result unavailability is retained once, not a trigger for automatic retry.

`resource_registry_command_facts_v1` exposes privileged original native facts and
historical-slot association for first-party receipt integration. It is NOT a
recipient result endpoint or permission to return a baseline through SDK/UI/
automation control. Native receipt contexts now exist: admit with
`open_resource_registry_receipt_v1`, inspect privileged historical slots through
`resource_registry_receipt_authorization_basis_v1`, prepare complete historical
authorization with its opaque basis, then release the original result through
`release_resource_registry_receipt_v1`. The basis binds the original context,
command, result and inspected whole-engine root; any root change requires fresh
classification. External authorization must independently remain current.
Only a complete, currently authorized original result can be released; denial is
not permission to filter it. Validate returned identity, not merely equal bytes.
Stop does not release the actual command claim; retire certificates before the
stopped context, and the context before its command. Host basis/result inspection
is privileged, not recipient-facing, and copies do not create managed holds.
`release_decoded_resource_registry_receipt_v1` now lowers the exact typed/dynamic
F06 envelope to that same native release and validates the actual original result.
Its passive 2/4/6/1-field grammar accepts only fixed constants and native opaque
roles; it allocates no input frame, traverses no arbitrary nested data and invokes
no current-fact callback. The trusted `ResourceRegistryReceiptPortV1` must match
the envelope's original command/context before current facts reach the owner.
It is a first-party in-process entry, not hostile-wire/script decoding or an
authority grant. Installed discovery is now available through
`prepare_native_registry_receipt_discovery_cut_v1` on the existing scoped /1
discovery owner. It requires the actual configured native registry parent with
matching host authority/engine. Acquire a context, list/inspect its entry/variant,
then use `release_discovered_registry_receipt_v1` with a fresh current discovery
cut and the original native receipt port/envelope. Both discovery release and
native historical release must independently authorize delivery. Copied generic
metadata cannot supply the private installed origin; discovery narrowing does not
cancel execution, close the native receipt context or consume its certificate.
The captured metadata includes the closed six-field JSON Schema 2020-12 outer
shape; actual opaque identities, current policy truth and lifetime guards remain
runtime obligations, not schema guarantees. Existing receipt enums/profiles are
not aliases. Registry-specific F19 command assembly is now installed too:
`prepare_native_registry_command_discovery_cut_v1` exposes the two original-command
variants, and `request_discovered_registry_command_v1` joins their original opaque
selectors to `request_decoded_resource_registry_command_v1`. The exact 2/4/5-field
typed/dynamic grammar preserves command/stream/preparation roles out of band and
refuses extra executable meaning. Discovery does not grant native admission,
baseline disclosure or retry authority. Successful command responses contain only
the original command identity; payload delivery remains the separate F06 path.
Matching identity-only retry can use the decoded/native protected edge directly
without fresh discovery, root, preparation, ordinary headroom or content rights.
Heterogeneous cross-family F19 assembly remains open.

`retire_resource_registry_command_v1` is separate from stream and technical
preparation retirement. Busy commands refuse retirement. A faulted command whose
native call began is retained as an ordinary registry admission/delivery fence;
generic preparation cleanup cannot erase it. Protected stream cleanup stays
available, but no reset/repair or unknown-record reclamation is implied. These
ports use immutable native facts and an exclusive safe-Rust borrow; they do not
qualify arbitrary reentrant FFI/host-supplier callbacks. Fixed optional service
timings distinguish first execution, protected lookup, facts and retirement.

Read admission/recovery is a separate F20 route: obtain its two effect-request
selectors with `prepare_native_read_admission_discovery_cut_v1`, then call
`request_discovered_read_v1` with the matching `NativeDiscoveredReadCommandV1`
variant and original native request/facts. Source and annotation results retain
their original, distinct task types in `DiscoveredReadOperationAdmissionV1`.
Catalog dependencies stay in the original target. Matching recovery after edits
or catalog withdrawal returns the same child; it neither refreshes admission nor
submits another read. Use the existing reader submission/evidence APIs and
`open_native_source_read_observation_v1` or
`open_native_annotation_read_observation_v1` for independently authorized native
operation observation. EOF/quiescence are evidence, not activation or restoration.
Observer stop/retirement does not cancel the producer, and spent parent identities
cannot be reused. Metadata is descriptive native SDK roles, not a transport codec
or execution grant.

Completed-source candidate activation now joins native ownership, F06 V8 and a
fixed installed source on the existing scoped discovery /1 owner. Its
`borrow_source_load_authority_v1` factory returns a `SourceLoadAuthorityV1` only
for the actual complete submitted task under `SourceLoadAuthorityFactsV1`'s
independent control/preparation/resource assertions. The handle is opaque and
non-cloneable; repeat authorized borrowing identifies the same live witness.
`source_load_authority_current_v1` rechecks it without returning source or
activating a document. Weak markers allow real task retirement to revoke it
without retaining the read buffer. This is a provenance prerequisite, not an
activation permission or replacement for current creation/owner-access checks.
Do not promote direct `activate_source_read_v1` retries to invocation replay:
its already-activated response observes the current document revision. The
`submit_source_load_v1` / `run_source_load_v1` pair retains original authority and
context, rechecks conditional creation/owner access and the actual shared writer,
and installs terminal/outcome with birth/binding/original-marker state under one
fence. `source_load_result_v1` independently releases historical identity and
exact disposition; `retire_source_load_v1` discharges only its invocation capture.
This private native staging is not a public replacement-root API, and none of
these calls dispatch another read or overwrite an existing owner.
Use `prepare_native_source_load_discovery_cut_v1` and
`submit_discovered_source_load_v1` for installed selector admission, and
`InvocationReceiptRequestV8::SourceLoad` for independently reauthorized historical
delivery. Older receipt profiles cannot acquire this facet. V8's Existing prefix
preserves actual V7 delivery; metadata is descriptive native-role information,
not a wire grammar, serialized authority or permission for arbitrary scripts.
Annotation candidate activation and remaining whole-engine gates are separate.
Annotation restoration and loaded-baseline adoption now have shared private
read-only preparation/fenced commit stages, including preconstructed results and
provenance, current native revalidation and optional whole-service timing. The
existing public native operations use these stages; they are not public root
replacement APIs. The owned binding below composes them; installed automation
still requires F06 and discovery assembly.
`borrow_annotation_load_authority_v1` now returns an opaque non-cloneable
`AnnotationLoadAuthorityV1` for a complete native task under independent
`AnnotationLoadAuthorityFactsV1`. `annotation_load_authority_current_v1` validates
that same original task/capture without disclosing bytes or restoring/adopting.
Weak markers do not prevent retirement. Stop before consumption revokes it;
consumed provenance survives stop but not contradiction or real retirement.
Approved sealed restoration data and operation/current writer permission remain
separate requirements; this handle alone is not an automation execution grant.
`AnnotationLoadRequestV1` now supplies the original opaque sealed restoration
artifact/pair or the original read/pair/baseline bound. Use
`submit_annotation_load_v1` / `run_annotation_load_v1` with independent current
phase facts, then separately authorize `annotation_load_result_v1` for historical
identity and restored/unchanged or installed/current status. Terminal publication
releases the engine's pending proof, retaining only original weak roles/fixed
metadata; caller-held artifacts remain explicit consumer pins. Protected
`retire_annotation_load_v1` discharges only command ownership/funding. This is a
trusted native SDK binding, not a wire grammar or arbitrary-script authority.
The selected route now joins installed discovery and F06 V9. Use
`prepare_native_annotation_load_discovery_cut_v1` with two variant visibilities,
then `submit_discovered_annotation_load_v1` with the matching restore/adopt-baseline
selector and original request. `native_annotation_load_argument_declarations_v1`
describes those trusted native roles, not a wire grammar. Deliver historical
results through `InvocationReceiptRequestV9::AnnotationLoad`; original capabilities,
complete reply bounds and independent current common/native release remain required.
Earlier receipt profiles cannot acquire this facet; V9 Existing delegates actual
V8 delivery. This adds no decoder, registry or read/write dispatch. Other whole-
engine facade/result routes remain separate.

`submit_discovered_native_ordinary_edit_v1` now joins the installed `/3` or `/4`
F11 ordinary-edit selector to `NativeDiscoveredOrdinaryEditV1`'s four normalized
forms: literal, explicit line, boundary and configured line. It reuses the native
owners and their original replay/currentness/evidence/settings behavior, with no
new inventory or issuer. `NativeDiscoveredCommandRouteV1` is an alias over the
same opaque discovery roles. This is explicitly a **trusted embedding SDK** API,
not authorization for arbitrary scripts: discovery and view contexts grant no
edit rights. Keep independent host authorization, and do not expose native raw
outcome getters as recipient result delivery. Complete recipient-policy and
ordinary-result facade integration remain open. The selected lifecycle routes
are exercised through F06, including historical results after disposal and native
hold retirement; repeated retirement refuses without refunding twice.

For accepted composition, normalize the closed native `composition-commit/1`
arguments or typed/dynamic F11 envelope with the original full view context,
seven-field execution context and explicit `CaptureLimitsV1`. Use
`prepare_native_composition_discovery_cut_v1` and
`submit_discovered_native_composition_v1`, or the independently authorized native
`submit_composition_replacement_v1` port. The native expected span is UTF-8 text
with exact byte offsets, not a public wire identity/byte codec. One accepted
surrounding-text range replaces once; secondary and peer selections map normally,
and only the original initiating primary collapses before normalization. Failed
preparation/envelope admission leaves the ticket unbound. Exact replay retains
the original context, replacement and limits; protected stop and ordinary native
terminal outcomes reuse the existing ledger. Host edit authorization and current
recipient result disclosure remain independent. Preedit, focus, platform indexing
and native composition sessions belong outside the engine; this is not full C2
recipient-result assembly or permission for arbitrary automation scripts.

For source-only exact batches, use the seven-field native `exact-source-batch/1`
projection with `ExactBatchEnvelopeValueV1::Sources`, `Reads` and `Scopes` carrying
the original typed `DocumentEdits`, V3 read request and source scopes/budget.
`normalize_exact_batch_arguments_v1` and `normalize_exact_batch_envelope_v1` read
configured limits but perform no current-document lookup. Capture byte/node/depth
units describe the closed native value graph and opaque identity leaves, not a
JSON/wire identity codec; read/scope interpretation budgets remain explicit and
part of original intent. Submit with `submit_exact_batch_arguments_v1`, or install
`prepare_native_exact_batch_discovery_cut_v1` and use
`submit_discovered_native_exact_batch_v1`. Host admission/service authorization is
independent of discovery and result disclosure. The private source/annotation
capture is never returned, replay does not refresh it, and current-service views
map through the common atomic publisher. No annotation authoring, new created
result reference, UI state, scheduler or external effect is implied. Mixed authored
source/annotation facade and complete recipient result assembly remain separate.

For mixed authored batches, normalize `atomic-source-annotation/1` with ordered
`AtomicAnnotationComponentV1` values and the original typed source/read/scope roles.
Pass `TrustedAtomicAuthoringV1` separately: every component binding names the
original invocation, ordinal, document and operation, with current existing F13
premises/limits/required reads. Source rights do not grant annotation authoring;
annotation-only batches do not require source mutation rights. Submit through
`submit_atomic_authoring_arguments_v1` or the sealed installed
`submit_discovered_atomic_authoring_v1`, and service with
`run_atomic_authoring_v1`; generic `run` refuses unvalidated mixed execution.
Before/candidate meaning and create allocations never refresh on retry. Use the
existing compound annotation result admission/release and F06 atomic-annotation
facet, not a command-response disclosure shortcut. Parent preparation/receipt
funding survives stop and forgetting; protected result-holder retirement releases
it and drops the retained mixed capture without restoring lifetime issuance.
Input limits describe native logical data/frames, not JSON/wire or RSS bounds.
Existing UI/substrate boundaries and independent host/policy truth remain intact.

For scoped optional marker capture, configure `DiagnosticCaptureLimitsV1` with
`configure_diagnostic_capture_v1` under a trusted-host `DiagnosticControlV1` and
the complete eight-field scope. `prepare_diagnostic_marker_v1` (or the finite
already-decoded marker guard) returns an original single-use token;
`capture_diagnostic_marker_v1` accepts that token and current collection facts
without record-time growth or clock access. It is descriptive instrumentation,
not effect evidence. `drain_diagnostic_markers_v1` requires separate current
release facts, the original native generation and an explicit bounded prefix.
Protected health needs its own control facts and never instruments itself.
Counter resets identify a new opaque generation; lifetime preparation issuance
and the one-consumer binding budget do not reset. Explicitly stop/retire remaining
holders before queue retirement. A native `DiagnosticDrainBindingV1` is unique,
not Clone; move it into its actual consumer rather than discarding the handle.
Dropping a handle does not release its known queue hold. Marker occupancy uses
canonical marker bytes; native context/reply/funding units are not JSON or RSS.
The richer `Recorder` remains separate. Direct drain is not replayable task
completion, export, persistence or UI scheduling.

For owned F26 diagnostic work, `open_diagnostic_drain_v1` inspects the actual
queue and prepays bounded task/batch/result/control storage before spending its
one consumer binding. Supply the matching scope under
`OWNED_DIAGNOSTIC_DRAIN_PROFILE_V1` and independent current control facts. Issue
with `issue_diagnostic_drain_v1`; issuance is not admission. Normalize the exact
contract/task/generation/limit request using the native argument or typed/dynamic
envelope guard. Use `submit_diagnostic_drain_arguments_v1` or the sealed
`prepare_native_diagnostic_drain_discovery_cut_v1` /
`submit_discovered_diagnostic_drain_v1` route. This is the existing F26
effectRequest/hostRequest facility, not a native editor invocation or scheduler.

Service with `run_diagnostic_drain_v1`, supplying fresh bridge facts and separately
attributed queue-release facts. Missing readiness leaves pending work; eligible
service attempts the current queue prefix once. Matching replay and
`query_diagnostic_drain_result_v1` never redrain. Result copies require fresh
ordinary release rights, not owner-control rights; generation change removes an
old retained payload without rewriting its performed fact. Protected
`query_diagnostic_drain_control_v1` reports phase/admission/disposition without a
batch. Unknown/faulted here describes the optional bridge, not source publication
or a native-engine fault. No panic text is retained in task products. Stop tasks,
retire issued/terminal holders, stop and retire the gateway, then retire the queue
under their respective current discharge facts. Global admission fencing prevents
new issuance/first submission but preserves admitted service, exact replay and
cleanup. Generic F06/events/wire and release performance qualification are separate.

For native returned-search replacement, use
`submit_returned_search_replacement_v1` with the original opaque result/occurrence,
complete source scope, explicit capture limits and independently attributed native
read/query/mutation facts. The result must first have passed actual native release;
a query ticket alone is insufficient. Run through `run_managed_search_consumer`
(or its source-only convenience port), supplying fresh facts. Generic run refuses.
Original intent, limits and result identity survive replay without re-preparation.
The canonical capture and returned reply frame have separately funded holds;
stop/forget retain them. After settlement use
`retire_returned_search_replacement_v1` with the existing
`InvocationObservationPolicyControlV1` exact context and protected discharge facts,
then retire the invocation. This does not retire the original query/issuer holds.
Frame accounting is native logical data, not JSON bytes or RSS. Use
`normalize_replacement_arguments_v1` or `normalize_replacement_envelope_v1` for
the exact four-field request (operation, index, replacement, invocation), supplying
the original result independently. Replace-all requires a null index; current
replacement requires a safe natural index validated by the actual owner. Submit
with `submit_replacement_arguments_v1`, or install
`prepare_native_replacement_discovery_cut_v1` and use
`submit_discovered_native_replacement_v1` with independent native facts. Owning
declarations describe only these existing `literal-search/1` replacement variants;
copied metadata has no handler. Retained ordinary outcome delivery uses the
original F06 observer and separate current release authorization, not command
replay or a new result store. Do not advertise a complete logical F05 search facade
or invent a source-label resolver for this native opaque-result route.

For F13, `submit_discovered_annotation_v1` joins the installed annotation selector
to the existing normalized create/update-body/reattach/delete request and scoped
owner. Pass `NativeDiscoveredAnnotationAdmissionV1` with independent execution
context/current authorization facts, plus an optional original result-admission
grant. Discovery is not annotation-write permission. Omitting result admission
uses the established no-result owner path and cannot acquire disclosure rights
by later replay. Result delivery stays in the F06 annotation facet, including the
original-allocation-scoped create grant; it does not return content through
command dispatch. Current source read-only status is independent of annotation
write authority. Existing service and protected retirement remain unchanged.

For F14, `submit_discovered_native_reversal_v1` accepts the original opaque
installed undo/redo selector and `NativeDiscoveredReversalRequestV1` (invocation,
native group/frontier target, closed direction, declared reads and result
admission). It delegates to the existing read-aware reversal owner. Host caller
authorization remains independent, exactly as for direct native reversal; do not
treat discovery or target-disclosure grants as mutation rights. Matching replay
does not re-resolve the frontier or execute twice. Use the existing F06 reversal
facet for historical results and current target-disclosure filtering. Native
multi-document atomicity and final read-dependency validation remain in force.

For installed F16, use `submit_discovered_context_v1` with the original opaque
discovery route, execution context and `NativeDiscoveredContextCommandV1`.
Its ten closed variants pair the native/scoped values, layer lifecycle,
serialization, definition and registration normalized inputs with their matching
facts and opaque bases. Each delegates to the existing guarded port: discovery
does not grant permission, construct bases, refresh intent or select defaults.
Service and result release remain separate. To request created-layer results,
use `submit_discovered_native_lifecycle_with_created_results_v1` with the same
normalized lifecycle input and independent result-admission facts before original
admission. Its normalized counterpart is
`submit_normalized_native_lifecycle_with_created_results_v1`. Both retain the
owning port's capture ceiling, parent funding and original result allocation.
Repeating opt-in after binding refuses; use the ordinary lifecycle arm for
matching replay. A command admitted without results cannot gain them later.
Bind observation policy before admission and use F06's `CreatedSettings` facet
with independent current release facts; discovery visibility grants neither
mutation nor result disclosure. Protected cleanup uses the existing result owner.

Keep each owner's replay semantics. Scoped registration specifically performs
current scoped service on resubmission: installation can stale the original
absent-slot basis, so a later resubmission can return unavailable while historical
result delivery remains valid. Do not interpret that refusal as evidence that
the original operation did not publish. No source text is rewritten by a
serialization metadata change, and no provider I/O is performed by registration.

| Layer | Responsibility |
| --- | --- |
| Engine | Explicit logical targets/revisions, validation, atomic publication, history, authorized reads, typed outcomes and effect/evidence ownership. |
| SDK | Ergonomic typed access to supported contracts, preserving opaque identities, bounds, permissions, errors and lifecycle obligations. |
| Automation and integration | Workflow composition, scheduling and clipboard orchestration above ordinary engine commands; provider wiring is explicit. |
| UI | Keymaps, modes/chords, focus, preedit presentation, layout, painting, hit testing, scrolling, accessibility/platform presentation and visual-to-logical target translation. |

Use the same semantic operations for UI and headless automation, not simulated key
presses. UI geometry resolves to an explicit logical target before submission.
Clipboard provider acknowledgment is not paste publication or cut deletion; keep
those workflows outside the semantic engine and preserve their original task,
capture, target and attributed evidence.

## Trust and compatibility

`SourceCoordinator` includes privileged trusted-host APIs. Native fact structures
carry host premises; filling their fields with true or copying context labels does
not authenticate a caller or grant authority. A wrapper around these APIs is a
trusted embedding SDK, not a security boundary for arbitrary scripts. Do not expose
raw trusted getters or control methods as recipient-authorized products. Untrusted
automation requires a real mediated boundary and separately verified host policy.

Keep executable operation/control/outcome enums exhaustive within their explicitly
versioned profiles. Reject unsupported behavior without silently choosing a legacy
API, upgrading rights or changing dependency meaning. APIs remain provisional;
do not publish stable compatibility promises or invent transport encodings. No
HTTP server, mandatory executor/runtime or network dependency is required.

## Supported operation matrix

Maintain a small matrix in the consumer package with one row per advertised
operation/profile: actual owning API, trust role, exact inputs/results, currentness/
disclosure checks, retry identity, limits, cleanup and an executable acceptance
test. Record unsupported operations explicitly. Reuse implementations rather than
reconstructing the entire design catalog in the SDK.

Native F06 currently supports funded original-policy/observer ownership, protected
phase-only status, and filtered coarse local-command status/outcome. Ordinary
references track permitted projection changes rather than global publication;
protected control remains independent of ordinary capacity. These native products
do not constitute a wire protocol, comprehensive event stream or stable SDK.
Native bounded waits are also available over owned ordinary references. Supply
validated logical clock/service facts explicitly: polls do not spend time, valid
epoch/tick events consume one step, and ended rows require protected retirement.
This is cooperative service, not an installed timer or wall-clock deadline.
Six native operation receipts now compose through an admission-bound typed facet:
annotation, reversal, relation, host/scoped registration and created settings.
Retain their separate native release permissions and original producer captures;
coarse details rights do not enable receipt access. The host-registration variant
is privileged host metadata, not a recipient-safe serialized product. Native
receipt faults disable only that receipt facility; do not turn them into rollback
or loss of protected phase control.
Distinguish those receipt-only faults from lifecycle publication interruption.
Retained disposal and binding/provenance publication now fence the engine owner
and propagate an unexpected panic as Unknown, not a retryable refusal. Do not
replay the mutation, infer success from a partially changed root, or drop retained
task responsibility. Native view attach/detach and membership have the same
publication fence and optional actual service timing. Their existing trusted
native calls remain usable. The [owned live-relation producer](../crates/vellum-engine/src/coordinator/invocation/live_relation.rs)
now supports attach, exact-document detach and workspace membership with original
request/context/target generations, shared funding and fresh applicable rights.
Use its separate retained-result release and protected hold retirement; a historic
result is not current view/workspace truth. Generic service cannot supply its fresh
facts. The [native F06 V5 profile](../crates/vellum-engine/src/coordinator/invocation/observation/receipts_v5.rs)
now composes those results under original observer ownership, a captured lifecycle
ceiling and common/native release checks. Use its matching open/query ports and
complete wrapper bounds; V1–V4 contexts cannot upgrade by fallback. Native receipt
faults preserve protected status and cleanup. Public decoder/discovery remain open.
Do not translate a propagated panic into an automatic relation retry or claim that
unchanged content means no relation installation occurred.
A standalone retained-disposal invocation producer now also
offers an original-ticket historical result under captured and current release
checks. Its exact-root plan can become stale; matching retry does not reprepare
or reexecute. Protected capture retirement releases its own hold without settling
Unknown work. The V2 receipt observation profile now composes this disposal result;
V1's exhaustive six-family result enum remains unchanged. V2 wraps those existing
results and adds retained disposal, with an admission-bound profile/family ceiling
and complete wrapper bounds. Use the matching version's query port: queries cannot
upgrade a V1 context. Other lifecycle producers remain unavailable; do not advertise
the complete lifecycle family from this port.
Native source-binding, annotation-binding and provenance release now have separate
original-ticket producer/result APIs in
[retained release](../crates/vellum-engine/src/coordinator/invocation/retained_release.rs).
Their expected lifecycle guards survive unrelated publication but not targeted
lifecycle change. An original refusal remains spent after late writer evidence;
use a newly authorized invocation to attempt release again. Historical applied/
unchanged results are not current binding or provider-quiescence claims. These
three result types compose through the explicitly selected V3 receipt profile:
[V3 implementation](../crates/vellum-engine/src/coordinator/invocation/observation/receipts_v3.rs)
and [public consumer scenarios](../crates/vellum-engine/tests/retained_release_observation_behaviors.rs).
V3 wraps prior products and adds retained release; V1/V2 remain exhaustive and
unchanged. Use the matching admission/query version and complete V3 reply bounds.
Do not reinterpret a prior context, route release through disposal or silently
fall back to raw trusted getters for recipients. These are provisional native
profiles, not a stable wire protocol or SDK compatibility promise.

The trusted native compound annotation producer/result port is now available in
[compound results](../crates/vellum-engine/src/coordinator/invocation/compound_results.rs).
Use `submit_compound_with_annotation_results_v1` only as a first-party embedding
over the existing native compound authoring path. Its result grants do not supply
mutation authority or public mixed-plan admission. Create grants bind to one
original annotation component and its exact allocation, never siblings; preserve
original order even if a UI filters or an automation layer reorders presentation.
`retained_compound_annotation_result_v1` applies captured/current disclosure rights
to historical results. Generic forgetting is blocked while this hold exists:
use strict retention/drain forgetting and separately owner-controlled retirement.
Do not route recipient requests around F06 via this privileged native getter.
The explicit [native F06 V4 profile](../crates/vellum-engine/src/coordinator/invocation/observation/receipts_v4.rs)
now composes atomic results under original observer ownership and independent
atomic-family/common/native result ceilings. Use its matching open/query ports
and complete wrapper bounds; old contexts and the old grant-less atomic selector
cannot upgrade by fallback. Receipt-only faults preserve protected status and
cleanup. Public decoder/discovery assembly and mixed-plan authority remain open;
standalone create grants and exhaustive receipt V1–V3 enums are unchanged.

The following engine work still needs coordination before advertising it:

- Other document-lifecycle receipt producers and complete public observer integration.
- Complete accepted facade/discovery coverage and remaining result families.
- Heterogeneous F19 events, revocation and complete command/result/event correlation.
- Complete engine-owned shutdown inventory, disposition and protected cleanup.
- Whole-engine operational/performance qualification.

Keep this matrix synchronized with current source as those items land; these
limitations are a handoff snapshot, not permission to defer required engine work.

C3 native lifecycle/retained-target qualification is closed in the fixed engine
ledger. The trusted retained-owner removal path and its existing native owners
pass the affected-owner checks; it does not require an additional public cleanup
verb. This does not close public observer/facade assembly or shutdown above.

## Lifecycle and acceptance

### First deliverable

Create a small trusted native Rust consumer package or example after coordinating
its workspace entry. Deliver one headless scenario covering document creation,
explicit-target editing, original outcome retrieval and history, with stale-target
and changed-intent retry tests. Publish its supported-operation matrix alongside
the code. Reuse existing behavior fixtures where appropriate, but exercise public
engine APIs rather than private engine test hooks.

Keep the first increment synchronous and transport-free unless the demonstrated
use case requires otherwise. Do not wait for a complete recipient facade to build
a trusted embedding, and do not present that embedding as safe for untrusted
scripts. Add the ratatui example only above the working headless layer; UI completion
is not a prerequisite for this first deliverable.

### Acceptance checks

1. Begin with a headless native consumer that creates/reads a document, submits an
   explicit-target edit, retrieves its original outcome and exercises history.
   Add only operations whose actual supported profiles satisfy the use case.
2. Test stale targets, denied/unknown authority, foreign identities, matching retry,
   changed-intent retry, no-op and bounded capacity. Matching retry must never
   allocate a new attempt or silently rerun the command.
3. Preserve pending, unavailable retained facts, known refusal, Unknown effect and
   quiescence as distinct meanings. A missing result is not nonapplication; an
   observer ending does not stop its producer.
4. Retain native handles through explicit protected retirement and delivery drain.
   Dropping an unretired invocation observer does not refund its parent reservation.
   Do not invent automatic refund, final reclamation or complete shutdown recovery
   where engine support is still open.
5. Make a thin ratatui example a consumer of this same layer. Keep keymaps and
   presentation state there; prove equivalent logical requests headlessly rather
   than requiring pixel or keymap equality. A separate UI is not an engine gate.

Instrument actual SDK/integration/UI interfaces separately from engine execution.
Use applicable MELT, Golden/RED, USE, STRIDE and LINDDUN checks for latency/errors,
bounded resource pressure, authority/identity and disclosure. Diagnostic events are
not an authoritative subscription stream or effect evidence. Preserve behavior
with capture disabled or saturated; keep content, caller labels and opaque tokens
out of ordinary diagnostics. Measure relevant consumer overhead before making
responsiveness claims; do not select a product threshold implicitly.

Run the new package's focused tests, strict Clippy and formatting checks. Initially,
the existing native observation consumer regression target is:

```sh
cargo test --offline -p vellum-engine --test invocation_behaviors
cargo fmt --all -- --check
```

Coordinate workspace-wide formatting edits and broaden engine/provider testing only
for shared-contract changes or agreed integration milestones.

## Completion report

Report the supported-operation matrix, executable headless scenarios, UI versus
SDK/integration ownership, exact checks/results and remaining engine dependencies.
Separate a trusted native embedding from a recipient facade and a prototype from
a qualified SDK. Consumer/UI completion does not close the engine goal.
