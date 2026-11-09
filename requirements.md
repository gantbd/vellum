# Vellum Requirements

V1 delivery clarification: the four user-approved F05/F16/C4/C5 choices in
[the specification's adopted v1 delivery clarifications](specification.md#adopted-v1-delivery-clarifications)
govern conflicting earlier language here. Logical-source naming is host-resolved
with engine-issued native bindings; distinct provider-context activation is
deferred; generic observation-ID lookup is integration-owned; and returning an
unresolved shutdown report does not permit final release of retained owners.
These are selected requirements, not claims of implementation or proof completion.

## 1. Purpose

Vellum is a reusable editor for prose, code and mixed textual documents, written
in Rust. It should provide an editing experience in the spirit of CodeMirror
while remaining independent of a particular UI framework.

Its architecture is a presentation-independent object/transformation kernel with
provider and UI adapters, not a requirement to emulate human screen interaction.
V1 remains a text/code editor library: documents are text-authoritative and the
approved adapters below remain required. The runtime symbolic content view in
section 3.4.1 is distinct from the architectural digital twin in section 14.

This document specifies intended behavior and architectural direction, not a
claim of delivered functionality. Read it through explicit contracts, bounded
state/effect models and independently supported evidence: formal reasoning,
model checking and testing have distinct scopes and limitations. Design
principles in section 2 summarize the assurance, security and observability
requirements developed in later sections.

Agentic work may span multiple workspaces containing code, notes, and other
textual domains. A workspace is a logical grouping, not necessarily a Git
repository, filesystem directory, UI window, or transaction boundary. Work
scope and concurrency must not be limited to the documents a person can view
simultaneously; explicit capabilities and resource budgets govern execution.

The editor must be usable as a pluggable widget in both `iced` and `ratatui`.
Applications may use either frontend to build a standalone editor or embed the
editor in a larger product.

### 1.1 Approved v1 scope

The first usable release targets both general text and code editing as a
reusable library. It includes embeddable `iced` and `ratatui` adapters and two
thin reference applications that demonstrate integration and exercise shared
behavior. The reference applications are examples and conformance tools;
supporting complete end-user editor applications is a separate commitment.

The release must include:

- Insertion, deletion, replacement, atomic multi-edit transactions, multiple
  selections, navigation, indentation, read-only enforcement, and undo/redo.
- Multiple documents and multiple views of a document, selection mapping across
  edits, and explicit document and view lifecycles.
- Document-associated comments and annotations, distinct from source text and
  transient decorations. Section 3.8 records the agreed minimal v1 contract;
  it does not require collaborative review.
- Agent edit proposals with anchored range and line targets, declared
  local document dependencies, and explicit conflict results. Exact-revision
  validation is sufficient for v1; mapping proposals across intervening changes
  is an optional later capability. Proposal identities and dependency references
  must not assume that all authors or resources share one workspace.
- An engine behavior facade with typed APIs, stable command identifiers, command
  metadata/discovery and host command registration. Key bindings and chords
  belong to frontend support and the reference applications, not engine state.
- Literal search, next/previous match navigation, selection-scoped search,
  replacement, and atomic replace-all.
- Optional incremental tree-sitter parsing and highlighting, including bounded
  mixed-language regions. Markdown fenced blocks are the reference use case,
  with a host grammar, at least one registered embedded grammar, and graceful
  fallback for unavailable languages, including Mermaid when unsupported.
- Scrolling, selection interaction, clipboard effects, and basic decorations in
  both frontend adapters.
- Typed host effects and completions, capability checks, cancellation, resource
  bounds, and reference file loading and saving integrations.
- Local operation identities, explicit host capabilities, and distinct applied
  and persisted outcomes. Cross-authority plans must be rejected as unsupported
  unless a host explicitly supplies the required coordination capability.
- A digital twin, deterministic tests, frontend conformance tests, applicable
  threat coverage, optional telemetry, and explicit performance budgets.

Deferred capabilities include rectangular selections, advanced structural
editing and navigation, collaborative editing, session recovery, scripting,
binary plugin ABIs, untrusted extension execution, regular-expression search,
workspace-wide search, language-server integration, semantic completion,
formatting, minimaps, elaborate gutters, IDE panels, autosave, file watching,
and external-change merging. Deferral of merging does not remove the need to
define protection against overwriting externally changed files.

Unicode correctness is part of the initial contract. Extended grapheme clusters
are the default foundation for user-facing character navigation, with explicit
tailoring where necessary. Shaping, bidirectional layout, and terminal width
require separate contracts. Frontends must share editing semantics; platform
differences in input composition and accessibility must be documented in an
explicit capability matrix.

Accepting host-supplied cross-document change plans does not require a built-in
language server, workspace search engine, or semantic rename engine in the
first release. Such plans must declare their required application guarantees;
hosts that cannot provide those guarantees must reject them explicitly.
The symbolic-kernel direction does not add a universal graph database, ontology,
inference/query language, semantic indexer, or structured-authoritative object
profile to v1. V1 consumes declared dependency/evidence contracts; stronger
discovery integrations remain optional host capabilities or deferred work.

Later sections describe both initial and longer-term requirements. Sections 1.1
and 1.2 govern v1 inclusion; a later section's "must" applies when the associated
capability is implemented and does not promote a deferred capability into v1.
Detailed behavior, supported platforms, budgets, and acceptance evidence remain
to be specified through the decisions listed in section 17.

[specification.md](specification.md) traverses the v1 areas with contract-first
semantic drafts. Its [coverage register](specification-coverage.md) tracks the
remaining clause-level audit, and its [decision register](specification-decisions.md)
collects material choices for user review after the drafting pass. These
are draft semantic contracts, not the full
v1 public API or a claim of implementation/proof completion; its explicit open
decisions do not override agreed requirements.

### 1.2 V1 boundary and compatibility

V1 is a local Rust editor library with both adapters and reference applications.
It must be usable without a network, identity service, policy server, durable
workflow engine, or remote client. A local authority may manage documents from
multiple workspaces; workspace membership must not imply a network boundary.

| Area | V1 commitment |
|---|---|
| Editing and workspaces | One local authority, shared documents and views, atomic local multi-document text commits |
| Agent proposals | Typed range/line targets, expected revisions and declared local dependencies, explicit conflicts |
| History | Local undo/redo with explicit grouping and linked local operation identity |
| Persistence | Host load/save effects, revision-specific outcomes, external-change protection |
| Execution | Deterministic local commits and bounded host-executed work |
| Automation | Typed in-process host invocation, registered Rust commands and bounded declarative composition |
| Authorship | Host API invocation with origin metadata; no proximity-based authority assumptions |
| Authorization | Explicit host-granted capabilities and read-only enforcement |
| Connectivity | No required listener, relay or transport |
| Replication | One authoritative editable instance per managed document |

Compatibility means preserving identities, ownership boundaries, typed
effects, origin metadata, and explicit outcomes. It does not require v1 to
implement a speculative distributed protocol or every future contract type.
Optional host integrations may add explicitly advertised guarantees but must not
weaken the base library's guarantees or silently claim stronger ones. Detailed
deferred designs have been removed; exclusions are not a future feature backlog.

The architectural integration review does not itself resolve the architectural
choices tracked in section 17.14. Their externally observable contracts must be
agreed before concrete specification; subsequent decisions are recorded there.
Exact types, schemas,
budgets, platform commitments and verification criteria remain a subsequent
pass; no storage dependency or verification tool is selected by these requirements.

## 2. Design principles

1. **One editor, multiple frontends.** Editing behavior must not be reimplemented
   by each UI integration.
2. **Semantic operations first.** User-visible operations must be addressable as
   commands through a public facade, independently of how they are invoked.
3. **Responsive by construction.** Interactive state changes must be fast and
   deterministic; expensive and independent work should run off the UI thread.
4. **Host-controlled integration.** The core describes external work and consumes
   its results without requiring a particular async runtime, UI framework, file
   system, clipboard, or language service.
5. **Incremental work.** Editing, parsing, highlighting, layout, and rendering
   should update only the state affected by a change whenever practical.
6. **Embeddability.** Public APIs must support widgets, full applications,
   headless tests, and programmatic use.
7. **Model before implementation.** Material design changes must first be
   represented in the project's digital twin so dependencies, interfaces,
   identities, data movement, failure modes, and trust boundaries can be
   examined before tests or production code are written.
8. **Explicit contracts and refinement.** Separate safety, progress and resource
   guarantees from environmental assumptions. Preserve observable contracts across
   implementations and compatible releases; a closed verb set does not prove
   whole-system finiteness, termination or lattice closure.
9. **Least authority and safe data movement.** Validate identity, current
   capabilities, freshness and disclosure at material boundaries. Authentic
   content, proximity, workspace membership and correlation do not confer
   permission. Trusted in-process extension code is not sandboxed by the facade.
10. **Observable pathways, protected observations.** Design MELT and golden
    signals along end-to-end pathways and at interfaces, including overload and
    failure branches. Apply STRIDE both to those pathways and their observation
    pipeline; telemetry supports diagnosis but is not authoritative operation
    state, preventive enforcement or durable audit.
11. **Independent, bounded assurance.** Combine specification/correctness review,
    explicit formal-methods claims and independent implementation evidence.
    Follow Cleanroom-inspired defect prevention and provenance discipline without
    claiming full process conformance or source-isolated reimplementation.
12. **Scope and evidence honesty.** Distinguish v1 from deferred capabilities and
    agreed design from implemented/verified behavior. Unsupported guarantees,
    unknown outcomes and observation limits must be explicit, not silently
    substituted with weaker semantics or hidden by optional diagnostics.

## 3. Architectural requirements

### 3.1 Component boundaries

The project should be divided into independently usable layers:

- An editor core containing document state and editing semantics.
- A framework-neutral workspaces coordinator for cross-document validation,
  atomic in-memory publication, and shared operation history metadata.
- A public facade that exposes supported capabilities.
- A command registry and command execution system.
- Tree-sitter and language-related services.
- View, layout, and rendering models that do not depend on a UI framework.
- An `iced` widget adapter.
- A `ratatui` widget adapter.
- Optional standalone applications built on those adapters.

Framework-specific event, rendering, and runtime types must not leak into the
core editor API.

Catalog-to-substrate correspondence is established by the trusted host/substrate,
then validated and retained as an opaque engine-issued binding to the complete
original registration. Locator resolution and physical identity evidence remain
outside the engine. Catalog labels cannot manufacture native provider authority,
equate opaque identities or couple independent read/write registration lifetimes.
Consumer currentness checks must not erase original dispatched evidence or replace
separate cancellation, discharge and retention contracts.

The engine owns document/annotation semantics, explicit logical selections and
their mapping, history, dependency/authority validation, atomic publication and
structured outcomes. It presents those behaviors independently of input devices,
focus, presentation or an interactive session. Relevant semantic configuration
belongs to its accepted interpretation; an application preferences object does
not make every preference engine-owned.

Frontends own keymaps, chords, interaction modes, focus, gestures, palettes,
themes, native composition/preedit and presentation policy. They resolve input
to explicit semantic targets and invoke the engine facade. Optional shared UI
helpers may serve both adapters without becoming engine dependencies. Useful
keymap/chord reference work is assigned to this layer, exercised by the ratatui
reference example; it is not a requirement for a second editing implementation.

Geometry, rendering and viewport policy remain optional adapter-facing services
under §8.1. Host/provider implementations execute filesystem, clipboard, native
accessibility and other external services; the engine owns only the applicable
semantic requests, validation and evidence contracts, not OS implementation.
An optional service being available through an editor integration does not make
its runtime state a mandatory part of the engine. Copy/cut/paste workflow verbs
and clipboard-specific task state belong to a headless-capable integration facade
above engine behavior contracts. UI gestures and host clipboard mechanisms remain
separate; engine capture, validation, publication and generic effect/evidence
contracts remain authoritative. Integration cannot create a second mutation path.

Changing a UI binding, mode, theme or pending chord without dispatching a semantic
command must not change engine documents, history or semantic generations, or
consume engine invocation slots. Delayed UI work must not silently retarget a
command to newly focused content. Engine admission rechecks its own authority,
dependencies and target versions independently of frontend enablement.

Each layer/interface must declare what state it owns, what guarantees it requires
and supplies, its authority/disclosure boundary, and its resource/failure model.
Use these contracts to compose pathway guarantees under section 14.4; crate
separation alone is not a proof of behavioral isolation or independent failure
containment. Optional analysis and telemetry must not become prerequisites for
ordinary authoritative editing.

### 3.2 Authoritative state

- The editor must have one authoritative mutation path for document and
  selection state.
- Serialize admitted mutations in a defined local order. Transitions must be
  deterministic given state and admitted events; independent authorities and
  asynchronous arrival need not share a global or schedule-independent order.
- Background workers must not mutate authoritative editor state directly.
- Organize internal authoritative state into typed components with explicit
  version, dependency and lifecycle contracts. Source, annotations, history and
  accepted configuration retain distinct ownership semantics, but participating
  changes stage/validate and publish coherently through the coordinator. Derived
  caches remain outside authoritative publication unless an explicit contract
  requires participation. This internal modularity does not add public dynamic
  registration, a plugin runtime or arbitrary publication callbacks.
- State exposed to background work must preserve an immutable identified source
  snapshot; cheap cloning must not share mutable authoritative text with workers.
  Immutability is not confidentiality or revocability. Section 15.5 establishes the
  read/disclosure lifecycle; an unrestricted snapshot delivered outside its
  trusted processing boundary is already a disclosure.
- Text-changing revisions must increase monotonically within a document
  incarnation. Reopening/replacing an incarnation must not let late work match
  a reused revision; cross-process/global monotonicity is not a v1 promise.
- Document text revisions, coordinator publication generations, view/selection
  state and configuration generations serve different freshness purposes.
  Validate the identities/generations an operation actually depends on rather
  than treating one text counter as a version of the entire system.

### 3.3 Public facade

Engine editing behaviors must be exposed through a cohesive public facade,
independent of how a UI, host or agent chooses to invoke them. This includes:

- Document edits and transactions.
- Comment and annotation queries/mutations and their host persistence requests.
- Cursor and selection manipulation.
- Navigation.
- Undo and redo.
- Search and replacement.
- Syntax-aware operations.
- Engine semantic configuration changes.
- Authorized content capture and validated insertion/deletion; clipboard requests,
  responses and copy/cut/paste workflows use the separate integration facade.
- File and language-service requests where those capabilities are enabled.
- Command discovery, inspection, validation, and execution.

This facade inventory describes capabilities when enabled, not additions to v1
scope. Queries and outcomes must be bounded and respect disclosure policy just
as mutations respect write authority. Pure inspection must not implicitly load
code, disclose text to a model, or dispatch an external effect.

Viewport/scrolling, keymap/mode changes, palettes and native UI actions belong
to optional presentation/frontend interfaces. A composite editor may expose them
alongside engine commands, but must identify their owner and must not route
UI-only transitions through engine publication merely to share one registry.

The facade must support both typed Rust APIs and stable command identifiers for
dynamic discovery and invocation.

Command discovery must expose bounded, versioned metadata for permitted verbs,
parameter/coordinate domains, required capabilities and supported outcome/context
profiles. Metadata is descriptive, not a grant; discovery and invocation each
apply current authorization. Human adapters and agents invoke the same contracts
without keystroke replay or implicit current-buffer focus. Unsupported capabilities
must be explicit rather than inferred from a command name or an installed provider.

### 3.4 Workspaces coordination and ownership

- Vellum must provide a workspaces coordinator above independently usable
  document editing primitives. It may manage multiple workspaces and textual
  domains; a single-document embedding remains supported.
- The v1 engine instance hosts one local authority managing many objects, not
  one editing engine per file. Single-owner identity does not require a dedicated
  thread or global process-wide singleton. Independent authorities may use
  separate instances; layout state belongs to each view/context, not each file.
- Each mutable document must have one authoritative owner at a time. Workspace
  membership must not create duplicate mutable copies of one document. Within
  one coordinator's authority, all commits must pass through its mutation path;
  direct mutations must not bypass dependency validation or operation ordering.
  Independent authorities need not share an executor or lock.
- Independently usable document primitives must not provide a second live write
  path into a coordinator-owned document. Any attachment/detachment or ownership
  transfer must preserve the single-owner invariant and invalidate incompatible
  outstanding work. Section 3.7 establishes the agreed resource alias, rebind and
  disposal direction; concrete contracts remain for specification under section
  17.14. A filesystem path is not sufficient authority identity.
- Workspace membership, document identity, authority identity, and cross-domain
  operation identity are distinct. Plans must explicitly identify participating
  resources and capabilities rather than infer authority from workspace labels.
- The host owns file access, external analysis, execution resources, and
  persistence. Workers and language providers return proposals or results, not
  direct mutations. Frontend adapters must not coordinate document commits.
- Preparation may run outside the mutation path using snapshots. Final
  validation and publication must be serialized within a local authority,
  bounded, synchronous, and free of host I/O or arbitrary extension callbacks.
  Required external authorization/freshness evidence must be obtained before
  that step under an explicit validity contract and rechecked against accepted
  local policy/dependency state. A remote policy service must not become a hidden
  callback inside publication or an unsupported claim of cross-system isolation.
- Coordinator snapshots must identify a consistent published state. Observers
  must be able to obtain all affected documents from one committed coordinator
  generation rather than inadvertently combine intermediate versions.
- Atomic local commits may span multiple workspaces managed by one authority.
  Across independent authorities or external systems, the host must provide an
  explicit coordination contract. Saga execution may be used when the plan
  permits intermediate visibility and compensation; strict atomic plans require
  an appropriate stronger capability and must otherwise be rejected.

#### 3.4.1 Presentation-independent symbolic kernel — agreed direction

The logical kernel exposes typed objects, versions, relationships, bounded
queries and validated transformations across objects/workspaces, without
requiring files, windows, pixel/cell geometry or simulated human interaction.
This runtime content twin is distinct from model.md's architectural design twin.

- Preserve one authoritative owner per object. Text documents remain
  text-authoritative in v1; derived symbols/relationships carry source context,
  provider provenance, freshness and declared discovery scope, not independent
  mutation authority or an assumed complete model of meaning. Semantic providers
  resolve transformations into ordinary validated proposals.
- Keep authority explicit per object rather than imposing a universal AST or
  graph-as-source representation. Future structured-authoritative object profiles
  and generated projections require their own explicit contracts and are not
  added to v1 by this direction. Authored annotations remain under their existing
  first-class contracts, distinct from inferred semantic evidence.
- Put filesystem/provider I/O, serialization implementation, native input,
  rendering, shaping and platform geometry at adapter boundaries. Agents and UIs
  use the same semantic mutation authority; adapters cannot bypass validation.
  Resource substrates must be modular behind engine-owned resource/effect ports;
  the first target module is the local filesystem. Additional substrates such
  as S3 require their own supported capability/identity/guarantee profiles and
  conformance, not filesystem emulation or a v1 cloud-provider implementation.
  Concrete provider/SDK/credential types must not enter the semantic engine.
  This module boundary is separate from the in-memory source/snapshot backend.
  Presentation-independent does not mean all view/query state is public: every
  projection, including relationships, counts and freshness tokens, obeys the
  existing disclosure boundary.
- Keep fidelity requirements, logical coordinate meaning, identity, authorization,
  dependency validation, history, atomic publication, effect evidence and resource
  bounds in the kernel's observable contracts. Their mechanisms may live at the
  edges, but failure there cannot weaken those guarantees. Whitespace, comments,
  malformed intermediate text and serialization metadata are not disposable
  presentation. Non-file providers use the same typed effect/result boundary.
- Separate kernel refinement from adapter conformance: prove allowed object-state
  transitions and scoped dependency/evidence properties independently of pixels;
  then verify conversion, fidelity, authorized release and input/effect translation
  at each adapter. No universal semantic completeness or bidirectional lossless
  mapping follows merely from representing relationships as a graph.
- Keep grammar parsers, document-content schema validators and domain-semantic
  analysis behind optional provider interfaces outside the authoritative core. The core owns
  object identity/revisions, permissions, typed dependency and evidence contracts,
  bounded change admission, history and atomic publication; it need not implement
  the interpretations. Validation of operation envelopes and core typed contracts
  remains mandatory; optional schemas here describe document content. Outside the
  core is a dependency/ownership boundary, not a requirement for a separate
  process, service or network. Ordinary editing
  must not depend on a current symbol graph or successful structural analysis.
- Prefer established applicable grammars and schemas for provider interpretation
  of prose, code and mixed documents instead of inventing competing structures.
  Name the accepted standard/dialect, version and explicit extensions; distinguish
  normative conformance from conventions and heuristic evidence. Grammars describe
  syntax, schemas constrain structure/values, and domain semantics may require
  additional evidence. No universal AST/ontology or validator is required in the
  core, and this direction does not add deferred structural-editing features to v1.
- Bind structural targets, validation evidence and proposed transformations to
  exact source/region mappings, provider and grammar/schema/profile versions and
  all relevant declared dependencies. An operation requiring structural validity
  or semantic completeness must obtain evidence sufficient for that guarantee;
  missing, stale, ambiguous or partial evidence cannot silently become a weaker
  operation. If conformance is promised, validate the candidate under the named
  applicable schema/profile before publication and revalidate its dependencies
  at publication without arbitrary provider callbacks in that boundary. Invalid
  or incomplete source remains editable by operations not requiring conformance;
  handled provider refusal/unavailability and rejected malformed/stale results
  must preserve source and not disable ordinary editing. Native/process-integrity
  faults retain the containment/fail-fast limits in sections 4.4 and 7.6; this
  logical boundary is not a sandbox or unconditional availability promise. Line/token/
  syntax targets are projections resolved to the shared change contract, not
  independent ownership or publication mechanisms. Lossless preservation remains
  mandatory, not a capability inferred from successful parsing or tokenization.

The state classifications below apply regardless of physical storage:

| Class | Authority and lifecycle | Examples / prohibited inference |
|---|---|---|
| Authored state | One owning mutation path; explicit identity, versions, history and persistence contract | Source text and annotation bodies/explicit targets; a parser cannot overwrite them as a cache refresh |
| Derived evidence | Context-bound, attributable and invalidatable; no independent source-mutation authority | Parsed symbols, inferred relationships, discovery results and automatic attachment evidence; a claimed relationship is not permission or proof of complete meaning |
| Consumer projection | Currently authorized release from authored state/evidence; bounded lifetime and scope | Agent query, tool result or human view; presentation does not create a second authoritative source |

Annotation records can contain authored and derived fields; classify the fields
rather than treating the whole record as either disposable cache or source text.
Logical view/selection/preedit state has its existing owner/lifecycle and is not
silently promoted to document content or a global semantic fact.

CQRS organizing direction: commands request validated authoritative transitions;
queries read identified snapshots or authorized projections without mutating
authored state. Derived read models retain source context/freshness and coverage
limits; an evidence-backed command still revalidates at publication. Authoritative
snapshot reads need not be eventually consistent merely because derived queries
can lag. Distinct interfaces/models do not require separate databases, services,
event sourcing, durable messaging or an expanded v1 semantic engine. Queries may
populate bounded derived caches/accounting without acquiring source ownership.
Validated external-effect observations use their evidence-ingestion contract and
cannot be dropped as ordinary new commands under overload. CQRS does not itself
choose or prove the protected-capacity/fair-sharing policy in section 7.5, whose
direction was subsequently agreed in residual review.

#### 3.4.2 Snapshot sharing and retention — agreed direction

Exclusive mutation authority coexists with shared immutable, version-identified
snapshots. Readers and workers may retain an accepted snapshot while the owner
publishes a newer state; they acquire no live mutation authority. Structural
sharing is allowed, not a selection of a text structure, pointer type or memory
reclamation algorithm. Long-running analysis must not hold the publication lock.

- Distinguish object lifetime, retained snapshot lifetime and current access
  validity. Disposal need not reclaim all snapshots immediately; retention does
  not renew permission or recall bytes already disclosed. Stale semantic evidence
  may remain physically retained but cannot support a current claim without its
  required validation. Published snapshots must remain immutable to their readers.
- Multi-object reads within one authority use a coherent identified snapshot or
  a version set with an explicit consistency contract, not accidental mixtures.
  Independent authorities require a declared coordination guarantee; relationships
  alone provide no cross-authority snapshot consistency.
- Storage must support indexed source-coordinate/line access, immutable snapshots
  and batch edits under explicit processing/retention budgets behind engine-owned
  interfaces. Avoid requiring whole-document materialization for ordinary bounded
  range access. Retained versions, fragmentation and reclamation costs are part
  of the contract/evaluation, not hidden by cheap snapshot cloning. A persistent
  UTF-8 rope leads the candidates; a piece tree remains a benchmark challenger.
  No algorithm, crate or numeric performance guarantee is selected here.
- A successful publication must support an authorized handoff to its identified
  resulting snapshot/version set, without waiting for rendering or derived indexes.
  Handoff is not unlimited retention or a new grant: recheck current release,
  charge retained snapshots and report unavailable/expired context explicitly.
  Incremental subscriptions must identify their source context and report gaps
  or invalidation with an authorized snapshot-resynchronization path; clients
  must not silently continue from a missing delta. Derived evidence may remain
  pending/partial even when the authoritative snapshot is available. Tokens and
  sequencing must not disclose activity outside the recipient's release scope.
- Charge actual retained obligations, including worker-held snapshots, staging,
  history and effect evidence. Cancellation intent alone cannot refund live
  retention. Define shared-storage accounting without alias-based evasion; exact
  accounting and numeric limits remain specification work under section 7.5.
- Under pressure, eligible derived caches may be dropped, never unsaved authored
  content or required unresolved-effect evidence. Reject/delay additional work
  when its obligations cannot be safely admitted; the precise admission policy
  remains open. Reclamation must not invalidate live snapshots, resurrect disposed
  incarnations or silently split retained history groups.

This closes the ownership/sharing architectural direction, not R-02's concrete
synchronization, discovery or reclamation proof obligations.

### 3.5 Scope boundary

V1 supports one local authority. Remote protocols, federation and replication
are out of scope; their deferred designs have been removed.

### 3.6 Unified operations, durability and supervision

- Human input, agent plans and host API requests use one operation model and
  authoritative mutation protocol. They are not separate editing modes.
- Distinguish milestones: applied to live authoritative state; recoverable from
  durable evidence; persisted at a specified destination and revision; and
  complete according to the operation's required steps. Do not collapse these
  into one success flag or assume they always occur in one linear order.
- V1 promises applied state and explicit host persistence outcomes. Crash
  recovery requires an additional durable recording contract; an applied
  acknowledgment alone must not imply recovery after process or machine loss.
- Local supervision manages executor lifecycle, restart bounds and escalation;
  worker restart is not evidence that an uncertain external effect is safe to repeat.
- Local commits must not await unrelated network calls, agent reasoning or the
  completion of a cross-authority workflow. Required durable acknowledgment may
  incur storage work under its declared contract; it is not made cost-free by
  asynchronous execution. Independent operations may continue within dependency
  and authority constraints.
- Keep the bounded local publication step distinct from acknowledgment contracts
  that depend on storage or other effects. No durable acknowledgment may be
  fabricated to keep an interface synchronous; any stronger host contract must
  expose its latency, uncertainty, failure and recovery assumptions explicitly.

### 3.7 Document/resource identity and lifetime — agreed direction

The agreed direction for the first architectural item is to make the document the
editor's logical entity and represent external resources through explicit,
versioned bindings. Agreement establishes architectural direction, not a completed
specification or a claim of persistent identity across process loss.

| Model | Strength | Tradeoff |
|---|---|---|
| Locator/path is document identity | Simple lookup for file-only editors | Renames, aliases, replacement at the same path and scratch/non-file text make identity and authority ambiguous |
| Physical resource ID is document identity | Can recognize aliases within a provider's declared identity domain | Not universal across providers, replacement and resource lifetime; does not naturally identify unsaved or generated documents |
| Document identity plus explicit resource binding — recommended | Separates editing lifetime, storage routing, versions and permissions; fits multiple workspaces and text domains | Requires a binding registry, host identity contract and explicit lifecycle operations |

- Distinguish document identity/incarnation, text revision, resource locator,
  host-declared resource key and freshness token, binding generation, and
  workspace/view membership. Content equality or a content digest does not
  establish resource identity or shared mutation authority.
- A live document incarnation has one authoritative text owner. Within one
  authority and host-declared resource identity domain, admit at most one live
  writable binding owner per resource key. Opening a known alias should resolve
  to the existing owner or report conflict, never overwrite unsaved text or
  create another authoritative mutable copy. A duplicate draft must be explicitly
  independent and unbound, not a second writer masquerading as another view.
- The host defines resource-key equivalence, scope, lifetime and uncertainty,
  including symlinks/hard links, case rules and replacement behavior where
  applicable. Locator normalization and physical IDs are evidence, not universal
  equivalence rules. V1 need not discover every filesystem alias; state the
  supported guarantee and reject admission when unresolved identity prevents
  satisfying a required uniqueness or write-safety contract.
- Resource resolution and joining an existing document must independently check
  read/write/disclosure rights. An alias match, known identifier or workspace
  membership must not disclose an inaccessible document or widen its authority.
  Resolution, load and admission may perform host work before bounded publication;
  simultaneous admissions must revalidate binding ownership before activation.
- Residual-review direction: ordinary save requires continuity with the captured
  binding under the provider's declared logical replacement semantics, not silent
  following of whichever object later occupies a locator. Providers must declare
  resource-key equivalence, continuity across rename/replacement/rebinding,
  identity-evidence validity/uncertainty and write-safety guarantees. Physical IDs
  and path normalization are evidence, not universal identity. Provider-controlled
  atomic replacement may preserve logical continuity where explicitly contracted.
  Retargeting/Save As is explicit and validates destination/overwrite/binding
  preconditions. Known aliases share owner/coordination identity; unresolved
  identity cannot silently satisfy a required uniqueness or write-safety claim.
- An unbound scratch document is a normal document. Rebinding a document changes
  its resource relationship/generation, not automatically its text identity or
  revision. Copy/export, rebind/Save As and renaming an external resource are
  distinct actions when exposed; none may silently overwrite or merge another
  document owner. Destination authority, ownership and required persistence
  guarantees must be validated before final binding activation.
- Effects identify the captured binding generation, exact resource/destination,
  document incarnation/revision and attempt. A rebind must account for pending
  writes: drain or establish their safe disposition before incompatible use of
  the old/new destination. An unknown write cannot be presumed cancelled.
  Historical completion evidence remains attributable to its original binding;
  it cannot mark the current binding saved or redirect a write to a new locator.
- View closure, workspace removal, unloading, document disposal and external
  deletion are different events. Closing the last view must not itself discard
  text/history or cancel unrelated work. Explicit disposal requires authorized
  discard or a satisfied retention/persistence contract, with conflict/refusal
  when unresolved obligations prevent safe disposal. No mandatory dialog is
  implied; hosts/agents use the same structured lifecycle outcomes.
- Keep view/membership, binding, residency and outstanding-operation state as
  separate dimensions rather than one overloaded open/closed flag. V1 may retain
  admitted live documents in memory and reject further admission under pressure;
  automatic unload/reload or durable recovery is not required. Any later eviction
  must preserve promised state or explicitly invalidate it under a declared
  contract; saved text alone is not preservation of history or unsaved state.
- Disposal invalidates mutation handles for that incarnation. Later open/reload
  must not reuse an incarnation in a way that accepts obsolete work. Late
  computations cannot resurrect a disposed target; late external-effect evidence
  is handled at the operation/host boundary, not erased or applied to a new
  incarnation. Outstanding work and retained snapshots need bounded retention;
  requests for disposal do not guarantee immediate reclamation or effect recall.
- Read/history/subscription handles and queued disclosures require a separate
  current-policy contract, not an inference from retained storage or a once-valid
  mutation handle. Section 15.5 establishes those lifetime and release boundaries;
  disposal cannot recall copies that were already disclosed.
- Verify ownership uniqueness under concurrent open/rebind, alias access without
  disclosure, last-view closure with unsaved text, and late save/parse completion
  after rebind/disposal. Model admission/disposal transitions and safety separately
  from eventual reclamation assumptions. Derive independent reference scenarios;
  observe admission, binding changes, refusal and late evidence through bounded,
  content-safe MELT/golden signals, with STRIDE at resolver and lifecycle interfaces.

References: [Emacs buffer/file identity](https://www.gnu.org/software/emacs/manual/html_node/elisp/Buffer-File-Name.html)
distinguishes names, visited files and file identity evidence;
[Neovim buffer validity/residency](https://neovim.io/doc/user/api/) separates
valid handles from loaded contents; [Helix commands](https://docs.helix-editor.com/commands.html)
distinguish view closure, buffer closure and writing. These inform separable
contracts, not adoption of their exact lifecycle or platform policies.

### 3.8 Comments and annotations — agreed v1 direction

Document comments and annotations are required capabilities. The recommended
architecture, now agreed, is a separate, first-class annotation layer attached to
document identity, with revision-bound targets and derived presentation. The
minimal v1 scope below does not imply richer review workflows.

| Representation | Strength | Tradeoff |
|---|---|---|
| Comments inserted into source text | Travel with ordinary source saves and text history | Change the artifact, require language-specific syntax and cannot represent private or non-source review cleanly |
| View-only decorations | Lightweight for transient diagnostics/highlights | Insufficient ownership, lifetime, persistence or history for authored comments |
| First-class annotations with host-managed persistence — recommended | Preserve document text and support shared views, structured targets and independent disclosure | Require annotation identity/version, anchoring, lifecycle and save contracts separate from source files |

- Treat a comment as a textual annotation body with a document-wide, point or
  range target; other registered annotation kinds may supply typed, bounded
  metadata. Annotation identity and content/version are distinct from text
  revision, target mapping generation and any rendered decoration. Transient
  parser/search diagnostics need not become authored annotation records.
- V1 includes bounded plain-text comment bodies, document/point/range
  attachment, identity/version-aware create/read/update/delete and explicit
  reattachment, shared headless APIs, and basic discoverable/accessible indication
  and text access in both adapters. Threaded conversations, mentions, notifications,
  assignment, approval workflows, rich-body execution and federated synchronization
  remain outside the agreed minimum. No elaborate review panel is required.
- Target creation binds document incarnation and source revision; range/point
  coordinates and boundary affinity use the normal anchor contract. Text edits
  map attachment state coherently with publication, across storage chunks and
  mixed-language regions. Positional mapping is not proof that a comment remains
  semantically applicable: distinguish located/mapped, affected/stale and
  detached/orphaned attachment evidence without conflating it with comment status.
- Use common, typed attachment/mapping semantics for authored annotation targets,
  derived diagnostic/decorative ranges and proposal targets, with explicit source
  context, boundary affinity and deletion disposition. Keep their applicability,
  persistence, history and release profiles distinct; reuse does not turn a
  diagnostic into authored content or a mapped proposal into an authorized edit.
  Producer namespaces prevent collisions but grant no read/write authority.
  Mapped location alone never proves semantic identity or target survival. Use
  purpose-specific closed profiles over one mapping algebra: positional targets
  may collapse to a declared boundary, while content-sensitive targets distinguish
  mapped location, affected applicability and detachment. Nonempty content ranges
  exclude pure insertions at their outer boundaries by default; collapsed ranges
  use one explicit point affinity, never inverted independently mapped endpoints.
  Partial target edits do not certify applicability; complete original-target
  deletion/replacement, including adjacent-edit coverage, detaches without erasing
  authored body/version state. Equal replacement bytes alone do not prove identity.
  Mapping never refreshes an exact-revision proposal or grants authority. Concrete
  point/replacement profiles and ordinary caret/selection defaults remain explicit
  refinement work rather than a universal clamping or detachment policy.
- Deleting a target must not silently delete its authored body or attach it to
  unrelated adjacent text. Retain an explicit detached/stale record under bounded
  policy; missing mapping history, external reload or ambiguous quote matching
  cannot authorize automatic reattachment. A quote/context selector may aid a
  suggested reattachment, not silently establish identity or edit authority.
- Annotation mutations use the same validated, capability-controlled operation
  path, with their own version preconditions. They do not advance source-text
  revision merely because a comment body changed. Queries and compound actions
  must identify the text and annotation generations they depend on; any required
  atomic text-plus-annotation operation must declare its participants/guarantees
  rather than publish an inconsistent anchor/body combination.
- Annotation authorship/content history, source undo and automatic anchor mapping
  are distinct. Source undo must not erase another author's later comment edit;
  attachment restoration through undo requires explicit validated mapping.
  Section 5.8 establishes the agreed frontier-only grouping/reversal direction;
  no independent text history is created per view.
- Close/rebind/dispose follows document identity under section 3.7, not whichever
  path or view happens to be current. Annotations count as retained authored
  state when deciding discard/disposal. Copying/exporting source text must not
  automatically copy annotations, provenance or permissions to another document.
- Keep source saving and annotation saving separate milestones and modified-state
  dimensions. Expose host load/save effects for identified annotation snapshots,
  including failure/unknown outcomes; source-save success does not imply comment
  persistence. Do not mandate sidecar files, a database, embedded markup or a
  durable service. Cross-process reattachment requires explicit stable document/
  resource identity and source-version evidence, not a reused session handle.
- Authorize annotation visibility and create/update/delete/export separately from
  source access where host policy requires it. Read-only source may allow comments
  without permitting text edits; source read access does not automatically reveal
  private annotations. Preserve verified versus claimed authorship. Comment text
  is untrusted data, not an agent instruction or capability grant; richer content
  or links cannot silently execute, load remote resources or exfiltrate text.
- Annotation bodies are only one disclosure surface. Section 15.6 establishes
  recipient-scoped metadata, query/event projections and operational disclosure
  limits, including counts, versions, quoted context and history conflicts.
- Compound annotation create-result authority is scoped to one original invocation
  component and its engine-bound allocation, not every object created by that
  invocation. Original component ordering is immutable; a component index is a
  locator, not a credential. Existing-record grants stay identity-specific.
  Capture each component's disclosure ceiling at admission and intersect it with
  current result rights. Publish historical component facts with the whole
  compound; replay, filtering or later edits cannot rebind or broaden them.
- Bound body sizes, annotation/target counts, retained versions and mapping work.
  Unresolved unsaved annotations require explicit retention/discard or admission
  refusal, not silent eviction. Verify attachment laws and concurrent version
  conflicts with independent scenarios, including target deletion, undo, rebind,
  reload and late completion. Apply STRIDE and content-safe MELT/golden signals
  across annotation admission, mapping, presentation and persistence interfaces.

The [W3C Web Annotation Data Model](https://www.w3.org/TR/annotation-model/)
supplies useful body/target/selector/state distinctions, not Vellum's byte
coordinate convention or a required RDF/JSON-LD format.
[CodeMirror range mapping](https://codemirror.net/docs/ref/) and
[Neovim extmarks](https://neovim.io/doc/user/api/) inform affinity and deletion/
undo behavior; their display mechanisms alone are not authored-comment storage.

### 3.9 Scope boundary

Scripting runtimes and extension ecosystems are out of scope for v1. Deferred
automation requirements have been removed; typed host APIs remain required.

## 4. Command system

### 4.1 Invocation independence

A command represents a semantic editor action. The following invocation paths
must be able to execute the same registered command without duplicating its
implementation:

- A single key or key chord.
- An Emacs-style `M-x` command prompt.
- A command palette.
- A menu, toolbar, or context menu.
- A mouse or pointer gesture.
- A host application API call.
- An accessibility action.
- An extension or automation.

Input bindings and presentation are adapters over commands, not the source of
editing behavior.

These are supported invocation surfaces, not a requirement for every reference
application to ship every menu/prompt UI. For each exposed surface, equivalent
requests with the same explicit target, state, profile and authority must enter
the same semantic contract. Resolve interactive focus before admission; host
and agent calls must not inherit accidental current-view state.

### 4.2 Command metadata

Registered commands should be discoverable and should provide enough metadata
for frontends to present and invoke them. The model should support:

- A stable identifier.
- A human-readable title and optional description.
- A category and optional aliases.
- Typed or schema-described arguments.
- Availability and enablement based on current editor state.
- Optional default-binding suggestions supplied by frontend/reference profiles,
  which hosts may override; binding resolution and chord state are not engine
  registry state or engine admission preconditions.
- Execution results and user-facing failure information.

### 4.3 Command execution

- Commands must be invocable programmatically as well as interactively.
- Equivalent invocations must have equivalent editor semantics.
- Command execution must validate arguments and current-state preconditions.
- Commands should produce explicit state changes, invalidations, events, and
  external effects rather than reaching into a frontend directly.
- The command model must allow commands contributed outside the core without
  compromising state ownership or thread safety.
- The system must define how synchronous commands, asynchronous commands, and
  commands awaiting host services report progress and completion.
- Discovery/enablement is advisory and may become stale; execution must recheck
  preconditions and current authority. Discovery metadata and failure detail must
  follow disclosure policy rather than reveal inaccessible resources or secrets.
- Distinguish validation, admission, commit and effect completion in outcomes and
  observability. Asynchronous convenience or compound-command composition must
  not silently change atomicity, cancellation or undo guarantees.

### 4.4 Closed operation algebra and fail-fast contracts

- The execution protocol must use a closed, versioned vocabulary of typed
  operations. Plans are bounded declarative requests, not a Turing-complete
  scripting language. Unknown verbs, malformed arguments, unsupported versions,
  and unavailable required capabilities must be rejected before effects occur.
- Extensions must register versioned operation contracts before use through an
  explicit registry/configuration change. Execution must not invent verbs or
  transitions. A registered host implementation is not thereby proven to
  terminate; host work still requires explicit time and resource bounds.
- Each operation must declare preconditions, possible effects, admissible
  outcomes, cancellation semantics, and resource limits. Plan size, fan-out,
  dependency depth, attempts, execution time, and retained state must be bounded.
- Every admitted state/event pair must have an explicit disposition: transition,
  recognized duplicate, late observation, rejected identity, or protocol fault.
  Invalid events must not silently mutate authoritative state or disappear.
- Contract violations must stop the affected execution scope immediately and
  produce a structured invariant/protocol failure. If owner integrity is
  uncertain, isolate or stop that owner. Supervision must not blindly restart
  the same defective operation or reinterpret it as an ordinary transient error.
- Failure must prevent further unissued actions in that scope. It must not
  claim that dispatched effects stopped or were reversed; their outcomes must
  follow the predefined reconciliation and recovery contracts.
- Residual-review direction: expose admission, local publication, per-effect
  evidence, execution control and caller observation as separate typed dimensions
  with explicit admissible combinations, not unconstrained flags or one aggregate
  success bit. Completed observation reports retain their as-observed meaning;
  later evidence is separately accessed under current disclosure policy. Unknown
  is not failure, quiescence is not success, and contradictory evidence is not
  resolved by last-message-wins. Retry requires a declared non-application,
  deduplication, idempotency or conditional-effect safety basis. Retain truthful
  authoritative facts separately from recipient-safe explanations. Project the
  complete report, including top-level categories, reasons, participant details/
  counts, correlations and recovery guidance; protected missing/denied/stale/
  conflicting causes share a safe unavailable explanation where existence is not
  disclosed. Select deterministic bounded reasons after disclosure filtering,
  not by the first internal check to fail; do not probe extra protected state just
  to explain it. Unavailable/withheld is not nonapplication or observed zero.
  Queued delivery rechecks current recipient/policy and its capture ceiling;
  guidance never grants authority or bypasses the actual safe-retry basis.
- Keep local atomic publication within one authoritative owner, with serialized
  final validation/publication and preparation against snapshots outside that
  boundary. Mutual exclusion is a required property, not selection of a mutex
  implementation; do not hold the publication lock while waiting for external
  work. Outstanding effects may retain logical reservations independently.
  V1 does not provide saga execution or durable recovery.
- Define the accepted profile's state/event relation and distinguish normal
  rejection of untrusted input from an internal protocol/invariant fault. The
  totality obligation is explicit handling of every admitted combination, not
  acceptance of unknown behavior or proof of termination of host code.
  Evaluate refinement, boundary tests and STRIDE/MELT coverage against that
  relation; keep any formal finite abstraction and its exclusions explicit.

## 5. Editing model

The editor core must support:

- Insertion, deletion, replacement, and transactional multi-edit operations.
- One or more cursors and selections.
- Character, word, line, page, and document navigation.
- Selection extension and direction.
- Undo and redo with coherent transaction grouping.
- Line-ending awareness.
- Unicode text, including correct handling of UTF-8, grapheme boundaries, and
  display width.
- Indentation and configurable tab behavior.
- Read-only documents or ranges where required by the host.
- Efficient operation on large documents.

Positions and ranges must have unambiguous coordinate systems. Conversions
among byte offsets, Unicode-oriented coordinates, tree-sitter coordinates,
visual columns, and frontend screen coordinates must be explicit.

### 5.1 Agent edit proposals and dependency validation

- Explicit target sets are first-class engine inputs, independent of visible
  cursors and frontend selection state. Human selection-driven actions and agents
  resolve to the same validated batch-edit path. Capture any selection state used
  as an input dependency; later focus/selection changes must not silently retarget
  the operation. No public lens API or new selector capability is implied.
- An agent edit proposal must identify its source snapshots, intended changes,
  expected target content, and the dependencies used to derive those changes.
- Targets must support anchored text ranges and line spans. Optional
  language-aware providers may resolve blocks, syntax nodes, and language
  tokens into ranges. LLM tokenizer tokens are not editor coordinates.
- Target resolution must preserve document identity and be tied to source
  revisions. Line numbers, text matches, or parser node handles alone must not
  be treated as stable identities across edits.
- An exact-revision policy must be available. An explicitly selected mapping
  policy may accommodate intervening changes only after validating target
  identity, expected content, and declared dependencies against the state used
  for commit. Validation and publication must not admit an intervening mutation.
- Ambiguous, missing, stale, or conflicting targets must produce structured
  conflict information. Fuzzy matching must not silently authorize application.
- Dependencies may include read-only context outside the replacement ranges.
  Validation of declared dependencies must not be presented as proof that an
  agent declared every assumption or that its proposed change is correct.
- Support an explicit bounded operation scope using the existing target/range
  and read-only contracts: capture document incarnation, source revision and
  permitted source-write ranges separately from read context/dependencies.
  Preserve original document identity and coordinates; the scope is neither a
  new document owner nor an authorization grant. Current access checks still
  apply independently to reads and writes. Scope-boundary edits/insertion affinity
  must be unambiguous under the declared range profile. Stale, ambiguous or
  exceeded scope rejects the whole publication, not just the crossing edits.
  Exact-revision admission is the baseline; an explicitly selected bounded mapping
  must revalidate scope as well as targets and cannot silently widen it. Other
  authored participants remain subject to their own explicit permissions/limits;
  a source-range scope alone is not permission to mutate annotation bodies or
  dispatch external writes. No visual lens or separate public lens API is required.
- Mapping history and dependency validation must be bounded. Unavailable
  evidence must cause explicit rejection or a request for a fresh proposal.

### 5.2 Cross-document changes and semantic impact

- A logical change plan must be able to describe related edits across multiple
  documents and workspaces in code, notes, or other textual domains, including
  documents not currently displayed and dependencies on documents that will not
  themselves be edited.
- Plans must distinguish their read dependencies from their write targets.
  Preconditions must cover relevant document identities and versions as well as
  any workspace, configuration, or semantic-index state used to compute impact.
- Host integration must distinguish authoritative unsaved document contents
  from persisted file contents. Loading a closed file for validation must use a
  host-supplied snapshot and identity; disk contents must not overwrite an
  existing unsaved document as a side effect of planning.
- Dependency discovery must account for relevant additions, deletions, renames,
  or changes in workspace membership. An unchanged list of previously found
  references is insufficient evidence that no new references exist.
- A semantic rename plan must distinguish symbol identity and binding from
  spelling matches, report the scope and limitations of reference discovery,
  and account for name collisions and unresolved references. Syntax-only
  matching must not be represented as semantic rename verification.
- Semantic analysis may be supplied by a host, language service, compiler, or
  agent. The core must validate the plan's declared preconditions and application
  contract without depending on a particular provider or claiming to prove its
  semantic completeness.
- Plans must declare required atomicity across document edits and any resource
  operations. A host must expose its supported guarantees and reject unsupported
  plans before mutation. Cross-authority saga execution is not a v1 alternative
  to strict local atomicity.
- In-memory application, file persistence, and file create/rename/delete
  operations must have distinct failure and recovery contracts. Atomic editor
  state publication must not imply atomic multi-file disk persistence.
- Changes must retain a shared operation identity for preview, conflict reports,
  telemetry, and history. Previewed plans must be revalidated before application.
  Cross-document undo and redo must have an explicit grouping and conflict policy
  that preserves subsequent unrelated edits or reports a conflict.
- Representative scenarios must cover a declaration and its callers changing
  together, an unchanged write target with a changed read dependency, a newly
  introduced reference, an unsaved dependent document, and host failure during
  application or persistence.

#### 5.2.1 Semantic evidence and discovery-query boundary

V1 needs contracts for supplied evidence, not a built-in semantic discovery
implementation. Applicable providers must state the following before their
results can support stronger operations:

- Identify source object incarnations/versions, discovery membership scope,
  relevant configuration/index/provider generations and provenance. Object
  identity, provider-local symbol identity and inferred equivalence are distinct;
  no universal stable symbol naming scheme is selected here.
- Distinguish exact caller-specified targets, results complete within a declared
  supported discovery scope, and partial/unsupported/unknown results. Define
  query meaning and coverage assumptions; absence of returned edges is not proof
  that no dependencies exist. A spelling search is not semantic rename proof.
- Declare both object read dependencies and dependencies on the discovery scope
  or query context, including relevant additions/deletions/renames. Freshness of
  previously returned objects alone cannot rule out newly matching references.
  Conservative scope invalidation is acceptable; narrow invalidation needs a
  provider contract. A snapshot token proves neither discovery completeness nor
  isolation from uncoordinated external changes.
- Bound query/traversal depth, fan-out, source/result bytes, processing and retained
  snapshots/evidence. Partial or exhausted work cannot silently satisfy an
  operation requiring complete discovery. Refresh/reject, or require a separately
  admitted narrower operation; do not silently downgrade the requested guarantee.
- Revalidate applicable evidence/context and current authority at publication.
  Discovery has no mutation ownership: write participants are explicit; read-only
  dependencies are validation inputs. All plans still enter section 5.3's path.
- Authorize acquisition and release separately. Cross-object edges, existence,
  counts, scope identifiers, result tokens and errors obey sections 15.5–15.6;
  traversing a relationship grants no target access. A recipient-filtered result
  must not imply global completeness or reveal omitted private participants.
- Keep cancellation of pure analysis separate from dispatched external effects.
  Invalidating a derived result may discard eligible cache state, not erase
  authored content or outstanding effect evidence. Attribute work and safe
  failure/saturation along the existing language/search and read-release paths.

These are architectural obligations, not a selected graph schema, protocol,
database, solver or expanded operation vocabulary. Exact types, scope-token
validity and provider-specific completeness/refinement remain R-01/R-02/R-07
specification and assurance work in the formal residual register.

### 5.3 Transaction preparation and atomic publication

- Represent intended source changes as explicit data against identified snapshots.
  Use one canonical source-change semantics for publication, selection/attachment
  mapping, conditional history inverses and derived invalidation. Different typed
  representations may refine that contract; adapters must not implement competing
  mutation or coordinate rules. Do not require total/invertible mappings where
  target deletion or context loss makes them undefined; return explicit disposition.
  Stage all required authoritative participants, including annotation/view/history
  state as applicable, through the same publication boundary. Internal modules
  retain typed state/version/dependency contracts; arbitrary extension callbacks
  must not run inside publication. This does not select a dynamic module runtime.
- Resolve each proposal into per-document change sets against identified
  revisions. Within each change set, edit ranges refer to the starting document
  and use half-open intervals. Sequential edit algorithms must compose their
  results into this representation before commit.
- Reject ambiguous overlaps and same-position ordering unless the caller has
  explicitly resolved them into an unambiguous change set. Validate resulting
  selections and read-only restrictions along with text changes.
- Stage candidate documents, selection mappings, and required history metadata
  before publication. Validate final dependencies, current authority, resource
  limits, and capability requirements at the commit boundary. Stale candidates
  must be rebuilt or rejected; validation alone must not authorize publishing a
  candidate based on a different document state.
- Validation must include non-text state used to prepare the operation, such as
  participating view/selection generations, read-only policy, resource binding
  and configuration. An unchanged text revision is insufficient when those
  dependencies changed. Recompute mappings against current state or reject;
  publication must not restore an obsolete selection merely because text matched.
- Publish a local logical change across managed documents and workspaces as one
  atomic coordinator state transition. Failed validation or staging must leave
  authoritative text, selections, revisions, and history unchanged. Failure
  diagnostics may still be emitted.
- A commit must have one operation identity and one new coordinator generation.
  Each text-changing document receives one new monotonic document revision;
  selection-only updates do not advance document revisions. Undo and redo that
  change text create new revisions rather than restoring old revision IDs.
  Cross-authority operations carry participant versions and local commit IDs;
  no globally atomic snapshot or total revision order is implied.
- Commit metadata must identify the affected documents and their before/after
  revisions. Observers and host effects may receive committed changes only
  after publication. Callback or later effect failure must not turn a committed
  change into a reported validation rejection.
- External dependency tokens must have explicit freshness and validation
  contracts. A locally serialized commit must not claim isolation from
  uncoordinated disk writers or external index changes; where needed, the host
  must provide a reservation or equivalent guarantee, or application must fail.
- Transaction boundaries and undo grouping are distinct. Related typing
  transactions may form an undo group; indivisible cross-document changes must
  retain their shared operation identity for linked history handling.

#### 5.3.1 Git-backed candidate staging — optional evaluation direction

Keep the core representation-independent. Evaluate Git branches and optional
worktrees as a host-backed way to prepare isolated agent candidates and retain
checkpoints, not as mandatory storage for every workspace or live document.

- A candidate must capture authoritative unsaved content and required dependency
  context, not assume repository HEAD is the editor's current state. Revalidate
  current authority, dependencies and budgets before promotion; a clean textual
  merge does not prove semantic correctness or admit publication by itself.
- One conditional authoritative root-reference update can publish an identified
  repository snapshot to readers that pin that root. It is not atomic checkout
  of multiple filesystem paths, atomic publication across repositories, or an
  atomic update of editor state plus a Git reference. Any such bridge needs an
  explicit authority/refinement contract before implementation. Reused commit
  identities do not replace non-confusable publication generations.
- Git reference locks protect reference operations; worktree administrative
  locks are not edit mutexes, and shared repository storage is not a security
  sandbox. Do not infer protection from arbitrary filesystem writers.
- For a workflow whose sole authoritative effect is validated single-root
  promotion, no saga compensation is required. Additional external effects keep
  their own outcome/reconciliation contracts; durable recovery remains deferred.
- Compare storage, checkout, lifecycle and integration costs before claiming
  this cheaper than alternative staging. Branch-based candidate preparation
  does not introduce user-facing history branching or selective undo.

References: [Git worktrees](https://git-scm.com/docs/git-worktree) and
[conditional reference updates](https://git-scm.com/docs/git-update-ref).

### 5.4 Persistence guarantees and failure reporting

- The initial built-in guarantee is atomic in-memory text application across
  documents managed by one coordinator. File persistence remains an explicit
  host effect and must not be reported as complete merely because text committed.
- Save requests and completions must identify the exact document revisions
  persisted. A completion for an older revision must not be relabeled as an
  acknowledgment of a newer one; comparison of current serialized content with
  an acknowledged baseline is a separate claim under section 5.9.
- A host may provide stronger persistence or resource-operation guarantees.
  Required and supported guarantees must distinguish all-or-nothing application,
  crash recovery, and visibility to external readers. Compensating undo after
  partial writes must not be described as atomic persistence.
- Multi-file persistence failures must report per-resource outcomes, including
  unknown outcomes, and retain enough operation information for explicit retry
  or recovery. Automatic reversal must not overwrite subsequent unrelated edits.
- File create, rename, and delete operations are capability-gated host operations;
  they are not implied by support for a symbol rename expressed as text edits.

### 5.5 Scope boundary

Cross-authority orchestration, sagas and durable recovery are out of scope. V1
rejects unsupported cross-authority plans before mutation.

### 5.6 Search and replacement — architectural direction

- Search is a shared, headless-capable service over identified document
  snapshots. Prompts, bindings, host APIs and agents must use the same matching
  and replacement semantics. Parsing and highlighting are not prerequisites.
- V1 requires literal search over one document or explicitly captured selection
  ranges, forward/backward navigation, replace-current and atomic replace-all.
  Query regex syntax, capture substitutions, workspace-wide search, structural
  matching and semantic rename remain outside the required v1 search service.
- Exact case-sensitive matching is the baseline. Explicit caseless matching
  should use a pinned Unicode default case-folding profile with exact mapping
  back to original byte ranges, including length-changing folds. Smart-case,
  locale-specific matching, canonical-equivalence matching and whole-word or
  identifier profiles are optional later capabilities, not implicit heuristics.
- Search must not normalize stored text. Replacement text is literal, preserves
  the supplied casing, and has no executable expansion syntax. Matching uses
  explicit text/line-ending semantics; any logical-newline matching profile must
  be separately named rather than silently rewrite the query or source.
- Query, matching profile, scope, document incarnation/revision and request
  generation identify a result set. Changing any of them supersedes prior work.
  A cleared/empty query has no matches and cannot produce zero-width replace-all.
- Matches must retain exact original byte ranges across storage chunks and case
  folding. Fold expansions must not create partial-source-character matches.
  UI rendering must not silently widen the replacement range to its painted
  glyph bounds. Grapheme navigation and exact substring matching are distinct.
- Choose deterministic leftmost non-overlapping matches in document order.
  Forward/backward navigation traverses that same set. Direction, start position,
  scope and wrap policy must be explicit; wrapping must be observable and must
  not cause an endless search cycle.
- Selection scope is captured independently of later search-result selections.
  Normalize overlapping scope ranges to avoid duplicate edits; never concatenate
  disjoint selected text to manufacture a match. Matches must lie wholly within
  an allowed contiguous scope range.
- Literal document search includes prose, fences and embedded language bodies.
  Language-sensitive restrictions require an explicit future scope/profile;
  spelling matches must not be presented as symbol references or safe refactors.
- Result generation, retained matches, highlighting and execution must be
  bounded and cancellable. Distinguish complete results, partial/truncated
  results, pending work, no matches and exhausted budgets. A displayed partial
  count must not be represented as an exact total.
- Replace-current must validate the selected occurrence against its source
  revision and matching profile. Replace-all must derive one complete,
  non-overlapping change set from one snapshot and pass normal transaction,
  read-only, capacity and revision validation. A stale or incomplete result set
  must be recomputed or rejected; it must not authorize partial replacement.
- V1 replace-all is one local atomic transaction and one undo group. Required
  read-only conflicts reject the whole operation rather than silently skip
  matches. If the full operation exceeds its budget, report that before mutation;
  a future explicitly partial/batched operation would be a different contract.
- Search preview and navigation do not enter text undo history. Cancellation
  may restore a safely mapped origin selection, never an obsolete document
  snapshot or another view's focus. Search text and history are sensitive host
  state and must not enter default telemetry or be persisted without a policy.

The comparison draws on [Helix search selections](https://docs.helix-editor.com/keymap.html),
[Neovim search options](https://neovim.io/doc/user/pattern/), and
[Emacs search/case behavior](https://www.gnu.org/software/emacs/manual/html_node/efaq/Controlling-case-sensitivity.html).
Unicode case folding informs an explicit matching profile; it does not imply
regex support or a complete Unicode regex conformance claim.

### 5.7 Editing laws and assurance obligations

Express editing guarantees as observable laws over identified snapshots and
declared profiles, with independent reference expectations under section 13.4:

- A validated change set determines exact resulting text independently of
  storage chunking and invocation path. Composition and position mappings must
  preserve their stated coordinate domains and insertion/deletion affinities;
  ambiguous or invalid mappings must not be guessed. Shared engine admission
  accepts unambiguous unsorted per-document original-snapshot edits and normalizes
  their order once. Normalization preserves complete edit records, individual
  boundaries, expected-content checks, context and associated semantic metadata;
  it is not silent coalescing, deduplication or identity-record elision. Reject
  unresolved overlap, coincident insertion and insertion/replacement endpoint
  ambiguity rather than infer intent from container or arrival order. Keep the
  work bounded under normal validation/resource contracts; normalized staging
  grants no authority and does not publish partial per-document results.
- An ordinary rejected local transaction leaves authoritative text, selections,
  revisions and history unchanged; a meaningful successful local change publishes
  its required participants coherently. A genuine semantic no-op returns validated
  unchanged success after all required identity, revision, authority, scope,
  capability and bounds checks. Equal source bytes alone are insufficient if
  selection, attachment, annotation, explicit history intent or other declared
  semantic effects change. Determine identity before automatic revision/history
  bookkeeping; unchanged success creates no edit publication generation, source
  revision or undo entry. Invocation-status bookkeeping remains separate and
  bounded. Do not suppress an explicitly requested external effect through text
  equality. No-op detection is bounded and operation-specific, not arbitrary
  program equivalence or whole-workspace comparison. Its predicate must be sound
  and complete for the declared supported profile; unavailable classification
  evidence does not justify an assumed identity or fabricated change. Previously
  dispatched external effects and diagnostic observations are separate state and
  cannot be claimed rolled back by rejection. One incarnation-scoped invocation
  identity binds immutable semantic intent and is admitted for execution at most
  once within its supported bounded admission lifetime. Matching retries observe
  the same pending execution or retained outcome; changed intent rejects. Keep
  execution eligibility, retained status and outstanding obligations separate:
  status eviction/expiry never makes an old invocation fresh, proves nonapplication
  or discharges unresolved work/effects. Retire identities without aliasing late
  work through namespace/counter reuse; cache absence alone is not admission.
  Current recipient policy governs status release, not historical execution facts.
  This requires bounded admission/fencing and bookkeeping contracts, not indefinite
  retention, crash-durable receipts or exactly-once external-effect guarantees.
- Reversal laws require explicit history/precondition scope. Restored text does
  not imply restored revision identities, permissions, clipboard, persisted
  bytes or external state. Section 5.8 establishes linked-frontier history and
  conflict direction; concrete retention rules remain for specification. Do not
  infer selective undo from an inverse text delta.
- Snapshot/dependency validation establishes application safety only for declared
  assumptions and supported host guarantees, not semantic correctness of agent
  reasoning or completeness of impact discovery. Protected-range checks apply
  to all invocation paths, including undo, composition and replacement.
- Cover preparation, admission, validation, publication, history and persistence
  milestones with pathway/interface signals and STRIDE analysis. Bound participant
  count, edit expansion, staging, retained dependencies and failure reporting.
  Critical verification must inspect authoritative outcomes rather than infer
  atomicity from a single success event.

### 5.8 Linked local undo/redo — agreed v1 direction

The agreed v1 policy is resource-scoped linear history with atomic linked groups
across all explicit participants. Reversal is frontier-only: no skipping later
work, hidden cascading undo or partial linked reversal. This covers local text
and authored annotation history, not semantic selective undo, distributed
compensation or durable recovery. Selective reversal remains deferred unless a
subsequent explicit scope decision introduces its stronger dependency contract.

| Policy | Strength | Tradeoff |
|---|---|---|
| One authority-wide undo stack | Simple total order and linked reversal | Unrelated work in other documents blocks or is undone before the intended local change |
| Independent histories with best-effort partial undo | Locally convenient | Can split an indivisible change, silently breaking its application guarantee; not acceptable for linked atomic operations |
| Scoped histories with linked frontier validation — recommended | Independent resources can progress; linked changes reverse atomically or conflict | Later work in any participant can block reversal, even if its text ranges are disjoint |
| Dependency-aware selective reversal | Can retain later work while reversing an older operation | Needs edit/dependency transformation, semantic limitations and stronger evidence; remains deferred |

- Keep authoritative history with the coordinator/documents, not views or author
  sessions. Use explicit history scopes: source-text history for a document and
  authored-content history for an annotation record. Pure navigation, focus,
  telemetry, parse/layout refresh and automatic annotation-anchor mapping do not
  become authored-content history entries. Annotation collection/record identity
  and deletion evidence must support bounded history retention without ID reuse.
- Each admitted edit records its logical operation and explicit history group.
  A linked operation/group names every text or annotation-content participant;
  grouping never silently splits its atomic participants. Ordinary typing may
  coalesce only under a documented bounded policy with explicit boundaries.
  Do not merge unrelated authors, source/comment histories or linked agent plans
  merely because they arrived close together. Exact grouping rules are later
  specification, not an implicit frontend timer heuristic.
- Resolve interactive undo to an explicit source document or selected annotation
  scope. Host/agent calls name that scope or a retained group; they do not inherit
  current focus. Discover the complete linked participant set before admission.
  Source undo must not silently target whichever comment was edited most recently.
- For v1, a group is undoable only when it is at the current undo frontier of
  every participant, with its required inverse data retained. Redo similarly
  requires the matching redo frontier across all participants. Validate current
  incarnations, expected contents, group/frontier versions, current rights and
  reversal-specific dependencies, then publish one atomic local transaction.
  Missing/blocked participants or rejected permissions cause no partial reversal.
- Eligibility follows validated history position and current preconditions, not
  equality with the original numeric text revision: undoing later work creates
  fresh revisions. Every accepted undo/redo has a new operation identity linked
  to the original group and advances changed text/annotation versions. It changes
  history position rather than appearing as another ordinary edit to undo forever.
- Example: G changes source documents A and B; H later changes B. Undoing G from
  A reports that B blocks it and leaves both unchanged. Undo H explicitly, then
  G may become eligible. A later edit to unrelated C does not block G. V1 does
  not skip H even when its range is disjoint, and must not auto-undo H as a
  hidden prerequisite. Reports must not disclose unauthorized participant detail.
- Later comments on A/B do not block source undo merely because their anchors
  map over that text. Preserve comment bodies and authored reattachments; update
  attachment evidence or detach safely under section 3.8. If G explicitly created
  or edited a comment as an atomic participant and that comment later changed,
  the linked undo is blocked until the relevant authored history is eligible.
- New authored work after undo invalidates the affected redo continuation.
  Invalidate linked redo as one unit across its participants and any dependent
  continuation; no surviving handle may redo only half the group. Independent
  scopes retain their histories. Branch selection/time-travel UI is deferred;
  never pretend an invalidated continuation is still replayable.
- Undo restores authored content under this contract, not historical authority,
  resource binding, other views' old selections, persisted bytes or external
  effects. Other live selections use normal mapping; any invoking-view caret
  restoration must validate that view/context and must not steal unrelated focus.
  Save/clipboard/tool effects are not replayed by redo. Forward semantic correctness
  or newly introduced symbol dependencies are not proved by a successful textual
  inverse; stronger reversal guarantees need explicit dependencies or rejection.
- Bound inverse data, groups, linked fan-out and retention. Eviction/disposal
  must preserve consistent history floors and links: invalidate dependent undo/
  redo eligibility explicitly, or refuse the lifecycle action if its promises
  require retention. Do not create dangling partial groups, pin memory forever,
  implicitly reload a participant or discard unsaved source/comments to enable
  undo. History unavailability is distinct from empty history or a conflict.
- Derive independent scenarios for linked blocking, fresh-revision round trips,
  comment preservation, changed rights, participant disposal, redo invalidation
  and retention boundaries. Model atomic frontier movement and content updates
  as one transition, separately from source persistence. Expose eligible/blocked/
  unavailable outcomes and content-safe conflict/latency/saturation signals;
  history IDs and provenance are not authority to reverse another resource.

References: [Emacs atomic change groups](https://www.gnu.org/software/emacs/manual/html_node/elisp/Atomic-Changes.html)
distinguish multi-buffer grouping from undo amalgamation;
[Neovim undo blocks and branches](https://neovim.io/doc/user/undo.html),
[Helix checkpoints/history](https://docs.helix-editor.com/commands.html) and
[CodeMirror history isolation](https://codemirror.net/docs/ref/) inform grouping
and explicit boundaries. They do not establish Vellum's linked-frontier guarantees.

### 5.9 Text/resource fidelity and saved state — agreed v1 direction

The agreed direction is to preserve accepted content by default, make conversions explicit,
and keep serialized-content equality, external freshness and persistence evidence
separate. Concrete profiles, types and acceptance evidence remain for specification.

| Policy | Strength | Tradeoff |
|---|---|---|
| Normalize to one encoding/newline convention on open or save | Simple internal/output policy | Can silently change untouched files and invalidate byte fidelity or annotation context |
| Preserve valid text and explicit serialization metadata — recommended | Predictable round trips without turning the editor into a binary editor | Requires BOM/profile metadata, line-ending-aware edits and explicit conversion failures |
| Preserve arbitrary bytes with text projections | Supports malformed and binary resources losslessly | Needs a different storage/coordinate/edit contract; not required for v1 |

- The required v1 file profile is strict UTF-8, including an explicit reversible
  policy for a leading UTF-8 BOM. Preserve the loaded BOM choice; do not treat a
  consumed signature as an ordinary text character for edit offsets. No implicit
  Unicode normalization or replacement-character decoding. Other encodings are
  optional host profiles, not a requirement for automatic detection, legacy
  codec distribution or arbitrary binary editing.
- Decode into a staged candidate with identified resource/version and profile.
  Invalid, unsupported or ambiguous decoding must fail without replacing live
  content, rebinding an existing document or destroying its history/annotations.
  Explicitly requested lossy import must be a separately labeled conversion,
  not a lossless open that silently permits overwriting original bytes.
- Preserve source newline sequences, including mixed LF/CRLF, lone CR and the
  presence/absence of a final newline. Do not trim whitespace, append a final
  newline or normalize existing endings as a save side effect. Record an explicit
  document insertion policy for newly generated line breaks; use LF for a new
  document unless the host supplies another policy. Literal insertion/paste
  remains literal unless a separate declared transformation applies.
- Newline conversion, Unicode normalization and other content cleanup are
  explicit validated text operations, with fresh revisions, mapped selections/
  annotations and normal history. Output encoding/BOM changes have separately
  versioned serialization settings; changing those settings may require saving
  without changing text revision. They must not masquerade as unchanged output.
- Internal UTF-8 edit coordinates describe the accepted document text, not raw
  file-byte offsets after BOM/encoding conversion. External byte ranges or
  persisted annotation selectors require an explicit profile/version mapping;
  do not infer equality between parser, document and serialized byte positions.
- For each lossless profile p, specify round-trip laws: encoding the decoded
  unchanged bytes with the captured metadata reproduces those bytes, and decoding
  an encodable text snapshot reproduces its text under that profile. Optional
  codecs that cannot satisfy a claimed law must narrow their contract or reject;
  unrepresentable output must not silently substitute, drop characters or switch
  encoding. These are specification obligations, not verified implementation claims.
- A save captures immutable text, serialization profile/settings generation,
  document incarnation/revision, binding/destination generation, baseline token
  and required acknowledgment strength. Prepare/validate encoding before issuing
  a destructive write. Later edits or setting changes cannot alter the captured
  payload; completion evidence names what was actually written and acknowledged.
- Keep a source baseline for the current binding: loaded/acknowledged serialized
  content, relevant profile and host resource token. Compare current intended
  output against it to report unchanged, modified, unbound or not-yet-established
  equality. Undo to equal serialized content may report unchanged despite a fresh
  revision. An older save receipt still names its original revision, never the
  newer revision whose output happens to match. Bound comparison/retention cost;
  digest equality alone must not be presented as exact byte equality without a
  declared assurance/collision policy.
- Content equality is not knowledge of current external contents. Track baseline
  comparison, external freshness/conflict, pending/failed/unknown save outcomes
  and acknowledgment/durability strength separately. A stale completion may
  update historical evidence but not the saved baseline of another binding.
  Matching a loaded baseline is not proof of a crash-durable write.
- Annotation authored state and serialized attachment/source-version evidence
  have their own baseline and persistence outcomes. Source edits may therefore
  require annotation persistence even when comment bodies did not change. A
  successful source save must not clear pending annotation changes. A combined
  save reports per-artifact outcomes and only claims joint atomic persistence
  when an explicit host capability supplies it.
- Known external changes or unavailable required baseline evidence block normal
  overwrite. Distinguish best-effort conflict detection from host-supported
  conditional-write guarantees: check-then-write and atomic replacement alone
  do not prove isolation from uncoordinated writers. Advertise the actual profile
  and reject a stronger requested guarantee when unavailable. Any authorized
  force-overwrite is a distinct explicit action, not an automatic retry strategy.
- The host write contract must state replacement visibility, failure/unknown
  behavior, durability acknowledgment and relevant resource-attribute/security
  preservation. No universal fsync/rename or metadata-preservation guarantee is
  implied by a text library. Encoding failure before dispatch is distinct from
  partial external write after dispatch; neither may clear unsaved authored state.
- Residual-review baseline: ordinary filesystem integration uses editor-like
  best-effort external-change detection, not mandatory universal atomic conditional
  writes. Capture content and relevant baseline evidence, check before saving,
  reject detected conflicts and preserve unsaved content on failure. Expose
  structured agent outcomes and explicit reload/comparison/save-elsewhere or
  authorized force-overwrite paths where supported; do not rely on a human prompt.
  All engine-mediated writers use the existing single-owner/serialized path;
  arbitrary external writers remain outside that coordination domain. Stronger
  protection is an optional advertised provider capability; reject a request
  requiring it when unavailable, without making normal file saving unsupported.
  HTTP examples are supporting semantics references only, not a transport or
  storage requirement. File watching and automatic external merging stay deferred.
- Verify with independent fixtures for BOM, mixed endings, no final newline,
  invalid input, unrepresentable output, undo-to-baseline, changed serialization
  settings, editing during save, external conflict, and source/annotation partial
  save. Apply STRIDE at decoder, serializer and resource interfaces; observe bytes/
  queueing/outcomes through bounded metrics and safe codes, never source payloads.

Residual-review acknowledgment direction: ordinary save completes the provider's
documented write/synchronization procedure, with synchronization enabled by default
where supported. Actual acknowledgment strength is explicit; no universal crash
durability or recovery journal is implied. A stronger requested strength cannot
silently degrade. If writing succeeds but required synchronization fails, retain
both observations and report the unsatisfied guarantee rather than claiming no
write occurred or blindly replaying it. Source and separately stored annotations
retain independent outcomes/baselines. Ordinary presentation stays simple while
structured receipts retain exact snapshot/destination/procedure evidence.

References: [Helix encoding and line-ending commands](https://docs.helix-editor.com/commands.html),
[Neovim encoding/BOM/end-of-line options](https://neovim.io/doc/user/options/) and
[Emacs output coding](https://www.gnu.org/s/emacs/manual/html_node/emacs/Output-Coding.html)
show why representation and conversion choices need explicit contracts. Vellum
does not adopt their normalization defaults or imply binary editing. Filesystem
replacement behavior is host/platform-specific, as reflected in
[Rust's rename contract](https://doc.rust-lang.org/std/fs/fn.rename.html).

### 5.10 Concurrent save evidence and coordination — agreed v1 direction

The following evidence constraints refine sections 3.7, 5.4 and 5.9; they do not
select an implementation or require overlapping writes. The residual review
selects single-flight per host-resolved destination, bounded FIFO of exact
admitted captures, explicit capacity/expiry outcomes and no implicit coalescing.
Numeric capacities, deadlines, shutdown treatment and provider guarantees remain
for specification:

- Keep capture/request order, text revision, serialization generation, actual
  host write order and receipt delivery order distinct. Baseline selection must
  not infer storage order from callback order or maximum text revision alone;
  output-profile changes need not advance text revision.
- Declare token capabilities separately: equality validation, atomic conditional
  write, non-reuse within an identity/retention epoch and attested write order.
  Opaque equality tokens are not ordered counters. A reused content validator
  must not be treated as proof that no intervening write occurred. Causal
  token-chain inference requires the corresponding non-ABA/provider contract.
- Retain exact acknowledged observations and unresolved attempts. If relevant
  ordering evidence is unavailable, do not fabricate a unique latest outcome.
  A latest known acknowledgment is still not proof of current external contents,
  absence of unknown writes or crash durability. An internal antichain of maximal
  acknowledged candidates is one reference construction, not a mandated API type.
- Stopping a caller does not release an unresolved resource-write obligation.
  A subsequent dispatch, rebind or retry must honor the selected coordination
  contract and actual safe-disposition evidence; no implicit force overwrite,
  token-refresh retry or relabeling of an older saved revision is introduced.

Working direction for consecutive queued saves, accepted provisionally in
residual review: preserve immutable captured content/profile and original baseline
evidence, while permitting explicitly recorded expected-baseline progression
through attributable preceding writes in the same destination/binding sequence.
Do not overwrite the original capture or mislabel a derived dispatch condition
as the caller's original evidence. Current external-conflict checks still apply
under the actual provider profile; unknown/contradictory outcomes cannot justify
progression, and explicitly pinned caller preconditions cannot silently relax.
Write-success/synchronization-failure remains separate evidence; further dispatch
requires safe disposition and satisfiable subsequent preconditions, not a generic
success flag. Exact receipt/condition schemas, causal validity and multi-save
provider refinement remain R-01/R-04/R-07 work; no blind token refresh is selected.

Coordination alternatives and selected direction:

| Direction | Benefit | Tradeoff / recommendation |
|---|---|---|
| Single-flight per host-resolved resource identity — agreed v1 | Smallest local overlap and unknown-outcome state space | May block behind unknown writes; independent destinations and local editing can continue |
| Overlapping conditional writes — not selected for base v1 | Uses capable providers' concurrency | Requires explicit conflict/winner and ordering contracts; no implied overlap capability |
| Single-flight with coalesced undispatched work — deferred | Reduces burst/agent write traffic | Requires explicit superseded-versus-persisted outcomes; never a silent queue optimization |

Single-flight alone does not choose busy versus pending admission, queue order,
wait/retention bounds or supersession. It is local resource coordination, not a
global editing lock, external-writer isolation or another editing mode. The
conditional guarantee still requires a suitable host capability. Distinct
serialization profiles or workspace labels must not split coordination for the
same resolved destination.

The [admission comparison](formal/save-admission.md) makes the next choice
explicit: reject Busy before admission, or retain bounded per-resource FIFO work
with exact admitted captures. Bounded FIFO is now agreed for admitted v1 saves;
capacity exhaustion rejects explicitly before admission. Busy remains a modeled
comparison, not the selected ordinary admission policy. Queue order does not imply fairness, current
authorization, external isolation or eventual resolution. Coalescing remains a
separate supersession decision, not a silent queue optimization.

The comparison also checks a candidate deadline/reservation profile: expiry of
never-dispatched queued work ends it explicitly and releases its work reservation;
expiry after dispatch stops caller control but preserves unresolved obligations.
Admission can conservatively reserve bounded aggregate work and future completion
capacity together. Staged reservation is an alternative requiring its own progress
contract. These checks do not choose actual capacities, clock/epoch/expiry rules,
shutdown policy or authoritative-receipt retention. Releasing an outstanding-work
reservation does not imply that retained evidence/history consumes no storage.

The [dispatch/evidence gate slice](formal/dispatch-gates.md) separates final local
launch authority/binding/provider-context checks from historical evidence
acceptance. A frozen source snapshot does not freeze permission; conversely, a
later source edit alone must not relabel an explicitly captured older save as a
request to save the latest text. Current caller revocation or binding invalidation
does not by itself invalidate a correctly authenticated, exactly matched
historical observation. Retaining that observation grants neither public release
nor current-baseline attribution, rebinding permission or new write authority.
Provenance and effect semantics are separate: authenticated progress/cancel intent
cannot discharge an unknown write; safe disposition requires the actual provider
guarantee. The finite receiver checks do not establish cryptographic validation,
provider truth, full declared-dependency discovery or revocation of an already
launched physical effect.

The [restricted save composition](formal/save-composition.md) now checks actual
admission/gate transitions together. Launch rejection removes the undispatched
reservation without relabeling the public rejection; only accepted safe
disposition releases an unresolved effect. Safe disposition and successful-save
acknowledgment remain distinct typed outcomes: quiescence alone is not a save.
Under the selected single-flight direction, local dispatch ordering can prevent
an old duplicate receipt from regressing the latest eligible known acknowledgment;
this does not promise current external contents or apply to arbitrary overlap.
Historical retention, current-binding baseline eligibility and current recipient
release authority remain separate gates. Direction was agreed by user review,
not by the bounded composition; that composition is not proof of concrete scheduling or complete lifecycle/provider/disclosure
refinement.

The lifecycle composition now also covers queued cancellation/expiry refunds,
active expiry retaining unknown obligations, admission closure and a wait-end
report that does not imply quiescence. Late evidence may reconcile after that
report. The [execution correspondence](formal/save-execution.md) reuses actual
kernel steps: budget/protocol termination must clean up undispatched work or stop
active caller control without refunding an unresolved effect. The internal
kernel now carries a neutral quiescence receipt fact rather than inventing a
success/failure grade; this does not select a public API schema.
Numeric deadlines, complete shutdown policy and service/preemption guarantees
remain outside these finite-prefix claims.

The [abstract storage bridge](formal/storage-bridge.md) derives completed-result
grades from the existing host compare/write model, not caller-provided success.
Contradictory positive/negative grades remain retained evidence and disqualify
that attempt as an uncontested successful baseline, including after duplicate
success. The [recipient bridge](formal/recipient-bridge.md) connects actual
storage/disclosure transitions for a fixed source-and-permitted-annotations view;
retained save evidence is not automatically a permitted field. These strengthen
bounded design evidence without choosing universal field permissions, actual
provider mechanisms or implementation/clean-room assurance claims.

[Concurrent-save contracts and evidence](formal/concurrent-saves.md) record the
bounded policy comparison and residuals. [RFC 9110 entity tags](https://www.rfc-editor.org/rfc/rfc9110.html#section-8.8.3)
and [If-Match](https://www.rfc-editor.org/rfc/rfc9110.html#section-13.1.1) illustrate
equality/conditional-write semantics, not a mandatory HTTP transport or a claim
of unique per-write versions. The editor-specific recommendation remains a
design proposal rather than a requirement derived verbatim from the RFC.

## 6. Tree-sitter and language support

- The supported language integration must provide incremental tree-sitter parsing;
  the authoritative core and ordinary editing must work without that integration.
- Edits supplied to tree-sitter must correspond exactly to committed document
  transactions.
- Parse results must be associated with a document revision.
- Stale parse and highlight results must never replace newer state.
- Language grammars and highlight queries must be pluggable rather than compiled
  into the editor core.
- Syntax highlighting should update incrementally and be limited to visible or
  affected regions where practical.
- The architecture should leave room for syntax-aware selection, navigation,
  indentation, folding, and structural editing.
- Parse errors and incomplete trees must not prevent ordinary text editing.

### 6.1 Language-service lifecycle — architectural direction

- Language support is an optional derived-state service over immutable document
  snapshots. It must not own document mutation or require a parser per view.
  Distinct analysis contexts may need separate parser state; views with the
  same context should reuse accepted results. Structural/schema providers for
  prose and code follow the same optional-provider boundary in section 3.4.1;
  required reference language integration remains required outside the core.
- A mutable parser and its incremental working state must have one executor
  owner at a time. Published parse snapshots must not be mutated when preparing
  later parses. Concrete parser scheduling and tree ownership types remain for
  the specification pass.
- Identify results by document incarnation and revision, analysis context,
  grammar generation, query generation and relevant configuration. A matching
  text revision alone does not establish freshness after language reconfiguration.
- Incremental parsing must consume committed edit transformations consistent
  with the parser's starting snapshot. Coalescing must preserve their composed
  meaning; if necessary transformation history is unavailable, rebuild from an
  identified snapshot under a declared budget rather than guess missing edits.
- Grammar compatibility and query compatibility must be checked before
  activation. Query updates should not force a text reparse when the grammar and
  tree remain valid; grammar replacement invalidates dependent parser results.
  Failed activation must leave a declared last-known-good configuration or
  explicit plain-text/degraded state, never a partially installed combination.
- Parse trees and node handles are revision/context-bound derived values, not
  permanent document or semantic symbol identities. Structural requests must
  validate freshness and may report pending or unavailable analysis.
- Use bounded, coalesced background parsing and demand-driven highlighting for
  visible/affected regions. Viewport queries may require larger syntax context;
  visible highlighting must not imply that parsing only those bytes is correct.
  Syntax-tree change ranges alone are not a complete highlight invalidation rule.
- Report missing grammar, invalid query, incomplete analysis, exhausted budget,
  cancellation and worker failure distinctly. Expected malformed source remains
  ordinary editable input, not an operation-algebra invariant violation.
- V1 language support must accommodate bounded embedded regions under section
  6.2. Grammar installation/build tooling, broad language distribution, arbitrary
  recursive injection support and LSP remain later capabilities. Region depth,
  parser count, included regions and total work must be bounded.
- Grammar/native scanner loading is a host-controlled code-loading boundary.
  No implicit downloads or execution of document-specified grammar code. Trusted
  in-process native grammars are not a sandbox; stronger crash containment needs
  an explicit host isolation capability.
- Parser output is derived evidence, not permission to mutate or a proof of
  program meaning. Language-sensitive proposals must retain exact source/context
  mappings and enter the ordinary validation path. Compare incremental results
  with fresh parsing under the same profile in later verification; this checks
  incremental consistency, not the grammar's semantic completeness.
- Attribute grammar/query activation, work admission, region fan-out, result
  acceptance and degraded fallback along the language-service pathway. Bound
  region/query work before scheduling and expose aggregate saturation; missing
  or truncated analysis cannot be reported as a complete semantic result.

### 6.2 Mixed-language documents and embedded regions

- Single-object ownership governs the document regardless of natural language,
  markup, embedded grammar or semantic interpretation. An English document with
  a German section, or Markdown with Mermaid, does not acquire additional owners
  or authorities at those boundaries. Region constraints govern a service's
  interpretation and declared write set, not ownership partitions or independent
  mutation paths. Explicit rights restrictions, where present, are a separate
  authorization concern and are not inferred from language labels.
- Tangling, export or semantic links do not themselves transfer ownership.
  Producing or mutating another authoritative object must explicitly identify
  its owner/authority, target identity and required effects. That is an additional
  participant or effect boundary under the normal transaction/host contracts;
  crossing a grammar boundary alone is not such a boundary.
- One authoritative document may contain a host language and multiple embedded
  language regions. Region parsers and highlights are derived views of the same
  revision, not independently editable copies or separate undo histories.
- V1 must cover Markdown with fenced code/diagram blocks, including Mermaid
  fences. Recognize embedded language labels through a host-controlled registry
  and explicit aliases. Labels are data, not executable commands, download
  instructions, library paths, or authorization grants.
- Discover regions from the host-language structure and registered bounded
  rules. Parse each independent fenced block independently by default; identical
  language labels must not silently concatenate separate blocks into one program.
  General combined/discontinuous injections remain a later explicit capability.
- Embedded analysis must retain an exact mapping to authoritative document byte
  ranges. Included ranges or projected snapshots must preserve mapping for all
  returned highlights, positions and proposed edits. Fence delimiters remain
  owned by the host language; the boundary and crossing-range policies must be
  defined before exposing language-sensitive commands.
- Changes to fences, language labels, host syntax or injection configuration may
  change region membership without changing most body text. Invalidate affected
  region mappings, child parsers and results accordingly. A highlight-only query
  update and a region-discovery query update have different dependencies.
- Define deterministic host/embedded highlighting precedence. Missing grammar,
  unsupported labels, incomplete fences and per-region budget exhaustion must
  have explicit fallback behavior. One failed embedded region must not disable
  unrelated regions, host-language styling or ordinary text editing.
- Bound total region count, nesting, parser instances and analysis work per
  document as well as per region. Repeated tiny fences must not bypass document
  resource limits. Concrete bounds belong to the specification pass.
- The reference scenarios must cover Markdown prose with multiple independent
  code fences and Mermaid content, unavailable embedded grammars, fence edits,
  language-label changes, mixed-language transaction ranges and stale results.
  At least one supported embedded grammar must demonstrate actual nested
  parsing/highlighting; fallback alone is not the mixed-language capability.
- Parsing/highlighting Mermaid source does not require rendering a diagram.
  Diagram previews, script execution, external renderers and network retrieval
  remain separate optional capabilities outside v1's required language service.

Relevant references are tree-sitter's
[language injections](https://tree-sitter.github.io/tree-sitter/3-syntax-highlighting.html#language-injection),
[Helix injection queries](https://docs.helix-editor.com/master/guides/injection.html), and
[Neovim language trees](https://neovim.io/doc/user/treesitter/).

## 7. Asynchronous and multithreaded operation

### 7.1 Execution model

- Latency-sensitive editor mutations must remain synchronous, deterministic,
  and bounded, with no waiting for host I/O or worker completion in publication.
- Expensive or independent computation should execute asynchronously or on
  worker threads when doing so improves responsiveness.
- The core must not require a specific executor such as Tokio or Rayon.
- A host must be able to run effects with its chosen executor and return typed
  completions to the editor.
- A synchronous executor must be possible for deterministic tests and simple
  embeddings.

### 7.2 Background work

Suitable background work includes:

- Incremental parsing and syntax highlighting.
- Search and replace-all discovery.
- Diagnostics, completions, and language-server requests.
- Formatting and code actions.
- Large-file indexing and symbol extraction.
- File loading, saving, and autosave.
- Outline and minimap generation.
- Expensive wrapping or layout calculations where safe and useful.

This list includes deferred integrations; only section 1.1/1.2 capabilities are
required in v1. Moving a capability to a worker does not add it to initial scope.

### 7.3 Correctness and scheduling

- Every state-dependent task must carry the relevant source identities and
  freshness context: document incarnation/revision when it depends on text,
  plus view, configuration, binding or other dependency generations as needed.
- The editor must reject or safely rebase stale results.
- Work must support cancellation or supersession where practical.
- Rapid edits should coalesce redundant background work.
- Work queues must be bounded to prevent unbounded memory growth.
- Scheduling should distinguish latency-sensitive work from speculative or
  background work.
- Section 7.5 establishes agreed host-assigned work classes, protected control/completion
  capacity and bounded fair admission; bounded queue size alone is not a fairness
  or progress guarantee.
- Recoverable worker failures must be scoped and surfaced without publishing
  invalid candidates. A panic or native-code failure does not imply safe reuse
  of mutable worker state or guaranteed process survival; containment depends on
  the declared host isolation boundary under sections 4.4 and 12.4.
- Shutdown must cancel or drain work predictably without hanging the host.

### 7.4 Effect state, evidence and supervision

- Core operation decisions should be deterministic transitions from state and
  explicit events to next state, requested effects, and reported outcomes.
  Time, authorization changes, worker failures, and completions are inputs;
  transition logic must not perform hidden I/O or invoke arbitrary host callbacks.
- Use typed effect requests and completions independent of the host executor.
  Requests must carry effect and originating operation identities, required
  capabilities, relevant document/view incarnations and dependency revisions,
  and typed parameters. Completions must identify their effect and attempt.
- Keep lifecycle, cancellation intent, outcome evidence, result applicability,
  and attempt identity distinct. Timeout is a deadline observation, not proof of
  failure; cancellation is a request, not proof that effects stopped.
- Accumulate authenticated outcome evidence without erasing prior facts.
  Evidence combination must be associative, commutative, and idempotent for
  observations whose order is irrelevant. Conflicting authoritative receipts
  must produce an explicit protocol fault or reconciliation requirement, not a
  last-message-wins result.
- A bounded lattice may represent a defined evidence domain; it must not be
  assumed to describe all editor or business states. Finite control variants
  do not make payloads, identifiers, histories, or the whole system finite.
  Define resource bounds and any finite verification abstraction separately.
- Validate completion identity and applicability before changing editor state.
  Stale parse/layout results may be discarded, but completed external writes
  remain outcome evidence even when their source document revision is old.
- Define retry policy per effect. Disposable computations may be recomputed;
  mutation retries require known-safe semantics or deduplication. Unknown write
  outcomes require reconciliation before incompatible retry or compensation.
  Assign one retry owner, with bounded attempts and explicit exhaustion results.
- Serialize saves to the same destination by default so an older write cannot
  finish after and overwrite a newer write issued by this host. Completion
  revision tracking alone does not enforce write order.
- Bound effect and completion capacity, coalesce replaceable work, and reject
  or defer admission explicitly under pressure. Reserve necessary capacity
  before committing actions that promise subsequent effect submission.
- Supervision must bound restarts and escalation and distinguish worker health
  from operation outcome. Replacing a worker must not give obsolete attempts
  authority to commit or authorize unsafe repetition of an uncertain effect.
- Shutdown must stop admission and apply an explicit, time-bounded cancellation,
  drain, and outcome-reporting policy. V1 requires deterministic in-process
  handling and controllable tests, not a durable journal or distributed runtime.
- Scheduling must preserve the operation/dependency contract, not a presumed
  deterministic arrival order. Given an admitted event sequence, transitions
  must be reproducible; different allowed schedules may produce different valid
  outcomes. Model both permitted interleavings and progress assumptions.
- A deadline bounds how long the coordinator waits or admits work; it does not
  prove that arbitrary trusted Rust/native code can be preempted or reclaimed.
  Cooperative cancellation and external isolation have different guarantees.
  Track queue wait, execution, attempt outcome and late evidence separately,
  preserving bounded causal correlation across executor handoffs.

### 7.5 Admission, fairness and shared-resource interference — agreed v1 direction

Agreed direction: combine hard aggregate bounds, host-established work/admission
classes, protected control/completion capacity and explicit fair-sharing policy
for admitted work. V1 specifies obligations at the core/host boundary, not a
particular executor, scheduling algorithm, distributed quota service or sandbox.

Residual-review resource-envelope direction is agreed alongside the scheduling
contract: soft degradation for optional work, hard admission
limits for promised atomic work, and explicit consistent history/retention floors.
Optional analysis/presentation may degrade without silently weakening a requested
transformation's completeness or atomicity. If the required transaction cannot
fit its resource contract, reject or explicitly boundedly defer before admission;
never partially apply it. Existing worker-held snapshots, history promises and
unresolved-effect evidence remain actual obligations until safely released under
their lifecycle contracts. Neither pressure nor a cancellation request erases
them. Numeric profiles are chosen/tuned using section 12.5.1's evidence;
numeric reservations/shares and concrete scheduling/eviction algorithms remain
open, not the agreed obligation to protect capacity and prevent starvation.
Every enabled operation requires a complete validated finite resource profile;
missing or nonfinite coordinates are not unlimited. Host-supplied profiles and
evidence-backed reference defaults must declare units/domains and applicable
ceilings. New atomic proposals that cannot fit reject before mutation rather than
enter a generic implicit queue; explicitly specified bounded queues, including
the save FIFO, retain their contracts. Adaptive/soft targets cannot expand hard
limits or grants. Reject proposed capacity-profile changes incompatible with
current charges, protected reservations or applicable commitments, preserving the
old profile and obligations rather than clamping, canceling or resetting them.
Planned draining and actual clock/service profiles require their separate explicit
contracts; numerical defaults require workload/host/accounting evidence, not
unmeasured universal presets.

The agreed pressure-policy order is obligation-aware: shed stale/replaceable
derived work, reduce optional diagnostics/presentation, defer or explicitly
truncate eligible optional analysis, evict eligible cache/history under consistent
floors, and reject/boundedly defer new work that cannot fit. Target the actual
constrained resource rather than mechanically exhausting each preceding step.
Never claim partial analysis complete, split atomic work, discard unsaved authored
state, silently revoke accepted history promises, or refund still-live snapshots/
effects on cancel intent. Pending work ends with an explicit applicable outcome;
overload is not an internal invariant fault. Human/agent origin alone determines
neither expendability nor privilege. This selects shedding semantics, not the
specific scheduler algorithm or numeric thresholds.

The protected-capacity/cost-aware-sharing contract is now agreed by user review.
Bounded reservations protect admitted completion, reconciliation, cancellation,
revocation, shutdown and minimal health obligations from speculative traffic.
Validate identity/attempt/payload before access to protected service. Share other
available service by declared bounded cost and originating scope, not request
count or human/agent origin alone. Child work/retries/aliases cannot create quota.
Eligible admitted work must not starve under declared host service and bounded/
cooperative execution; this is not a promise to admit all traffic or preempt
arbitrary native code. Keep required reservations conservatively unavailable to
borrowing unless safe reclamation/non-interference is established. Observe queue
age, actual service, useful completion and reservation pressure together; numeric
shares/quanta/latency objectives and algorithm selection require workload evidence.

| Policy | Strength | Tradeoff |
|---|---|---|
| One bounded FIFO | Simple memory bound and arrival order | A burst or expensive head item can monopolize capacity and delay control work |
| Strict class priority | Fast service for preferred work | Can indefinitely starve lower classes; caller-controlled priority is an escalation path |
| Class-aware admission with protected capacity and bounded sharing — agreed direction | Preserves safety/progress handling while limiting interference | Requires cost/scope accounting, numeric service profile and explicit host scheduling assumptions; no particular algorithm selected |

- The host assigns permitted work classes from validated operation type, access
  context and required guarantees; untrusted labels, author claims, workspace
  names or trace metadata cannot grant priority. Human and agent origin alone
  does not determine privilege or service class. Bound active contexts and retain
  parent/delegation budget attribution so creating tasks/documents/aliases cannot
  manufacture new quota without a separately authorized grant.
- Separate safety/control work, authorized latency-sensitive editing/presentation,
  derived/background work and optional diagnostics where their obligations differ.
  This is a semantic classification, not a required fixed set of queue enums.
  Reserve bounded admission/processing capacity for accepted effect completions,
  cancellation/revocation/shutdown handling and minimal health/loss accounting.
  A flood of new speculative work must not consume those reservations.
- Protected capacity is not an unrestricted priority lane: validate identity,
  attempt and payload bounds before accepting a completion/control event into
  it. Reject/coalesce bounded recognized duplicates where semantics permit;
  malformed or unrelated events cannot impersonate outstanding effects. Distinct
  authoritative observations must still reach evidence reconciliation.
- Before committing an operation that promises effects, account for the bounded
  outstanding requests, expected completion/evidence handling and required history/
  staging resources it creates. Retire reservations only when their obligations
  are discharged or transferred under an explicit unknown-outcome policy; a
  cancellation request is not proof that a worker or write stopped consuming
  resources. No optional diagnostic record needs such an authoritative reservation.
- Admission considers bytes/retained snapshots, participant fan-out, edit expansion,
  staging/history/annotation cost, outstanding effects and bounded processing work,
  not request count alone. Shared objects need defined accounting so neither
  double-counting nor alias-based evasion silently defeats the budget. Estimates
  must have explicit bounds or staged budget acquisition; if full atomic work
  cannot fit, reject before mutation rather than silently split the transaction.
- Define charge/admission scopes by host authority and accepted access/resource
  contracts, not presumed repository/workspace tenancy. Apply aggregate ceilings
  as well as bounded per-context/resource/class limits where supported. Exact
  quotas, cost models, borrowing and algorithms belong to the specification pass;
  per-document limits alone must not allow unbounded total resource use.
- Resource-accounting direction is agreed: maintain independent ceilings for
  memory/retention, processing, concurrency/outstanding effects, fan-out and time;
  a scheduling cost score cannot substitute for any physical/resource ceiling.
  Compose the declared host/authority envelope with granted originating scopes
  and operations. Children/retries/aliases inherit attribution rather than create
  quota. Preserve consistent shared-storage accounting and estimates versus actual
  usage; no whole-document duplicate charge for a mere handle clone, nor invisible
  storage retained through shared references. Use conservative bounds or bounded
  staged acquisition where cost is not initially exact; if required obligations
  cannot fit, reject before authoritative mutation, never partially publish.
  Numeric values, accounting/reclamation mechanisms and visibility into unrelated
  host/native allocations remain unselected; this is not a universal process-RSS
  or arbitrary-provider CPU guarantee.
- Fairness and aging/service-share policy must state when an eligible admitted
  lower-class operation can make progress under competing traffic. Bound work
  units or require cooperative yield where execution permits it. No arbitrary
  aging rule may promote work past authorization or protected-class limits.
  Progress depends on available host executor service and finite/cooperative
  work; it is not guaranteed preemption of arbitrary in-process/native code.
- Prefer explicit rejection, bounded deferral with expiry, or coalescing of
  replaceable work over indefinite enqueueing. Distinguish not admitted, pending,
  superseded, budget exhausted and an uncertain already-dispatched effect.
  Retry guidance cannot authorize replay of a write; one retry owner and bounded
  retry budget apply, and retries remain charged to the originating scope.
- Never evict authoritative effect evidence or unsaved text/comments merely to
  relieve diagnostic/background pressure. History/cache eviction must follow its
  declared retention/floor contract. Safe degradation can drop stale derived
  candidates, reduce optional presentation or sample/drop optional telemetry;
  it cannot weaken atomicity, permission checks or content-preservation promises.
- Diagnostic capture/drain/export has its own bounded resource envelope and
  feedback-suppression rules under section 12.7. Expose minimal drop/health status
  without requiring the full event queue, arbitrary formatting or export to work.
  Repeated rejection/drop alerts must not create a new overload loop. No guarantee
  of complete forensic evidence, permanent availability or host process survival
  follows from protected diagnostic accounting.
- Observe offered/admitted/rejected/coalesced/completed work and attempt rates,
  queue age/wait, service share, reservation usage, retained bytes, budget exhaustion
  and diagnostic loss where applicable. Use bounded, authorized class/scope
  projections; fairness metrics must not expose private principals, annotations
  or other users' activity by default. Do not misclassify shedding/supersession
  as a successful useful completion or an internal invariant fault.
- Formal safety obligations include aggregate bounds, no unauthorized class
  elevation, reservation integrity and all-or-nothing admission for required
  atomic publication. State starvation/progress assumptions separately. Verify
  independently with mixed human/agent bursts, many aliases/documents, tiny-request
  fan-out, expensive staging, cancellation that fails to stop a worker, completion
  floods, unavailable drains and shutdown under saturation. Test both telemetry
  on/off and that private-scope metrics remain correctly projected.

[Google SRE overload handling](https://sre.google/sre-book/handling-overload/)
informs criticality-aware admission and explicit shedding;
[OpenTelemetry performance/blocking guidance](https://opentelemetry.io/docs/specs/otel/performance/)
informs bounded instrumentation resource use. These are architectural inputs,
not adoption of RPC infrastructure or a guarantee about an arbitrary host executor.

### 7.6 Deadlines, shutdown and provider-fault scope — agreed direction

Use local monotonic durations for live deadlines, separately from finite work
budgets and caller waiting. Specify the start milestone of queue, preparation,
execution and end-to-end budgets; preparation is not free work outside admission.
Check expiry again at dispatch/publication admission, not only when a timer fires.
Children and retries inherit remaining limits rather than manufacture new quota.
Clock epoch, suspend behavior and unavailable/invalid clock observations need a
declared platform profile; do not subtract clocks across hosts or reuse process
instants after restart. Expiry stops new work and requests cooperative cancellation;
it neither reverses a publication nor proves that an external effect stopped.
Retain valid late evidence and actual resource obligations independently of timers.
No numeric timeout or arbitrary native-code preemption guarantee is selected.

Shutdown protects unsaved authored source and comments. Return a structured
needs-decision outcome supporting explicit save, discard or cancellation of the
shutdown; a UI prompt is one adapter, not an engine requirement. Do not silently
autosave/discard, and revalidate the acknowledged versions before destructive
disposal. Waiting for a decision need not freeze unrelated work. Once disposition
is accepted, stop new admission in the affected scope, cancel replaceable queued
work, and drain/reconcile accepted effects within a declared wait budget. Report
unresolved effects to the host. A forced close requires explicit host authority;
discarding local content does not undo an already issued write. Continued effects
need a safe retained owner or explicit lifetime transfer, not freed callback state.
The library does not own process termination. V1 promises neither restart recovery
nor a crash journal; a bounded shutdown wait is not a bounded worker lifetime.

Ordinary conflicts and I/O errors are not provider protocol contradictions. For
confirmed contradictory evidence from an accepted provider context, retain both
facts, exclude the contested baseline, stop the affected attempt and block
incompatible writes to that destination pending reconciliation. Expand isolation
to the provider/owner only when evidence indicates that wider integrity is
uncertain; escalate explicitly to the host. Re-enablement requires authorized
revalidation or replacement context while retaining relevant old receipts; do not
erase evidence, reset identity or blindly retry. Reject forged/unrelated messages
without allowing an attacker to quarantine an honest destination merely by
injecting invalid input. Exact fault classifications, escalation/re-entry contracts
and resource/time profiles remain specification and conformance work, not a newly
selected circuit-breaker framework.

The familiar protect-unsaved-work baseline is adapted to structured agent/host
outcomes rather than mandatory interaction. For clock limitations, see
[Rust Instant](https://doc.rust-lang.org/std/time/struct.Instant.html): monotonic
does not imply uniform ticks or portable suspend accounting.

## 8. View and rendering model

- Layout and presentation state must be separate from document state.
- The editor must support multiple views of the same document with independent
  selections, viewports, and display settings.
- Rendering should expose a framework-neutral semantic view/layout contract
  sufficient for both graphical and terminal frontends. It need not impose one
  identical glyph/cell scene on different geometry providers.
- Rendering must support incremental reuse and report invalidation information.
  Conservative wider invalidation is permitted when wrapping, shaping, or other
  dependencies prevent safe precise reuse.
- The model must account for soft wrapping, horizontal and vertical scrolling,
  tabs, variable-width graphical fonts, terminal cell widths, and zero-width or
  combining characters.
- The viewport should permit virtualization so large documents do not require
  full-document layout or rendering.
- The architecture must support decorations such as selections, cursors,
  diagnostics, search matches, comment/annotation indicators, matching brackets,
  and inline or gutter content. Authored annotations remain semantic content
  under section 3.8, not solely view decorations.

Layout, decoration and hit-test results must identify their document/view
incarnations, text revision and relevant layout/configuration generation. An
input action derived from obsolete geometry must be remapped only under a
validated policy or rejected/recomputed; screen coordinates are not stable edit
targets. Read-only semantic ranges and edit coordinates must not be widened or
weakened by painted glyph bounds or virtual content.

Define conversion laws only over their valid domains: bidi, wrapping, ligatures
and zero-width content may need explicit affinity or ambiguous/unavailable
results rather than a presumed one-to-one byte-to-screen mapping. Section 8.1
establishes agreed ownership of shaping/bidi/wrapping and navigation direction;
concrete provider interfaces and conformance remain for specification.

Bound layout work, retained geometry, decorations and invalidation propagation.
Track queueing, reuse, fallback and frame-budget pressure under the pathway
contract, not through per-glyph telemetry. Treat document/decorative text as
display data: it must not become terminal control sequences, executable markup
or authority-bearing commands through rendering. Native provider trust and any
limits of in-process failure containment must be explicit.

### 8.1 Layout ownership and navigation — agreed v1 direction

Agreed direction: a framework-neutral layout service contract with geometry
providers, shared logical semantics/display policy and one coherent accepted
layout per view/context. Providers may reuse host/framework facilities or an
optional common implementation; no particular shaping engine or crate is selected.
This shared presentation/interaction contract is an optional adapter-facing layer,
not a dependency of the authoritative object kernel. The approved v1 frontends
still require their applicable geometry integrations; headless invocation does not.

| Ownership model | Strength | Tradeoff |
|---|---|---|
| One mandatory shared glyph/layout engine | Strong implementation reuse | Forces graphical dependencies/assumptions on headless and terminal users; does not make terminals obey its glyph geometry |
| Each frontend implements layout and interaction independently | Fits native facilities | Risks divergent semantics and mismatched measurement, navigation and hit testing |
| Shared semantic/policy contract with geometry providers — recommended | Keeps editing consistent while supporting glyph and cell backends | Requires provider conformance, explicit capabilities and coherent geometry snapshots |

- The text/command layer owns logical UTF-8 coordinates, explicit conversions,
  grapheme-based character navigation/deletion policy, logical selection ranges
  and validated mutation. It does not own fonts, GPU resources, terminal control
  or an approximate parallel version of provider caret geometry. Programmatic
  exact-byte operations retain their declared boundary contract; glyph shaping
  must not silently widen their target ranges.
- The shared view/layout policy owns requested wrap mode, tabs, indentation of
  continuations, viewport/scroll intent, logical versus visual movement choice,
  decoration precedence and source/display projection rules. A geometry provider
  owns measurement, font/shaping/fallback where applicable, bidi ordering, fitting
  actual visual rows and the source-to-geometry mappings for that policy. Treat
  these as one coordinated layout computation, not competing independent passes.
- Unicode segmentation, line-break opportunities and bidi rules are distinct
  inputs. A pinned/tailored UAX #29 profile defines logical grapheme behavior;
  UAX #14 informs eligible wrap opportunities, while provider widths determine
  actual breaks. UAX #9 and any declared permitted higher-level protocol define
  bidi display when claimed. Pin versions, tailorings and conformance evidence
  in the specification pass; do not claim conformance from a dependency name.
- Authoritative text remains in logical source order. Soft wrapping, bidi
  display, escaped-control views, virtual comment indicators and font ligatures
  do not rewrite stored text or change source revision. Grapheme clusters and
  shaped glyph clusters are not interchangeable units. Layout-only changes
  invalidate dependent geometry, not the document's editing history.
- Painting, caret stops, selection geometry, pointer hits, visual navigation,
  IME candidate rectangles and native accessibility geometry must derive from
  the same accepted provider layout/context. Native provider indices require
  validated conversion to canonical source bytes; do not assume scalar, UTF-16,
  grapheme or glyph indices are byte offsets. No second approximate width model
  may authorize a different edit target from the displayed layout.
- A layout snapshot identifies document/view incarnations, text revision,
  relevant annotation/decoration/projection generations, dimensions/scale, font
  or terminal-width profile and layout-affecting configuration. Paint and input
  must use mutually compatible accepted context. Stale geometry must trigger
  recomputation/validated remapping or explicit refusal, not a guessed target;
  accepting work for one viewport must not install it as another view's geometry.
- Distinguish logical next/previous grapheme and logical-line operations from
  visual left/right, visual-row and screen-page operations. V1 requires shared
  logical movement and declared visual-row behavior for supported layouts;
  key bindings may choose a named semantic command/profile, never redefine an
  operation silently per backend. Full visual bidi movement is advertised only
  when the provider can supply reliable caret ordering and affinity.
- Preserve a preferred visual position for vertical movement within a compatible
  layout, resolving to valid source caret stops with explicit wrap/bidi affinity.
  A contiguous logical selection may paint as multiple visual fragments; drawing
  those fragments does not turn it into a rectangular or disjoint source edit.
  Visual rectangle selection remains deferred. Pointer-to-source mapping must
  distinguish source content, virtual/decorative content and unavailable targets.
- Graphical providers must account for proportional glyph widths, combining
  text and shaping supported by their declared script/font profile. Terminal
  providers work in cells with an explicit width/tab/emoji/ambiguous-width and
  bidi responsibility profile for the supported terminal/backend combination.
  Avoid double reordering by both provider and terminal. Unknown terminal behavior
  cannot justify a claim of exact geometry or full bidi parity.
- Both adapters must preserve source text and logical navigation even where
  native display support is limited. Offer an explicit safe escaped/projection
  fallback or report unavailable geometry for unsupported runs; do not silently
  guess widths or let spoofed control characters choose mutation targets.
  Directional-control visibility/source-safe display is a host-selectable named
  profile; comments and embedded languages do not silently activate a new bidi
  policy. A source-code display profile must not be forced on ordinary prose.
- Virtualization may bound prepared geometry, not truncate the semantic document.
  Wrapping, shaping and bidi may require context beyond the visible bytes; request
  it under budgets, with pending/partial/unavailable states when needed. Long
  paragraphs, many decorations or provider failure must not produce fabricated
  exact hit tests. Conservative invalidation and reduced presentation are valid
  only under their stated fallback contract.
- Formal properties include source preservation, valid caret/range domains,
  generation consistency and agreement between hit testing and caret geometry
  over the provider's supported mapping domain. Where geometry is non-injective,
  define an equivalence/affinity relation rather than an impossible universal
  inverse. Compare incremental/fresh layout under the same profile and use
  independent simple providers/Unicode fixtures plus real-platform evidence;
  pixel equality between frontends is not the conformance oracle.
- Bound layout/decoration work, retained snapshots and native resources. Attribute
  queueing, measurement, acceptance, reuse/fallback and observable presentation
  separately through MELT/golden signals. Apply STRIDE to provider inputs, virtual
  content, bidi/control spoofing, text disclosure and native resource loading;
  observation does not replace coordinate validation or safe rendering.

References: [UAX #29](https://www.unicode.org/reports/tr29/),
[UAX #14](https://www.unicode.org/reports/tr14/),
[UAX #9](https://www.unicode.org/reports/tr9/) and
[UTS #55 source-code handling](https://www.unicode.org/reports/tr55/) distinguish
segmentation, break opportunities, ordering and source-display concerns.
[iced paragraph geometry](https://docs.iced.rs/iced/advanced/text/trait.Paragraph.html)
and [Ratatui cell rendering](https://ratatui.rs/concepts/rendering/under-the-hood/)
inform different provider contracts, not a selected framework version.
[Emacs logical/visual motion](https://www.gnu.org/software/emacs/manual/html_node/emacs/Bidirectional-Editing.html),
[Helix wrapping policy](https://docs.helix-editor.com/editor.html) and
[Neovim display options](https://neovim.io/doc/user/options/)
inform explicit profiles without adopting their complete interaction models.

## 9. Frontend requirements

### 9.1 Shared expectations

Both frontends must:

- Translate native input events into shared editor input or commands.
- Render from the shared view model without recreating editing semantics.
- Manage focus, pointer interaction, scrolling, and redraw requests.
- Allow host applications to override bindings and styling.
- Surface editor events and external effects to the host.
- Support input method composition where the underlying frontend permits it.
- Expose accessibility information and actions where the framework permits it.

Frontend adaptation must preserve the semantic contract, not merely reproduce
input sequences. Resolve focus-dependent actions to explicit document/view
incarnations and validated coordinate context before execution; delayed events
must not silently target a newly focused view. Discovery, disabled controls and
native affordances do not replace current authorization/precondition checks.

Conformance must distinguish shared editing outcomes from native capabilities
and presentation. Attribute latency to input admission, commit, layout and the
presentation milestones actually observable to that host; do not claim display
or accessibility delivery from a redraw request alone. Test adapter boundaries
with controlled events and real platform evidence as required by section 13.4.

### 9.2 `iced`

The `iced` integration must be embeddable as a normal widget and must not require
ownership of the application event loop or executor.

### 9.3 `ratatui`

The `ratatui` integration must be embeddable as a widget in a caller-owned frame
and event loop. It must respect the allocated terminal rectangle and terminal
capabilities.

### 9.4 Text input and composition — architectural direction

- Distinguish command key events, committed text, and temporary composition
  (preedit). Do not reconstruct text from key names or run command bindings
  independently on events already consumed by an input method.
- Keep preedit in input-session/view state, anchored to a document incarnation,
  revision and replacement range. It must not change authoritative document
  text, advance its revision, enter saved content, or be parsed as committed
  source. Preedit display may require temporary view layout.
- An accepted composition commit becomes a normal validated replacement and
  a coherent undo group. Do not create undo entries for each preedit update.
  Empty preedit updates are not by themselves proof of cancellation; native
  adapters must preserve their platform's commit/cancel event ordering.
- V1 composition targets the primary selection. Preserve secondary selections
  through ordinary mapping without automatically duplicating composed text.
  Any future multi-target composition requires an explicit policy.
- Independent edits may continue while composition is active. Map the target
  only when its identity and expected content remain valid. Conflicting edits,
  disposed targets and changed authority must cancel or reject the composition
  explicitly rather than insert at a guessed new location.
- Focus loss, view switches, undo requests and conflicting commands need
  explicit native commit/cancel policies. A late commit must not be redirected
  to the newly focused view. Native replacement/surrounding-text requests must
  use validated coordinate conversion and disclosure limits.
- IME candidate geometry must derive from the accepted layout and composing
  range. Text-input limits, layout invalidation and cancellation must be bounded
  and testable independently of a live input method.
- All adapters must accept committed Unicode text. Native preedit, candidate
  positioning, surrounding-text exchange and key distinctions are platform
  capabilities, not assumed frontend parity. Terminal input must not fabricate
  composition events that its backend does not expose.

The [iced input-method events](https://docs.rs/iced/latest/iced/advanced/input_method/enum.Event.html)
provide a concrete preedit/commit reference. Emacs's
[input methods](https://www.gnu.org/software/emacs/manual/html_node/elisp/Input-Methods.html)
also distinguish input conversion from text encoding. Neither reference fixes
the complete native platform behavior for Vellum.

### 9.5 Accessibility — architectural direction

- Expose semantic text, document/view identity, editable/read-only state, focus,
  primary caret/selection, and supported actions independently of painted glyphs
  or cells. Native assistive actions must enter the same validated command path.
- Keyboard access, visible focus, discoverable commands and a way to leave the
  editor, command prompt, or pending chord are baseline frontend requirements.
  Status and errors must not rely on color alone; cursor animation and visual
  effects must admit host accessibility preferences.
- Virtualization must not restrict the semantic document to currently painted
  text. Provide bounded range access and explicit coordinate mapping for native
  text interfaces, including wrapped lines and embedded-language source text.
- Publish text, selection and focus changes consistently with committed state.
  Coalesce announcements and avoid narrating every background parse or frame.
  Temporary composition must be distinguishable from committed text where the
  native accessibility interface supports it.
- Accessibility reads and surrounding-text access must follow the host's
  disclosure policy; semantic export is not exempt from resource boundaries.
- Document support by frontend, OS, backend and relevant framework version.
  Identify unsupported native facilities and fallbacks explicitly. Keyboard
  operability alone must not be advertised as complete screen-reader support.
  V1 must not claim cross-platform native accessibility parity without evidence.

[WCAG 2.2](https://www.w3.org/TR/WCAG22/) supplies useful keyboard, focus and
nonvisual-feedback principles; it is not by itself a native-desktop conformance
claim. Native text interfaces such as
[UI Automation TextPattern](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/ui-automation-textpattern-overview)
illustrate why semantic text/selection access must be separate from rendering.

## 10. Host services and effects

Platform and application services must be expressed as explicit requests and
responses rather than hidden global dependencies. Relevant services include:

- Clipboard access.
- File dialogs and file I/O.
- Timers.
- Task spawning and cancellation.
- Language servers and other external processes.
- System theme and accessibility settings.
- Logging and diagnostics.

The host must be able to determine which capabilities are available. Unsupported
services must fail explicitly and gracefully.

At each host-service interface, identify the caller's granted scope, requested
guarantee, resource identity/binding and relevant operation/effect attempt. The
host must state what a completion actually attests; dispatch, acceptance,
completion and durability are not interchangeable. Validate completions through
section 7.4 before using them as state-transition inputs.

Service contracts must expose failure/unknown outcomes and limits, and identify
which freshness, ordering, cancellation or conditional-write guarantees depend
on host enforcement. Diagnose queueing, dispatch and acknowledgment separately
under sections 12.5-12.6 without copying payloads into diagnostics. The host is
part of the declared trust boundary, not an oracle that makes every external
operation atomic. File identity, encoding/line-ending fidelity and saved-state
policies remain open under section 17.14; this interface rule does not choose
those policies or require deferred services in v1.

### 10.1 Clipboard and text transfer — architectural direction

This is a required editor-integration contract, usable by automation without a UI,
not clipboard-specific state or workflow verbs in the semantic engine. The
integration composes authorized captures, generic effect/evidence contracts and
validated edits through the behavior facade; provider mechanisms remain at the
host edge. This ownership does not weaken any transfer, disclosure, retention,
original-target, acknowledgment or local atomicity requirement below.

- Treat internal text transfer, the system clipboard and platform selections
  as distinct sources/destinations with explicit host capabilities. Ordinary
  deletion must not implicitly export text. Clipboard reads and writes must
  follow host disclosure policy; unsupported system access must be explicit.
- V1 requires plain Unicode text transfer and multi-selection replacement.
  Named registers, a persistent kill ring, rich media, executable paste formats
  and synchronization of clipboard histories are not required. A host may expose
  a bounded session-local structured payload without implementing those features.
- Copy captures source text and normalized selections from an identified
  snapshot. Empty selections have no implicit whole-line meaning in the base
  copy/cut commands; line-copy/cut is a separate declared command behavior.
- A structured internal payload may preserve ordered selection fragments.
  Pair fragments with targets in document order when counts agree. A plain-text
  payload is inserted as a whole at each target by default, not split by newline.
  Mismatched structured counts need an explicit flatten/broadcast policy or a
  structured rejection. External text must not acquire trusted internal metadata
  solely because it equals a previous copied string.
- Export of multiple fragments to a plain-text clipboard needs a documented
  separator/serialization policy. Preserve exact fragment boundaries internally;
  a plain-text round trip must not promise lossless reconstruction of them.
- Paste captures target document/view incarnations, revision and selection
  ranges when requested. A later clipboard result must not target newly focused
  text. V1 may reject changed revisions; any future mapping must validate target
  identity and expected content before committing one atomic replacement group.
- Clipboard text and bracketed terminal paste enter the text-insertion command
  path, not key binding, macro, command-prompt or shell interpretation. Formatting,
  indentation and line-ending transformations need explicit paste policies.
  Bound payload size, target count, total expansion and outstanding requests;
  truncation must not be silently presented as a complete paste.
- System cut is a staged operation: validate edit/disclosure rights and retain
  the captured payload, request its clipboard export, then revalidate the source
  and commit deletion only after the host confirms the required write outcome.
  A failed, cancelled or unknown write outcome must leave document text intact.
- If clipboard export succeeds but source revalidation fails, report copied but
  not cut and preserve current text. Clipboard export and document deletion are
  not one atomic transaction across systems. A host acknowledgment must state
  whether it means accepted ownership or actual transfer; it does not imply
  durable storage. A local retained payload follows explicit bounded retention.
- Copy has no text undo entry; cut and paste each form coherent text undo groups.
  Undo must not overwrite a later system clipboard value or claim to reverse an
  earlier disclosure. Results arriving after cancellation/disposal remain subject
  to the effect identity and outcome-evidence rules in section 7.4.
- Session-local payloads and clipboard histories are sensitive data. Do not
  persist, log, poll, synchronize or route them between authorities implicitly.
  Future remote sessions must name whose clipboard is being accessed; forwarding
  through a terminal/provider does not itself authorize access to another device.

Comparative references: [Helix registers](https://docs.helix-editor.com/master/registers.html),
[Neovim clipboard providers](https://neovim.io/doc/user/provider/), and
[Emacs clipboard/kill-ring integration](https://www.gnu.org/s/emacs/manual/html_node/emacs/Clipboard.html).
Vellum adopts explicit source/destination and failure contracts rather than any
editor's full register or kill-ring model. The
[asynchronous Clipboard API](https://www.w3.org/TR/clipboard-apis/) is a useful
request/permission reference, not a native-platform guarantee.

## 11. Configuration and extensibility

- Configuration should be composable across editor defaults, language settings,
  application settings, and per-view overrides.
- Frontend keymaps must support chords, modes or contexts, precedence and host
  overrides. Their runtime state and activation are not engine configuration.
- Themes and style roles must be frontend-neutral while permitting frontend-
  specific rendering choices.
- Hosts must be able to register commands, language support, decorations, and
  event listeners through documented extension points.
- Extension callbacks must not be able to bypass editor invariants.
- Public extension points must define thread-safety and lifetime expectations.

### 11.1 Configuration contracts and safe activation

- Apply configuration contracts at the owning layer. Engine semantic settings
  and frontend preferences may use shared pure resolution utilities, but keymap/
  chord state must not be installed into the engine root. The engine does not
  promise a joint atomic publication of its state and a UI preference change.
  A UI must submit any resulting semantic command through ordinary admission.
- Separate document semantics, view/input preferences and authority restrictions.
  A view override cannot relax read-only or disclosure policy. Declarative
  settings, keymaps and language labels are data, not a route to arbitrary
  execution or an implicit grant to load code, access files or use the network.
- Define per-setting scope, precedence and merge behavior rather than assuming
  every setting admits one generic deep merge. Binding removal, explicit blocking
  and fallthrough remain distinct. Section 11.2 establishes the agreed resolution
  and activation direction; concrete contracts remain for specification.
- A configuration/registry generation must identify the accepted contract used
  to resolve commands and derived work. Stage and validate a proposed change;
  failed activation must leave a coherent prior profile or an explicit degraded
  state, not partially installed semantics. Section 11.2 establishes activation
  and in-flight-work direction; concrete interfaces remain for specification.
- Bound chord/context evaluation, registrations, listener work and retained
  configuration generations. Extensions must request subsequent mutations
  through the validated operation path, not reenter an in-progress publication.
  Callback failure cannot retroactively reject a committed transaction.
- Configuration inspection, activation, rejection and degradation need structured
  outcomes and content-safe observability across the registry/provider boundary.
  Validate policy provenance and applicable capabilities; a configuration source
  being readable does not make its instructions trusted.
- Changing configuration is a behavioral-contract change where it affects
  command meaning, coordinate/matching profiles, authority or effect semantics.
  Preserve version/profile attribution and identify the refinement/conformance
  obligations under sections 13.4 and 16.2 before implementation.

### 11.2 Scoped resolution and activation — agreed v1 direction

The agreed direction is typed scoped configuration, explicit ordered resolution
and staged atomic activation of coherent profiles. Preserve old context for work
already admitted, but never treat old configuration as a continuing authority
grant. This contract does not require a scripting engine, live configuration
watcher, binary-plugin hot reload or authority-wide pause for every preference.
Apply these rules within each owning layer under §3.1: engine semantic profiles
and frontend input/presentation profiles do not form one atomic preferences store.

| Activation model | Strength | Tradeoff |
|---|---|---|
| Mutate individual settings/providers in place | Simple setters and immediate effects | Observers can see incompatible intermediate combinations; rollback and in-flight interpretation become ambiguous |
| Drain all work, then replace configuration | Simple quiescent boundary | Couples unrelated scopes and may wait indefinitely on effects; does not resolve unknown external outcomes |
| Stage, validate and atomically publish a scoped profile — recommended | Coherent activation with bounded publication; unrelated work may continue | Requires generation/dependency tracking, safe retained contexts and explicit migration/rejection rules |

- Register each setting's owner, type, valid scopes, composition rule, dependency impact
  and sensitivity. Distinguish document semantics/serialization, view geometry/
  input preferences, analysis context, and host authority/resource limits.
  Unsupported keys/types or invalid combinations reject the candidate; unknown
  authority-bearing data must not be accepted as decorative metadata.
- For overridable preferences, resolve from lower to higher precedence: library
  defaults, host application defaults, selected language profile, explicitly
  applicable workspace profile, explicit document override, then view/input
  context where that setting permits it. Only legal scopes participate. An
  explicit host-enforced restriction is not another overridable preference.
- Because one document may belong to several workspaces, membership alone must
  not select its document-wide semantics. Require an explicit profile association
  or ordered applicable-profile list; incompatible equal-priority inputs reject
  activation. Do not use membership enumeration, current focus or open order as
  an implicit tie-breaker. Views may differ only for settings declared view-local.
- Scalars replace by precedence; lists/ordered rule sets replace unless their
  schema explicitly specifies stable composition; maps merge only at declared
  keys with that key's rule. Absence inherits; removing an override exposes lower
  layers. Reset-to-library-default is an explicit value, not another spelling of
  removal. Do not silently concatenate commands, capability sets or provider lists.
- Frontend key binding resolution distinguishes bind, block/consume, and fallthrough.
  Removing a binding in a layer is not a block: it permits lower layers to apply.
  An explicit fallthrough bypasses that layer's applicable default for the key;
  block stops lower lookup. Equal-rank conflicts and command/prefix ambiguity must
  be resolved explicitly or reject activation. No arbitrary callback predicates,
  unbounded recursive remapping or raw key-replay scripting is added to v1.
- Host policy constrains effective permissions, disclosure and resource ceilings
  after preference resolution; configuration cannot exceed those grants. Reject
  forbidden requested values rather than silently broaden rights or claim an
  unsatisfied guarantee. Relaxing a host restriction requires separately authorized
  policy change, not a document/view override. A readable workspace config is
  untrusted input until its accepted scope and provenance are established.
- Stage profile resolution, schemas, command identifiers and compatible provider/
  grammar/query assets before activation. Host preparation effects remain explicit
  and capability-controlled; preparation may fail without replacing the active
  profile. Validate expected scope/configuration generations and publish the
  coherent affected set at a bounded local boundary, with no I/O or arbitrary
  callbacks. Superseded candidates cannot overwrite a newer activation.
- Keep configuration generation, current capability/policy state and per-service
  readiness distinct. A profile may explicitly disable/degrade an optional service,
  but failure must not silently install half a required combination. Unrelated
  scopes need not be republished; any cross-scope invariant requires a coherent
  activation unit covering its dependencies. Publication does not require waiting
  for every view to repaint or every new parse to finish.
- Work already admitted retains the command/schema and relevant configuration
  context under which it was prepared. At commit/result acceptance, revalidate
  current authority and affected dependencies; use an explicitly compatible result
  or reject/recompute, never reinterpret the old payload under new semantics.
  A purely cosmetic change need not invalidate unrelated computation; exact
  dependency classification must be specified and verified.
- An effect already dispatched retains its original request and outcome contract.
  Removing a provider/command or revoking permission cannot erase its late evidence
  or prove cancellation. Bound retained provider contexts and define drain/refusal/
  isolation when safe reclamation is not possible; do not unload code still in use.
- Relevant frontend keymap/mode changes cancel pending chord interpretation explicitly;
  consumed prefix keys are not replayed as text or commands under the new map.
  Keymap changes do not reconstruct IME input. Composition follows section 9.4:
  preserve a compatible native session or explicitly cancel/reject it, without
  redirecting a late commit to new focus. Concrete native ordering remains for
  the platform specification. These frontend transitions are not engine activation
  participants and cannot prevent an otherwise valid headless semantic command.
- Make profile inspection explain effective value, origin/precedence and relevant
  generation within disclosure limits. Configuration activation/rejection is a
  separate operation, not source-text undo history or a way to replay effects.
  Intentional rollback is a newly validated activation, not restoration of old
  authority or an old generation identifier.
- Formal obligations are deterministic resolution for identical ordered inputs,
  coherent publication, freshness preservation and non-expansion of host authority.
  Composition need not be commutative and is not assumed to form a global lattice.
  Independently verify same-rank conflicts, workspace ambiguity, failed/superseded
  activation, chord cancellation, stale pure results and late external evidence.
  Bound resolution depth, registrations, retained generations and migration work;
  apply STRIDE and content-safe MELT/golden signals along these interfaces.

References: [CodeMirror configuration compartments](https://codemirror.net/examples/config/)
and [facets/precedence](https://codemirror.net/docs/ref/) inform scoped coherent
configuration; [Helix layering/reload](https://docs.helix-editor.com/configuration.html)
and [declarative remapping](https://docs.helix-editor.com/remapping.html) inform
host-controlled preferences; [Emacs keymap fallthrough](https://www.gnu.org/software/emacs/manual/html_node/elisp/Format-of-Keymaps.html)
and [Neovim global/buffer-local mappings](https://neovim.io/doc/user/api/)
illustrate why binding scope and suppression must be explicit. Their scripting
or macro facilities are not adopted by this contract.

## 12. Observability and errors

### 12.1 MELT instrumentation

MELT—metrics, events, logs, and traces—is a required part of the editor's
development and operational model rather than an after-the-fact integration.
The editor should expose structured, backend-neutral instrumentation for:

- **Metrics:** counters, gauges, and histograms for rates, durations, sizes,
  queue depths, cache behavior, cancellations, and discarded stale work.
- **Events:** structured records for meaningful state transitions such as a
  command completing, a document revision being committed, work being
  superseded, a grammar failing to load, or a worker failing.
- **Logs:** diagnostic messages with severity, stable targets, and structured
  fields suitable for local development and production troubleshooting.
- **Traces:** correlated spans across input handling, command dispatch,
  transactions, background effects, parsing, layout, rendering, and frontend
  presentation.

Instrumentation must:

- Be optional and must not require a particular telemetry backend.
- Allow the host to connect its preferred metrics, logging, and tracing stack.
- Correlate asynchronous work with its originating command, document, view,
  revision, and task where applicable.
- Preserve correlation when work moves between threads or executors.
- Use documented, stable names and structured fields for public signals.
- Keep metric dimensions bounded. Per-operation correlation identifiers may
  appear in access-controlled events and traces, not as metric labels; their
  volume, retention and field sizes must also be bounded.
- Avoid recording document contents, selected text, file contents, command
  arguments, or other sensitive data by default.
- Have bounded and measured overhead, particularly in input and render paths.
- Section 12.7 establishes bounded callback-free capture and host-controlled draining;
  an instrumentation hook must not implicitly admit arbitrary subscriber code
  into publication, even when its exporter is described as asynchronous.
- Permit expensive diagnostic instrumentation to be enabled for development
  without imposing its cost on normal embeddings.

### 12.2 Golden signals

Development and performance review must use the four golden signals:

- **Latency:** input-to-visible-update time and the duration distributions of
  command execution, parsing, highlighting, layout, rendering, file operations,
  and external-service requests.
- **Traffic:** input-event, command, edit, parse, frame, search, and external-
  request rates, including burst behavior.
- **Errors:** failed commands and effects, worker failures, grammar-service
  faults, save failures, and frontend integration errors. Track expected
  conflicts, cancellations, superseded work, syntax diagnostics and missed
  frame budgets separately rather than treating them all as system faults.
- **Saturation:** worker and effect queue occupancy, executor utilization,
  outstanding tasks, memory and snapshot pressure, cache pressure, and frame-
  budget exhaustion.

Golden signals must be inspectable in development builds and exercised by
representative benchmarks and stress tests. Regressions should be evaluated
against explicit budgets rather than averages alone; latency distributions must
include tail percentiles.

### 12.3 Errors and diagnostics

- Public failures should use structured error types where callers can recover.
- User-facing messages should be distinct from programmer errors and logs.
- Instrumentation should make it possible to follow a user action through
  command dispatch, state mutation, scheduled effects, async completion,
  invalidation, and presentation without exposing private content.
- Worker failures and degraded operation must be observable even when the host
  chooses not to display them to the user.
- Sensitive document contents must not be logged by default.

### 12.4 Outcome, fault and health architecture

Authoritative operation status must remain available independently of optional
MELT instrumentation. Logs and sampled traces are diagnostic observations, not
the operation ledger, persistence receipt, authorization decision or recovery
journal. V1 requires structured local outcomes and inspectable current health;
it does not require durable diagnostic history, a telemetry service or a new
workflow runtime.

- Distinguish successful/no-op results; expected rejection such as stale
  preconditions, denied authority or unsupported capability; cancellation and
  supersession; resource exhaustion; dependency/service failure; and internal
  contract or invariant faults. These are architectural categories, not a final
  error enum. Malformed document syntax is language-service data, not by itself
  an editor fault.
- Report effect evidence and applicability separately from the command outcome,
  following section 7.4. A timeout can leave an external outcome unknown; a
  committed edit with a failed save remains committed and unsaved. Notification
  or exporter failure must not turn a published transaction into a rejection.
- Structured outcomes must identify the relevant operation/effect and affected
  scope, stable machine-readable reason, known commit/persistence milestones,
  and safe next actions where available. Callers must not parse localized prose
  or infer retry safety from severity or a single generic retryable flag.
- Recovery recommendations must respect current preconditions, authority and
  effect evidence. Replan, retry, reconcile, disable a derived service and
  escalate for intervention are distinct actions; none may silently weaken an
  atomicity or authorization requirement.
- Recoverable failures remain scoped to the affected operation or service.
  Internal faults follow the fail-fast and supervision rules in sections 4.4
  and 7.4; catching a panic does not prove that mutable state is safe to reuse.
  Diagnostics must report degraded or isolated scope without promising that an
  in-process library can contain every process-level failure.
- Expose a bounded, headless health/capability snapshot for hosts and both
  reference frontends: configured/enabled, available, degraded, disabled or
  unknown as applicable, with safe reason codes and freshness. Optional service
  absence is not automatically unhealthy. Distinguish configuration checks from
  live probes; probes with effects require explicit host authority and budgets.
- User messages, host outcomes and supervisor signals may present different
  detail but must agree on underlying state. Coalesce repetitive presentation
  without losing authoritative outcomes. Optional diagnostic queues may drop
  or sample under pressure, with observable loss; operation results must not
  depend on those queues or an exporter remaining available.
- Telemetry must not introduce network access or block publication on export.
  Bound collection, buffering and shutdown; suppress recursive instrumentation
  failures. Exporter failure must not change edit semantics. Durable security
  audit, if later required, needs a separate explicit admission/persistence
  contract rather than being implemented as best-effort logging.
- Default diagnostic fields must exclude document text, search/clipboard
  payloads, paths, credentials, arbitrary host error strings and command
  arguments. Correlation tokens are not authority and may themselves reveal
  activity. Hashing content or paths is not automatically anonymization.
  Diagnostic export and richer capture require explicit scope, disclosure
  policy and retention limits; error formatting is also a disclosure boundary.
- Section 12.8 establishes field classification/allowlisting before capture and
  separately authorized release, covering annotations, quoted context, previews,
  configuration and debug/support output as well as ordinary telemetry fields.

Design references: [OpenTelemetry error recording](https://opentelemetry.io/docs/specs/semconv/general/recording-errors/)
and [instrumentation error handling](https://opentelemetry.io/docs/specs/otel/error-handling/)
inform observation semantics and isolation from application behavior, without
mandating an SDK. [Neovim health checks](https://neovim.io/doc/user/health/) and
[Helix health reporting](https://docs.helix-editor.com/lang-support.html) inform
inspectable environment/capability diagnostics;
[Emacs condition handling](https://www.gnu.org/software/emacs/manual/html_node/elisp/Handling-Errors.html)
informs machine-distinguishable recovery rather than message parsing. Their
runtime mechanisms do not override Vellum's closed operation model.

### 12.5 Pathway and interface coverage

MELT and the golden signals must be designed together along end-to-end pathways
and at their interfaces, not only within individual components. MELT provides
complementary observation forms; latency, traffic, errors and saturation define
the questions those observations must answer. This does not require four
separate telemetry records for every event or indiscriminate per-token tracing.

- The digital twin must identify supported pathways and their interface
  crossings: input/API admission, command resolution and authorization,
  preparation/validation/publication, worker scheduling/completion, host-effect
  dispatch/acknowledgment, invalidation/layout/presentation, and lifecycle or
  supervision transitions. Include rejection, overload, cancellation, stale
  completion and failure branches, not only the successful path. Deferred
  federation and durable-execution pathways remain marked deferred.
- At each material interface, define the producer/consumer, ownership and trust
  boundary, correlation handoff, observable milestones and applicable golden
  signals. Record explicit non-applicability or visibility limitations rather
  than inventing measurements of host or external behavior the editor cannot see.
- Latency must distinguish admission/queue wait, execution/service time and
  end-to-end completion. Commit, effect acknowledgment, persistence and visible
  presentation are separate milestones; a submitted frame is not proof that a
  user saw it. State measurement endpoints and clock domains explicitly; use
  local monotonic durations and do not subtract unsynchronized remote clocks.
- Traffic must distinguish offered, admitted, completed, rejected, coalesced and
  discarded work where relevant. Separate logical operations from effect
  attempts and fan-out so retries do not masquerade as increased useful work.
- Errors must preserve the outcome distinctions in section 12.4, identify the
  failing boundary and affected pathway, and distinguish an attempt failure
  from the final logical outcome. Define rate denominators and count scope to
  avoid counting one propagated fault as multiple independent failed operations.
- Saturation must expose the relevant constrained resource and backpressure:
  queue occupancy/age and limits, worker capacity, retained snapshots/bytes,
  pending effects or presentation budgets. Where hosts own capacity, report
  known observations and unknowns rather than implying full executor visibility.
- Correlate fan-out, fan-in, coalescing, retries and asynchronous continuations
  through causal relationships; do not force every pathway into a single nested
  call stack. Instrumentation must not introduce serialization or change work
  ordering merely to produce a simpler trace.
- During specification, maintain a pathway/interface coverage map linking
  milestones, signals, privacy classification, collection cost and verification
  scenarios. Verify correlation, boundary attribution and overload behavior with
  controlled hosts, including when diagnostic collection is disabled or drops
  records. Concrete schemas and numeric budgets remain for that later pass.
- State which contract property each observation can support and what remains
  unobservable. Correlation is evidence of attributed activity, not proof of
  causality, completeness or correctness. Compare observed outcomes with the
  modeled milestones; unexpected divergence must be diagnosable without letting
  telemetry itself authorize a transition or override authoritative state.


#### 12.5.1 Observation-led tuning — agreed direction

Resource and scheduling profiles must be observable enough to diagnose pressure
and evaluate tuning against representative human/agent workloads. Borrow mature
mechanisms, but select numeric defaults using evidence rather than copying an
interactive editor's thresholds or presuming agents create idle periods.

- Expose bounded, authorized observations of queue age/service time, admitted and
  useful completed work, conflicts/stale-result rejection, processing/retained
  bytes, snapshots/history/staging, outstanding effects and reconciliation age.
  Attribute the constrained boundary and distinguish useful work from retries,
  supersession and partial analysis. Include pending/failed observations, not
  just successful-operation latency. Host-unobservable quantities stay unknown.
- Relate measurements to the effective bounded policy/profile and its activation
  context, workload class/size buckets and supported environment. Bound provenance
  retention and series cardinality; profile versions, paths, object identities and
  private principals must not become unbounded metric labels or public activity
  counters. Prefer distributions over isolated averages; exact schemas/buckets
  and latency objectives remain specification work.
- Change tunable profiles through authorized staged configuration, with recorded
  rationale, comparable workload evidence and an explicit rollback path. Apply
  changes prospectively and account for existing obligations; lowering a limit
  cannot erase a running write, retained snapshot or required history/evidence.
  Atomicity, authority checks, fidelity, outcome truthfulness and boundedness are
  invariants, not tuning knobs. No automatic adaptation is authorized merely by
  collecting telemetry; any later controller needs its own bounded policy and
  validation contract.
- Measure instrumentation overhead, capture/export loss and on/off behavior;
  require safe local inspection without mandatory network export. Diagnostics
  remain outside publication and cannot be required to complete an ordinary
  operation. Compare throughput, latency, pressure and failure outcomes together
  so shedding work or hiding stalls cannot appear as a performance improvement.

This establishes tuning evidence requirements, not numeric defaults or a scheduler
algorithm. Section 7.5's direction is agreed separately through user review;
instrumentation itself neither selects policy nor proves its fairness.

#### 12.5.2 Architecture-wide observation coverage

Before concrete telemetry specification, maintain an explicit observation-intent
index for every execution pathway, identity dimension, connection, data flow,
data asset, interface, safety/progress obligation and environmental assumption.
[The coverage index](formal/observability.json) and model §11.3 record this pass.
This is architectural metadata, not frozen runtime schemas or installed sensors.

- For each item, identify existing instrumentation placements, the governing
  authoritative contract/state, relevant acceptance scenarios, observation focus
  and what cannot be inferred. Link reusable metrics/events/logs/traces intent
  and all four golden-signal questions; record unknown or not-applicable cases
  with reasons rather than inventing measurements. Security remains cross-cutting
  STRIDE; no additional undefined MELT signal is introduced.
- Identity handling needs mismatch/expiry/continuity and scope validation
  observations, not raw identity values. Distinguish host-established identity,
  caller claims, operation/attempt correlation and recipient-scoped tokens.
  None is an interchangeable version, credential or metric label.
- Connection plans distinguish in-process invocation, local host services and
  an explicitly enabled host diagnostic sink. Do not add a listener, TCP/TLS
  handshake, network identity service, cross-clock timing or core exporter merely
  to complete a signal table. Host-unobserved service facts remain unknown.
- Count offered/admitted/completed/rejected work at declared boundaries; separate
  logical operations, child work, attempts, participants and diagnostic records.
  Include pending/censored age and actual retained obligations. A propagated fault
  is not several independent failures; sampling/loss/off is not useful zero work.
- Data-flow observations must distinguish capture, transformation, acceptance,
  release, persistence and disposal. Current release applies to each new event
  delivery even after resync. Payload bodies, paths, quoted targets, private
  topology, raw host errors, credentials and stable fingerprints are not default
  diagnostics. Permitted operational disclosures follow §15.6, not an absolute
  promise of secrecy despite shared-resource interference.
- A signal supports diagnosis of contract enforcement; it is not the grant,
  publication, receipt ledger, invariant proof or proof of a trusted supplier's
  honesty. Invariant truth and operation status come from authoritative state and
  typed outcomes, even when diagnostics are disabled, sampled, lost or unavailable.
- Validate the observation index against the catalogs and check sensitivity to
  missing placements, wrong instrumentation families and diagnostic-as-authority
  mistakes. Semantic suitability still requires review; structural completeness
  is not whole-system coverage, runtime instrumentation or conformance evidence.

Concrete names/units, event fields, label budgets, histogram buckets, target
latencies/service bounds, overhead and executed native/host/security scenarios
remain specification/evidence work. The architecture pass must not invent them.

#### 12.5.3 Complementary review lenses — agreed architectural direction

Use the following lenses together with MELT, golden signals and STRIDE. They
answer different questions over the same contracts and observations; they do not
require separate telemetry pipelines, dashboards, services or runtime frameworks.
Apply each where meaningful and record exclusions/unknowns rather than requiring
a Cartesian product for every catalog element. This adopts review obligations,
not a claim that every corresponding analysis or implementation test is complete.

| Lens | Required architectural question and application | Boundary / tradeoff |
|---|---|---|
| USE: utilization, saturation, errors | For each constrained resource, what is used, what work cannot be served immediately, and what fails? Include workers, publication staging, reservations, retained source/history/results, effect slots and diagnostic buffers | Distinguish service-busy time from occupied capacity, and short bursts from long averages. Use host evidence only where available; logical bytes are not physical memory. Resource inventory/accounting refinement remains specification work |
| RED: rate, errors, duration | For request-bearing interfaces/pathways, what logical work arrives, which outcomes violate the declared service contract, and how long does disposition take? Reuse golden-signal traffic/error/latency observations | RED is a compact request-oriented view, not additional duplicate counters or a replacement for saturation, pending ages or useful-work analysis. Separate offers/admissions/completions and attempts; a denied, stale, conflicting, cancelled or unsupported request is not automatically a service failure |
| Outcome SLIs/SLOs | Define useful authorized outcomes and eligible populations: correct atomic edits, exact captured-save evidence, current accepted analysis, and responsive protected control work | Specify denominators, observation milestones, workload/host assumptions and pending/censored treatment before numeric targets. Admission shedding or excluding inconvenient outcomes must not manufacture improvement. Correctness/authorization invariants are not probabilistic error budgets |
| LINDDUN privacy analysis | Examine linking, identifying, unwanted non-repudiation, detecting involvement, excessive data disclosure, unawareness/unintervenability and non-compliance across identities, annotations, flows, retention and diagnostics | Privacy is broader than access control. Pseudonyms, aggregate counts and correlation can reveal activity. Minimize linkability/retention and preserve current recipient projection and declared operational disclosures under §15.6; no legal-compliance certification or blanket erasure guarantee |
| STPA-style interaction analysis | For consequential control pathways, identify possible losses and constraints when actions are omitted, provided unsafely, mistimed/misordered or applied too long/stopped too soon | Examine current release, publication, cancellation/quiescence, save evidence, clipboard cut/paste and diagnostic remediation. Correct local components can still interact unsafely; use focused analysis, not a claim of safety certification |
| Failure-mode analysis and fault injection | Link representative failures to local/system effects, detection limits, containment, safe disposition and a verification scenario | Cover stuck workers, exhausted reservations, dropped diagnostics, stale/duplicate/delayed evidence and provider contradictions. Prefer focused FMEA-style records and targeted fault scenarios over speculative numeric risk scores; planned tests are not executed evidence |
| Structured assurance cases | Link each consequential claim to its contract, argument, assumptions, hazards/controls, proof/test evidence and current artifact identity | Reuse existing requirement/model/scenario/proof links. Distinguish checked, reviewed, assumed, planned and contradicted claims. A GSN-style argument or valid graph is not itself a proof, independent oracle or Cleanroom certification |
| Conservation/lifecycle accounting | State which obligations/resources are created, retained, transferred and actually discharged, including opening balances and scope/epoch boundaries | Cancellation, timeout, dropped records or caller departure must not discharge still-live worker/effect obligations. Do not conflate resource release, terminal control, observed completion and successful persistence; derive exact identities only after units and transitions are specified |
| Semantic quality, useful work and progress | Observe accepted/current results versus superseded/stale work, declared dependency coverage, retry/invalidation/fan-out amplification, oldest eligible work and service distribution | Complete-within-scope, partial, unsupported and unknown remain distinct. Throughput or fast rejection alone does not prove useful service, semantic correctness, fairness or eventual completion |

The organizing trace is contract → possible loss → prevention/containment →
observation → verification evidence → permitted response. Record measurement
freshness, coverage/loss and known blind spots; diagnostics cannot become the
authoritative ledger or bypass ordinary command gates. Conservation laws should
be checked against authoritative state where specified, not reconstructed from
sampled logs/traces. Statistical service objectives never weaken hard safety or
disclosure constraints.

Use existing placements and profiles in §12.5.2. During specification, refine
resource inventories, failure/control scenarios, assurance arguments and SLI
populations alongside the relevant contracts. Keep exact counters, budget/SLO
values, algorithms, runtime checks and actual fault-injection evidence explicit
as remaining work rather than inventing them at requirements stage.

References: [USE](https://www.brendangregg.com/usemethod.html),
[RED](https://grafana.com/files/grafanacon_eu_2018/Tom_Wilkie_GrafanaCon_EU_2018.pdf),
[outcome SLOs](https://sre.google/workbook/implementing-slos/),
[LINDDUN](https://linddun.org/threat-types/),
[STPA handbook resources](https://psas.scripts.mit.edu/home/books-and-handbooks/),
[NASA model-based failure analysis](https://science.nasa.gov/science-research/science-enabling-technology/technology-highlights/integrating-model-based-systems-engineering-and-fault-management-to-enable-autonomous-space-missions/)
and [assurance-case foundations](https://insights.sei.cmu.edu/documents/1222/2012_005_001_28161.pdf).
These inform the review method; no framework implementation or certification is adopted.

The selected abstract-contract pass in [formal/review-lenses.md](formal/review-lenses.md)
and model §11.5 adds restricted accounting, diagnostic-projection and conditional
closed-backlog progress checks, plus typed assurance arguments. It does not
discharge every review obligation above: actual resource/field/clock/service
profiles, general privacy/semantic correctness and implementation evidence remain
necessary. Final results are scoped to their explicit assumptions and input hashes.

### 12.6 STRIDE perspectives on observability

Apply STRIDE both to the pathways observed by MELT/golden signals and to the
observation pipeline itself: producers, context propagation, buffers, aggregation,
health interfaces, configuration, export, storage and diagnostic presentation.
Telemetry is security-relevant data, not an implicitly trusted control plane.

| Perspective | Observability threat and required architectural response |
|---|---|
| Spoofing | Forged producer identity, health reports or correlation context can misattribute activity. Separate locally established identity from caller-supplied claims; validate and bound imported context, and preserve its provenance without treating trace IDs as credentials |
| Tampering | Injected fields, altered classifications, replayed records or manipulated aggregation can falsify signals. Use structured validated records, safe diagnostic rendering and protected configuration; distinguish observed facts from external claims, preserving attempt and revision scope |
| Repudiation | Missing, sampled, suppressed or overwritten records can obscure attribution. Define which security events are observable, their attribution limits and collection-loss indicators. Optional local telemetry is not durable audit, proof of completeness or cryptographic non-repudiation |
| Information disclosure | Text, paths, identifiers, timing, activity rates and cross-workspace correlations can leak through any MELT form or health endpoint. Minimize at source, restrict access and propagation by authority/disclosure scope, and govern export and retention; aggregate data is not automatically safe |
| Denial of service | Event floods, adversarial label cardinality, expensive formatting, exporter stalls and diagnostic recursion can consume editing resources or blind monitoring. Bound producer work, field sizes, queues, dimensions and export; expose overload/loss through a bounded independent status path rather than recursive logging |
| Elevation of privilege | Diagnostic probes, configuration or automated reactions can become routes to file/network access or mutation. Keep observation read-only by default, authorize effectful probes and configuration changes, and require any remedial action to enter the ordinary validated capability-controlled operation path |

Golden-signal analysis must consider adversarial manipulation as well as ordinary
operating conditions:

- **Latency:** identify measurement provenance, clock boundaries and missing or
  censored completions. Completed-only samples must not conceal stalled work;
  report pending age and timeout/cancellation counts separately.
- **Traffic:** preserve offered/admitted/rejected and logical/attempt distinctions
  under floods, replay and retry amplification. Document collection loss and
  sampling so an apparent traffic reduction is not mistaken for recovery.
- **Errors:** prevent attacker-controlled messages or labels from redefining
  trusted outcome classes. Distinguish denied attacks, service failures and
  monitoring gaps; neither a denial spike nor a quiet dashboard proves compromise
  or safety on its own.
- **Saturation:** include the observation pipeline's own resource pressure and
  its interference with editor work. Missing or stale health observations mean
  unknown visibility, not zero utilization or healthy capacity.

Extend the section 12.5 coverage map with applicable STRIDE threats, protected
assets, trusted signal sources, mitigations, detection limits and verification
scenarios. Detection supports but does not replace preventive validation and
authorization. Sampling and privacy choices must state what evidence they lose;
no full STRIDE-by-signal Cartesian product or monitoring deployment is mandated.
V1 covers the local instrumentation and host boundary; future collectors,
federation and durable audit require their own extended threat model.

References: [Microsoft STRIDE categories](https://learn.microsoft.com/en-us/azure/security/develop/threat-modeling-tool-threats),
[OpenTelemetry security](https://opentelemetry.io/docs/security/) and
[context-propagation security](https://opentelemetry.io/docs/concepts/context-propagation/).
These inform threat analysis, not a claim that telemetry alone establishes a
security guarantee.

### 12.7 Instrumentation execution boundary — agreed v1 direction

The agreed direction is bounded callback-free capture/accounting in critical paths,
followed by host-controlled draining, formatting and export outside authoritative
publication. A host may drain after calls or on its executor; no mandatory thread,
async runtime, telemetry SDK or network backend is added to the core.

| Execution model | Strength | Tradeoff |
|---|---|---|
| Invoke arbitrary subscribers inline | Simple integration and immediate observations | Subscriber filtering/formatting may block, panic or reenter while state is changing; an asynchronous exporter alone does not solve this |
| Require a dedicated telemetry worker/service | Moves downstream work away from callers | Adds lifecycle/runtime coupling and still needs bounded capture, shutdown and overload rules |
| Bounded capture with host-controlled drain — recommended | Preserves the publication boundary and executor independence | Diagnostic delivery is not instantaneous or complete; requires explicit loss, ordering and record-lifetime contracts |

- Critical paths include command admission/validation/publication, effect and
  completion admission, and latency-sensitive layout/input/render handling.
  Capture there may use bounded owned primitives, stable codes and vetted local
  accounting; it must not invoke user callbacks, arbitrary Display/Debug/serialization
  methods, dynamic subscriber filters, exporters or cleanup code with unconstrained
  destructors. Do not capture borrowed mutable editor state or raw content snapshots
  merely to format them later.
- Reserve capture storage outside the critical transition or use an explicitly
  bounded non-waiting mechanism. Capture failure/full capacity may drop or coalesce
  optional diagnostics and update bounded loss/health accounting; it must not
  block on a consumer, grow memory without bound or reject/roll back an otherwise
  valid editor mutation. Allocation, locking and reclamation behavior remain
  implementation obligations to verify, not assumed properties of a queue name.
- Authoritative state transitions/results and required effect capacity remain
  independent of diagnostic capacity. A record must describe its actual milestone:
  prepared/admitted is not committed, and dispatched is not acknowledged. Make
  committed records visible only consistently with publication; subscriber failure
  or absent capture cannot revise the already published result.
- Records retain event/milestone provenance, relevant operation/effect/attempt and
  accepted generation context under the disclosure contract. Preserve original
  measurement time/domain separately from enqueue, drain and export observation
  where needed. Delayed delivery is not extra editor execution time, and wall-clock
  proximity is not proof of causality.
- Preserve necessary local sequence/causal relationships without promising global
  delivery order or an event for every transition. Sampling, coalescing and loss
  must not leave a diagnostic consumer entitled to infer that a missing event
  never happened. Exact operation status comes from the outcome/health interfaces;
  it cannot be reconstructed reliably from sampled MELT records alone.
- Drain outside publication, editor-owner mutation guards and renderer critical
  sections. Host callbacks receive only immutable, authorized records; mutation
  requested in response enters a new ordinary validated operation after the prior
  transition, never reenters it. Projection/release policy under sections 15.5-15.6
  still applies to queued records and delivery destinations.
- The host chooses whether to bridge records into tracing/logging/metrics SDKs.
  Such a bridge must not silently route SDK subscriber callbacks back into capture
  paths or install a global process subscriber as a library side effect. A host
  that deliberately invokes its own blocking drain on its UI thread owns that
  scheduling cost; it is not a promise of core publication latency or isolation
  from malicious in-process code.
- Contain recoverable adapter/consumer failures to the diagnostic pipeline and
  expose degraded status through bounded callback-free health/loss accounting.
  Do not recursively log a logging failure, retry exports without bound or fall
  back to stderr/full error formatting with sensitive state. Catching a Rust
  unwind is not containment of process abort, native faults or memory exhaustion.
- Disabling/switching consumers must have bounded retention and explicit drain/
  discard semantics for queued records. Configuration cannot broaden their data
  scope retrospectively; a new destination needs its authorized release policy.
  Shutdown may perform an explicit bounded best-effort drain outside publication,
  report loss/unfinished export and return without a mandatory final flush.
  That does not waive authoritative handling of in-flight non-telemetry effects.
- Define loss/pressure accounting independently of a full event queue: bounded
  counters/status for drops, capacity and consumer health, with reset/overflow and
  observation-freshness semantics specified later. Loss visibility itself obeys
  recipient projection; a per-user diagnostic view is not a grant to see other
  users' event rates. Detailed per-event evidence is not promised under overload.
- Formal assurance compares authoritative transitions/outcomes with capture off,
  successful, full and failed for the same admitted event sequence and controlled
  external inputs. Do not claim identical real-time schedules or zero overhead:
  measure capture cost and downstream interference separately. Independently test
  blocking/panicking/reentrant formatters, slow drains, queue saturation, policy
  change before release, delayed records and shutdown without an available sink.

[Rust tracing subscribers](https://docs.rs/tracing/latest/tracing/trait.Subscriber.html)
and [subscriber layers](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html)
illustrate why emission hooks are an execution boundary, not evidence of deferred
execution. [OpenTelemetry error handling](https://opentelemetry.io/docs/specs/otel/error-handling/)
and [batch processor contracts](https://opentelemetry.io/docs/specs/otel/trace/sdk/)
inform failure isolation, bounded buffering and flush limits. These references
do not select an SDK or prove that arbitrary host callbacks are safe.

### 12.8 Diagnostic data contract — agreed v1 direction

The agreed direction is schema-allowlisted collection with source classification and
separate recipient/destination authorization at release. Redaction downstream
is defense in depth, not permission to first collect arbitrary sensitive state.
V1 requires bounded structured operational diagnostics; richer content capture
is an optional explicitly authorized host diagnostic capability, not a default.

| Collection model | Strength | Tradeoff |
|---|---|---|
| Capture objects freely, redact at export | Easy troubleshooting and flexible sinks | Sensitive data has already entered memory/queues/local logs; missed fields and debug formatting defeat late filtering |
| Capture only aggregate counters | Small data surface | Insufficient attribution for conflicts, effects and protocol faults; aggregates can still reveal private activity |
| Allowlisted structured fields plus controlled richer capture — recommended | Useful correlation while minimizing collection/disclosure | Requires maintained field schemas, classification, scope/retention and independent leakage tests |

- Each diagnostic schema must declare stable event/metric identity, field type,
  source/provenance, collection purpose, sensitivity/disclosure scope, size and
  cardinality limits, retention and permitted destinations. Operational detail,
  data sensitivity and delivery authority are separate axes; a debug severity
  or development build does not grant access to private data.
- Default operational records may use bounded stable action/reason classes,
  phase/outcome, durations, queue/size buckets and approved opaque correlation
  fields. Content-free is not public: counts, timing, identifiers and linkable
  activity still require the relevant projection under sections 15.5-15.6.
  Full operation/document/view identifiers belong only in authorized bounded
  event/trace context, not unbounded metric dimensions.
- Exclude source/comment bodies, old/deleted text, selected/pasted/search text,
  quoted anchor context, proposed edit payloads, dependency previews, resource
  locators, raw author claims and configuration values from default capture.
  Configuration/grammar/query names and custom command identifiers may contain
  private/user-controlled strings; registration does not automatically make
  them safe label values. Replace them with approved classes/handles where needed.
- Credential, token, key and authentication-secret fields must not be diagnostic
  payloads. Arbitrary captured content can itself contain embedded secrets;
  masking/scanning cannot certify their absence. Any optional content-capture
  profile must state that risk, minimize requested fields and remain subject to
  host disclosure policy rather than promise a universally safe support dump.
- Enforce the accepted schema at capture. Unknown fields or values outside the
  permitted type/size domain are omitted/rejected with bounded safe diagnostic
  accounting, not serialized reflexively to explain the rejection. This does
  not change the authoritative input/error or turn an accepted operation into
  a failure. Schema resolution/authorization must not call arbitrary callbacks
  inside the capture path established by section 12.7.
- Bound field names and values, event-name vocabularies and total metric-series
  growth across reconfiguration, not just current registry size. Use stable
  approved dimensions/buckets; map unexpected or excessive diagnostic categories
  to a safe bounded class or drop them. Do not create per-path, annotation-ID,
  policy-generation or newly registered command metric series without an explicit
  bounded policy. Sampling is not a cardinality or confidentiality control.
- Separate externally supplied trace/baggage context from trusted identity and
  locally established correlation. Accept/propagate only explicitly allowed,
  bounded fields at each boundary; never automatically forward baggage to files,
  subprocess environments, model/tool services or export destinations. Scope
  correlation identifiers to their intended audience; hashing a predictable
  path/identity is neither anonymization nor a substitute for access control.
- Render records safely for each sink, including terminal controls, newlines,
  delimiters and markup. Preserve structured meaning without treating untrusted
  strings as templates, commands or links to execute. Default public error and
  debug summaries must not recursively dump command arguments, source snapshots,
  inverse history or arbitrary host error objects. Formatting outside publication
  is necessary for execution safety, not sufficient for privacy.
- Richer capture, when supplied, requires an explicit host-established grant
  naming data classes, resource/range scope, purpose, recipient/destination,
  duration and storage limits. Enabling it must not retroactively enrich queued
  records or authorize old protected snapshots. Expiry/revocation stops new
  collection and unauthorized queued release; retained records follow explicit
  retention/access policy and cannot be promised erased from already exported copies.
- Release may further narrow a captured record but cannot infer missing private
  fields merely because a sink now requests more detail. Revalidate current
  projection/destination authority under section 15.5. Local-file, console,
  diagnostic UI and remote export are distinct destinations; a network export
  remains host-controlled and off unless explicitly configured. Collector-side
  filtering does not replace source minimization or authorize a new destination.
- Support bundles, diagnostic snapshots and application-controlled crash reports
  use the same contract, with explicit inventories and safe defaults; they must
  not silently archive workspace files, comment stores, environment variables
  or full process memory. OS/native crash dumps and trusted-host memory inspection
  lie outside the library's prevention guarantee and require a separate host
  policy. No mandatory crash collector is introduced into v1.
- Verify classification and projection with independent synthetic canaries in
  source, comments, anchor quotes, old history, configuration, arguments and host
  errors. Test formatting/trace propagation, schema evolution, field/series floods,
  changed permissions and sensitive-mode expiry across queues and sinks. Tests
  must inspect captured records as well as final exports; a redacted exported
  fixture alone does not prove sensitive data was never collected.

References: [OpenTelemetry data minimization](https://opentelemetry.io/docs/security/handling-sensitive-data/)
and [context propagation security](https://opentelemetry.io/docs/concepts/context-propagation/)
inform classification and propagation boundaries;
[OWASP logging guidance](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
informs safe event construction, exclusions, sink encoding and failure/leakage
testing. These do not certify arbitrary content capture or select an SDK.

## 13. Quality requirements

### 13.1 TDD and BDD development practice

- Development should follow a model-review-test-implement-refactor sequence.
  Before writing a test or production change, update and review the relevant
  portion of the digital twin, including its expected behavior and failure
  modes.
- After the model is coherent, follow test-driven development: express a failing
  test for the modeled behavior or defect, make the smallest coherent production
  change that passes, and refactor while the model and suite remain valid.
- User-visible capabilities should be specified with behavior-driven scenarios
  that describe observable outcomes rather than implementation details.
- BDD scenarios should cover equivalent behavior across command invocation
  paths, including bindings, command prompts, palettes, host APIs, and
  accessibility actions where applicable.
- Acceptance scenarios should be shared across the core and frontend conformance
  suites wherever the same behavior is promised by both `iced` and `ratatui`.
- Bug fixes must include a regression test that fails for the reported defect.
- Tests must be deterministic by default. Time, scheduling, task completion,
  file services, clipboard services, and other nondeterministic dependencies
  should be controllable through test hosts and synchronous executors.
- Test intent and externally observable behavior should remain clear even when
  lower-level unit, property, fuzz, concurrency, or performance tests are also
  required.
- Tests and BDD scenarios must be traceable to the modeled component,
  interaction, requirement, risk, or threat they verify.

### 13.2 Test coverage and verification

- Core behavior must be testable without a window or terminal.
- Tests should cover Unicode, revision races, command equivalence, undo grouping,
  multi-selection edits, coordinate conversion, and stale async results.
- Property-based and fuzz testing should be used for edit sequences, positions,
  parsers, and undo/redo invariants where practical.
- Frontend adapters should have conformance tests against shared behavior.
- Performance benchmarks should cover typing latency, large-file opening,
  scrolling, parsing, highlighting, and search.
- Benchmarks and stress tests should capture the golden signals and retain
  comparable results so regressions in tail latency, error rate, throughput, or
  saturation can be detected during development.
- Tests should verify instrumentation correlation across asynchronous work and
  ensure sensitive document content is absent from default telemetry.
- The project should define latency and memory budgets before performance claims
  are treated as acceptance criteria.
- Public APIs must be documented with threading, ownership, invalidation, and
  error semantics.
- Unsafe Rust should be avoided unless justified, isolated, and tested.

### 13.3 Comparative editor references

Architectural and behavioral comparisons must include Helix, Neovim, and GNU
Emacs alongside CodeMirror and VS Code where relevant. Use upstream source,
official documentation, and identified versions or revisions as evidence.

Compare observable semantics and design tradeoffs, not only feature presence.
Relevant areas include text storage and snapshots, transactions and selections,
markers/anchors, undo, commands and keymaps, document/view lifetimes, rendering,
tree-sitter integration, file safety, remote interfaces, and extensibility.
Distinguish mechanisms that fit Vellum's constraints from assumptions tied to an
editor's runtime, compatibility commitments, or interactive model. Reference
behavior does not override the closed operation algebra or expand v1 scope.

Storage evaluation must include Helix's rope-based design, Neovim's memline
block tree, and Emacs's gap buffer, together with the rope and piece-tree
alternatives. During requirements/design, compare their contracts and tradeoffs
without running implementation experiments or selecting a dependency. Benchmark
representative Vellum workloads in the later implementation/evaluation phase
before final dependency selection; popularity alone is not an acceptance
criterion. Section 19 records the architectural comparison and review findings.

Prefer established mechanisms and unsurprising observable behavior. For a
material departure, record the reference baseline, demonstrated agent/user value,
additional complexity and assumptions, failure/recovery behavior, and evidence
needed to justify it. Do not replace agent-safe explicit targets, typed results,
bounded concurrency or atomic change guarantees with implicit focus, key replay
or mandatory human prompts merely to imitate an interactive editor. Novelty
without a concrete benefit is not an architectural goal. Layer simple common
operations over rigorous internals; users/agents need not assemble every internal
protocol dimension for an ordinary edit or save. See section 19.6's sanity review.

### 13.4 Layered assurance and evidence boundaries

Use complementary verification layers rather than treating BDD, code coverage,
the digital twin or a model checker as a sufficient correctness oracle. Choose
the depth of evidence according to the consequence of a violated contract;
atomic publication, authority boundaries and persistence evidence require
stronger assurance than cosmetic presentation choices.

| Layer | Required evidence and limits |
|---|---|
| Design/protocol model | Review invariants and explore bounded state/event interleavings for transaction publication, effect lifecycle and supervision. Record bounds and environmental assumptions; model success alone does not prove implementation conformance |
| Pure core and generated sequences | Check edit/mapping/history laws, Unicode boundaries, all-or-nothing rejection, revision freshness and bounded operation transitions using examples and generated sequences. Preserve seeds and minimized reproductions without private text |
| Interface and pathway conformance | Exercise shared semantic scenarios through public invocation paths, coordinator/worker/host interfaces and both adapters. Compare authoritative text, selection, revisions, effects and outcomes; do not require identical pixels or platform capabilities |
| Real integration | Verify representative native input, clipboard, filesystem, terminal and rendering behavior against actual adapters/services. Controlled hosts cannot establish OS-specific atomicity, accessibility or display guarantees |
| Adversarial and resource behavior | Inject dependency failure, delayed/duplicate/reordered completion, queue pressure and hostile inputs; verify STRIDE mitigations, bounded degradation, MELT correlation and golden-signal attribution along pathways |

- Separate safety (what must never happen), progress/liveness (what must
  eventually happen under stated assumptions) and resource/performance claims.
  External availability and scheduler fairness must not be silently assumed;
  bounded editor response to an unresponsive dependency is distinct from that
  dependency eventually completing its work.
- The closed verb set permits exhaustive checks of supported control-state/event
  combinations under a stated configuration. It does not make arbitrary text,
  identifiers, concurrent schedules or host behavior finite. State-space
  abstraction and exploration limits must be explicit; no blanket completeness
  claim follows from a finite vocabulary.
- Deterministic tests must allow controlled scheduling and interleaving, not
  only sequential execution. Complement these with actual concurrent integration
  checks where ownership, publication or cancellation crosses threads; a passing
  stress run is evidence, not proof of race freedom.
- Use independent expectations for critical laws and outcomes. Where practical,
  compare optimized operations with a deliberately simple reference model;
  generating implementation and assertions from the same unchecked rule can
  reproduce the same defect twice. Metamorphic laws must state preconditions:
  undo may restore text while producing a fresh revision, not restore every
  aspect of prior system state.
- Reference-editor comparisons are evidence about specific compatible semantics,
  not universal differential oracles. Pin compared versions and normalize
  intentional differences in coordinates, selection policy and undo behavior.
- Preserve reproducible evidence linking requirement, model assumption, scenario,
  threat, implementation revision and applicable environment. Mark unsupported,
  unverified, deferred and failed claims distinctly; test count or line coverage
  alone cannot discharge a behavioral or security requirement.
- The specification pass must define risk-based acceptance gates and evidence
  retention, including how known failures or platform limitations are reported.
  Requirements/design work does not itself require building a test harness,
  selecting verification tools or running performance experiments.
- Derive critical assertions from authoritative outcomes and state, not solely
  from logs or trace presence. Verify both that enforcement survives disabled
  instrumentation and that enabled instrumentation observes the intended
  pathway/interface milestones without unauthorized disclosure. A passing
  telemetry assertion alone cannot prove atomicity, persistence or authorization.

References: [Lamport on specification validation and satisfaction](https://lamport.azurewebsites.net/pubs/spec-book-chap.pdf)
supports separating model analysis from implementation refinement.
[Neovim's testing layers](https://neovim.io/doc/user/dev_test.html),
[Helix integration-test helpers](https://github.com/helix-editor/helix/blob/master/helix-term/tests/test/helpers.rs)
and [Emacs ERT](https://www.gnu.org/s/emacs/manual/html_node/ert/index.html)
provide editor precedents for functional, state-oriented and regression evidence,
without selecting their runtimes or harnesses for Vellum.

### 13.5 Formal-methods and Cleanroom direction

Adopt Cleanroom-inspired defect prevention: precise externally observable
specifications, explicit state models, correctness arguments for incremental
refinement, and verification expectations derived independently from the
implementation. Combine this with the TDD/BDD practice in section 13.1; do not
claim conformance to the full historical Cleanroom process.

- Describe behavior first as inputs, outputs, effects and assumptions; introduce
  abstract state and implementation structure as separate refinement steps.
  For high-risk transitions, record preconditions, postconditions, preserved
  invariants and termination/resource arguments. State whether an argument is
  reviewed reasoning, bounded model checking or a machine-checked proof.
- Keep authoring, correctness review and acceptance verification logically
  distinct. Independent reviewers or separately derived oracles are preferred
  for critical contracts; shared tooling or generated code must disclose common
  assumptions. This does not mandate separate organizations or agent teams.
- Usage-based reliability evaluation must use an explicit operational profile
  covering human and agentic workloads, bursty multi-edits, service delays and
  supported environments. Keep representative statistical sampling distinct
  from adversarial and boundary testing; neither substitutes for the other.
- Any statistical reliability claim must state the profile, sample procedure,
  exposure unit, failure definition, uncertainty and applicability limits.
  Zero observed failures is not a proof of zero failure probability; the
  specification pass must define the evidence needed before making such claims.
- Maintain provenance for reference ideas, specifications, code, grammars,
  queries and test assets. Architectural comparison is not authorization to copy
  implementation or fixtures. Record intentional reuse and its applicable
  license obligations separately from independently authored work.
- Cleanroom reliability engineering and source-isolated clean-room
  reimplementation are different concerns. Existing upstream-source inspection
  must not be relabeled as source isolation. If an isolated reimplementation is
  later required, define its scope, information boundary and independent roles
  before that work; no such isolation or legal assurance is claimed here.

Reference: the [SEI Cleanroom Software Engineering Reference](https://insights.sei.cmu.edu/library/cleanroom-software-engineering-reference/)
combines mathematical specification/correctness verification with statistical
usage-based testing. It informs this direction without selecting tooling or
superseding the assurance limits in section 13.4.

## 14. System digital twin and design assurance

### 14.1 Living system model

The project must maintain a digital twin of the intended system before and
throughout implementation. The twin is a version-controlled, reviewable design
model, not merely a retrospective architecture diagram.

The canonical architectural model is [model.md](model.md). It uses C4-informed
context/deployment/component views, explicit catalogs, execution pathways,
invariants, STRIDE and an interface-level MELT/golden-signal map. Its status and
evidence ledger distinguish agreed, proposed, deferred and unverified behavior;
creating the model does not resolve pending design choices or verify software.

At minimum, it must describe:

- Components, crates, processes, threads, workers, frontends, host services,
  external systems, and deployment boundaries.
- Dependency maps showing allowed direction, optional dependencies, ownership,
  lifecycle, version constraints, and prohibited coupling.
- Public and internal interfaces, commands, effects, completions, events, data
  contracts, error contracts, compatibility expectations, and capability
  requirements.
- Network paths, protocols, listeners, clients, endpoints, connection lifecycle,
  encryption and authentication expectations, and offline behavior.
- Human, application, process, extension, document, view, task, and service
  identities, including how each identity is created, authenticated where
  applicable, authorized, correlated, expired, and prevented from being
  confused with another identity.
- Data-flow diagrams showing sources, destinations, transformations, storage,
  caching, replication, retention, deletion, classification, and trust-boundary
  crossings.
- Control flow for commands, transactions, asynchronous effects, cancellation,
  failures, retries, recovery, shutdown, and frontend presentation.
- Resource and concurrency models covering queues, locks or ownership,
  snapshots, backpressure, scheduling priorities, and memory limits.
- MELT instrumentation and golden-signal coverage along end-to-end pathways
  and at material interfaces, including causal handoffs, failure branches,
  resource limits and observation gaps, following section 12.5.
- STRIDE threats, mitigations, residual risks, and verification evidence attached
  to the components and flows they affect.

### 14.2 Model-driven design process

- New capabilities and material changes must begin with a proposed update to the
  digital twin before tests or implementation are written.
- Design review must use the twin to identify circular dependencies, leaky
  abstractions, confused-deputy risks, ambiguous ownership, identity confusion,
  unsafe data movement, missing failure handling, unbounded resources, and
  observability gaps.
- Alternatives and rejected designs should be recorded when the decision has
  lasting architectural, compatibility, performance, or security consequences.
- Expected normal, degraded, adversarial, startup, recovery, and shutdown paths
  must be walked through against the model.
- The model must distinguish current, proposed, and deferred behavior so that it
  does not claim capabilities the implementation does not provide.
- Approved model elements should have stable identifiers that requirements,
  decisions, scenarios, threats, tests, metrics, and code modules can reference.
- An implementation change is not complete until the implemented system, tests,
  and digital twin agree. A design-only change may complete with implementation
  and evidence explicitly pending; it must not be labeled implemented or verified.
  Drift detected in either direction must be resolved explicitly.

### 14.3 Representation and validation

- The canonical twin should use text-based, diffable, reproducible source files
  and generated views where practical.
- Diagrams should be generated from or checked against canonical model data to
  reduce undocumented drift.
- The model should support automated validation of identifiers, references,
  dependency rules, interface compatibility, flow completeness, and required
  security or observability annotations where practical.
- Model validation must run before test and build validation in continuous
  integration.
- Generated artifacts must be reproducible and must not require proprietary
  tooling for contributors to inspect them.
- Sensitive deployment details, credentials, and secrets must never be embedded
  in the twin.

### 14.4 Model fidelity and claim boundaries

The digital twin is a design/assurance model, not a claim of a complete executable
replica, production-state synchronization or formal verification of the library.
Different views may describe topology, data flow, protocol behavior and resource
constraints; their shared identities and contracts must remain consistent.

- Separate normative contracts, environmental assumptions and observed evidence.
  In particular, host promises about conditional writes, effect idempotency or
  identity must not become unconditional editor guarantees.
- Formalize the highest-risk bounded protocols where it materially improves
  assurance. Record abstractions, fairness assumptions, checked properties and
  exploration bounds; select notation and tooling in the specification pass.
- Link concrete implementation transitions and interface outcomes back to the
  modeled contracts. Model validation and implementation conformance are separate
  obligations, following section 13.4.
- Counterexamples, test failures and operational observations must inform model
  review. Do not weaken a normative invariant merely to match a defective
  implementation; changes to intended behavior require an explicit decision.
- Keep assurance proportional: update the affected contracts and linked views,
  not unrelated model detail. No remote simulation, live telemetry backend or
  deployment inventory is added to v1 by the term digital twin.
- Model composition must expose boundary assumptions: which component supplies
  each required guarantee, which observations support it, and what happens when
  it fails. Locally valid component models do not automatically prove an
  end-to-end property; shared identities, event ordering, limits and trust
  assumptions must agree along the composed pathway.

### 14.5 Formal execution and model/requirements feedback

Use Mermaid for architectural/dynamic views, TLA+ for the explicitly abstracted
execution/evidence kernel, and RDF/Turtle with SHACL for typed structural
traceability/coverage. These are design/assurance assets under `formal/`, not
production library dependencies. They supplement the canonical [model.md](model.md).

- **REQ-FM-01 — closed dispatch:** each accepted profile declares finite verb,
  event and control vocabularies with a total disposition function. Unsupported
  inputs reject safely; unexpected internal protocol cases fault their scope.
  Closure is relative to that profile and abstraction, not all editor inputs or
  a promise that arbitrary host code terminates.
- **REQ-FM-02 — decreasing execution rank:** each admitted finite execution must
  have a well-founded rank/budget. Every nonterminal processing step, including
  retries, duplicate/invalid-event handling, polling/ticks and child/restart work
  charged to that execution, consumes its rank or terminates. Recursion, task
  creation and reconfiguration cannot refill it implicitly. Exhaustion stops
  further control execution with a structured outcome; it cannot discard or
  falsify evidence of previously dispatched effects.
- **REQ-FM-03 — service assumptions:** progress in real time additionally requires
  bounded host service/deadline delivery and finite/cooperative processing or
  supported isolation. Separate a finite number of model transitions from a
  wall-clock guarantee. Reconciliation of late/unknown writes remains a separately
  bounded obligation, not an exception that secretly resumes an exhausted run.
- **REQ-FM-04 — scoped evidence algebra:** define immutable observation facts for
  one fixed effect/attempt/scope; retain a finite powerset domain with subset
  order, union join and intersection meet where appropriate. Contradictory
  receipts remain evidence that requires reconciliation. Removing contradictory
  elements or merging facts from unrelated attempts must not silently destroy
  closure, provenance or correctness. Do not call the whole execution state a
  lattice merely because this evidence abstraction is one.
- **REQ-FM-05 — categorical claim scope:** name objects, morphisms, composition,
  identities and the limits/colimits actually claimed. A free category of valid
  execution traces provides compositional structure, not automatic categorical
  completeness; incompatible terminal outcomes need not have a common terminal
  object. The finite evidence poset can form a complete lattice/thin category
  without making business/execution states complete. Do not add meaningless
  success/failure merges simply to obtain a mathematical closure claim.
- **REQ-FM-06 — structural evidence:** RDF/SHACL validation establishes declared
  graph constraints, such as typed interface endpoints, boundary/instrumentation
  links and acyclic architectural dependencies. It cannot prove that omitted
  components/threats do not exist or that an implementation conforms. Separate
  generated textual references from manually reviewed semantic relationships.
- **REQ-FM-07 — reproducible feedback:** keep checked model/configuration, bounds,
  tool versions/hashes, positive results and expected-negative counterexamples.
  Link each counterexample to an invariant/requirement and record whether it
  changes intended behavior, the abstraction, or a future implementation. Update
  requirements, model, formal artifacts and evidence together; stale evidence
  cannot count as a current proof. Proof sketches, bounded checking and
  proof-assistant-checked theorems must have distinct evidence status.
- **REQ-FM-08 — non-vacuity and sensitivity:** pair safety/control checks with
  reachable useful-success and relevant failure/unknown-outcome witnesses.
  Reject-all or exhaust-all behavior cannot stand in for the required capability.
  Check consistency of proof premises where supported, and require deliberately
  weakened controls to violate the named property; syntax/type/tool failures or
  an unrelated counterexample do not satisfy that expectation. Witness existence
  does not establish universal progress or scheduler fairness.
- **REQ-FM-09 — assumption discharge and composition:** each proof slice names
  the supplier and verification obligation for assumed identity, policy, storage,
  execution and disclosure guarantees. State explicit refinement relations and
  pre/postcondition bridges before composing transaction, history, annotation,
  persistence and recipient-observation claims. Shared names, graph links and
  individually passing models are not an end-to-end proof. Solver-checked
  verification conditions must identify their encoding/trusted solver and remain
  distinct from independently checked proof certificates or implementation proof.

These requirements authorize executable design validation, not editor code,
performance experiments or a production dependency choice. The initial kernel
is deliberately smaller than the full public command catalog; refinement and
additional models are required before broader correctness claims.

The current slice contracts, conditional arguments and residual obligations are
recorded in [formal/slices.md](formal/slices.md), with executable bounds/checks in
[formal/slices.mjs](formal/slices.mjs). Model-only steps and horizons are not
production budget choices. Outstanding cross-slice bridges, concrete text/anchor
and history refinement, concurrent same-artifact save/receipt ordering, host
guarantees and independent verification remain specification/assurance work;
passing a slice does not silently resolve them. A restricted shared-state bridge
now checks publication/history/mapping coherence, private-comment preservation,
published effect capture, recipient projection and imported execution-kernel
steps. This is bounded reference-model evidence, not a concrete representation
choice, full observable refinement of all slices, or inherited executor fairness.

References: [TLA+ refinement](https://lamport.azurewebsites.net/tla/proving-safety.pdf),
[TLAPS proof checking](https://tla.msr-inria.inria.fr/tlatoolbox/doc/prover/prover.html),
[RDF concepts](https://www.w3.org/TR/rdf11-concepts/) and
[SHACL validation](https://www.w3.org/TR/shacl/).

## 15. Security and STRIDE assurance

### 15.1 Threat modeling

- The project must maintain a threat model covering the editor core, frontend
  adapters, host-service boundary, background workers, extensions, grammars,
  language servers, file access, persisted state, and telemetry export.
- Trust boundaries, protected assets, entry points, privileged operations, and
  attacker-controlled inputs must be identified before security-sensitive
  capabilities are implemented.
- STRIDE analysis must be performed against the digital twin's components,
  identities, interfaces, network paths, data stores, and data-flow boundaries.
- Include both security observations along pathways and threats against MELT,
  golden-signal measurement and health reporting themselves, following section
  12.6. Link this analysis to the pathway/interface coverage map in section 12.5.
- Threat analysis must use STRIDE: spoofing, tampering, repudiation, information
  disclosure, denial of service, and elevation of privilege.
- Each applicable threat must have a documented mitigation and one or more
  verification methods. Categories judged not applicable must include a brief
  rationale rather than being silently omitted.
- The threat model must be reviewed when a change adds or moves a trust boundary,
  parses a new untrusted format, launches a process, accesses the file system or
  network, loads an extension or grammar, or exports telemetry.
- State adversary powers and trusted computing assumptions explicitly. Local
  capability checks constrain supported editor interfaces; they do not sandbox
  the embedding application, malicious in-process Rust or native grammar code.
  Do not claim isolation from a compromised authority that owns the process.
- For each material boundary, distinguish prevention, detection, containment and
  recovery obligations. A formal claim must state its modeled adversary and
  assumptions; neither a clean STRIDE checklist nor a model-checking result is
  evidence against threats excluded from that model.

### 15.2 STRIDE-derived tests

Automated tests must cover applicable STRIDE threats, including at minimum:

- **Spoofing:** forged or mismatched document, view, revision, task, extension,
  or language-service identities cannot impersonate a trusted source.
- **Tampering:** malformed, stale, reordered, or adversarial commands, effects,
  completions, persisted data, grammar data, and protocol messages cannot bypass
  validation or corrupt authoritative editor state.
- **Repudiation:** security-relevant actions and failures expose attributable,
  correlated outcomes/events within the stated diagnostic scope without recording
  sensitive text. Verify collection-loss visibility and attribution limits;
  optional telemetry must not be represented as complete durable audit evidence.
- **Information disclosure:** logs, metrics, traces, errors, crash reports,
  clipboard handling, and extension APIs do not expose document contents or host
  secrets outside their authorized boundary.
- **Denial of service:** adversarial text, syntax trees, queries, event floods,
  command floods, oversized inputs, deep nesting, long lines, and nonresponsive
  workers are constrained by cancellation, time, memory, queue, and recursion
  limits where applicable.
- **Elevation of privilege:** requests through supported command, extension,
  grammar/language-service and host-service interfaces cannot acquire capabilities
  not explicitly granted by the embedding host. This does not claim a sandbox
  against trusted in-process code bypassing those interfaces.

STRIDE coverage should combine unit, integration, property, fuzz, concurrency,
and resource-exhaustion tests as appropriate. Security regression tests must be
added for every confirmed vulnerability. Security tests must themselves avoid
embedding real secrets or sensitive document contents in fixtures and output.

### 15.3 Secure defaults

- Untrusted input must be validated at every trust boundary.
- Capabilities should be denied unless the host explicitly supplies them.
- Capability discovery or UI enablement is not an authorization decision. Check
  current authority at mutation admission/commit and before effect dispatch as
  required by the effect contract; changes after dispatch cannot be represented
  as having recalled an external effect. Authentication services remain optional.
- Retained content and historical access grants do not authorize new reads or
  disclosures. Section 15.5 establishes authorization at bounded release boundaries,
  including pending results and subscriptions; already delivered copies and
  trusted in-process memory access are explicit enforcement limits.
- Resource limits must fail predictably without corrupting recoverable state.
- Security-relevant failures must expose structured outcomes/health independently
  of optional telemetry, with MELT hooks under sections 12.4-12.6. Disabled,
  sampled or overloaded instrumentation must not weaken enforcement or imply
  complete incident evidence.
- Dependencies that parse or execute untrusted input must be reviewable and
  replaceable, with versions governed by the project's compatibility and update
  policy.

### 15.4 Local identity and provenance boundary

V1 uses explicit host-established access contexts and current local capabilities.
Document/version provenance, caller identity, authorization and operation
correctness are distinct; authentic content is not an instruction-execution grant.
Remote identity, federation and delegation designs have been removed. Sections
15.5–15.6 govern current local release and field projections.

### 15.5 Read/disclosure lifecycle — agreed v1 direction

The agreed direction is to gate new reads/disclosures using current accepted host policy,
while keeping internally retained immutable state distinct from delivery to a
recipient. V1 needs an explicit local access context and release contract, not
a network identity service, external policy engine or untrusted-code sandbox.

| Access model | Strength | Tradeoff |
|---|---|---|
| A snapshot/handle permanently carries its original grant | Cheap direct access and simple lifetime | Cannot stop new reads through that handle after revocation; overstates current-policy enforcement |
| Recheck every scalar access | Fine-grained mediated control | High overhead, weak fit for parsing/chunk APIs, and cannot recall bytes already returned |
| Gate bounded releases; treat unrestricted snapshots as explicit disclosure — recommended | Fits snapshot-based workers and local capability enforcement with honest revocation limits | Requires recipient/purpose scope, delivery ordering and distinction between trusted memory and mediated interfaces |

- A read/query/subscription/export request must bind a host-established access
  context, operation, resource/document incarnation, requested revision/range or
  historical scope, data class and destination/disclosure purpose where relevant.
  Caller-provided author labels or tracing context must not establish identity
  or permission. An embedding may have one trusted local context; it need not
  implement user accounts to state which interface grants it supplies.
- Authorize source text, historical/deleted content, annotations, derived results,
  metadata and outbound disclosure according to the applicable host policy.
  Source-read permission is not automatically permission to read private comments,
  inspect old secrets in undo history, or send source to a model/tool/clipboard.
  Do not require a particular role taxonomy; publish the distinctions that the
  supported policy profile enforces rather than imply a sandbox against the host.
- Creation authority does not imply result-disclosure authority. A create-result
  grant may cover only this invocation's one newly created annotation within its
  exact document and recipient context; the engine binds it to the original
  allocation. Preserve a pre-admission field ceiling and current release checks.
  It grants no access to existing annotations or another invocation's creation.
- Retention keeps data available to its owner under explicit obligations; it does
  not preserve an old recipient grant. Cached text, saved snapshots, inverse data,
  layouts and worker results require current authorization before new mediated
  delivery. A raw snapshot or unrestricted memory reference handed to a trusted
  host/worker/caller is already access to its contents, not a promise that every
  subsequent dereference will be reauthorized.
- Define a local ordering/linearization point for each bounded release decision
  against accepted policy and lifecycle state. Requests admitted before revocation
  may still be pending: discard or reproject their queued recipient payload before
  a later release if no longer authorized. Policy change must not require network
  I/O inside publication. Externally supplied decisions/leases need an explicit
  validity/freshness contract; unavailable required evidence fails closed.
- Subscription admission is not a perpetual grant. Recheck at bounded delivery
  batches or an explicitly authorized bounded lease boundary, including policy,
  incarnation and requested data class. Revocation, expiry or disposal stops
  subsequent unauthorized delivery and releases/cancels work where appropriate;
  it need not erase the owner's authoritative state or useful permitted work.
  Regrant cannot silently resume an old stream across a gap or new incarnation.
- Selected v1 scope (2026-11-06): explicit observation revocation withdraws one
  exact originally issued context's authorization. It requires separate current
  revocation authority and native scope/control validation. Absorbing closure may
  share implementation with stop, but stop permission alone does not authorize
  revocation. Sibling contexts and independently authorized future admissions are
  unaffected. No grant-wide policy withdrawal or recall of delivered bytes is
  promised; those would require a separately specified issuer/cohort contract.
- Separate producer acceptance from recipient release. A late external-effect
  receipt may remain necessary authoritative evidence even when its original
  caller no longer has permission to view the resource. Retain/reconcile that
  evidence at the appropriate trusted operation/host scope and expose only the
  currently authorized projection; do not discard write evidence to enforce
  confidentiality or leak its payload to keep an observer informed.
- Once data crossed an authorized release boundary, revocation cannot make the
  recipient forget it, recall copies, cancel a completed export or guarantee
  secure erasure from allocator/OS/crash state. Minimize copies, bound retention
  and define release/disposal responsibilities, but do not claim memory zeroization
  or deletion from backups without a separate supported host contract. Stronger
  enforcement against recipients requires a separate isolation design.
- Disclosure boundaries include headless APIs, events, history inspection,
  accessibility/surrounding-text reads, clipboard, diagnostic/support export and
  host model/tool integration when supplied. Notifications and metadata are data
  too; their permitted projections must be stated separately from full bodies.
  This rule does not introduce any deferred transport or model service into v1.
- The safety property is: each new mediated release is permitted for its actual
  recipient, data scope, destination and accepted policy/lifecycle state at the
  declared release boundary. It is not that already disclosed content becomes
  inaccessible after revocation. Model request, pending, release, revoke/expire
  and dispose ordering, including retained historical state, with those limits.
- Verify independently that policy changes block queued reads, subscription
  batches, historical export and late result disclosure without losing unknown
  write evidence; verify that immutable snapshot copies are not advertised as
  revocable. Instrument admission/release/denial, policy staleness, pending age
  and retention pressure through bounded safe codes. Do not include denied text,
  inaccessible identifiers or raw author claims merely to explain a denial.

[NIST SP 800-207](https://csrc.nist.gov/pubs/sp/800/207/final) anchors resource-
focused access decisions without location-based implicit trust. The snapshot
release/retention distinctions above are Vellum architectural recommendations,
not a claim that NIST specifies this editor's handle model or certifies it.

### 15.6 Annotation and metadata projections — agreed v1 direction

The agreed direction is to define an authorized projection for each recipient/disclosure
scope, not merely remove private comment bodies from otherwise global responses.
The same principles apply to source/history metadata and diagnostics where those
surfaces cross distinct access scopes. No separate privacy service is required.

| Model | Strength | Tradeoff |
|---|---|---|
| Hide bodies but expose global metadata | Easy shared counts/versioning | Leaks existence, activity, authorship and relationships through metadata |
| Fully isolated annotation stores per reader | Reduces some shared observation | Complicates shared review/ownership and still cannot eliminate all host timing/resource side channels |
| Recipient-scoped projections with explicit operational disclosures — recommended | Fits shared documents, visibility rules and headless/frontend APIs | Requires scoped tokens/caches, filtered events and a declared limit to privacy claims |

- The host policy profile must state permitted existence, body, target/quoted
  context, authorship/provenance, status/time and aggregate/activity disclosures.
  These are data classes to account for, not a mandatory fine-grained permission
  product. Document access or an annotation's descriptive audience/rights field
  is not enforcement of annotation visibility; authority comes from the validated
  host context and policy, not content-controlled labels.
- Enumerations, counts, navigation, search, indicators, accessibility reads,
  history inspection and export operate on the authorized projection. Label a
  count as visible/accessible scope, not a global total; do not expose hidden
  counts, skipped indexes, hidden-presence flags or partial-body placeholders
  unless that existence/aggregate disclosure is explicitly permitted. Complete
  results mean complete for the stated authorized scope, not all stored records.
- Collection generation, pagination cursor and cache/version tokens exposed to
  scoped callers must not reveal global sequence gaps or correlate inaccessible
  activity by construction. Use projection-/scope-aware tokens with explicit
  policy/context validity; they are not permission grants or substitutes for
  internal revision/dependency validation. Exact encoding is later specification.
- Source text and annotation-content/mapping generations remain distinct. A
  hidden-only annotation update must not itself change exposed source revision
  or produce an annotation-specific event for an unauthorized reader. Do not
  attach the raw coordinator generation or an unfiltered participant list merely
  to simplify public cache invalidation or cross-thread correlation.
- Apply projection to events and queued deliveries, not just initial queries.
  Reauthorize and invalidate/rebuild affected recipient caches on policy or
  membership changes under section 15.5. Visibility loss may require removing
  previously displayed content; it must not be described as author deletion or
  include hidden replacement content. Regrant/resync uses current authorized
  state and explicit gap handling, not replay of an unfiltered historical feed.
- Permission-denied, nonexistent, stale and conflicting cases must have an
  explicit disclosure profile. Where existence is not disclosable, use a common
  caller-facing unavailable outcome and avoid names, authors, counts or quoted
  text in explanations. Keep true internal causes/outcome evidence in the
  appropriate trusted scope; a safe public reason must not falsify authoritative
  operation state or turn an unknown external write into a known rejection.
- Atomic undo/edits still validate every real participant and reject rather than
  partially apply. An unauthorized caller must not obtain a hidden dependency
  graph through conflict previews or blockers. Generic blocked/unavailable or
  capacity responses may still reveal operational interference; explicitly
  identify those allowed disclosures and reduce unnecessary detail rather than
  promise both complete atomic transparency and perfect hidden-state secrecy.
- Diagnostic/health APIs, metric series, traces and support export are recipients
  too. Projection does not automatically grant their operators access to private
  annotation identities, author activity or source excerpts. Keep global owner
  observability separate from user-visible observations, with explicit grants,
  bounded field vocabulary and retention. Aggregation/hash substitution alone
  does not establish safe disclosure.
- Caches and derived views must be keyed by a valid disclosure scope/projection
  policy, not only document revision. A richer cached response must not be reused
  for a less privileged recipient. Layout/decorations and native accessibility
  must consume the same authorized annotation projection; invisible bodies with
  visible unauthorized gutter markers are still a disclosure.
- Define a bounded observational property: for fixed recipient policy and equal
  authorized source/annotation projections, changes only to inaccessible records
  do not alter protected functional outputs such as visible bodies, counts,
  identifiers, source revision or annotation-specific events, except explicitly
  declared operational disclosures. Timing, memory contention and trusted-host
  inspection are not claimed constant-time or fully noninterfering in v1.
- Verify this with independent paired-state scenarios: add/edit/delete a hidden
  annotation while comparing permitted outputs; vary visible policy and warm
  cache scope; test quoted anchors, paginated results, source undo conflicts,
  failed saves, revocation and queued event delivery. Include safe metric/log
  outputs and UI/accessibility agreement, while treating owners' authorized
  internal observations as a different projection, not a violation by themselves.

The [W3C Web Annotation Data Model](https://www.w3.org/TR/annotation-model/)
informs annotation metadata but is not an access-control system. The existence-
hiding option in [RFC 9110 section 15.5.4](https://www.rfc-editor.org/rfc/rfc9110.html#section-15.5.4)
is an analogy for safe error projection, not a requirement to use HTTP statuses
inside the library. Disclosure minimization also follows
[OpenTelemetry sensitive-data guidance](https://opentelemetry.io/docs/security/handling-sensitive-data/).

## 16. Compatibility and packaging

### 16.1 Packaging boundaries

- The core, tree-sitter integration, frontend adapters, and standalone apps
  should be separable crates or features so consumers pay only for what they use.
- Optional integrations must not force unrelated runtime or platform
  dependencies into the core.
- Supported Rust, `iced`, `ratatui`, tree-sitter, and platform versions must be
  documented and tested.
- Serialization formats, if exposed for commands or persistent state, must have
  an explicit compatibility policy.
- Published packages must identify the provenance and redistribution terms of
  bundled code, grammars, queries and test/example assets. Optional integrations
  must not acquire new authority, perform downloads or initialize external
  services merely because a feature is compiled or a package is imported.

### 16.2 Behavioral compatibility and extension evolution

Use Rust package/API compatibility plus explicit behavioral contract versions.
Successful compilation alone does not establish compatibility of commands,
effects, authorization, coordinates, history or observable outcomes. V1 exposes
in-process Rust contracts, not a stable native binary-plugin ABI; future wire
protocols and persisted formats require separate versioning decisions.

- Model a supported profile as an explicit set of verbs, schemas, capabilities,
  lifecycle rules and resource/semantic guarantees. Registration may establish
  a new validated profile but cannot introduce unknown verbs during execution.
  Distinguish absent, unsupported and denied capabilities; negotiation never
  confers authority or silently downgrades an operation's required guarantees.
- For an existing supported profile, compatible evolution must preserve its
  documented contract under the same environmental assumptions: do not strengthen
  caller obligations or weaken promised results, safety or progress guarantees.
  Use observational refinement over public states, outcomes and effects, allowing
  internal steps and implementation changes that preserve that contract. Do not
  require identical traces when the contract permits nondeterminism.
- Review changes to coordinate meaning, Unicode/matching profiles, command
  defaults, selection/undo behavior, ordering, persistence milestones, failure
  classes and resource ceilings as behavioral changes, even if Rust types still
  compile. Document intentional breaking changes and migration; security fixes
  may require restricting behavior but must not be mislabeled as transparent.
- Choose extension points deliberately. Keep authority-bearing verbs and control
  states closed within the accepted profile; use explicit version/capability
  changes for additions. Extensible descriptive metadata may allow unknown
  fields only where they cannot alter execution or validation. Rust enum
  exhaustiveness policy is a later API design choice, not permission to accept
  unknown operation semantics.
- Validate host/provider compatibility before activation and associate results
  with the relevant profile/configuration generation. Reconfiguration must define
  treatment of in-flight work; stale completions cannot acquire the new profile's
  authority or be interpreted using incompatible schemas.
- Use adapters only where their mappings preserve required semantics and have
  conformance evidence. Reject unsupported atomicity, cancellation or persistence
  guarantees rather than emulate them with weaker behavior under the same name.
- Maintain a supported toolchain/dependency/platform/feature matrix, including
  minimal headless core and each promised adapter/integration. Independent
  packaging is useful isolation, not proof that arbitrary dependency versions or
  feature combinations compose correctly. State supported combinations and
  downstream-facing dependency exposure explicitly.
- Compatibility evidence must combine downstream compile checks, shared behavior
  scenarios and model/contract review for high-risk changes. Document deprecation,
  support lifetime and migration policies during the specification pass; no
  indefinite compatibility or stable Rust ABI is promised.
- A compatibility claim must identify the profile, assumptions and externally
  observable guarantees preserved, including declared resource and disclosure
  limits. Internal restructuring must preserve pathway/interface observability
  semantics without requiring identical span topology or diagnostic record counts.

References: [Cargo SemVer compatibility](https://doc.rust-lang.org/cargo/reference/semver.html)
informs Rust source/API changes; [specification refinement](https://lamport.azurewebsites.net/pubs/spec-book-chap.pdf)
informs preservation of externally observable contracts. Neither automatically
proves that a particular release or adapter is compatible.

## 17. Identified gaps and unresolved decisions

The product direction and first-release scope are established. The following
decision register records resolved topics and the remaining work needed before
the requirements are implementation-ready.

Read each entry as one of: agreed architectural direction awaiting specification,
an open architectural choice, or a deferred capability whose design is required
only before its introduction. Section 17.14 consolidates the open architectural
choices; an open field here does not reopen approved scope.

Resolving a material decision requires more than selecting a mechanism:

- State its owning contract and affected pathways/interfaces, v1 or deferred
  applicability, dependency decisions and host/environmental assumptions.
- Distinguish normative behavior from representation choices. Record preserved
  invariants, permitted outcomes, progress assumptions and resource bounds;
  identify the abstraction/refinement relationship for formalized protocols.
- Record alternatives and consequences, including compatibility and migration.
  Identify any changed authority, identity, disclosure or failure boundary.
- Identify applicable STRIDE threats and MELT/golden-signal observations along
  affected pathways, including observation gaps and threats to instrumentation.
  A diagnostic event is not itself a preventive mitigation or durable receipt.
- Define the independent review/oracle and verification evidence needed to
  support the decision, with provenance and limits. Architectural agreement,
  completed specification and verified implementation are separate statuses;
  evidence not yet produced must remain explicitly pending.

Use proportional links to the owning requirements and assurance model rather
than duplicating every contract in every entry. A deferred item must not block
v1 merely because its eventual protocol, proof or deployment is unspecified.

### 17.1 Initial product scope — resolved

Sections 1.1 and 1.2 define the approved v1 scope: a library for general text and
code editing, both frontend adapters, and thin reference applications. Initial and
deferred capabilities are listed there. Detailed acceptance contracts remain
open in the topics below.

### 17.2 Text and history representation

- Section 19.1 recommends a persistent chunked text structure, with a rope as
  the leading design candidate. The concrete structure and library are not
  selected; evaluate them during implementation against the snapshot contract.
- Snapshots must preserve exact revision content, be shareable across workers,
  and be inexpensive without mandatory full-document copying. Specify retention,
  reclamation and memory budgets; in-memory persistence is not crash durability.
- Keep authoritative history at the operation/document layer with explicit
  invocation scope, not independent mutable histories per view. Define concrete
  undo grouping, linked local reversal, redo and retention policies.
- Define behavior for extremely long lines and very large files.
- Document/resource identity and lifetime direction is agreed in section 3.7.
  Refine concrete handle and retention contracts, including alias/rebinding assumptions,
  unsaved-state ownership, view closure, disposal and outstanding effects.
- Sections 3.4 and 5.1-5.5 establish workspaces coordinator ownership, agent
  proposals, dependency validation, atomic in-memory application, and persistence
  boundaries. Section 5.7 adopts validated unchanged success for genuine semantic
  no-ops. Resolve concrete protocol types, staging and publication budgets,
  operation-specific identity predicates, linked undo/redo policy, and optional stronger host
  guarantees before implementation.

### 17.3 Coordinates and Unicode

Residual-review direction agreed: retain canonical UTF-8 document-byte edit
coordinates, distinct from serialized file offsets and frontend geometry.
Conversions are typed and context-bound; invalid boundaries are rejected and
ambiguity is explicit, not silently rounded. Exact-revision proposal admission
is the mandatory baseline; optional bounded mapping yields candidate targets
that still require content, dependency and current-authority validation.
Use a small declared set of attachment profiles for caret/insertion affinity,
annotation endpoints/deletion detachment and proposal mapping. A collapsed range
is not evidence of an intact target; source edits preserve annotation bodies,
and undo restoration must respect later explicit reattachment. Concrete profile
definitions and mapping/refinement proofs remain specification work.

- Define concrete public types for canonical UTF-8 byte positions/ranges and
  anchors, with explicit revision context and conversion to other coordinates.
- Specify behavior at invalid UTF-8 boundaries, grapheme boundaries, tabs,
  combining sequences, bidirectional text, and ambiguous-width terminal glyphs.
- Decide the required level of bidirectional-text support for each frontend.
- State the domains and failure behavior of coordinate conversions; specify
  round-trip laws only where mappings are unambiguous. Visual affinity, grapheme
  boundaries and source bytes are not interchangeable coordinate systems.

### 17.4 Command and extension contracts

- Section 16.2 establishes profile-based behavioral compatibility and the v1
  source/API boundary. Specify profile identity, activation/reconfiguration,
  version policy and conformance obligations before implementation.
- Section 4.4 establishes a closed execution vocabulary and fail-fast behavior.
  Define its concrete verb catalog, schemas, state/event dispositions and bounds.
- V1 dynamic invocation is in-process over registered contracts, alongside typed
  Rust APIs. Scripting, binary plugin ABIs and remote invocation are out of scope.
  Section 3.9 records only the exclusion; no future runtime design is retained.
- Define the argument schema, return type, progress model, and cancellation model
  for commands.
- Commands request host work through the typed effect/completion boundary;
  convenience async APIs must preserve its ownership and validation guarantees.
- Define command identifier namespacing, conflict handling, and versioning.
- V1 host extensions use trusted in-process Rust registration. Native grammar
  loading remains a distinct host-controlled boundary under section 6.1.
  Untrusted plugins, a stable binary-plugin ABI and general extension-process
  protocols are out of scope; no design for them is retained.

### 17.5 Concurrency model

- Section 7.4 establishes effect state/evidence separation, bounded retries and
  supervision requirements. Define the concrete evidence algebra, admissible
  events, reconciliation actions, retention and resource budgets during the
  specification pass; no global lattice-closure claim is made.
- Decide which crate owns task scheduling interfaces and cancellation tokens.
- Define snapshot retention limits and memory backpressure.
- Specify which stale results may be rebased and which must be discarded.
- Exact-revision rejection remains sufficient for v1 proposals. Mapping/rebasing
  must not become an implicit requirement while specifying stale-work handling.
- Define dependency validation and freshness tokens across documents, workspace
  membership, configuration, and semantic indexes, including bounded retention
  of the evidence needed to map agent proposals.
- Define ordering when multiple asynchronous completions arrive together.
- Preserve deterministic transition behavior for the admitted sequence, not a
  promise of identical results across every permitted arrival/interleaving.
  Never coalesce away authoritative evidence of an external effect merely
  because its computed UI result is stale.
- Establish measurable responsiveness, throughput, and memory targets.

### 17.6 Rendering contract

- Section 19.2 records the shared-semantics/provider-geometry boundary. Define
  the concrete layout data, measurement and hit-test interfaces without assuming
  proportional glyphs and terminal cells share identical geometry.
- Define decoration ordering, clipping, hit testing, cursor animation, and
  frontend-specific styling escape hatches.
- Section 8.1 establishes agreed shared layout policy with provider-owned shaping,
  bidi, width fitting and coherent geometry. Specify provider interfaces,
  supported profiles and concrete implementation ownership/conformance next.

### 17.7 Language ecosystem

- Sections 6.1 and 6.2 establish the optional service boundary, context/version
  freshness, executor ownership and v1 bounded mixed-language scope. Concrete
  lifecycle states, region mapping/precedence, grammar/query compatibility
  profiles, budgets and cancellation mechanisms remain for the specification pass.
- Define grammar discovery, loading, version compatibility, and distribution.
- Built-in language-server integration is deferred under section 1.2; preserve
  host effect interfaces for future providers.
- Define how highlight, indentation, injection, folding, and locals queries are
  located and overridden.
- Specify behavior when grammars are missing, incompatible, or fail to load.

### 17.8 Platform behavior

- Sections 9.4 and 9.5 establish composition and accessibility direction.
  Complete native focus/commit/cancel behavior, coordinate conversions, secondary
  selection policy details and per-platform capability/verification matrices.
- Define the initial operating-system and terminal support matrix.
- Specify clipboard, IME, dead-key, keyboard-layout, and accessibility behavior
  per frontend and platform.
- Decide how frontend key events are normalized without losing platform-native
  distinctions.

### 17.9 Persistence and collaboration

- V1 load/save and external-change protection are required. File watching,
  external-change merging, crash/session recovery, autosave and collaborative
  editing are deferred under section 1.2.
- Section 5.9 establishes text/resource fidelity and separate saved-state
  baselines. Refine their concrete contracts and state which external-change
  guarantees each host can actually enforce;
  check-then-write detection must not be specified as unconditional isolation.
- Section 5.10 selects single-flight per resolved destination and bounded FIFO
  of exact admitted captures; coalescing is deferred. Specify concrete admission,
  expiry, budgets and token/order capabilities without conflating receipt arrival,
  revision order and storage completion. The agreement and proof comparison do
  not provide actual host guarantees.

### 17.10 Acceptance criteria

- Convert qualitative requirements such as "efficient," "large," and
  "responsive" into measurable targets.
- Define representative corpora, machines, terminals, fonts, and workloads for
  benchmarks.
- Define compatibility guarantees and the stability threshold for the public
  facade.
- Separate correctness/conformance gates, representative usage/reliability
  evaluation, adversarial limits and performance budgets. Define workload and
  environment assumptions before later evaluation; a coverage percentage,
  benchmark average or zero observed failures cannot substitute for these gates.

### 17.11 Instrumentation contract

- Section 12.4 establishes outcome/fault separation, independent health reporting
  and disclosure/overload boundaries. Specify concrete outcome schemas, health
  freshness, presentation mapping and diagnostic-loss behavior in the later pass.
- Refine the completed architecture-wide coverage index under section 12.5.2
  into concrete measurement endpoints and denominators, queue/service latency,
  saturation limits, context handoffs, collection loss and STRIDE mitigations.
  Architectural observation intent is not implemented instrumentation evidence.
- Select the backend-neutral Rust facade and determine whether existing
  `tracing` and metrics ecosystem types may appear in public APIs.
- Define stable names, units, field schemas, correlation identifiers, and
  cardinality limits for MELT signals.
- Establish acceptable instrumentation overhead and which signals are enabled
  in development, release, and diagnostic builds.
- Define golden-signal budgets, tail percentiles, benchmark baselines, and the
  regression policy used in continuous integration.
- Define how reference applications expose bounded local diagnostics. The
  library must not initiate telemetry network export; any host export requires
  explicit configuration and disclosure scope under sections 12.4-12.6.

### 17.12 Development and security assurance

- Sections 13.4 and 13.5 establish layered, formal-methods and Cleanroom-inspired
  assurance. Specify independent review/oracle boundaries, proof obligations,
  operational profiles and evidence criteria without claiming full Cleanroom
  conformance or source-isolated reimplementation.
- Select the BDD scenario format and runner, if any, without requiring consumers
  of the library to adopt that tooling.
- Define which changes require executable acceptance scenarios in addition to
  lower-level tests.
- Establish a test-evidence matrix mapping STRIDE threats and mitigations to
  automated tests, manual reviews, or other verification.
- Define supported resource ceilings and adversarial fixtures for denial-of-
  service testing.
- Decide the security policy, vulnerability reporting process, dependency-audit
  cadence, and minimum supported versions for security fixes.

### 17.13 Digital-twin conventions and tooling

- [model.md](model.md) establishes canonical Markdown catalogs, Mermaid views
  and stable architecture/interface/data/pathway/security/instrumentation IDs.
  RDF/SHACL generation/validation and scoped TLA+/SMT checks are selected for
  design assurance. Specify production/API schema tooling and implementation
  refinement separately; diagrams remain manually maintained catalog views.
- Extend the model's identifier/traceability conventions to individual formal
  requirements, implementation modules, test artifacts and evidence results.
- Define the review threshold for a "material change" and a lightweight path for
  changes that do not affect architecture or modeled behavior.
- Decide which parts of the twin are manually authored, generated from code, or
  verified bidirectionally.
- Define how current, proposed, experimental, deprecated, and removed elements
  are represented.
- Establish model completeness and freshness checks, including how architectural
  drift blocks integration.
- Treat completeness checks as relative to the declared system boundary and
  modeling rules. Reference integrity or generated-view consistency is not
  proof of behavioral completeness or implementation refinement.

### 17.14 Design closure audit and remaining architectural decisions

This audit distinguishes established direction, unresolved architecture and
specification detail. It is not a proof of completeness, implementation
readiness, formal verification or categorical/lattice closure. Requirements
remain a design artifact; no implementation or verification result is implied.

Established boundaries include one authoritative mutation path, local atomic
multi-document publication, revision-bound proposals and derived state, typed
host effects, separate effect evidence, capability enforcement, optional bounded
language services with mixed regions, shared frontend semantics/provider
geometry, and pathway/interface observability with STRIDE coverage. Federation,
durable execution, recovery and untrusted extension execution remain deferred.

The following tracks agreed directions and remaining architectural proposals
before tightening their concrete contracts. The reverse-review record in section
19 is historical; subsequent agreements below update its open-decision snapshot:

| Open area | Missing design decision | Formal/assurance perspective |
|---|---|---|
| Public operation/result contracts — direction agreed in residual review | Contract-first, representation-independent, small closed versioned vocabulary; explicit target identity/context, coordinate conversions, anchor dispositions, outcome guarantees and fidelity profiles. Preserve previously agreed canonical UTF-8 document-byte coordinates and linked-frontier history; exact schemas and algorithms remain for specification | Concrete implementations must refine allowed abstract transitions and preserve invariants; storage choice and a closed verb list alone do not establish refinement |
| Symbolic kernel and adapter boundary — direction agreed | Sections 3.4.1 and 5.2.1 separate authored authority, contextual derived evidence and authorized projections; file/provider and UI mechanisms remain at edges. V1 scope is unchanged; no universal graph/query engine or structured-authoritative profile selected | Existing bounded publication/evidence results remain scoped; discovery soundness/completeness, semantic-to-source refinement, safe graph projections and adapter conformance remain explicit obligations |
| Document/resource identity and lifetime — direction agreed | Section 3.7 establishes document-first identity, versioned bindings, host-scoped alias resolution and separate view/disposal lifetimes; concrete contracts remain for specification | Ownership uniqueness and lifecycle transitions; no handle reuse confusion, resurrection or unintended disclosure through membership |
| Comments and annotations — v1 direction agreed | Section 3.8 establishes first-class document-associated annotations, revision-bound targets, minimal authoring/presentation and separate host persistence; concrete schemas and history/fidelity integration remain for specification | Attachment evidence is not semantic validity; annotation versions, disclosure, source undo and persistence remain distinct |
| Linked local undo/redo — v1 direction agreed | Section 5.8 establishes frontier-only scoped linear histories, atomic linked-frontier validation, separate authored-annotation history and explicit redo/retention invalidation; selective reversal remains deferred | Reversal preconditions, atomic participant validation and fresh revisions; preservation of unrelated work or explicit rejection |
| Text/resource fidelity and save state — v1 direction agreed | Section 5.9 establishes strict UTF-8 with reversible BOM metadata, preserved newline/content fidelity, explicit conversion and separate source/annotation baselines, external freshness and write evidence; concrete contracts remain for specification | Round-trip laws only under stated profiles; failed conversion cannot silently lose data or corrupt authoritative text; saving and history do not imply identical state |
| Configuration and registration activation — v1 direction agreed | Section 11.2 establishes typed scope/merge rules, explicit precedence and workspace-profile selection, staged scoped activation and retained in-flight context with current-authority checks. Frontends separately own keymap activation and chord cancellation under §3.1; concrete contracts remain for specification | Deterministic resolution, atomic engine semantic-profile activation, stale-generation rejection and no authority expansion through view settings; no joint engine/UI preferences transaction |
| Layout/interaction ownership — v1 direction agreed | Section 8.1 selects shared adapter-facing interaction/display policy with provider-owned geometry and per-view/context layout; one local engine/authority manages many documents. No renderer or concrete platform matrix selected | Consistent layout-generation hit testing and coordinate relations where mappings are defined; unsupported geometry is explicit rather than fabricated parity; adapter conformance remains unproved |
| Concurrent saves — v1 direction agreed in residual review | Section 5.10 selects single-flight per resolved destination and bounded FIFO of exact admitted captures; capacity rejection/expiry are explicit, coalescing deferred. Numeric budgets/deadlines/shutdown and provider contracts remain open | Preserve exact monotone evidence and unknown obligations; validate provider token/order contracts; no false latest-write inference from revision or receipt delivery |
| Scheduling and protected capacity — v1 direction agreed in residual review | Section 7.5 selects bounded protected control/completion capacity plus cost-aware fair sharing and originating-scope attribution; concrete scheduler, numeric shares and safe borrowing mechanisms remain unselected | Separate reservation integrity/no quota manufacture from progress under explicit host service and bounded/cooperative processing; agreement is not a verified fairness theorem |

These are design choices, not requests to select dependencies or write code.
Reference-editor behavior and formal properties should inform each decision;
exact types, schemas, constants and test harnesses follow only after direction
is agreed. Platform support commitments need a subsequent explicit matrix, not
an assumption that both adapter names imply every OS/terminal capability.

Remaining specification work includes the concrete operation catalog and FSM
dispositions, evidence merge/conflict rules, Unicode/matching profiles, region
mapping/precedence, dependency tokens, admission/backpressure and retention
budgets, command identifiers, capability matrices, compatibility/support policy,
MELT schemas, golden-signal measurement endpoints and STRIDE evidence map.
Storage/tooling selection and performance evaluation belong to later phases.

For each remaining architectural decision, record its externally observable
contract, assumptions, alternatives, recommendation and consequences. Attach
safety, progress and resource obligations separately; identify the refinement
mapping and independent verification approach appropriate to its risk. Update
the digital twin and decision register together, following sections 13.4-14.4.
No claim of full Cleanroom conformance or source-isolated reimplementation is
introduced by completing this audit.

### 17.15 Security/instrumentation review follow-up

The focused review identified additional boundary decisions, separate from the
earlier architectural integration pass. Resolve their direction before concrete
security and instrumentation schemas are specified; no implementation evidence
is implied by recording them.

- Read/disclosure lifecycle: direction agreed in section 15.5; concrete release
  boundaries and evidence remain for specification.
- Private annotation/metadata projections: direction agreed in section 15.6, including
  counts, scoped tokens/events, caches, conflicts and operational disclosure
  limits; concrete contracts remain for specification.
- Instrumentation execution: direction agreed in section 12.7, separating bounded critical-
  path capture from arbitrary subscriber/formatter/export work, with causal/loss,
  reentrancy and failure boundaries; concrete contracts remain for specification.
- Diagnostic data contract: direction agreed in section 12.8, covering collection-time
  schemas/classification, annotations/quotes/previews, configuration, propagation,
  debug/crash/support output and independent release grants; concrete contracts
  remain for specification.
- Shared-resource interference: direction agreed in section 7.5, covering host-assigned
  classes, protected control/completion capacity, bounded fair admission, retry/
  retention accounting and projected overload visibility; concrete budgets,
  service/accounting profiles and conformance evidence remain pending.
- Architecture-wide evidence map: [model.md](model.md) and
  [formal/observability.json](formal/observability.json) cover 89 elements across
  nine catalog families, with 14 observation profiles and a second-agent semantic
  review. They link security controls, authoritative outcomes, MELT/golden-signal
  observation intent and planned scenarios. Concrete schemas, denominators,
  budgets and executed evidence remain for specification under sections 12.5-12.6
  and 17.11; catalog coverage is not implementation or semantic-completeness proof.

## 18. Suggested first decision sequence

The following is the original dependency-oriented planning sequence, not a list
of wholly undecided topics. Direction is now recorded for many of these areas;
section 17.14 identifies the remaining architectural decisions. Complete those
before the concrete specification pass, using this sequence to check dependencies
rather than reopening approved scope or selecting implementation tools early:

1. Initial product scope, non-goals, and modeled system boundary.
2. Digital-twin schema, identifiers, views, validation, and change workflow.
3. Dependency, interface, identity, network, and data-flow models.
4. Threat model, trust boundaries, and STRIDE mitigation map.
5. Document, position, selection, transaction, and history models.
6. Command facade and effect/completion protocol.
7. Revision, snapshot, cancellation, and scheduling semantics.
8. Shared view/layout boundary.
9. Tree-sitter grammar and query lifecycle.
10. Frontend event and rendering adapters.
11. MELT instrumentation contract and golden-signal budgets.
12. TDD/BDD workflow, traceability, and executable acceptance conventions.
13. STRIDE verification matrix derived from the approved model.
14. Quantitative acceptance criteria and compatibility policy.

### 18.1 Reverse-order architectural integration pass

Supplement the forward requirements walkthrough with a reverse traversal from
section 19 toward section 1. The purpose is to accumulate later insights and
propagate their consequences into earlier contracts, not to reverse runtime
dependency direction or replace the forward design/verification workflow.

- Carry an explicit set of review lenses backward: behavioral refinement and
  compatibility; Cleanroom-inspired specification, correctness and independent
  evidence; bounded operation/evidence models; MELT and golden signals along
  pathways and interfaces; STRIDE for both the system and its observation
  pipeline; and v1/deferred scope boundaries.
- At each section, identify which accumulated principles apply, integrate their
  architectural consequences locally, and record assumptions, conflicts and
  unresolved decisions. Add newly encountered principles to the carried set for
  earlier sections. Do not impose every lens on every sentence mechanically.
- Prefer strengthening the owning requirement and linking its assurance
  obligations over repeating later chapters verbatim. Keep authoritative
  outcomes separate from observations, model claims separate from implementation
  evidence, and local guarantees separate from host/environmental assumptions.
- Distinguish clarification of agreed intent from a new design choice. Compare
  meaningful alternatives before resolving the latter; do not infer new scope,
  numerical budgets, dependencies or platform promises from a review principle.
- Record traversal progress and material consequences in section 19.4. Revisit
  already reviewed sections if an earlier contract exposes a contradiction;
  review order is not evidence precedence and later prose does not automatically
  override approved foundational requirements.
- After reaching the beginning, perform a forward consistency check across the
  revised pathways and interfaces. A reverse pass cannot alone establish global
  consistency, completeness, formal closure or implementation conformance.

## 19. Architectural comparison and retrospective review

### 19.1 Text storage and snapshots — design direction

This review compares architectural mechanisms, not measured performance. No
storage crate, tree implementation, chunk size, or allocator is selected. The
recommendation is a persistent chunked text structure behind Vellum-owned
document and snapshot interfaces, with a UTF-8 rope as the leading candidate.

| Reference/design | Architectural strengths | Tradeoffs for Vellum |
| --- | --- | --- |
| Helix: rope-backed text and explicit change sets | Relevant Rust precedent for multiple selections, transaction composition, and mapped positions | Helix's scalar-oriented transaction coordinates must not be silently adopted as byte coordinates; storage choice alone does not define an agent protocol |
| Neovim: memline block tree | Relevant reference for line-oriented access, block storage, and recovery integration | Its block/recovery design is not automatically the immutable worker-snapshot contract Vellum needs |
| Emacs: gap buffer | Simple mutable text with efficient edits near the gap | Moving edits far apart and retaining independent immutable versions require additional consideration |
| VS Code: piece tree | Edits reference original/added buffers with indexed line access | Persistent snapshots and fragmentation management require explicit design; a piece tree remains a viable alternative |
| Flat string | Simple representation and useful correctness reference | Moving large suffixes or copying whole versions is a poor default assumption for scattered edits and retained snapshots |

Upstream references: [Helix architecture](https://github.com/helix-editor/helix/blob/master/docs/architecture.md),
[Helix transactions](https://github.com/helix-editor/helix/blob/master/helix-core/src/transaction.rs),
[Neovim memline](https://github.com/neovim/neovim/blob/master/src/nvim/memline.c),
[Emacs buffer gap](https://www.gnu.org/software/emacs/manual/html_node/elisp/Buffer-Gap.html), and
[VS Code text-buffer design](https://code.visualstudio.com/blogs/2018/03/23/text-buffer-reimplementation).
These are architectural references consulted on 2026-10-22, not claims about
every release or relative application performance. Rolling source links must be
pinned to a revision before implementation evidence relies on specific internals.

The storage contract must require:

- Immutable, revision-identified snapshots that can coexist with continued
  editing and cross-document transaction preparation.
- Chunk/range access for parsing, search, saving and agent context extraction.
  Whole-document materialization must be explicit and budgeted.
- Identical text semantics regardless of chunk boundaries, including graphemes,
  CRLF sequences and matches crossing chunks. Physical chunk IDs are not stable
  document, anchor, block, or token identities.
- Explicit accounting for retained versions, deleted text, transaction staging,
  history, and reclamation work. Cheap snapshot creation does not imply cheap
  indefinite retention or bounded destruction latency.
- Declared capacity and degraded-operation policies for large documents and
  long lines. Storage, parser and rendering limits must be separable so optional
  analysis can degrade without corrupting ordinary editing.

A rope is preferred because this combination emphasizes immutable snapshots
and scattered edits. That is an architectural fit judgment, not a benchmark
result or proof that ropes outperform gap buffers or block/piece trees.
V1 needs one storage implementation behind a small interface, not a general
runtime-pluggable storage framework. Concrete algorithms and evaluation follow
the later model/specification/implementation process.

### 19.2 Sweep of earlier requirements

The following findings retain prior decisions while adding missing distinctions.
They are architectural direction; exact types, budgets, transition tables and
acceptance scenarios remain for the subsequent specification pass.

| Area | Perspective from the reference editors | Vellum direction and scope |
| --- | --- | --- |
| Transactions and selections | Helix makes selections and change sets explicit; Emacs supports atomic change groups across buffers | Keep shared transaction/selection mapping and atomic local multi-document publication. Define overlapping-selection normalization and primary selection identity without adopting modal input as a requirement |
| Coordinates and anchors | Helix uses scalar positions in transactions; Neovim exposes byte columns and extmark gravity; Emacs markers have insertion affinity | Keep typed UTF-8 byte edit coordinates, explicit conversion, and separate logical/visual positions. Anchors need insertion affinity, deletion invalidation/collapse, and declared restoration behavior through undo |
| History | Neovim distinguishes undo blocks, branches and persistent undo; Emacs uses undo boundaries | Separate operation records, user undo groups, redo branches and recovery history. Keep explicit scope and linked local undo in v1; branching UI, durable history and semantic selective reversal are later capabilities |
| Documents and views | Emacs buffers can exist without windows; indirect buffers share text while retaining distinct context; Neovim separates buffers and windows | Document ownership and history are independent of view lifetime; closing a view alone does not authorize discarding unsaved state. Workspace membership cannot duplicate authoritative text. Analysis/view settings may differ; shared mutation restrictions cannot be weakened by a view |
| Resource lifetime | Neovim distinguishes valid buffer handles from loaded buffers | Distinguish identity, loaded/resident contents, retention, and disposal. Validate late results against document/view incarnation and revision; closing a view must not resurrect or destroy unrelated document work |
| Commands and discovery | Emacs commands are independent of bindings; Helix exposes static and typable commands; Neovim has direct APIs | Preserve one semantic contract across typed calls, discovery, prompts and bindings. Resolve implicit interactive focus into explicit targets before execution; host/agent calls must not inherit accidental current-buffer state |
| Repetition and composition | Modal operators, command sequences and macros demonstrate the value of reusable actions | Support bounded declarative command composition; keep text insertion separate from key events. Key replay is not the authoritative automation protocol. General scripting and recursive macros remain deferred |
| Configuration and keymaps | Helix uses declarative remapping; Emacs distinguishes unbound/fallthrough behavior and map precedence | Use typed setting scopes and explicit merge rules within each owning layer. Frontend binding removal, blocking and fallthrough are distinct. Bound frontend chord/context evaluation; prevent settings from introducing arbitrary execution or making input policy an engine dependency |
| Rendering | Neovim separates UI clients from editor state; Emacs and Helix separate text from display context | Share selection, decoration, viewport and navigation semantics; let geometry providers handle glyphs or terminal cells. Use layout generations, consistent hit testing, bounded virtualization and safe conservative invalidation |
| File safety | Emacs distinguishes file locks and external modification detection; editor buffers can diverge from disk | Preserve content/encoding intent and saved baselines. Save identified snapshots, serialize destination writes, and report external conflicts. Advisory locks or check-then-write detection are not a universal conditional-write guarantee |
| Extensibility and failure | Reference editors expose powerful scripting and callbacks | Borrow discoverability and composition, while retaining Vellum's closed operation algebra. Notification callbacks cannot mutate during publication; trusted Rust registration is not an untrusted-code sandbox |
| Language services | Helix separates language configuration and language-specific queries | Identify grammar/query/configuration versions separately from text revisions. Validate query compatibility and bound injections/analysis when enabled; a full language ecosystem or LSP is not added to v1 |
| Remote access and federation | Neovim demonstrates editor APIs usable by external clients | Preserve explicit targets, origins and receipts, but remote presentation does not itself provide federation or offline replication. Their implementation remains deferred |

Behavioral references supporting the sweep:

- [Helix selection model](https://docs.helix-editor.com/usage.html) and
  [command remapping](https://docs.helix-editor.com/remapping.html).
- [Neovim API: byte coordinates, extmarks, buffer handles and clients](https://neovim.io/doc/user/api/)
  and [undo](https://neovim.io/doc/user/undo.html).
- [Emacs buffers](https://www.gnu.org/software/emacs/manual/html_node/elisp/Buffers.html),
  [indirect buffers](https://www.gnu.org/s/emacs/manual/html_node/emacs/Indirect-Buffers.html),
  [marker insertion types](https://www.gnu.org/software/emacs/manual/html_node/elisp/Marker-Insertion-Types.html),
  [atomic change groups](https://www.gnu.org/software/emacs/manual/html_node/elisp/Atomic-Changes.html),
  [keymap fallthrough](https://www.gnu.org/software/emacs/manual/html_node/elisp/Format-of-Keymaps.html), and
  [file locks](https://www.gnu.org/software/emacs/manual/html_node/elisp/File-Locks.html).
- [Helix language configuration](https://docs.helix-editor.com/languages.html).

Reference-editor mechanisms are not specifications of Vellum. In particular,
atomic change groups are not evidence of isolation from every observer, remote
APIs are not federated authorization, and editor scripting is not part of the
closed v1 operation vocabulary. Persistent undo and durable workflows remain
separate from persistent in-memory text structures.

### 19.3 Remaining design choices

Section 17.14 is the consolidated architectural decision register; this list
retains topic-specific follow-up and is not a separate claim of design closure.

- Language-service lifecycle direction is recorded in section 6.1. Complete
  its concrete contracts and acceptance evidence in the specification pass.
- IME/composition and accessibility direction is recorded in sections 9.4 and
  9.5. Complete platform behavior and verification matrices in the specification
  pass without promising unsupported native capabilities.
- Search/replacement direction is recorded in section 5.6. Complete the exact
  matching/line-ending profile, caseless mapping, navigation defaults and bounded
  result interface during the specification pass.
- Set quantitative capacity/latency/retention targets in the specification pass.
  No numeric performance acceptance or storage dependency is selected here.

### 19.4 Reverse traversal record

The reverse integration pass in section 18.1 and the forward consistency check
are complete at requirements/design level as of 2026-10-22. This record describes
design review only, not implementation, testing, certification or proof of
completeness. Remaining architectural choices are preserved in section 17.14.

- **Section 19 — reviewed for later-principle integration:** comparisons remain
  evidence about architectural mechanisms, not imported specifications or proof
  of Vellum's properties. Storage alternatives must satisfy the same observable
  text/snapshot contract; future evaluations must separately examine correctness,
  resource bounds, retained-version pressure and reclamation latency. Reference
  code/asset provenance and independent correctness oracles remain explicit
  under sections 13.4-13.5. The consolidated open decisions remain in 17.14.
- **Section 18 — reviewed:** added the reverse traversal and its accumulated
  lenses, decision boundaries, revisit rule and final forward consistency check.
  Dependency-oriented implementation planning remains separate from review order.
- **Section 17 — reviewed:** decision resolution now distinguishes architectural
  agreement, completed specification and verified implementation; records
  assumptions, refinement obligations, pathway/interface signals, STRIDE and
  independent evidence. Added local follow-up for lifecycle, coordinate laws,
  file fidelity and signal provenance; preserved exact-revision v1 proposals
  and deferred-capability gates. The five architectural choices in 17.14 remain
  open; this pass has not silently selected their policies.
- **Sections 16 through 12 — reviewed in reverse order:** packaging now carries
  provenance/activation boundaries and compatibility preserves observable
  guarantees rather than span topology. Security distinguishes trusted-process
  assumptions, enforcement and optional detection. The twin distinguishes
  design completion from implementation verification and exposes compositional
  assumptions. Assurance and observability now explicitly prevent diagnostic
  presence/correlation from substituting for authoritative correctness evidence.
- **Sections 11 through 6 — reviewed in reverse order:** configuration now owns
  scope/generation and safe-activation obligations without selecting its open
  precedence policy. Host boundaries distinguish guarantees from attestations;
  frontend/layout contracts preserve explicit targets, generation-aware geometry
  and display-data safety. Async contracts distinguish reproducible transitions
  from schedule-independent outcomes and deadlines from actual preemption.
  Language results remain bounded derived evidence, never mutation authority.
- **Sections 5 through 1 — reviewed in reverse order:** editing now includes
  non-text dependency validation and explicit assurance laws; commands separate
  invocation surfaces, admission and effects. Architectural contracts distinguish
  local event order, incarnations and freshness generations while prohibiting
  alternate writes into coordinator-owned state. The front matter now carries
  refinement, least authority, pathway/interface observability, STRIDE and
  Cleanroom-inspired independent evidence, with the five open choices preserved.
- **Forward consistency check — complete:** section 19.5 records composed
  pathway checks, reconciled wording and remaining design/evidence boundaries.

### 19.5 Forward consistency check and review outcome

After the reverse traversal, review the accumulated contracts in execution and
dependency order. The following checks were performed against this document,
not against an implementation or executable formal model:

| Composed pathway | Consistency reviewed and boundary retained |
|---|---|
| Discovery/input/proposal → admission → preparation → validation → publication | One semantic path, explicit targets, current capabilities and declared text/non-text dependencies; no hidden external callback during bounded local publication. External policy/token validity remains a host contract, not automatic cross-system isolation |
| Publication → history/observers → host effects | One coherent local publication and fresh text revisions; post-publication callback/effect failure cannot rewrite the commit outcome. Linked undo/redo admission and conflict policy remain open |
| Load/save → completion → saved-state reporting | Snapshot-specific writes, ordered destination work and explicit failed/unknown evidence; no implication of atomic multi-file durability or protection from uncoordinated writers. Resource aliases/binding, fidelity and saved-baseline semantics remain open |
| Configuration/registration → worker execution → derived-result acceptance | Profile/context generations and bounded activation/fallback prevent incompatible results acquiring authority. Exact precedence, activation timing and in-flight policy remain open, not selected by generic generation tracking |
| Parsing/search/layout → frontend interaction → validated edit | Derived results retain source context; truncation is not completeness, painted geometry is not an edit range, and all proposed mutations reenter validation. Layout ownership and concrete platform promises remain open |
| Effect dispatch → timeout/cancellation/restart → late completion/shutdown | Cancellation intent, attempt identity, authoritative effect evidence and applicability remain separate. Deterministic transitions do not imply schedule-independent outcomes; deadline expiry and panic handling do not guarantee process isolation or code preemption |
| Operation/health outcomes → MELT/golden signals → export/diagnostic action | Authoritative outcomes survive optional diagnostic loss; signal provenance, bounds and disclosure follow interfaces. STRIDE covers both observed pathways and monitoring; diagnostic or automated remedial actions cannot bypass normal authority checks |
| Contract/model → implementation → verification → compatibility claim | Agreed design, completed specification and verified behavior remain distinct. Formal claims retain assumptions/bounds; independent oracles and source provenance remain required. Compatible releases preserve declared contracts rather than merely compile |

The check reconciled older wording about global deterministic ordering, generic
worker-panic containment, text-only freshness, view/document lifetime and the
extension ABI. Deferred background-service examples and facade/UI inventories
are explicitly not v1 feature commitments. No implementation dependency, numeric
budget, unsupported platform capability or resolution of the five open policies
was introduced by this review.

Numbered headings and direct numbered section references were also checked for
duplicates/missing targets. These are structural document checks only. No
benchmark, implementation experiment, executable model check or software test
was run as part of this design review. External comparisons retain their cited
scope and provenance; no full Cleanroom or source-isolation claim is made.

The architectural integration review is complete; the design is not declared
closed or implementation-ready. Next resolve the five choices in section 17.14,
starting with document/resource identity and lifetime, then conduct the concrete
specification/acceptance pass using the evidence disciplines in sections 13-14.

### 19.6 Low-surprise and agent-value sanity review

Subsequent review on 2026-10-23 supersedes earlier open-decision snapshots where
explicit residual agreements are recorded in section 17.14. This is an upstream
documentation/behavior comparison and design assessment, not executed differential
testing, a benchmark, certification or proof of reference-editor equivalence.
Rolling documentation describes observed upstream contracts; pin released builds
and relevant source revisions before implementation/conformance claims rely on
specific behavior. Earlier upstream code inspection remains recorded provenance;
no source-isolated clean-room claim is introduced.

| Area | Established reference / assessment | Vellum disposition and agent value |
|---|---|---|
| Functional state and view separation | CodeMirror documents immutable state updated by transactions, with a separate view | Retain immutable snapshots and one mutation path; headless proposals need no screen simulation. A multi-object coordinator is Vellum's extension, not a CodeMirror guarantee |
| Programmatic editing and coordinates | Neovim exposes typed buffer/window APIs, byte-oriented text positions, extmarks and sequentially scheduled editor operations | Keep explicit targets, validated conversions and structured results; no mandatory RPC transport, socket or current-buffer assumption |
| Grouped atomic changes | Emacs supports atomic change groups spanning buffers through its lower-level group API, separately from undo amalgamation | Atomic multi-object publication and linked frontiers are justified agent enhancements; complete observer isolation and Vellum's undo policy are not inferred from Emacs |
| Mixed-language interpretation | Helix documents language injections and explicit combined-region settings | Keep one source owner with bounded derived interpretations; no extra authority per language or mandatory universal semantic graph |
| File conflict detection | Emacs/Neovim document modification checks and explicit overwrite decisions | Ordinary files use best-effort detection with structured conflicts and retained unsaved content; strong conditional protection is optional, not required of all filesystems |
| Save acknowledgment | Neovim documents synchronization enabled by default with a latency cost | Use the actual provider write/sync profile, exact captured snapshot and explicit failure/uncertainty; no implicit recovery journal or universal hardware durability |
| Agent multi-edits/history | Existing editor transaction/grouping mechanisms supply building blocks, not a complete concurrent agent protocol | Keep read/write/discovery dependencies, atomic local participants, immutable captures and linked-frontier rejection; these prevent partial refactors and stale application without requiring agent inference from UI prompts |
| CQRS/symbolic evidence | Read/write separation is useful organization, not a requirement for two services or stores | Keep source-authoritative v1 documents, contextual optional evidence and authorized projections; separate databases, generic query languages and inference engines remain unselected |
| Scheduling and retention | Reference editing behavior does not establish Vellum's admission/fairness or aggregate resource bounds | Protected capacity/cost-aware sharing is subsequently agreed by user review; concrete service/cost evidence and numeric profiles remain needed. No actor/thread per object or unbounded priority lane is implied |
| Git and external orchestration | Git supplies snapshot/reference primitives, not a general editor/provider transaction | Optional staging only; do not require a worktree per task or saga for ordinary local edits/saves. Cross-authority/durable capabilities remain deferred |

Follow-up findings, not newly selected mechanisms:

- Specify normal consecutive queued saves carefully: exact captured content must
  not imply that all queued requests reuse an obsolete external baseline after
  the engine's own preceding save. Any permitted causal baseline progression
  must be explicit, preserve external-conflict detection and differ from blindly
  refreshing a failed condition. The single-attempt storage bridge does not prove
  this multi-save provider contract. Carry under residual R-01/R-04/R-07.
- Optional snapshot mapping, semantic discovery and more permissive undo must
  not become prerequisites for basic agent edits. Exact-revision rejection is
  a safe v1 baseline; dependencies and diagnostics must make refresh actionable
  within disclosure limits rather than rely on a user watching the screen.
- Reference editors' scripting, callback conventions, forced overwrites and UI
  prompts are not copied wholesale. Preserve the bounded registered vocabulary,
  explicit authority, no hidden publication callbacks and machine-readable
  outcomes. These are deliberate agent/safety constraints, not general superiority
  claims about the reference editors.
- Preserve familiar simple edit/save/navigation paths. Surface richer receipt,
  projection and lifecycle detail when relevant; do not expose mandatory universal
  algebra/graph/provider orchestration for every basic operation. Concrete public
  API ergonomics and concurrent-agent workloads still need conformance/evaluation.

Sources: [CodeMirror system guide](https://codemirror.net/docs/guide/),
[Neovim APIs](https://neovim.io/doc/user/api/),
[Emacs atomic groups](https://www.gnu.org/software/emacs/manual/html_node/elisp/Atomic-Changes.html),
[Helix injections](https://docs.helix-editor.com/master/guides/injection.html),
[Emacs interlocking](https://www.gnu.org/s/emacs/manual/html_node/emacs/Interlocking.html),
[Neovim file editing](https://neovim.io/doc/user/editing/) and
[synchronization policy](https://neovim.io/doc/user/options/#'fsync').
The Git comparison retains section 5.3.1's primary references and limits.

### 19.7 Residual direction review completion

The 2026-10-23 review is complete at the requirements/architecture level. This
means every residual has a direction or an explicit evidence/deferred boundary;
it does not mean every residual is closed, specified, proved or implemented.
This section and section 17.14 supersede historical open-choice snapshots in
sections 19.4–19.5. Detailed residual evidence gates remain in
[the completion audit](formal/completion-audit.md).

| Residual | Baseline carried forward | What remains before a stronger claim |
|---|---|---|
| R-01 | Contract-first closed vocabulary; typed source coordinates and exact-revision admission | Complete public schemas, attachment/inverse laws and concrete representation refinement |
| R-02 | Exclusive ownership, immutable snapshots and explicit read/discovery dependencies | Real invalidation, coverage, source mapping, accounting and reclamation |
| R-03 | Single-flight bounded FIFO saves, shared interaction/provider geometry, protected cost-aware scheduling | Concrete profile and adapter/scheduler conformance, not another policy selection |
| R-04 | Host-scoped identity and honest filesystem guarantees; attributable queued-baseline progression retained provisionally | Actual alias, conditional-write, synchronization, receipt and multi-save provider contracts |
| R-05 | Separate resource ceilings, inherited budgets, monotonic deadlines, protected unsaved work and bounded shutdown waiting (§§7.5–7.6) | Numeric profiles, clock/service contracts and resource/lifecycle tests |
| R-06 | Explicit field grants and current release checks on all projections (§§15.5–15.6) | Complete field/token/event/cache policy and non-disclosure conformance |
| R-07 | Separate authored state, contextual evidence and authorized projections; kernel-to-edge refinement obligations | General composition, actual codec/change/provider bridges and independent oracles |
| R-08 | Trusted host establishes authority; identity/correlation handles and semantic edges are not credentials | Concrete local provenance, revocation and compromise mechanisms; remote identity remains deferred |
| R-09 | Layered bounded checks plus explicit deductive/refinement obligations (§§13.4–13.5) | General proofs/certificates and trusted proof boundaries, not renamed assumptions |
| R-10 | Local bounded MELT/golden signals, STRIDE coverage and observation-led tuning (§12) | Schemas, complete release coverage, overhead and adversarial tests |
| R-11 | Independent review/oracles and explicit source provenance (§13) | Actual independent work and operational reliability evidence; no source-isolation claim |
| R-12 | Attempt/destination isolation first, justified wider escalation, explicit recovery (§7.6) | Precise trusted-fault matrix and quarantine/re-entry conformance |
| R-13 | V1 scope exclusion | Detailed deferred designs removed; no future-design work or runtime required for this release |

Disclosure baseline: source access does not automatically grant private comments,
history, receipts, topology or diagnostic metadata. Apply current grants before
cache reuse/release; an internal derived read model is not an authorization cache.
Use snapshot-specific recipient-scoped tokens where needed, not globally revealing
revision counters. Concrete roles/issuers/field grades are not invented here.
Editor behavior is precedent for interaction, not proof of zero-trust disclosure.

Assurance baseline: separately establish successful change application, rejection
without authored mutation, conditional inverse validity, mapping/composition where
defined, authorized projection and honest effect evidence. Do not extend a strong
conditional-write proof to ordinary filesystem saves, or a single-attempt proof
to queued baseline progression. No proof assistant, certification scheme or remote
identity framework is selected by closing this review.

### 19.8 Cohesive strengthening from the editor baselines

Carry forward the following architectural consolidations, not new product scope.
They reuse the existing engine and boundaries rather than add services, storage
backends or another editing mode. The purpose is fewer competing interpretations
of the same operation while keeping simple edit/save paths simple.

| Consolidation and precedent | Benefit | Cost / boundary / recommendation |
|---|---|---|
| Shared change semantics: transaction-oriented editors, notably CodeMirror; grouped editing in Emacs (§19.6) | One validated source-coordinate change description drives source publication, conditional history inverses, attachment mapping and derived invalidation | Reuse semantics, not necessarily one concrete struct. Mapping is partial and profile-bound; do not pretend all transforms are invertible or force UI geometry into the change algebra. Prefer this over adapter-specific edit engines |
| Explicit snapshot handoff: immutable editor state and Neovim revision-bearing buffer updates | An authorized operation receipt identifies the publication against which a caller can query; agents need not wait for a screen or index to catch up | Retention is bounded and authorization rechecked. If a snapshot is unavailable, report that explicitly; derived answers may still be pending/partial. No global multi-authority consistency or leaked global revision token is implied |
| Dependency-directed derived work: transaction updates and Helix language injections | Invalidate/recompute only affected source/context regions where coverage is established; avoid global reparsing on every agent edit | Membership/negative/discovery dependencies still count. If precise coverage is unavailable, conservatively invalidate a broader declared scope. Keep analysis optional; do not require a universal incremental graph engine |
| Shared contract metadata: Neovim typed/versioned API metadata and editor command/keybinding separation | Agents and both frontends can discover permitted verbs, parameter domains and required capabilities without key replay or implicit focus | Metadata itself is authorized/bounded, not a grant or full private registry dump. Keep a small closed versioned vocabulary; no arbitrary code or transport required |
| One causal context across boundaries: consolidate existing attempt, revision, dependency, authority, budget and profile identities | Publication, save receipts, history and instrumentation can explain the same action without contradictory parallel metadata | Use minimal typed boundary projections, not a giant universal request object. Correlation is not authority; retries get attempt identity while preserving originating resource obligations |
| Profile-based conformance across engine, providers and adapters | A headless scenario can test the same promised contract through direct, `iced`, `ratatui` and controlled host boundaries; stronger providers advertise stronger guarantees explicitly | Shared fixtures are useful but not independent oracles. Pin reference versions, include weaker normal-filesystem profiles, and test invalid/stale/unknown/partial outcomes. No adapter may advertise a guarantee solely because a stronger abstract model passed |

The first five are cohesive requirements clarifications of existing contracts;
the sixth is the verification architecture for them. Concrete data structures,
cache policies, invalidation algorithms, public schemas and conformance harnesses
belong to the subsequent specification/implementation pass. Resource savings and
latency improvements remain hypotheses to measure under section 12.5.1, not
benchmark results. Invalidation must not silently require a deferred semantic
indexer or language server; basic source editing remains independent of them.

Alternatives deliberately not adopted: separate per-file engines/actors add
ownership and cross-file coordination overhead without an established need;
mandatory event sourcing/graph databases add persistence/query obligations to
ordinary editing; unrestricted scripting expands the execution/proof boundary;
implicit rebasing or selective undo weakens the chosen exact-revision/frontier
baseline. Each can have uses, but none is necessary to realize these improvements.
Optional bounded mapping and Git staging retain their existing evaluation scope.

Primary references for this pass: [Neovim API metadata and buffer updates](https://neovim.io/doc/user/api/)
and [Helix injection profiles](https://docs.helix-editor.com/master/guides/injection.html).
CodeMirror/Emacs comparisons retain the sources and limits in section 19.6;
those sites could not be freshly retrieved in this pass, so no additional upstream
behavior is inferred from them. The snapshot/context/conformance consolidations
are Vellum design conclusions, not claims that any reference editor supplies
Vellum's full multi-object, disclosure or concurrency guarantees.

### 19.9 Editor-engine ideas to carry forward

Captured from the subsequent editor-engine discussion. Borrow engine primitives,
not implicit focus, keystroke automation or a reference editor's entire runtime.
The following reinforce existing requirements and section 19.8, without selecting
implementation dependencies or expanding deferred capabilities.

Adoption follow-up: the strongest ideas are now normative at their owning
boundaries: authorized command metadata (§3.3), snapshot handoff and subscription
recovery (§3.4.2), common typed attachment semantics (§3.8), explicit target sets
(§5.1), and shared change semantics/coherent state participation (§5.3). Existing
atomic multi-document publication (§5.3) and single-owner mixed-language profiles
(§6.2) remain required. This promotes the six established directions below, not
the candidate lens API, plugin runtime or storage implementation. Concrete schemas,
mapping/refinement laws and measured conformance remain outstanding.

| Established direction / precedent | Engine consequence | Tradeoff and safety boundary |
|---|---|---|
| Immutable state and explicit transactions — CodeMirror | Changes are data against identified snapshots; reuse change semantics for publication, conditional history, attachment mapping and invalidation | Explicit bookkeeping replaces competing mutation paths. Not every transformation is invertible, and sharing semantics does not require one universal struct |
| Selection-first batch editing — Helix | Explicit target sets are engine inputs. Resolve supported line/range and optional block/language-token selectors into revision-bound source ranges before publication | Define overlap/ambiguity rejection and preserve read/discovery dependencies. Do not require modal interaction, active cursors, a parser or optional semantic discovery for basic edits |
| Mapped, namespaced attachments — Neovim extmarks | Reuse attachment/mapping semantics across comments, diagnostics, bookmarks and proposal targets, with declared insertion affinity and deletion disposition | Namespaces organize producers; they are not authorization boundaries. Moved anchors do not prove semantic target survival. Authored annotations, derived decorations and proposal applicability retain distinct state/history/release contracts; examples do not add a bookmark feature commitment |
| Atomic groups across documents — Emacs | Related participants publish coherently through the local coordinator, with separately specified history grouping | Preserve Vellum's own dependency/isolation and linked-frontier contracts. Reference-editor grouping does not prove observer isolation, persistence atomicity or undo safety |
| Revision-bearing updates and discoverable APIs — Neovim | Authorized receipts and bounded incremental subscriptions identify source context; permitted command metadata enables direct agent invocation | Define event gaps/resynchronization, current release and retention. No implicit current buffer, mandatory RPC transport or public global revision sequence |
| Embedded-language interpretation — Helix | Contextual language regions share one source owner and explicit source mappings/invalidation | Bound nesting/fan-out and declare combined-region profiles. A language label grants neither execution permission nor ownership; basic editing does not depend on language analysis |

Prioritize **transactions, explicit target sets and mapped attachments together**:
they let human actions and agent multi-edits use the same engine machinery. This
is architectural priority, not implementation sequencing or a claim that the
required mapping/inverse laws have already been proved.

Disposition of the three additional ideas, following user agreement: adopt the
minimal bounded operation scope, typed internal state components and indexed
snapshot/storage contract. Only optional public lens/module APIs and the concrete
storage implementation remain candidates; this is not blanket deferral.

| Idea / disposition | Benefit | Alternatives / recommendation / remaining decision |
|---|---|---|
| Scoped editing — minimal operation scope adopted (§5.1), inspired by Emacs narrowing | Bound source writes while reading separately authorized context; retain original identity/coordinates | Existing range/read-only contracts avoid a new lens subsystem. Stale/ambiguous/cross-boundary writes reject the whole operation; no implicit widening. Optional visual lenses/separate public APIs remain unselected; exact profiles and conformance remain to specify |
| Typed transactional state components — internal organization adopted (§§3.2/5.3), inspired by CodeMirror state fields/compartments | Modular ownership/lifecycles without partially published authoritative updates | Typed components declare versions/dependencies and use staged validation/coherent publication. Derived caches normally remain outside that boundary. Public dynamic registration and an arbitrary callback/plugin runtime remain unselected; concrete component/refinement design is pending |
| Indexed persistent text storage — contract adopted (§§3.4.2/19.1), backend unselected | Indexed bounded range/line access, batch edits and immutable snapshots with actual retention accounting | Persistent UTF-8 rope leads; piece tree remains a benchmark challenger. Evaluate scattered batches, snapshot retention, serialization, fragmentation, reclamation and memory as well as typing before choosing an implementation. No reference-editor measurement substitutes for Vellum workload evidence |

Do not import implicit global focus, key replay as the automation API, unrestricted
publication hooks, automatic stale-edit rebasing, selective undo or mandatory
per-file actors. The cohesive direction remains one headless transactional engine,
explicit target sets, reusable change/attachment semantics and multiple authorized
views. Source authority, current-release checks and exact-revision/frontier
baselines are unchanged.

Primary references: [CodeMirror state/transactions](https://codemirror.net/docs/guide/),
[CodeMirror state fields and configuration compartments](https://codemirror.net/docs/ref/),
[Helix selection-oriented editing](https://docs.helix-editor.com/usage.html),
[Neovim APIs, extmarks and buffer updates](https://neovim.io/doc/user/api/),
[Emacs atomic groups](https://www.gnu.org/software/emacs/manual/html_node/elisp/Atomic-Changes.html),
[Helix injections](https://docs.helix-editor.com/master/guides/injection.html),
[Emacs narrowing](https://www.gnu.org/software/emacs/manual/html_node/elisp/Narrowing.html)
and [VS Code text-buffer design](https://code.visualstudio.com/blogs/2018/03/23/text-buffer-reimplementation).
The preceding discussion checked primary documentation/search excerpts; no
installed-editor experiment, new upstream code inspection, performance result
or stronger formal/conformance claim follows from capturing these ideas.

Subsequent adopted-contract assurance is recorded as PS-EC in
[the proof-slice contracts](formal/slices.md). It adds actual finite sequence
splice/batch/inverse/point/chunk laws, scoped coherent publication, advisory
discovery, snapshot release and gap recovery, plus symbolic integer boundary
conditions. Its explicit profile and current full report govern the claim;
this does not establish all Unicode/profile laws, real storage performance,
parsing completeness or concrete implementation refinement. Deferred feature
designs have been removed; brief scope exclusions are not a future backlog.

The subsequent residual proof pass adds arbitrary-length pointwise source-array
laws for inverse content, frame preservation, two-edit composition and chunk read
refinement, plus RFC 3629 scalar encoding/decoding/canonical-byte conditions.
PS-DR additionally checks a bounded shared-cell retention ledger, cancellation
versus actual worker release, capacity admission and current source/context/grant
acceptance of derived work. Sources and remaining proof inputs are recorded in
[the proof-slice contracts](formal/slices.md). These do not establish grapheme
behavior, a concrete parser/storage implementation, physical allocation costs,
all public schemas or general independent refinement; no product/profile choice
is made merely to manufacture a proof.
