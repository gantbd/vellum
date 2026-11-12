# Vellum security and privacy threat model

Status: current local profile, fixed authority ceiling, semantic consumer gate
and observed-only local-provider qualification modeled; multi-principal/Aegis
and immutable provider confinement remain open. Reviewed against the source tree
on 2026-11-11.

This document applies STRIDE and LINDDUN to Vellum's current local daemon,
native protocol, MCP connector and local-filesystem adapter, plus the promised
semantic-service and Aegis integration seams. It is a release control, not a
claim that planned components exist or that a same-user process is trusted.

## Scope and governing assumptions

The modeled deployment is one macOS user running one Vellum daemon over a Unix
socket in a private directory. MCP is a supervised stdio peer which reaches the
daemon through the native client and transport. The service serializes access
to one in-memory workspace and the selected adapter accesses local files.

The engine is the sole authority over Vellum document, revision and history
state. Hosts establish identity and authority facts. Protocols carry bounded
requests and evidence. Connectors translate; adapters perform effects; neither
acquires semantic authority by proximity to the engine. Metrics and diagnostic
events are non-authoritative. This categorical ownership is part of every
mitigation below.

The current profile assumes the kernel correctly enforces Unix ownership,
permissions, descriptor and process isolation. It does not assume that every
process running as the same OS user is trustworthy. It makes no remote-network,
multi-user, sandbox, credential-vault, malware-resistance or universal
crash-durability claim.

## Assets and privacy interests

| ID | Asset or interest | Required property |
| --- | --- | --- |
| A1 | Source and annotation content, including unsaved text | Confidentiality; exact, authorized mutation; bounded retention |
| A2 | Document, workspace, history and revision state | Integrity, availability, isolation between client projections |
| A3 | Local files, locators, metadata and replacement evidence | Least authority, byte fidelity, truthful effect and durability claims |
| A4 | Client, invocation, request, witness and effect identities | Uniqueness within scope, bounded retention, no use as authentication by accident |
| A5 | Principal assertions, grants, scopes and revocation state | Authenticity, attenuation, expiry/revocation, non-transferability |
| A6 | Native/MCP messages and retained replies, including the local durable journal | Bounded canonical interpretation, replay distinction, confidentiality at each hop and at rest |
| A7 | Socket, lock, process and file descriptor identities | Exclusive ownership, substitution detection, safe cleanup |
| A8 | Effect, delivery and persistence evidence | Attribution and uncertainty preservation; never inferred success |
| A9 | Metrics, diagnostics, audit and transcripts | Content safety, bounded cardinality, declared purpose and retention |
| A10 | Semantic provider executable/profile/private cache, indexes, diagnostics and runtime-evaluation results | Immutable launch identity, workspace scoping, freshness, provenance, resource bounds, confinement and declared retention |

## Actors

- **Human operator:** selects files, starts components and may authorize work.
- **Agent or harness:** invokes bounded MCP tools; it is not trusted merely
  because it is supervised or names itself.
- **Trusted composition host:** establishes caller/session facts, selects
  adapters, limits and the local fixed action ceiling, and may later mediate
  Aegis. A defect here can invalidate the deployment's authority assumptions.
- **Same-user untrusted process:** can attempt socket connection, process/stdin
  observation, resource exhaustion, file races or endpoint substitution.
- **External filesystem writer:** editor, build tool, sync client or process
  outside Vellum's writer coordination.
- **Vellum daemon/service/engine:** trusted to implement their documented,
  categorically separate contracts; each still treats boundary input as
  untrusted.
- **Filesystem adapter and OS:** perform effects and return evidence; they do
  not decide whether an edit was authorized.
- **Local or future semantic provider:** an LSP/static-analysis/runtime bridge that must be
  treated as a separate, potentially fallible and content-observing process.
- **Future Aegis service:** decides bounded authority from host-established
  facts. Its decision evidence is not a reusable bearer credential.

## Trust boundaries and data flows

| Boundary | Flow | Data crossing | Authority rule |
| --- | --- | --- | --- |
| TB1 Harness → MCP stdio | JSON-RPC request; JSON-RPC/tool result returns | Self-asserted client description, correlation ID, tool arguments, source snapshots, refusals | MCP IDs and `clientInfo` do not authenticate or authorize |
| TB2 Native client → Unix socket transport | Length-prefixed canonical V1 request/reply | Client/request IDs, revisions, witnesses, operations, source/results | Private socket limits reachability only; protocol values convey no ambient authority |
| TB3 Transport → daemon/service | Decoded request and bounded reply | Closed command/refusal algebra and retained replay state | Malformed transport input cannot become a semantic command |
| TB4 Service → editor/engine | Validated application request/outcome | Workspace operation, expected revision, client-scoped witness | Engine owns source/history transition; service cannot invent success |
| TB5 Engine/host → filesystem adapter | Prepared read/save intent; attributed evidence returns | Bounded bytes, pinned conditions, resource handles, effect disposition | Host selects and authorizes capability; adapter cannot broaden it |
| TB6 Adapter → local filesystem/OS | Open, inspect, stage, write, sync, rename, cleanup | Content, metadata, descriptor and inode/device observations | Paths are locators, not identity or authority; uncertain effects stay uncertain |
| TB7 Vellum → semantic provider | Narrow query/request and typed result | Bounded text/ranges, language/workspace identity, diagnostics, symbols, edits | Provider advises; it cannot mutate Vellum state or execute code without a separate authorized operation |
| TB8 Composition host → authentication port → authority port; future Aegis adapter | Exact authentication and authority questions with separately request-bound evidence | Opaque challenge/claim/session references and adapter-owned credential context; then established principal/session, Vellum action/resource and future policy/version/expiry/revocation facts | Authentication can hand off only its exact established context and grants no actions. The optional local-daemon profile requires this handshake before frame decoding; ordinary commands retain a separate fixed action ceiling, while the exact export route consumes the fresh affine session as one factor of its operation/source/destination authority product. The default invocation remains owner-only offline; the CLI selects the profile only with an explicit credential path. Future Aegis policy still consumes host-established credential facts rather than authenticating credentials itself |
| TB9 Semantic supervisor → provider/OS | Qualified launch, bounded stdio and lifecycle control | Executable/profile identity, argv, environment, cwd, inherited handles, cache, process/resource and cleanup state | Launch authority is distinct from query authority; production composition must prove artifact continuity, confinement, resource ceilings and owned cleanup without ambient fallback |

Source may intentionally cross TB1, TB2, TB5–TB7. Encryption is not provided by
the current local transports; confidentiality depends on process/OS isolation
and on the harness, peer and filesystem destinations selected by the host.

## STRIDE analysis

Status meanings: **present** is implemented in the current profile; **partial**
has bounded mitigation but a residual gap; **required** is a release gate for a
future or broader profile.

| ID | Category and scenario | Boundary/assets | Mitigation and evidence | Status / residual risk |
| --- | --- | --- | --- | --- |
| S1 | Same-user process impersonates a human, agent or trusted host at the socket | TB2, TB8; A1–A5 | Private real parent directory, restricted socket mode and refused existing endpoint; the optional required daemon profile uses an OS-CSPRNG incarnation, monotone connection sessions and exact request-bound adapter evidence before decoding | Partial: the default invocation remains owner-only offline; the credential-enabled profile is only as strong as its private file and does not authenticate a remote peer |
| S2 | Harness supplies deceptive MCP `clientInfo` or reuses correlation IDs | TB1; A4–A6 | MCP description and IDs are treated only as description/correlation; native requests receive their own identities and replay rules | Present, provided no host promotes MCP metadata to authority |
| S3 | Path, symlink, hardlink, descriptor or socket is substituted | TB2, TB6; A3, A7 | Persistent lifetime lock, owner/mode/type/link/identity checks, identity-aware cleanup, anchored file operations and conservative refusal of unsupported aliases/types/metadata. Persistent crash recovery additionally requires exact recorded path/device/inode, refused connection and a second pre-unlink identity check | Partial: same-user processes can alter the non-cryptographic evidence; malformed, live, missing or conflicting evidence refuses without unlinking; external writers and final-check/effect races remain |
| S4 | Semantic peer claims another workspace/language server or returns results for stale content | TB7; A2, A10 | The host contract binds opaque request/document identity, exact Vellum source revision and optional peer index generation; transplanted completions fail closed and stale discloses no current version | Partial: the consumer gate exists; no peer authentication, provider or process provenance is currently trusted |
| S5 | A replaced executable, interpreter, profile or plugin is launched under previously qualified identity | TB9; A7, A10 | The local profile consumes single-use opaque artifact/profile evidence, hashes bounded executable bytes, binds canonical paths plus exact executable/cwd metadata and revalidates before worker admission and immediately before path-based spawn | Partial and still excluded from distribution: path spawn is not immutable descriptor execution, and interpreter, dynamic-library and plugin closure remain unproved |
| S5 | Principal assertion or authority decision is forged, replayed or transplanted | TB8; A4, A5 | The required local handshake accepts only exact challenge/claim/session-bound evidence, coalesces rejection/unavailability/transplant/replay, uses a fresh OS-CSPRNG daemon incarnation plus non-reused ordinal, and closes the session after its exchange. The optional local credential adapter binds the exact transcript to one descriptor-anchored private record. Authority remains a separate exact context/resource/action decision | Partial: expiry/revalidation/revocation, durable replay audit and Aegis integration remain open. OS randomness failure refuses every configured handshake |
| T1 | Native/MCP message is malformed, extended with unknown fields or version-confused | TB1–TB3; A2, A6 | Bounded readers, exact framing, canonical wire codec, closed versions/schemas, unknown-field refusal and semantic/protocol error separation | Present for the selected local profiles; no cryptographic integrity claim |
| T2 | Edit is silently retargeted after intervening workspace change | TB1–TB4; A1, A2 | Expected workspace revision, fresh native snapshot/witness for MCP replacement, stale refusal and retained request conflict detection | Present for exposed operations; witnesses are not authorization tokens |
| T3 | Retained request or durable operation identity is reused with different input, or another subject probes a receipt | TB2–TB4; A2, A4, A6 | Volatile replay remains client/request scoped. The separate durable operation relation keys a caller-chosen 128-bit reference by stable workspace and authenticated subject, then binds it to the stable explicit intent. Different subjects may independently use the same reference, eliminating the execute-time existence oracle; lookup coalesces subject/workspace mismatch with absence. Changed intent conflicts, pending remains unknown, startup canonically validates intent/receipt correspondence, and no live witness is treated as historical evidence | Present for native wire/client, required-auth daemon and authenticated MCP composition. Owner-only durable resume remains disabled; no remote/distributed exactly-once claim |
| T4 | Filesystem content or metadata changes between validation and publication | TB5–TB6; A1, A3, A8 | Pinned conditions, staged replacement, supported metadata preservation, rechecks and typed execution/synchronization evidence. Create-exclusive export binds the exact request to a descriptor-backed destination object and publishes with `RENAME_EXCL`; ancestor rename/swap cannot redirect it and collision is known nonapplication | Partial: ordinary overwrite remains best-effort detection rather than an atomic conditional write or external mutex; post-export directory-sync failure remains explicitly unknown |
| T5 | Diagnostic, index or semantic edit is accepted for the wrong source version | TB7; A1, A2, A10 | The consumer gate checks request/source/index binding and query/result compatibility. Rename remains a proposal; `vellum-semantic-apply-service` independently binds provider, client, document and source revision, re-enters ordinary Vellum authorization, rechecks current snapshots and publishes through the existing multi-document memory transaction. The pinned live rust-analyzer qualifier feeds its exact rename result through this gate and separately proves stale-source, provider-transplant and client-isolation refusal before one linked-history publication | Implemented and qualified for the named in-memory apply profile. Persistence remains independently authorized and reports `NotAttempted`; no cross-file disk atomicity or automatic semantic-plan application is claimed |
| T6 | Restart rebinds an old document/root ID by inventory order or to a retargeted locator | TB2, TB5–TB6; A2, A3 | Private bounded v3 record binds exact locator bytes to ordinals and separately retains issued prefixes plus reservation ceilings; adapter checks resource kind and containing-directory identity; service requires every old-prefix active binding to remain exact and every newly issued tail to be contiguous. Authorized runtime membership advances a non-reused generation/issued-prefix record before an accepted reply; checkpoint refusal or uncertainty closes the reply and fences | Present for startup and persistent-daemon runtime bindings. V1/V2 refuse rather than invent migration. Ordinary save replacement intentionally preserves locator identity; same-name/same-kind external replacement remains an explicit limit. Transient embedding retains runtime membership without a restart claim |
| T7 | A slow observer mistakes evicted coordination events for complete history | TB2–TB4; A2, A4 | Fixed retention, one-event wire pages, workspace-scoped cursors, and an accepted explicit Gap result; exact request replay retains the original page and never republishes | Present only within one process. Restart resets scope/feed; durable Hermes delivery is absent |
| T8 | A membership admission response leaks unrelated workspace shape or reconstructs the admitted set from a racy later inventory | TB1–TB4; A2, A3 | The editor returns the exact serialized transition. The service projects those native identities directly into file/directory admission receipts; directory identities are complete, bounded, ordered, workspace-scoped, and packed into one bounded wire field. Legacy whole-inventory admission tags remain decode-only | Present for native and MCP admission. Receipts are historical evidence, not proof of current membership after later commands |
| T9 | Executable, cwd, environment, inherited handles or restart generation changes between qualification and effect | TB9; A7, A10 | Launch consumes single-use qualified evidence and revalidates exact observed identities at the final boundary. Admission is serialized; timeout or unconfirmed cleanup retains an affine worker/fence owner. Confirmed absence or cleanup may clear the singleton fence; indeterminate cleanup yields `RestartRequired` and retains it. The initialize root is byte-derived from the same qualified workspace before process effects | Partial and still excluded from distribution: path-spawn races, inherited-handle closure and automatic restart remain open. Vellum linearizes initialize against its supervised stdout pump but does not prove an OS/ProcessKit pipe was physically empty |
| T10 | Semantic service and broker disagree about which requests a provider/source transition invalidated | TB7; A2, A8, A10 | Canonical bounded receipts use the ordered product pre-provider × operation × post-provider. One physical poll is one composite broker event; raw provider-state/delivery events are absent. The broker binds provider-before, validates observed/settled exact-successor stages, disposition and the captured request set using a shadow state, then commits atomically. Poll failure carries and commits any lawful pre-transition before fatal shutdown. Missing, extra, reordered, wrong-cause or wrong-epoch evidence latches fatal divergence; the actor fails, preserves recovery ownership and leaves the exchange outcome unknown. Bounded terminal correlation prevents request-ID rebinding and false late-response absence. Timed composition preflights its exact dual index before every effect, and one normalized cleanup owner retains actor/build origin, deadline, client, extraction and terminal evidence. Nonblocking stop admission immediately fences commands; same-spawn managed ownership joins distinct clean, refused, unknown, abandoned and failed-before-stop evidence with exact recovery and terminal cleanup | Present through supervised launch, broker/actor, bridge service, fresh-started LSP adapter, timed/deadline-disabled mapper, managed stop/recovery and terminal cleanup owners. A standalone process fixture qualifies framed composition, diagnostics, nested symbols, versioned rename and lawful failure, but production-language-server readiness and semantic quality remain unqualified. Malformed identity, readiness and provider payloads fail closed rather than becoming trusted semantic output |
| T11 | Candidate/prior payloads or desired deployment are substituted across an activation or rollback cut | Release artifact boundary; A7–A9 | Exact canonical approval binds distinct manifest digests and one transaction. Same-owner canonical payloads are rehashed against qualification evidence and exact inert plists. The opt-in executor synchronizes immutable records before and after every launchctl effect, verifies exact program/PID plus live/running/healthy/ready daemon status, and restores and requalifies the exact prior version after known candidate health failure. Any missing post-effect confirmation is `outcome_unknown`, never success or automatic retry. Offline reconciliation revalidates the exact identities plus a contiguous canonical state-machine prefix without executing an effect | Implemented for one effective-user `gui/<uid>` LaunchAgent label. Codesigning, notarization, live-state recovery after an unknown launchctl effect, multi-user policy and daemon/MCP atomic replacement remain external |
| R1 | Caller or service denies an edit, delivery or save | TB1–TB6; A4, A8, A9 | Attributed request/effect identities, typed retained outcomes, authenticated local effect receipts and content-safe counters aid diagnosis | Partial: the plaintext local receipt journal is neither durable audit nor non-repudiation proof |
| R2 | Authority decision cannot be related to the exact action later performed | TB8→TB4/TB5; A5, A8 | The service validates exact context/resource/action evidence before command interpretation and retains the resulting refusal/outcome under the request identity. Export additionally joins authenticated session, operation-derived resource, exact source version and pinned destination object before capture | Partial: local request/action binding is implemented, but durable multi-principal audit and policy-version causation are not |
| R3 | Provider restart cause, selected artifact or cleanup disposition cannot be reconstructed | TB9; A8–A10 | Restart policy retains generation/cause/fuse state; embedded and ordered replacement products retain exact transition, replay, invalidation and affine displaced-provider cleanup evidence. Actor suspension/resumption preserves content-safe usage rather than resetting it. Fresh real-process candidate usage and displaced/current terminal cleanup receipts remain independently owned | Partial: the binary does not yet join the complete replacement product; executable fingerprints remain excluded from metric labels |
| I1 | Source, paths, arguments or identities leak through metrics/errors/logs | All; A1, A3–A6, A9 | Fixed-cardinality content-free counters; documented exclusion of source, paths, identities, witnesses, arguments and raw OS errors | Present for declared metrics; harness transcripts, process inspection and third-party logs remain outside Vellum control |
| I2 | Snapshot or semantic query discloses more workspace content than needed | TB1, TB7; A1, A10 | Current MCP snapshot disclosure is explicit; future peer must use narrow, bounded query scopes and minimize transmitted content | Partial for MCP by design; semantic peer required |
| I3 | Socket/native traffic or the durable receipt journal exposes content to another local observer | TB2, TB6; A1, A6 | Private directory/socket, owner-only journal directory/files, no-follow/type/mode/owner/link checks and OS process isolation | Partial: no transport or at-rest encryption; requests contain inserted text/locators and `ReplaceAndSave` replies contain source snapshots; hostile same-user access, backups and sanitization are out of scope |
| I4 | Provider exfiltrates source or retains identifiable indexes/cache through ambient filesystem or network authority | TB7, TB9; A1, A10 | A production confined profile must deny unneeded filesystem/network/process effects and declare cache retention/deletion | Required before semantic-local distribution; cleared environment and private cwd are not confinement |
| D1 | Oversized, partial, slow or numerous requests exhaust memory, CPU or connections | TB1–TB3; A2, A6, A7 | Frame/text/field/client/lifetime bounds, bounded line reader, timeouts, sequential MCP loop and saturating observations | Partial: same-user callers can still consume admitted capacity and synchronous work time; rate/fairness policy is limited |
| D2 | Semantic server consumes unbounded CPU/memory, hangs, floods diagnostics or grows an index | TB7, TB9; A2, A10 | Query deadlines, cancellation, bounded result/stderr retention and supervised local lifecycle exist; the consumer contract rechecks provider output. The supported Unix launch profile installs `RLIMIT_CORE=0` and a `077` umask. An optional closed strict policy requires one ProcessKit whole-tree memory ceiling, process-count ceiling and CPU quota; exact ceilings are profile-bound, incapable hosts refuse before spawn, and group-creation failure refuses without ambient fallback. The live supervisor samples content-free ProcessKit resource statistics and preserves per-axis availability and high-water observations without converting unsupported values to zero | Partial: the compatible and pinned rust-analyzer profiles remain explicitly unbounded and report all three enforcement axes unsupported. macOS/POSIX process groups expose an active group unit but no CPU, memory or I/O accounting, and backend measurements are not cross-platform comparable. Index/cache/FD bounds, filesystem/network/subprocess confinement, abrupt-parent cleanup, bounded automatic restart and provider distribution remain open; a process-count ceiling is not subprocess confinement, and core-dump suppression is not a resource sandbox |
| D3 | Filesystem stalls or repeated collisions consume service availability | TB5–TB6; A2, A3 | Explicit adapter refusals and bounded metadata/content; no hidden retry policy | Partial: filesystem latency and platform availability are external premises |
| D4 | Complete durable receipts and pending intents permanently consume the fixed journal | TB2, TB6; A2, A6, A9 | No invisible eviction; one daemon-owned shared capacity across both versioned relations; pre-effect capacity refusal; aggregate retained-slot occupancy/capacity/saturation gauges; native authenticated retirement uses a distinct `RetireReceipt` authority, durable marker and permanent non-reused tombstone, with crash/restart qualification. Authenticated MCP projects only that exact operation and closed outcomes; it neither grants authority nor falls through to lookup/execute | Active receipt capacity is reclaimable only through the independently authorized native operation. Epoch rotation remains unavailable; retired-identity exhaustion still requires bounded refusal, larger preconfigured capacity or a future separately authorized rotation/migration |
| D5 | Provider crash loop, fork/output bomb or outcome-unknown cleanup exhausts the host | TB9; A2, A7, A10 | A bounded restart controller enforces one affine cleanup/backoff/launch flight, finite nonzero backoff, no query replay, and permanent uncertainty/exhaustion fuses. The ordered actor can suspend, replay snapshots into one pinned-ready fresh process, invalidate pending queries and deadlines, retain the displaced provider, resume only after its terminal cleanup, preserve accumulated usage, and permanently fence old senders. Reconciliation divergence is non-resumable | Automatic binary installation remains absent until policy composes the complete candidate/current/displaced cleanup product with physical confinement and elapsed-time evidence. Queries are never automatically replayed |
| E1 | Connector, adapter or semantic peer bypasses engine admission and mutates authority directly | TB1, TB5, TB7; A1, A2, A5 | Dependency law and closed ports: inbound peers submit protocol commands; outbound adapters only interpret prepared effects | Present architecturally; dependency tests must continue to enforce it |
| E2 | A path, witness, request ID, MCP capability or diagnostic is mistaken for authority | All; A3–A5, A8, A10 | Types separate locator, correlation, evidence and grant roles; every current service command maps to a closed action and is mediated before editor, witness, discovery or persistence effects. Semantic query/synchronization and plan application use distinct lattice atoms and the apply service performs a fresh authority check | Present for the current native and local semantic command sets; runnable semantic transport, runtime-evaluation authority and dynamic principal policy remain separate gates |
| E3 | Authority is widened, reused after revocation, or used for a different action/resource | TB8; A5 | The fixed action set is a finite bounded lattice, defaults read-only, attenuates only by meet, and returns exact request-bound evidence; future dynamic policy requires expiry/epoch revocation and publication-time recheck | Partial: static local attenuation is implemented; dynamic revocation/Aegis is not |
| E4 | Provider project configuration, plugin or child process obtains effects outside the semantic query capability | TB9; A1, A5, A10 | Production confinement must close filesystem, network, subprocess, IPC and inherited-handle capabilities with no ambient fallback | Required before semantic-local distribution; the current selected provider may inspect these independently |

## LINDDUN analysis

| ID | Privacy threat | Surface | Mitigation and evidence | Residual risk / requirement |
| --- | --- | --- | --- | --- |
| L1 | Linkability of one person's files and actions across sessions | Client/request IDs, socket/process timing, transcripts, Aegis and future audit | Bounded scoped identities, volatile retention and content-free metric dimensions | OS/process observers and harness logs can correlate; future durable audit needs purpose and retention limits |
| L2 | Identifiability from paths, source, symbols, diagnostics or principal references | MCP snapshots, filesystem, semantic peer, authority seam | Ordinary Vellum metrics exclude these values; interfaces should use opaque resource/principal references where feasible | Source itself can identify an author/project; disclosed content cannot be made anonymous by opaque IDs |
| L3 | Unwanted non-repudiation through durable detailed activity records | Future audit/Aegis, transcripts, retained outcomes | Current counters and replay state are volatile and not audit proof | Any durable audit must minimize fields, declare access/retention and separate security need from behavioral surveillance |
| L4 | Detectability that a file, project, language or operation exists | Socket/process presence, traffic sizes/timing, tool catalog, semantic-server process | Fixed catalogs and content-free logs avoid direct naming | Presence and traffic shape remain visible to eligible local observers; no traffic-analysis resistance claim |
| L5 | Disclosure of source, inserted text, paths, errors, grants or indexes | Every effectful boundary | Private local endpoints, strict result schemas, redacted/content-safe observations and conservative adapter errors | MCP/harness, filesystem and semantic peers intentionally receive selected content and become independent disclosure domains |
| L6 | Unawareness about which agent/peer reads or mutates content | Host composition and MCP tool calls | Current tool catalog separates snapshot, single-document replacement, atomic in-memory workspace edit, and persistence; host is responsible for presentation/consent. Workspace-edit receipts expose the exact changed identities/revisions and state that persistence was not attempted | No built-in human consent or purpose UI; multi-agent use requires orchestration policy and visible attribution |
| L7 | Non-compliance with purpose, minimization, deletion, residency or access obligations | Logs, histories, transcripts, provider cache/indexes, future audit | Bounded state and explicit disconnect/retirement support minimization mechanisms | Provider notice/purpose, cache/index retention and deletion disposition remain undeclared; process exit or receipt retirement is not erasure, and no general privacy-policy, residency or retention scheduler is provided |
| L8 | Durable identity metadata creates a cross-session file-location index | Private workspace identity record | Record is private 0600 under a 0700 owner directory, strictly bounded, contains no source bytes, never enters metrics/errors, and is used only for startup recovery | Exact locator bytes and stable ordinals are intentionally linkable at rest; host backup/retention and directory access policy remain deployment responsibilities |
| L9 | Membership removal reveals another client's capture activity | Document-removal receipt and operational evidence | Public/wire receipts report only opaque identity and `Removed`/`AlreadyRemoved`; revoked-witness cardinality remains a content-free saturating internal counter and is never a reply field or metric dimension | Eligible same-process operators can observe aggregate counter movement; client identity, witness identity and per-document dimensions remain excluded |
| L10 | Runtime membership checkpoints create an expanding durable path/history log | Persistent identity boundary | Each atomic generation contains only the bounded current active locator bindings plus non-reused ordinal high-water marks; removal drops the locator from the next record, and operational evidence is content-free | Filesystem remnants, backups, snapshots, and the adapter's transient stage may preserve prior record bytes outside Vellum's deletion claim; locator unlinkability is not claimed |
| L11 | Coordination telemetry reveals who edited which content | Change-feed boundary | Events omit source, locators, edit ranges, client/request/witness/subject/session identity; counters expose only aggregate polls/pages/gaps/publication/eviction | Opaque resource IDs and source revisions remain linkable within one process; traffic timing remains detectable |
| L12 | Receipt cleanup or status becomes an existence oracle, ABA reuse or unauthorized erasure | Durable journal lifecycle | Implemented retirement binds server workspace, authenticated subject, operation ref and exact intent; absent/already-retired/foreign coalesce; complete mutation requires the distinct global `RetireReceipt` ceiling; pending intent is ineligible; a permanent bounded tombstone absorbs delayed reuse. The additive status projection is read-only, content-free, bound to the active authenticated subject and workspace, and coalesces absent with foreign without exposing intent or receipt bytes. MCP delegates only to these distinct native operations and exposes no authority-bearing value | Qualified locally through native and authenticated MCP paths: no adapter-only delete/expiry/reset, active capacity releases only after validated cleanup sync, and malformed crash debris fences; a same-subject caller holding an operation reference can distinguish its pending/complete/retired state by design. Plaintext remnant sanitization and backup erasure remain outside the claim |
| L13 | A durable export destination reveals project layout and remains linkable after receipt retirement | Export intent, pending journal record and permanent tombstone | Public receipts, Debug, metrics and status omit the destination; authenticated lookup coalesces foreign ownership with absence; bounded retirement removes the replayable receipt | The canonical intent and absorbing tombstone retain the literal destination in private plaintext storage; filesystem side effects are independently visible; encryption, unlinkability, backup erasure and human-purpose/consent are not claimed |
| L14 | Durable activation state links exact software versions and operator activity | Private activation approval, immutable versions, pointer and journal | Scratch and executor receipts omit paths and payloads and retain only fixed policy labels, digests, phase/outcome and a caller-provided transaction correlation. Executor journal entries are same-owner, create-exclusive, mode-closed and synchronized around effects | Manifest digests, journal count and transaction timing remain intentionally linkable; backup retention, deletion, operator identity, launchd/system logs and purpose/notice remain deployment responsibilities |

The local authentication projection deliberately does not distinguish rejected
claims, unavailable providers, malformed evidence, transplantation, or replay.
Operational evidence is aggregate and fixed-cardinality; it contains no claim,
challenge, subject, session, credential, or provider error. Eligible local
observers can still correlate connection timing and traffic shape, and an
adapter may create its own independent identification/disclosure domain.

## Boundary-specific controls and nonclaims

### Local daemon and native protocol

Release evidence must retain adversarial framing/version/unknown-field tests,
request replay/conflict/stale tests, client/lifetime-capacity tests, slow/partial
I/O tests, private endpoint checks and inode-aware cleanup tests. The current
profile has a persistent single-owner lock, verified endpoint identity,
phase-aware not-sent/outcome-unknown transport evidence, and an owner-only
shutdown control that wakes acceptance and drains admitted obligations.
That same local control socket exposes a fixed-size versioned status projection
containing only liveness, lifecycle, health, readiness, aggregate USE
utilization, and closed saturation. It exports no RED request/error counts,
latency distribution, paths, content, identities, principals, arguments, or
raw errors. Status reads copy already published state and cannot probe or alter
readiness. The client checks a stable same-effective-user `0600` single-link
socket identity, while explicitly making no authentication claim for processes
sharing that account. Malformed, partial, slow, wrong-version, unsafe-mode, and
identity-replaced endpoints fail closed under the existing bounded timeout.
SIGINT/SIGTERM observation re-enters that exact control path. Exact bounded
predecessor endpoint records allow conservative recovery after a killed
process; absent or conflicting evidence remains an operator-visible refusal.
This is not cross-user authentication, a crash-proof record update, or a total
wall-clock shutdown deadline.

The current socket is not an authenticated multi-principal or remote boundary.
Canonical encoding prevents ambiguous interpretation; it does not provide a
MAC, encryption, peer identity or durable replay protection.

### MCP peer

MCP remains a projection over the native client, never the canonical authority.
Its process supervisor must constrain executable selection, arguments,
environment, working directory and transcript handling. The current sequential
stdio profile does not promise preemptive cancellation, concurrent fairness,
remote MCP security, human consent or durable audit.

### Semantic session boundary

The semantic session protocol treats exchange correlation, mutation replay
identity, membership and incarnation as independent opaque evidence. None is
authority. Every post-join exchange is rebound to the exact incarnation and
membership, and state-changing intent is replayed only under an exact operation
identity. Unknown, duplicate, reordered, trailing, malformed and over-limit
wire values refuse before service dispatch.

The standalone MCP composition assigns lifecycle/synchronization and query
correlations to disjoint lower/upper 128-bit partitions with monotone headroom.
Before an arrived request executes, it reserves a worst-case synchronization
epoch plus one request operation. Falling below that reserve drives bounded
Leave, fresh Join and complete snapshot reopening while retaining the same MCP
owner and stdio lifetime. This prevents cross-role correlation ABA and prevents
ordinary exhaustion from consuming the retirement tail. It does not silently
compact historical broker replay evidence or rotate the service incarnation.

The pure serialized broker prevents cross-membership request collisions and existence
oracles by remapping `(membership, external request)` to sovereign internal
request identities. Terminal deliveries remain exact and non-destructive until
Release, after which a bounded non-reusable tombstone prevents ABA. Exactly one
synchronizer owns an epoch; loss fences queries and completes bounded cleanup
before replacement. Provider fingerprints, source values, manifest evidence
and opaque identities are forbidden in error text and metric labels.

The transport-neutral session client now retains phase-aware
`NotSent | Reply | OutcomeUnknown` truth, validates every reply binding and
keeps exact ambiguous mutation intent for explicit replay. The broker and client
remain pure components. The bounded serialized actor adds only mailbox and
effect-interpreter ownership: a pre-admission refusal is `NotSent`, while a
worker or effect failure after admission is `OutcomeUnknown`. Trusted policy
loss and retirement use a separate bounded control path so command saturation
cannot suppress revocation. The local service now owns a private Unix listener
and one fixed credential challenge/response before semantic framing. This
establishes local attribution only; it still claims no encryption, remote peer
identity, durable semantic replay or automatic retry.

Every authenticated transport exit, including ordinary peer loss rather than
only process shutdown, projects the connection's exact subject/session/policy
epoch onto the actor's authority-loss control path. While waiting for the next
connection, the service drives bounded retirement so abandoned memberships,
requests, synchronized documents and synchronization ownership cannot remain
live indefinitely. Admission and completion are reported only as content-safe
control outcomes. Failure to admit authority loss remains explicit and does not
become a false cleanup claim.

After retirement reaches exact quiescence and the provider owns no external
cleanup obligations, a trusted actor control replaces the complete broker with
a fresh OS-CSPRNG incarnation before the listener accepts another connection.
The transition preserves only the current provider epoch and
observation/provenance; memberships, synchronization
witnesses, request tombstones and join/operation replay intent remain intact
until that atomic boundary and are then retired as one old incarnation. Same-
incarnation, non-quiescent, queued-command and outstanding-obligation attempts
refuse. This is explicit incarnation retirement, not in-place replay eviction.

`SemanticQuery` and `SemanticSynchronize` are separate closed host authority
actions. Authentication does not grant either action, and Query does not imply
Synchronize. The local probe composition derives the role ceiling through an
independent authenticated policy port, then the join request attenuates it. The
current operator probe grants both roles; dynamic principal policy remains a
deployment obligation.

The editing daemon's one-request-per-connection authentication profile is not
used as semantic membership: every semantic membership is bound to the one
authority session retained by its authenticated sequential connection.
Reconnect is not session resumption, and plain Unix-domain transport plus HMAC attribution provides neither
confidentiality nor same-user peer isolation.

Ready-to-next-epoch synchronization is a bounded transaction: the client seeds
a private stage from committed witnesses, fences queries, preflights projected
document/source/witness retention, and publishes only an exact accepted
Complete. Refusal cannot false-publish and outcome uncertainty retains the
stage behind the replay fence. Synchronization epochs cannot recur within an
incarnation. Retained old witnesses cannot validate a late Complete result
unless the membership is still witness-validated at the exact Ready epoch and
the provider is Ready; rejected late results retain their request reservation.

The broker's compact document witness retains language, exact target/revision,
source byte length and UTF-8 boundary positions, but not source bytes. Those
sizes and boundaries remain linkable process-local metadata and must not become
telemetry dimensions. Its SHA-256 seal is collision-resistant commitment under
an explicit digest-induced quotient, never literal equality or authority. The
shared checked logical-byte metric is protocol-content accounting, not wire,
allocator, RSS or physical-retention measurement. `ReleaseRequest` drops retained result bytes while keeping
the non-reusable request tombstone; it is lifecycle reclamation, not secure
erasure of provider, allocator, swap, crash-dump or transport copies.

| Model | Threat at this boundary | V1 control | Residual / nonclaim |
| --- | --- | --- | --- |
| STRIDE: spoofing / elevation | Correlation or self-requested roles treated as authority | Opaque identities are evidence only; the local channel authenticates once, installed policy independently attenuates role requests, and exact incarnation/membership bind replies | Local HMAC attribution is neither remote identity nor same-user isolation; the operator probe policy is intentionally coarse |
| STRIDE: tampering / repudiation | Frame mutation, operation ABA or ambiguous lost reply | Canonical strict decoding, exact mutation-intent replay, operation budget, non-reusable request tombstones, transport disposition separated from application outcome | No MAC, signature, durable audit ledger or transport persistence claim |
| STRIDE: information disclosure | Cross-member request oracle or sensitive values in telemetry | Sovereign request remapping, absent/foreign coalescing, redacted Debug and closed metric-label rule | No transport confidentiality; frame sizes and timing remain observable |
| STRIDE: denial of service | Oversized frames, counts, text, retained requests or replay state | Exact deployment-narrowed limits, count-before-allocation, operation/request budgets, reserved cleanup, request-bound membership rollover before ordinary headroom is exhausted, authority-loss retirement on every authenticated transport exit, quiescent fresh-incarnation rotation between clients, and affine MCP reconnect after definitive Leave plus definitive global `ReplayCapacity` | Reconnect never follows a not-sent or outcome-unknown mutation and provides no durable replay, remote availability or unbounded lifetime claim; pending retired-membership collections become exact replayable cancellation receipts |
| LINDDUN: linkability / identifiability | Digest, document counts, opaque IDs or provider fingerprint correlate workspaces and users | These values are excluded from logs, errors, discovery metadata and metric labels | A transport observer may correlate length, cadence and connection lifetime |
| LINDDUN: non-repudiation / detectability / disclosure | Retained terminal results or compact witness shape reveal activity, language, cardinality or source-length structure | Exact membership-scoped retention, content-free fixed-cardinality observations and explicit Release; source is absent from errors and observations | Release is not secure erasure; transport confidentiality, storage encryption and provider-cache deletion remain composition obligations |
| LINDDUN: unawareness / non-compliance | Agent queries without human-visible purpose or retention policy | Protocol exposes capability/readiness only and grants no UI consent authority | Human notice, consent, purpose limitation, deletion policy and jurisdictional compliance remain composition obligations |

The authenticated catalog exposes create-exclusive copy export only as a
seventh intent shared by the durable execute/lookup/retirement tools. Syntax discovery
is not a grant. Exact document/revision/destination matching is enforced again
by the daemon policy; the response is locator-free and mismatched receipt sort,
document or revision is `service_malformed`. The live restart qualification
now includes separately launched shipped daemon and MCP binaries. It persists
an operation reference before execution, drops the first MCP process without
consuming its response, restarts both processes, and recovers the retained
receipt without repeating the saved edit. The in-process matrix remains the
stronger source of exact counters and injected fault classification; the
process case proves only the named local profile.

### Workspace and filesystem adapter

Workspace membership is not filesystem authority. A locator is not stable
identity. Save authorization, immutable intent, provider execution and baseline
publication remain separate stages. The supported adapter profile refuses
conditions whose metadata/link/race semantics it cannot preserve; operators
must not weaken those refusals to gain apparent compatibility.

The adapter does not sandbox an agent, lock out external writers, provide a
credential vault, establish atomic conditional writes, or guarantee universal
crash durability. Unknown application or synchronization outcomes must remain
unknown through the service and connector layers.

### Semantic peer and LSP adapter

Expose narrow semantic queries such as bounded symbol search, definition,
references, diagnostics and type-at-point. Do not expose an editor process as
ambient authority. Rename/refactor responses are proposals bound to exact
document versions; applying them is a separate Vellum mutation authorized and
revalidated through the ordinary path.

The current `vellum_host::semantic` consumer gate implements those bounded
query/outcome, freshness, capability and passive-observation contracts.
`vellum-semantic-service` additionally owns exact whole-snapshot admission,
strictly increasing revisions, provider readiness/index state, bounded pending
and delivery pools, cancellation, stale invalidation, exact completion
validation and per-client routing. The client facade carries no grant, and an
opaque client reference is not authentication. A wrong client cannot inspect,
change, close, cancel or receive another partition's work. Late/unknown
completions are counted without retaining their content.

The semantic apply service is a separate mutation gate, not authority carried
by a provider plan. It requires an exact client-owned semantic/workspace
binding, current ready provider provenance and generation, exact `EditSource`
evidence, final source revisions, scalar-bound nonoverlapping ranges and bounded
payloads before one in-memory workspace prepare/commit. Expected source strings
are derived from those final snapshots rather than trusted from the provider.
A stale, locked, foreign, missing or malformed participant prevents the whole
publication. Its first receipt exposes persistence only as `NotAttempted`, so
memory atomicity cannot be confused with multiple filesystem writes.

The current LSP adapter selects no executable or concrete transport. It accepts
only an already-owned byte port, path-free injective virtual URIs and a bounded
generation issuer. Generic initialization remains indexing until a private
server-profile readiness port supplies current-index evidence. A pinned
protocol profile instead binds an opaque nonzero profile digest, exact expected
server identity and one audited readiness method before startup; absence or
mismatch fails closed, and independent polling cannot promote its readiness.
Terminal retirement retains that exact adapter's content-safe identity,
readiness, capability and usage evidence with process cleanup. Provider
provenance is composition-installed; peer-reported identity is only accepted
as an exact pinned match. Strict framing, static capability negotiation, exact
request/revision/generation/provenance binding, aggregate source capacity,
cancellation, partition-bound result locations, and fail-closed
diagnostic/symbol/rename-plan validation reduce parser differential,
cross-document alias/disclosure, stale-result and memory-exhaustion risks.
Protocol shutdown is correlated and distinguishes graceful exit from explicit
abort. Strict rename decoding accepts only current versioned text edits and
rejects resource operations or annotations. Provider replacement publishes only
after deterministic full snapshot replay under fresh stable provenance and
generation; delivery capacity is preflighted before invalidation. External
deadline evidence becomes a distinct timeout rather than cancellation. Whole
snapshots still disclose source to the selected server. The exact bounded
deadline scheduler exists, but an automatic background clock driver, immutable
executable provenance and supervised process restart remain open.

The local semantic supervisor narrows executable provenance to an
operator-constructed canonical absolute executable and fixed argv, clears the
ambient environment, rejects loader-affecting variable names, installs only
explicit variables, and uses a canonical effective-user-owned `0700` private
cwd outside the canonical workspace root. A bounded content hash and exact
filesystem identities feed opaque redacted artifact/profile identities; their
single-use qualification is revalidated before worker admission and again at
the final path-spawn boundary. Supported Unix launches install `RLIMIT_CORE=0`
and a `077` umask. Bounded queues, aggregate
stdin byte credit and stderr accounting prevent unbounded retained I/O; close
and abort are monotone admission fences. Deadline escalation and ordinary
owner drop/unwind
distinguish accepted whole-group hard kill from confirmed finish/reap of the
supervised leader and make no descendant-reap or macOS abrupt-parent-death
cleanup claim. Unexpected, nonzero and
unconfirmed cleanup fail closed. Launch admission is serialized; a launch
timeout or unconfirmed cleanup retains an affine worker/fence owner. Confirmed
absence or cleanup clears the singleton fence explicitly; indeterminate cleanup
produces restart-required evidence and leaves the fence set. Dropping the owner
moves it to an exact-once recoverable process-global slot rather than detaching.
The outer wait remains bounded even when the worker is delayed. Synchronous
qualification/revalidation work is outside that asynchronous launch deadline.
This is local mechanism evidence, not a sandbox, signature, immutable executable
descriptor, network-denial guarantee or protection from a same-authority
replacement race after the final observation. Provider interpreters, dynamic
libraries, plugins and project interpretation remain properties of the selected
language server.

Provider launch and workspace/project trust remain injected operator policy. In
particular, the service and adapter do not
execute project configuration, plugins, build scripts or runtime evaluation;
adding an adapter that does so would create a new effect caller and threat-model
gate. Whole snapshots intentionally disclose the admitted source to the chosen
provider, so client isolation is not anonymization and provider retention or
exfiltration remains an unclosed privacy risk. Fixed counters are content-safe
logical RED/USE evidence, not provider
latency, CPU/RSS, golden-signal, semantic-correctness or availability evidence.

Language servers and runtime evaluators are distinct risk classes. Static
analysis still observes source and may execute project configuration or plugins.
Runtime evaluation can execute user code and requires a separately selected
sandbox/process/network/filesystem profile and explicit grant. No runtime
evaluation authority follows from a diagnostic or symbol-query grant.

### Current authority seam and future Aegis adapter

The current local profile installs a host-selected fixed interpreter. Default
constructors admit only workspace inspection, source reads and document
selection. File admission, directory discovery, editing and persistence are
independent explicit ceilings. Every fresh retained request is checked before
command effects; a matching replay returns the retained reply without a second
decision. Denied, unavailable and mismatched-evidence outcomes fail closed and
remain separately observable without recording principal, path or source data.
This is static single-OS-user composition, not caller authentication,
delegation, consent, revocation or Aegis policy.

The trusted host authenticates or otherwise establishes principal/session
facts; Aegis does not validate credentials. Vellum asks Aegis whether that
principal may perform one bounded action on one resource under current
version/session facts. Vellum must fail closed when a decision is missing,
expired, revoked, malformed or unavailable and must recheck before publishing
an effect where intervening revocation matters.

Decision evidence is attribution, not a transferable bearer grant and not
proof the effect completed. Vellum must not deserialize live grants from its
document store, infer scope from a path, cache beyond the decision lifetime, or
log raw grants/principal assertions. Embedded and daemon profiles must enforce
the same action/resource mapping.

## Release gates

The following gates are cumulative. Evidence must identify the exact profile,
revision, platform and limits; passing a lower tier does not imply a higher one.

### G0 — GitHub source distribution

- Threat model and security-reporting links are present in the source archive.
- Workspace publication remains disabled by the release-contract gate.
- Existing boundary, protocol, MCP, daemon and adapter tests remain green.
- Cargo-install evidence uses fresh private Cargo/install/target roots, exact
  daemon+MCP binary closure, non-symlink non-writable executables, content-safe
  startup refusals and no ambient build flags or global Cargo cache. A lone
  Vellum archive and an empty-cache third-party source closure remain open.
- Documentation does not advertise authentication, sandboxing, remote service,
  atomic conditional save, durable audit or privacy compliance not implemented.

### G1 — Local single-user daemon

- Private directory/socket owner, mode, type and identity are verified without
  following attacker-controlled aliases.
- A lifetime single-owner lock is qualified. Exact recorded crash residue is
  recovered only after refused-connect plus owner/mode/type/link/identity and
  immediate pre-unlink rechecks. Missing, mismatched, malformed, live or
  otherwise unproven evidence refuses without unlinking; automatic launchd
  policy remains an external deployment responsibility.
- Transport reports not-sent versus outcome-unknown and preserves semantic
  uncertainty; graceful drain has bounded obligations and a tested deadline.
- STRIDE cases S1, S3, T1–T4, D1, D3 and E1–E2 have executable adversarial
  evidence. Content-safe RED/USE/golden signals saturate without affecting
  semantics.
- The release is explicitly described as same-user and single-machine. The
  CLI defaults to owner-only offline and selects required authentication only
  with an explicit private credential. That profile is principal-authenticated
  only to the extent of its file-backed adapter and does not satisfy the
  Aegis/dynamic-policy requirements of G2.
- The inert launchd renderer follows no symlinks, requires an exact same-owner
  `0700` state directory and private persistent/log inputs, refuses existing
  endpoint paths and unresolved placeholders, and emits only the read-only
  direct-exec profile. Rendering is neither plist installation nor activation;
  it adds no socket-activation or authenticated-health claim.
- Release-candidate qualification executes both the candidate and selected
  rollback renderers under fresh private scratch authority. Exact inert output
  and `plutil` acceptance are required even when the renderer bytes and
  manifest checksum agree. Retained evidence contains only closed dispositions,
  not the deployment record, plist or private paths. This reduces tampering and
  disclosure risk at the packaged-behavior boundary without granting install,
  activation or `launchctl` authority.
- The scratch activation transaction binds exact candidate/prior digests,
  stages immutable mode-closed version bundles, and atomically selects one
  desired bundle through a synchronized pointer. Its durable journal retains
  explicit terminal/refused/unknown cuts and content-safe receipts. It executes
  no payload and reports `launchdEffect: not-attempted`.
- The separate opt-in launchd executor consumes the same qualification and
  affirmative approval identities, rehashes both daemon payloads, accepts only
  their exact inert plists, and requires an exact healthy prior job before any
  mutation. Synchronized pre/post records bracket every launchctl command.
  Candidate success requires exact program/PID and bounded version-2
  live/running/healthy/ready status; known health failure triggers exact-prior
  rollback and the same status gate. Tests inject every platform effect and
  never invoke launchctl. Ambiguous launchctl completion is terminal
  `outcome_unknown`, not rollback or retry.
- Offline reconciliation validates the complete immutable journal prefix and
  exact candidate/prior identities, rejects gaps and impossible transitions,
  and classifies durable terminal versus pending/unknown residue. It performs
  no observation or effect, never claims current launchd state, and always
  requires explicit operator action for a nonterminal prefix.

### G2 — Agent/human multi-principal use with Aegis

- Host-authenticated principal/session facts and closed Vellum action/resource
  mappings are reviewed and tested across embedded, native and MCP paths.
- Attenuation, expiry, revocation, request binding, publication-time recheck,
  Aegis-unavailable failure and non-bearer evidence handling are qualified.
- Default authority is read-only; write, filesystem discovery, semantic query,
  refactor and runtime evaluation are separate least-privilege scopes.
- Audit fields, access and retention have an explicit purpose/minimization
  review addressing L1–L3, L6 and L7.

### G3 — Semantic peer

- Peer process provenance, workspace/document/version binding and supported
  language/profile are explicit.
- Query/output bytes, items, diagnostics, process memory/time and index lifetime
  are bounded; cancellation, crash, stale output and restart are tested.
- Proposed edits re-enter Vellum authorization, stale checks and history; peer
  results never mutate source directly.
- Static analysis and runtime evaluation have separate capabilities and threat
  models. Runtime execution remains disabled unless independently sandboxed and
  authorized.
- Disclosure, transcript, cache/index deletion and project-configuration/plugin
  behavior are documented and tested for the selected peer.

### G4 — Remote service

No remote release is currently designed or authorized. It requires a new
network threat model and independent evidence for authenticated encryption,
credential lifecycle, origin/resource-server policy, admission/rate/fairness,
tenant isolation, remote cancellation, durable replay/audit, privacy retention,
incident response and deployment hardening. Local gates do not compose into
these guarantees automatically.

## Review triggers

Re-run this analysis when a trust boundary, actor, protocol version, authority
mapping, adapter profile, durable store, semantic/runtime peer, telemetry sink,
network transport or supported deployment changes. Every accepted threat must
map to an executable test, formal model, platform qualification or explicit
residual-risk/nonclaim; prose alone does not close a gate.
