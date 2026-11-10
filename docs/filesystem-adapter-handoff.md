# Filesystem adapter implementation handoff

Extend and qualify the existing `vellum-adapter-local-fs` module independently of semantic
engine completion. Start with its conservative macOS profile; do not build a
second filesystem adapter or broaden its guarantees implicitly. This handoff
organizes implementation against existing contracts, not new requirements.

## Starting points

- [Adapter implementation](../adapters/local-fs/src/host.rs) and
  its colocated real-filesystem tests.
- [Resource module design](substrate-module-design.md) and
  [engine API ownership](engine-api-boundaries.md).
- [Specification](../specification.md), especially §1.2 and §§11/16–17.
- [Current implementation overview](../README.md#first-local-filesystem-adapter).
- [Current engine closure worklist](engine-test-plan.md#current-engine-closure-reconciliation).

Read the current code and tests before selecting the first missing case. Design
references establish intended behavior; actual platform tests establish the
adapter's supported guarantees.

## Edit scope and coordination

Own changes under `crates/vellum-adapter-local-fs/**`, adapter-specific tests and this
handoff. Coordinate edits to workspace manifests, dependencies, shared fixtures,
engine resource/evidence types or normative documents with the engine thread.
The SDK/UI stream owns its consumer packages and must not duplicate filesystem
mechanisms. Keep engine/provider integration tests attributable to one stream.

For a missing shared contract, report the exact operation, required observable
behavior, existing type limitation and smallest proposed change. Do not emulate
the missing guarantee behind the engine's back or silently alter receipt meaning.

## Responsibility boundary

The engine owns document identity/revisions, immutable save intent, engine-mediated
writer coordination, receipt validation and eligible baseline adoption. The
adapter owns locator resolution, native handles, byte I/O, identity observations,
replacement/synchronization, metadata handling and truthful execution evidence.
The host wires and authorizes the provider. Availability, a path or authored
content is not authority.

Logical scratch birth now preserves authored U+FEFF separately from output BOM
intent. Keep that distinction at the byte edge: consume engine-encoded captures,
and do not strip/insert signatures or normalize authored content to make a save
possible. Absent-signature ambiguous output must refuse before provider work.
An unbound scratch owner grants no filesystem destination/create/replace right;
the conservative native adapter capability/refusal profile is unchanged.

Preserve original task/dispatch correlation and return application, no-future-effect
and quiescence evidence separately. An I/O error does not establish nonapplication;
a completed replacement does not establish requested synchronization. Unknown or
late effects retain their original responsibility. One engine-mediated writer is
not an external mutex against other processes.

The engine's common native intake is now `ingest_provider_evidence_v1` with the
exhaustive `vellum_engine::resource::ProviderEvidenceV1` source-read,
annotation-read and save variants. Construct evidence with the original
`ProviderPort` and dispatch, then move it into this facade; existing family-local
intake remains available. Do not reconstruct IDs/status flags from a pathname or
descriptor, or discard late evidence because current launch permission changed.
The engine validates original attribution and reconciles duplicates/conflicts;
the adapter remains responsible for truthful/authenticated substrate observation.
Quiescence, no-future-effects, application and synchronization are separate facts.
Intake neither activates read candidates nor publishes saved baselines. See
`tests/provider_evidence_behaviors.rs` for executable public usage.

## Existing supported baseline

The macOS implementation stages replacement in the anchored destination directory,
rechecks the pinned condition, preserves supported ownership/mode and bounded
extended attributes, and reports requested file/directory synchronization. It
supports existing regular local files under its declared procedure, with
best-effort external-change detection.

Keep explicit refusals for unsupported symlinks, hardlinks, nonregular/remote
files, ACLs, flags/modes, ownership or metadata. No in-place fallback is selected.
Replacement changes inode identity and does not preserve creation/access/
modification timestamps. The final check and rename can race an external writer.
Atomic conditional writes, hostile-directory protection, universal crash durability,
exclusive creation and typed causal-version procedures are not established by
this profile. Extend them only through an explicit profile/contract decision and
actual platform qualification; a host-advertised capability alone is insufficient.

## Implementation and acceptance

### First deliverable

Run the existing macOS adapter target, then select one uncovered failure or
instrumentation path inside the current staged-replacement profile. Add a failing
behavior test, implement the smallest adapter-local change and report the focused
result. Existing coverage already includes ordinary saves, scratch export and
Save As, alias changes, metadata refusals, stage collisions and replacement faults;
extend that coverage rather than rebuilding those workflows.

The deliverable is a working adapter improvement with truthful receipts, not a
larger platform promise. If the selected case requires a new engine contract,
raise that dependency and choose an independent adapter-local case meanwhile.

### Acceptance checks

1. Run the existing adapter tests and identify concrete missing behavior or failure
   coverage within the selected profile. Prefer an executable failing scenario
   before changing implementation.
2. Preserve lossless bytes, BOM/newlines and exact pinned captures across open,
   edit, save, repeat save, copy export and Save As where currently supported.
3. Exercise alias changes, external modification, bounded metadata, stage collisions,
   cleanup failure and failures before/after replacement and synchronization.
   Assert both file state and each attributed receipt, including Unknown cases.
4. Test unsupported conditions before writing; refusal must preserve the old file
   and avoid a stronger completion claim. Keep native unsafe seams narrow and
   documented; OS truth is a platform premise, not a Rust proof.
5. Instrument actual resolution/read/probe/stage/write/sync/cleanup paths as
   applicable. Separate engine queue/service, adapter execution and observation
   delay. Test disabled, available, timed and saturated capture equivalence.

Use MELT and Golden/RED for attempts, duration and errors; USE for actual bounded
resources, saturation and resource errors; STRIDE for substitution/races/authority;
LINDDUN for paths, content and identity disclosure. Do not create empty signal
catalogs or emit paths, resource tokens, credentials or raw OS errors in ordinary
diagnostics. Performance is table stakes: measure relevant read/write workloads,
state workload/platform conditions, separate build cost and instrumentation
overhead, and do not invent an accepted latency threshold.

On macOS, start with focused checks:

```sh
cargo test --offline -p vellum-adapter-local-fs
cargo clippy --offline -p vellum-adapter-local-fs --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Narrow tests by scenario while iterating. Coordinate workspace-wide formatting
changes; unrelated files belong to other work. Run broader engine/provider gates
when a shared boundary changes or at an agreed integration milestone, not after
every isolated adapter edit.

## Completion report

Report implemented behaviors, exact OS/procedure support, focused commands and
results, remaining refusals/gaps and any shared-contract changes requiring engine
coordination. Keep model checks, native implementation tests and actual platform
guarantees distinct. Adapter completion must not be reported as engine completion.
