# Modular resource substrates

Parallel implementation starts from the [filesystem adapter handoff](filesystem-adapter-handoff.md).

Resource substrates are modules outside the editor engine. The first target is
a local filesystem module. S3 illustrates a possible later module; it is not a
v1 implementation requirement, a selected cloud dependency or a claim of matching
filesystem guarantees.

The current adopted specification 0.213/model 0.277 preserves the resource-module
boundary and separates clipboard workflows into the headless integration facade.
Its frozen batch passes all 770 checks over 977 inputs, with source/worker/root
endpoint and archive integrity audits. This establishes
the design boundary, not an implemented filesystem provider or its actual guarantees.

## Engine contracts and module mechanisms

The engine owns document incarnations, logical revisions, authored content,
history, capabilities, local publication, immutable save captures and interpretation
of attributed evidence. It depends on engine-owned resource/effect contracts,
not filesystem calls, paths, object-store SDKs or provider credential types.

Each resource module implements the supported resolution, read, write and
completion procedures through those contracts. Locators, handles, credentials,
connections, platform calls and SDK details stay at the substrate edge. The host
wires the modules and grants access; authored content and module availability do
not select or authorize external execution.

Existing IF-10 carries binding/identity evidence, IF-04 carries resource effects
and completions, and IF-13 carries serialization/artifact preparation. Source and
annotation resources retain their explicit roles and distinct ordinary destinations.
The engine remains usable with no resource module attached.

## Capability and identity profiles

A module declares its actual comparison domain, identity/continuity evidence,
read coherence, conflict/precondition checks, write/completion semantics,
acknowledgment strength, supported synchronization and cleanup obligations.
Stronger capabilities are explicit; unsupported requirements refuse without a
silent downgrade. Common contracts do not require every module to emulate rename,
directory hierarchy, filesystem locking or any other provider-specific mechanism.

Provider-scoped opaque evidence is not universally comparable or ordered. A path
or object key does not become engine document identity or permission. Cross-module
aliases require actual supported equivalence evidence; incomparable evidence is
not known distinction. Module replacement cannot reset admitted attempts, erase
unknown effects or release retained responsibility by itself.

One engine-mediated writer coordinates each accepted resolved destination identity.
This does not create a universal external mutex, cross-module transaction or joint
atomic persistence guarantee. Each artifact keeps its own attributed outcome.

## Local filesystem first

Implement and qualify one local filesystem substrate before adding other resource
modules. Its supported OS/procedure profile must state resolution/alias limits,
external-change checking, read coherence, replacement visibility, synchronization,
attribute/security preservation and failure/unknown disposition. Existing ordinary
best-effort save semantics remain the baseline; no universal atomic conditional
write, crash durability or complete alias oracle is assumed.

The module's actual file operations and platform tests remain implementation and
provider-conformance gates. No particular OS matrix, temporary-file naming scheme,
syscall sequence or numeric timeout is selected by the module choice alone.

## Packaging and evidence

Keep the local filesystem implementation separately wired from the semantic engine.
Later modules conform to the same applicable contracts with their own declared
profiles and evidence. This needs neither a runtime plugin loader nor arbitrary
hot swapping, a distributed workflow service or remote identity infrastructure.

Reuse contract scenarios for lossless bytes, exact capture, current authority,
identity uncertainty, conflict, partial/unknown effects, late/duplicate evidence,
cancellation and protected discharge. Actual provider tests establish only the
guarantees they exercise. Formal supplied-evidence checks do not establish the
truth of an OS or service acknowledgment.

INS-08 observes resolution/binding, INS-07 preparation/fidelity, and INS-04
dispatch/completion. Keep engine queue/service, module execution and observation
delay distinct. Do not expose paths, bucket/object names, credentials or opaque
resource tokens in ordinary metric labels or diagnostic payloads.

This resource-module choice is separate from the engine's in-memory source
representation. Rope/piece-tree/gap-buffer evaluation remains behind the engine's
source/snapshot abstraction; a filesystem or object store is not that text backend.

Existing contracts are in [specification §1.2 and §§11/16–17](../specification.md)
and the [substrate boundary evidence](../formal/engine-substrate-boundary.md).
The [closure ledger](specification-closure-ledger.md) retains package/interface
reconciliation and actual implementation/platform evidence separately.
