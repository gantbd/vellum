# Vellum production and semantic tooling roadmap

Status: local working roadmap, deliberately untracked

Date: 2026-11-10

## Purpose

This roadmap takes Vellum from a rigorous editing engine to a production-ready service that humans, agents, and composite applications can use through independent projections.

The organizing rule is categorical closure: each component has a bounded authority, explicit algebra, lawful composition, and observable failure semantics. Effects cross named boundaries. State changes carry witnesses. Partial or uncertain outcomes are represented rather than hidden.

## Execution ledger

This ledger records implemented, verified slices without treating a partially completed phase as complete.

### 2026-11-10 — first parallel wave

- **Release contract:** implemented a fail-closed no-crates.io gate, pinned toolchain, release/security/change policies, dual license files, and a lone-checkout GitHub policy workflow. Full lone-checkout compilation still requires an immutable Keystone distribution decision.
- **Security and privacy:** added a concrete STRIDE/LINDDUN model with assets, actors, eight trust boundaries, explicit residual risks, and cumulative source/local-daemon/Aegis/semantic-peer/remote release gates. Remote operation remains unauthorized.
- **Workspace foundation:** implemented empty workspace startup, live-process workspace/document/root identities, bounded file/directory admission, content-free inventory, and foreign-identity refusal. Durable/restart-stable protocol identities and complete service commands remain outstanding.
- **Workspace discovery:** moved the discovery port to `vellum-host`, added descriptor-anchored `openat`/no-follow local traversal with deterministic ordering and explicit entry/file/depth/path bounds, and marked returned locators as requiring revalidation. Refusals publish no partial path set.
- **Native inventory:** added a bounded, content-free native inventory command with canonical wire fixtures; empty workspaces accept clients and return `no_active_document` for document commands. Workspace/document/root protocol identities are non-reused only within the live process and are not restart-stable.
- **Service replay:** made the bounded request register's `new`, `matching_replay`, `changed_input_conflict`, and `capacity_full` outcomes explicit and separately observable, with no implicit eviction.
- **Transport truthfulness:** distinguished requests for which no byte was sent from requests whose outcome became unknown after transmission began; added phase-specific content-safe counters and fault tests.
- **Daemon ownership:** added an owner/mode-verified private endpoint, persistent no-follow lifetime lock, lock/socket identity revalidation before acceptance, fencing after replacement, and exact-identity socket cleanup. Automatic stale-socket deletion remains deliberately absent.
- **Keystone observations:** implemented and tested shared sticky saturating counter/duration primitives with the explicit law that overflow clamps to the maximum and remains sticky. Consumer adoption and qualification remain outstanding.

### 2026-11-10 — second parallel wave

- **Native workspace commands:** added bounded native add-file, add-directory, and select-document commands across protocol, canonical wire, client, and service. Scoped document/root identities are projected through monotone native-to-protocol maps rather than collection ordinals; refused additions publish no partial inventory.
- **Durable ordinary save:** added exact-witness save for the active document with the adapter's application, synchronization, quiescence, no-future-effects, failure, cleanup, baseline-publication, and unknown-outcome facts. External changes do not become writes or false baselines. Save As and retained-save-task reclamation remain outstanding.
- **Client uncertainty fence:** the native client retains one exact request after `outcome_unknown`, permits only same-command replay under the same request identity, fences different commands and disconnect, and treats transport extraction as explicit abandonment. It performs no hidden retry.
- **Daemon composition:** the installed daemon now starts empty or from bounded file/directory roots, injects the qualified descriptor-anchored local discovery adapter, and refuses symlink/special startup roots without following them.
- **Daemon lifecycle and observations:** added closed running/draining/stopped/fenced transitions, bounded admitted obligations, readiness/health, passive content-free USE/RED/golden-signal snapshots, and ownership revalidation at active lifecycle transitions. Signal/control wakeup, latency histograms, and externally operable drain remain outstanding.
- **Coordinated GitHub source:** added an exact-revision two-repository Vellum/Keystone archive qualifier. The qualifier distinguishes locked workspace-build evidence from path-install smoke evidence and refuses registry publication or inclusion of TUI/GUI packages.
- **Release-candidate artifacts:** added a manual, read-only, commit-pinned macOS workflow that builds only the daemon and MCP peer, emits a content-safe manifest and SHA-256 checksums, and uploads unsigned/unattested artifacts. It does not create a GitHub Release or publish to crates.io; real execution awaits committed clean repositories and remote availability of the pinned Keystone revision.

### 2026-11-10 — third parallel wave

- **Save-task reclamation:** ordinary saves now retain unknown, nonterminal, contested and current-baseline tasks while reclaiming only terminal non-baselines and baselines superseded by a newer published baseline. A two-slot engine profile survives repeated edit/save cycles without hiding retirement failure or manufacturing capacity.
- **Closed service authority:** `vellum-host` owns a nine-action finite bounded lattice and exact request-bound decision evidence. Service defaults are read-only; trusted compositions may install an explicit ceiling or supplied authority port/context/resource. Every fresh command is mediated before editor, witness, discovery or persistence effects; denied, unavailable and mismatched evidence fail closed, while exact replay reuses the retained reply without reauthorization. Multi-principal authentication, dynamic revocation and Aegis remain open.
- **Save As safety boundary:** no native Save As effect was exposed. Existing-destination replacement plus rebinding requires an explicitly approved destination-scoped overwrite policy, independent Persist and Rebind decisions, and typed provenance for every trusted export/activation fact. A later local export substrate now implements create-exclusive publication, but it is not yet bridged to codec-owned source preparation, the service/protocol, durable receipts, or daemon destination policy; `--allow-save` remains Persist-only.
- **Externally operable shutdown:** a separate owner-only control socket accepts one fixed shutdown request, sets a monotone flag and wakes blocked data acceptance without counting the wake as service traffic. The daemon then projects Running to Draining to Stopped through a Keystone obligation ledger. Crash residue is still refused because predecessor endpoint identity is not authenticated; SIGTERM/launchd integration therefore remains open.
- **Read-only daemon composition:** the default daemon installs the read-only action ceiling and no runtime discovery adapter. Independent CLI ceilings opt into file admission, directory discovery, editing or persistence; directory discovery installs its adapter only when selected. These flags are static same-OS-user composition, not principal authentication or delegation.
- **MCP extraction boundary:** MCP production code and portable conformance tests now consume only `vellum-sdk-client`; daemon-coupled qualification is explicitly retained by Vellum. A machine extraction contract rejects symlinks, non-regular files and traversal, archives the declared peer payload, builds/tests it, path-installs it and smokes the binary. Single-URL Git installation remains blocked by immutable Keystone source closure; no sibling repository or remote was created.
- **Keystone observations:** Vellum adopts a Keystone fixed-duration histogram with closed ordered bounds, overflow, saturating population/sum and sticky whole-distribution fidelity loss. Daemon latency covers only admitted request handling and excludes blocked accept, startup, control, drain and pre-admission refusal. Optional observation remains passive and non-authoritative.
- **Keystone replay and lifecycle:** a generic replay register now captures the common Vellum/Agora four-outcome relation (`new`, `matching_replay`, `conflict`, `full`) with existing-key lookup before capacity, fallible full preallocation and no eviction or retry policy. Vellum preserves its exact counters and disconnect-scoped removal. The serialized daemon composes `keystone-lifecycle::ObligationLedger` without moving Vellum lifecycle, fencing or shutdown consequences into Keystone.
- **Effect-boundary closure:** an exact mechanism-specific source gate now enumerates recognized production filesystem, local-network, process-launch and native-FFI callers. New callers and stale trusted members both fail; adversarial mutants exercise every recognized mechanism. The gate explicitly does not claim total confinement from textual scanning.
- **Threat-model reconciliation:** STRIDE/LINDDUN claims now distinguish implemented static request/action mediation, lifetime ownership and graceful control from still-open principal authentication, revocation, durable audit, signal supervision and authenticated crash-residue recovery.
- **Offline supply-chain closure:** a deterministic dependency policy and CycloneDX 1.5 source inventory reconcile all 86 locked components, reject Git/unknown registry/unlicensed/checksumless/escaped or undeclared path inputs, and exercise adversarial fixtures without downloading tools. Coordinated GitHub CI is commit-pinned and read-only. Release remains fail-closed because four Keystone crates across five manifest edges are external sibling paths, `keystone-replay` is not present at the pinned Keystone revision, and the pinned `keystone-observe` manifest lacks the histogram/saturation API Vellum actually compiles.
- **Compatibility baseline:** a canonical offline baseline now freezes workspace/package metadata, package features and targets, conservative library-source tree digests, and the complete current wire-golden inventory. The fail-closed checker rejects drift, links, escapes, malformed evidence and six adversarial mutants. This is an intentionally over-sensitive change detector, not Rust semver analysis or exhaustive wire-compatibility proof.
- **Unix ownership reuse audit:** Vellum, Chronos, Moira and Keystone were compared at the owner/mode/type/link/no-follow/device/inode laws. Vellum and Keystone have a plausible pinned-resource identity intersection, but creation, stale cleanup, symlink and replacement policies are not categorically equal and Keystone checkpoint work is still in flight. No generic guard was extracted; Vellum instead added adversarial parent/control symlink, stale control, replacement-socket and exact-cleanup evidence, with a policy-free extraction handoff recorded for later.
- **Integrated qualification:** the combined workspace passes formatting, warnings-denied Clippy, the full all-feature/all-target offline test suite, warnings-denied documentation, MCP extraction, effect/project/release/supply-chain gates, and focused Keystone codec/observation/lifecycle/replay tests. A pre-existing bare Unicode specification URL was corrected to close the documentation gate.
- **Structured operational events:** the daemon now has a closed, content-free lifecycle/request/refusal event algebra with process-local correlation, monotone sequence evidence, a fixed-schema stderr sink, and passive failure/panic containment. Real exchanges and lifecycle transitions are unchanged when the sink refuses or unwinds. This is neither durable audit, exporter delivery, span propagation nor distributed tracing, and its coarse activity leakage is documented.
- **Restart identity issuance:** a host-owned, generation-checked metadata contract persists workspace scope plus separate exact document/root issued prefixes and reservation ceilings through a private local-filesystem adapter. Recovery allocates at issued plus one, never at the reservation ceiling; removal preserves each issued mark. Missing is initial only when this startup created the persistent lock, while v1/v2, deletion after an earlier run, malformed/conflicting state, holes in a newly issued tail, and unknown commit outcomes fail closed before publication.
- **Workspace view lifecycle:** native protocol, wire, client and service now expose explicit idempotent open/close of per-client selection state under a read-only `ManageView` action. Outcomes distinguish opened/already-open and closed/already-closed, exact replay does not repeat transitions, scoped document identities remain stable, and passive counters contain no content. Document/root removal remains deliberately absent because append-only slot identities, retained views/history/witnesses/save tasks, missing disposal receipts and missing directory-membership provenance do not yet close a truthful cleanup contract.
- **Offline artifact lifecycle:** schema-2 candidate manifests now capture effective build configuration and hashes of lock, toolchain, compatibility, dependency, source-policy and SBOM inputs. Local gates reject corruption, extras, links, incompatible schema/target/policy, failed smoke and self-rollback, and require a distinct verified prior candidate. Readiness and rollback evidence are canonical, while `activationAuthorized` remains false: no external activation, release, signing, attestation or publication occurred.

### 2026-11-10 — fourth parallel wave

- **Authentication handoff seam:** `vellum-host` now owns a transport-neutral authentication port whose opaque challenge, claim, subject and session evidence is exactly request-bound and categorically separate from action authority. Bounded consultation, closed authenticated/rejected/unavailable outcomes and a parameter-free authority-context handoff prevent evidence transplantation and subject substitution. The local daemon still composes no authenticator; Aegis integration, credential exchange, expiry, revocation and remote authentication remain open.
- **Bounded inventory pagination:** editor, protocol, canonical wire, client and service now expose an explicit document-then-root inventory page with a conservative hard V1 maximum of two entries, chosen from the larger root representation so a page plus continuation fits the canonical sixteen-field wire profile. Its opaque workspace/generation/offset cursor uses structural generation independent of source revision, with a terminal `next: None`. Foreign, stale, invalid and oversized cursor requests fail distinctly before projection; exact replay neither reauthorizes nor changes pagination counters. The original full-inventory command remains compatible, while older exhaustive decoders must upgrade for the additive command/outcome and refusal tags.
- **Overload and deadline evidence:** accepted Unix streams now report exact OS timeout exhaustion separately from generic request failure, while admission-capacity refusal and latency-overflow evidence remain bounded and content-free. Synchronized behavior tests cover blocked-accept exclusion and a silent admitted client through deadline, readiness restoration and drain completion. The serialized driver still cannot preempt an admitted exchange, kernel listen backlog is not daemon admission, and a continuously progressing peer has no total wall-clock deadline.
- **Integrated fourth-wave qualification:** after pagination, authentication and overload reconciliation, the full all-feature/all-target offline workspace suite passes, including daemon Unix-socket behavior, MCP live-daemon qualification, all 23 service scenarios and canonical page fixtures. Formatting and diff hygiene also pass; compatibility, release and supply evidence must be refreshed and rerun after the in-flight fifth-wave schema work settles.

### 2026-11-10 — fifth parallel wave

- **Durable startup resource bindings:** the private bounded v3 workspace-identity record binds exact locator bytes and resource kinds to document/root protocol ordinals. Every live binding is bounded by its sort's issued prefix, which is bounded by its reservation; a newly advanced contiguous tail must be present in the candidate bindings. Recovery reconstructs projections from those bindings rather than inventory order. Same-name/same-kind replacement intentionally has locator semantics; v1/v2 need explicit migration and unknown commits need operator recovery.
- **Workspace membership retirement:** additive document/root removal commands now use a distinct `RetireMembership` authority and bounded non-reused tombstones. Active selected documents refuse, exact known replays distinguish `AlreadyRemoved` from unknown identities, document witness records are revoked, roots detach without an invented document cascade, and the public receipt omits cross-client witness cardinality. This retires workspace membership only: native source, file content, history, save tasks and provenance remain retained until their owners can prove truthful reclamation.
- **Signal supervision and conservative crash recovery:** the standalone daemon maps SIGINT/SIGTERM through its existing owner-only control socket, preserving the same ownership checks, wakeup and drain algebra. Persistent exact data/control endpoint evidence permits recovery only while holding the exclusive private lock, after type/owner/mode/link/inode checks, a specifically refused connect probe and an immediate second identity check; malformed, mismatched, live and unrecorded endpoints are never removed. A real SIGKILL→restart→SIGTERM scenario covers both endpoint recovery and graceful final cleanup. Same-user compromise, crash-short-written evidence, SIGKILL drain, launchd socket activation and authenticated readiness remain nonclaims.
- **Supply reconciliation:** the minimal safe signal iterator is pinned with default features disabled; its `Apache-2.0/MIT` declaration is normalized explicitly. The deterministic inventory now closes over 88 locked components and retains exactly two known Keystone release blockers.
- **Integrated fifth-wave qualification:** formatting, diff hygiene, warnings-denied Clippy, warnings-denied rustdoc, the complete all-feature/all-target offline suite, 25 daemon integrations, 25 service scenarios, 12 canonical wire scenarios, MCP extraction, release/artifact/source qualifiers and supply-chain mutants pass together. Compatibility evidence was regenerated after review of the additive vocabulary and private-format changes.

### 2026-11-10 — sixth parallel wave

- **Fail-closed dynamic identity boundary:** until a staged membership transition can durably checkpoint its exact binding before an accepted reply, the persistent standalone daemon rejects any action ceiling containing `AdmitFile`, `DiscoverDirectory` or `RetireMembership` before acquiring its lock or touching metadata/socket state. The CLI no longer advertises those unusable flags. Startup roots remain supported because they commit before publication, while explicitly transient embedded daemon/service compositions retain runtime membership semantics without a restart-stability claim.
- **Deterministic fuzz and soak qualification:** an offline opt-in gate now runs fixed-seed corpus mutation over canonical request/reply decoding, bounded outer-frame and one-exchange properties, repeated exact replay/pagination/removal lifecycles, and actual daemon connections carrying valid, malformed, oversized and truncated frames. Default evidence covers 4,096 wire mutations, 2,048 transport cases, 64 service cycles and 16 daemon cycles; budgets have explicit maxima, failures remain content-safe, ordinary tests do not run the loops, and a manual-only GitHub workflow preserves the gate in the coordinated source archive. This is not coverage-guided fuzzing, exhaustive state exploration, a concurrency proof, a latency benchmark or a durability/power-loss test.
- **Integrated sixth-wave qualification:** the complete locked, offline, all-feature/all-target suite passes with the dynamic membership refusal boundary and opt-in qualification targets present. Formatting, diff hygiene, warnings-denied Clippy and rustdoc, project/effect boundaries, MCP extraction, GitHub-source/release/artifact qualifiers, and the supply gate pass together. The refreshed conservative baseline closes over 17 packages and 21 exact wire fixtures, rejects all six compatibility mutants, and the fixed-seed qualification completes all four bounded campaigns.

### 2026-11-10 — seventh parallel wave

- **Bounded revisioned change feed:** protocol, wire, client and service now expose a read-only pull feed of content-free document/root membership and document-change facts. V1 returns at most one worst-case event per page under the sixteen-field wire ceiling, binds cursors to workspace and a restart-advancing epoch, reports retention loss as an accepted `Gap`, and distinguishes foreign from invalid positions. Publication is preflighted before a fresh mutation and occurs only after success; replay, saves, views, selections, no-ops and failures are inert. This is not push delivery, a durable outbox, audit, external-writer observation, exactly-once delivery or a Hermes implementation.
- **Local authentication composition:** the daemon can optionally require local handshake protocol V1 before decoding a Vellum frame, using the existing transport-neutral host gate, an OS-CSPRNG incarnation, exact challenge/session/claim binding, one-exchange session lifetime and a single coarse external refusal. Owner-only offline remains the shipped default. Authentication evidence is not action authority and the handshake supplies no confidentiality, mutual same-user isolation, credential protocol, remote/server authentication or authenticated health claim.
- **Semantic peer contract:** `vellum-host` and the embedded SDK now expose a bounded outbound seven-query algebra with exact request, Vellum source-revision and peer index-generation witnesses; a finite read/rename-plan capability lattice; closed stale/unsupported/unavailable/refused outcomes; and passive content-safe observations. Rename is a plan only. No Vellum wire command, provider, LSP/PSI/nREPL adapter, process launch, MCP projection, edit authority or atomicity claim was added.
- **Integrated seventh-wave qualification:** focused service, wire, client, MCP, daemon, host and embedded-SDK suites pass with warnings denied. Two new exact change-feed fixtures bring the coordinated compatibility inventory to 23 before the following wave. Deterministic multi-client schedules cover selection isolation, stale first-writer-wins behavior, inert replay, request conflicts, feed continuation and content-safe evidence without claiming thread-parallel execution, distributed linearizability, remote isolation or durable delivery.

### 2026-11-10 — eighth parallel wave

- **Explicit durable edit attempt:** additive `ReplaceAndSave` preauthorizes independent Edit and Persist actions before mutation, performs the ordinary exact-witness replacement, and then reports a neutral `ReplaceAndSaveReceiptV1` whose persistence sum is `Durable`, `NotDurable`, `Unknown` or `NotAttempted`. A successful edit is never erased by a later save refusal or uncertain adapter outcome; exact replay does not repeat either effect. External-writer supersession remains intact, and only synchronized application plus exact baseline publication earns `Durable`.
- **Inert launchd distribution profile:** a checksum-closed release payload now contains a deterministic LaunchAgent renderer that emits XML to stdout only. Exact schema, canonical no-symlink config/binary/state paths, owner/mode/type rules, fixed private children, absent endpoints, direct read-only daemon arguments and `077` umask fail closed under adversarial qualification. Release-candidate qualification now executes the exact packaged renderer for both candidate and selected rollback daemon under fresh private scratch state, requires the exact inert output and `plutil` acceptance, and retains only content-free dispositions. It performs no installation, activation, `launchctl`, socket activation, health authentication, signing, attestation or publication; candidate schema 3 records those denials.
- **Multi-client protocol qualification:** deterministic bounded schedules exercise human, agent and observer clients across local views, shared revisions, replacement, persistence, replay, conflicting request identities, read-only authority and feed polling. Client request namespaces and views remain isolated while source state and revision order remain shared. The serialized service still makes no thread-parallel or remote-transport isolation claim.
- **Integrated eighth-wave qualification:** the locked offline all-feature/all-target workspace suite passes with 29 daemon integrations, 36 service scenarios and 15 canonical wire scenarios. Formatting, diff hygiene, warnings-denied Clippy/rustdoc, launchd mutants, project/effect boundaries, release/artifact/GitHub-source/MCP extraction and the 88-component supply gate pass. The conservative compatibility baseline closes over 17 packages and 25 exact wire fixtures and rejects all six mutants. The fixed-seed gate also passes all 4,096 wire mutations, 2,048 transport cases, 64 service cycles and 16 real daemon cycles after waiting non-destructively for a concurrent Cargo artifact lock.

### 2026-11-10 — ninth parallel wave

- **Transactional runtime membership checkpoint:** the persistent daemon now retains its identity metadata port and places a checkpointing service boundary between fresh membership mutation and wire reply. Successful file/directory admission and document/root retirement project the complete exact active binding set, non-reused high-water reservations and advanced generation, then durably store it before the reply can escape. Exact replay skips the store; authority denial never reaches it. Candidate projection failure, known store refusal, capacity failure and unknown commit outcome all suppress the semantic reply, record content-free evidence and fence the daemon without claiming rollback. Restart tests cover exact discovered identities and retirement followed by non-reusing re-admission. Read-only remains default; explicit runtime membership flags are usable again.
- **Sovereign semantic service/client substrate:** new `vellum-semantic-service` and `vellum-semantic-client` crates raise the workspace to nineteen packages. The service accepts exact bounded immutable snapshots, enforces monotone source revisions, tracks injected provider readiness/capabilities/index generation, bounds pending work and delivery, cancels or marks stale work when snapshots or provider generations advance, routes out-of-order completions only to their owning client and exposes content-safe observations. The zero-authority client fixes a correlation partition but grants nothing. No provider process, LSP correctness, transport, retry, authentication, MCP tool, edit application or workspace-trust decision is claimed.
- **Offline cargo-install qualification:** the coordinated source qualifier now installs only the intended daemon and extracted MCP packages through `cargo install --locked --offline --path` under fresh private Cargo home/install/target roots, strips ambient Cargo/Rust/compiler overrides, checks exact executable closure and modes, and runs self-identifying/content-safe smokes. Adversarial fixtures reject missing, wrong, extra, writable, symlinked and escaped binaries. Synthetic path-only and coordinated local smokes pass; the real lone-archive path remains explicitly blocked by immutable Keystone source closure and an empty-cache registry-source gap rather than borrowing the developer environment.
- **Integrated ninth-wave qualification:** formatting, diff hygiene, warnings-denied Clippy/rustdoc, the complete locked offline all-feature/all-target suite, 36 daemon integrations, 36 Vellum service integrations, semantic service/client tests, project/effect boundaries, cargo-install/launchd/release/artifact/GitHub-source/MCP extraction and supply mutants pass together. The refreshed conservative evidence closes over 19 packages, 25 wire fixtures and 90 locked components, with the same two explicit Keystone blockers and all six compatibility mutants rejected. Fixed-seed qualification again passes 4,096 wire mutations, 2,048 transport cases, 64 service cycles and 16 real daemon cycles.

### 2026-11-10 — tenth parallel wave

- **Useful modern MCP workspace surface:** the MCP peer now implements the stateless `2026-07-28` lifecycle through `server/discover`, `tools/list`, `tools/call`, `resources/list` and `resources/read`; the removed legacy `initialize` handshake is rejected. Content-free paginated resources expose only canonical scoped document/root identities, document reads select the exact identity inside the same request, and root reads revalidate membership and kind against bounded native inventory rather than trusting URI claims. Five closed tools cover explicit-document snapshot, revision-witnessed replace, replace-and-save with the full persistence sum, bounded change polling with opaque cursors and explicit gaps, and authority-mediated membership. Missing resources use the current `-32602` contract, while native refusals and access failures remain closed and distinct. No paths, witnesses or authority are projected.
- **MCP uncertainty continuity:** every document read/edit carries workspace and document identity rather than relying on an earlier MCP selection. If select, snapshot or `ReplaceAndSave` loses its native reply, the connector returns one opaque process-lifetime operation handle and permits only the identical argument tuple to resume the exact retained native request; it never takes a second snapshot or manufactures a new edit. This state is bounded to the one supervised stdio process and is not a durable MCP session, remote idempotency key or cross-process recovery claim.
- **Authenticated durable effect receipts:** the persistent daemon composes a separate Vellum-owned fixed-capacity intent/receipt journal for membership, `Save` and `ReplaceAndSave`. It synchronizes the exact canonical intent before execution, checkpoints accepted membership before receipt, synchronizes the exact reply before transport publication, reserves all recovered client ordinals before endpoint publication, and returns the historical reply without repeating the effect. Pending, corrupt, unsafe, conflicting, future-generation, write-refused and indeterminate state refuses or fences rather than guessing. Exact restart tests cover membership and source-bearing durable edit receipts without a duplicate filesystem write.
- **Receipt identity and privacy closure:** durable creation and retrieval require the optional required-authentication profile and bind only its stable subject reference; a later authenticated session for that subject may retrieve the exact historical receipt without requiring the retired service client to be active. Another subject receives the same inactive-client outcome as absence, and new clients cannot inherit old ordinals. This proves disclosure continuity, not fresh action authorization. `OwnerOnlyOffline` creates no durable receipts; the explicit local-credential CLI composition now exposes this replay surface without granting fresh action authority. Records are bounded private plaintext and may contain locators, inserted source and returned snapshots; there is no encryption, signing, dynamic-policy revalidation, eviction, compaction, expiry, deletion or operator migration contract.
- **Receipt lifecycle production gate:** native receipt retirement now targets the server-bound workspace, authenticated subject, caller operation reference and exact canonical intent. A permanent descriptor-anchored tombstone absorbs delayed exact execution as `Retired`, preserves changed-intent conflict, and releases active receipt capacity only after validated cleanup synchronization. Retired identities consume a separate bounded monotone capacity and never regain meaning; pending intent, malformed/duplicate storage, unsafe resources and indeterminate cleanup fence. Authentication and the distinct global `RetireReceipt` ceiling are conjunctive for complete mutation candidates. The authenticated MCP catalog now projects the exact operation and closed outcomes without granting that authority or falling through to lookup/execute. Adapter-only deletion, age/pressure eviction, Hermes acknowledgement and directory replacement remain rejected. An additive external status projection remains deferred.
- **Hermes/Keystone law audit:** direct substitution of Hermes for either Vellum relation was rejected. Vellum's volatile register is a disconnect-retired, authority-bound `(ClientId, RequestId) -> exact RequestV1 + ReplyV1` relation; Hermes retains producer/idempotency/envelope-to-sequence facts without session retirement, and its consumer acknowledgement proves cursor advancement rather than downstream effect completion. Vellum's evicting change feed also has explicit gap and restart-epoch laws that Hermes's current no-compaction stream does not. Hermes remains a candidate only for a separately specified Vellum outbox/operation identity. Keystone checkpoint storage may supply crash-safe mechanism after immutable revision closure, but not Vellum schema, authority or replay meaning.
- **Integrated tenth-wave qualification:** the combined locked offline workspace passes formatting, diff hygiene, warnings-denied all-target Clippy and rustdoc, the full Unix-socket-enabled workspace test suite, 40 daemon integrations, 23 portable MCP conformance scenarios plus one real-daemon MCP scenario, project/effect/release/artifact/launchd/MCP-extraction/compatibility/supply gates, and direct locked offline path installation of the daemon and MCP binaries from the current working tree. The fixed-seed gate again passes 4,096 wire mutations, 2,048 transport cases, 64 service cycles and 16 real daemon cycles. Conservative evidence remains 19 packages, 25 exact wire fixtures and 90 locked components with exactly the same two Keystone release blockers; immutable lone-GitHub-source installation and a fresh empty-cache registry source remain unclosed, and no remote, publication, activation, commit or push occurred.

### 2026-11-10 — eleventh parallel wave

- **Restart-stable operation identity:** a fixed opaque 128-bit operation reference and separate closed durable intent algebra now exclude transport client/request IDs, selected views and session witnesses. Membership carries its exact locator/target; save and replace-and-save carry an explicit scoped document and source revision. Stable historical receipts contain no live `VersionWitness`.
- **Pure receipt lookup:** native protocol, canonical wire tags, transport and client expose distinct durable execute and read-only lookup paths. Complete, request-conflict, pending, absent-or-foreign and unavailable are total closed outcomes. Lookup has no service execution port and cannot authorize, create an intent, mutate membership/editor/filesystem state or fall through on absence.
- **Versioned persistence and authenticated daemon composition:** the subject-namespaced `.vellum-durable-operation-v2` relation coexists with the legacy client/request journal without reinterpretation. Presence of the superseded operation-v1 directory refuses startup and writes until an explicit migration is provided. Pending operation intents recover as `OutcomeUnknown`; complete records validate exact intent/receipt correspondence at startup. Required-auth subject and restart-stable workspace scope bind disclosure. One shared daemon capacity covers both active relations and pending slots, with content-free aggregate utilization and fixed-cardinality RED counters.
- **Lost-reply restart evidence:** real Unix-socket qualification deliberately discards the durable execute reply after the daemon completes the effect, terminates the client thread and daemon, writes an external sentinel, restarts with a different transport identity, and retrieves the stable receipt without a second service effect or filesystem write. Client-side uncertainty separately fences different operations/intents and disconnect until exact execute or lookup becomes definitive.
- **Authenticated MCP projection:** the MCP library adds separate durable execute, pure receipt-lookup and exact receipt-retirement tools only when constructed over the authenticated connector. The harness supplies and persists a canonical 128-bit `vdo1` reference before the first write; real daemon/MCP restart evidence through the production local credential adapter retrieves the historical receipt with zero second effects. Retirement requires identical intent and delegates only to the native retirement operation; the daemon's separate `RetireReceipt` ceiling remains authoritative. Current `vo1` remains a one-process uncertainty handle. The shipped binary now selects cross-process resume only with an explicit credential path; omission remains owner-only.
- **Issued-prefix receipt closure:** v3 exact issued marks now validate recovered ordinary and durable intents/outcomes for the correct resource sort, including historical removed identities, retired operations, optional file roots and every directory-admission document. Legacy full-inventory admission replies and v1/v2 identity records refuse. Mutually hostile same-UID writers still need one enforced shared lock or atomically versioned namespace to exclude transient version-name races; owner-only permissions are not relabeled as that proof.
- **Integrated eleventh-wave qualification:** formatting and diff hygiene, warnings-denied all-target/all-feature Clippy and rustdoc, the complete locked/offline Unix-socket workspace suite, doc tests, three repeated parallel MCP restart runs, portable Unicode 18 conformance, project/effect/release/artifact/launchd/MCP-extraction/compatibility/supply gates and all compatibility/supply mutants pass together. Fixed-seed qualification also passes 4,096 wire mutations, 2,048 transport cases, 64 service cycles and 16 real daemon cycles. Refreshed conservative evidence closes over 19 packages, 27 exact wire fixtures and 90 locked components. The source SBOM retains exactly two explicit source-identity blockers, while the coordinated GitHub install has a third blocker because pinned `keystone-observe` lacks the public histogram/saturation API despite its manifest being present. The full ICU reference target still requires host ICU configuration; no remote, publication, activation, commit or push occurred.

### 2026-11-10 — twelfth-wave export precursor review

- **Create-exclusive protocol promise:** the durable protocol and canonical wire
  reserve a create-exclusive-only export intent and a locator/content-free
  checked receipt. The decoder rejects impossible persistence/failure states.
  No service interpreter, daemon composition, SDK convenience, or MCP tool is
  installed, so this schema reservation does not execute an export.
- **Fail-closed provenance review:** an independent hostile review found that
  the original precursor accepted caller-manufactured source witnesses and raw
  bytes, borrowed the too-broad `PersistSource` action, spent two authority
  decisions sequentially, and authorized a lexical path without proving it was
  congruent with the pinned directory object receiving publication. These are
  release blockers, not deferred refinements.
- **Required substrate correction:** exposure now requires a non-forgeable
  one-use codec capture, a distinct `ExportSourceCopy` action, one atomic
  source/destination authority product, authenticated subject/session and exact
  document/revision congruence, a host-installed directory object capability,
  closed receipt recovery, bounded operational projections, and deterministic
  namespace-race plus crash-cut qualification. Until all factors close
  together, the physically qualified `RENAME_EXCL` writer remains private.

### 2026-11-10 — thirteenth-wave export closure qualified

- **Non-forgeable source and atomic authority:** codec-owned affine capture now
  binds the exact native document incarnation, revision and serialization. The
  distinct `ExportSourceCopy` decision retains that complete question and is
  granted atomically with an exact create-exclusive destination object before
  any bytes are disclosed; generic authority budgets are not partially spent.
- **Pinned object/effect closure:** the host-owned affine effect port carries a
  destination object installed for the exact context, resource, locator and
  effect. Planning checks that congruence before capture, and local-fs again
  checks the private object type and full request before publishing against its
  pinned directory descriptor. Ancestor rename/swap and same-leaf/different-
  parent substitution cannot redirect publication.
- **Recovery and embedded service closure:** the durable recovery validator
  checks canonical bytes, owner scope, authenticated subject, workspace
  generation, issued prefixes, receipt sort, document and revision. A real
  local-journal crash cut distinguishes pending/unknown from exact completion.
  The opt-in authenticated service interpreter maps the protocol document to
  the exact native version, bypasses generic sequential authority, executes
  only through the injected effect port and projects content-free counters and
  checked receipts. Ordinary/synthetic service constructors refuse export.
- **Daemon and recovery closure:** an exact operator-installed policy fixes the
  canonical `vdo1` operation, document, revision and absolute UTF-8 destination
  before endpoint publication. The required-auth daemon consumes the fresh
  affine session, journals before execution, installs the descriptor-backed
  create-exclusive destination object and executes the real local-fs effect.
  Startup delegates canonical bytes, owner/scope, generation, issued-prefix
  and intent/receipt validation to the host recovery algebra. A hostile
  future-generation record refuses before socket publication.
- **Crash and MCP closure:** real Unix-socket qualification discards a completed
  export reply, restarts the daemon and retrieves the historical receipt with
  no second effect or publication; pending export remains `OutcomeUnknown`.
  The authenticated MCP catalog extends its existing durable
  execute/lookup/retirement intent union rather than adding an export-specific
  tool. Exact intent matching includes the
  destination, while the checked receipt, discovery, Debug, errors and metrics
  remain locator- and content-free. An in-process MCP server over the production
  daemon/connector repeats the lost-reply restart proof, changed-destination
  conflict and foreign-subject nondisclosure. Separate shipped-process
  qualification remains a release-layer gate, not an engine semantic gap.
- **Residual and distribution truth:** the canonical intent and absorbing
  tombstone retain the destination in private plaintext storage (LINDDUN L13),
  and filesystem side effects remain visible. The refreshed source inventory
  contains 93 locked components and still truthfully reports the two pinned
  Keystone source blockers plus the separate pinned `keystone-observe` API
  blocker. No remote, publication, signing, activation, commit or push is
  claimed.

### 2026-11-10 — fourteenth-wave semantic adapter baseline

- **Provider-owned readiness:** the semantic service now derives readiness,
  capabilities and index generation from its privately owned provider before
  and after synchronization, submission, cancellation and polling. Unequal
  states invalidate admitted work exactly once; callers cannot publish a
  divergent provider state beside the provider.
- **Evidence-backed index currency and provenance:** generic LSP initialization
  now remains `Indexing`; only a privately owned server-profile readiness port
  can promote it, and every re-promotion after reindexing issues a fresh
  generation. Opaque provider identity plus bounded path-free profile/version
  are installed by composition, carried in state and replies, and validated at
  both host and service gates rather than learned from the language server.
- **Bounded LSP mechanism:** new `vellum-adapter-semantic-lsp` owns strict
  bounded `Content-Length` framing, JSON-RPC correlation, negotiated UTF
  position conversion and full virtual-document open/change/close publication
  over an injected nonblocking byte transport. It has no executable discovery,
  process launch, filesystem, socket-construction, workspace-trust or edit
  authority.
- **First useful query workflows:** static initialization enables `ReadIndex`
  only when pull diagnostics and hierarchical document symbols are both
  advertised. Full diagnostic reports and document symbols are converted to
  exact source-revision/submission-generation-bound results; local symbol
  filtering never issues workspace-wide queries. Correlation, cancellation,
  late orphans, malformed envelopes/ranges, output overflow and transport
  refusal remain closed and observable.
- **Isolation and capacity closure:** virtual URIs are path-free and injective,
  frames and pending work are bounded, source state is send-before-publish, and
  both the semantic service and adapter enforce a 64 MiB aggregate retained
  source ceiling with current/high-water telemetry. This replaces the prior
  multiplicative reference-limit exposure that could retain tens of GiB.
- **Plan and lifecycle hardening:** every provider-returned location is checked
  against the requesting client's exact admitted snapshot and UTF-8 boundaries;
  diagnostics remain primary-document-bound, and foreign or overlapping rename
  plans fail closed before rename is enabled. Protocol shutdown is an explicit
  busy/drain/correlated-null/exit/close transition, while deadline escalation is
  a separately observed abort and `Drop` is never graceful evidence.
- **Strict rename planning:** static `renameProvider` negotiation now enables
  `PlanRename` only beside the required read-index capability. Rename requests
  use exact UTF-aware positions, and replies accept only versioned, text-only
  `documentChanges` for current admitted virtual documents. Changes maps,
  resource operations, annotations, foreign or stale documents, split scalar
  positions and ambiguous ranges fail closed; `null` is an explicit empty
  review plan rather than an inferred provider failure.
- **Replacement and deadline algebra:** trusted composition can replace a
  provider only after a ready, fresh-provenance/fresh-generation candidate has
  replayed every retained exact snapshot in deterministic order without a state
  transition. Failure returns the candidate while the installed product stays
  authoritative; success swaps once, invalidates old pending work once and
  returns the old provider for graceful shutdown. Composition-driven expiry has
  a distinct `TimedOut` delivery and no clock is hidden in the core. Delivery
  capacity is preflighted so neither transition discards its terminal evidence.
- **Authorized semantic application:** new nonpublishable
  `vellum-semantic-apply-service` binds exact client-owned semantic snapshots to
  presentation-free workspace documents, rechecks ready provider
  provenance/generation, obtains exact `EditSource` evidence, derives expected
  bytes from final fresh snapshots and publishes a bounded multi-document plan
  through one engine prepare/commit and linked history group. Stale, foreign,
  malformed or locked participants yield zero publication. Persistence is
  explicitly `NotAttempted`; the filesystem is not part of the atomic claim.
- **Review-driven closure:** semantic spans now refuse inverted intervals at
  construction and provider ingress, point queries refuse split UTF-8 scalar
  offsets pre-effect, semantic bindings support exact monotone rebind and exact
  retirement, and the LSP JSON boundary rejects duplicate keys recursively
  before generic value construction. Replaced providers return an exact bounded
  retirement plan so refused closes remain retryable/escalatable rather than
  masquerading as graceful drain.
- **Local supervisor substrate:** new nonpublishable
  `vellum-semantic-local` owns canonical operator profiles, fixed argv,
  cleared/allowlisted environment, private cwd, process groups, bounded stdio
  and stderr, launch/grace/kill-reap deadlines, owner-loss cleanup and content-
  safe RED/USE counters. A compiled hostile child qualifies exact launch facts,
  saturation, graceful reap, forced group kill, crash and owner loss. The probe
  binary is not yet a useful semantic daemon or MCP surface.
- **Integrated semantic qualification:** formatting, diff hygiene,
  warnings-denied all-target/all-feature Clippy and rustdoc, the complete locked
  offline workspace suite including real Unix sockets, semantic adapter/service
  tests, project/effect/release boundaries, MCP extraction and compatibility and
  supply mutants pass together. Refreshed evidence closes over 23 packages, 27
  wire fixtures and 113 locked components, retaining exactly the two source-SBOM
  blockers and separate pinned Keystone API-closure blocker.
- **Residual truth:** the adapter is an LSP 3.17-compatible subset, not a
  supervised language-server product. Dynamic registration, flat symbol
  information, push/unchanged diagnostics, related documents, deadline
  scheduling, a complete service/adapter/session runtime driver, executable
  immutability/sandbox policy, supervised provider restart, separately evidenced
  per-document persistence, semantic SDK/transport, sibling MCP projection and
  a durable agent-facing apply workflow remain open. No remote,
  publication, activation, commit or push is claimed.

### 2026-11-10 — fifteenth-wave semantic composition closure

- **Composition-owned virtual documents:** the LSP adapter now accepts an
  injected URI issuer and can assign an opaque path-free URI on first snapshot
  admission without teaching the semantic service about LSP. URI construction
  proves a narrow grammar shared by explicit and dynamic registration; plain or
  encoded separators, traversal, query, fragment, authority and scheme
  substitutions fail before publication. Issuance, collision and capacity are
  bounded, retries are deterministic, and document/source state remains
  send-before-publish across admit, change, close and reopen.
- **Partition-bound asynchronous SDK:** `vellum-semantic-client` now defines a
  transport-neutral boxed-async client port whose ordinary operations are
  intrinsically bound to one composition-selected partition. The sovereign
  service no longer directly implements that ordinary port, so a client cannot
  retarget a handle or drain a forged peer partition. Client observation
  contains only readiness and capabilities; provider identity/profile/version
  remains private. Explicit expiry and one bounded pump live on a separate
  trusted runtime-driver port. This is an embedded SDK boundary, not
  authentication, a wire protocol or a remote transport.
- **Local embedded runtime:** `vellum-semantic-local` now preflights the exact
  adapter/service capacity product before launch, constructs and starts the LSP
  adapter over its supervised byte transport, and privately owns the semantic
  service. Its facade exposes only trusted client admission, partition-bound
  SDK handles, exact expiry, one bounded drive and content-safe
  readiness/capability/usage projections. It exposes no provider replacement,
  provenance, executable, process or background-loop control; the shipped
  binary remains a probe rather than a semantic daemon.
- **Supervisor lifecycle hardening:** stdin writes are independently abortable
  and protected by a 64 MiB byte-credit budget; close/abort are monotone
  admission fences. Terminal disposition is independent of stdout capacity and
  only an exact requested zero exit or evidenced successful forced cleanup can
  be `Closed`. Process finishing and event/output drainage run concurrently;
  exact leader membership detects an exited leader even when descendants retain
  inherited pipes. Post-start launch refusal requires accepted whole-group hard
  kill plus supervised-leader finish/reap or returns `CleanupUnconfirmed`.
  Counters distinguish group kill acceptance from leader reap and make no
  descendant-reap claim. Canonical-path replacement and absence of a process
  sandbox remain explicit residuals.
- **Hostile qualification:** focused evidence covers path-free URI refusal,
  dynamic replay and client isolation; SDK retarget/drain attempts, cancellation
  versus timeout and redacted observation; blocked stdin, byte-credit recovery,
  full terminal queues, nonzero/unexpected exit, owner loss, inherited-pipe
  descendants and confirmed post-start cleanup; and pure runtime preflight,
  start refusal and bounded drive. The focused semantic stack passes 19 adapter,
  20 service, 8 apply, 6 SDK and 15 local-runtime/supervisor tests with strict
  Clippy, rustdoc, formatting and diff hygiene.
- **Integrated qualification:** the complete locked/offline all-feature
  workspace suite passes outside the filesystem/socket sandbox required by its
  Unix adversarial fixtures, together with warnings-denied all-target Clippy,
  warnings-denied workspace rustdoc, formatting, diff hygiene, MCP extraction,
  project/effect/release/artifact/cargo-install/launchd gates and compatibility
  and supply-chain mutants. Refreshed conservative evidence remains 23 packages,
  27 wire fixtures and 113 locked components with the same two source-SBOM
  blockers and one additional pinned Keystone API-closure blocker.
- **Residual truth:** no clock scheduler, supervised provider restart, local
  semantic daemon/session protocol, authenticated remote semantic transport,
  sibling semantic MCP process, durable plan handle or outcome-unknown-safe
  rename-apply receipt exists. No remote, publication, activation, commit or
  push is claimed.

### 2026-11-10 — sixteenth-wave semantic lifecycle and projection closure

- **Opaque deadline scheduling:** the local semantic composition can retain
  same-clock expiry evidence only for exact pending client/request ownership.
  Dual key/deadline indexes bound memory and attempts, distinguish absent from
  foreign without disclosing the latter, retain retryable capacity refusals and
  advance a deterministic fair cursor so a saturated earliest request cannot
  starve later due work.
- **Truthful provider replacement:** staged replacement replays the sovereign
  service's bounded synchronization state before publication and returns an
  owned retirement obligation for the old provider. Graceful shutdown,
  escalation, forced-pending, forced-confirmed and cleanup-failed phases remain
  distinct; candidate, startup-cleanup and retirement owner loss request an
  unconfirmed abort rather than inventing completion evidence.
- **Durable-plan mechanism contract:** `vellum-semantic-apply-service` now
  offers a bounded, versioned and SHA-256-sealed journal algebra plus a
  journaled plan registry. Plan publication follows committed plan evidence;
  effects follow committed pending evidence; receipts follow committed
  completion evidence. Recovery rejects corrupt or incongruent histories and
  never reapplies a pending operation. Wrapper record/byte ceilings are exact
  recovery ceilings even when an injected adapter permits more. The supplied
  fault journal is intentionally volatile: no local-filesystem durability or
  daemon integration is claimed.
- **Agent-facing semantic projection:** new nonpublishable
  `vellum-semantic-mcp` is a bounded in-process MCP 2026-07-28 projection over
  the partition-bound semantic SDK. It exposes the seven read/plan queries and
  capability resource with qualified metadata, private zero-TTL cache policy,
  bounded client metadata and correlated overload errors. Construction requires
  sealed sovereign-validation evidence, so alternate public ports cannot
  transplant foreign documents or malformed spans. It is a library prepared
  for later peer extraction, not a runnable MCP process, transport or
  authentication boundary; rename application remains unadvertised.
- **Adversarial review closure:** independent review drove retry fairness,
  forced-shutdown truth, same-limit journal recovery, complete cacheable-result
  metadata, schema-valid bounded client metadata and nested-result trust
  closure. Focused semantic gates pass with 24 local-runtime tests, 20 service
  tests, 19 adapter tests, 19 journal/apply tests, 6 client tests and 11 MCP
  projection tests, plus strict Clippy and rustdoc.
- **Integrated evidence and residual truth:** compatibility now closes over 24
  workspace packages and 27 wire fixtures; the source inventory contains 114
  locked components while retaining the same two source-SBOM blockers and one
  pinned Keystone API-closure blocker. A semantic background daemon/session
  protocol, runnable stdio MCP peer, authenticated semantic transport, durable
  filesystem journal adapter, MCP rename-apply receipt, executable immutability
  and sandbox policy remain open. No remote, publication, activation, commit or
  push is claimed.

### 2026-11-11 — seventeenth-wave durability and session-lifecycle closure

- **Crash-conservative local plan journal:** the semantic journal's opaque-byte
  mechanism contract now lives at the neutral host boundary while semantic
  codecs and recovery meaning remain in the apply service. The macOS local
  adapter uses descriptor-relative private files, exact workspace/owner
  namespace binding, dense canonical record order, file and directory
  `F_FULLFSYNC`, strict record/byte ceilings and no eviction. A nonblocking
  exclusive advisory lock prevents cooperating compositions from racing one
  journal; initial and final boundaries recheck directory and file security,
  path identity and namespace equality. Post-create uncertainty remains
  `Indeterminate` and restart never silently repairs or truncates evidence.
- **Explicit semantic client retirement:** the sovereign service now fences a
  retiring partition, signals cancellation for exact owned pending requests,
  dispositions queued deliveries, and retries refused or unavailable document
  closes without releasing source snapshots. A client slot is reclaimed only
  after all owned source is confirmed closed. Because the provider port has no
  cancellation acknowledgement, request references remain in a bounded
  quarantine until authoritative provider replacement; late completions cannot
  cross a re-admitted client lifetime.
- **Independent review closure:** review found and closed live permission-drift,
  concurrent-writer, final namespace-check and invalid-limit classification
  defects before acceptance. The append-journal qualification passes 7/7; the
  final semantic-service suite passes 24/24; formatting, diff hygiene and
  project/effect gates pass with 24 crates and ten declared filesystem effect
  callers.
- **Provider and peer architecture:** the shortest lawful background-service
  path is an application-owned serialized semantic session broker, explicit
  synchronization/query role products, a strict local protocol and a separate
  composition-owned operator control plane, plus a
  transport client that independently re-earns the sealed sovereign-validation
  proof. A thin MCP executable over the current mutable runtime borrow would be
  unsound and is not planned.
- **Distribution fence:** semantic-local remains outside release artifacts.
  Immutable executable/profile closure, actual provider confinement, retained
  launch-cleanup ownership, macOS abrupt-parent cleanup, physical resource
  policy, bounded single-flight restart and physical USE/latency evidence are
  release blockers before it can ship as a background service. Automatic query
  retry remains forbidden.
- **Residual truth:** the real-disk journal and semantic recovery algebra still
  need one concrete restart composition test and daemon wiring. Client
  retirement still needs provider cancellation acknowledgement for earlier
  quarantine reclamation. No semantic session protocol, validated transport
  client, background actor, runnable semantic MCP peer, remote, publication,
  activation, commit or push is claimed.

### 2026-11-11 — eighteenth-wave semantic session and launch-ownership closure

- **Provider-neutral semantic session:** separate nonpublishable protocol and
  wire crates define exact exchange, replay-operation, incarnation, membership,
  synchronization-epoch and request-lifetime products. The 12-command protocol
  uses a closed command/outcome/refusal fiber, one attenuated Synchronize×Query
  role product, explicit full-snapshot begin/complete manifests and
  transport-only `OutcomeUnknown`. Strict canonical decoding is bounded and
  domain-separated; two semantic fixtures are frozen independently from the 27
  editing-wire fixtures.
- **Conservative local launch ownership:** semantic-local now consumes affine
  bounded artifact/profile qualification, binds canonical paths and exact
  executable/cwd observations, rejects loader-affecting environment names,
  revalidates at the final path-launch boundary, installs Unix
  `RLIMIT_CORE=0`, and retains an unresolved launch worker behind a permanent
  non-overlap fence. Its assurance product says `ObservedOnly` or `Unsupported`
  where path spawning, confinement and abrupt-parent cleanup are not proved.
- **Concrete durability composition:** the semantic plan registry is exercised
  against the real local append journal across drop/reopen. Completed work
  replays without re-entering authority or workspace effects; committed pending
  work recovers only as `OutcomeUnknown`, capacity never evicts, and foreign
  owner namespaces refuse.
- **Integrated evidence:** focused protocol/wire tests pass 7/9, semantic-local
  passes 26 tests with its isolated timeout child executed by a parent test,
  semantic apply restart qualification passes, and warnings-denied focused
  Clippy, formatting, diff hygiene, 26-crate boundaries, release/effect gates,
  dual compatibility inventory and all seven compatibility mutants pass. The
  refreshed source inventory contains 116 components with the same two SBOM
  blockers and one separate pinned Keystone API-closure blocker.
- **Residual truth:** a pure serialized semantic broker and transport-neutral
  phase-aware client ledger now exist. They namespace request identities,
  validate exact Collect/result provenance, retain replay/tombstone truth and
  drive trusted cleanup after policy loss without minting protocol authority.
  No socket, daemon wiring or runnable semantic MCP peer exists, and neither
  component implements authentication, confidentiality, persistence or
  automatic retry. Path-based provider
  spawn, interpreter/loader/plugin closure, confinement, CPU/RSS/FD ceilings,
  macOS abrupt-parent cleanup and restart control remain open. Semantic-local
  remains outside release artifacts; no remote, publication, activation,
  commit or push is claimed.

### 2026-11-11 — nineteenth-wave semantic-session transaction closure

- **Pure broker and phase ledger:** the unpublished broker is a serialized
  authenticated-context reducer with no provider, socket, filesystem or runtime
  dependency. The transport-neutral client preserves
  `NotSent | Reply | OutcomeUnknown`, permits only explicit exact-intent replay
  under fresh exchange correlation, and never invents delivery truth.
- **Transactional repeated synchronization:** a Ready membership can begin a
  fresh incarnation-global epoch from its committed witness set. Open, Change
  and Close update a bounded private stage through one affine in-flight effect
  lane; Complete publishes only the exact declared manifest. Queries remain
  fenced throughout staging, epochs cannot be reused after membership turnover,
  and projected document/source/witness limits refuse before provider effects.
- **Source-coherent request lifecycle:** accepted Change and Close terminalize
  affected Submitting/Pending requests as retained `Stale`; later provider
  delivery quarantines. Exact Collect remains non-destructive, Release reclaims
  the reserved result bytes while preserving the non-reusable tombstone, and
  policy-loss cleanup carries no client exchange, operation, replay or reply
  identity.
- **Shared accounting and observation:** protocol-owned checked functions define
  logical retained-intent and worst-case result-reservation bytes for every
  command and all seven query families. These are content accounting metrics,
  not heap, wire, RSS or physical-retention claims. Broker instrumentation uses
  fixed-domain counters, gauges and high-water marks without source, provider,
  identity or manifest labels.
- **Integrated evidence:** focused tests pass 10 protocol, 24 broker, 20 client
  and 9 semantic-wire cases. Warnings-denied Clippy, formatting, diff hygiene
  and the 28-crate boundary gate pass. The complete locked/offline workspace
  test suite passes in bounded cohorts, including 26 semantic-local tests plus
  its isolated launch-timeout child, and all 11 semantic MCP tests. The
  post-start launch-cleanup qualification now asserts the semantic receipt
  that can be emitted only after bounded hard-kill acceptance and leader reap;
  it no longer compares end-to-end launch time with the later-starting inner
  cleanup budget. Warnings-denied workspace rustdoc and all doc tests pass.
  Compatibility closes over 28 packages, 27 editing fixtures and 2 semantic
  fixtures, with all seven adversarial mutants rejected. The refreshed source
  inventory contains 118 locked components with the same two source-SBOM
  blockers and one separate pinned Keystone API-closure blocker.
- **Residual truth:** broker and client state are volatile; no durable session
  replay store, actor, socket, authentication, confidentiality, daemon wiring
  or runnable semantic MCP peer is claimed. Compact UTF-8 witnesses remain
  linkable process-local evidence and their digest commitments do not constitute
  literal source equality, authority or secure erasure. No remote, publication,
  activation, commit or push is claimed.

### 2026-11-11 — twentieth-wave semantic actor and authority closure

- **Bounded serialized actor:** the unpublished semantic-session actor owns the
  pure broker behind independent bounded command and trusted-control mailboxes.
  Pre-admission saturation, drain and closure return the exact command as
  `NotSent`; accepted work returns one opaque non-clone ticket and becomes
  `OutcomeUnknown` rather than false failure if the worker disappears.
- **Private affine interpretation:** five typed, synchronous provider methods
  accept only domain payloads and return closed outcomes. Broker continuations
  never leave the actor; it constructs the exact matching completion event and
  performs at most one provider or retirement drive per admitted control.
- **Fail-closed lifecycle:** drain fences commands immediately, Stop first
  enters a linearized `Stopping` phase, admitted earlier controls complete in
  FIFO order, later controls refuse, and clean termination requires empty
  command admission plus exact broker quiescence. Control admission preserves
  event ownership on refusal and returns explicit
  `Applied | Quarantined | OutcomeUnknown` completion truth. Owner loss fails
  closed, reconciles admitted uncertainty and terminates the worker; nonterminal
  join never blocks.
- **Separate semantic authority:** `SemanticQuery` and
  `SemanticSynchronize` are independent atoms in the finite host authority
  lattice and remain absent from the default read-only set. A future daemon
  must derive session roles by meet of operator ceiling, authenticated policy
  grant and requested attenuation; authentication alone grants neither.
- **Persistent-channel contract:** the editing daemon's one-request-per-fresh-
  session connector is explicitly ineligible for semantic membership. The
  future local semantic transport must authenticate once, retain one bounded
  authority session, process sequential frames, preserve exact
  `NotSent | Reply | OutcomeUnknown`, and never treat reconnect as replay or
  resumption.
- **Integrated evidence:** all 10 actor tests, 26 host tests and their doc tests
  pass locked and offline; warnings-denied all-target/all-feature focused
  Clippy passes. Tests cover real actor-path effect completion, command and
  control backpressure ownership, owner-loss fencing, Stop/control ordering and
  exact in-flight uncertainty after a provider panic or fatal provider-poll
  refusal. Compatibility closes
  over 29 packages, 27 editing fixtures and 2 semantic fixtures; the refreshed
  source inventory contains 119 locked components with the same two source-SBOM
  blockers and one separate pinned Keystone API-closure blocker.
- **Residual truth:** the effect port's nonblocking behavior remains a trusted
  composition contract. Actor state is volatile and no durable semantic
  membership/replay store, semantic-local provider bridge, native semantic
  transport, socket, daemon or runnable MCP peer is claimed. HMAC attribution
  and Unix socket ownership would not provide confidentiality or same-user
  isolation. No remote, publication, activation, commit or push is claimed.

### 2026-11-11 — twenty-first-wave semantic frame-transport closure

- **One-attempt frame adapter:** the unpublished semantic transport implements
  the session client's exchange port over an injected byte-frame exchange. It
  canonicalizes one command, calls the port at most once and admits only a
  canonical reply with exact kind, exchange, operation, incarnation and
  membership binding.
- **Exact phase truth:** local encoding or request-bound refusal is `NotSent`
  without invoking the port. Port `NotSent` and `OutcomeUnknown` remain exact;
  oversized, malformed, noncanonical or mismatched reply bytes after
  delegation become `OutcomeUnknown`, never false non-delivery or application
  refusal. The adapter performs no reconnect or automatic replay.
- **Content-safe traffic evidence:** fixed-cardinality saturating counters cover
  attempts, exchange calls, logical request/reply frame bytes, exact outcomes
  and closed refusal classes. These are content-accounting values, not claims
  about physical I/O, kernel buffers, allocation, successful transmission or
  confidentiality.
- **Integrated evidence:** all 7 transport tests and doc tests pass locked and
  offline, including successful Describe, Join and bound Leave replies, every
  reply-binding mismatch, pre-send refusal, post-send malformed/oversized
  uncertainty and counter saturation. Warnings-denied all-target/all-feature
  focused Clippy passes. Compatibility closes over 30 packages, 27 editing
  fixtures and 2 semantic fixtures; the refreshed source inventory contains
  120 locked components with the same two source-SBOM blockers and one separate
  pinned Keystone API-closure blocker.
- **Residual truth:** the adapter trusts the injected port's assertion that
  `NotSent` means no request byte reached the peer. No socket, listener,
  authentication handshake, persistent authority session, server dispatcher,
  daemon, MCP executable, TLS, confidentiality, reconnect or resumption is
  claimed. No remote, publication, activation, commit or push is claimed.

### 2026-11-11 — twenty-second-wave ordered semantic bridge service

- **Operation-specific ordered algebra:** `SemanticBridgeServiceV1` privately
  owns a newly constructed empty service and exposes distinct Open, Change,
  Close and Submit reports. Each report preserves entry provider state,
  optional pre-provider transition, the closed operation outcome and optional
  post-provider transition. Change and Close alone can carry their exact
  `SourceChanged` or `SourceClosed` receipt; Open cannot represent one.
- **No duplicate terminalization:** bridge invalidation removes, quarantines
  and cancellation-signals exact pending requests and returns canonical
  `ProviderStateChanged` or source receipts without enqueueing synthetic Stale
  or Cancelled deliveries. Legacy `SemanticServiceV1` behavior is unchanged.
- **Bounded sovereign surface:** construction requires one client and one
  provider poll per pump. Submit retains the per-client delivery-obligation
  ceiling before the provider effect and installs an accepted request before
  post-provider observation, so immediate invalidation remains explicit.
  Construction proves empty service state, not provider freshness or an empty
  provider completion buffer.
- **Terminal actions and driving:** bridge Cancel and composition-driven expiry
  return ordered reports and never synthesize delivery. A pre-provider
  transition may satisfy the exact target without a second cancellation
  signal. The bridge-only fallible provider port performs exactly one poll;
  failure performs no post observation, while success distinguishes no event,
  validated delivery, late completion and unknown orphan after post-provider
  observation. Late and orphan counts remain separate fixed-cardinality
  observations.
- **Settled transition truth:** a provider transition records both the state
  that triggered invalidation and the settled state after its cancellation
  signals. The stage remains explicit across cancellation-induced ABA and is
  mapped as one atomic composition observation; a following operation uses the
  settled state. Poll report Debug exposes only closed disposition classes and
  receipt cardinalities, never reply structure, revisions, identities,
  provider metadata or result cardinality.
- **Evidence:** all 38 semantic-service unit tests pass locked and offline,
  including exact maximum-bound receipts, source/post disjointness, provider
  ABA (`A -> B -> A`), refused/unavailable operations with transition evidence,
  inert repeated refresh, submit-immediate-invalidation, delivery reservation,
  exact cancellation/expiry, pre-transition satisfaction, cancellation-induced
  provider ABA, poll
  absence/failure/delivery/late/orphan, content-safe debug and absence of
  synthetic deliveries. Doctests, formatting and warnings-denied
  all-target/all-feature Clippy pass.
- **Residual truth:** the wrapper intentionally exposes no legacy pump,
  delivery queue or provider replacement. Fresh-provider/runtime type state,
  recoverable ownership, explicit retirement, a composite broker poll event
  and broker shadow validation are still required before actor or local-runtime
  wiring.

### 2026-11-11 — twenty-third-wave broker/actor ordered-transition closure

- **Exact broker-owned reports:** Open, Change, Close, Submit and Cancel now
  complete with broker-owned ordered reports. Every report binds the exact
  provider epoch/projection present before the service effect. Provider stages
  carry distinct observed and settled projections under one exact-successor
  composition epoch, preserving lawful cancellation-induced ABA.
- **Atomic shadow validation:** the broker captures a bounded canonical set of
  service-live request identities at effect emission. It applies pre-provider,
  operation and post-provider receipts to a shadow set, validates the whole
  product, then commits exact terminal identities and the settled provider
  projection atomically. Broad document scans are removed.
- **Global affine lane:** only one sovereign service effect may be outstanding
  across every membership, synchronization, Submit, Cancel, Leave and
  retirement path. Interleaved provider, delivery, policy or cleanup events
  fail as transition-order divergence rather than being silently recaptured.
  The broker also enforces its global pending ceiling.
- **Fatal composition divergence:** provider-before mismatch, skipped/same-state
  epoch fabrication, missing/extra/wrong-cause receipt evidence, unsupported
  query mismatch or impossible cancellation latches the broker while retaining
  pending/replay uncertainty. Later work remains fenced. The actor maps command
  or control divergence to `OutcomeUnknown`, phase `Failed`, bounded queue
  reconciliation, worker exit and port drop.
- **Evidence:** all 26 broker tests and 12 actor tests pass locked and offline;
  both doctest suites and warnings-denied all-target/all-feature Clippy pass.
  Focused coverage includes observed-B/settled-A ABA, exact successor epochs,
  atomic receipt poison with no partial commit, affine interleaving, stable
  Cancel versus pre-transition Stale, retained uncertainty and actor fatal
  command/control divergence.
- **Residual truth:** the actor provider drive still consumes one broker event;
  it does not yet translate the service's ordered fallible poll report into one
  composite broker event. The local mapper, shared epoch/projection owner,
  fresh runtime type state, recovery owner and explicit terminal retirement
  remain required before wiring.

### 2026-11-11 — twenty-fourth-wave composite drive and recovery closure

- **Atomic physical provider drive:** one successful provider poll is now one
  broker-owned report containing provider-before, optional pre/post transition
  stages and exactly one `NoEvent | Delivery | Late | Orphan` disposition. The
  whole product is shadow-validated before commit; a mismatched disposition
  cannot publish an otherwise valid transition.
- **No provider bypass:** raw Delivery and ProviderStateInstalled broker and
  actor events are removed. Policy loss remains the only general external actor
  event; provider state and completion can enter only through the composite
  drive. Broker event Debug is variant-only and content-safe.
- **Recoverable affine ownership:** actor spawn returns distinct owner, sender
  and must-use recovery capabilities. The worker borrows its broker and effect
  port inside the unwind boundary, so clean stop, owner loss, fatal divergence,
  poll failure and caught provider panic all return the exact runtime after a
  nonblocking finished check. Recovery is cleanup authority, not retirement.
- **Closed terminal correlation:** bridge live-pending capacity is independent
  of a bounded provider-lifetime tombstone relation. Cancellation, expiry,
  source/provider invalidation and successful delivery all terminalize an
  identity without making it reusable. Duplicate completion remains exactly
  Late. Client retirement and consuming checked provider extraction expose
  exact blockers and carry counted residue only into a retirement-only provider
  capability.
- **Fallible LSP bridge:** the LSP adapter implements the bridge poll contract
  with distinct poison, runtime-unavailable and delivery-refused failures.
  Cancelled LSP correlations remain in a bounded tombstone map until response
  or abort, so consuming a late response cannot produce false absence;
  cancel-notification send failure is latched.
- **Evidence:** broker 28/28, actor 13/13, semantic service 44/44 and LSP adapter
  25/25 unit tests pass locked and offline. Their doctests, formatting and
  warnings-denied all-target/all-feature Clippy gates pass. Independent review
  found the false-absence, cancel-send and raw-event bypass defects above; all
  were corrected before this wave closed.
- **Residual truth:** local actor wiring still requires one mapper-owned epoch
  and provider projection, fallible Open/Change/Close/Submit/Cancel mapping,
  coordinated initial binding, explicit deadline-expiry composition and the
  final provider retirement driver. No semantic daemon, MCP endpoint, remote
  transport, crates.io publication, activation, commit or push is claimed.

### 2026-11-11 — twenty-fifth-wave ordered local composition

- **Closed broker genesis:** a coordinated constructor accepts only epoch-zero
  provider binding. Unavailable and Indexing require bottom capabilities and no
  provenance; Ready requires exact provenance. The same predicate guards every
  observed and settled successor state.
- **Fallible affine effects:** Open, Change, Close, Submit and Cancel now return
  ordered reports or a closed fatal failure. An admitted failure is
  `OutcomeUnknown`, retains the pending broker continuation, stops serving and
  preserves the exact port for recovery. Actor construction refusal, including
  injected thread-launch failure, returns the untouched broker and port.
- **Coordinated local mapper:** the new `ordered` composition owns the fresh
  bridge service, one client, broker genesis and sole checked epoch allocator as
  one affine prepared value. Callers cannot extract a raw service, mutate the
  binding or pair the port with another broker. Every build stage returns a
  must-use residue containing the exact uninstalled provider or cleanup port;
  an unstarted prepared runtime has a cleanup-only exit.
- **Failure evidence before shutdown:** physical-poll failure carries
  provider-before, any lawful pre-provider transition and a closed failure
  class. The broker validates and commits that evidence before the actor fails,
  preventing recovered service state from outrunning the broker epoch.
- **Timeout ownership:** expiry is a sixth serialized actor drive with exact
  Expired, transition-satisfied and known-not-pending outcomes. Physical
  provider delivery is Peer-only; cancellation and timeout cannot be fabricated
  by a provider report. The current mapper constructor is explicitly
  deadline-disabled and therefore has one truthful NoDue state.
- **Evidence:** the combined locked/offline qualification passes LSP 25/25,
  semantic service 44/44, broker 33/33, actor 17/17 and local semantic 45 tests
  plus the intentionally isolated launch-timeout child; ordered mapper coverage
  is 19/19. Warnings-denied all-target/all-feature Clippy passes across all five
  crates. Construction residue, genesis projections, operation matrices,
  source causes, pre/post and ABA stages, projection-kernel/epoch refusal,
  late/orphan/Peer delivery, false terminal-delivery poison, fatal recovery and
  Debug redaction are covered.
- **Residual truth:** this is an injected-provider base, not the production LSP
  launch composition. It still needs a specialized adapter-limit witness,
  supervised launch constructor, real bounded scheduler with recovery discard
  receipt, and graceful/forced provider retirement integration. No shipped
  daemon/MCP surface or timeout policy is claimed.

### 2026-11-11 — twenty-sixth-wave scheduler and terminal cleanup closure

- **Fresh-started LSP admission:** ordered LSP construction consumes only a
  fresh adapter whose initialize send was accepted exactly once. Used,
  registered, buffered, terminal and send-refused adapters return exact cleanup
  ownership. The generic injected-provider constructor is crate-private.
  Semantic, document, aggregate-source, pending, URI and worst-case JSON/frame
  bounds close before service installation.
- **Timed typestate:** a distinct timed port owns an injected monotonic clock,
  fixed relative budget and bounded exact request/due relation. It reserves
  before Submit, schedules only accepted still-live work, clears every exact
  transition/source/cancel/delivery/failure/expiry receipt and drives one
  canonical earliest due request. No caller-selected wire deadline or Chronos
  authority is implied.
- **Exact scheduler recovery:** both directions and due values of the dual index
  are validated before every service effect. Install proves both slots before
  mutation; post-effect reconciliation is infallible. Stop obligations are
  mandatory and expose only counts/poison/consistency, so live, orphaned,
  mismatched or poisoned state cannot stop cleanly. Retirement returns a
  must-use Disabled, exact discard or Unresolved receipt with saturating
  counters.
- **Forced service abandonment:** a distinct consuming bridge path records
  exact clients, documents, pending work, deliveries, quarantine, cancellation
  and close outcomes. Attempt-bounded and unsafe-provider policies remain
  separate; provider panic is caught, classified unknown and stops all later
  provider calls. The resulting private-field parts capability cannot become
  the graceful provider wrapper.
- **Affine provider terminal proof:** the local retirement driver consumes
  checked or forced bridge providers. Checked cleanup may finish
  `GracefulConfirmed`, `ForcedConfirmed` or `CleanupFailed`; forced abandonment
  can only finish forced or failed and carries the exact abandonment receipt.
  Genuine graceful closure racing escalation remains graceful when no abort
  effect occurred. Whole-group kill acceptance plus supervised leader reap is
  not relabeled as descendant reap or confinement.
- **Evidence:** final combined locked/offline suites pass adapter 27/27, semantic
  service 49/49, broker 33/33, actor 18/18 and local semantic 63 tests plus one
  intentional isolated child. The ordered scheduler has 29 focused tests and
  retirement 20 focused tests. Warnings-denied all-target/all-feature Clippy,
  formatting, diff hygiene, compatibility and supply-chain gates pass.
- **Residual truth:** the specialized path still consumes an already-owned
  transport/adapter; qualified supervised process launch is not yet joined into
  the prepared actor constructor. Chronos-backed scheduling requires its own
  task-retirement receipt. No semantic daemon/MCP endpoint, remote operation,
  confinement, crates.io publication, activation, commit or push is claimed.

### 2026-11-11 — twenty-seventh-wave supervised composition and recovery closure

- **Effect-free preflight:** the production composition consumes one qualified
  launch and closes transport-frame, LSP, semantic, session, service and timed
  scheduler bounds before process launch. Disabled and timed constructors retain
  the exact qualification, collaborators and clock on every pre-effect refusal.
- **Initialize/output linearization:** Vellum's supervised stdout pump and the
  first initialize admission share one gate. Output accepted before initialize
  taints the transport and refuses startup with cleanup ownership; accepted
  initialize marks the stream claimed before later stdout can enter. This proves
  the Vellum pump ordering, not physical absence of bytes in an OS or ProcessKit
  buffer.
- **Normalized affine cleanup:** unstarted, actor-build-refused and recovered
  runtimes enter one must-use owner carrying actor/broker origin, deadline
  disposition, saturating client-retirement evidence and checked, forced or
  uninstalled extraction. Timed inconsistency has an explicit `Abandoned`
  transition. Only a consuming aggregate receipt joins that evidence to the
  provider/process terminal result; raw cleanup ports are not public exits.
- **Owned unresolved launch:** timeout and cleanup-unconfirmed launch failures
  retain an affine worker/fence owner. Nonblocking resolution yields only
  `NoApplicationConfirmed`, `CleanupConfirmed` or `RestartRequired`. The first
  two consume evidence and clear the singleton fence; indeterminate cleanup
  retains the fence and requires process restart. Legacy callers orphan the
  same owner into an exact-once recoverable process-global slot rather than
  detaching it.
- **Bootstrap cleanup sums:** transport freshness, fresh-start and ordered-build
  refusals retain their transport, adapter or bridge cleanup authority. A
  transport rejected before adapter construction has a forced-only terminal
  owner and cannot fabricate provider or bridge-abandonment evidence.
- **Evidence:** the final local semantic test binary passes 74/74 ordinary tests
  with three intentionally isolated child cases; their parent qualifications
  execute the launch-timeout, confirmed-absence and restart-required paths.
  Warnings-denied all-target/all-feature Clippy, formatting, diff hygiene,
  compatibility, supply-chain, project-boundary, effect-boundary and release
  gates pass.
- **Residual truth:** exact causal reclassification of stdout arriving after the
  provisional transport wrapper still collapses to the adapter's generic
  delivery-refused startup class, although cleanup ownership and aggregate
  freshness counters remain exact. The real-provider readiness/operation/failure
  matrix, active nonblocking actor stop, confinement and Chronos retirement are
  still required before the actor bridge can ship.

### 2026-11-11 — twenty-eighth-wave supervised process and stop qualification

- **Exact startup cause:** a shared closed witness now attributes only the first
  initialize refusal. Pre-initialize pump output is exactly
  `TransportFreshness(BufferedOutput)` and retains forced transport/provider
  cleanup ownership; later output cannot reclassify an unrelated capacity or
  delivery refusal.
- **Nonblocking actor stop:** consuming stop admission moves `Draining` to
  `Stopping` atomically and returns one affine ticket. Polling yields the same
  ticket while pending, a clean receipt, a refusal receipt plus the exact owner
  restored to `Draining`, or explicit outcome-unknown evidence. Explicit and
  implicit ticket abandonment are instrumented and are never termination
  evidence. Recovery remains a separate affine authority.
- **Same-spawn recovery product:** `spawn_managed` binds actor control and
  recovery at construction so receipts from different actors cannot be joined.
  Stop refusal restores that exact product and records bounded history;
  Clean, OutcomeUnknown, Abandoned and FailedBeforeStop remain distinct. Clean
  requires clean recovery, failed-before-stop requires failed recovery, and an
  impossible mismatch retains cleanup authority. The stop/recovery witness is
  carried through normalized cleanup into the terminal provider receipt.
- **Supervised process matrix:** a standalone compiled LSP fixture now exercises
  qualified launch, real framed stdio, fresh adapter startup, ordered actor
  composition, injected Unavailable/Indexing/Ready projections, membership and
  synchronization, graceful cleanup, and post-ready process failure through a
  failed-origin cleanup receipt. Command, control and stop observations are
  nonblocking and deadline-bounded.
- **Evidence:** actor qualification passes 23/23 tests. The local semantic
  binary passes 82 ordinary tests with three intentionally isolated children;
  the three supervised-process cases pass end to end. Warnings-denied Clippy
  passes for both crates.
- **Residual truth:** the compiled fixture proves the supervised
  process/transport/composition path, not readiness derived from a production
  language server or real diagnostics/symbol/refactor operations. Physical
  OS/ProcessKit prebuffer absence, confinement and Chronos task retirement
  remain unclaimed.

### Semantic-local actor bridge acceptance gate

Receipt, fresh bridge service, ordered broker report, atomic composite drive,
recoverable actor ownership, client retirement, checked provider extraction,
fallible LSP polling, coordinated genesis and the injected-provider mapper are
closed. Fresh-started LSP admission, exact adapter-limit proof, deadline-enabled
and disabled policies, scheduler retirement, forced bridge abandonment,
supervised launch, normalized recovery and provider/process terminal receipts
are also closed. The pinned rust-analyzer qualification below closes one real
provider-derived readiness and semantic-operation profile. The production
local-LSP bridge deliberately remains unshipped until the selected deployment
also supplies the confinement and resource controls claimed by that deployment.
Any stronger physical prebuffer guarantee still requires a two-phase
provider-start barrier; the current claim ends at Vellum's supervised pump.

Until then, the shipped local semantic binary remains a probe over the legacy
runtime and must not claim the actor bridge.

### 2026-11-11 — twenty-ninth-wave semantic-operation and initialization closure

- **Deterministic semantic operations:** the compiled standalone provider now
  receives synchronized `didOpen` state and returns nonempty diagnostics,
  nested document symbols and a versioned `documentChanges` rename plan. The
  ordered service/broker/managed-actor path verifies exact request, target
  revision, provider, generation, spans, symbol kinds, diagnostic class and edit
  set, then releases each result and reaches graceful terminal cleanup. Debug
  evidence is checked not to contain diagnostic, symbol or replacement text.
- **Bounded initialization profile:** a typed adapter profile joins provider
  identity, framing limits, semantic limits and initialization policy. It can
  bind one bounded file workspace, advertises fixed workspace-folder and
  `workspaceEdit.documentChanges` capabilities, captures bounded opaque
  `serverInfo`, and passes only a closed health × quiescent notification to an
  audited readiness observer. Generic initialize remains Indexing. The
  supervised constructor byte-encodes and binds the exact qualified workspace
  root before launch; the standalone provider rejects a mismatched initialize
  root.
- **Strict inbound classification:** responses, notifications and inbound
  server requests are disjoint. Malformed configured readiness notifications,
  malformed server identity and unsupported inbound requests fail closed;
  unrelated notifications are counted and ignored without becoming readiness.
- **Pinned same-instance evidence:** an opaque bounded protocol profile now
  binds a nonzero profile digest, exact expected `serverInfo` and one audited
  readiness method before initialize. Missing or mismatched identity fails
  startup; notification-bound profiles cannot be promoted by independent
  polling. The production supervised constructors retain root derivation while
  accepting this borrowed policy, and terminal retirement receipts capture the
  exact adapter's closed qualification evidence and usage. A standalone
  process scenario proves exact identity × notification readiness × graceful
  terminal cleanup without exposing provider strings or methods.
- **Timed same-spawn closure:** deadline-enabled actors now have the same managed
  stop/recovery product as disabled actors. Scheduler authority survives
  recovery and must retire or be explicitly abandoned before base cleanup.
  `NotApplicable`, `Compatible` and `Mismatch` joins persist through terminal
  receipts rather than requiring consumers to rediscover contradictions.
- **Evidence at this checkpoint:** the LSP adapter passed 32/32 tests and
  semantic-local passed 87 ordinary tests with three intentionally isolated
  children. The subsequent real-provider wave supersedes these counts.
- **Residual truth at this checkpoint:** this closed the production profile and
  same-instance receipt seam, not yet a real language provider. The subsequent
  wave closes that provider tier while retaining explicit confinement and
  physical CPU/RSS nonclaims.

### 2026-11-11 — thirtieth-wave pinned rust-analyzer qualification closure

- **Standalone execution policy:** a closed adapter variant binds exactly one
  canonical `src/lib.rs` project to the qualified workspace. It supplies the
  audited rust-analyzer initialization policy with Cargo project discovery,
  build scripts, proc macros, sysroot discovery and check-on-save disabled. No
  raw JSON or caller-selected provider option crosses the adapter boundary.
- **Exact path/URI relation:** `linkedProjects` receives the canonical absolute
  filesystem path while LSP document traffic receives its injective `file://`
  URI. Workspace-relative locators reject traversal, ambiguity, noncanonical
  encoding and root escape before any process effect.
- **Pinned package and artifact:** the independent unpublished qualification
  package has its own lockfile, strict profile and fixture manifests, symlink
  and replacement defenses, and a fixed official rust-analyzer 2026-11-09
  macOS ARM64 artifact identity. Preflight is a distinct factor and can never
  substitute for semantic evidence.
- **Real semantic product:** the canonical pinned run derives Current only from
  two validated `experimental/serverStatus` notifications, synchronizes the
  exact fixture, returns one real error diagnostic, exact nested module and
  function symbols, and a versioned two-edit rename. It releases all three
  requests, leaves the session, joins clean timed-actor stop with compatible
  same-spawn recovery, retires deadlines, performs checked provider extraction
  and proves graceful LSP and process closure.
- **Shutdown classification correction:** a legal late readiness notification
  can precede the correlated shutdown response. Shutdown now classifies
  notifications, inbound requests and responses before correlation: validated
  notifications cannot impersonate the response, inbound requests remain
  fail-closed, and only the exact null response sends `exit` and closes the
  transport.
- **Content-safe receipt:** the successful receipt contains only identities,
  digests, counts, spans, closed classes and support dispositions. It records
  three submitted/completed requests, one opened snapshot, one rename plan,
  two readiness notifications, one promotion, zero demotions/unavailability,
  zero malformed/orphan/inbound-request evidence and no saturation. The final
  copied test binaries pass 39/39 adapter tests and 89 semantic-local tests with
  three isolated child cases; warnings-denied Clippy passes for both crates.
- **Residual truth:** executable and working-directory identity are observed,
  ambient-environment clearing and core-dump disabling are enforced, while
  filesystem, network, subprocess and abrupt-parent confinement remain
  unsupported. Its explicit unbounded resource policy reports whole-tree
  memory, process-count and CPU enforcement unsupported. The receipt does not
  claim CPU/RSS measurement, transitive toolchain confinement, automatic
  restart, provider distribution or general language-server compatibility.

### 2026-11-11 — thirty-first-wave fail-closed provider resource policy

- **Closed policy product:** semantic-local now distinguishes the explicit
  compatible `Unbounded` policy from `RequiredWholeTree`, whose nonzero memory,
  process-count and CPU-millicore ceilings are bounded by fixed V1 maxima. The
  existing profile constructor remains source-compatible and selects
  `Unbounded`; callers must opt into the strict constructor deliberately.
- **Qualification before effect:** the exact policy discriminator and integer
  ceilings participate in the opaque launch-profile digest. A spawn-free host
  mechanism check precedes filesystem qualification and process creation.
  Strict policy currently accepts only ProcessKit Job Object or cgroup-v2
  mechanisms; other mechanisms return the typed unsupported-policy refusal.
- **Enforcement without downgrade:** provider launch creates the ProcessKit
  group with all three ceilings before starting the provider. Unsupported,
  invalid or unenforceable resource-limit setup returns a typed confirmed
  launch failure with no process application; it never retries unbounded. The
  existing affine timeout, cleanup owner and permanent launch fence remain the
  authority for uncertainty after process application can begin.
- **Assurance and nonclaims:** memory, process-count and CPU are three distinct
  assurance axes. A receipt for a successful strict launch reports them
  enforced; qualification alone does not claim process application or measured
  utilization. Unbounded profiles report them unsupported. A process-count
  ceiling is not subprocess confinement, a configured limit is not CPU/RSS
  observation, and this slice does not claim filesystem, network, subprocess
  or abrupt-parent confinement.
- **Pinned-profile truth:** the current macOS rust-analyzer profile still uses
  the compatible unbounded constructor. Its receipt must continue to report all
  three physical-resource axes unsupported; no prior receipt is upgraded by
  this seam. Because policy is newly profile-bound, even the explicit unbounded
  profile has a new launch-profile identity and derived receipts/baselines must
  be regenerated rather than compared across the contract revision.
- **Qualification evidence:** injected mechanism tests prove supported and
  unsupported preflight without platform coincidence, exact ProcessKit option
  projection and ceiling refusal. The copied semantic-local binary passes
  92 tests with three isolated child cases ignored; package-scoped all-target,
  all-feature warnings-denied Clippy passes offline.

### 2026-11-11 — thirty-second-wave source-install delivery unit

- **One install package, two sovereign peers:** the nonpublishing `vellum`
  distribution package owns no runtime behavior and installs exactly
  `vellum-daemon` and `vellum-mcp`. Their CLI interpretation remains in the
  owning daemon and connector libraries, so independent package builds and MCP
  extraction do not depend on the distribution package.
- **Straight source path:** a coordinated checkout now installs both first-
  release executables with one `cargo install --locked --path
  distribution/cargo-install` command. The source qualifier builds and installs
  that single package, then requires the exact two-binary closure and both help
  and startup-refusal surfaces.
- **GitHub truth:** the intended remote command is `cargo install --locked
  --git https://github.com/gantbd/vellum --rev RELEASE_COMMIT vellum`.
  Immutable-object qualification passes locally from Vellum
  `4463291ba58f30c64fc715722c38983c3e55221b` and Keystone
  `3486252cdbaeb2d784756076f43ef6157ed0b7f8`; remote reachability remains
  deliberately unclaimed.
- **Reviewed closure:** project and release boundaries now close over 31
  nonpublishable workspace packages. Compatibility records the additive
  binary-only package and the CLI-module extraction; source-SBOM closure is 121
  components with zero source or pinned-API blockers. The separate semantic
  qualifier closes over 72 components with zero source blockers.

### 2026-11-11 — thirty-third-wave product-surface closure

- **Agent-value semantic queries:** the production LSP adapter now lowers
  definition, references, implementations and type-at-point/hover through the
  existing closed `ReadIndex` algebra. Server capability negotiation remains
  method-specific; exact byte positions are translated without splitting UTF-8;
  response locations must bind an already-open Vellum snapshot; foreign URIs,
  oversized results and malformed hover content refuse and are metered without
  exposing source.
- **Externally visible receipt pressure:** additive private control status V2
  (`[2,2]` / `VCS2`) exposes retired durable-operation tombstone occupancy,
  capacity and saturation. V1 remains byte-compatible and omits the new fields;
  the installed `--status` command requests V2 and emits content-free JSON.
- **Real immutable-install gate:** coordinated source qualification now runs the
  documented positional `cargo install --offline --locked --git ... --rev ...
  vellum` command against exact local Git objects. Remote verification checks
  both repository commits rather than only Keystone.
- **Shipped-process replay:** separately launched daemon and MCP binaries now
  qualify lost-reply recovery across restart. The harness persists its operation
  reference, observes the saved product, discards the MCP response/process,
  restarts both peers and retrieves the durable receipt without rewriting a
  subsequently changed file.
- **Focused evidence:** 41 semantic-LSP tests, control-status codec and real CLI
  tests, the shipped-process restart scenario, immutable-install fixture,
  release/supply/compatibility gates and diff hygiene pass. The compatibility
  baseline changes only the semantic-LSP and daemon source-tree hashes. The
  committed source qualifier installs both binaries from immutable Vellum
  `b83c1eef732ba93444b4c8ec8cb5524d802dd8b5` plus the same pinned Keystone
  object; remote reachability remains unverified by policy.

### 2026-11-11 — semantic MCP executable boundary

- **Runnable protocol peer:** `vellum-semantic-mcp` now owns a bounded,
  sequential newline-delimited stdio loop with exact response flushing,
  notification silence, bounded oversized-frame draining, truncated-EOF
  refusal and fixed-cardinality content-safe exit observations.
- **No false composition:** default startup refuses because the persistent
  authenticated semantic-session dispatcher and sovereign-validation bridge do
  not exist. Explicit `--unavailable` mode composes the connector with a real
  sovereign service partition and permanently unavailable provider for MCP host
  and failure-path qualification only. It cannot admit snapshots or answer a
  useful semantic query and never advertises readiness.
- **Executable qualification:** process-level tests exercise discovery,
  unavailable capability observation, a real query refusal, clean EOF metrics,
  missing-composition startup refusal and non-execution of a valid but
  unterminated EOF fragment.
- **Next bridge:** a production peer still requires an application-owned
  persistent listener/dispatcher, authenticated role derivation, actor/provider
  ownership and a transport client that revalidates synchronization witnesses
  before entering the sealed MCP client boundary. The connector must not infer
  membership from request ids or mint that proof.

### 2026-11-11 — peer materialization and production-launch gate

- **Materialized native MCP peer:** a deterministic emitter now constructs an
  independently buildable `vellum-mcp` repository payload from one committed
  Vellum tree. The payload has its own manifest, lock, tests, licenses and
  source receipt, depends on exact Vellum and Keystone Git revisions, and passes
  both locked tests and a disposable immutable-Git `cargo install` smoke. The
  refreshed ignored staging payload and local sibling bind Vellum
  `02f2670fd2a886c58c5188dd014958049fbe392d` and Keystone
  `3486252cdbaeb2d784756076f43ef6157ed0b7f8`. The current sibling commit is
  `8a96a983396aa4bb419340c079b2ac83e9671722`; it has no remote and nothing has
  been published.
- **GitHub-ready source policy:** the emitted peer now includes pinned,
  read-only GitHub Actions CI, locked test/Clippy/format gates, a Rust contract
  test joining `Cargo.toml`, `Cargo.lock` and `release/source.json`, contributor
  guidance and source-integrity security policy. It still creates no release,
  package, attestation or remote.
- **Closed semantic shipping predicate:** semantic-local now promotes an
  ordinary single-use qualified launch only when the meet of all eleven fixed
  assurance axes is `Enforced`. Production launch functions accept only that
  affine token. Refusal is redacted, retains the exact probe token and exposes
  typed per-axis support without weakening `ObservedOnly` or `Unsupported`.
- **Truthful macOS result:** the current macOS profile is mechanically
  ineligible for production. Executable and cwd identity are observed rather
  than immutable; filesystem, network and subprocess confinement, abrupt-parent
  cleanup, and whole-tree memory/process/CPU enforcement remain unavailable.
  ProcessKit 3.3.4 supplies the latter ceilings through Job Objects or cgroup-v2,
  not the selected macOS process-group mechanism.
- **Validation:** the semantic MCP peer passes 15/15 package/process tests and
  strict Clippy/rustdoc; both production-promotion tests, project/release
  boundary gates, formatting and diff hygiene pass. The independently
  reproducible actor panic-timing failure found during this wave is closed by
  the later receiver-ownership correction below rather than reclassified.

### 2026-11-11 — witness-sealed semantic session query bridge

- **Seal ownership closed:** `vellum-semantic-client` now owns the adapter from
  a query-capable, witness-validated semantic-session client into the private
  sovereign semantic-client seal. Membership, authentication and canonical
  decoding alone remain insufficient.
- **Uncertainty preserved:** an outcome-unknown Submit retains its exact intent
  in the session ledger. Only an identical later MCP call can request explicit
  replay; changed intent conflicts and terminal results remain stable. Collect
  is nonmutating and may be attempted again after an unknown read outcome.
- **Witnessed delivery:** the in-process end-to-end test performs Describe,
  attenuated Join, transactional snapshot synchronization, fresh ready
  observation, uncertain Submit, exact replay and Complete collection through
  the real session-client nested result validator before entering the sealed
  query port. Wrong-role and unsealed construction refuse.
- **Still open:** no authenticated persistent listener, socket dispatcher or
  process-level useful MCP composition exists. Synchronization input must
  eventually come from composition-owned native Vellum snapshot products, not
  an MCP request or filesystem manifest.

### 2026-11-11 — actor panic-order recovery closure

- **Root cause fixed:** the actor worker previously moved its command/control
  receivers inside `catch_unwind`. A provider panic could therefore disconnect
  an admitted stop acknowledgement while shared phase still said `Stopping`,
  before the catcher published `Failed`. Receivers now remain owned outside the
  unwind boundary, so failure state precedes acknowledgement disconnect.
- **Exact uncertainty evidence:** deterministic actor and ordered-composition
  tests first prove the admitted stop is pending in `Stopping`, then require an
  outcome-unknown receipt observing `Failed`, one worker panic, one uncertain
  stop, compatible same-spawn recovery and exact cleanup ownership.
- **Validation:** 32 repeated race runs pass; the actor suite passes 23/23 and
  semantic-local passes 94 tests with its three intentional isolated children
  ignored. Strict Clippy/rustdoc and diff hygiene pass for both packages.

### 2026-11-11 — local semantic Unix client exchange

- **Runtime byte path:** `vellum-semantic-transport` now has a usable
  client-side Unix stream exchange with a persistent connection and bounded
  big-endian length framing. It is the physical byte interpretation under the
  existing canonical semantic frame validator.
- **Exact uncertainty:** private-path/connect and pre-byte write failure are
  `NotSent`. Lost, timed-out, malformed-length or oversized replies after a
  transmitted request are `OutcomeUnknown` and permanently fence that
  connection object; it does not reconnect or retry a possibly live session.
- **Local policy and evidence:** construction and connect enforce an absolute
  canonical path, effective-user ownership and private parent/socket modes,
  with identity rechecked around pathname connect. Separate deadlines and
  fixed-cardinality logical I/O observations cover bounded RED/USE inputs
  without claiming authentication, peer identity, confidentiality or physical
  resource measurement.
- **Qualification:** real Unix socket tests cover two exchanges on one
  connection, private-path refusal, read timeout, oversized reply refusal and
  a server that consumes the request then drops the reply.
- **Still open:** the authenticated listener/dispatcher and its persistent
  authority session remain the next product slice. No TCP, TLS, automatic
  reconnect, provider ownership, snapshot synchronization or MCP policy is
  added here.

### Semantic-provider qualification ladder

Provider qualification is two explicit tiers; absence is never a passing
result.

1. **Deterministic required tier — closed.** The standalone compiled fixture
   receives synchronized documents and returns bounded nonempty diagnostics,
   nested symbols and a versioned `documentChanges` rename plan through the
   complete ordered service/broker/actor path. This is fixture protocol
   compatibility, not production-provider semantics.
2. **Pinned real-provider tier — closed for the named profile.** A nonpublishable
   `qualification/semantic-lsp-provider` package consumes an absolute canonical
   executable plus a checked-in profile binding platform, executable and
   fixture digests, server identity, argv/environment, readiness schema and
   time/resource ceilings. Missing artifacts, identity mismatch or missing
   readiness evidence are refusals, never skips. The first profile now produces
   a complete real-provider receipt against the fixed official artifact; it
   remains qualification evidence rather than a shipped peer adapter.

The generic LSP seam now has a bounded initialization profile, declared
`documentChanges` support, bounded `serverInfo`, strict response/notification/
server-request separation and a notification observer from which a pinned
profile can derive Indexing, Current or Unavailable. Initialize success alone
continues to mean Indexing. The supervised constructor binds the initialization
root to the same qualified workspace root used for process launch. Its pinned
counterpart additionally binds exact server identity and a provider-specific
readiness method, prevents poll-only promotion, and retains the exact adapter's
closed evidence and usage with provider/process terminal cleanup. The pinned
rust-analyzer tier consumes that seam through the explicit standalone selector;
generic pinned LSP behavior remains unchanged.

A provider receipt is the product of artifact/profile identity, initialize and
server identity, readiness transitions, synchronization, diagnostics, symbols,
rename, stop disposition, actor recovery and provider/process terminal
evidence. It contains only digests, kinds, counts, spans, latency/error/
saturation counters and terminal classes. Normal pull requests require the
deterministic tier; a manual or scheduled release workflow requires the pinned
provider receipt. Rust Analyzer does not by itself qualify its transitive
Cargo/rustc/build-script/proc-macro execution or confinement.

### Local activation transaction closure

- **Closed transition algebra:** release tooling now has explicit prepared,
  candidate-effect, rollback-pending, rollback-effect and terminal states with
  distinct confirmed, refused and outcome-unknown results. A canonical private
  approval binds the exact transaction and distinct candidate/prior manifest
  identities before scratch mutation.
- **Immutable scratch layout:** candidate and prior daemon/MCP/plist triples are
  hash-verified into mode-closed digest directories. One synchronized regular
  pointer atomically selects the complete desired version, while a separately
  synchronized journal records the exact phase, selected digest and generation.
  Crash failpoints before and after candidate and rollback cuts make
  journal/pointer divergence explicitly unknown.
- **Truthful boundary:** the simulator executes no staged payload and does not
  invoke, install, stop or inspect launchd. Content-safe receipts state
  `launchdEffect: not-attempted` and `one-desired-state-pointer-only`; a real
  platform executor, running-process identity and post-activation data-plane
  health receipt remain release gates.

No item in this ledger authorizes crates.io publication or claims completion of its containing delivery phase.

### 2026-11-12 — semantic MCP lifetime and peer delivery

- **Bounded continuous MCP lifetime:** membership-local operation exhaustion
  rolls over under a fresh membership without ending stdio. Definitive global
  replay exhaustion now retires and closes the old authenticated connection,
  lets the service rotate a quiescent fresh incarnation, fully resynchronizes a
  new witnessed session and rebinds the same affine MCP owner. Pending work
  owned by the retired membership becomes an exact replayable cancellation;
  not-sent and outcome-unknown mutations never trigger reconnect.
- **Zero-authority semantic SDK:** new `vellum-sdk-semantic-client` groups the
  semantic client, protocol, witnessed session client, bounded transport and
  explicit embedded qualification service without adding behavior or granting
  roles. `vellum-semantic-mcp` no longer imports those implementation crates
  directly.
- **Semantic peer materialization:** the deterministic extraction contract
  copies only declared regular files, qualifies locked tests, warnings-denied
  Clippy, path installation and exact-revision Git installation, and emits a
  standalone non-published source tree. The local sibling repository binds
  Vellum `25afc5a38af99963c15df4488cb42f93786be3ff`, Keystone
  `3486252cdbaeb2d784756076f43ef6157ed0b7f8`, and initial peer commit
  `0c713411c26caefa683cba11ba762fd2b9c72ce7`. It has no remote.
- **Coordinated source enforcement:** the clean-archive source manifest now
  requires the semantic extraction contract, documentation and verifier/emitter
  pair. Coordinated GitHub CI qualifies both MCP extraction closures and then
  materializes the semantic peer from the exact checked-out Vellum/Keystone
  revisions, including locked tests, strict Clippy and an exact-revision Git
  install smoke. This adds no publication or remote-write authority.
- **Pinned-provider apply reconciliation:** the live rust-analyzer qualifier
  already feeds its exact versioned rename plan through the stale-safe apply
  service, proves stale-source, provider-transplant and client-isolation
  refusal, and publishes exactly one linked in-memory history group with disk
  persistence `NotAttempted`. The security model and acceptance order now
  reflect that delivered boundary; provider confinement remains separate.
- **Source-of-truth correction:** native Vellum workspace state and its
  revisioned change feed remain the semantic synchronization authority.
  Automatic external-file merging remains deferred rather than being smuggled
  in as a semantic watcher; external changes retain their existing explicit
  conflict/reload policy boundary.
- **Provider resource observations:** the supervised semantic process now
  samples ProcessKit resource statistics every 250 milliseconds and publishes
  only fixed-cardinality success/failure, availability and high-water values.
  Unsupported CPU, memory, I/O and peak-process axes remain absent rather than
  becoming false zeros; POSIX active units remain explicitly group counts.
  Focused tests, the full 105-test semantic-local suite, warnings-denied Clippy,
  compatibility mutants and architecture/release gates pass. This is shipped
  measurement behavior, not a macOS confinement or enforcement claim.
- **Native launchd deployment helper:** GitHub candidates now build the inert
  `vellum-launchd-render` helper as a locked, non-publishable Rust workspace
  executable. Candidate and rollback qualification execute the exact native
  payload and `plutil`-check its fixed read-only profile; the artifact no longer
  requires Node, while installation and activation remain unauthorized.
- **Durable operation status surface:** the authenticated native and MCP paths
  expose a separate content-free read-only status query with the closed
  pending/complete/retired/absent-or-foreign/unavailable sum. It is bound to the
  active subject and workspace, preserves uncertain client state, and cannot
  execute, retrieve receipt content, or retire.
- **Bounded restart policy:** semantic-local now exports a single-flight affine
  restart controller with a finite explicit backoff schedule, no query replay,
  and permanent fuses for cleanup/launch uncertainty and exhaustion. It is a
  consumable policy boundary but is not silently installed in the binary and
  owns no clock or process authority.

### 2026-11-12 — agent edit, call hierarchy, restart and activation delivery

- **Atomic agent workspace edit:** the native protocol, canonical wire, client
  SDK and MCP peer now expose one bounded multi-document replacement command.
  The service captures exact revisions, validates the complete product before
  one in-memory batch commit, omits exact no-ops and returns a revisioned
  receipt. Stale participation refuses the whole edit; persistence remains an
  explicit subsequent effect rather than a false atomicity claim.
- **Call hierarchy surface:** incoming and outgoing call queries now traverse
  host algebra, semantic protocol/wire, service, broker, client, LSP and the
  semantic MCP peer. The LSP composition performs exact prepare-call-hierarchy
  followed by the selected direction, retains bounded opaque item provenance
  and refuses ambiguous preparation.
- **Opt-in supervised hot restart:** the local semantic composition can launch
  a fresh qualified candidate, wait for readiness, replay retained snapshots,
  swap once, retire the old provider and expose the new runtime only after
  terminal cleanup reconciliation. Queries are invalidated without replay.
  Direct embedded launch now carries the same workspace initialization as the
  supervised composition, and a fresh index generation lawfully distinguishes
  a restarted instance of the same qualified provider profile.
- **Launchd activation executor:** a separate non-publishable Rust package now
  validates candidate and prior identities plus private approval, writes a
  durable pre/post-effect journal around exact `launchctl` invocations, checks
  running PID/program identity and daemon-v2 status, and either commits or
  rolls back to the exact prior version. Unknown launchctl outcomes stop without
  retry. Tests use an injected executor; no host launchd activation occurred.
- **Integrated qualification:** formatting, diff hygiene, project/effect and
  release boundaries, compatibility fixtures, supply-chain mutants, launchd
  render/activation, source-install qualification, strict Clippy and
  warnings-denied documentation pass. Semantic-local has 111 sandbox-safe
  passes plus three separately qualified private Unix-socket cases; the two
  real-provider restart paths pass independently. No crates.io publication,
  remote write or launchd activation was performed.

### 2026-11-12 — candidate packaging and ordered restart seam

- **Candidate activation product:** GitHub candidate schema 4 preserves exactly
  daemon and MCP as Cargo-install artifacts while adding the renderer and
  separately authorized activator as checksum-closed deployment tools. Candidate
  qualification invokes the activator help path only and records
  `launchdEffect: not-attempted`; no launchctl effect was exercised.
- **Two-materialization equivalence:** the pinned workflow builds the candidate
  twice into distinct roots and refuses upload unless the complete regular-file
  closures have separate identities and equal bytes/SHA-256 products. The
  create-exclusive evidence is content-safe and does not claim cross-host
  reproducible builds, signing or attestation.
- **Clean-topology qualification:** an actual detached Vellum checkout plus a
  clean pinned Keystone worktree exposed and corrected a fixture-only mismatch:
  the real release topology is one `vellum` install package with two binaries,
  not two source packages. Commit `424a18d` builds that exact topology twice;
  both schema-4 products passed byte-equivalence with manifest digest
  `1800954fc01eec1b8980f5ef9e533af847e38646a6c778b41ed84f519f1fd85c`.
- **Atomic-edit observations:** service/daemon operational evidence now counts
  workspace-edit requests, refusals, requested/changed/unchanged documents and
  changed/all-no-op transactions. Exact request replay does not double-count.
- **Ordered provider replacement:** the authenticated timed actor can suspend
  with command/control/in-flight queues empty while retaining broker-pending
  provider work, recover its exact broker/port product, replay snapshots into a
  ready candidate, reconcile one provider epoch, invalidate pending requests
  and their deadlines without query replay, and resume with accumulated usage.
  Old senders remain fenced; reconciliation divergence yields a non-resumable
  fused cleanup owner retaining both provider products.
- **Explicit standalone replacement:** semantic-local now advertises only the
  opt-in `--restart-on-sighup` path. It classifies SIGHUP as
  `OperatorRequested`, waits until no authenticated stream is active, suspends
  the ordered actor, launches and qualifies a fresh pinned-ready process,
  installs it, requires terminal displaced-provider cleanup, and resumes with a
  fresh sender. The default remains restart-disabled; uncertainty fuses and
  exits, and queries are never replayed. Automatic provider-failure/unavailable
  restart remains deliberately absent.
- **Maintained clean-source gate:** `make check-clean-release-candidate` now
  clones exact detached local Vellum/Keystone commits without hard links,
  excludes dirty working-tree content, forces Cargo offline with exact local
  URL rewrites, builds twice and retains only the candidate plus content-safe
  equivalence evidence. Existing output is refused non-destructively. The exact
  maintained gate passed against Vellum `b767f78979f86cae93abdc3951b2ac0f760fd3ee`
  and Keystone `3486252cdbaeb2d784756076f43ef6157ed0b7f8`, producing
  byte-equivalent schema-4 candidates with manifest digest
  `a9db092c9c42884af17581b2635bb50b00120679a427893b2b959b008bc52b5d`.
- **Activation residue reconciliation:** the activator can read-only reconcile
  one exact canonical journal after crash/unknown outcome, revalidating
  approval, qualification, payloads, modes, ownership, contiguous sequence and
  the allowed activation prefix. It distinguishes durable terminal,
  prepared-only, pending, explicit unknown and confirmed-effect-without-terminal
  states while always reporting `automaticRetryAuthorized: false` and
  `launchdEffect: not_attempted`.
- **Real ordered LSP candidate:** a separately qualified fresh-started pinned
  process can be driven to readiness, installed through the suspended ordered
  runtime and kept hidden until the displaced real provider has a terminal
  cleanup receipt. A real process fixture proves generation 1 to 11, snapshot
  replay, old-sender fencing, usage continuity and graceful cleanup of both
  displaced and current providers. The standalone opt-in SIGHUP composition
  uses this same controller/candidate/cleanup product with finite bounds.
- **Closed MCP edit result:** `vellum.workspace_edit` discovery now publishes a
  closed bounded success/error schema rather than an unconstrained object.
- **Failed-provider restart product:** semantic-local now retains the exact
  provider-drive terminal class and exposes a typed failed-actor recovery owner
  that preserves the broker, scheduler, snapshots, pending work and provider
  cleanup authority. The default-off `--restart-on-provider-failure` path
  invalidates pending queries/deadlines without replay, installs one separately
  qualified pinned-ready provider, and resumes only after whole-group cleanup
  plus leader reap. A real crashed-process fixture qualifies the complete path;
  generic actor failure never becomes restart authority.
- **Downloadable GitHub candidate:** the workflow no longer uploads a raw
  directory whose executable modes are flattened by artifact transport. It
  uploads one deterministic canonical USTAR containing exactly the candidate
  and byte-equivalence evidence, deliberately flattens the outer mode to
  `0644`, then safely re-extracts, validates and smokes all native payloads.
  Traversal, links, extras, mode loss, malformed headers, corruption, padding
  and trailing content are refused. The exact clean-source gate passed at
  Vellum `282785f66112f1ca32b95ae60ffb39ffbeb7b4d1`; its flattened-mode download
  product reverified all six files and native smokes with manifest digest
  `f2ae59d60447f0a61591e2ba5e8acf40e62d58c918ea909304f5e950bf31e453`.
- **MCP catalog closure:** the extraction contract, live daemon qualification
  and connector documentation now agree on six ordinary tools plus four
  authenticated durable additions, including `vellum.workspace_edit` and
  `vellum.durable_status`. The official-client scenario is updated in the
  intentionally untracked qualification workspace, but remains unrunnable
  until an exact reviewed npm lock/SBOM for client 2.4.0 is available; no
  TypeScript enters the shipping Rust product.

## Architectural decisions

### TUI, GUI, and MCP are peers

`vellum-tui`, `vellum-gui-macos`, and `vellum-mcp` are peer inbound projections with independent release lifecycles. None belongs inside Vellum's semantic core, and none defines the canonical protocol.

```text
vellum-tui ---------\
vellum-gui-macos ----> vellum-sdk-client -> vellum protocol/service -> engine
vellum-mcp ---------/
composite workbench -/
```

This makes the repository boundary agree with the existing architectural law that human interfaces and agent connectors are peer projections.

Vellum owns:

- The source, document, revision, history, transaction, and persistence semantics.
- The editor substrate and lawful edit algebra.
- The native protocol, service boundary, transport contracts, and client SDK.
- Outbound adapters required to realize core effects.
- Initially, the local daemon composition root.

Peer repositories own:

- Their projection-specific interaction model and policy.
- Their executable, packaging, tests, documentation, and release lifecycle.
- Translation between their public surface and Vellum's client protocol.

The local daemon remains in Vellum until its operational and release lifecycle demonstrably diverges. If it does, extract it as a sibling service application without moving service semantics or the client contract out of Vellum.

The presentation and process boundaries are explicit roadmap requirements:

- `vellum-tui` remains a reusable editor presentation library. It owns editor
  projection, local input, confirmations, and typed lifecycle requests, but not
  argument parsing, terminal setup/restoration, daemon startup, global shortcut
  policy, or process exit.
- The standalone Vellum CLI/application remains a separate composition root. It
  may run the reusable TUI as a non-headless editor, or expose bounded headless
  document/workspace operations through the same client and service contracts.
- Workbench imports the TUI library rather than the Vellum CLI shell. Its own CLI
  may attach to an existing Vellum service or host Vellum's service runtime with
  peer runtimes beneath one executable and lifecycle, without merging source,
  document, history, persistence, or authorization authority.
- Workbench headless mode composes the same service/client graph without
  constructing terminal, focus, selection, or cursor state.

Qualify standalone interactive, standalone headless, Workbench interactive,
and Workbench headless paths with the same revision witnesses, dirty-state exit
law, durable-operation lookup, authorization, and outcome-unknown fencing.

### The SDK is the dependency boundary

Peer applications consume `vellum-sdk-client`; they do not reach into engine internals. The SDK remains in the Vellum repository because it is the typed projection of Vellum's protocol and compatibility contract, not an application.

The dependency direction is one-way:

```text
peer application -> SDK/client -> wire protocol -> service/host -> engine
```

Core packages must never depend on TUI, GUI, MCP, orchestration, or workbench packages.

### Editing and language intelligence are distinct capabilities

Vellum should not imitate a headless IDE. Buffer, cursor, and selection emulation add synchronization state without adding much agent value. Vellum's value is reliable source mutation, revisioned history, persistence, and coordination.

Language intelligence should be a separate sovereign component with a narrow semantic-query surface. It consumes immutable document snapshots and revisions; it does not own editing state. Its initial surface should be limited to roughly five to eight operations:

1. Symbol search.
2. Definition lookup.
3. Reference lookup.
4. Implementation and call-hierarchy lookup.
5. Diagnostics.
6. Type or hover information at a position.
7. Refactoring or rename preparation.
8. Refactoring or rename application through Vellum.

LSP is the baseline backend. PSI or another richer semantic substrate can be an optional adapter where it provides meaningful additional guarantees.

Runtime evaluation is a separate, higher-authority capability:

- Clojure static intelligence: `clojure-lsp` and `clj-kondo`.
- Clojure runtime intelligence: an explicitly authorized nREPL adapter.
- BEAM static intelligence: a suitable language-server adapter.
- BEAM runtime intelligence: an explicitly authorized remote shell or IEx adapter.

Static semantic queries and runtime evaluation must not share an implicit authority boundary.

### Semantic refactoring is a witnessed transaction

Refactoring is a two-phase composition:

1. The semantic component prepares a change plan against exact document revisions and records provider/version/index provenance.
2. Vellum validates those witnesses and atomically applies the plan in memory,
   or refuses it as stale.
3. Persistence, when requested, is a subsequent per-document effect vector
   whose partial and unknown outcomes are reported without a false cross-file
   atomicity claim.

Results must expose source revision, index generation, provider identity and version, freshness, and any unknown or incomplete state. Silent resynchronization is forbidden.

## Agent-facing MCP shape

The MCP projection should remain small and capability-discovered so its tool surface does not consume unnecessary model context. It should expose Vellum's semantics, not editor UI state.

Candidate capabilities:

- Enumerate workspaces and documents using stable identifiers.
- Create an empty workspace.
- Add or remove a file or directory from a workspace.
- Read a revisioned document snapshot.
- Prepare and apply revision-witnessed patches or replacements.
- Save or create an unbound exclusive copy with explicit authority and outcome
  receipts; Save As/rebinding remains a separate unexposed product.
- Inspect transaction/history state needed for coordination and recovery.

The MCP server must not expose cursor movement, selection manipulation, or a generic headless-editor command surface. Language-intelligence queries belong to their own peer service and MCP projection, though an orchestration harness may compose the two for an agent.

MCP is an inbound projection, not the canonical internal protocol. Native applications and composite workbenches should use the typed client directly.

## MCP extraction plan

1. Freeze the current `vellum-mcp` behavior with protocol, golden, and compatibility tests.
2. Create a sibling `vellum-mcp` repository with its own workspace, executable, documentation, test suite, and release policy.
3. Preserve package and binary names during migration.
4. Initially use a local path dependency on the sibling Vellum checkout; replace it with an immutable Git revision or vendored release input for distribution.
5. Move connector-specific tests and documentation to the peer repository.
6. Add a Vellum architecture check that rejects inbound application or connector packages in the core workspace.
7. Prove parity with the existing connector, including an official MCP client interoperability test.
8. Remove the in-tree connector only after the peer passes the release gates.
9. Keep repository histories and remotes local until remote publication is explicitly authorized.

This pattern should become uniform across engines: independently releasable inbound applications live as sibling repositories; core protocols, services, clients, and semantic engines remain together as separate crates in the engine repository.

## Production gaps to close

### Current acceptance order

Scope correction (2026-11-12): platform-specific realizations are not part of
the current delivery goal. macOS App Sandbox/XPC, launchd evolution, GUI
packaging/signing/notarization, and future Linux/Windows confinement backends
remain separately owned follow-on work. The current goal covers the portable
engine/service/client/SDK and agent protocol surfaces, operational lifecycle,
and reproducible GitHub-source distribution. Platform work may supply evidence
about an interface but must not redirect this acceptance order or move a
target-specific mechanism into the portable engine.

Vellum's substrate remains ahead of its useful agent and product surface, but
the first three closure steps in the earlier order are now implemented and
qualified: workspace identity/membership/change semantics are public through
the SDK and MCP peer; membership, save and replace-and-save have stable
authenticated operation identities and historical receipts; and the shipped
credential composition has lost-reply restart evidence. Remaining work must
continue from the actual boundary, in this order. The thirteenth wave also
closes authenticated create-exclusive export through daemon and MCP without
promoting it to Save As:

1. close operational lifecycle and reproducible GitHub-source distribution,
   including the remaining immutable Keystone source/API evidence;
2. preserve the qualified pinned-provider-to-stale-safe-apply transaction as a
   portable, unshipped semantic composition; platform confinement and resource
   enforcement are deferred to separately owned peers; and
3. add remote transport only under a separately specified authentication,
   confidentiality, authorization, replay and migration contract.

Later layers may not be used to paper over an earlier missing contract. In
particular, a language server is not workspace membership, MCP is not the
canonical engine protocol, a socket owner is not an authenticated principal,
and a retry is not a durable receipt lookup.

### Durable editing

Ordinary `Replace` intentionally mutates only the in-memory workspace. The
additive `ReplaceAndSave` path returns explicit persistence evidence. The
persistent daemon and authenticated MCP composition now journal and retrieve a
stable effect receipt across process loss without repeating the effect;
owner-only operation remains deliberately non-durable. Authenticated
create-exclusive copy export is now exposed through that same operation
lifecycle without changing the document binding. Save As/rebinding remains a
different, unexposed product requiring independent Persist and Rebind
authority.

Required work:

- Preserve the qualified export authority/recovery laws and closed durable,
  not-durable, unknown and not-attempted evidence through release packaging.
- Define Save As/rebinding separately; never infer it from a successful copy
  export or reuse the create-exclusive receipt as binding evidence.
- Add product-level crash/restart qualification through the shipped CLI and MCP
  peer, not only raw native transport tests.

### Complete workspace semantics

Agent and human clients need the same complete workspace model:

- Stable workspace and document identifiers.
- Empty startup.
- File and directory discovery and enumeration.
- Add, remove, open, close, and select operations.
- Explicit limits and pagination rather than a hidden fixed initial-file ceiling.
- Revisioned notifications suitable for multiple concurrent clients.

MCP resources should be built only after stable identifiers and lifecycle semantics exist.

### Explicit authority

Same-user access to a local socket is not sufficient write authorization.

- Default daemon and MCP sessions to read-only.
- Require explicit `--allow-writes` or an equivalent scoped grant.
- Separate read, edit, persist, filesystem-discovery, semantic-query, and runtime-evaluation capabilities.
- Make authority tighten-only within a session.
- Record authorization decisions without logging source contents or secrets.
- Leave room for Keystone/Aegis policy without coupling the engine to a particular policy service.

### Self-contained GitHub distribution

A clean Git archive must build without relying on sibling directories that happen to exist on a developer machine.

- Replace relative Keystone dependencies with immutable Git revisions, vendored inputs, or a signed release bundle.
- Pin the Rust toolchain and lock dependencies.
- Support `cargo install --git ... --locked` for the intended binaries when authorized.
- Produce macOS release artifacts with checksums and provenance.
- Keep every workspace package `publish = false` until crates.io publication is explicitly approved.
- Correct stale crate paths in documentation and examples.

### Release engineering and supply chain

- Add CI for format, lint, tests, documentation, architecture rules, and clean-clone installation.
- Add repository and package licenses, `SECURITY.md`, and `CHANGELOG.md`.
- Enforce `cargo deny`, dependency policy, advisory checks, and an SBOM/provenance path.
- Treat lockfiles and generated protocol fixtures as reviewed release inputs.
- Run Unicode, clipboard, filesystem, and platform qualification suites where applicable.
- Build documentation with warnings denied.
- Establish a semver/API baseline for public crates and the wire protocol.

### Service operations

- Graceful SIGTERM handling, draining, and bounded shutdown.
- Stale-socket recovery and a single-process lock.
- Separate liveness and readiness semantics.
- Structured, content-safe logs and trace correlation.
- RED metrics for requests and USE metrics for resources, plus latency, traffic, errors, and saturation golden signals.
- Configurable and observable resource limits.
- Connector restart ceilings and backoff rather than unbounded restart loops.
- Launchd packaging for the local background service.
- Slow-client, overload, cancellation, and backpressure evidence.

### Artifact lifecycle

Adopt the useful Axon release pattern:

- Immutable artifact manifest.
- Checksums and provenance.
- Captured effective configuration.
- Pre-activation smoke tests.
- Corruption and incompatibility rejection.
- Readiness-gated activation.
- Prior-version selection and rollback.

Adopt Engram's secure-default lessons where applicable: per-install secrets, explicit folder trust, a closed configuration schema, and a security configuration lattice that can only reduce authority.

## Lessons carried forward from Engram and Axon

These are adopted practices, not ornamental references. Their tests and gates should be translated into Rust and Vellum's own boundaries. A lesson includes the limitation discovered by the source project; copying the mechanism while dropping its stated ceiling would be a regression.

### Engram: grounded claims and honest limits

- Classify consequential claims by provenance: externally grounded, derived, observed, designed, or still open. A design choice must not be presented as a theorem, and an analogy must not be presented as an isomorphism.
- Compute confidence from the weakest supporting evidence. A claim with an unverified dependency remains unverified even when the rest of its chain is strong.
- Keep architecture, security, operations, and privacy as cross-cutting overlays over one semantic substrate. An overlay points to and constrains the owning components; it does not duplicate their domain models.
- Give every invariant a provenance path back to a specification, implementation boundary, and executable observation. Mechanically check indexes and traceability so an apparently complete roadmap cannot silently orphan requirements.
- Record known ceilings as executable tripwires. A security test may assert that a token is integrity-protected but not confidential, that loopback is scope rather than isolation, or that a sandbox is currently disabled. If later hardening makes such a test fail, the claim and threat model must be rewritten rather than the evidence deleted.
- State the trust ceiling explicitly. Same-user local processes are one trust domain unless OS isolation establishes otherwise; local sockets, loopback, file modes, and bearer tokens must never be described as mutual isolation.

### Engram: capability and configuration discipline

- Authority is an unforgeable, scoped capability rather than ambient process access. Safety axes compose by meet and can only tighten; capability inventories compose by join only where widening is explicitly intended.
- Complete mediation belongs at a small reference-monitor boundary. For Vellum, all filesystem mutation must pass through one persistence authority, all external process/network access through named outbound adapters, and all agent write operations through the same revision and authorization checks regardless of projection.
- Add exact trusted-computing-base closure tests. The test must fail both when a forbidden caller gains an effect mechanism and when the declared trusted set contains a stale member. The detector must have adversarial self-tests so an empty or incomplete scan cannot pass ceremonially.
- Treat such lints as mechanism-specific evidence, not proof of total confinement. A lint that closes one HTTP client, filesystem facade, or process launcher does not close alternative libraries, raw sockets, FFI, or subprocess escape paths.
- Keep workspace trust outside workspace-controlled configuration. An untrusted directory may supply readable source content, but its local configuration, hooks, semantic-server commands, and executable metadata remain inert until registered by an operator-controlled store. A workspace must not be able to trust itself.
- Decode trust and authority state fail-closed. Missing, corrupt, partially valid, or unrecognized trust registries produce the empty authority set rather than a best-effort subset.
- Use a closed configuration schema with bounded key vocabularies, safe wire projections, sensitivity metadata, and lifecycle metadata. Unknown keys fail clearly; credentials project only `set` or `unset`; reading configuration never implies live edit authority.
- Generate per-install roots with owner-only permissions where credentials are necessary, rotate or expire derived credentials, compare authenticators safely, and never place secrets in token claims. Signed claims provide integrity, not secrecy.
- Keep real credentials out of child-process environments where possible. Use just-in-time credential mediation bound to the caller's identity and grant, pin the authorized destination, reject redirects that could replay credentials, and log only content-safe attribution.

### Axon: durable effects and truthful outcomes

- Separate pure decisions from effects and journal the transition between them. Persist intent and identity before dispatching an effect; persist its receipt before advancing dependent state.
- Retry a pure or idempotent action within a recorded bound. Retry a non-idempotent effect only with an idempotency key or an authoritative probe. After a crash or timeout, represent an ambiguous effect as `unknown` and reconcile it; never infer failure merely because a reply was lost.
- Use an outbox for durable delivery. Reconnect or process loss must redeliver the recorded result rather than recompute a possibly effectful operation.
- Make crash recovery monotonic: resume or verify the same frozen plan, authority, policy, and revisions. Recovery must not widen scope or silently adopt current policy.
- Model external work as convergent, not atomic. Local success cannot close an obligation owned by a language server, filesystem, Git repository, telemetry collector, backup, provider, or other process.
- Distinguish `refused`, `blocked`, `pending`, `converging`, `partial`, `unknown`, and `complete`. Completion requires terminal evidence from every in-scope owning boundary.
- Preserve disclosed seams for dependencies that do not yet exist. Inject an interface and name the missing integration; do not speculate an implementation or make test startup depend on it.

### Axon: lifecycle and data lineage

- Maintain a machine-checked inventory of authoritative, derivative, operational, client, and external surfaces. Source scanning helps find surfaces but is not proof that the inventory is complete.
- Distinguish catalog removal, tombstoning, payload collection, retention compaction, process expiry, external convergence, and physical sanitization. One outcome is never evidence for another.
- Resolve identity, revisions, authority, lineage closure, blockers, holds, external obligations, policy version, and expected evidence into an immutable dry-run plan before destructive mutation.
- Verify an effect through its owning read boundary. Absence from an index or catalog is insufficient when the payload or derivative may remain readable elsewhere.
- Keep receipts content-minimized: identities, digests, policy and authority references, timestamps, outcomes, exclusions, and residual obligations rather than the edited or erased source itself.
- Treat Git worktree removal, branch deletion, history rewriting, remote convergence, backup expiry, and media sanitization as separate authorities and claims. Vellum must not acquire destructive repository behavior merely because it can edit files in a repository.
- Revoke authority before retiring identity, reserve retired identifiers, and preserve governed residual references when immutable evidence cannot be rewritten safely.

### Axon: operational evidence

- Keep unit and property tests independent of live external resources. Language servers, collectors, policy services, and network listeners are injected or gated in normal tests.
- Complement those tests with reusable live scripts against the real compiled binary, real transport, and real service. Each script has deterministic setup/cleanup, bounded waits, explicit `PASS`/`FAIL`, and a non-zero failure exit. One-off manual sessions do not count as regression evidence.
- Separate constant-cost liveness from acceptance readiness. Liveness performs no filesystem, registry, adapter, collector, or network probes. Readiness is bounded, reports closed component states, and uses cached owner reports rather than introducing new probe-time dependencies.
- Draining immediately removes readiness while retaining liveness. One authenticated owner coordinates a bounded drain; timeouts and abandoned clients remain visible; shutdown is reported only after the process is actually gone.
- Telemetry export is optional and non-blocking. Bounded queues, batching, timeouts, and drop counters prevent an unavailable collector from blocking startup, request handling, readiness, or shutdown.
- Bound metric attributes with closed vocabularies and explicit `unknown`/`other` values. User strings, paths, document IDs, symbols, errors, and source contents must not become metric dimensions.
- Define saturation only where a real bounded resource and denominator exist. If no queue or capacity bound exists, report saturation as unavailable rather than fabricating it from latency or CPU use.
- Correlate the whole client/server lifecycle: enqueue, queue wait, dispatch, server handling, reply, timeout, cancellation, abandonment, and stale-reply discard. Keep client and server error denominators distinct.

### Axon: immutable release activation

- Enumerate the complete artifact payload into a canonical manifest containing paths, sizes, modes, hashes, target, version, and effective configuration digest.
- Reject symlinks, unexpected file types, missing required files, non-canonical manifests, payload mismatches, writable retained artifacts, and corruption. Artifact verification must re-read canonical disk bytes.
- Normalize permissions, freeze payloads, identify the artifact by its manifest digest, and publish it with a same-filesystem atomic rename. Never overwrite an existing content-addressed artifact.
- Keep construction, retention, selection, and activation as separate effects. Atomic selection records exact `current` and `prior` identities but does not imply the process started successfully.
- Bind the running PID to the selected artifact identity. Activation verifies prior and candidate, authenticates and drains the prior process, starts the candidate, waits for readiness, and only then promotes selection.
- If the candidate is unhealthy, stop it and restart the unchanged prior selection. If rollback itself fails, report operator intervention rather than claiming recovery.
- State explicitly that code/config rollback is not a storage-migration rollback. Schema and durable-data compatibility require their own forward/backward plan and evidence.

### Axon: specification and verification discipline

- Maintain a requirements baseline with stable identifiers and bidirectional traceability among requirement, state machine, implementation, unit/property test, and live evidence.
- Correct the specification before implementing a newly discovered state, transition, or failure class. Do not hide a specification gap in an implementation special case.
- Use state machines for retries, branching, ordering, fan-out, fallback, lifecycle, and recovery behavior. Verify state and transition coverage rather than only terminal outputs.
- Make boundedness structural: closed vocabularies, finite state, queue and payload limits, deadlines, retry ceilings, concurrency ceilings, and explicit fuel for recursive or iterative evaluation.
- Validate path grants independently at use time. Reject absolute paths and `..`; distinguish exact-file grants from explicitly marked directory-prefix grants; never rely solely on validation performed when a grant was created.
- Sketch and test presentation layout through the real reducer/render pipeline before wiring expensive protocol changes. Preserve pure UI state and effect planning so TUI and GUI libraries remain composable.

### Lessons not to copy blindly

- A same-user token or Unix socket is authentication within one local trust domain, not isolation from peer processes.
- A signed bearer token is not an encrypted envelope.
- `local_only` is an egress-scope statement, not a confidentiality guarantee.
- A static TCB lint closes only the mechanisms it recognizes.
- A content-addressed release artifact does not solve durable-data migration.
- A tombstone, deleted catalog record, stopped process, or expired cache does not prove data erasure.
- Optional observability must never become an availability dependency.
- A passing unit suite does not replace a reusable real-transport acceptance journey, and a live journey does not replace deterministic unit, property, and model evidence.

## Lessons from established editor ecosystems

These precedents reinforce the peer-projection architecture. Their durable common shape is a long-lived document engine, independent clients, revisioned change streams, semantic providers, and explicit extension boundaries. Vellum should adopt that shape without importing an editor's UI state, ambient plugin authority, or historical compatibility accidents.

### JetBrains Platform

- Treat a semantic program model as a richer optional backend, not as Vellum's document model. PSI demonstrates the value of typed syntax trees, reference resolution, call hierarchy, inspections, and meaning-preserving refactoring; the text and persistence authority still belongs to Vellum.
- Keep semantic reads cancellable and invalidate them when writes intervene. JetBrains non-blocking read actions restart or cancel around write activity; Vellum semantic queries should likewise bind to exact revisions and terminate stale work promptly.
- Prepare expensive transformations outside the write critical section. Validate and compute a refactoring plan against immutable snapshots, then keep Vellum's apply-and-persist transaction minimal.
- Represent index readiness as a first-class state. JetBrains distinguishes index-unavailable “dumb mode” from “smart mode” and allows only explicitly qualified features while indexes rebuild. Vellum's semantic peer should declare which queries are available in each index state and never silently return degraded results as complete.
- Use compact declaration indexes where a full tree is unnecessary. A symbol index can retain externally visible declarations and provenance without keeping every full parse tree resident.
- Preserve command and write-action semantics around semantic mutations. A refactoring should be one named, auditable command with undo/replay identity rather than an unstructured series of file writes.
- Follow the useful parts of JetBrains' current MCP surface: tools are individually exposable, expensive trees are bounded and pageable, ambiguous symbols return candidates rather than guesses, results disclose timeout/incomplete states, and router-only tools can stay out of the default model context.
- Keep destructive or execution authority separate from semantic analysis. JetBrains requires explicit MCP enablement and separately exposes a confirmation-bypassing “brave mode”; Vellum should retain confirmation and scoped grants rather than offer a broad equivalent mode.
- Treat the JetBrains backend as an adapter with a higher resource and startup cost. Its richer PSI behavior is especially valuable for JVM languages, but it must not become a required dependency for Vellum, MCP, or the baseline semantic protocol.

### Visual Studio Code

- Separate the language client from the language-analysis process. VS Code's LSP architecture validates a reusable semantic server behind multiple editors and supports incremental open/change/close synchronization rather than repeated whole-document transfer.
- Define semantic features as data providers—hover, definition, references, symbols, diagnostics, rename, and code actions—while projections own presentation. This is the correct boundary for the proposed semantic-intelligence peer.
- Require error-tolerant analysis. Source is routinely incomplete between edits; a semantic provider that accepts only buildable programs cannot support a tight agent or human feedback loop.
- Place extensions according to the resources they require: UI-local, workspace-local, remote, or web. Vellum peers should similarly declare execution locality and must not assume that the UI, workspace filesystem, semantic server, and engine share a machine.
- Isolate extension execution from the UI path and activate capabilities lazily. Startup and ordinary editing must not load every adapter or let a slow extension block presentation.
- Centralize workspace trust and support a useful restricted mode. Reading and manually editing source may remain available while workspace-supplied commands, settings, tasks, language-server launch commands, and executable hooks remain disabled.
- Enforce trust in the command implementation, not only by hiding UI. VS Code explicitly notes that a hidden command can still be invoked; Vellum authorization must therefore be checked at the effect boundary for every projection.
- Model workspaces as zero or more resource roots and allow untitled documents. Filesystem access should use URI-like resource identities and provider capabilities so local, remote, virtual, read-only, and generated resources do not collapse into one path assumption.
- Preserve filesystem-provider facts such as case sensitivity, read-only status, symbolic links, modification time, size, and watch behavior. These facts participate in revision and stale-write decisions.
- Adopt multi-resource edit plans, but strengthen their guarantees. VS Code documents all-or-nothing behavior for text-only workspace edits while resource create/delete sequences can stop after failure; Vellum must expose whether a mixed text/filesystem transaction is atomic, compensated, partial, or unknown.
- Separate dirty state, backup/recovery state, save, revert, and undo. A recovery snapshot protects work after a crash but does not claim that the authoritative file was saved.
- Do not mistake the extension host for a security sandbox. Extensions can access files, processes, and networks; process separation protects responsiveness and failure containment more than confidentiality.

### Neovim

- Make TUI, GUI, embedders, remote plugins, and automation clients peers over one documented API. Neovim's external UI protocol is direct precedent for keeping `vellum-tui`, `vellum-gui-macos`, and `vellum-mcp` outside the engine repository.
- Expose functions and events, not a serialized UI command loop. Clients call typed operations and subscribe to changes; the engine remains usable headlessly or embedded as a library.
- Publish machine-readable API metadata: protocol level, compatible baseline, prerelease status, function signatures, typed handles, events, UI capabilities, introduction version, and deprecation version.
- Prefer additive compatibility. New functions, event fields, and options should be optional; released signatures and field meanings should remain stable. Unknown optional fields are ignored, while unknown required capabilities fail clearly.
- Require every client to self-identify with name, version, kind, process identity, and advertised methods. This improves orchestration, compatibility diagnosis, audit attribution, and per-client policy.
- Use opaque, type-distinct handles for documents, workspaces, subscriptions, and transactions. Do not let an integer or string for one entity be silently accepted as another entity type.
- Stream buffer changes with a monotonic revision witness. Neovim's `changedtick` is direct precedent for checking that no intervening edit occurred before applying a client response.
- Treat reload and detach as synchronization discontinuities. A client must discard incremental assumptions and fetch a fresh snapshot after external reload, unload, or missed events.
- Namespace moving annotations independently from source text. Neovim extmarks show how diagnostics, semantic ranges, review comments, and agent intentions can track edits without becoming document content or colliding across providers.
- Negotiate presentation capabilities among attached UIs. The safe default should be the meet of capabilities supported by every active projection; a projection-specific override must be explicit and must not change engine semantics.
- Support attach, detach, reconnect, and headless continuation as normal lifecycle states. UI loss must not imply engine loss or forced persistence.
- Keep remote plugins in separate processes for fault and latency containment, but do not treat RPC access as least authority by itself. Named pipes or local sockets are preferable to unauthenticated localhost TCP, and method-level grants remain necessary.

### Emacs

- Keep buffers independent from files. A buffer may visit a file, remain untitled, represent generated or process content, or have a custom persistence function. This supports Vellum's empty startup and later addition of files and directories.
- Maintain explicit dirty state and monotonic modification ticks. Saving clears dirty state; an edit increments a witness that semantic and remote clients can use to refuse stale work.
- Provide real atomic change groups across one or several buffers. Prepare, activate, accept, and cancel are direct precedent for Vellum's multi-document transaction algebra and for grouping a refactoring into one undo unit.
- Preserve the caveat: in-memory atomic buffer mutation is not durable filesystem atomicity. Vellum must separately report persistence and recovery outcomes.
- Treat undo boundaries as semantic command boundaries. One agent patch or refactoring should normally be one undoable command, while intentionally separable replacements may expose finer boundaries.
- Detect external writers with file locks and supersession checks, but regard locks as coordination evidence rather than absolute exclusion. Revision/content witnesses remain authoritative before save.
- Separate autosave recovery from explicit save. Periodic recovery artifacts should bound crash loss without pretending to update the visited file.
- Run the engine as a daemon with attachable clients. Frames and clients may come and go while buffers, subprocesses, and unsaved work continue under an explicit lifecycle policy.
- Represent subprocesses and connections as owned asynchronous objects with input, output, status, cancellation, and termination. Vellum adapters need bounded equivalents rather than blocking the document engine.
- Use hooks as typed lifecycle events, but avoid Emacs' ambient shared mutation as the extension model. Vellum events should have declared ordering, authority, failure behavior, and resource bounds.
- Keep secret lookup behind pluggable credential backends rather than storing credentials in workspace files or editor state.

### Architectural consequences for Vellum

1. The engine is a headless, embeddable document and transaction authority; it is not a TUI, GUI, MCP server, or language server.
2. TUI, GUI, MCP, composite workbenches, and future automation clients attach through the SDK and protocol as peer projections.
3. Semantic intelligence is a peer service with LSP as its baseline adapter and richer PSI-like adapters where justified.
4. Documents carry stable typed identity, monotonic revision, dirty/persisted state, and an event stream with explicit resnapshot boundaries.
5. Multi-document text edits are atomic in memory and form one undoable transaction. Filesystem persistence has a separate, explicit outcome algebra.
6. Diagnostics, semantic ranges, comments, and agent intentions are revision-bound, provider-namespaced annotations rather than source mutations.
7. Workspaces may be empty, untitled, multi-root, local, remote, virtual, or read-only. Resource-provider capabilities replace local-path assumptions above the filesystem adapter.
8. Workspace trust centrally gates executable configuration, semantic-server launch, hooks, tasks, process execution, and network access while preserving safe browsing and editing.
9. Protocol metadata and capability discovery allow clients to negotiate optional behavior without bloating the default MCP tool list.
10. MCP exposes the small common semantic surface directly and places uncommon or expensive tools behind capability discovery or a bounded router.
11. Every query and edit carries cancellation, timeout, bounds, revision/index provenance, and an explicit complete/partial/stale/unavailable result.
12. Recovery snapshots, undo history, dirty buffers, persisted files, and release artifacts remain distinct states with distinct evidence.

### Official precedent reviewed

- JetBrains: [threading model](https://plugins.jetbrains.com/docs/intellij/threading-model.html), [PSI modification](https://plugins.jetbrains.com/docs/intellij/modifying-psi.html), [indexes and dumb mode](https://plugins.jetbrains.com/docs/intellij/indexing-and-psi-stubs.html), and [IDE MCP server](https://www.jetbrains.com/help/idea/mcp-server.html).
- VS Code: [extension hosts](https://code.visualstudio.com/api/advanced-topics/extension-host), [workspace trust](https://code.visualstudio.com/api/extension-guides/workspace-trust), [language-server architecture](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide), and [workspace/filesystem/edit APIs](https://code.visualstudio.com/api/references/vscode-api).
- Neovim: [RPC and API contract](https://neovim.io/doc/user/api/), [external UI protocol](https://neovim.io/doc/user/api-ui-events/), [LSP client framework](https://neovim.io/doc/user/lsp.html), and [GUI/client-server lifecycle](https://neovim.io/doc/user/gui/).
- Emacs: [server/daemon clients](https://www.gnu.org/software/emacs/manual/html_node/emacs/Emacs-Server.html), [atomic change groups](https://www.gnu.org/software/emacs/manual/html_node/elisp/Atomic-Changes.html), [buffer modification state](https://www.gnu.org/software/emacs/manual/html_node/elisp/Buffer-Modification.html), [saving buffers](https://www.gnu.org/software/emacs/manual/html_node/elisp/Saving-Buffers.html), and [autosave recovery](https://www.gnu.org/software/emacs/manual/html_node/elisp/Auto_002dSaving.html).

## Broader-workspace reuse and Keystone extraction

The peer repositories contain several implementations that close Vellum gaps, but reuse has three distinct forms:

1. Compose with a sovereign peer through its public protocol.
2. Extract a small, law-bearing mechanism into Keystone after two consumers agree on semantics.
3. Reuse doctrine, fixtures, or verification tooling without creating a runtime dependency.

Similar-looking code is not enough to justify Keystone extraction. The failure meaning, durability witness, authority boundary, bounds, and recovery behavior must also match. Keystone must not become a universal engine, policy service, workflow language, or command bus.

### Reuse peer engines without absorbing them

| Source | Reuse for Vellum | Boundary that remains sovereign |
| --- | --- | --- |
| Aegis | Scoped, attenuating grants for read, edit, persist, filesystem discovery, semantic query, and runtime evaluation; revocation and redacted decision evidence | Authentication and principal establishment remain host concerns; Aegis owns authority semantics and grants; Vellum owns document actions and outcomes |
| Hermes | Durable Vellum change feed, ordered replay, inbox deduplication, and independent subscriber cursors | Hermes owns ledger and cursor semantics; Vellum owns documents, revisions, saves, and source persistence |
| Moira | Multi-engine coordination among Vellum, Chronos, Daimon, Agora, and Hermes, including compensation and uncertain effects | Moira owns workflow and recovery policy; neither Vellum nor Keystone becomes the orchestrator |
| Chronos | Scheduling and deadline activation through its public service; local-daemon operational precedent | Chronos owns task and scheduling semantics |
| Daimon | Authorized execution of agent work and explicit execution budgets | Daimon owns provider and execution semantics; Vellum applies witnessed edits but does not become an agent runtime |
| Agora | Human/agent coordination and chat through a peer service and composable projection | Agora owns rooms, messages, membership, and moderation |
| Rune and Dendrite | Formal flow/FSM validation, architecture laws, adversarial review, MCP query-service precedent, and provenance-bearing specification evidence | Rune's DSL/corpus and Dendrite's query service remain development and assurance systems, not Keystone runtime crates |

Concrete cross-engine adapters should live at a composition boundary and consume public clients. Vellum's engine must not import peer internals. In particular, an Aegis integration maps a host-established principal and Vellum action to an authorization decision; the resulting evidence is attribution, not a reusable bearer credential.

### Keystone mechanisms ready for extraction or qualification

#### Receipt-gated publication and uncertainty fencing

Aegis and Hermes independently implement the same high-value host discipline:

1. Prepare a candidate state and outcome without publishing either.
2. compare-and-exchange a checkpoint against an exact revision.
3. Validate the durable receipt.
4. Publish the new engine state and outcome only after a matching receipt.
5. Permanently fence the in-memory host after a weak, mismatched, indeterminate, or failed persistence result.

Extract the mechanical portion into Keystone: a prepared transition, expected revision, receipt validation, publish-after-receipt gate, and explicit fenced state. Keep checkpoint encoding, recovery policy, replay/refusal rules, domain errors, and signal vocabulary in Aegis and Hermes. Vellum should then use the qualified mechanism for durable service metadata and any command whose success depends on a durable receipt.

#### Private local resources and phase-aware IPC

Chronos, Vellum, Keystone's directory store, and Moira each hold part of a common local-service safety contract. Keystone should consolidate the agreed Unix mechanisms behind small crates or modules:

- Create or validate an exact private directory without recursively changing pre-existing parents.
- Open lock and resource files with no-follow behavior; verify type, owner, mode, link count, and device/inode identity.
- Hold a lifetime advisory lock and detect replacement of an owned resource.
- Bind, restrict, verify, and clean up a Unix socket only when its identity still matches.
- Read and write bounded length-prefixed frames with deadlines.
- Distinguish a failure before transmission (`not_sent`) from a failure after transmission begins (`outcome_unknown`).
- Return content-safe evidence and sticky saturation observations.

Domain wire schemas, request routing, retry policy, and daemon lifecycle stay with each engine. Vellum should adopt this mechanism to close its process-lock, stale-socket, private-endpoint, and truthful retry gaps. Chronos's immutable revision-file store and Keystone's single-register directory checkpoint have different retention and recovery laws and must not be conflated.

#### Bounded idempotency and replay register

Agora and Vellum services both retain a bounded mapping from a client/request key to the original request and reply. Extract a generic register whose outcomes are `new`, `matching_replay`, `conflict`, and `full`. Eviction must be an explicit policy, never an invisible side effect. Consumers retain request equality/digest semantics, reply types, refusal mapping, and any durable representation.

Client request namespaces should first adopt `keystone-bounds::CheckedCounter`; a new identifier mechanism is unnecessary unless multiple clients prove requirements beyond the existing algebra.

#### Saturating observations

Sticky saturating counters are duplicated across Aegis, Agora, Chronos, Hermes, Moira, Vellum, and Keystone adapters. Add qualified counter and duration-accumulator types to `keystone-observe`, with an explicit overflow bit and a single specified value-after-overflow law. Domain metric layouts and operation names remain local.

The duration contract must be reconciled before extraction: some peers clamp to `Duration::MAX`, while another preserves the previous value and marks saturation. Shared code may not silently choose between those meanings.

#### Lifecycle obligation ledger

Qualify `keystone-lifecycle::ObligationLedger` through Vellum's graceful drain and Daimon's controlled shutdown, using Chronos as operational precedent. The shared mechanism may track open/draining/closed state, bounded obligations, completion/cancellation, and unresolved disposition. Components retain deadline policy, obligation identity, and shutdown consequences.

#### Codec and conformance tooling

Vellum, Agora, and Moira already exercise `keystone-codec`. Keystone should supply reusable golden-fixture, malformed-input, round-trip/property, parser-fuzz, and compatibility-conformance helpers. This strengthens shared wire rigor without imposing a universal envelope or schema.

### Candidates that require more evidence

Do not extract these merely because Vellum needs them:

- **Atomic user-file publication.** Vellum needs stale-content witnesses, temporary-file publication, sync ordering, permission/metadata handling, and partial/unknown outcomes. Extract only after a second consumer demonstrates the same file-publication contract. Keystone's checkpoint store is not an arbitrary document store.
- **SQLite ownership and transactions.** Moira has strong database identity, sidecar-lock, schema, size-bound, full-sync, and indeterminate-commit behavior. Keep it in Moira until another SQLite consumer agrees on those exact semantics.
- **Bounded process execution.** Daimon, semantic-language adapters, and Dendrite show the need for argv-without-shell, controlled cwd/environment, bounded streams, deadlines, cancellation, process-tree termination, and reaping. Define the Rust contracts in two consumers before promotion; do not mechanically port Dendrite's Elixir implementation.
- **Immutable release artifacts.** Axon's manifest, checksums, staging, activation evidence, and rollback model should become cross-cutting only after Vellum and another Rust service share an exact artifact contract. Activation policy remains product-owned.
- **Filesystem watch and debounce.** Dendrite supplies useful hot-reload precedent, but watching is platform- and policy-sensitive. Start with a Vellum/semantic-peer port, then extract only matching mechanism.
- **Protocol envelope.** Invocation, idempotency, causation/correlation, revision, deadline, budget, capability, principal-assertion reference, and trace fields may justify a small bounded envelope after the peer DTOs are compared. Do not create a universal command bus.
- **Capability metadata.** Mechanical bounded discovery may be shared; the meaning and authorization of capabilities remain component-owned.

Forge currently appears to contain legacy Loom-era work rather than an authoritative live contract. Do not promote it into Keystone unless ownership and current semantics are re-established.

### Work that must not move into Keystone

The following remain categorical authorities even where their implementations use Keystone mechanisms:

- Aegis grants, attenuation, epochs, resources, actions, and authorization decisions.
- Hermes streams, records, cursors, replay, and inbox semantics.
- Moira workflows, compensations, reconciliation, and recovery policy.
- Chronos tasks, schedules, leases, and activation policy.
- Daimon agent runs, provider contracts, tools, and execution policy.
- Agora rooms, membership, messages, and moderation.
- Vellum documents, edits, revisions, transactions, workspaces, and save semantics.
- Rune's specification language and Dendrite's query-service behavior.

### Recommended extraction and adoption order

1. Reconcile Keystone's qualification documents with its current Aegis/Hermes directory-store adoption; some older Chronos notes still describe the directory adapter as unavailable.
2. Add and migrate saturating observation primitives, a small low-risk qualification across several consumers.
3. Extract the receipt-gated publication/fencing mechanism from Aegis and Hermes and prove it with shared model and fault tests.
4. Consolidate private local-resource ownership and phase-aware IPC using Chronos and Vellum as primary consumers and Keystone/Moira as corroborating evidence.
5. Extract the bounded replay register from Agora and Vellum services.
6. Qualify the lifecycle ledger through Vellum and Daimon.
7. Integrate Vellum with Aegis and optionally Hermes only through public host/service boundaries.
8. Defer file publication, SQLite, process execution, artifact distribution, watching, and protocol-envelope extraction until the required second contracts exist.

Each extraction requires algebraic laws, model/state-machine tests, fault injection, content-safe RED/USE observations, STRIDE/LINDDUN review, and migration evidence from at least two real consumers. Existing in-flight peer changes are evidence to reconcile, not APIs to freeze accidentally.

## Semantic-intelligence rigor

The semantic peer should obey the same operating rigor as Vellum:

- File watching or disciplined `didOpen`/`didChange`/`didClose` synchronization.
- Explicit index states: cold, warming, current, stale, failed, and unavailable.
- Bounded queues, timeouts, cancellation, and deterministic overload behavior.
- Provider conformance tests and recorded golden fixtures.
- Property tests for position/range and edit conversions.
- Model-based tests for synchronization and refactoring transactions.
- Differential tests where two providers implement the same query.
- Fault injection for provider crashes, truncated replies, stale indexes, and restart.
- STRIDE and LINDDUN analysis at every trust and data boundary.
- Content-safe telemetry for warmup time, query latency, queue saturation, stale/refused results, provider errors, CPU, and memory.
- Skills or prompting that encourage semantic queries when they offer leverage, without forbidding ordinary search and direct reading.

## Delivery phases

### Phase 0 — freeze the release contract

- Declare the first release daemon/MCP-only; TUI and GUI remain unreleased peers.
- Ratify repository and dependency boundaries.
- Set default read-only authority.
- Resolve licensing, security policy, and `publish = false` consistency.
- Reconcile older release gates with current scope.

Exit gate: one non-contradictory release contract and threat/privacy model.

### Phase 1 — make the agent service functionally complete

- Add stable workspace/document identity and enumeration.
- Support empty workspaces and adding files/directories.
- Add durable save semantics and externally changed-file detection.
- Add idempotent operation receipts and explicit uncertain outcomes.
- Qualify concurrent human and agent access at the protocol level.

Exit gate: an edit accepted through the service survives restart or reports a precise non-durable outcome.

### Phase 2 — extract MCP

- Establish the sibling repository.
- Move the MCP executable, policy, tests, and docs.
- Consume only the public client SDK.
- Pass compatibility and official-client interoperability suites.
- Remove the in-tree connector after parity.

Exit gate: `vellum-mcp` can be built, tested, installed, and versioned without an inbound dependency from Vellum.

### Phase 3 — make GitHub distribution reproducible

- Remove ambient sibling-path dependencies from release builds.
- Add CI and clean-archive installation tests.
- Pin toolchains and dependencies.
- Build checksummed, provenance-bearing macOS artifacts.
- Exercise the exact documented installation path.

Exit gate: a clean machine can install and verify the intended service and MCP peer from immutable GitHub inputs without crates.io.

### Phase 4 — operationalize the service

- Add health, readiness, metrics, tracing, and content-safe logs.
- Add graceful lifecycle, socket recovery, launchd, limits, and backpressure.
- Add manifest, smoke, activation, and rollback mechanics.
- Run failure, security, privacy, and soak tests.

Exit gate: deployment, upgrade, failure, and rollback have deterministic evidence and bounded outcomes.

### Phase 5 — establish the semantic-intelligence peer

- Specify the narrow query algebra and revision/provenance witnesses.
- Implement the service/client split and LSP adapter baseline.
- Integrate diagnostic and symbol workflows first.
- Add two-phase rename/refactoring through Vellum.
- Keep nREPL and BEAM remote-shell adapters behind separate explicit authority.

Exit gate: semantic results are freshness-qualified, bounded, observable, and can drive a stale-safe Vellum transaction.

### Phase 6 — converge human and agent workflows

- Route TUI and GUI through the client/service protocol when shared state is required.
- Preserve embeddable UI libraries for composition under a third executable.
- Demonstrate a workbench combining Vellum with peer engines without importing their application shells.
- Qualify multi-client revision, notification, and conflict behavior.

Exit gate: standalone and composed applications reuse the same lawful engine and client contracts.

## Non-goals for the initial production release

- Shipping the TUI or macOS GUI.
- Publishing to crates.io.
- Wrapping headless VS Code, Electron, or a full IDE over MCP.
- Modeling cursor or selection state for agents.
- Defining a universal IDE abstraction.
- Mixing runtime evaluation authority into static semantic queries.
- Allowing implicit writes, saves, or filesystem discovery.
- Exposing the local daemon to a remote network.
- Treating MCP as the canonical protocol or SDK.

## Continuing assurance gates

Every phase must preserve:

- TDD and BDD acceptance evidence.
- Algebraic, property, model-based, and state-machine tests.
- Deterministic replay where effects permit it.
- Explicit error and uncertainty types; no false success.
- Architecture dependency checks.
- STRIDE and LINDDUN review.
- RED, USE, and golden-signal instrumentation.
- Performance budgets and regression measurements.
- Fuzzing of parsers, protocol boundaries, and persistence recovery.
- Compatibility fixtures and migration tests.
- Reproducible builds and auditable dependency inputs.

The roadmap is complete only when these properties hold in the shipped artifact, not merely in the engine's internal model.

## Execution ledger — 2026-11-12

Delivered in the current integration tranche:

- A verified, non-activating GitHub candidate installer now validates the closed bundle before writing and installs only `bin/vellum-daemon`, `bin/vellum-mcp`, and a canonical durable receipt. Exact reruns are idempotent; partial, extra, substituted, linked, noncanonical, and interrupted installations refuse.
- Fresh launchd installation is a separate explicit transaction consuming that canonical `0400` installation receipt. It verifies the exact two installed `0500` products, byte counts, digests, approval, plist, health, journal, and reconciliation state. Upgrade evidence and first-install evidence remain distinct.
- The native service and MCP peer expose `vellum.resolve_locator`: a read-only, exact in-memory join from one caller-known absolute locator to opaque workspace identities. It performs no filesystem access, membership admission, canonicalization, or path projection and reports content-safe resolved, absent-or-foreign, refused, and unavailable counters.
- CI and release-candidate qualification now build the daemon and MCP executable together before their authenticated shipped-process test. This prevents a stale ambient daemon binary from producing a false protocol failure.
- The ordinary MCP catalog first reached seven tools plus four authenticated durable additions in this tranche. A later delivery adds exact-revision ordinary save, bringing the current catalogs to eight and twelve. Compatibility, extraction, supply-chain, installer, activation, Clippy, wire, service, daemon, MCP, and shipped-process gates pass.
- Exact committed-source qualification passed at `92a0746448f171803af8a6b6c7bae768436dbbdb`. Two clean builds compared equal; the canonical bundle, verification result, and installed receipt bind manifest digest `e99b534eaa3d95b7c4d34e2929c3d0cbd597f6872acbc5feba9851a0996aa363`. Fresh installed daemon and MCP help probes passed without activation.
- Official `@modelcontextprotocol/client` 2.4.0 interoperability is now maintained by an exact 13-component npm lock and CycloneDX source SBOM. Automatic and pinned `2026-07-28` negotiation, discovery, eight ordinary tools, fifteen authenticated tools, resources, locator non-disclosure, snapshot, atomic workspace edit, exact-revision durable save and clean EOF/disconnect pass. The official decoder exposed and drove fixes for missing resource cache/TTL evidence and non-object durable input schemas; TypeScript remains qualification-only.
- Standalone `vellum-mcp` peer emission now carries a deterministic payload manifest over the complete peer tree, and its generated Rust contract test rejects added, removed, changed, symlinked, noncanonical or source-revision-mismatched payloads. No crates.io or remote publication is implied.
- Candidate schema 5 hash-binds its exact build recipe. Clean qualification adds canonical source provenance joining the local Vellum and Keystone commit/tree objects to the manifest and byte-equivalence digests; bundling, verification and installation reject substitution. Remote reachability, signatures, attestations, tagging and publication remain separate unperformed effects.
- Exact schema-5 committed-source qualification passed at Vellum `6f77ca21e8b21840b3390690cf9b8ef21aadf82b`; its eight-file provenance-bearing bundle reverified and installed daemon/MCP with manifest digest `32944c03739dd62db6392540600f94112450fbe1056d999cdc2d9e0d9d4618f3`. Activation remained unperformed.
- The semantic MCP extraction contract now matches the runtime's ten-tool surface, including bounded incoming and outgoing call hierarchy; extraction tests fail closed on catalog drift.

The ordinary and semantic MCP peers are now independently materialized in
sibling repositories. Their locked extraction gates pass from the current
Vellum source: the ordinary peer consumes the public client SDK, credential
adapter and immutable Keystone source; the semantic peer consumes the native
and semantic SDK facades plus the credential adapter, and passes locked tests,
strict Clippy and path installation. The current semantic MCP catalog contains
the complete bounded query surface, including incoming/outgoing call hierarchy
and two-phase rename planning; stale-safe application remains owned by Vellum's
separate semantic-apply service. Remote Git reachability is not claimed.

Still ahead on the full roadmap: immutable remote GitHub release execution and
provenance plus semantic-provider confinement sufficient for distribution. The
first daemon/MCP release is locally qualified; its remaining steps require
remote publication/signing or a real activation decision. No crates.io
publication is authorized.

The maintained full gate subsequently exposed an opaque readiness diagnostic:
the shipped-process test reported only EOF when a managed execution sandbox
denied the child daemon's Unix-socket bind. Daemon startup now projects every
typed binding refusal to a closed content-safe operational class, and the
process qualifier retains terminal status plus redacted stderr on pre-readiness
exit. The exact daemon/MCP journey passes outside that socket-restricting
sandbox; affected unit suites, strict Clippy, compatibility/supply-chain gates,
semantic-provider qualification and warnings-denied documentation also pass.
The tracked correction is commit `09c69ef`.

Read-only remote qualification now establishes the precise publication
blocker: neither Vellum `67f96e3b8e1a34532371c8a119b83f6333fb7d52` nor pinned
Keystone `3486252cdbaeb2d784756076f43ef6157ed0b7f8` is fetchable from its
declared GitHub repository. Both repositories advertise no heads or tags, and
exact shallow fetches return `not our ref`. The maintained qualifier was
improved to probe both owners before refusing and now emits the content-safe
aggregate `[vellum, keystone]`; no remote configuration, push, tag, release or
publication occurred. The aggregate qualifier correction is commit `7ec958b`.
The release-candidate workflow now also invokes a bounded
`--verify-remote-only` mode before artifact construction. That mode validates
the local commit/API closure and both exact shallow fetches but deliberately
does not repeat the complete build/install gate already owned by the workflow.
Success and dual-unavailable fixtures, release policy, compatibility mutants
and supply-chain gates pass; the actual current remote-only run still refuses
the complete `[vellum, keystone]` blocker set.
The workflow enforcement is commit `4de24b3`; shared multi-root planning is
commit `a76800c`, and the current qualified Vellum source is `f4a53ba`.

The historical Ratatui application was recovered from Vellum revision
`920dbfccf3806378fd0274cc51fadffac405931b` into a clean independent local
`vellum-tui` repository, now at commit `bf4e4fe`. The package has a reusable
presentation library and thin standalone composition root, starts in a lawful
empty state or from bounded file/directory inputs, uses the descriptor-anchored
local discovery adapter, refuses links/special resources, and expands at most
sixteen unique files without partial discovery mutation. This recovered
embedded editor does not retain an empty directory as shared workspace
membership or watch it for later files; that stronger contract remains in the
newer presentation-free `Workspace` and Phase 6 service-backed UI convergence.
Twelve behavior/source
contract tests, locked offline build, strict Clippy and warnings-denied rustdoc
pass from the real sibling checkout. The historical optional desktop binary was
removed; `vellum-gui-macos` remains a separate peer. The TUI is still outside
the initial release and unpublished. Its current coordinated-checkout path
dependencies must become exact Git dependencies only after the pinned Vellum
source is remotely reachable; no remote, tag, push, crate publication or
release was created.
Its read-only GitHub CI contract pins the checkout action, Vellum
`f4a53ba664acf2a12b6fadfd7d378eef63307c39` and Keystone
`3486252cdbaeb2d784756076f43ef6157ed0b7f8` exactly, then runs formatting,
tests, strict Clippy and warnings-denied rustdoc. The source-contract test
rejects moving checkout actions, source revisions or publication effects; the
workflow will fail closed until those exact upstream objects are reachable.

The historical Iced application and its exact qualified dependency patches
were likewise recovered into an independent local `vellum-gui-macos`
repository, now at commit `6ee9d6f`. Its reusable library retains the prior
attachment, IME/preedit, clipboard, comment, rendered-geometry and
multi-selection protections; the standalone binary owns the Iced/macOS
composition. Empty launch was preserved and both startup and in-app Open now
accept bounded file/directory inputs through the same local discovery adapter.
As with the TUI, directory expansion in embedded mode is not durable/shared
root membership and does not imply filesystem watching.
The exact reviewed Iced geometry and AppKit private-pasteboard patches, upstream
archive hashes and license texts are retained under `vendor/`; stock Iced does
not satisfy this renderer contract. Sixty-three all-target behavior/source tests,
locked offline build, strict Clippy and warnings-denied rustdoc pass from the
real sibling checkout. The GUI remains unreleased and unlaunched; signing,
notarization, app-bundle distribution and live-window qualification were not
performed. As with the TUI, exact Git dependency conversion waits on Vellum
remote source reachability, and no remote, tag, push or publication occurred.
The GUI peer carries the same exact, read-only coordinated-source CI shape and
source-contract rejection of upload, release, push or crates.io publication.
It now also exposes `ServiceGui<T: ServicePort>` as a reusable parallel Iced
controller and accepts `--service SOCKET [--credential CREDENTIAL]` in the
standalone composition root. Framework edits are reduced to one witnessed
replacement, exact snapshot round-trip is required before editing, change-feed
polling refreshes stale projections, dirty exit requires confirmation, and
unknown replace/save outcomes retain only the exact replay operation while
locking other effects and exit. All 66 all-target tests, strict Clippy and
warnings-denied rustdoc pass in both isolated and authoritative checkouts. The
embedded `Bridge` and its qualified renderer/IME/comment behavior remain
unchanged.

The GUI peer subsequently converged on the same accessible `plushie-iced`
0.8.4 lineage as Chronos at `e1535a2`, without dropping AccessKit. Vellum's
bounded cached-layout selection geometry was ported into exact vendored
Plushie packages; archive hashes, upstream revision
`e423a8efb3ec953071be5f2fc995cdb99c21b320`, the WebAssembly-only dependency
resolution correction and nonclaims are recorded and source-tested. The
obsolete stock-Iced vendor graph was removed. Commit `b5004d4` separates
component lifecycle from standalone process exit for both embedded and service
presentations, and `6e3c354` exports the GUI message facade required by an
external composition root. The authoritative peer passes 69 ordinary tests,
strict Clippy, warnings-denied rustdoc, headless desktop recovery and its real
daemon open/edit/save/disconnect journey. No live window, signing,
notarization, remote, release or crates.io publication was performed.

Multi-root admission is now one bounded local-filesystem adapter mechanism
rather than duplicated presentation logic. Vellum commit `a76800c` adds a
lawful empty identity, deterministic duplicate-free union, aggregate root,
entry, file and retained-path-byte limits, descriptor-anchored discovery,
redacted closed refusals and revalidation-required locator evidence. Its 67
focused tests, strict Clippy, warnings-denied rustdoc, compatibility and
repository policy gates pass. TUI `b79330f` and GUI `af68328` consume that
adapter facade, remove their direct planning dependency on `vellum-host`, and
pass their complete authoritative suites. This does not claim durable empty
directory membership, filesystem watching or shared service-backed workspace
state; those remain Phase 6 work.

Phase 6 now has a presentation-neutral service client substrate. The new
`vellum-workspace-client` owns only client-local view-open, inventory,
selection, redacted snapshot, persistence and change-cursor state; it projects
explicit current/stale/absent freshness and clean/dirty/indeterminate
persistence while preserving the native client's exact
`NotSent | OutcomeUnknown` fence. Witnessed replace and save operations retain
the exact replay witness across uncertainty, external changes stale only the
affected projection, and a projected self-revision does not spuriously stale
the caller's own edit. Empty inventory remains a lawful state and no source,
filesystem, cursor, retry or UI-loop authority enters the crate.
`vellum-sdk-client` re-exports it for agent, TUI, GUI and composite workbench
consumers. Eight focused behavior tests, strict Clippy, warnings-denied rustdoc,
35-crate boundary/release/compatibility gates and 125/84-component supply-chain
closure pass. The substrate delivery is commit `5e4d9f5`; editable projection
is commit `f4a53ba`.

The TUI now adopts that substrate as an optional local-service backend without
changing its embedded mode. `ServiceEditor<T: ServicePort>` is a reusable
presentation controller with only a UI-local byte cursor; inventory,
selection, snapshots, persistence, change-feed freshness and exact replay
remain service/client concerns. The standalone binary accepts `--service
SOCKET` and optional descriptor-anchored `--credential CREDENTIAL`, supports
empty startup plus explicit file/directory admission, performs witnessed
editing and save, refuses unsafe exit from an indeterminate operation, and
returns the transport through the client's exact disconnect semantics. Its
manifest reaches protocol/client/transport only through `vellum-sdk-client`.
All fifteen ordinary/ignored behavior and source-contract test definitions,
locked build, strict Clippy and warnings-denied rustdoc pass in both isolated
and authoritative checkouts. A separate CI-owned live journey builds the exact
Vellum daemon, waits on canonical control-socket readiness, drives the reusable
controller through open/edit/save/disconnect, verifies exact persisted bytes,
and requests orderly shutdown; it passes outside the managed environment that
forbids Unix-socket binding. The real-daemon peer delivery is `vellum-tui`
commit `708bb92`; pane-scoped rendering, its public frame facade and the closed
service-input translation facade advance the current peer to `bf4e4fe`.
The GUI adopts the same SDK boundary at `vellum-gui-macos` commit `6ee9d6f`;
its 67 ordinary tests and separate headless real-daemon journey pass with the
same persistence and shutdown evidence without opening a window.

The broader workspace contains a mature Agora engine/service/client SDK and a
separately owned effect-free `agora-vellum-adapter`, but no reusable Agora
Ratatui or Iced presentation peer. A truthful Vellum+Agora workbench therefore
cannot yet be qualified: defining Agora presentation state or authority inside
the workbench would invert ownership and make the composition root a second
engine/UI source of truth. The workbench item is gated on Agora exposing the
same kind of reusable presentation library; once present, a third application
may own only the shared event loop, layout and routing while each peer retains
its SDK/service authority. A Vellum-only wrapper is not accepted as composition
evidence.

A separate unpublished `workbench-tui` repository now closes the local
dual-service presentation composition at commit `36ae34d`. One Ratatui event loop and
frame render Chronos and Vellum through their public pane-scoped presentation
facades. The host owns only geometry, focus transfer and the global exit key;
all other input is routed only to the focused peer. Its source contract pins
Vellum `f4a53ba`, Vellum TUI `bf4e4fe` and the current Chronos workspace
revision `03c4684`, rejects direct Vellum/Chronos engine or service
dependencies, and asserts unpublished/no-remote status. All-target tests,
locked offline build, strict Clippy and warnings-denied rustdoc pass from the
authoritative sibling checkout. The binary attaches an existing Chronos local
service and Vellum Unix service through public client/transport boundaries,
keeps Chronos writes opt-in, supports Vellum's optional local credential, and
preserves dirty confirmation and indeterminate-effect refusal on whole-process
exit. A bounded headless snapshot gate builds both exact daemons, waits on both
canonical readiness contracts, establishes both live sessions, renders a
current combined projection, disconnects Vellum exactly and drains both
services. This is a production presentation composition, not an orchestration
claim: cross-pane workflow, compensation and recovery policy belong to Moira or
a separate orchestration harness. Agora still lacks a reusable presentation
peer.

A separate unpublished `workbench-gui-macos` repository now closes the
framework and lifecycle composition proof at commit `c2c9a23`. One accessible
Plushie Iced runtime imports the independent Chronos and Vellum presentation
libraries; the host owns the window, active-tab routing and global close
coordination while each peer retains engine and effect authority. Only the
active pane receives user intent, Chronos backend delivery remains routed, and
Vellum's dirty/uncertain close laws complete before the host may terminate the
shared application. Because Cargo honors patches only at the workspace root,
the workbench explicitly selects Vellum's qualified renderer and AppKit
packages. Five headless behavior/source tests, locked offline build, strict
Clippy, formatting and warnings-denied rustdoc pass without opening a window.
Chronos GUI is now a clean independent local peer at commit `127d082`; its 11
tests, strict Clippy, formatting and warnings-denied rustdoc pass. Workbench
commit `5f9b396` binds that exact peer revision. The sibling Chronos engine
checkout still contains uncommitted restructuring, so the workbench separately
records `chronosEngineSourceQualified=false` rather than fabricating a
coordinated dependency revision. No remote, release, signing, notarization or
crates.io action occurred.

That integration exposed and fixed a real release-tool defect: SBOM
`--emit=-` used an asynchronous stdout write immediately followed by
`process.exit`, truncating large CycloneDX output nondeterministically. Stdout
emission is now synchronous and regression-compared byte-for-byte with the
canonical serializer before JSON parsing.
The correction is commit `67f96e3`.

Semantic-provider production promotion remains correctly closed on macOS.
ProcessKit 3.3.4 explicitly supplies process supervision rather than a sandbox;
its macOS backend cannot enforce the required filesystem, network, subprocess
and whole-tree resource axes, and path launch is not descriptor-anchored.
Completing that item requires a real sandbox/helper deployment architecture or
a separately supported confinement platform. No weaker evidence-only
promotion was added.

The concrete macOS closure is now narrowed to a separate signed App Sandbox
XPC helper, not an expansion of the engine or ordinary daemon. Apple's current
contract gives each XPC service its own restricted sandbox, ties the service
process to its client lifetime, and prefers XPC over an inherited child for
privilege separation. Network access remains absent without an entitlement;
workspace access must be passed as an explicit user-selected capability or
bookmark, and a packaged provider child inherits only the helper's static
sandbox. The workspace already contains generated `objc2-foundation`
`NSXPCConnection` bindings, so a Rust implementation can isolate the remaining
Objective-C protocol declaration in a reviewed macOS adapter seam. This helper
would be an app-bundled/signing-dependent distribution peer; it must not be
represented as a plain `cargo install` binary. Linux namespace/cgroup
confinement remains a valid alternative first production platform, but no live
Linux runtime is currently available on this host for enforcement
qualification.

The first helper delivery boundary is now implemented in Vellum as the
unpublished `vellum-semantic-helper-protocol` crate. It admits only canonical
relative snapshot paths, sorts and deduplicates bounded immutable manifests,
commits path/length/content into a redacted SHA-256 root, binds each start to
an opaque qualified-provider digest, and exposes a fixed isolation product
whose no-network, read-only-snapshot, private-temporary-directory and
client-lifetime requirements cannot be disabled by a caller. Poll/status binds
the exact session and snapshot commitment. The crate owns no XPC, traversal,
provider-launch or engine authority. Seven behavior tests, strict Clippy,
warnings-denied rustdoc, formatting, project-boundary and non-publishing
release-contract gates pass. This closes the shared algebra, not the runtime
claim: the separately signed App Sandbox XPC helper and its enforcement
qualification remain outstanding.

The next filesystem boundary is implemented in the independent macOS peer's
`vellum-adapter-semantic-snapshot-fs-macos` crate. A caller
supplies an already-open private directory descriptor plus bytes in canonical
manifest order. The adapter verifies every length and digest before namespace
creation, creates every component descriptor-relatively with `NOFOLLOW`,
publishes the complete tree by no-replace rename, seals files/directories
`0400`/`0500`, and returns an affine directory descriptor instead of a path.
Its explicit destroy path and drop fallback use a bounded creation ledger;
content-free saturating counters report attempts, publication, refusal and
logical bytes. All 70 local-filesystem tests, including three snapshot
adversary/lifecycle cases, pass with strict Clippy. This closes snapshot
materialization, not XPC enforcement or signing.

The helper's transport product is implemented as the unpublished
`vellum-semantic-helper-wire` crate. It uses the qualified Keystone bounded
codec substrate and a fixed canonical envelope for start/poll/stop and
status/refusal. Start binds the exact session, snapshot manifest, provider
qualification and quantitative resource product; the fixed isolation booleans
are reconstructed from the closed protocol type rather than caller-controlled
wire flags. Manifest decoding rejects noncanonical order, duplicates, invalid
paths/digests, trailing data and all limit violations, while encoding reserves
the exact checked size before growth. Source bytes and directory descriptors
are never serialized. Three adversarial/canonical tests, strict Clippy,
warnings-denied rustdoc, formatting, taxonomy and release-contract gates pass.

The macOS client boundary is implemented in the independent unpublished
`vellum-semantic-helper-macos` peer as its XPC adapter crate. A small ARC Objective-C bridge
owns NSXPC and exports one reviewed C seam; safe Rust pins the exact helper
service identifier and derives the designated code requirement only from a
validated ten-character Apple Team ID. Each exchange carries the canonical
frame and, for start only, the already-open snapshot directory capability.
The bridge rejects frames above 24 MiB, uses the asynchronous reply contract
with a fixed 30-second deadline, invalidates on timeout, and never closes the
caller's descriptor. Rust never retries, classifies transport failure as
outcome-unknown, permanently fences that client, and validates reply exchange,
session and snapshot commitments before state advances. Content-free
saturating usage separates local/helper refusals, replies, invalid frames,
uncertainty and traffic volume. The Objective-C bridge compiles under ARC and
blocks; two tests pass, including runtime protocol registration and exact
binding checks; strict Clippy/rustdoc/taxonomy/release gates close over 38
non-publishable Vellum crates. The platform-specific snapshot and XPC adapters
were extracted from Vellum core into peer commits `e8f5429` and `2deed2c`;
the latter binds and verifies exact Vellum revision
`9a7de22b71051f84552ebe4450329b6352664418`. All five peer tests, strict Clippy,
formatting, warnings-denied rustdoc and its source-contract gate pass independently. The
ordinary daemon and MCP dependency trees contain no helper, snapshot or XPC
package. The signed helper implementation and live bundle journey remain
outstanding.

An attempted host-OS CI matrix was outside the current goal and was removed in
Vellum commit `99a7eb6`; no macOS/Linux/Windows qualification claim remains.
The audit did independently expose stale evidence from the already-committed
semantic helper extraction: the compatibility baseline omitted both helper
protocol crates and the source SBOM still described 125 rather than 127
components. Those target-neutral evidence corrections were preserved.
Compatibility and supply-chain mutant suites, candidate
builder/lifecycle/reproducibility/bundle/install fixtures, source qualification
and the release contract pass.

The independently releasable MCP peers were then reconciled against the
current deterministic extraction products rather than left at their earlier
Vellum pins. The ordinary peer advances to local commit `8f4a727`: its exact
`bc95dd8` payload adds the current workspace editing/locator surface, 40 passing
tests, a complete payload hash manifest and a source-tree integrity test. The
semantic peer first advanced to `605215c`, then Vellum commit `8019f11` closed
the same deterministic repository-tree integrity law already applied to the
ordinary peer. The semantic peer's final local commit `04a2f3b` binds that
exact Vellum revision, includes the current query/lifecycle behavior, a complete
payload hash manifest and 32 passing tests. Both peers
bind Keystone `3486252`, pass formatting, locked offline tests, strict Clippy,
warnings-denied rustdoc, exact-revision Git installation and binary help smokes.
No peer remote, upload, release or crates.io publication was performed.

The semantic peer source boundary was subsequently tightened without changing
runtime behavior or adding platform scope. Vellum commit `82b171b` makes the
generated security policy bind `Cargo.toml`, `Cargo.lock`, the exact-source
receipt and deterministic payload receipt as one supply-chain contract. It
also states the payload's privacy boundary (relative names, lengths and hashes;
no source content, credentials or host paths) and the read-only/non-publishing
CI boundary. Semantic peer commit `a6e1291` binds that exact Vellum revision.
All 32 peer tests, strict Clippy, warnings-denied rustdoc and exact local Git
installation qualification pass. No remote, publication or OS qualification
was performed.

The ordinary agent surface now closes the practical persistence continuation
after atomic multi-document editing. Vellum commit `356ce2a` adds
`vellum.save`: it selects one opaque document identity, verifies the exact
revision returned by snapshot/workspace edit, submits the existing witnessed
native save, and projects the complete durable/not-durable/unknown receipt.
Each document is saved independently; no atomic multi-document durability is
claimed. Outcome-unknown replay is admitted only with the returned same-process
operation and identical arguments, and no token is minted for an unrelated
pre-existing uncertainty fence. Content-free saturating counters partition
requests, exact replays, durable, not-durable, unknown and refused outcomes.
The official client proves `workspace_edit -> save -> exact persisted bytes` in
both ordinary and authenticated compositions. Protocol/service/MCP tests,
strict Clippy, warnings-denied rustdoc, architecture/effect boundaries,
compatibility and supply-chain mutants, deterministic extraction, exact local
Git installation and the shipped executable catalog gate pass. Standalone
`vellum-mcp` commit `c275d47` binds the exact Vellum revision and passes 44
direct tests plus payload/source integrity. No platform qualification, remote,
release or crates.io publication was performed.

The first Save As delivery prerequisite is now implemented solely in the
platform-neutral host contract. `SaveAsAuthorityRequestV1` forms one affine
product over the exact Persist and Rebind requests, engine document version,
and overwrite-existing destination request. A complete grant retains all three
request-bound decision witnesses plus a congruent one-use destination object;
wrong actions, contexts, resources, source versions, destinations, effects,
foreign objects, partial products, and reused or attenuated authority fail
closed before source capture. The authorized plan accepts only the matching
engine-owned immutable serialization and hands adapters bytes plus locator-free
evidence through a total result. Four new BDD tests bring the host suite to
33 passing tests; strict Clippy, warnings-denied rustdoc, compatibility,
effect/project-boundary, and non-publishing release gates pass. This does not
add a filesystem interpreter, Ubuntu/Linux/macOS/Windows qualification, wire
command, MCP tool, or UI surface. The retained session/workspace transaction,
orthogonal physical/binding receipt, and durable journal ordering remain next.

The locator-free native Save As receipt algebra is now closed in `vellum-host`.
Its private state sum admits exactly ten valid rows from the 24 raw physical ×
binding × terminal/fenced products: durable publication may be activated,
not activated, or binding-unknown; non-durable, physical-unknown, and
not-attempted publication can only be not activated; every unknown is fenced.
Only durable + activated reports permission to change the stable workspace
binding. Closed reason sums preserve stage truth, and a non-empty bounded
fence semilattice records physical uncertainty, binding uncertainty, retained
tasks, release uncertainty, and faults without locator-derived dimensions.
Three new behavior tests bring the host suite to 36 passing tests, including
exhaustive product closure and redaction. The retained session coordinator
remains the next step; no wire, adapter, UI, MCP, or platform target was added.

The portable retained Save As coordinator is now implemented in
`vellum-session`. It is generic over a command-scoped port with opaque
preparation, attempt, and activation tokens; the coordinator itself knows no
path, resource adapter, filesystem, policy, protocol, or UI type. Every
fallible transition retains the exact token needed to reconcile: admission
errors fence the preparation, write errors fence the attempt, activation
errors retain the recovery obligation, and release/retirement errors retain
both attempt and activation. Reconciliation may only observe, recover an
already-published activation, release, retire, or discharge a preparation—it
cannot repeat admission, write, or activation. `LocalHost` also gained a
non-effecting activation recovery method that reconstructs the destination
handle only when the original engine task reports binding publication.
Ten target-neutral fake-port BDDs cover happy path, every fence class,
exact-once effects, known not-durable/not-activated closure, activation
recovery, and illegal-transition zero-call behavior. Focused strict Clippy and
warnings-denied rustdoc pass. The workspace-owned retained transaction and
stable private binding swap remain next; no OS-specific interpreter or
qualification was added.

The coordinator seam was then corrected before workspace integration: retained
token ownership is now a separate `SourceSaveAsTokenSetV1`, while each
lifecycle call may use an ephemeral `SourceSaveAsSessionPortV1` borrowing the
real workspace host. This prevents the coordinator from being fake-only or
requiring a long-lived host borrow across requests. The same ten portable BDDs
and strict Clippy pass after the correction.

The session bridge now performs the first real workspace-facing authority
handoff. `authorize_source_save_as_preparation` obtains the complete atomic
authority product before native preparation or byte capture, retains its
one-use carrier with the exact engine preparation, and discharges a prepared
artifact if capture fails. The monolithic lifecycle port was split into
prepare, admission, service/observation, activation/recovery, and
release/retirement ports so each workspace step can borrow only its actual
dependencies. Admission can mutate the preparation to transfer affine
authorization only after native queue success, and service can mutate the
attempt to consume the destination object immediately before write. The ten
portable lifecycle BDDs, focused strict Clippy, and warnings-denied rustdoc
remain green. Workspace retention and the stable binding commit are still
open; no protocol or platform surface was added.

The portable workspace Save As transaction is now delivered. Workspace owns
the retained transaction under its stable `WorkspaceDocumentId`, blocks
conflicting edits, ordinary saves, and membership removal before activation,
and changes only the private locator/native handle after the engine proves a
durable write and activated binding. Active views, document identity, and
inventory generation do not change; later ordinary saves use the new binding.
The first target-neutral in-memory adapter test exercises the complete
authority -> preparation -> admission -> write -> activation -> binding swap
-> later edit/save -> release/retirement path and exposed a real bridge defect:
authority capture had consumed the native preparation before admission. The
engine now offers an explicitly bounded retained capture which preserves that
exact preparation for the one native admission, with a focused ownership-law
test. Engine/host/session/editor library tests, strict Clippy, and
warnings-denied rustdoc pass. No Ubuntu/Linux/macOS/Windows qualification,
adapter implementation, protocol, MCP, UI, remote, release, or crates.io work
was added. Actual receipt construction and instrumentation, service
composition/projection, then durable binding checkpoint/journal recovery remain
before protocol exposure.

Workspace receipt construction and bounded instrumentation are now delivered.
The exact locator-free authority evidence survives native task retirement and
projects the retained operation's actual physical, binding, and lifecycle
state into the closed host receipt. Every in-flight receipt is fenced by the
retained task; physical, binding, and release uncertainty add their own lattice
elements; only explicit release plus retirement becomes terminal-replayable.
A second portable in-memory behavior changes the destination after admission:
service returns uncertainty, reconciliation observes without a second write,
and the original workspace binding remains authoritative. Content-free current
gauges cover capacity, lifetime issuance, retained/live/fenced operations,
physical and binding classes, and completion. Focused host receipt tests, both
workspace behaviors, strict Clippy, and warnings-denied rustdoc pass. Service
composition still needs RED counters and per-stage latency, followed by the
durable binding checkpoint/journal; no protocol or platform surface was added.

The in-process service composition now owns the next Save As layer without
opening a wire command. It resolves the stable protocol document projection to
the workspace transaction, exposes every exact lifecycle and reconciliation
step, and advances the existing workspace-identity metadata generation exactly
once when the private binding commits. A portable service behavior proves the
checkpoint revision changes and its metadata contains the new locator before
the receipt is terminal. It also proves that a baseline-retained native task
stays fenced when retirement is refused; workspace and service now expose
explicit release/retirement reconciliation rather than leaving those fences as
dead ends. Service usage includes content-free begin/refusal/binding-commit
counters and retained/live/fenced capacity gauges. The generic durable
operation intent/completion journal still must be joined around this lifecycle
before protocol projection. No OS-specific adapter, protocol, MCP, UI, remote,
release, or crates.io work was added.

The portable durable Save As ordering is now expressed as one affine host
transaction over the existing operation journal and workspace-identity metadata
ports. Construction synchronizes a frozen intent before any caller can execute
the external write. The checkpoint transition accepts only the exact successor
generation with unchanged allocation prefixes, membership, root bindings, and
non-target document bindings; precisely the selected document locator must
change. Journal completion requires both that checkpoint and a terminal
replayable receipt proving a stable-binding commit. Intermediate crash cuts stay
pending, and the transaction deliberately exposes no effect or retry method.
The target-neutral workspace behavior proves intent -> binding checkpoint ->
completion ordering, rejects an unchanged binding without writing metadata, and
refuses completion while the receipt is fenced. Focused tests, warnings-denied
Clippy, and warnings-denied rustdoc pass. Daemon composition, recovery projection,
and any eventual protocol surface remain open; no platform-specific work was added.

Save As now has portable service-level RED and golden-signal instrumentation.
One closed stage sum covers Begin, Admission, Persistence, Activation, Release,
and Retirement; reconciliation calls join the stage whose retained obligation
they close. Every invocation contributes saturating requests, completions or
errors, and a monotonic latency sample/total. The evidence is content-free and
non-authoritative: it cannot affect admission, lifecycle transitions, authority,
or receipts. The portable service behavior covers successful stages and repeated
retirement failure without relying on a nonzero clock tick.

The pre-admission failure seam required by durable composition is now explicit.
The retained session coordinator separates preparation discharge from admission
authority and can close a prepared operation without queueing or writing. Only a
proven discard reaches Complete; already-consumed, unknown, and failed discharge
become the existing absorbing admission fence. Workspace removes a proven-closed
transaction immediately to reclaim bounded capacity, while service exposes the
same operation under the Admission RED stage. Twelve portable session behaviors
and three portable workspace behaviors cover exact-once discharge, zero admission
and write calls, unchanged resources, and non-retry after uncertainty.

The in-process service now composes the complete durable Save As owner without
opening protocol or adapter authority. It materializes the exact starting
identity, acquires authority/capture, synchronizes journal intent before native
admission, and returns one opaque owner for every lifecycle call. Intent refusal
immediately invokes the preparation-discharge law: proven closure reclaims the
slot, while uncertainty returns the retained opaque operation. Successful
activation automatically checkpoints the exact successor identity, and journal
completion reads only the workspace's retained terminal receipt. Terminal
non-binding outcomes may complete directly from IntentDurable; binding-changing
outcomes require BindingCheckpointed. The portable service behavior proves
refused-intent zero-write cleanup, Intent -> BindingCheckpoint ordering, and
pre-terminal completion refusal. Daemon recovery projection remains open.

Save As restart classification is now an adapter-neutral host contract rather
than a daemon inference. A recovered record must match the authenticated owner,
canonical request and reply bytes, exact source revision, issued document,
terminal receipt, and the current document locator's equality with the expected
destination. A binding-changing receipt additionally requires a later identity
generation. Any synchronized intent without a reply is absorbing OutcomeUnknown;
the validator has no admission, write, activation, or retry operation. Canonical,
revision, destination, receipt, and owner conflicts are closed refusals, and all
debug surfaces redact locator and payload. Daemon decoding and installation remain
open; no protocol tag was introduced.

Durable-operation catalog routing is now portable and independent of locator
encoding. A fixed V1 envelope adds one closed `Effect | SaveAs` discriminator
around non-empty bounded opaque canonical bytes. It is exact-length canonical,
allocation-fallible, payload-redacted, and rejects truncation, trailing data,
unknown sorts, empty payloads, and limit excess. Save As construction and
recovery require the SaveAs sort, and the service test proves the journal receives
that envelope with the caller's payload unchanged. The journal storage adapter
remains byte-opaque and unchanged; native path encoding stays outside core.

Save As binding publication now reaches every portable observer without
projecting a locator. The workspace change algebra adds the closed
`DocumentBindingChanged` case under the existing stable document identity;
service activation preflights both the exact document projection and change
revision before the native binding effect, then publishes exactly once on the
false-to-true binding commit. Reconciliation cannot duplicate the event. Wire
tag `6` is additive and canonical, the reusable workspace client preserves its
revision-current snapshot while invalidating only its persistence projection,
and MCP emits `document_binding_changed` with no locator or source revision.
Wire (26), workspace-client (9), service-library (6), portable Save As (1), and
MCP-library (13) tests pass, as do strict Clippy, warnings-denied rustdoc,
project/effect boundaries, seven compatibility mutants, and the nonpublishing
release contract. No application, adapter, platform qualification, remote,
release, or crates.io work was added.

The retained Save As result now has a canonical portable receipt without
opening an execution command. `DurableSaveAsReceiptV1` preserves the host's
exact ten admissible physical × binding × lifecycle products, including typed
failure reasons and a non-empty known-bit fence set. Its fixed 31-byte wire
codec is locator-free, rejects unknown tags, impossible products, empty or
unknown fences, truncation and trailing data, and has an exact byte witness.
The durable service owner retains the stable protocol document identity and
projects the host receipt exhaustively back through the protocol constructor;
invalid projection fails closed. Protocol (2), wire (27), service library (6),
portable Save As (1), and the full service behavior suite (40) pass, as do
formatting, strict all-target Clippy, warnings-denied rustdoc, project/effect
boundaries, compatibility plus seven mutants, the nonpublishing release
contract, and supply-chain adversarial fixtures. The full service run also
corrected a stale exact-usage expectation to include the already-public
reference Save As capacity gauge. No execution request, adapter, application,
platform qualification, remote, release, or crates.io work was added.

The durable Save As journal can no longer be given caller-selected opaque
request bytes beside different semantic arguments. A private-field,
destination-redacted `DurableSaveAsIntentV1` now binds the exact stable
document identity, expected revision, non-empty destination and explicit
overwrite-existing/create-exclusive effect. Its bounded canonical codec has an
exact byte witness and rejects unknown effects, empty destinations, truncation
and trailing data. The durable service encodes that typed intent itself and
requires exact congruence with the host destination request before authority,
capture or journal effects; the journal envelope therefore decodes back to the
same intent. A mismatched destination is a pre-effect refusal. Protocol (2),
wire (28), service library (6), and the portable Save As behavior pass, along
with formatting, strict all-target Clippy, warnings-denied rustdoc,
project/effect boundaries, compatibility plus seven mutants, nonpublishing
release policy, and refreshed 127-component source-SBOM adversarial closure.
No external dependency, ordinary command, adapter, application, platform
qualification, remote, release, or crates.io work was added.

Durable completion now owns the symmetric reply boundary as well: it projects
the retained host receipt and canonically encodes that exact portable receipt
inside the service. Callers can no longer pair one terminal receipt with
unrelated journal reply bytes. Preterminal receipts still refuse without a
completion event. Portable service behavior, strict Clippy and warnings-denied
rustdoc pass; restart record decoding can now treat both request and reply
bytes as service-owned canonical values.

A presentation- and adapter-neutral restart decoder now unwraps only the Save
As durable-operation sort, decodes the canonical typed intent and optional
portable receipt, and enforces exact document, source-revision and terminal
lifecycle congruence. A synchronized record without a reply projects to the
absorbing `OutcomeUnknown`; a congruent terminal reply projects to `Complete`;
a fenced reply refuses. The decoder explicitly does not validate authenticated
owner, workspace generation or the durable binding checkpoint: those remain
the existing host recovery algebra's next join rather than being inferred from
bytes. Portable behavior, strict Clippy and warnings-denied rustdoc pass.

The decoded record now joins the existing host recovery authority without
installing a daemon or adapter. A pending record becomes `OutcomeUnknown` only
under the exact authenticated subject, workspace and current generation. A
binding-changing terminal receipt refuses against the pre-checkpoint identity
and becomes `Complete` only after the current durable identity binds the exact
destination and advances beyond the record generation. The join has no
admission, write, binding, retry or cleanup method. Portable behavior, host
(37) and service-library (6) tests, strict all-target Clippy, and
warnings-denied rustdoc pass.

The durable Save As lifecycle now has its first complete portable agent
surface. Protocol, native client, canonical wire and transport expose distinct
submit, read-only lookup and exact receipt-retirement operations; they do not
coerce Save As into the synchronous durable-effect algebra. The closed observed
state is `Pending | Fenced(receipt) | Complete(receipt) | Retired`, and wire
validation rejects a receipt whose lifecycle does not match its state.
Transport uncertainty retains the exact `Effect | SaveAs` client-side coproduct
and fences changed operation/intent reuse until positive exact observation or
conflict. The durable MCP projection adds three peer tools with a closed schema,
content-free state counters and locator-free results; it exposes no admission,
write, activation or reconciliation control. Service ports default unavailable,
so a daemon that does not own the full authority/journal/metadata/adapter product
cannot fall through to ordinary execution. Client (13), wire (29), transport
(12) and MCP library (14) tests pass, as do strict all-target Clippy,
warnings-denied rustdoc, project/effect boundaries, compatibility plus seven
mutants, nonpublishing release policy and supply-chain closure. The daemon owner
and process-restart product qualification remain next; no platform-specific
adapter, UI, remote service, release or crates.io work was added.

The installed daemon owner now classifies recovered durable Save As records
without acquiring Save As execution authority. Startup preserves legacy raw
durable-effect records while routing only the tagged Save As envelope through
the canonical service decoder and authenticated workspace-identity validator;
mixed operation families coexist in the same bounded journal. Agent lookup is
a projection over that once-validated state: synchronized intent without a
reply remains absorbing `OutcomeUnknown`, a terminal receipt becomes
`Observed(Complete)`, changed input conflicts, and an exact tombstone becomes
`Observed(Retired)`. Exact terminal receipt retirement reuses the existing
`RetireReceipt` authority and journal tombstone transaction, never the live
workspace Save As lifecycle. Focused restart tests prove mixed legacy/Save As
recovery, pending uncertainty, changed-input conflict, terminal replay,
single retirement and tombstone replay across another restart. The complete
daemon gate passes 13 library, 50 primary integration and 16 durable-restart
tests (one opt-in soak remains intentionally ignored), together with strict
all-target Clippy and the compatibility baseline. This is recovery projection
inside the existing Unix application host, not a new platform mechanism or a
cross-platform qualification claim; Save As execution remains unavailable
until a separately owned portable authority/adapter composition is installed.

The recovery projection now reaches the shipped agent product rather than
stopping at daemon integration. A real daemon binary and real MCP binary
project a recovered terminal Save As receipt, changed-intent conflict,
authorized exact retirement, the absorbing retired tombstone and a separately
recovered pending `OutcomeUnknown`; the tombstone remains observable after
both processes restart and no response contains the private destination. This
delivery audit also found and corrected a stale product contract: runtime had
seven authenticated additions while the extraction manifest, shipped catalog
test and official-client qualifier still declared four. The canonical surface
is now eight ordinary tools plus seven authenticated additions (fifteen total
under the credential profile). The complete MCP suite passes 14 library, 30
protocol and 5 live tests with one separately owned shipped-catalog test; that
catalog test, strict Clippy, warnings-denied rustdoc, extraction, release
contract, compatibility plus seven mutants, and official
`@modelcontextprotocol/client` 2.4.0 automatic/pinned interoperability all
pass. The official qualification reports eight ordinary and fifteen
authenticated tools. Submit remains honestly unavailable in the shipped daemon
because no platform-neutral execution authority/adapter composition has been
installed; lookup and receipt retirement do not fall through to execution.

The current authorized local/GitHub-first scope is closed at Vellum
`d7de5f2587b3299983792c95f83fcff39ca54d0c`, pinned Keystone
`3486252cdbaeb2d784756076f43ef6157ed0b7f8`, and standalone `vellum-mcp`
`f193c69`. Exact committed-source qualification built the locked release
workspace and installed precisely `vellum-daemon` and `vellum-mcp` through the
single distribution package using immutable Git dependency identities; both
content-safe smoke probes passed. Artifact lifecycle, candidate bundle and
installation, interruption, clean-source, cargo-install fixtures, release and
supply policies, architecture/effect boundaries, extraction, shipped catalog,
official MCP client interoperability and peer generation all pass. The
qualifier explicitly did not verify remote reachability. Remote push/tag/release,
crates.io, platform-specific service/UI realization and semantic-provider
confinement remain separately authorized follow-on work and are not claims of
this completed local scope.

### 2026-11-16 — workspace Keystone identity normalization

- **One selected substrate identity:** all six production dependency edges,
  the shipping lock, three qualification locks, the fail-closed dependency
  policy and coordinated source receipt now select Keystone
  `2e4faf10ec0a70805aa1db1555f93c77fcf0a1e4`, matching the workspace and
  Workbench source graph. The older receipts above remain historical evidence.
- **No category expansion:** Vellum still consumes only `keystone-codec`,
  `keystone-lifecycle`, `keystone-observe`, and `keystone-replay`; no
  presentation, application-shell, peer, authority-policy, or Workbench code
  moved into Keystone.
- **Qualification boundary:** local qualification uses the documented exact
  sibling Git-object rewrite while retaining the declared GitHub source
  identity. Remote reachability, publication, signing and activation remain
  unclaimed until separately authorized and proven.
