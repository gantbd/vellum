# Offline artifact lifecycle

Status: local release-candidate contract only. Nothing in this lifecycle
publishes, installs, activates, signs, attests, or mutates a remote service.

## States and evidence

The candidate builder produces two product binaries, the inert
`vellum-launchd-render` helper, the explicitly authorized
`vellum-launchd-activate` executor, and canonical `manifest.json` and
`SHA256SUMS` files. Manifest schema 4 identifies the exact Vellum and Keystone
commits, effective Cargo build selection, Rust/Cargo versions, target, profile,
feature mode, path-remap environment, and hashes of the lock, compatibility,
dependency, SBOM, source-policy, toolchain, renderer, and activation-executor
inputs. The manifest is itself
checksummed. This is deterministic content evidence; it is not a signature or
attestation.

`qualify-release-candidate.mjs` is the only transition from `built` to
`ready-for-manual-activation-review`. It refuses:

- missing, extra, symbolic-link, non-regular, corrupt, or reordered content;
- unknown manifest schemas, fields, claims, artifacts, targets, or build modes;
- a candidate whose captured policy inputs differ from the reviewing checkout;
- a candidate built for a different runtime target;
- failed or non-self-identifying `--help` smoke probes; and
- an absent, identical, corrupt, incompatible, or unsmokable rollback candidate;
  and
- a candidate or rollback renderer that cannot reproduce the exact inert plist
  from a fresh private deployment record or whose output fails `plutil`; or
- a candidate or rollback activation executor whose checksum, executable mode,
  identity, or explicit-authorization help contract differs.

Both deployment tools are checksum-closed and must remain executable. Their
adversarial qualifications are `make check-launchd-distribution` and `make
check-launchd-activation`. Candidate
qualification additionally executes the exact packaged helper for both
selected versions against fresh same-owner private scratch state and validates
the exact output with `plutil`; it does not install a plist, invoke `launchctl`
or simulate launchd process supervision. The helper is a locked workspace-built
Rust executable, so the distributed artifact requires no Node runtime. See the
[launchd distribution profile](launchd-distribution.md).
The activator qualification invokes only `--help`, records
`launchdEffect: not-attempted`, and cannot reach the explicit effect switch. The candidate
manifest explicitly denies launchd installation, launchd socket activation,
activation, and authenticated-health claims.

The emitted qualification record names both immutable manifest identities,
artifact hashes, content-safe binary, renderer, and activator-help smoke
dispositions, and the
selected prior Vellum revision. Renderer evidence contains no scratch path,
configuration or plist. The record sets `activationAuthorized` to `false`.
Readiness is therefore a gate input for a future separately authorized
activation mechanism, not an activation side effect.

The GitHub workflow materializes the complete candidate twice through separate
builder invocations and uploads only after their regular-file closures are
byte-identical. The separate equivalence record names file hashes and exact
build/source identities. It deliberately does not claim independent build
environments, a reproducible build across runs, signing, or attestation.

## Local qualification

After building a new candidate and retaining a distinct prior candidate:

```sh
node scripts/qualify-release-candidate.mjs \
  --candidate-dir=/absolute/path/to/candidate \
  --prior-candidate-dir=/absolute/path/to/prior \
  --evidence-file=/absolute/path/to/new-evidence.json \
  --repository-root="$PWD"
```

The evidence destination must already have a real parent directory and must
not exist. The tool creates it exclusively only after every validation and
smoke gate succeeds. It does not choose an installation location.

## Local activation transaction model

`activation-transaction-lib.mjs` supplies the release layer's closed
activation/rollback algebra and a non-activating scratch simulator. An exact
canonical `0600` approval binds one transaction identifier, candidate manifest
digest, distinct prior manifest digest, requested action, and affirmative
authorization. Unknown fields, noncanonical JSON, aliases, wrong ownership or
mode, changed identities, and absent authorization refuse before the scratch
root is changed.

The simulator stages the candidate and prior daemon, MCP peer, and rendered
plist into separate digest-named version directories. Every input is a
canonical same-owner regular file with an exact SHA-256 identity; executable
payloads become `0500`, plists and version identities become `0400`, and the
completed directory becomes `0500`. Files and directories are synchronized
before publication. Existing immutable versions are accepted only after their
complete identity, mode, file closure, and hashes revalidate.

One canonical `0600` desired-deployment pointer selects an entire immutable
version and is replaced through write, file sync, rename, and parent-directory
sync. The accompanying durable journal records prepared, candidate-effect,
rollback-pending, rollback-effect, terminal, refused, and unknown states.
Deterministic crash points exercise both known resumable cuts and journal/pointer
divergence; divergence is always reported as `outcome-unknown`. Receipts contain
only transaction/digest identities, phase, outcome, generation, and fixed
policy labels—never scratch paths or payload content.

This is deliberately **one desired-state pointer only**. The pointer is not a
symlink, launcher, loaded launchd job, or claim that the plist and a running
process changed atomically. The simulator never invokes `launchctl` or executes
the staged binaries. Its receipts always state `launchdEffect: not-attempted`.
The separate authorized platform executor consumes the same exact
qualification and approval identities. Its own append-only synchronized
journal brackets real launchd effects; it preserves `outcome-unknown` whenever
invocation may have occurred without matching confirmation. The scratch
pointer remains non-activating evidence and is not silently promoted into a
loaded-job fact.

## Activation and rollback boundary

The separate [`vellum-launchd-activate`](launchd-activation.md) package now
consumes the existing exact qualification and affirmative approval contracts.
It requires the selected prior job to be exact and healthy, durably brackets
each launchctl effect, validates candidate process identity and bounded daemon
status, and restores and requalifies the exact prior version when candidate
health fails. An effect without post-invocation confirmation remains
`outcome_unknown` and is never retried automatically.

This does not authorize activation by qualification, builds, Cargo install or
release workflows. The executor requires a separate execution switch and
private operator-created approval/configuration. It selects launchd daemon
state only; MCP replacement, source retrieval, signing and remote policy remain
separate contracts.

The qualification record alone does not prove source retrieval, reproducible
bytes across builders, malware absence, signature identity, notarization,
launchd supervision, successful activation, or successful rollback. Those last
two claims require a terminal executor receipt from the exact approved local
transaction.
