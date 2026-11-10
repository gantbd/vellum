# Component architecture

Version: 1  
Adopted: 2026-11-09  
Canonical home: https://github.com/gantbd/keystone/blob/main/COMPONENT_ARCHITECTURE.md

This document defines the shared architectural law for the peer components in
this workspace. Each repository carries a complete copy so that an independent
checkout and remote remain understandable offline. Keystone is the canonical
home because it owns cross-cutting mechanisms and laws, but Keystone does not
own any component's domain semantics.

A repository may implement only the layers it currently needs. The architecture
standardizes ownership and dependency direction; it does not require speculative
crates, transports, services, or user interfaces.

## Governing law

Every sovereign engine remains categorically closed over its bounded,
versioned domain. Effects cross explicit ports. External interactions use
closed, versioned protocols. Human interfaces and agent connectors are peer
projections. Keystone supplies lawful, bounded mechanisms but no domain
vocabulary, routing policy, or hidden authority.

The default implementation discipline is:

- a functional core with explicit effects at the boundary;
- total transitions over closed command, outcome, refusal, and event types;
- explicit cardinality and byte bounds for retained data;
- deterministic interpretation under caller-supplied time, identity, and
  capability facts;
- typed uncertainty rather than inferred success;
- TDD/BDD acceptance evidence and adversarial boundary tests;
- STRIDE and LINDDUN threat and privacy analysis;
- RED, USE, and golden-signal instrumentation with bounded cardinality;
- content-safe diagnostics that are never recovery or authorization evidence.

## The common shape

A mature component can support three deployment modes through the same semantic
contract:

1. A trusted host embeds its engine or application layer.
2. A standalone service composes the engine with persistence and effect
   adapters.
3. Human and agent clients use a typed local or remote protocol.

~~~text
                         inbound adapters
               TUI · GUI · MCP · HTTP · peers
                              |
                       component client
                              |
                      component protocol
                              |
                       component service
                              |
                    application/host layer
                              |
                       sovereign engine
                              |
                       outbound ports
                              |
                storage · clock · provider · OS

             Keystone mechanisms and laws sit below,
             never above or between domain authorities.
~~~

An application may initially use an in-process client. Moving it to a local
daemon or authenticated remote service must not change semantic behavior or
bypass authorization, version checks, effect fencing, or instrumentation.

## Layer vocabulary

Use these names when the corresponding boundary has an independent consumer,
compatibility contract, or release lifecycle:

| Layer | Responsibility |
| --- | --- |
| component-engine or component-core | Pure authoritative state and deterministic transitions |
| component-host | Trusted in-process embedding and outbound capability ports |
| component-protocol | Serializable commands, outcomes, events, refusals, witnesses, and negotiation |
| component-service | Concurrency, persistence, recovery, authorization, and composition |
| component-client | Transport-neutral access to the service contract |
| component-adapter-* | Concrete outbound effect interpretation |
| component-mcp | Agent-facing inbound protocol projection |
| component-tui / component-gui-* | Human-facing inbound presentation |

SDK is a distribution or audience label, not a precise architectural layer.
A package named SDK must document whether it is a trusted host SDK, a remote
client SDK, or an umbrella containing separately named crates.

Do not create a layer merely to satisfy this table. Start with the smallest
closed implementation and extract a boundary when use and evidence earn it.

## State ownership

Every component must classify retained state into these disjoint categories:

| State class | Owner |
| --- | --- |
| Authoritative domain state | Sovereign engine |
| Durable coordination state | Service or explicit coordinator |
| Per-client projection state | One identified human or agent client |
| Invocation and effect state | One bounded operation identity |
| Metrics and diagnostics | Non-authoritative observation |

Shared domain state and per-client state must not be the same object. An agent's
cursor, selection, filters, pending confirmation, or active view must not alter
a human's view. A human interface must not consume an agent's effect witness.
Operations that need presentation state receive their own client identity;
headless semantic operations should not fabricate a cursor.

Observation state may be dropped, saturated, or disabled without changing a
domain result. It cannot establish delivery, durability, authorization, or
recovery.

## Dependency law

Dependencies point toward semantics and ports, never toward presentations or
concrete effects.

~~~text
applications/connectors -> client -> protocol
service -> protocol
service -> application/host -> engine
service -> selected outbound adapters -> host ports
all layers may opt into narrow Keystone mechanisms
~~~

The engine must not depend on:

- a UI framework or connector protocol;
- a concrete filesystem, database, clock, network, model, or clipboard;
- a service runtime;
- another engine's internal types;
- Keystone facilities unrelated to mechanisms it actually uses.

Inbound adapters translate requests into the component protocol. Outbound
adapters interpret effect ports. An adapter must not acquire domain authority
merely because it performs an effect.

## Commands and observations

The component owns a closed command algebra and a closed result/refusal algebra.
Interpreters may be in-process or remote, but they must preserve the same laws.

Where useful, model:

- command construction as a bounded free algebra interpreted by the engine or
  application service;
- observable evolution as a bounded coalgebra with explicit snapshots and
  event continuation;
- effect evidence as a finite lattice whose joins cannot turn uncertainty into
  success;
- capability attenuation as monotone movement toward less authority;
- recovery migrations as explicit versioned morphisms that preserve stated
  invariants or refuse.

Free or cofree structure is adopted only when its carrier, interpreter, laws,
and bounds are explicit. Algebraic terminology is not evidence by itself.

## Protocol boundary

Each component owns its semantic protocol. There is no universal semantic
Component trait and no shared untyped command bus.

Components may share a small bounded envelope containing:

- component and protocol version;
- invocation and idempotency identity;
- causation and correlation identity;
- authenticated-principal assertion reference;
- expected revision or version witness;
- explicit deadline, budget, and capability scope;
- content-safe trace context.

Semantic payloads, refusals, migrations, and compatibility rules remain owned
by the component.

Mutations should carry an expected version or equivalent witness. Matching
replay is an observation of the original result. Reuse of an idempotency key
with different semantic input is a conflict. Stale commands refuse rather than
silently retargeting another resource or version.

MCP is a projection of the native component contract, not the canonical domain
protocol. MCP resources expose bounded read models; MCP tools submit explicit
commands. Prompts are conveniences and carry no authority.

## Standalone and embedded parity

A presentation or connector should depend on a client port with at least two
possible interpretations:

~~~text
                     component client port
                       /               \
              in-process client     remote client
                      |                   |
                service runtime       wire transport
~~~

The standalone binary is a composition root. It selects a service runtime,
adapters, limits, and transports. The embedded form receives those choices from
its trusted host. Neither form may introduce domain behavior absent from the
other.

A remote client need not reproduce internal types. Its protocol DTOs form a
deliberate anti-corruption boundary and are mapped explicitly to internal
commands and outcomes.

## Cross-component composition

Peer engines communicate only through public protocols and retained evidence.
They do not import or mutate one another's internal state.

~~~text
Agora event
    |
  Moira ---- authorize ----> Aegis
    |
    +------ schedule ------> Chronos
    +------ execute -------> Daimon
    +------ edit ----------> Vellum
    +------ publish -------> Agora
    +------ record --------> Hermes
~~~

Moira owns durable workflow coordination and effect identities without
absorbing peer semantics. Chronos owns scheduling and leases, not execution.
Daimon owns authorized execution evidence, not task selection. Aegis decides
bounded authority from host-established credential facts but does not
authenticate credentials. Hermes carries ordered domain events without
interpreting them. Agora owns room semantics. Vellum owns editing semantics.

Cross-component work is a saga of attributable effects and receipts, not an
implicit distributed transaction. Unknown outcomes remain fenced until explicit
reconciliation evidence is admitted.

Rune owns its language and deterministic tooling. Dendrite is a standalone
service projection of Rune tooling. Axon and other agent runtimes consume peer
services; they do not become alternate homes for those services. Product
applications such as Engram or Pies may embed or connect to components while
retaining their own presentation and workflow boundaries.

## Keystone's role

Keystone is cross-cutting but subordinate. It provides small, reusable,
opt-in mechanisms and their laws, including:

- bounded bytes, strings, collections, counters, and identifiers;
- canonical codec primitives;
- evidence lattices and lifecycle obligations;
- generic checkpoint, append, and immutable-blob storage ports;
- deterministic clocks and fault plans;
- content-safe debug wrappers and bounded observation buffers;
- protocol-envelope primitives;
- shared property, model, and conformance test kits.

Keystone must not own:

- task, document, room, grant, workflow, participant, or agent semantics;
- any component's commands, events, refusals, or migrations;
- authorization, retry, reconciliation, or routing policy;
- UI or per-client behavior;
- service discovery;
- a universal runtime or Component trait;
- dependencies on sovereign engines.

Adoption is per crate. A component must not depend on an unrelated Keystone
crate merely for uniformity. Domain-specific schemas and limits remain with
the domain even when encoded using Keystone primitives.

## Service obligations

A standalone service must explicitly define:

- the single-writer or consensus assumption for each authority;
- serialization and concurrency behavior;
- persistence acknowledgment and uncertain-commit behavior;
- startup, recovery, migration, shutdown, and ownership fencing;
- authentication facts versus authorization decisions;
- capability attenuation and revocation;
- subscription ordering, replay, backpressure, and overflow;
- resource and invocation quotas;
- adapter selection and qualification;
- transport security and protocol negotiation.

One process should serialize mutations for an authority unless a separately
specified consensus mechanism exists. Optimistic revisions detect stale input;
they do not by themselves create a distributed single writer.

## Human and agent parity

Humans and agents are identified peers at the protocol boundary, but parity
does not mean identical presentation.

Both receive:

- explicit identities and capability scopes;
- versioned snapshots and typed refusals;
- bounded operations and retained receipts;
- the same semantic authorization and effect rules.

Human interfaces may maintain cursors, layouts, dialogs, and accessibility
state. Agents normally use semantic commands over stable resource identifiers
and version-bound ranges. When an agent needs presentation behavior it receives
its own client projection.

No connector may convert natural-language intent directly into ambient
filesystem or process authority. Planning, confirmation, execution, and
acknowledgment remain separate facts.

## Required evidence

Before claiming a component layer or adapter ready, record:

- its dependency and authority boundary;
- state ownership classification;
- closed command, outcome, refusal, and event vocabulary;
- compatibility and migration policy;
- embedded-versus-remote conformance evidence where both exist;
- adapter qualification matrix;
- recovery, replay, and uncertainty contract;
- STRIDE and LINDDUN analysis;
- RED, USE, and golden-signal definitions;
- bounded resource budget;
- cross-component identity and receipt rules;
- explicit non-claims and unavailable gates.

Tests should include scenario acceptance, invariant and property tests,
fault-injection at effect boundaries, replay and stale-version cases,
serialization golden fixtures, and live qualification only for the surfaces
actually claimed.

## Adoption and evolution

This document constrains new boundaries and guides refactoring; it does not
declare incomplete layers implemented. Each repository should state which
layers exist, which are planned, and which do not apply.

Changes begin in Keystone's canonical copy and are synchronized to every peer
repository in the same workspace change. All copies carry the same version and
must be byte-identical. A component-specific exception belongs in that
component's architecture documentation and must name the law being refined,
the reason, the retained invariants, and its verification evidence.

Historical implementations remain evidence, not current authority. Naming,
packaging, and repository extraction should follow dependency direction and
independent release lifecycle rather than deployment convenience.

