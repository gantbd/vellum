# Single user v1 release gates

V1 is a local, reusable editor engine for one user who may run concurrent agents.
Its release boundary is the approved scope in requirements §§1.1–1.2, including
both iced and ratatui adapters and thin reference applications. Single user does
not imply a single writer, one document, one workspace or absence of external
file changes.

UI adapters/examples are required for the wider editor release, not part of the
engine or prerequisites for completing its behavior contracts. Resource substrates
are separately wired modules; local filesystem is the first target. A future
object-store module is not a v1 requirement, nor is this resource-module choice
the in-memory source/snapshot backend selection.

This classification separates release obligations from optional capability
obligations and additional assurance depth. It does not remove approved features,
declare residual packages complete, or establish release readiness. The broader
specification and scoped-reference-evidence goal remains open.

## Classification rules

- **Release gate:** required behavior or evidence needed for the approved local
  profile. An incomplete gate blocks the corresponding release claim.
- **Conditional obligation:** required when an advertised optional capability,
  host profile or provider is included. Omitting optional support is explicit;
  enabling it activates its compatibility, security and evidence obligations.
- **Later hardening:** broader generality or additional assurance depth that is
  not needed to establish the supported local contract. A known safety defect in
  a shipped path cannot be reclassified as later hardening.

Optional dependencies do not make required integration optional. For example,
parsing may be disabled, but the approved optional parsing integration still needs
its promised mixed-language reference case. No rows below authorize guessed
host versions, physical bounds, provider guarantees or numeric performance targets.

## Release gates

| Area | Required local contract and acceptance evidence |
| --- | --- |
| Editing and fidelity | Lossless source preservation for supported inputs, Unicode and line profiles, insertion/deletion/replacement, multiple selections, navigation, indentation, read-only enforcement and literal search/replacement. Test boundaries, malformed inputs, no-ops and round trips. |
| Publication and dependencies | One local mutation authority; exact-revision proposals; declared local document dependencies; atomic local multi-edit and multi-document publication; explicit conflicts. Demonstrate no partial publication and no silently repaired stale targets. Atomic in-memory publication is not atomic multi-file disk persistence. |
| Ownership and history | Explicit document/view lifetimes, selection and annotation mapping, undo/redo grouping and linked operation identities. Demonstrate correct lifetime and cleanup after cancellation, faults and late delivery on shipped paths. |
| Comments and annotations | Approved document-associated records, both position and content point profiles, supported history/projection/restoration, and the versioned reference JSON format. Ordinary source and annotation destinations remain distinct resolved resources. |
| Persistence | Supported host load/save procedures, faithful codecs, revision-specific outcomes and protection against overwriting external changes. Keep applied, persisted, partial and unknown outcomes distinct; do not infer crash durability or recovery from acknowledgment alone. |
| Engine facade | Coherent typed operations and dynamic semantic equivalents, closed executable meanings, explicit outcomes, required command metadata/discovery and host command registration. Cover every advertised v1 operation; shape validation never substitutes for authority, revision or original-handle checks. No mandatory network codec or public acquisition verb is introduced. |
| Frontends and host effects | Both required adapters and thin applications share engine semantics. UI owns keymaps, input translation and presentation. Engine owns behavior and accepted evidence; substrate owns concrete I/O. Verify the supported clipboard, geometry, IME and accessibility procedures or document their capability limits. |
| Language integration | Approved optional incremental parsing/highlighting, pinned reference assets and bounded mixed-language integration with graceful unsupported-language fallback. Inspect actual compatibility and licenses for shipped assets. |
| Local security and resources | Explicit host capabilities, inert document/annotation content, validated requests, bounded work and capture, cancellation, protected cleanup and explicit failure outcomes. Concurrent agents cannot bypass one local authority or acquire rights from authored instructions. |
| Instrumentation | Content-safe diagnostics, operation dispositions, timing, declared occupancy/saturation and observation-loss reporting sufficient to explain failures and resource pressure. Cover critical execution, identity, data and host-effect boundaries with the adopted security/privacy/hazard perspectives. Telemetry export remains optional. |
| Supported release profile | Concrete crates/features, tested host/framework/dependency matrix, license review, conformance tests, stated budgets and measured performance/resource evidence for supported workloads. Neither a trait nor a mock proves a platform guarantee. |
| Assurance and traceability | Trace release claims to contracts, implementation and tests. Apply targeted formal/reference evidence to critical transitions and retain its assumptions. Run the current frozen suite for adopted specification batches; run implementation conformance and failure tests before release. Documentation/model tests alone do not establish a working editor. |

## Conditional obligations and later hardening

| Area | Conditional obligation | Not a base v1 prerequisite |
| --- | --- | --- |
| Identity and deployment | Host-supplied coordination or stronger authority guarantees require explicit advertised support and evidence. | Federation, distributed identity/policy services, remote authorship transport, listeners and relays. |
| Execution | Any advertised host-executed work needs bounded admission, current authority and reliable completion/cleanup semantics. | A durable workflow service, distributed saga infrastructure or an OTP runtime. These patterns may inform local design without becoming deployment dependencies. |
| Configuration and catalogs | Required settings, host command registration and discovery must remain coherent. Additional providers or runtime activation activate their full lifecycle obligations. | General-purpose provider/package ecosystems, arbitrary profile combinations and speculative hot activation. Existing scoped references remain evidence, not a requirement to reproduce their private machinery in production. |
| Language and schemas | Advertised alternative grammars, schemas, dialects and providers need compatible bounded execution, provenance and licenses. | Universal semantics, automatic dependency discovery, arbitrary ontology/query engines or remote schema retrieval. |
| Hosts and persistence | Every additionally claimed platform/provider/durability guarantee requires actual conformance and failure evidence. | Broad untested support, collaborative storage, session recovery and stronger durability than the supported provider establishes. |
| Extensibility | Required in-process commands and bounded declarative composition retain their contracts. | A scripting runtime, binary plugin ABI, untrusted extension sandbox or full editor application ecosystem. |
| Observation | Any exporter or additional operational promise needs privacy, loss, overhead and workload evidence. | A mandatory telemetry backend, duplicated framework-specific counters or unsupported percentile objectives. |
| Assurance depth | Specific contractual formal or independent-assurance claims require the evidence they promise; existing gaps remain explicit. | Blanket categorical completeness, independent Cleanroom certification of the whole product, or identical model/fault/mutation coverage for every private helper as an automatic release gate. |

## Routing the residual packages

Each package remains open until its own closure criteria are satisfied. Release
priority does not erase conditional or later work from the specification review.

| Package | Release priority | Conditional or broader work |
| --- | --- | --- |
| C1 Configuration | Required settings consumers, current meaning, host registration and discovery. | Additional activation profiles, provider/catalog generality and optional codecs. |
| C2 Dependencies | Complete declared dependencies for supported local operations and actual current validation. | Automatic semantic footprint discovery and additional provider domains. |
| C3 Public products | Complete advertised v1 typed/dynamic arguments, results, outcomes, events and required discovery. | Unadvertised extensions, separately supported transports and broader profile combinations. |
| C4 Editing | Adopted text/selection/Unicode behavior and supported frontend geometry integration. | Deferred structural features, extra tailoring and unsupported presentation profiles. |
| C5 Annotations and history | Approved mapping, restoration, persistence, destination separation and ordinary-save integration. | Collaborative review, optional interchange and unadvertised storage providers. |
| C6 Assets | Required reference integration, actual pinned assets/licenses and bounded supported execution. | Additional language/schema ecosystems and production bundling beyond advertised support. |
| C7 Hosts and packages | Narrow actual compatibility matrix, both adapters/applications and supported provider procedures. | Additional hosts, providers and distribution promises. |
| C8 Resources and observation | Safe ownership/discharge, validated bounds and critical-path diagnostics with supported-workload measurements. | Additional exporters, workloads and stronger physical/global promises not made by v1. |
| C9 Assurance | Claim-to-evidence trace, model reconciliation, current frozen verification and implementation acceptance. | Broader independent certification and exhaustive extra proof depth beyond release claims. |

## Evidence and completion discipline

Preserve formal/reference work already completed. Choose new proof work by the
risk and observability of the shipped behavior: publication/conflict handling,
history/mapping, persistence and resource ownership come first. Every adopted
behavior still needs a defined proof or test obligation; private implementation
choices need not each introduce a new public protocol or a mirrored subsystem.

A simpler owned-handle design may replace elaborate registry machinery only if
it preserves the observable contract and proves equivalent safety for supported
paths. Lost replies, inaccessible installed contexts, partial publication,
premature refunds and unbounded retention remain defects, not deferred features.

Report specification/evidence progress separately from implementation acceptance
and release readiness. The earlier approximate 50 percent estimate is withdrawn;
it was not grounded in a fixed completion measure. Do not derive a new
percentage from test counts or these unequally sized rows.
