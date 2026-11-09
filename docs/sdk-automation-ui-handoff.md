# Native integration and frontend handoff

Use the working Rust consumer layers rather than rebuilding engine owners.
The APIs remain provisional native Rust interfaces, not a published wire SDK,
untrusted-script sandbox or frozen compatibility promise.

## Starting points

- [LocalHost](../crates/vellum-session/src/host.rs): one engine authority and
  filesystem module across multiple original document handles. Supports exact
  edits/batches, declared reads, linked history, original save tasks and explicit
  protected lifecycle operations.
  Optional native invocations use `issue_invocation`, `submit_source_invocation`,
  `run_invocation` and retained `invocation_outcome` with explicit configured
  limits. Reuse the original engine-issued ID and exact intent for matching
  replay; a terminal result describes that operation then, not current source.
  Stop, forgetting disclosure and retirement are separate native operations.
  `file_edit --invocation` exercises this route through real save/reopen/cleanup.
  This is not a clipboard effect ledger or its missing joint continuation seam.
  `capture_text_transfer` requires an original issued invocation and exact native
  view context, with explicit per-capture fragment/plain/logical-byte limits. It
  retains the normalized selection set and copied inert fragments; plain data
  joins with LF, while all-empty selections return no export. This is privileged
  host data, not a structured clipboard identity or recipient/provider grant.
  Tasks must separately reserve aggregate retention, mediate release and
  preserve dispatch/evidence/continuation contracts. Capture creates no task,
  acknowledgment, source publication, history group or terminal invocation result.
  The optional `host::copy_transfer` gateway now implements that task ownership
  for **plain copy, cut export and paste**. Configure explicit `CopyLimitsV1`, register the complete
  host-declared provider/destination/acknowledgment profile, issue a native
  invocation, and `admit_copy` against its exact native view context. Matching
  admission retrieves the original task/capture even after later source changes;
  changed context/provider refuses. All-empty copy/cut tasks cannot dispatch. Retain the
  sole `dispatch_copy` token: loss is unknown work, not permission to retry.
  `release_copy_plain` rechecks the named current provider and release/disclosure
  premises, returning copied inert data only. No provider callback or OS I/O is
  hidden in these methods. `record_copy_evidence` requires authentic host facts
  and the original port/token; acceptance, nonapplication and quiescence remain
  distinct, exact duplicates are inert and contradictions remain contested.
  Stop/revoke deny new disclosure without deleting authentic late facts. Drain,
  retention and session release require separate truthful host attestations;
  unknown/contested/live work blocks retirement. Providers retire separately.
  Logical byte accounting includes retained captures and provider tombstones,
  not allocator/RSS peaks, transient capture construction or recipient-owned copies.
  `copy_usage` has fixed per-operation RED counters, logical task/work pressure
  and opt-in monotonic acknowledgment/unknown/work timing independent of trace
  saturation. Off/clock failure remains missing timing, not zero. This implements
  neither actual OS provider truth, session-structured handles, hostile wire
  decoding nor live event delivery.
  `admit_cut` additionally requires explicit edit/view-write premises and retains
  a private publication-capable claim, checking native empty-literal descriptor
  capacity before any export. No source edit is prepared before acknowledgment.
  `continue_cut` then uses that original invocation for native literal deletion:
  accepted uncontested export is necessary, quiescence alone is insufficient.
  Current provider revocation/permission denial, stale source/view/selection or
  native preparation limits produce a spent known nonpublication result; restoring
  rights does not retry the old intent. Generic permission-aware literal admission
  retains denial in the existing engine ledger, without clipboard vocabulary in
  the engine. Submit/run occur in one exclusive host call with no provider I/O or
  callbacks between validation and native root/outcome publication. Local applied/
  unchanged/refused facts derive from the native retained outcome, not a duplicate
  committed flag. `cut_report` serially joins that historical decision with current
  external evidence. Stop/no-export before a source attempt does not fabricate a
  native source outcome; unexpected native service failure preserves local Unknown
  and blocks retirement and further cut starts through a visible integration
  fence. Undo/redo never repeats the external export, and later
  conflicting receipts cannot roll back source or erase unresolved obligations.
  `configure_paste_transfers` adds explicit target/plain/expanded-byte limits on
  this same registry; common task/port/dispatch types currently retain their
  provisional `Copy*V1` names. `TextTransferKindV1` and
  `TransferLocalDispositionV1` now describe all supported native transfer kinds.
  `admit_paste` captures only exact target context/selections, requires read/edit/
  view-write/retention premises and reserves canonical/conflicting/native body
  buffers before provider work. Paste does not require export-disclosure rights.
  `dispatch_paste` checks the original target again and current named readable
  registration plus launch/read premises; it never consults newer focus.
  `record_paste_evidence` authenticates/correlates/bounds before mutation, separately
  checks payload retention and retains exact duplicates/contradictions without
  last-arrival replacement. Only bounded UTF-8 plain data is supported. Ordinary
  copy receipt/release routes cannot bypass paste data admission. Empty Accepted
  data remains different from no result. Receipt reserves cover one canonical
  body, one conflicting body and the native intent; further differing witnesses
  keep contested evidence without unbounded storage. `continue_paste` submits the
  reserved owned literal text and runs the native invocation in one exclusive
  call, revalidating original source/view/selections and native companions. The
  complete literal is inserted at every target, never split/interpreted. Current
  denial or ordinary native limits produce a spent known refusal. Historical
  results and current external facts join in `paste_report`; native faults retain
  Unknown and fence new paste starts. Stop/revoke permit authentic late facts,
  not a new source attempt or erasure of unknown effects. Shared retirement keeps
  original provider work/drain/session/retention guards and burns the native ID.
  `configure_logical_views` enables caller-selected native view limits. Retain
  original view IDs, attachment stamps and `LogicalViewContext`; explicit
  attach/set/endpoint/literal-edit operations refuse stale or foreign contexts.
  Selection-derived edits publish source and initiating/peer selections with the
  existing annotation/history companions. Optional `configure_line_edits` and
  `edit_logical_lines` preserve explicit bounded prefix/newline semantics.
  Detach and retire original view owners explicitly; profile configuration is
  not a permission grant. `file_edit --logical` is a runnable native-view consumer.
  Reference UIs also enable explicit `SessionConfig::line_edits` budgets and
  route indent/outdent through the original native line intent. The iced bridge's
  literal/range `proposal` rejects line intents explicitly; use `line_edit`, not
  a guessed replacement plan. This block action refreshes its resulting display;
  ordinary typing retains incremental patches. Unsupported/exhausted profiles
  refuse without a UI editing fallback.
  Optional literal search uses `open_search`, `search`, `navigate_search`,
  `replace_search` and `close_search` with caller-selected native `SearchLimits`.
  Close the original session and drop result/occurrence captures to discharge
  their distinct retention; do not turn partial results into complete totals or
  silently rerun stale replacements. This is a trusted-host route, not recipient
  authorization. Both reference frontends now route literal search/replacement
  through the shared presentation owner. Search is tab-owned, stale results
  refuse action, and close releases native sessions and result captures.
  Optional annotations use `commit_compound`, `annotation`, exact
  `capture_annotations` and separate `set_annotations_locked` policy. Creation
  identities are original native handles returned only after successful commit.
  `reverse_annotation` selects the annotation frontier; ordinary `reverse`
  selects a source frontier. Both reverse the full linked group when eligible.
  `retire_annotation` requires actual history/capture release and supplied host
  facts; it is not deletion of persisted JSON. The shared `Editor` now offers
  original-target `CommentDraft`, versioned `CommentHandle`, cached rows, explicit
  create/update/delete/reattach, separate annotation lock and native comment-history
  reversal. TUI F10/Shift-F10/F11 exposes these session controls; multiline forms
  preserve literal bytes and previews escape control/bidi data. Source-only saves
  and Save As leave comments unsaved; explicit discard gates close. Current dirty/
  presence reads use native document summary/facts without a whole-source snapshot
  or loan, and mapping reuses unchanged body strings. Native snapshot capacity zero
  does not disable these trusted current queries. The iced Comments pane now uses
  these same native operations, with lossless bounded literal forms, separate
  scratch undo/redo, original source/form attachments and unapplied-form close
  protection. Widget-tree/IME tests are headless, not desktop qualification.
  Both apps now expose separately confirmed annotation-file save/restore using
  the host's optional bounded UUIDv4 identity/source-witness policy. Never report
  a source save as saving annotations or infer historical-source restoration
  evidence from valid JSON. Unactivated historical JSON remains Save-fenced.
  SDK persistence now supports `bind_annotations` to an explicit existing file,
  `register_annotation_encoder`, `encode_annotation_json` with host-provided
  persistent associations, `queue_annotation_save`, `service_annotation_save`
  and separate `publish_annotation_save_baseline`. Source saves use the same
  native writer and current distinct-resource evidence at admission, dispatch
  and baseline publication. Alias/missing-file refusal does not create a file.
  Shared `Editor` persistence uses `bind_comment_file`, `capture_comments` and
  `save_comments(snapshot, associations, limits)`, with separate
  `last_comment_save_outcome`. The trusted embedding supplies persistent identities
  and exact source witnesses; this is not automatic UI path/identity discovery.
  `load_comments(context, approved, limits, baseline_budget)` performs an explicit
  initial load into an untouched annotation set. Inspect `last_comment_read_status`
  and `last_comment_restoration_outcome` on errors: restoration may have succeeded
  before independent baseline adoption failed. Retry only that adoption with
  `adopt_comment_read_baseline`; do not silently re-read/restore. Use explicit
  `retire_failed_comment_read` or `retire_checkpointed_comment_read` for their
  distinct native discharge routes. Close also releases disposed reads and the
  loaded-baseline endpoint after actual bindings/history/provenance release.
  Dirty checks use native written source/set facts conservatively, including an
  unsaved deletion-to-empty. Capture loans must be dropped before close. Original
  tasks survive errors; superseded annotation artifacts discharge separately.
  Annotation bindings share an exact observation guard across clones; only a
  truthful attributed write advances it, not a newer probe. A missing post-write
  observation fences later saves instead of refreshing away uncertainty.
  Decoding saved JSON is not native restoration or a two-file transaction.
  Reopen through `queue_annotation_read` / `service_annotation_read`, then
  `decode_annotation_json` and `restore_annotation_json` with explicit
  `RestorationAssociations` and an original registered restorer. Preserve the
  read attempt on failure; inspect status/outcome separately. Enabling the
  native read-baseline endpoint and adopting an individual loaded baseline are
  explicit operations, not an inference from valid JSON. Use unified
  `annotation_baseline_evidence` for Written/Loaded and `compare_annotations`
  for actual current-content comparison. `resource_usage` reports native logical
  queues/retention, not physical allocation or completion. Disposed annotation
  bindings have their own release; read retirement can still refuse real holds.
  `retire_checkpointed_annotation_read` requires a current independently written
  baseline; `retire_disposed_annotation_read` requires actual disposed provenance
  release. Both refuse retained read loans. `disable_annotation_read_baselines`
  detaches endpoints only after their consumers discharge. None implies erasure,
  file deletion or whole-engine shutdown.
- [LocalSession](../crates/vellum-session/src/local.rs): single-file convenience
  over that same host; not a second implementation or engine-per-file mandate.
- [Editor](../crates/vellum-model/src/app.rs): shared presentation state above
  the host. The reference profile attaches original native logical views;
  provider-neutral [clipboard routes](../crates/vellum-model/src/app/clipboard.rs)
  require explicit `configure_clipboard` limits and a native invocation profile.
  Register the real named provider, then begin copy/cut/paste against the active
  native context. The returned redacted `ClipboardPlan` identifies the original
  task across focus changes; it is not a grant. Provider dispatch/data/evidence
  stay explicit outside source publication. `complete_cut` / `complete_paste`
  adopt native history once and preserve copied-not-cut/unknown truth; UI refresh
  failure cannot relabel publication. `fresh_local_decision` is presentation
  adoption metadata, not an independently assigned native committed bit. Retained
  replay never reprojects current selections or duplicates history. UI bookkeeping
  capacity refusal explicitly stops the local intent before native publication.
  Original controls are kept across failed admission/cleanup. Retire task and
  provider controls with real discharge evidence; discard consent cannot bypass
  unresolved clipboard work during close. The common facade supports native
  multi-selections used by ratatui. Iced has matching provider-neutral routes;
  initiation validates its visual token/current single-primary profile, while
  completion follows the original plan, never current focus. A new applied active
  result spends old widget input and refreshes source projection; background
  results and retained retries do not reset the current attachment. Replacement
  projection caches are staged before installation. `vellum-adapter-clipboard-macos`
  now supplies `EditorClipboard` over those routes: explicit host premises,
  one pending original operation, retained provider receipts and independent
  publication/retirement. Delivery retries never repeat platform work;
  platform-result summaries remain distinct from native local outcomes. An eager
  export rejected after clearing prior contents is not a rollback or permission
  to cut. Both reference UIs now wire source gestures through the shared
  `ReferenceClipboard` workflow with explicit trusted-local-owner premises.
  Provider binding is lazy; delivery retries never repeat platform I/O, unknown
  work blocks close, and canceled unsaved-close preflight keeps the idle binding.
  Live qualification of the selected AppKit profile remains open;
  these headless routes are not desktop, clipboard or secondary-geometry assurance.
  primary selection caches project their mapped state, with explicit UI-local
  grapheme projection and viewport behavior. Close detaches/retires original
  views before document cleanup. Source and undo history remain in the engine.
  `select_matches` validates the original complete native result and installs its
  bounded logical set without rerunning search. Terminal multi-selection edits,
  movement and deletion reuse native normalization/publication. Both reference
  apps now explicitly enable the qualified Unicode18 provider; session Unicode
  capture/evidence/command methods remain provider-neutral. The shared editor
  retains one exact-version evidence/cache entry and discharges original ownership
  on failure/close. Iced native source gestures use declared 'after' projection
  before logical target creation; exact SDK points, literal ranges and admitted
  commands are never repaired by that path. Candidate previews are computation,
  not source authority, and are measured separately from evidence generation.
  The iced example still exposes only its primary-selection visual workflow.
- [Ratatui consumer](../crates/vellum-tui/src/terminal.rs) and
  [terminal process](../crates/vellum-tui/src/main.rs): frontend-only keymaps,
  safe terminal projection, explicit quit/discard and terminal restoration.
- [Native contract matrix](public-contract-matrix.md) and
  [engine API boundaries](engine-api-boundaries.md): broader engine profiles,
  advanced native owners and responsibility/compatibility contracts.
- [Reference UI work](reference-ui-design.md): remaining presentation obligations.
  Iced desktop qualification, full feature parity and native accessibility/IME
  qualification remain open.

Run commands and reference-profile limitations are in the [README](../README.md).

## Ownership and trust

Applications share one host across documents; serialize mutable access instead
of cloning authority or creating an engine for every tab. Keep original native
handles and exact versions. Discovery metadata and handle possession do not
grant permission. The local host is trusted: an automation or remote boundary
must establish its caller's authority separately before disclosing or mutating
anything. Document text and annotations are inert data, not host instructions.

UI owns bindings, focus, visual movement/layout, preedit, accessibility bridges
and platform clipboard interaction. Shared presentation actions are not new
engine verbs. Logical engine views, selections and provider-backed semantics
must retain their existing owners when those profiles are integrated; the
reference cursor is not qualification of full selection/provider parity.
Integration owns workflow orchestration. Substrate modules own actual I/O;
use the [existing filesystem module](filesystem-adapter-handoff.md), not UI writes.
`Editor::open_additional` joins existing owners without replacing unsaved source.
`LocalHost::set_editing_locked` exposes trusted host source policy enforced by
native publishers; UI lock indicators neither grant authority nor alter OS
permissions. Existing native policy also refuses ordinary saves while locked;
unlock explicitly rather than bypassing that guard in an integration.
Do not stringify opaque identities, add a speculative transport, or infer stronger
filesystem guarantees from the engine's atomic in-memory publication.

## Save and lifecycle rules

Explicit source destinations use `prepare_source_export`, `queue_source_export`
and `service_source_export` with current host authority facts. A CopyExport does
not rebind or mark the original saved. SaveAs requires separate
`activate_source_save_as`; retain and use its returned document handle for later
ordinary saves. Activation failure cannot roll back an acknowledged write.
`release_source_export` controls the original reservation; native task retirement
remains separate. Filesystem preflight can leave preparation pending while a
native admission refusal can spend it—do not infer either from a generic error.
Missing destinations refuse rather than inventing a creation procedure.

Retain the save handle returned by admission before service. A service error
does not erase the attempt, prove nonapplication or authorize automatic retry.
Read the original outcome; distinguish application, synchronization, no-future
effects, quiescence and cleanup. Baseline publication is a separate local step.
Preserve known Written even when synchronization fails. A current baseline can
retain its original save task legitimately.

Do not equate Drop with completion. The presentation host explicitly checks
discard consent/clean state, stops original attempts, verifies quiescence,
disposes documents, releases bindings/history/provenance, retires save owners
and requests guarded reclamation. Retained history closure can retire multiple
groups; original handle cleanup must respect the actual owner. Completed steps
remain recorded across later refusal. Native guards still decide whether pins
are discharged. These operations neither delete files nor certify physical
erasure or whole-engine shutdown. Advanced engine operations retain original
owners through `into_parts()`. Its `Result` returns the intact host/session on
outstanding integration claims or copy-provider registrations; never discard that
refusal as completed cleanup.
`claim_invocation` reserves an original issued lifetime in the integration layer,
bounded by the native retained-invocation configuration. Ordinary execution and
retirement routes refuse a claimed ID. Use the opaque claim for native source
submission/execution (publication kind only) and explicit retirement. Neutral
claims cannot fabricate source outcomes. Drop does not release a claim, and a
retired ID cannot become a new attempt. Claim usage exposes logical retention,
successful claims/retirements and ownership-guard refusals (not all native errors).
This is not a clipboard task ledger or proof of provider acknowledgment/quiescence;
the owning integration must discharge external obligations before retirement.

## Development and acceptance

Exercise actual public routes and real temporary files. Start with focused tests
in `vellum-session`, `vellum-model` and `vellum-tui`; reserve workspace-wide
qualification for the integrated release checkpoint. Preserve tests for stale
or foreign targets, cross-file dependencies, linked history, external changes,
lossless bytes, explicit failures, capture pressure and protected cleanup.
Frontend tests must connect events to actual engine behavior, not just render
static widgets. Test real framework/terminal lifecycle as well as headless paths.

The [macOS GUI bridge](https://github.com/gantbd/vellum-gui-macos/blob/main/src/bridge.rs) now uses that same host and
maps a normalized display to exact original source positions without a diff or
widget-owned undo. Headless checks pass; native desktop interaction is not yet
qualified. Its text projection must never normalize serialized source. Numeric
performance budgets, the production Unicode provider and the supported release
matrix need explicit decisions and qualification; dependency availability is not
evidence of IME, screen-reader, bidi or platform conformance.

Earlier per-slice descriptions are preserved in the
[historical handoff](sdk-automation-ui-handoff.history.md). Their “open” native
engine statements do not override the current completed engine delivery or code.
