# Vellum

A headless, agent-friendly editor engine under development. Requirements and
models remain a provisional design baseline; supported native behavior is now
implemented through the Rust owner facades described below.

## Current native baseline

The engine supports lossless source handling, bounded cross-document edits,
annotations, logical views, linked history, retained queries/search/results,
semantic settings and typed publication observations. Resource tasks preserve
original ownership and separately attributed evidence; cooperative shutdown keeps
unresolved obligations and permits final retirement only after actual discharge.
Instrumentation is optional, bounded and content-safe, with separate service,
queue and first-attributed-result timing and explicit diagnostic loss.

UI/keymaps/OS clipboard and automation ID lookup belong above the engine.
Filesystem/S3 mechanisms belong below it; the first substrate module is the
separate local-filesystem adapter. Analysis and distinct provider-context
activation remain disabled; v1 catalog correspondence is binding-only.
The [native contract matrix](docs/public-contract-matrix.md) defines supported
profiles and the [fixed delivery checklist](docs/engine-test-plan.md) records
qualification. All seven native delivery gates are closed: full debug and release
runs each pass 2,880 tests across 88 targets; strict workspace lint, formatting,
the documentation command and the explicit pinned Unicode gate pass. The
release-only performance experiment is separately qualified, not silently waived.
This is not a selected wire protocol, product latency SLO or production-provider
certification.

## Rust implementation

Run `make check` from this directory (Rust/Cargo, rustfmt, Clippy and Make).
Alternatively run the four commands in the Makefile individually. Checks run
offline; the engine crate has no third-party dependencies. The separate macOS
filesystem adapter uses pinned `rustix` bindings; the separate reference annotation
JSON codec uses pinned Serde/serde_json. `Cargo.lock` is kept
for repeatable workspace builds. The current development check uses Rust 1.95.0;
this is not a selected MSRV or a supported-platform promise.

The separate `vellum-unicode-reference` adapter uses pinned SHA-256/Base64/JSON
libraries and the existing pinned ICU4J 79.1/Unicode 18 reference tool. It is not an
engine dependency or production-provider selection. `make check` compiles all
feature-gated test code but does not need Java. Run the strict reference gate with:

```sh
VELLUM_ICU4J_JAR=/absolute/path/to/icu4j-79.1.jar make check-unicode
```

Java is required (`VELLUM_JAVA` may name its executable). Missing prerequisites,
wrong artifact/version, protocol errors and corpus mismatches fail this explicit
gate; nothing is silently skipped. Provider execution never runs in engine commit.

### Runnable headless session

`cargo run -p vellum-engine --example headless_edit` discovers the installed
batch-edit command, changes `Hello, world!` to `Hello, agents!`, verifies replay
without re-execution, reads a separately permission-filtered retained result,
and releases its invocation, observation, history, document and discovery owners.
It uses explicit trusted-host premises, not a policy evaluator or credential
system; it requires no UI, filesystem adapter, service or network runtime.
`cargo test -p vellum-engine --test headless_delivery_behaviors` exercises this
same executable path with capture disabled, enabled, timed and saturated.
This example exercises editor-owner cleanup. The separate public shutdown
scenarios exercise retained reports, late evidence, reclamation and final control
retirement; running this demo alone is not whole-engine qualification.

### Implementation notes (chronological)

The notes below retain earlier slice boundaries. Historical “open” statements
describe those slices, not a request to rebuild routes closed in the current
native contract matrix and fixed delivery checklist.

The `vellum-engine` crate contains:

- Native `engine-measurement/1` descriptor/value semantics and optional actual
  diagnostic offered/captured/drained/dropped counters. Saturation is explicit,
  unknown is not zero, and cross-generation deltas refuse. The legacy bounded
  event queue remains unchanged. No exporter, histogram default, UI measurement
  or claim of installed probes for all engine populations is added.
- The actual source/annotation transaction publisher has separately parent-funded
  disposition counters: applied, unchanged, refused and invariant fault are not
  inferred from log volume or participant counts. Protected collection teardown
  cannot cancel editing or rearm counters; other publisher populations remain open.
- Native invocation measurements count first admission cohorts and once-only local
  dispositions, not issuance, offers or replay. Late activation excludes previously
  admitted jobs; forgetting results does not erase counts. Retained execution faults
  remain distinct from ordinary refusal and from provider/effect completion.

- Pure UTF-8 source decode/encode and bounded original-snapshot multi-edit
  preparation, with executable Given/When/Then scenarios. See the
  [engine acceptance plan](docs/engine-test-plan.md) for the model and limits.
- Explicit native logical scratch birth preserves authored leading U+FEFF and
  mixed endings, with independent serialization/newline intent and no implicit
  view, membership, baseline or edit history. Scratch and import share a bounded,
  fault-fenced owner publisher while preserving their distinct text/birth-counter
  semantics. Owned scratch invocation now captures one funded immutable intent,
  binds its result to the actual publisher allocation, and supports separately
  authorized historical identity release and protected hold retirement. Explicit
  F06 V6 composes that native scratch result without upgrading older profiles;
  complete public ingress/discovery remains open.
- A closed trusted-native lifecycle behavior facade delegates scratch, the three
  live relation operations and prepared disposal to their existing invocation
  owners. It adds no job, executor, UI/substrate path or authority; native results
  and cleanup remain separate. This is not a string-ticket/data-envelope decoder.
  Its exhaustive V2 facade includes unchanged V1 plus explicit prepared disposal
  with matched fresh coverage, retaining one outer timing/fault boundary.
  A data-first native request entry covers the same five operations without a
  caller-retained disposal plan: first preparation is authorized and parent-funded,
  while matching replay never refreshes a later root or rearms forgotten work.
  Exact bounded in-process argument guards now lower the five native forms while
  preserving native roles; registered discovery/dispatch integration remains open.
- Explicit native disposal binds all original versions, binding presence and
  retain/discard choices to the original owner, with fresh applicable coverage
  and actual publisher checks. Legacy service cannot bypass it; neither choice
  claims physical erasure or collapses legacy bound-at-zero into unbound.
- A trusted-owner source coordinator with opaque incarnation handles, exact
  revisions/read dependencies, bounded preparations, read-only policy checks and
  all-or-none cross-document source publication. This is not yet the complete
  command/recipient/annotation/view facade.
- Bounded canonical source queries: exact range bytes and separate logical line
  body/delimiter products, with current revision/liveness checks and no incidental
  total counts. Explicit versioned byte/line/scalar/UTF-16 conversions never round
  invalid positions. CRLF interiors stay valid byte coordinates but have no line
  body-column projection. This is not grapheme movement or geometry.
- Opt-in headless logical views: non-reused identities, explicit attachment and
  selection epochs, bounded positional normalization and line/document endpoints.
  Source edits and undo/redo stage every affected view with source/annotation/history
  publication; stale peers and exhausted counters refuse wholly. Equal-byte edits
  may change selections without source revision/history. No keymap, focus or rendering.
- Literal selection editing: exact original selections derive bounded replacements;
  explicit mapped or carets-after policies join the same atomic source, annotation,
  view and history publication. Stale peers, ambiguity and exhausted counters refuse
  without a prefix. Retained literal invocations compare original context/text/policy
  before reacquisition, execute once and keep forgotten outcomes spent. No UI input
  interpretation or implicit text normalization.
- Retained selection-only set/endpoint commands use the same one-shot invocation
  ledger and shared native selection publisher. Typed retained outcomes distinguish
  selection changes/refusals from source commits. Read-only movement is permitted;
  no source revision or history entry is manufactured. Original raw-list order,
  primary and endpoint budgets remain retry intent, even for equal resulting sets.
- Explicit simple line edits: bounded selected-line indent/outdent and generated
  LF/CRLF/CR with optional prior-prefix copying. Tab-aware columns, minimal prefix
  suffix replacements and newline carets use shared atomic source/annotation/view/
  history publication. Retained line invocations preserve original settings/work
  profile, preparation/refusal and one-shot semantics. No guessed indentation style,
  grammar or UI keybinding.
- Configured line edits resolve explicit target/key roles from actual source-domain
  settings and retain a charged interpretation. Current interpretation epochs,
  target source generation, values and source/view context revalidate through the
  same publisher. Stop/forget retain the lease until protected release/retirement;
  no setting key catalog, recipient grant or UI preference is inferred.
- Runtime boundary-table ownership: exact-source capture, bounded structural
  acceptance, non-reused provenance, stale-source checks and protected retirement.
  The Unicode 18 profile is explicit; host attestation and table shape do not prove
  segmentation conformance. The reference adapter now passes the pinned corpora
  through actual native capture, commands, replay and undo. Native motion/deletion
  now consume those tables with finite work, final eligibility checks, cluster-safe
  implicit deletion and exact explicit selections. Both operations now use original-
  evidence one-shot invocation/replay and typed retained results/refusals. Production/
  incremental provider selection remains open. Incompatible word vectors refuse
  only that word command, preserving grapheme editing and unchanged boundaries.
  No provider process runs in source publication.
- Opt-in bounded source invocations: immutable normalized intent, matching retries
  without re-execution, separate retained-outcome queries, status forgetting and
  retirement without identity reuse. Source-only stop is not provider cancellation.
  Usage gauges distinguish retained results, pending work and spent lifetime IDs.
- Context-bound pending cancellation: strict current protected-control facts and
  the original seven-field context set only monotone stop intent. Actual service
  later records stopped-before-publication without source/configuration changes.
  Request acceptance is not settlement, refund or provider quiescence. Existing
  scoped settings/declaration/lifecycle and serialization jobs support this port;
  context-free trusted-owner jobs refuse it rather than inventing a context.
  Faults remain fenced, publication-first outcomes survive late cancellation, and
  independent basis/result holds still require their own protected cleanup.
- Opt-in mediated current/retained-source contexts: parent-funded immutable captures,
  opaque non-grant selectors and range/line queries with fresh complete-scope and
  policy checks. Later source edits neither refresh nor invalidate a capture.
  Accepted loss, disposal and explicit stop close delivery without resurrection;
  internal holds preserve actual stopped snapshots until protected discharge.
  Shared spent issuance never refills, and only empty-pool retirement refunds its
  reservation. Retained acquisition copies an original available context's exact
  cut without a current-root fallback or a hold on its original. Copies remain
  independently releasable/stoppable, and read retention never grants stale edits.
  This source-only typed port does not imply annotation/combined, arbitrary
  historical/version-set, full managed-search, discovery or public wire-product closure.
- Opt-in managed source/combined/version-set literal matches and navigation: a separately parent-funded result
  pool holds the actual original read context before evaluation. Opaque result
  handles require fresh, independently attributed source and query facts for each
  release. Query permission loss fences its result, not independent source reads.
  Protected retirement discharges the source hold once; row retirement never
  refunds pool funding or lifetime issuance. The existing literal matcher is shared.
  Navigation shares the trusted ordering/wrap kernel, binds opaque occurrences to
  their actual original result, and requires a complete eligible result. It spends
  no new holds or issuance. Managed replace-all/current and match-selection jobs
  recheck the original result at preparation and service, with independently
  attributed mutation authorization. They use the shared ledger/publishers;
  generic execution cannot bypass fresh facts. Prepared work owns no new read lease.
  A closed first-party issuer sum retains source-only, combined or version-set ownership;
  combined results require both classes' current whole-projection rights. The
  shared search pool survives bootstrap-pool retirement. Other projection issuers
  and complete public products remain open.
- Annotation/combined preparation cuts: a separately parent-funded pool copies only
  actual permitted facets from one root. Annotation-only cuts retain no source or
  source-product profile. Fresh whole-projection handoff validation can absorb
  loss; protected cleanup never refills issuance. A separately funded installed
  owner consumes a cut once and survives preparation-pool retirement. Combined
  range/line reads preserve the original cut and require both classes' current
  rights. F03 pages, inspection and target locations use context-bound opaque
  selectors, whole-page registration and fresh release checks; retries reuse
  identities, and retirement removes lookup paths without refilling issuance.
  Combined F05 holds are independently capped and share nonrefilling issuance
  with contexts/F03 selectors. Held stopped contexts cannot retire. Broader
  retained-version-set/arbitrary-history/public products remain open.
- Retained annotation/combined acquisition copies an original available installed
  cut, with explicit source-only, annotation-only or combined narrowing. It cannot
  widen, fill absent facets from a newer root or share the original's stop state.
  Copies have fresh selectors and no hold on the original; shared search/current-use
  checks preserve old read meaning without granting stale mutation authority.
- Coherent version-set preparation stages ordered explicit members under one
  exclusive owner borrow, reusing the projection builder rather than issuing child
  reads. Its private cut survives newer publication; whole-set handoff validation
  observes every required member. Atomic installation transfers the cut once to a
  separately funded parent and ordered opaque member aliases. Any member stop
  closes its whole parent. Stop requires original owner scopes, not drain/release
  facts; retirement still requires those facts and zero outstanding whole-parent
  holds. Already
  delivered bytes cannot be recalled. Preparation-pool retirement cannot erase the
  installed set. Member source range/line reads use fresh all-peer checks and original
  source values with independent inner/outer envelopes. Member annotation pages,
  inspection and locations use fresh all-peer checks and member-bound opaque
  record/cursor selectors; whole-page registration is atomic and retries reuse IDs.
  Member searches hold the whole parent, recheck every required peer at release/
  execution, and use complete protected control frames for result retirement.
  Retained sets can independently copy ordered member/class subsets from an
  available original parent, preserving old provenance without widening or sharing
  holds/stop state. Complete public read/result products remain open.
- Source-only, combined and coherent-member source replies now pass a shared
  pre-release binding/shape guard. Malformed native products fence the owner and
  return unavailable; a bounded trusted-owner invariant code survives lost telemetry.
  These checks are not a replacement for the byte/line oracle or public wire codecs.
- Annotation/combined and coherent-member F03 replies validate actual staged alias
  bindings and optional facets before selector commit. Malformed products fence the
  owner without installing partial selectors; protected cleanup remains available.
  Native checks do not imply decoded-facade guards or rollback after release.
- Current/retained source, annotation/combined and coherent acquisitions check
  final identities/classes/rosters against actual candidates before installation.
  Malformed handoffs preserve prepared cuts and registry issuance while fencing
  the owner; bounded optional diagnostics do not determine health.
- Native inert-data capture provides a bounded nonrecursive copier for future
  decoded facade normalization. It preserves plain values without copying opaque
  capabilities; parent reservation, routing and authority remain facade obligations.
- A native source-read adapter now composes those captures with fixed source,
  combined and coherent-member ports, actual parent workspace reservation and
  protected cleanup. Final release follows cleanup validation. The full decoded
  typed/dynamic facade and public identity/wire products remain open.
- Native annotation enumeration/inspection/location use the same read-adapter
  guard, including annotation-only coherent members. Late facade failures withhold
  data without rolling back complete native selector registration or spent issuance.
- Native current/retained read acquisition now shares that guard with eight bounded
  frames and exact private preparation binding before loss or consumption. Late
  failures retain native installations and consumed cuts; decoded/public products
  and complete undelivered-result recovery remain open.
- Native history-metadata bases capture complete linked participants and both
  frontier stacks without journal payloads or inverse-history pins. Scoped endpoint
  comparison ignores unrelated publication; it grants no reversal authority and
  does not itself provide recipient-facing history contexts.
- A separately funded native history owner now retains bounded alias projections,
  serves frontier/availability/permitted-detail advice and permanently invalidates
  observed stale contexts. Protected retirement preserves monotone issuance;
  reversal preparation remains independent. Pre-release result guards fence
  malformed native products. A fixed native facade now copies bounded ordinary
  queries/current facts while preserving original handles and owner-held ceilings;
  typed/dynamic `history-query/1` native envelopes now share exact argument capture
  and the original opaque context. Native typed/dynamic source-read arguments also
  preserve original source/combined/coherent-member and document capabilities;
  serialized document-identity and public wire/result codecs remain open.
  Native annotation-query typed/dynamic arguments now preserve context-specific
  record/continuation selectors and optional facet withholding through that guard.
  Native snapshot-handoff arguments bind all six current/retained acquisition ports,
  original references and ordered typed participants before loss or cut consumption.
- Opt-in linked source history: atomic frontier-only undo/redo, fresh operation
  identities/revisions, bounded journals, pins and complete dependency-closed
  retirement. Reversal invocations use the same retained-outcome ledger. Typing
  coalescing and recipient-specific history views remain open.
- Opt-in pinned source saves: immutable encoded captures, opaque resource binding,
  shared destination FIFO, separate application/synchronization/control evidence,
  bounded contradictory witnesses and explicitly published historical baselines.
  A saved baseline is not proof of current external contents or crash durability.
  Producer and outstanding delivery holds prevent premature task retirement.
- Opt-in exact literal search: bounded sessions/results, original UTF-8 ranges,
  captured scopes, explicit next/previous wrapping and complete-versus-exhausted
  results. Replace-current/all use ordinary atomic source/history publication and
  invocation replay. Superseded queries and source revisions refuse at final
  publication. Known occurrences from partial results may be selected or replaced
  explicitly; navigation and replace-all still require complete results.
  Managed search now checks native product shape and retained-result identity at
  creation, release, occurrence and navigation boundaries; surprises fence the owner.
  A fixed native search adapter now bounds independent query/current-facts copies,
  preserves original issuer/result bindings and shares protected admission/refund.
  `try_search_facade_v1` preserves typed native errors through all four ports;
  the existing optional-result API remains a compatibility projection. Authority
  refusals stay deliberately indistinguishable, and late faults do not imply rollback.
  Source, annotation, history and acquisition facades (native and decoded) now
  expose equivalent `try_` outcome paths, sharing one protected cleanup/health gate.
  A closed native observation-stop facade composes source, annotation, coherent-set
  and history control without ordinary read admission or retirement. It preserves
  original scopes, independent retained copies and protected cleanup under faults.
  Exact passive-native typed/dynamic stop envelopes canonicalize closed constants
  without allocation and bind the original target before native control. They are
  not a wire parser or an identity codec.
- Owned search occurrences drive retained selection commands with explicit
  replace-all-selections, replace-primary or add-primary modes, direction and
  affinity. Normalization cannot change the original replacement extent. Final
  query/view checks and shared replay prevent implicit requery or stale publication;
  selection alone needs no source-write permission or history entry.
- Exact-snapshot source-operation scopes independently restrict replacement
  cells and insertion gaps. Source and compound preparations retain and recheck
  bounded restrictions through ordinary atomic publication; scoped source/compound jobs
  share the replay ledger. These caller restrictions grant no authority, create
  no standing document policy and do not implicitly constrain future undo/redo.
- Closed declared-read footprints: exact source, annotation-set, live body and
  authored-target versions constrain compound preparation, publication and replay.
  A separate v3 profile adds configuration domain/state generations with all three
  interpretation epochs, composing with v2 workspace and v1 source/annotation reads.
  Domain reads preserve unaffected values; metadata-sensitive state reads invalidate
  more broadly. Invalid versions, stale/foreign subjects and duplicates refuse.
  Read-only participants gain no writes or history links. Pure advice uses the same
  predicate checks without reserving future validity; scopes and reads compose in
  the existing publisher. Recipient observation grants remain a separate contract.
- Optional bounded logical workspaces: non-reused identities, shared document
  membership, exact generations and independent container retirement. Both document
  disposal paths remove actual membership edges atomically. A separate v2 read
  profile adds exact membership-generation dependencies without extending v1's
  four-predicate enum. No ownership transfer, paths, focus or settings association.
- Retained view/workspace create/retire commands share the one-shot ledger and
  direct controls' native staging. Execution rechecks current profile, capacity
  and exact retirement context, preserving unrelated source work. Typed historical
  identity receipts survive later target retirement; stop/forget never reopen work.
  Recipient release and dynamic lifecycle adapters remain separate qualifications.
- Intrinsic document serialization metadata now has a separate positive-safe
  generation and closed strict-UTF-8/BOM profile. Trusted direct and retained
  mutation recheck exact target authority, context, bounded declared/mandatory
  reads and current format before publishing. Source text/revisions, journals and
  undo stay independent. Save captures retain their original format and generation;
  dispatch/baseline publication reject capture drift. Unencodable intent refuses
  before save issuance or I/O. Optional bounded preparation/validation/publication
  signals contain no document content. Separately funded recipient-bound format
  bases now retain only context/document/generation. Scoped commands privately
  derive native conditions and share the same invocation slot/publisher; complete
  preflight precedes absorbing loss observation. Stop retains charge, protected
  drain-based retirement works after faults and lifetime issuance never refills.
  Public codecs and real policy truth remain open. This is neither a settings-
  registry key nor a filesystem write procedure.
- Bounded semantic-settings compilation: six closed types, explicit registered
  defaults/scopes, replace or declared-field map composition, finite restrictions,
  precise origins and masked-conflict rejection. Nested inputs copy into flat
  internal arenas. Initial installation, trusted reads and bounded set/remove/reset
  activation are executable. Original contributor footprints reject stale changes;
  target handles bind owner-local registration identities rather than list positions.
  Current and retained roots resolve those identities independently; value rebuilds
  preserve registrations without creating uncharged interpretation snapshots.
  Disjoint changes merge into the current root. Metadata and effective domain
  versions advance separately; identity needs no publication counter. Settings jobs
  share the one-shot invocation ledger, exact signed-zero-aware retry intent and
  typed historical outcomes. Stop/forget cannot reopen execution; interrupted
  publication fences Unknown. A separate v2 settings route conjuncts bounded
  caller read dependencies with mandatory contributor checks through the same
  staging/publisher. Neither stale no-ops nor replay can refresh old assumptions.
  Native full declaration/association replacement now uses that publisher and
  ledger, preserving every authored layer while revalidating types, scopes,
  restrictions and target resolution. Facet epochs and effective-domain maps
  advance independently; removed target registrations cannot revive on restore.
  Additional reads, old interpretation leases and preparation-aware retirement
  remain intact. This is privileged owner behavior, not recipient registry-role
  authorization or dynamic F16 codecs. Bounded authored-layer creation/removal
  now shares the publisher and ledger, with complete reassociation, nonrefundable
  contiguous incarnation issuance and compact copying of surviving contributions.
  Ordinary value plans resolve original layers by identity after unrelated removal;
  storage compaction cannot retarget their writes or cause index-only staleness.
  Native created-value memory limits are logical bytes, not a chosen wire encoding.
  Recipient-bound conditional layer bases now capture all seven execution-context
  labels and the actual contributor footprint without releasing native versions or
  values. Explicit recognized loss closes them; foreign/malformed/unknown facts and
  transient readiness do not establish loss. Interpretations and bases share funded
  capacity/issuance but have distinct opaque types and checked pool-row kinds.
  Stop retains its charge; protected drain-based retirement works after owner fault.
  These facts must come from the trusted embedding, not a requester. Scoped mutation
  now lowers exact unique/used basis vectors through the same native normalizer and
  publisher, with whole-proposal preflight before stateful loss observation. Its
  service port requires fresh host facts; the generic runner cannot bypass them.
  Captured mandatory/caller reads, complete plan/parent wrapper bounds and sealed
  original intent remain fixed across replay. Changed mandatory counts refuse before
  copying fresh requirements. Scoped success exposes only changed/
  unchanged, not native epochs. Recipient result release, public codecs and actual
  host-policy authentication remain open; a basis alone grants no mutation.
  A distinct declaration-role basis now shares the funded settings pool without
  retaining a source/layer snapshot or gaining ordinary layer authority. Scoped
  definition activation privately lowers its exact facet condition into the native
  declaration kernel; only changed facets require their current independent grants.
  Whole-proposal/read/wrapper preflight precedes stateful loss observation. Source
  and pre-preparation layer progress commute with the basis; native full preparation
  dependencies remain decisive afterward. Direct/retained scoped execution shares
  the ledger/publisher, historical replay and Unknown fence, with redacted plans,
  optional bounded signals and protected fault cleanup. Public recipient products
  remain open.
  A separate conditional layer-lifecycle basis now captures the configuration/
  incarnation pair in the same funded pool. It exposes availability only; capture
  grants no create/remove/reassociation authority. Zero/exhausted issuer state is
  conditionable, source progress commutes, and configuration progress invalidates
  the condition. Scoped lifecycle lowering/service now privately derives create
  identities and removal generations, validates complete target associations and
  fresh full-identity grants, and shares the native publisher and retained ledger.
  Original basis/read/reference intent is sealed across replay; complete candidate
  and wrapper preflight precedes stateful loss observation. Lifecycle requests may
  also select existing layers through a bounded vector of ordinary opaque write
  bases. Every basis must be used and select a distinct actual layer; service
  rechecks original dependencies and fresh independent basis facts. Equivalent
  replacement handles or reference forms cannot substitute on replay. This uses
  the same admission, plan, runner and publisher, not another execution lane.
  An optional prepared-plan result port now reserves bounded parent capacity before
  admission and releases ordered opaque creation references only after common
  terminal success and current full-envelope permission checks. References confer
  no write authority: fresh basis capture uses current actual-layer checks. Replay
  reuses references; protected forget invalidates them and releases the hold without
  retiring independent bases or rearming work. Public codecs remain open. Its distinct
  aggregate owner-root prerequisite is now implemented under the native profile
  described below; component ceilings remain independent.
  The selected native logical-root profile now measures actual semantic state,
  including history and persistence baselines, separately from retained work.
  Its allocation-free trusted-owner gauge has bounded content-free diagnostics.
  Shared settings preparation now requires an explicit embedding-selected ceiling,
  checks its actual whole candidate before retaining a plan, and remeasures current
  unrelated state at publication. Scoped preflight runs this before basis-loss
  observation. The compound publisher now checks actual staged source/annotation/
  view/history/frontier contributions together before installation, and annotation
  preparation admits spent identity slots before issuance. Source birth, initial
  source/binding/baseline loading, standalone logical view/workspace membership/
  selection changes and provider/encoder/restorer registration now check actual
  candidate capacity too, including retained revalidation. Source/written/loaded
  baseline replacement, annotation binding and initial restoration now check their
  complete candidates, while competing guards preserve pending ordinary-baseline
  root headroom (maximum per document, additive across documents). Current-root
  and future-demand gauges remain trusted-owner diagnostics, not authority or a
  physical-memory/refinement proof. Full public facade work remains open. See
  [native root accounting](docs/native-root-accounting.md).
  Immutable interpretation leases now use a prepaid
  bounded pool with non-reused identities and preparation-aware retirement.
  Its reservation checks actual owner-local retention headroom; it is not a global
  physical-memory budget or a migration of existing source/history accounting.
  The native configured-line consumer is executable; other consumers and complete
  recipient/discovery/configuration-read contracts remain open.
- Pure bounded attachment candidates: explicit position/content point profiles,
  boundary-exclusive content ranges, affected/detached evidence and coherent
  source/placement preparation. Equal source bytes do not erase target-origin
  loss. This is the annotation mapping prerequisite, not authored record storage,
  complete companion acquisition or annotation command/history acceptance.
- Opt-in authored comments: non-reused identities, independent body/target
  versions and write policy, reattachment and retained deletion. Source edits map
  the actual current registry under a private set dependency. Source/annotation
  scopes share atomic publication, linked field/membership history, pins and redo
  invalidation. Compound invocations retain one original create identity. Missing
  historical target-mapping evidence refuses; recipient release, annotation
  persistence and final retirement remain open.
- Bounded coherent annotation captures: exact source/set frame, immutable complete
  live records and canonical source text. Snapshot clones/artifacts retain their
  actual capture against the configured capacity. Retained-absent rows are not
  storage rows; detached bodies and historical authored selectors remain intact.
- Shared source/annotation artifact retention and explicit annotation exports:
  original encoder context and immutable bytes, common destination FIFO and
  independent application/control evidence. Export never activates an ordinary
  binding or publishes a source baseline. Byte-delivery stop does not cancel
  admitted effects or refund retained bytes; task/delivery holds prevent early
  retirement. These exports promise no later baseline installation.
- Initial ordinary annotation associations: one document owner and typed resource
  ownership, exact-basis binding plans and explicit current known-distinct host
  comparison. Source save admission, dispatch and baseline publication recheck the
  actual pair independently. The first comparison profile supports one exact
  provider registration; cross-provider comparison refuses, not assumed distinct.
  Association does not decode comments, install a saved baseline or enable rebind.
- Explicit historical annotation baselines: an export may capture an original
  ordinary association; later binding cannot retrofit it. Separate adoption checks
  current pairing, original context, acknowledgment and newest eligible launch
  serial. Exact-byte current-output comparison spends no persistent artifact IDs.
  Baselines and still-eligible producers pin their actual tasks/artifacts. Neither
  historical equality nor a receipt establishes current external freshness.
- Native ordinary annotation save: atomic capture/task/capacity admission from a
  current transient codec result, immutable pinned bytes and separate prepaid
  baseline installation. Each pending save reserves a worst-case native descriptor
  budget and one publication-counter unit; other publications preserve those units.
  Admission stop and safe capacity cleanup do not erase evidence or recycle IDs.
  Before-codec and reference wire-envelope funding remain separate open contracts.
- Owned annotation reads: bounded start/chunk/end/failure transcripts, exact
  duplicate handling, two conflict witnesses and independent quiescence. Read
  registration is distinct from the bound writer; launches share execution
  headroom. Immutable captures retain their actual read and can be strictly
  decoded without native restoration, identity activation or baseline adoption.
- Initial annotation hydration: an independently provisioned restorer stages
  approved identities and real managed source witnesses from an owned read.
  Publication requires a fresh lifetime inventory, preserves source/history and
  installs record floors atomically. Consumed reads retain provenance across stop
  and late contradiction. Hydration is not replacement or merge.
- Explicit loaded annotation baselines: a bounded same-owner read/writer attachment
  adopts the actual consumed input under its original approved encoding context.
  Loaded facts are not write acknowledgments. Exact raw-byte comparison preserves
  noncanonical JSON differences; queued saves retain their original loaded producer
  even after a written baseline replaces it.
- Checkpointed read discharge: a current acknowledged native capture, complete
  live consumer scan and actual delivery drain allow atomic retirement of the
  obsolete read/provenance without changing authored state or writer evidence.
  Explicit attachment detach releases both endpoint charges; reattachment has a
  fresh incarnation. Rebind and contested recovery remain open.
- Retained disposal and key release: exact-root preparations end live editing
  while preserving source, comments, history, bindings and effect evidence.
  Independent source/annotation key release checks actual effect disposition and
  archives original bindings/baselines without releasing their producers. Current
  queries, edits, captures and save launches reject disposed owners; historical
  artifacts and late receipts remain available.
- Explicit disposed provenance release: no active binding or retained history may
  be silently overridden. Obsolete consumed reads can then retire without a save
  checkpoint; claimed ordinary writes atomically retire their own artifact and
  claim after separate safety/drain checks. Either retirement order works. Cold
  archives, authored records and spent IDs remain; this is not storage erasure.
- Optional content-free synchronous service timing covers native document birth,
  immediate/retained disposal, lifecycle queries and disposed binding/provenance
  release. Off/disabled/full capture preserves behavior; integrity faults report
  unknown rather than ordinary refusal. This is not a reclamation or latency SLO.
- Retained disposal, source/annotation binding transfer and provenance publication
  now fence the owner across multi-root installation. Unexpected unwinding emits
  an optional content-free Unknown attempt and propagates the original panic;
  partial or fully installed-but-unacknowledged state is not safely retryable.
  Known pre-install refusal remains non-faulting, and successful publication clears
  its fence. This adds no rollback, task refund, quiescence or lifecycle receipt.
- Native view attachment/detachment and workspace membership now also fence their
  row/counter/publication installation against unexpected unwind. The original
  panic propagates with optional content-free Unknown service timing; even a
  completed-but-unacknowledged installation is not a safe retry. Known refusal and
  validated no-op remain non-faulting. Normal native service timing is optional
  and bounded. Native owned attach/exact-detach/membership ports now add original
  request/context/target binding, real shared funding, fresh applicable service
  rights and publisher-captured historical results. Protected stop and once-only
  hold retirement preserve Unknown work and unrelated obligations. Explicit native
  F06 V5 now composes those historical results under original observer ownership,
  lifecycle ceiling and common/native release checks, without upgrading V1–V4.
  Complete public lifecycle ingress remains open.
- The native retained-disposal invocation now binds one immutable exact-root plan,
  complete context and disclosure ceiling to its original issued ticket, with a
  real shared retention reservation. Fresh service checks precede publication;
  the publisher stages the historical result before installation. Only its matching
  committed outcome releases the typed disposal result. Protected capture retirement
  does not settle Unknown work or refund issuance. Other lifecycle operations and
  their F06 lifecycle receipt assembly remain open; disposal composes in the
  explicit V2 profile described below.
- Original-ticket native invocations now also cover disposed source-binding,
  annotation-binding and provenance release. They recheck the existing writer and
  targeted lifecycle guards, stage applied/unchanged history inside the actual
  native decision, and expose typed historical results under captured/current
  disclosure checks. Late evidence does not retry a spent refusal. Protected
  capture retirement preserves Unknown execution and unrelated holds. These three
  results compose through explicit F06 V3; the exhaustive V1/V2 enums stay unchanged.
- Unit and external-consumer tests for optional diagnostic capture.
- A bounded, preallocated recorder with no record-time callbacks, I/O, clocks,
  formatting or storage growth. Capacity is host-selected; allocation failure is
  explicit. Overflow drops the newest diagnostic record and counts loss.
- A test-only scripted substrate seam for arbitrary bytes, read failure and
  completed/failed/unknown writes. This fixture is not a filesystem adapter or
  the production resource/evidence contract.

No UI, integration workflow, filesystem mechanism, exporter or runtime is required
by the engine crate. Unsafe Rust is forbidden. No service starts on import.
Engine-managed strict source decoding and capture/comparison encoding now emit
optional bounded content-free codec signals. They report actual conversion work,
not publication or persistence; metadata changes, pinned dispatch and earlier
admission refusals do not fabricate codec activity. Standalone pure value codecs
remain unobserved unless called through a managed edge.

## First local-filesystem adapter

`vellum-local-fs` implements the selected staged-replacement procedure on macOS.
The real-file tests exercise open → atomic source edit → query → pinned save,
including reversible BOM/newlines, new-inode continuity and repeat saves.
The adapter anchors a directory descriptor, exclusively creates a same-directory
stage, checks the original pinned condition again, writes the capture, preserves
supported mode/ownership and verifies bounded extended-attribute equality, then
renames. Requested synchronization checks both the file and directory.

Symlinks, nonregular/remote files, hardlinks, ACLs, special modes/flags, ownership
that cannot be preserved and metadata outside the declared budget refuse. No
in-place fallback is provided. Inode identity and creation/access/modification
timestamps are not preserved by this replacement profile. Checks are best-effort:
external writers can race the final check/rename. This is not an atomic conditional
write, hostile-directory sandbox, universal durability guarantee or other-platform
qualification. Raw paths/OS errors do not enter diagnostic records.

Unsafe code remains forbidden in the engine. The adapter denies it except for one
documented native seam for macOS ACL absence and xattr copy; preservation is
checked before replacement. Those platform truth assumptions are not Rust proofs.

## Reference annotation JSON codec

`vellum-annotation-json` implements the adopted v1 interchange shape outside the
semantic engine. Native capture export requires complete injective host document,
record and source aliases, including actual historical source text when needed.
The codec emits immutable bounded UTF-8 JSON and validates those exact bytes with
its strict decoder before delivery. Both point profiles/affinities, scalar bodies,
decimal counters and authored/located contexts are preserved independently.

Duplicate decoded keys, unknown fields/versions, invalid UTF-8/escapes/surrogates,
JSON BOM, noncanonical decimal strings, ambiguous identities, invalid current
scalar boundaries and exceeded budgets refuse. It uses the established parser,
not a new JSON lexer. Node/depth/string/body/output limits are logical bounds;
complete allocator/CPU/retention accounting is not claimed. Codec artifacts retain
their native snapshot. Engine-admitted artifacts share live/lifetime slots and a
held-byte budget with source saves; transient codec results and caller copies
remain host-owned accounting obligations.
Decoded aliases and versions are data, never live edit tokens or continuity proof.

The codec can transfer its exact output/capture to a privileged encoder port,
then to the engine's retained artifact ledger and explicit export queue. A native
integration test writes the JSON through the same local-filesystem adapter and
separate receipts used for source saves, without touching source bytes or saved
baselines. The port attests producer validation; it is not a sandbox or recipient
authorization mechanism. Generalized load/reload, loaded-baseline adoption, prepaid
before-codec capture reservations and wire-envelope funding remain open.
Encoding is not a persistence acknowledgment.
The filesystem adapter also emits owned captured-read receipts. `decode_read`
retains the original capture alongside a read-only decoded value; file aliases
and counters remain data. `into_restoration` requires explicit document/record/
source approvals and revalidates exact input under its declared finite profile.
The engine checks actual read provenance and managed witnesses before fresh
hydration. Imported keys/incarnations survive re-export; changing them is refused.
Missing historical witnesses refuse intact. Neither decoding nor hydration adopts
a loaded baseline or claims an atomic-resource snapshot.
Explicit annotation baseline adoption/comparison is implemented independently,
including a native codec-to-filesystem scenario. A baseline budget bounds its
fixed native descriptor and referenced byte payloads. Ordinary native admission
reserves its worst token width, private bookkeeping and publication credit before
issuance; this is not the reference JSON capacity envelope or global allocator
accounting. Transient encoding remains host-funded and can be refused at later
admission without issuing an engine artifact/task.
The UTF-8 source-file codec also now refuses absent-BOM plus literal leading
U+FEFF before save issuance; the edit remains valid, while canonical text supplied
to annotation encoding is never passed through a file/BOM decoder.

## Instrumentation seam

`instrumentation::Recorder` holds fixed-field `Observation` values identifying a
provisional boundary, diagnostic disposition and optional service duration.
Measurements may be host-supplied or explicitly enabled at native boundaries;
missing measurements remain absent, not zero latency.
Fields cannot carry document content, paths, arbitrary errors, credentials or
resource handles. These types are provisional diagnostics, not command vocabulary.
No stable signal names, wire format or backend dependency are selected yet.

Capture may occur within an execution boundary; drain and export must occur
outside authoritative publication. Draining transfers owned records without
calling user code. Export policy, recipient authorization and retention belong
to the host adapter; do not export merely because a record was captured. The
recorder is single-owner, not a concurrent queue or global subscriber.

An adapter can derive counts and service-time distributions from drained records,
and observe queue depth/capacity/loss. These are sampled diagnostics, never the
authoritative operation outcome. Loss can bias derived rates and distributions.
Do not turn completion into a durability claim or unknown into success. Disabled
or saturated diagnostics must not control admission, publication or settlement.

This is the capture/drain foundation for MELT, not complete MELT/RED/USE or golden
signal coverage. Correlated spans, stable schemas, separate queue/service/effect
timing, clock domains, exporters and measured overhead will be added at actual
execution sites. No SLO, constant-time performance or physical allocation bound
is claimed by these tests.

`Recorder::with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1)` opts into
local elapsed timing for supported read-facade attempts, native protected stop,
its normalization phase and all seven native invocation service entry points
(the source-search compatibility wrapper delegates once). It is off by default.
Invocation service and terminal replay use distinct markers; successful service
is not a claim that the job committed. Unwinding produces no completed sample.
Completed read timing includes
validation, native work and protected cleanup; admission refusals are measured at
admission. Native stop and normalization are separate episodes, not a whole wire
round trip. Missing/regressed/panicking clock reads yield no duration and cannot
authorize, fault or settle engine work. No clock is sampled for this profile when
capture is disabled or already full at entry; later capture loss uses the existing
drop accounting. `record` and drain stay clock-free. Timings are operational metadata,
not recipient-safe exports, CPU time, deadlines, SLOs or external clock attestation.

Separately, the adopted `engine-measurement/1` value catalog has bounded native
collectors for diagnostic stages, invocation admission/disposition and transaction
publication. Opt-in `configure_release_measurements_v1` now counts whole attempts
through the nine native/decoded read-adapter paths after protected cleanup and the
delivery health fence. Compatibility wrappers count once. The six-point privileged
snapshot is prospective and content-free; saturation becomes explicitly inexact,
stop yields off, and protected retirement releases only its own parent reservation.
These are not all-engine release totals, recipient-authorized telemetry or measured
latency. Enabling a collector requires its explicitly funded shared-parent headroom.

## Next executable slice

Native resource-catalog metadata now supports bounded atomic read/write batches:
explicit absent/present expectations (including present generation zero), immutable
owned preparation, fresh final reconstruction, one shared non-reused registration
clock, and typed trusted-owner results. Changed batches publish once; validated
no-ops retain the root and clock. Root accounting/currentness and optional bounded
stage diagnostics/timing cover the actual implementation. This does not replace
provider ports, execute providers, activate document bindings, grant advertised
capabilities or expose a recipient-safe catalog. Registration metadata also runs
through the existing invocation ledger: complete original request/context/profile
replay, fresh service-time preparation, protected stop and sealed typed retained
results with captured/current permission intersection. Optional service/result
signals and existing admission/disposition counters observe the actual path.
Scoped conditional registration bases now retain exact present rows or an absent
slot predicate under full recipient/policy scope, fresh independent observation
permissions, finite lifetime issuance and shared-parent retention. Ordinary replies
contain only opaque handles/condition; separate host-only resolution requires an
additional grant. Protected retirement cannot refund issuance or reopen the issuer.
The scoped native adapter now uses the same registration job: exact basis-reference
bindings and index permutations lower to the existing batch, re-resolve at service
and final staging, and release only original index/present-or-absent receipts under
captured/current permission intersection. Native/scoped routes cannot reinterpret
each other's job. Historical receipt delivery does not re-test staled input rows.
Passive native scoped envelopes now normalize their exact closed request/configuration
schema into the same typed admission path. Original invocation roles are borrowed,
actual basis handles stay separate, and normalized data grants no authority. Bounded
inert/typed workspace and optional normalization/decoded-admission timing cover the
actual path. This is already-decoded native data, not a JSON/wire-token codec.
Native registration now also normalizes its exact passive envelope and full
provider/resource/expectation/configuration batch into the same typed job.
It shares the closed configuration parser with scoped registration; normalization
grants no authority and fresh decoded submission still checks the original native
profile/context. Duplicate slots, unsafe numeric identities/stamps, wrong-table
configurations and extra/shadow fields refuse before typed output allocation.
Native/scoped serialization now have bounded passive Rust envelopes as well.
Original invocation/document/dependency roles remain borrowed; inert data cannot
mint them. Strict Boolean/format/generation validation and joint inert/typed
preflight precede output allocation. Decoded submission rechecks profiles and
delegates to the existing authority, basis, service and replay guards. Optional
content-free normalization/admission timing observes these actual paths. This
does not introduce a wire codec; discovery integration is described below.
Scoped semantic settings now has a passive six-field adapter with closed indexed
set/remove/reset changes and lossless inert value conversion. Basis handles remain
separate; decoded submission uses the original conditional settings job. Explicit
normalization workspace limits do not consume native set-value node/depth budgets.
Optional content-free timed normalization/admission observations cover the path.
Scoped layer lifecycle now has its bounded passive eight-field adapter as well:
exact creation/contribution and association-reference schemas feed the original
conditional job. The original lifecycle basis stays separate; labels/indices grant
no native identity or write authority. Optional normalization/admission timing
does not change allocation, replay or current service permission checks.
Scoped definition activation now has a bounded passive six-field adapter, including
iterative closed type/value conversion. The original basis remains separate and
current facet grants remain decisive. Normalization is not schema compatibility,
write authority, a wire codec or discovery /2 enablement.
Native ordinary settings now has a passive envelope and a distinct context-bearing
port into the same ledger/publisher. It retains the complete caller context,
native conditions and caller/required reads, rechecks current full-identity grants
at service, and refuses generic-runner bypass or raw/scoped intent substitution.
Original preparation identity is sealed without retaining its old settings root
after settlement. Existing trusted-owner APIs remain explicitly privileged.
Native definition activation now shares that sealed context/dependency carrier
and publisher. Its exact epoch-conditioned envelope reuses the scoped declaration
codec; service requires current grants only for facets that actually change.
Values/definition/raw/scoped intents cannot reinterpret a ticket. Optional bounded
normalization/admission diagnostics remain content-free.
Native layer lifecycle now completes the ten specified F16 envelope paths. It
shares the sealed carrier and contribution/target codecs, rechecks complete
affected-identity and association grants, and supports the existing separately
funded created-reference result profile. This is not full facade/discovery closure.
That result owner now has optional content-free service timing across all three
admission routes, staging/publication, filtered release, protected forget and
fresh basis capture. Interrupted work emits no invented completion span.
All ten specified F16 inner decoders now use their own static argument declarations
for exact header validation and expose the same descriptions on normalized
products. A shared allocation-free field guard rejects shadow/unknown headers;
opaque roles, nested semantics and current authority retain their existing owners.
A bounded native F16 descriptor source now derives ten outer-schema branches
from those declarations and supplies both context variants' five metadata facets.
It has actual optional success/refusal timing and grants no capability or wire
codec merely by inspection.
Discovery ingestion now bounds the complete metadata capture before finite input
recognition. Its closed outer-schema grammar admits no references, extensions or
evaluation; /1 still accepts only legacy input descriptions.
Actual recognition success/refusal has optional content-free service timing.
The explicit `owned-contract-discovery/2` profile now uses that same owner and
the fixed F13/F14 prefix plus F16 context row. Both profiles share one prepaid
capacity/selector/lifetime-issuance domain; changing profiles does not reset it.
Generic lookalikes cannot replace built-in cuts, wrong profiles cannot consume
or poison another context, and schema release remains captured/current filtered.
This closes the selected /2 source/owner integration, not all-family discovery,
nested/result/wire codecs or measured performance acceptance.
The first /3 prerequisite is now an actual passive `selection-edit/1` literal
adapter, not a descriptor-only claim. It checks ten exact fields, borrowed native
roles and the original separately supplied view context, then makes one bounded
literal copy. Decoded submission reuses the existing ledger/preparation/publisher,
including replay, current service checks and mapped/caret-only effects. Actual
normalization/admission timing is optional and content-free. The explicit
`line-edit/1` adapter now similarly validates closed indent/outdent/newline
settings and delegates to the existing line job. Prefix requests copy one bounded
unit; newline requests allocate no unit. Both adapters share the original native
view-role binding guard. The configured-line envelope now binds an actual native
settings target plus two closed action-key roles, copying only bounded key names.
It resolves no settings or interpretation at normalization; decoded admission
uses the existing configured job and protected interpretation lifetime. The
boundary ordinary-edit envelope now similarly binds original native evidence,
validates closed logical units/directions and positive counts, and delegates to
the existing Unicode deletion job. Normalization copies no table or payload.
All four /3 envelopes now back the explicit `owned-contract-discovery/3` source.
It preserves the /2 three-row prefix and appends only ordinary-edit, with an
exact four-form inert schema, private origin, shared limits/issuance, and current
disclosure checks. Generic or differently shaped schemas cannot impersonate that
product. Source/recognition timing remains optional and content-free; this is not
later-profile, all-family/result-codec or physical-performance qualification.
The four native F12 selection/navigation envelopes now have bounded Rust argument
guards and actual handoff to the existing selection and Unicode movement jobs.
They preserve raw selections/primary, endpoint budgets, original opaque search
matches and boundary evidence, including explicit direction/affinity/mode/extend.
No keymap, visual motion, source write, query resampling or name resolution is
introduced. Optional content-free timing covers normalization and admission.
This does not enable later F12 discovery or supply the full recipient-authorized
facade, prepaid ingress/result/event codecs or performance qualification.
The four `annotation-authoring/1` forms now also have a lossless passive guard
driven by the native mutation owner's declarations. It preserves original opaque
roles/dependencies, bounded scalar bodies/profile labels, complete native targets
and full-width typed versions, with optional content-free normalization timing.
The separate scoped native execution path now retains the original context,
profile/limits, dependencies, allocation and plan under actual shared funding.
Fresh service gates precede the common publisher; protected retirement discharges
its own hold without reusing identities or converting faults into clean outcomes.
Optional content-free admission/service/retirement timings cover these paths.
Private decision receipts are now staged before root installation, preserving
historical membership/versions without body/target snapshots or later-row lookup;
known refusal/stop, forgetting and protected retirement share their owning lifecycle.
Standalone native annotation receipts now have admission-bound filtered release.
Create-result authority covers only this invocation's original allocation; current
grants meet the captured field ceiling, without another host handshake. Result
frames use original funding; replay cannot add rights or reexecute.
Trusted native compound annotation results now bind each create grant to its
original component and allocation, reserve shared funding before copying, and
stage the complete historical vector inside the existing publisher. Captured/
current grants cannot reveal siblings or widen original field ceilings. Strict
retention/drain forgetting and protected once-only hold retirement preserve spent
issuance and Unknown. Explicit native F06 V4 now composes these atomic results
under original observer ownership, separately captured atomic capability and
common/native release checks. Public mixed-facade assembly and discovery
enablement remain open. Requested profile and
dependency meaning are not discarded to call a legacy API.
Original invocation observation policies now have a native ownership foundation:
pre-admission immutable context/policy capture, exact shared-ledger funding,
idempotent binding and separately protected retirement. Size preflight precedes
context comparisons and copying. Live private capture users block discharge;
cleanup does not settle or rerun the producer. Optional content-free timing covers
binding, usage and retirement. That role owns policy metadata only; observer
ownership and delivery have separate funding and authorization obligations.
Native F06 ownership now also exposes protected phase-only status through an
explicitly retained host context. It charges the complete declared local envelope,
captures original policy provenance and validates actual scoped-producer context
where present. Pending/terminal use two independent preallocated control identities;
ordinary issuance may be zero. Unknown facts refuse without poisoning the context,
explicit authority loss closes it, and protected close/retirement never runs or
settles the producer. Optional content-free timing covers these actual endpoints.
That protected facet alone is not ordinary outcome/receipt delivery, wait support,
a recipient acquisition verb or a stable SDK. Host registry/drain and F06 assembly remain
open; dropping an unretired native handle does not refund its shared-parent hold.
Ordinary native F06 status and retained-outcome reads now project fixed-size local
command facts under the original/captured/current disclosure meet. They do not
clone raw outcomes, traverse dynamic payloads or disclose errors, participants or
correlation. Native result/frame bounds are checked at context admission. Ordinary
references advance only when the permitted projection changes, never for unrelated
publication; repeated reads reuse them, and ordinary exhaustion leaves protected
control available. Optional actual query timing preserves off/full behavior.
An explicitly selected V2 receipt profile now composes the native retained-disposal
result with F06 observation. V1's exhaustive six-family result enum is unchanged;
V2 wraps those existing products and adds only retained disposal. Context admission
captures the profile and family ceiling with complete native reply bounds; queries
cannot upgrade a V1 context. Native and common disclosure checks remain conjunctive.
The explicit V3 receipt profile wraps prior products and adds the three retained
release operations under their original/native result gates. Its selected version,
captured capability and complete wrapper bounds are fixed at observer admission.
Prior versions cannot query a V3 context or acquire this support by fallback.
V4 wraps prior products and adds original-index compound annotation results;
prior contexts cannot upgrade, and receipt-only faults preserve protected status
and cleanup. Remaining document-lifecycle receipts, public decoding/discovery and complete
registry/shutdown reconciliation remain open. This does not freeze an SDK or wire
format, certify the model or establish a measured latency guarantee.
Native F06 waits now use prepaid rows, owned ordinary references and finite logical
budgets. Polling reads actual state without running work or consuming time; valid
epoch/tick advances strictly decrease rank, while invalid clocks end unavailable.
Delivery, denial, expiry/stop and context invalidation end eligibility, not the
producer. Ended rows retain capacity until protected discharge, and context
retirement cannot bypass them. Admission/service/discharge timing and trusted
retained/waiting gauges are optional and content-free. Poll/tick lookup uses an
opaque slot rather than a waiter scan; admission, all-waiter closure and inventory
remain bounded scans. Clock/drain truth stays with the host; no timer/runtime,
wall-clock deadline, scheduling fairness or measured latency guarantee is implied.
Native F06 receipt assembly now connects annotation, reversal, relation, host
registration, scoped registration and created-settings results. Captured family
ceilings, separate current receipt permission and original/captured/current
publication rights precede each family's native release checks. Admission validates
the combined wrapper and original native payload bounds before copying; created
references retain borrowed delivery but still require the complete frame bound.
Receipt reads neither execute commands nor issue ordinary progress references.
Native receipt integrity faults disable that receipt facility without relabeling
producer outcomes or refunding holds. Optional actual admission/release timings
are inclusive of nested native phases and contain no payload/context labels.
Atomic-annotation and document-lifecycle selectors remain explicitly unsupported;
host-registration metadata remains trusted-host delivery, not a recipient codec.
The passive `literal-search/1` argument guard now preserves all three logical
query forms, their original opaque read-context role, logical reference, matching
profile, generation, ordered scopes and operation-specific navigation. Exact
field sets, safe naturals and explicit byte/range limits precede copying. It does
not establish reference-to-native-source correspondence or silently convert to
the older `SearchRequest`; `/4` discovery remains disabled pending native binding
and exact reply products. Actual guard timing is optional and payload-free.
The native retained-search owner also times actual configuration, evaluation,
release, occurrence/navigation and protected retirement when opted in. Partial,
refused and integrity-fault paths retain their meanings; unavailable diagnostics
do not select outcomes. These service spans are not a search latency SLO.
Managed-search capacity/usage gauges no longer scan lifetime result tombstones;
checked row transitions maintain exact counts, while cold pool retirement still
reconciles actual retained rows before discharging funding.
The four native view/workspace create/retire envelopes also validate their exact
five/seven fields and original opaque roles before delegating to existing relation
jobs. Original native expectations are preserved without target lookup, generation
refresh, UI actions or recipient-grant inference. The envelope adapter itself is
not a complete recipient-facing F17 facade.
Optional `submit_relation_with_result_v1` separately captures a ticket-bound
disclosure ceiling and complete context; `retained_relation_result_v1` intersects
it with current grants and verifies the original native receipt before returning
historical opaque targets. Legacy jobs cannot opt in later. Forgetting drops the
receipt without reopening execution. This scopes disclosure only, not execution
authorization or the still-open F06 observation-context composition.
Undo/redo publication now independently checks complete original participant and
inverse meaning, then exact staged frontier movement before any root installation.
Malformed internal plans fault without a partial reversal; normal stale/permission
refusals keep their existing semantics. These bounded checks have optional timing.
Complete F06 receipt composition remains separate unfinished work.
Native F14 result opt-in now pre-funds and retains minimal applied/not-published
decisions. Applied uses the executing invocation as its causal anchor; optional
historical group/source-frontier/annotation-frontier disclosure is the meet of
captured and current exact grants. Publication scope/epoch facts remain trusted
host premises, not a complete F06 observation owner or a wire identity encoding.
`PERF_EXAMPLE=search_responsiveness` uses the same performance targets to exercise
Unicode prose, captured-scope code and 1.1-MiB mixed content through that native
owner, including checked release/navigation and protected cleanup. It reports
tails under off/available/timed/saturated capture and enforces an explicit optional
total-p99 budget without treating incomplete or partial work as a pass.
The first [responsiveness gate](docs/performance.md) runs actual trusted-owner
edit/commit/query workloads with capture off, enabled, timed and saturated. Build
once with `make perf-build`, rerun with `make perf-run`, or enforce an explicit
caller-supplied total p99 budget with `make check-perf PERF_MAX_P99_NS=...`.
This is not whole-engine performance acceptance or UI/provider latency coverage.
An opt-in V2 invocation measurement profile now retains bounded pending ages and
admission-to-local-disposition durations independently of diagnostic capture.
Replay does not restart timing; unknown clocks/interrupted execution remain
explicit. This native owner aggregate is not a queue, scheduler or recipient grant.
`make perf-build` and `make perf-run` also accept
`PERF_EXAMPLE=invocation_responsiveness` to measure actual invocation and observer
cost, including a snapshot after every operation and verified job/hold cleanup.
Catalog-backed source reads now retain host-established, engine-issued opaque
bindings. Full-row currentness gates admission/dispatch/activation, without
invalidating unrelated rows or erasing original late evidence and owned recovery.
Optional bounded binding/currentness diagnostics observe the actual path. Native
capabilities and opaque identities are not inferred from metadata. Annotation reads
now use the same closed capture with an independent annotation grant: currentness
gates admission, dispatch, new capture and restoration. Late original evidence,
held captures, completed restoration outcomes and owned recovery survive withdrawal;
capture and operation budgets include the complete registration. Catalog-backed
ordinary source/annotation writes now accept an engine-issued binding to the
original opaque native writer. Full-row currentness gates admission, dispatch and
new baseline installation; original receipts and installed baselines survive
withdrawal. The existing artifact hold charges the complete capture, with no
second task registry. Owned source/annotation jobs now retain that binding in the
same parent ledger; matching recovery neither relaunches nor needs a current row.
Bound/unbound or changed binding intents cannot share an origin. Catalog own-chain
saves require declared continuity/causal/nonreuse support and actual native support,
using the existing causal writer and credits. Source copy/Save-As (including typed
after-results) and annotation exports now retain catalog captures through the
existing writer. Save As rechecks currentness before binding activation. Annotation
exports own separately bounded artifact envelopes without mutating reusable inputs;
late receipts and protected release/retirement survive withdrawal. Document birth
now records an explicit generated-newline default: LF for compatibility creation/
load, or a supplied host/catalog LF/CRLF/CR policy. Alias joins preserve the existing
owner. The default-newline facade lowers through the existing bounded line editor;
explicit/configured delimiters and literal edits are unchanged. Recipient/wire
products, later discovery profiles and broader acceptance-plan gaps remain open.

The first open → validated atomic edit → query → local-filesystem save path is
executable within the macOS profile above. Continue with missing engine behavior
families and resource recovery/ordinary-save composition, adding Rust tests and
instrumentation with implementation rather than translating every model first.

Still unimplemented and therefore not covered by green scaffold tests:

- Recipient-facing authority/disclosure, versioned typed/dynamic command admission
  and the remaining logical-view/lifecycle companions.
- Remaining annotation persistence composition, whole-document final reclamation,
  general retained target mapping,
  other retained-outcome kinds,
  asynchronous effects and lifecycle/shutdown.
- Generalized resource recovery/rebinding, complete public persistence composition
  and physical exclusive-create procedures within any claimed substrate profile.
- End-to-end engine telemetry on/off equivalence and concurrency/fault behavior.

Typing coalescing is conditional in specification §14.1, not a mandatory engine
completion item. UI/keymaps/clipboard workflows and additional filesystem platforms
are outside the selected engine/local-filesystem target. Conditional provider or
timer profiles must not be advertised before their contracts are implemented;
their optional mechanisms are not a mandate to embed runtimes in the engine.
The current engine closure worklist is recorded in docs/engine-test-plan.md.

The source preparation primitive now has on/off/saturation equivalence tests;
the trusted source coordinator also tests capture-mode equivalence for publication.
Source history tests also cover these capture modes and journal integrity faults.
Native dependency-aware reversal submissions now retain an explicit group or
expected source/annotation frontier, direction and canonical V3 read conjunction
in the shared invocation ledger. Replay preserves the original target kind and
read budget; service revalidates dependencies before atomic linked publication.
The service guard rejects reversal plans reclassified as ordinary edits or with
changed group/direction/read capture. Existing bounded diagnostic boundaries cover
this pathway; this does not implement F14 recipient admission, wire schemas or
discovery registration.
Pinned-save, search/replacement and native filesystem tests cover these capture modes as well.
None establishes equivalence for effects or companion states not yet built.

Internal `retire_absent_annotation` now releases eligible deleted-record payloads
after exact floor/current-gate checks and actual history/capture discharge. It
retains a funded identity/version tombstone without reusing lifetime issuance;
source, live annotation-set versions and peer frontiers are unchanged. This is
trusted-owner cleanup, not a public command, automatic history eviction or erasure
of previously delivered copies. Whole-document reclamation remains separate.

Source saved-state comparison now checks encoded-output and combined
current/baseline byte bounds before serialization. `saved_state_with_budget`
accepts explicit independent limits; the compatibility `saved_state` uses the
configured resource blob ceiling for output and twice that ceiling for comparison.
Larger temporary inspection is explicit, not an unbounded compatibility bypass.
Equality remains historical byte equality, not current filesystem freshness.

The source codec now supports a private prepared-artifact lifecycle. Preparation
captures exact source/serialization bytes and reserves shared artifact credit and
held bytes without issuing an artifact or writer task. Retention transfers that
charge once; protected discard releases it, including under an editing fault.
Stale/foreign/spent preparations cannot recapture or revive data. Codec capture
itself grants no destination-write or binding-activation authority.

Native source export now consumes that preparation into the shared FIFO writer
and an original-task destination reservation. Copy export never binds or adopts
an ordinary baseline. Save As admission and activation require structured resource
distinction. Fresh launch facts, original conditions,
safe effect disposition and separate quiescence/drain remain mandatory.

Native Save As activation now installs only a qualified, safely disposed capture
after a current postwrite probe and both destination barriers. A changed context
archives the exact old binding/baseline under an explicit row envelope and actual
root capacity; same-context activation preserves identity/history, and scratch
first-save invents no old binding. Later edits are not marked saved by an older
capture. Local activation and reservation release are separate retained outcome
facts; neither implies another write. Archived producers remain pinned. Full public
invocation assembly remains open.

Explicit source binding-history release now removes only the live owner's known
archive under exact binding/profile/current-pair and independent discharge checks.
It leaves edit history, the current binding/baseline, writer tasks and artifact
charges untouched. Root/promise checks still apply to shrinking archives; an empty
archive is unchanged even at publication exhaustion. Missing/false discharge facts
or unexpected native row drift cannot silently erase pins. This trusted engine
control does not require file-write rights or perform substrate I/O.

Pure source rebinding now changes only the local association, under current
binding/registration/pair facts and both old/new effect barriers. Pending document
export reservations and conflicting baseline promises block the change. A changed
context archives the exact prior association and clears the current saved baseline;
same-context requests are unchanged. A bound document with no baseline returns
`MissingEvidence` for saved-state comparison and ordinary pinned-save admission,
not empty bytes or an invented validator. Explicit Save As can establish that
baseline. Rebinding preserves authored content and undo history, issues no writer,
and does not read, rename or write a file.

Source-writer retirement now checks actual nonstopped queued consumers. Their
original baseline origin and required later written records remain retained even
after a newer current baseline is published. Newer queued anchors do not pin older
records; stopping a consumer removes its queue pin without claiming producer
quiescence or clearing faults. Detected anchor/capture/serial drift fences cleanup.
This closes retention, not own-chain condition advancement or causal-provider truth.

Native `queue_own_chain_save_v1` / `dispatch_own_chain_save_v1` now use the same
writer as pinned saves. Original captures/anchors stay immutable; a separate
launched condition advances only through retained, safely resolved, same-context
causal write facts, with current per-edge progression permission and both provider
capabilities. Gaps, foreign registrations, epoch/token reuse and mismatched final
probes refuse without refreshing intent. Zero-edge identity derivation makes no
progression claim; legacy launch methods cannot bypass the selected own-chain role.

One-time `configure_source_continuity_v1` requires explicit edge, derivation,
receipt-metadata and aggregate retained-credit limits. Minimum expanded metadata
credit is checked before artifact/task issuance; launch checks growth and assigns
a serial only on success. Stop does not refund credit; actual safe/drained producer
retirement does. These native logical projections exclude payloads funded by the
existing codec/condition bounds and do not claim wire-byte or physical reservation.
The provider-disclosed continuity receipt preserves original/derived versions and
ordered origin/serial/edge facts, separately from the original source capture.

Native ordinary source and annotation saves support writer-owned operation admission.
`request_owned_source_save_v1` privately binds origin, source/format/binding/complete
target registration, pinned or own-chain policy and the complete native execution
context before creating one child. A matching authorized request recovers that
same task without re-encoding, dispatch, baseline adoption or terminal-result replay.
Changed intent/scope refuses; guarded child retirement leaves a spent operation
tombstone. One-time explicit parent-store/capture/reply limits fund the lifetime
metadata independently of codec payloads. `request_owned_annotation_save_v1` shares
that namespace/store, capturing the original encoding context, complete source/
annotation association and pinned destination bytes/version. It requires annotation
disclosure, not source-read permission; matching lookup does not replace a candidate
or baseline claim. Actual claim, effect, drain and artifact discharge remain
independent of metadata recovery and parent tombstoning. Annotation own-chain and
typed causal conditions refuse without fallback. Full public envelope/invocation/
event and permission-filtered terminal receipt assembly remain open.

Owned source/annotation tasks now have funded native save observation contexts.
Status, retained evidence and independent protected control recheck original scope
and captured/current ceilings without dispatch, baseline adoption or producer pins.
Late facts change ordinary progress; protected progress contains only stop/quiet
bits. Bounded host-clock-step waits never renew their budgets or stop a producer.
Scope loss closes observation without refunding obligations; local receipt faults
preserve minimal control and producer state. Context/pool/ended-wait cleanup requires
independent protected discharge. Annotation-load observation now uses the same
pool, scope/policy intersection, progress, waits and protected cleanup lifecycle.

Annotation loads now support owned native read admission and matching recovery.
The parent retains the original operation, execution scope, source/set, complete
writer associations and independent reader registration. An authorized identical
retry returns the same task without submission or further issuance; changed intent
refuses. Actual EOF, quiescence, capture/provenance drain and task retirement remain
separate. Retirement spends the origin; it never authorizes re-execution. Shared
retention funds bounded lifetime metadata before admission. Stopping new reads
does not stop existing producers, and sponsorship returns only after actual drain
and protected retirement.

`open_native_annotation_read_observation_v1` attaches only to that actual owned
reader. The retained product includes ready/submitted phase, stop/quiescence,
conflict/refusal, received counts and at most two original end/failure witnesses.
Chunk conflicts cannot disclose chunk bodies. End versions and original coherence
remain lossless; EOF does not restore content or adopt a baseline. Read grants and
annotation disclosure are independent of source-read/write-disposition permission.
The provisional shared API and static markers are now named `NativeOperation...`;
`NativeOperationReceiptV1` is a closed save/source-load/annotation-load sum covering
all four native kinds. Save-specific admission remains
`open_native_save_observation_v1`; loads use their distinct source/annotation read
admission methods. Standard-schema/accepted-registry metadata and publication-
receipt assembly remain open; annotation reads are not substituted for source reads.

The engine now has a separate native source-reader lifecycle. Explicit host limits
fund its lifetime slots and pending work through shared retention. Admission issues
a ready task; submission issues one original-provider dispatch; protected intake
accepts bounded ordered chunks, exact end/version facts and independent quiescence.
Opaque and epoch/token versions stay distinct. Incremental UTF-8 validation accepts
split scalars/signatures and rejects invalid prefixes; exact EOF creates a candidate,
not a document. Explicit activation uses the existing initial-load ownership path:
create a fresh owner or join an existing owner unchanged, never reload dirty text.
Repeated activation checks the original actual writer association, independently
of reader registration. Stop never attests quiescence; dispatch/evidence pins must
drain before retirement. Source and annotation inventories are separate and share
the native outstanding-work ceiling. Atomic snapshot requests refuse without
downgrade. No I/O, locator interpretation or UI action is in the engine.

Owned source-load admission now binds the complete original native target,
strength, operation/effect/attempt and execution scope to one actual reader task.
Matching authorized recovery returns that same task without submission, decoding,
activation or another issuance—even after the admission fence or native root
fault, when retained metadata is intact. Changed intent refuses. Labeled raw origins
cannot be adopted or reused under owned names; unlabeled legacy tasks are never
adopted either. Guarded child retirement atomically spends its parent and retains
the origin. A separately funded bounded operation inventory refunds no credit
until protected global drain/irreversible retirement; it must retire before the
reader sponsorship. `open_native_source_read_observation_v1` uses the same
observation/progress/wait/control kernel as the other three native kinds. Its
receipt excludes candidates and activation/baseline results, retaining only safe
native counts/control and at most two original end/failure witnesses. Source read
and source disclosure grants are separate from annotation/write-disposition grants.
Recipient facade/profile assembly, portable schema mapping and a new filesystem
worker remain separate deliverables.

Native F06 now has an explicit optional typed/dynamic input gateway over these
same observation contexts. It accepts the four existing forms and original opaque
task/context/progress roles, copying only bounded inert data and null role shadows.
Wrong contracts, versions, fields, kinds, scope or targets refuse without fallback
or poisoning another context. `enable_native_operation_decoder_v1` funds eight
input frames plus native bookkeeping once from actual shared retention; queries
spend no new workspace reservation or observer admission. Busy ownership excludes
recursive service/retirement. Decoder-local faults retain that funding and leave
direct native protected control intact; native receipt faults remain separate.
Only guarded native context retirement returns decoder funding. This is a native
in-process binding—not a wire/hostile-object codec, complete accepted-registry
schema/discovery relation, producer grant or public event transport.

Native discovery now has a separately funded base-profile `/1` cut/context boundary.
Unsupported/later profiles refuse rather than borrowing broader schema semantics.
It copies bounded, explicitly attested registry metadata and complete visibility/
order data, then consumes a scope/publication-bound cut to capture only permitted
entries/variants/facets and an independently observable target. Literal metadata
does not register a handler, execute a schema, configure UI bindings or grant
invocation rights. Protected stop/retirement preserves lifetime issuance, and a
spent registry cannot be replaced to reset its budget. Native list/inspect/advice
queries now consume fresh scoped cuts, intersect current disclosure with captured
ceilings, and publish opaque selectors only after the complete bounded reply is
ready. Unknown facts withhold; explicit authority/retention loss stops the original
context. An actual pre-install reply guard checks original roles, dense pages,
bounded approved metadata and tentative selector use. Malformed producer results
fence discovery without publishing tentative selectors, refunding spent issuance,
or faulting the editor root; original protected cleanup remains available.
False advice is data, not execution permission. Original request roles and query
bindings are recognized before loss effects or cut consumption: foreign selectors,
mismatched entry/variant or cursor-limit bindings and malformed page scalars cannot
stop a valid context or consume its cut. Valid recognized loss still absorbs before
current page capacity or transient delivery refusal. Optional diagnostics separate
request recognition and query execution, including opt-in synchronous service timing.
Cuts now bind the native current
root through the existing guards. A fixed builtin /1 source also derives the six
annotation/history logical descriptions from native handler declarations, with
bounded trusted-owner inspection and optional local preparation/inspection timing.
It discharges source origin only; authority, visibility/order, resources and advice
remain separate facts. Private origin survives context acquisition, so generic
lookalikes cannot substitute for builtin current cuts (or the reverse). Metadata
neither registers nor enables execution. Outer schemas and full facade/value
binding remain open. Generic cuts still bind the native current
root through owner/publication identity plus exact administrative headers,
append-only registration positions and attachment lifecycle generation/incarnation.
This catches same-size changes and present/absent attachment cycles without changing
authored revisions. Attach reserves its detach step before taking credit, so global
publication exhaustion cannot block that cleanup. F01 now also has an explicit,
context-sponsored typed/dynamic native gateway for all three query forms. Its
decoder consumes shared closed field/role declarations, copies bounded arguments
and current facts, then calls the existing current-root/disclosure/result guards.
Original opaque roles and full u64 scope epochs are preserved. Workspace credit is
reserved once and returns only through actual protected context retirement;
capture/post-native faults fence discovery without undoing spent issuance.
These are passive Rust values, not a JSON/wire codec or decoder grant inferred
from metadata. Schema/accepted-registry truth, complete public facade assembly
and normalized/wire/physical refinements remain open; the native stamp is not
an external accepted-registry or filesystem-state attestation.

Native source evidence now supports explicit bounded epoch/token versions and
separate actual before/after causal receipts. Causal-receipt and non-reuse claims
are independently registered and default off; accepting causal facts alone does not
authorize own-chain advancement. Result/edge conflicts share a two-fact typed
witness cap and fault the destination in either arrival order. Compatible facts
and exact duplicates do not erase faults. No epoch is parsed from opaque validators.

Provisional Rust API change: `SaveDispatch::condition_v2` represents absence,
opaque versions and typed causal versions without loss. `condition_v1` now returns
a `Result` and refuses typed causal versions. `expected_validator` is the legacy
opaque view and returns None for a causal version as well as absence; use the
full v2 view or `expected_causal_version` to distinguish them. No token-only
downgrade, fabricated absence token or epoch is permitted.

Absent source exports can now explicitly select an epoch/token after-result
through `SourceExportOptionsV2`; v1 absence remains opaque. Creation has no before
version or causal edge. A concrete before-version observation contradicts absence
and is retained in the bounded fault witnesses. Typed Save As after-results can
establish an ordinary baseline and a later zero-edge own-chain anchor; copies
remain unbound. Capability, create authority and actual effect truth are separate.
This implements the engine receipt contract, not a filesystem create procedure.

New provider destination/causal capabilities default off. The macOS
adapter still supports present best-effort staged replacement only and refuses
unsupported conditions/guarantees, including typed causal versions, before writing.
A real adapter regression preserves the old file and returns explicit non-write,
safe-disposition and quiescence receipts even for a host-misadvertised causal port.
Existing-file copy export is
tested through the real adapter, as is Save As activation using its fresh postwrite
observation and real receipts; exclusive-create and atomic-conditional host
procedures are not established by scripted capability tests.

The JS models remain reference material. Rust scaffold passes do not replace their
proof obligations, qualify the full design or prove behavior not implemented yet.
The historical design acceptance percentage is not engine completion.
