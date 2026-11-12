# Changelog

All notable changes to Vellum will be recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and released versions
will follow Semantic Versioning once a public release contract is selected.

## [Unreleased]

### Changed

- Added a pure release activation/rollback algebra and private-scratch
  qualifier. Exact approval binds distinct candidate/prior identities;
  immutable digest-named daemon/MCP/plist bundles, a synchronized atomic
  desired-state pointer, durable journal, deterministic crash cuts and
  content-safe terminal/refused/unknown receipts are qualified without
  executing payloads or invoking launchd. The pointer is explicitly not a
  claim of atomic job activation, and every receipt reports the launchd effect
  as not attempted.
- Consolidated the engine, host, protocol, service, adapters, connectors,
  applications, and SDK facades into the current categorical workspace.
- Made the entire workspace explicitly non-publishable while GitHub source
  distribution and cross-repository dependency contracts are hardened.
- Separated per-client view state from shared workspace membership. Opening and
  closing a view is now an authority-mediated, replay-safe transition, while
  removing a document or root retires only its bounded, non-reused membership
  identity and does not claim filesystem deletion or native-resource disposal.
- Made persistent runtime membership changes checkpoint the complete active
  locator binding set plus separate document/root issued prefixes and
  reservation ceilings before an accepted reply can escape. Recovery allocates
  at issued plus one; removal preserves the marks. Refused or indeterminate
  checkpoints suppress the reply and fence the daemon rather than inventing
  rollback.
- Advanced private workspace identity persistence to `VLMID03`. Newly advanced
  issued tails must be contiguous and present in candidate bindings; v1/v2
  records refuse without fake migration. Recovered ordinary, durable, and
  retired receipt evidence is validated against the correct sort's issued mark,
  including every directory-admission identity.
- Kept semantic intelligence outside the editor and native Vellum protocol.
  The provider-neutral semantic service owns immutable-snapshot synchronization
  and bounded query routing; its client facade carries no edit, process-launch,
  transport, authentication, or workspace-trust authority.
- Made the semantic client port intrinsically partition-bound and asynchronous.
  Ordinary clients can observe only readiness/capabilities; provider provenance,
  exact expiry and bounded pumping remain on composition-owned surfaces.
- Added an explicit semantic-provider physical-resource policy. The compatible
  constructor remains deliberately unbounded; the strict constructor requires
  one whole-tree memory, process-count and CPU policy, binds its exact ceilings
  into the qualified profile identity, preflights host support before spawn and
  refuses unsupported or unenforceable policies instead of degrading. Support
  receipts keep the three axes distinct and retain filesystem, network,
  subprocess and abrupt-parent confinement as independent nonclaims.
- Added exact pending-owner deadline scheduling with deterministic bounded fair
  retries, plus staged provider replacement and truthful graceful, forced-
  pending, forced-confirmed and cleanup-failed retirement phases.
- Added explicit semantic-client retirement. Retiring partitions are fenced,
  pending work is cancellation-signalled and quarantined, queued deliveries are
  dispositioned, refused document closes remain retryable, and client capacity
  is reclaimed only after source-bearing snapshots are confirmed closed.
- Split semantic query from semantic synchronization as independent closed host
  authority atoms. Both remain absent from the default read-only authority set;
  authentication alone grants neither.
- Added a shared, non-wire semantic invalidation receipt atom for future local
  composition. It carries one closed cause and a caller-bounded, strictly
  ascending unique set of opaque request identities; malformed evidence cannot
  be normalized into apparent agreement and debug output exposes only cause and
  cardinality.

### Added

- Added separate unpublished semantic-session domain and canonical wire crates.
  Their V1 protocol separates read-only exchange correlation from replay-bound
  mutation identity, binds every reply to exact incarnation/membership context,
  admits only the closed command/outcome/refusal fiber, and models full-snapshot
  synchronization as explicit begin/complete epochs with bounded manifests.
  Strict decoding rejects noncanonical, malformed and over-limit frames; frozen
  command/reply fixtures are tracked independently from the native editing wire.
  The protocol itself owns no broker, socket, authentication, persistence or
  transport phase ledger; those remain separately composed crates.
- Added a nonpublishable transport-neutral semantic-session client ledger with
  phase-aware `NotSent | Reply | OutcomeUnknown` handling, explicit exact
  mutation replay under fresh exchange correlation, attenuated Join validation,
  a monotone definitive-operation budget, transactional Ready-to-next-epoch
  staging with incarnation-global epoch-ABA refusal, compact source-free synchronization witnesses, bounded
  replay/result/witness retention and full request/result binding checks. It
  contains no socket, authentication, broker, provider selection or automatic
  retry policy and cannot mint the embedded semantic client's private sovereign
  seal.
- Added a nonpublishable pure serialized semantic-session broker reducer. It
  multiplexes authenticated memberships over one sovereign partition,
  namespaces exact request retention, enforces shared checked replay/result
  accounting, serializes affine synchronization effects, supports transactional
  Ready-to-next-epoch Change/Open/Close publication, and drives bounded
  cancel-then-close cleanup after policy loss without fabricating protocol
  exchange, operation, reply or authority identities. Content-free fixed-domain
  counters/gauges include replay, retention, membership, request, witness,
  synchronization, cleanup and malformed/orphan delivery saturation evidence.
- Added a nonpublishable bounded semantic-session actor around the pure broker.
  It preserves exact pre-admission versus post-admission phase truth, keeps
  affine continuations private behind five typed effect methods, separates
  trusted controls from client traffic, returns event ownership on control
  backpressure, and fails closed on owner loss. Its drain/stop lifecycle is
  volatile and adds no socket, authentication, persistence or retry claim.
- Added a nonpublishable transport-neutral semantic-session frame adapter. It
  performs exactly one canonical exchange, preserves `NotSent` versus
  post-send `OutcomeUnknown`, validates every reply binding, and records only
  fixed-cardinality traffic/outcome observations. Socket, authentication,
  reconnect, server and retry policy remain outside the crate.
- Added a fresh-empty, single-client semantic bridge service surface with
  operation-specific ordered reports. Open, Change, Close and Submit preserve
  separate pre-provider, operation and post-provider facts; exact canonical
  invalidation receipts replace synthetic terminal deliveries without
  changing the legacy service algebra. The surface deliberately remains
  non-wirable until runtime recovery and broker-side atomic receipt validation
  exist. Bridge cancellation and composition-driven expiry now preserve
  pre-transition satisfaction without duplicate signals, while a separate
  fallible one-poll provider port distinguishes absence, failure, validated
  delivery, late completion and orphan completion.
- Replaced the semantic broker's broad transition terminalization with exact
  ordered-report validation. The broker captures provider/request expectations
  at effect emission, validates provider-before plus observed/settled
  pre/operation/post evidence against a shadow set, and commits only exact
  receipt identities. Mismatch latches fatal divergence; the actor returns
  outcome unknown, fails and reconciles admitted uncertainty.
  One global affine effect barrier now spans memberships and retirement work.
- Closed provider polling as one atomic composition event. A successful drive
  carries provider-before, optional pre/post transitions and exactly one
  no-event, delivery, late or orphan disposition; raw provider-state and
  delivery actor events were removed. Invalid evidence cannot partially commit
  provider epoch or request state, and broker event debugging is variant-only.
- Split actor control from recovery ownership. A must-use recovery capability
  preserves the exact broker and effect port across clean stop, owner loss,
  divergence, provider failure and caught panic; recovery is nonblocking and
  does not claim client or provider retirement.
- Closed bridge correlation lifetime independently from live pending capacity.
  Cancelled, expired, invalidated and delivered request identities remain
  bounded non-reusable tombstones for the provider lifetime, duplicate replies
  remain exactly late, client retirement is explicit, and checked consuming
  provider extraction returns a retirement-only capability with counted
  residue.
- Implemented fallible LSP bridge polling. Transport and cancel-send failure,
  malformed/refused delivery and local poison remain distinct; cancelled LSP
  correlation is bounded and retained so consuming a late response can never
  be reported as no event.
- Made every actor service effect fallible. Open, Change, Close, Submit and
  Cancel failure now produce outcome-unknown, stop serving and retain the exact
  broker continuation and port for recovery. Invalid limits and thread-launch
  refusal likewise return the untouched runtime instead of dropping cleanup
  authority.
- Added coordinated epoch-zero broker genesis for unavailable, indexing and
  ready providers. One validity law requires bottom capabilities and no
  provenance outside Ready, and exact provenance inside Ready, at genesis and
  every successor transition.
- Added an injected-provider, deadline-disabled ordered local composition. One
  affine prepared value owns the fresh service/client, broker and only epoch
  mapper; build failures retain exact raw or prepared residue, actor spawn is
  lossless, and recovered values expose cleanup-only client retirement and
  checked provider extraction.
- Added ordered expiry and poll-failure evidence. Timeout is now a dedicated
  sixth actor effect rather than a provider delivery, physical delivery is
  Peer-only, and a failed physical poll carries any already-applied
  pre-provider transition so the broker can reconcile it before fatal actor
  shutdown.
- Sealed ordered LSP construction behind a fresh-started adapter typestate.
  Used, buffered or terminal adapters and failed initialize sends retain exact
  cleanup ownership; session/service/LSP semantic, document, source, pending,
  URI and worst-case framed-body bounds close before installation. The generic
  injected-provider constructor is crate-private.
- Added a distinct timed ordered composition with an injected monotonic clock,
  fixed relative policy, exact dual request/due indexes and one canonical
  expiry per drive. Every effect preflights scheduler consistency, terminal
  receipts clear infallibly, and clean stop refuses live, inconsistent or
  poisoned obligations. Scheduler retirement returns exact disabled,
  discarded or unresolved evidence.
- Added forced bridge abandonment and terminal provider receipts. Forced
  extraction preserves exact live-state and attempted/skipped/unknown effect
  counts, catches provider panics without issuing later effects, and yields a
  forced-only capability. The local driver joins that evidence with protocol,
  transport and process termination as `ForcedConfirmed` or `CleanupFailed`;
  ordinary graceful and escalation races remain distinct.
- Added composition-owned dynamic path-free LSP virtual-document issuance and
  a local embedded semantic runtime facade that preflights bounds, launches the
  qualified process transport, starts the adapter and privately owns the
  semantic service. The shipped binary remains a probe, not a daemon.
- Hardened local semantic supervision with aggregate stdin byte credit,
  monotone close/abort fences, fail-closed terminal classification, concurrent
  finish/output drainage, inherited-pipe detection and confirmed cleanup for
  post-start launch refusal. Observations distinguish whole-group hard-kill
  acceptance from supervised-leader reap and do not claim descendant reaping.
- Added single-use local-provider qualification with redacted artifact/profile
  identities, exact executable/cwd revalidation, loader-variable refusal and
  explicit assurance classifications. Launch timeout now retains its worker
  owner behind a permanent non-overlap fence; supported Unix launches suppress
  core dumps without claiming executable immutability or confinement.
- Added a bounded sealed semantic-plan journal contract whose synchronization
  order prevents handle or receipt publication ahead of durable evidence and
  whose recovery never reapplies a pending effect. The included fault journal
  is joined by a macOS `F_FULLFSYNC` local adapter with descriptor-relative
  private storage, exact workspace/owner namespace binding, exclusive
  cooperating-writer ownership, strict no-eviction bounds and conservative
  crash uncertainty. It remains unwired to daemon and MCP composition.
- Added a nonpublishable in-process MCP 2026-07-28 semantic projection library
  over the partition-bound client SDK. Cacheable results are private and
  zero-TTL, client metadata is bounded, and construction requires sealed
  sovereign validation for nested document/span safety. It is not yet a
  runnable MCP peer or authenticated transport, and it does not advertise
  rename application.

- Added a descriptor-anchored single-subject local credential adapter and
  optional path-only daemon/MCP composition. Provisioning is exclusive,
  rotation is staged, proof attempts are bounded, omission remains
  `OwnerOnlyOffline`, and authenticated identity remains categorically
  separate from the daemon's global action ceiling.
- Added a revision-witnessed `ReplaceAndSave` native operation with independent
  edit/persist authority and explicit `Durable`, `NotDurable`, `Unknown`, and
  `NotAttempted` persistence outcomes after an accepted edit.
- Added exact, locator-free file and directory admission receipts. File
  admission distinguishes a newly added document from an existing member;
  directory admission returns the exact root and ordered set of documents added
  by that operation instead of substituting a later inventory snapshot. Legacy
  admission outcomes remain decodable, while the new reply shapes are frozen by
  canonical wire fixtures.
- Added bounded workspace lifecycle and inventory surfaces: empty daemon startup,
  file or directory startup roots, explicit view open/close, paginated opaque
  document/root inventory, authority-mediated admission and retirement, and a
  content-free revisioned change feed with explicit gap and restart-epoch
  semantics.
- Added the modern MCP `2026-07-28` peer surface for exact-document snapshots,
  revision-witnessed replacement, replace-and-save outcomes, bounded change
  polling, workspace membership, and canonical workspace resources. MCP cursors
  and uncertainty handles are connector-owned and process-local; they are not
  durable sessions, credentials, or remote idempotency keys.
- Added a bounded semantic-query algebra for symbol search, definition,
  references, implementations, diagnostics, type-at-point, and rename planning,
  plus sovereign semantic service/client crates for exact immutable-snapshot
  synchronization and response routing. A no-process-authority LSP adapter now
  supplies bounded framing, virtual-document synchronization, exact UTF
  position conversion, pull diagnostics, hierarchical document symbols,
  private readiness-gated generations, composition-installed provider
  provenance, cancellation, correlated graceful shutdown/explicit abort and
  content-safe capacity telemetry over an injected transport. Static rename
  support accepts only current versioned text-only document changes and rejects
  resource operations, annotations, foreign targets and ambiguous ranges.
  Provider result
  locations are revalidated against the owning client's exact snapshots before
  delivery. Provider replacement requires deterministic full snapshot replay
  under fresh stable provenance/generation, while composition-driven expiry is
  delivered distinctly as `TimedOut`. No semantic daemon, background clock or
  automatic-restart loop, PSI/nREPL adapter, runnable MCP transport, or
  filesystem-backed semantic-plan journal ships in this change.
- Added a separate nonpublishable semantic-apply service. It exact-binds
  client-owned semantic snapshots to workspace documents, rechecks current
  provider and source freshness, requires exact edit authority, derives
  expected bytes from fresh workspace snapshots, and publishes bounded
  multi-document rename plans through one linked in-memory transaction.
  Persistence is explicitly `NotAttempted`; no multi-file disk atomicity or
  ambient authority is claimed.
- Added a nonpublishable local semantic process supervisor substrate with
  canonical operator profiles, fixed argv, cleared/allowlisted environment,
  private cwd, process groups, bounded stdio/stderr, deadline escalation,
  owner-loss cleanup and confirmed reap. Its compiled hostile-child
  qualification covers launch facts, saturation, graceful/forced exit and
  crashes. The binary remains a probe, not a semantic daemon or MCP endpoint.
- Added the production ordered-LSP composition over qualified supervised launch.
  It closes transport, LSP, semantic, session, service and scheduler bounds
  before process effects; linearizes first initialize admission against the
  supervised stdout pump; and retains exact prelaunch, transport, adapter,
  bridge and clock residues. The pump proof is not a claim that OS or
  ProcessKit buffers were physically empty.
- Added normalized affine semantic cleanup from unstarted, actor-build-refused
  and recovered runtimes through deadline retirement or explicit abandonment,
  client retirement, checked/forced/uninstalled extraction and a consuming
  aggregate provider/process terminal receipt. Launch timeout and unconfirmed
  cleanup now retain a recoverable affine worker/fence owner whose only closed
  resolutions are confirmed absence, confirmed cleanup or restart required.
- Added nonblocking affine actor-stop tickets. Stop admission fences commands,
  pending polls retain the ticket, refusal returns the exact draining owner,
  disconnection is outcome unknown, and explicit or implicit abandonment is
  instrumented without fabricating termination evidence.
- Added a same-spawn managed actor product that binds stop and recovery
  authority before execution. It retains bounded refusal history, distinguishes
  clean, outcome-unknown, abandoned and failed-before-stop paths, rejects
  incompatible recovery evidence, and carries the joined witness through
  normalized terminal cleanup.
- Added a compiled standalone LSP-process acceptance matrix over qualified
  supervision, framed stdio, fresh adapter startup, ordered session products,
  graceful cleanup and post-ready process failure. Readiness is injected; this
  does not claim production-language-server semantic-operation qualification.
- Extended the deterministic fixture tier through synchronized documents,
  nonempty diagnostics, nested symbols and versioned rename plans with exact
  bounded correlation and content-safe evidence. This qualifies protocol
  composition, not production-provider semantic quality.
- Added a typed LSP adapter profile joining provenance, framing and semantic
  bounds with one bounded initialization policy. Fixed workspace/edit
  capabilities, opaque bounded server identity, audited closed readiness
  notifications and strict response/notification/server-request separation are
  now qualified. Supervised launch derives the initialize root from the exact
  qualified workspace before process effects.
- Added an opaque pinned-provider protocol profile joining a nonzero profile
  digest, exact server identity and one audited readiness method. Identity
  absence or mismatch fails startup, notification-bound readiness cannot be
  promoted by independent polling, and the exact adapter's content-safe
  evidence and usage survive into provider/process terminal cleanup.
- Extended the managed same-spawn stop/recovery algebra to timed actors while
  retaining scheduler retirement and explicit compatible/mismatch disposition
  through terminal cleanup.
- Added an independent unpublished pinned rust-analyzer qualification package
  with a strict artifact/profile/fixture preflight and a complete live semantic
  receipt. The fixed 2026-11-09 macOS ARM64 provider proves notification-bound
  Current readiness, synchronized diagnostics, nested symbols, a versioned
  two-edit rename, clean timed-actor stop, compatible recovery, retired
  deadlines, checked provider extraction and graceful process closure.
- Added a closed rust-analyzer standalone initialization policy. It binds one
  canonical `src/lib.rs`, keeps filesystem path and LSP URI representations
  distinct, disables Cargo discovery, build scripts, proc macros, sysroot
  discovery and check-on-save, and exposes no raw provider JSON surface.
- Corrected LSP shutdown classification so a validated late notification cannot
  impersonate or invalidate the correlated shutdown response. Inbound server
  requests and malformed notifications remain fail-closed; only the exact null
  response sends `exit` and closes the transport.
- Pre-initialize stdout now carries an exact closed freshness witness, so later
  stdout cannot reclassify an unrelated startup refusal.
- Closed review findings around inverted semantic spans, split-scalar query
  offsets, binding rebind/retirement, recursively duplicated LSP JSON keys and
  drainable provider replacement.
- Added authenticated create-exclusive `ExportCopy` from non-forgeable affine
  codec capture through one atomic source/destination authority product,
  descriptor-backed destination object and real `RENAME_EXCL` publication.
  The exact operator policy binds canonical operation, session, document,
  revision and absolute UTF-8 destination; recovery canonically validates the
  durable record before replay. Lost-reply and pending crash cuts prove one
  publication or explicit `OutcomeUnknown`. This remains a copy: it is not Save
  As, overwrite, activation or document rebinding.
- Added a passive, fixed-size status request on the daemon's existing private
  control socket and a `vellum-daemon --status` client. It reports only closed
  lifecycle, health, readiness, aggregate utilization, and saturation facts;
  sampling does not execute a health probe or mutate readiness or counters.
- Added a transport-neutral authentication gate, an optional required local
  handshake composition, and an authenticated connector seam in
  `vellum-transport`/`vellum-sdk-client`. The daemon remains
  `OwnerOnlyOffline` by default and can explicitly select the local private-file
  credential profile. No Aegis adapter, remote handshake, confidentiality, or
  authorization grant is implied.
- Added a fixed-capacity authenticated local intent/receipt journal for the
  persistent daemon. It synchronizes exact effect intent before execution and
  exact replies before publication, binds recovered records to stable workspace
  and subject identity, and never silently evicts receipts. This is bounded local
  crash-recovery substrate, not distributed exactly-once delivery, durable audit,
  remote replay, or Hermes interoperability.
- Added a caller-chosen opaque operation reference and an authenticated,
  read-only cross-process receipt lookup for restart-stable membership, save,
  replace-and-save and create-exclusive copy-export intents. Pending work
  remains inspectable without automatic retry; stable receipts contain no
  session witness; operation
  records use a separately versioned, subject-namespaced v2 local format and
  share the daemon's fixed retained-journal capacity. Superseded v1 state
  refuses without an explicit migration. Journal enumeration, reads, creation,
  and synchronization are descriptor-relative; duplicate composite keys and
  namespace replacement fail closed.
- Added authenticated-only MCP tools for durable execute, pure receipt lookup,
  and exact durable receipt retirement. The harness persists a canonical nonzero 128-bit operation
  reference before the first effect; exact intent conflicts, pending outcomes,
  absence/foreign ownership, and unavailability remain distinct. A real
  daemon/MCP restart test loses the first response and retrieves the historical
  receipt with no second effect. Create-exclusive export is a seventh intent in
  those existing tools, not another tool; its receipt omits destination and
  source and is bound to the exact document/revision. The shipped MCP CLI
  selects this surface only with an explicit descriptor-anchored local
  credential; omission remains the owner-only profile. The export destination
  remains private plaintext in the canonical intent/tombstone and is not
  claimed unlinkable or erased from backups. Retirement requires the identical
  stable intent, calls only the native retirement operation, and preserves the
  daemon's independent `RetireReceipt` authority requirement.
- Added content-free active durable-receipt and monotone retired-identity
  occupancy, capacity, and saturation observations. Native receipt retirement
  now requires fresh authentication plus the distinct global `RetireReceipt`
  ceiling for a complete mutation candidate, commits a permanent exact-intent
  tombstone before cleanup, recovers every validated post-commit subset, and
  makes delayed exact execution terminate as `Retired` without a second effect.
  Authenticated MCP receipt retirement now projects that freshly authenticated,
  separately authorized, crash-safe operation without granting its authority;
  an additive external status projection remains deferred.
  manual journal deletion and implicit pressure eviction are unsupported.
- Added an offline coordinated-source cargo-install qualifier with fresh Cargo,
  install, and target roots; exact daemon/MCP binary closure; content-safe
  startup probes; and adversarial path, symlink, mode, and ambient-environment
  fixtures. Actual GitHub-source readiness remains fail-closed on the recorded
  Keystone and offline dependency-source blockers.
- Added the nonpublishing `vellum` source-install package. One Cargo invocation
  now installs the independently owned `vellum-daemon` and `vellum-mcp`
  executables; the package contains no runtime policy or adapter selection.
  The equivalent `cargo install --git ... --rev RELEASE_COMMIT vellum` command remains
  blocked only on immutable Vellum/Keystone source revisions and remote source
  availability, not on multi-package installation shape.
- Added inert, checksum-closed GitHub release-candidate and launchd-rendering
  machinery for the daemon and MCP binaries. It performs no installation,
  activation, signing, attestation, GitHub Release publication, or crates.io
  publication; TUI and GUI applications remain independent release units.
- Extended local candidate qualification through both packaged launchd
  renderers: the candidate and its selected rollback version must each render
  the exact inert direct-exec plist from fresh private scratch state and pass
  `plutil`. Evidence retains only closed smoke dispositions and grants no
  installation, activation, socket-activation or authenticated-health claim.

### Security

- Added a private-reporting policy and a fail-closed release-contract check.
- Added explicit STRIDE/LINDDUN boundaries, authority separation, bounded
  RED/USE and golden-signal observations, content-safe receipts, and
  fail-closed ownership/type/mode/link/identity validation for persistent local
  state and Unix endpoints.
- Kept every workspace package `publish = false`. The supported work remains
  GitHub-first source qualification. Keystone now resolves from one exact
  qualified Git revision with zero source-SBOM blockers. Remote availability
  of the Vellum release commit and pinned Keystone object, plus a genuinely
  offline dependency bundle, remain unclaimed.

[Unreleased]: https://github.com/gantbd/vellum/commits/main
