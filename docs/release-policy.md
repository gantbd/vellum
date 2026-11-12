# Release and distribution policy

Status: source distribution only; no crates.io publication is authorized.

## Current contract

- Every workspace and qualification package has effective `publish = false`.
- Vellum may be cloned, built, tested, and installed from source under the
  repository licenses.
- Vellum's four Keystone packages use one exact full-revision Git source. A
  sibling checkout is optional for builds and is used only to inspect the
  pinned object during local qualification.
- The nonpublishing `vellum` distribution package owns no runtime semantics. It
  installs exactly the `vellum-daemon` and `vellum-mcp` peer executables from
  one Cargo invocation; their implementation and independent package ownership
  remain unchanged.
- The coordinated source manifest pins the reviewed Keystone commit and names
  the only Keystone crates in Vellum's current closure. A local checkout of the
  object is not evidence that the commit is retrievable from GitHub.
- `vellum-tui` and `vellum-gui-macos` are independent presentation
  repositories and are outside this release unit.
- `vellum-semantic-mcp` is likewise an independently materialized peer. Its
  exact-Git source shape is qualified in coordinated CI, but it remains outside
  the first daemon/native-MCP release candidate while semantic-local provider
  confinement is unclosed.
- A GitHub tag or source archive is not, by itself, a production-support or
  compatibility promise.

The root `Cargo.toml` is the single policy source for workspace members.
Individual packages may inherit `workspace.package.publish` or set
`publish = false` directly, but may not name an allowed registry.

## Gates

`make check-release-contract` verifies the source-level policy without network
access. It checks the workspace taxonomy, package publication settings,
required legal/security documents, the pinned Rust toolchain, and the
repository metadata used by GitHub distributions. `make check` also performs
Cargo metadata, dependency/effect-boundary, compatibility-baseline,
supply-chain/SBOM, formatting, lint, test, and documentation checks. The
source-policy gate additionally inspects a local copy of the pinned Keystone
object when one is available.
Coordinated CI also verifies both MCP extraction closures and materializes the
semantic MCP peer from one committed Vellum tree, then exercises its locked
tests, strict lint and exact-revision Git installation through local URL
rewrites. This proves the distribution shape without creating a remote.
The supply-chain gate and its explicit nonclaims are documented in
[`supply-chain.md`](supply-chain.md).

[`compatibility-baseline.md`](compatibility-baseline.md) defines the exact,
conservative package/source/wire change detector used by
`make check-compatibility`. It is deliberately not represented as Rust semver
analysis or as proof that every possible wire message is compatible.

Distribution claims must also remain within the applicable tier in the
[STRIDE/LINDDUN threat model](security-threat-model.md). Source availability
does not close the local-daemon, multi-principal, semantic-peer or remote-service
security gates.

The GitHub release-contract workflow intentionally runs the source-level gate
from a lone checkout. Full build CI also checks out the exact qualified
Keystone revision so policy can inspect the object independently of Cargo's
exact Git resolution. Coordinated CI runs both the synthetic verifier fixture
and the actual committed clean-archive, immutable-Git Cargo installation
qualification. That continuous local-object evidence does not establish that
either commit is retrievable from GitHub.

## Coordinated GitHub source qualification

[`release/github-source.json`](../release/github-source.json) is the
machine-readable source contract. It records the optional local verification
path, pins Keystone by full commit, excludes `vellum-tui` and `vellum-gui-macos`, and
identifies `vellum-mcp` as a peer connector to an already-running daemon.

From a Vellum checkout with the pinned Keystone object available at the local
verification path, run:

```sh
node scripts/qualify-github-source.mjs --vellum-ref=HEAD
```

The probe creates a fresh Vellum archive, checks effective non-publication and
the exact Keystone Git dependency closure, and maps both declared repository
URLs to the inspected local Git objects without changing Cargo's source
identities. It prefetches the locked graph into a fresh private `CARGO_HOME`,
then builds the archived workspace and executes the documented
`cargo install --locked --offline --git ... --rev ... vellum` command into a
fresh install root and target directory. It then executes
both installed peer help paths plus an empty-invocation refusal. It requires the
exact two-binary closure, rejects symlink/non-regular/group- or world-writable
payloads, excludes TUI/GUI binaries, and checks that refusal output discloses
none of its temporary source/build paths. Ambient `CARGO_*` overrides, Rust
flags, and compiler wrappers are removed from the child environment. It separately performs
a release workspace build with `--locked`, after injecting
a temporary direct dependency and requiring that the locked build refuse. This
proves the archive build consumes the workspace lock.

Cargo's path installer remains local convenience and is not credited with
immutable-source reproducibility. The locked workspace build and locked Git
install are separate required evidence: the former proves the archive consumes
the workspace lock, while the latter proves the documented one-package command
selects the exact commit and binary closure. The probe never installs or
releases either presentation repository.

`node scripts/qualify-github-source.test.mjs` constructs two temporary Git
repositories and exercises the complete archive, exact-Git resolution,
lock-refusal, install and help path. This qualifies the verifier before its own
files are committed; it is not evidence that the current Vellum `HEAD` contains
those files.

The verifier does not copy or read registry sources from an ambient Cargo home.
Consequently a genuinely offline payload must carry a qualified dependency
source closure; the current Vellum archive does not. The synthetic verifier
fixture has only local exact-Git dependencies and proves
the isolation, exact binary closure, adversarial filesystem checks and process
behavior—not third-party retrieval. Before calling a bundle remotely
reproducible, also run `--verify-remote`, which requires network access and
proves that GitHub advertises both the selected Vellum commit and the pinned
Keystone commit:

```sh
node scripts/qualify-github-source.mjs --vellum-ref=RELEASE_COMMIT --verify-remote
```

Current remote-distribution blocker: the local Vellum and Keystone checkouts
have no configured remotes, so availability of both release objects at their
declared GitHub URLs has not been established. Silently following a moving
branch is not an acceptable workaround.

## Semantic-provider qualification

Semantic-provider evidence has two fail-closed tiers. The deterministic tier,
`make check-semantic-provider-qualification`, runs the standalone excluded
qualification package with its own lockfile and no ambient provider binary. It
is part of ordinary coordinated CI and proves the bounded fixture/protocol
contract only.

The live tier, `make qualify-semantic-provider`, requires both an explicit
checked-in profile and an absolute verified executable path. The manual and
weekly `semantic-provider-qualification.yml` workflow obtains the immutable
official rust-analyzer 2026-11-09 macOS ARM64 gzip asset from its fixed HTTPS
release URL, verifies both the published archive digest and the checked-in
uncompressed executable digest, and then invokes the live target against the
fixed profile. There are no workflow artifact inputs or moving repository
variables. Missing profile, download or decompression failure, digest mismatch,
unsupported host, provider
identity mismatch, missing readiness evidence, or any semantic-operation
failure is a failed job—not a skip or a passing absence.

A live receipt is evidence only for the pinned provider artifact, platform,
profile, fixture, readiness schema and operations named by that receipt. It is
not general language-server compatibility and does not qualify transitive
Cargo/rustc/build-script/proc-macro execution, network or process confinement,
CPU/RSS policy, automatic restart, distribution, activation, or publication.
No provider artifact is uploaded, installed, or added to a release candidate
by this workflow.

The local release-candidate qualifier also executes the exact packaged launchd
renderer for both the candidate and its selected distinct rollback version,
and executes only `--help` on each packaged activation executor. Both are
locked workspace-built Rust executables and add no Node runtime requirement to
the artifact. The renderer
uses only private scratch paths, requires the exact inert read-only profile and
`plutil` acceptance, and retains content-free dispositions rather than the
rendered plist or deployment paths. This is packaged-renderer compatibility
evidence, not authorization to install a plist, invoke `launchctl`, activate a
service or claim authenticated health.

`make check-cargo-install-qualification` exercises the successful synthetic
contract without remotes or network. `make qualify-github-source` applies the
same verifier to the actual committed `HEAD` and never substitutes dirty
working-tree bytes for either revision. Coordinated CI requires both commands
after checking out the exact Vellum and Keystone revisions. Remote reachability
remains a separate, explicitly deferred `--verify-remote` gate.

The source archive is not a final release artifact. Checksummed manifests,
signatures/provenance and immutable release-bundle assembly remain separate
gates after both repository objects are remotely retrievable.

## Non-publishing release candidates

The manually dispatched `release-candidate.yml` workflow checks out the exact
Vellum workflow revision and the Keystone commit pinned by the coordinated
source manifest. On macOS its product scope remains exactly `vellum-daemon`
and `vellum-mcp`, both built from the installation-only `vellum` package; it
also builds the two separately classified launchd
deployment tools. It stages the product binaries, executes each staged
`--help` path, and checksum-closes the inert, relocatable
`vellum-launchd-render` helper and activation executor. It uploads a short-lived Actions artifact
containing the two binaries, the renderer, the separately authorized activation
executor, a content-safe `manifest.json`, and `SHA256SUMS`. The renderer emits a
fail-closed read-only LaunchAgent plist but
does not install or activate it; see the
[launchd distribution profile](launchd-distribution.md).

The workflow invokes the clean-source qualifier at the selected commit. That
qualifier makes detached local clones of the exact Vellum and Keystone commits,
sets Cargo offline, builds the locked candidate twice into distinct scratch
products, and refuses upload unless their complete regular-file closures are
byte-identical. It uploads a separate content-safe byte-equivalence record
beside one candidate. This is evidence for those two materializations under one
workflow environment; it is not a cross-run reproducible-build, signature, or
attestation claim. Local source clones and exact URL rewrites are used; the
qualifier performs no remote fetch, release, package publication, or activation.

`vellum-semantic-local`, `vellum-semantic-mcp`, and all TUI/GUI applications
are explicitly outside this artifact set. In particular, semantic-local must
not enter a production distribution until immutable executable/loader identity,
provider confinement, physical resource policy, abrupt-owner cleanup,
operator-visible reconciliation of permanently fenced launch obligations, and
bounded restart qualification are closed. Its current single-use observed
identity, retained join ownership and core-dump suppression do not close those
distribution gates.

The same clean committed-source operation is available locally. Working-tree
changes are neither consumed nor required to be discarded; the exact selected
commit is cloned into private scratch authority. The coordinated Keystone
commit must be present locally, and Cargo remains offline:

```sh
node scripts/qualify-clean-release-candidate.mjs \
  --vellum-ref="$(git rev-parse HEAD)" \
  --output-dir=../vellum-qualified-candidate
```

This is an unsigned, unattested candidate artifact—not a GitHub Release, a
crates.io publication, a self-contained source archive, or a production
support claim. `vellum-tui` and `vellum-gui-macos` remain outside its closure.
The workflow uses read-only repository permissions and commit-pinned checkout
and upload actions; it never receives release or package-write authority.

The candidate is not activation-ready merely because it built. The offline
[artifact lifecycle](artifact-lifecycle.md) separately validates its immutable
content and effective build inputs, re-runs smoke probes, and requires a
distinct verified prior candidate before emitting readiness and rollback
evidence. That evidence explicitly does not authorize or perform activation.

The uploaded candidate also contains the separate
[`vellum-launchd-activate`](launchd-activation.md) package. It remains
deliberately absent from the `vellum` Cargo-install binary closure. Its only
effectful CLI path requires an explicit execution switch, the exact
qualification evidence, a separately affirmative private approval, exact
candidate/prior daemon and plist identities, and an empty private transaction
directory. Build and workflow qualification checksum it and exercise only its
help path plus injected fake effects; neither invokes `launchctl`. A real terminal receipt is local
activation/rollback evidence, not a GitHub Release, signature, attestation or
multi-principal authorization claim.

## Enabling publication later

Publication requires a deliberate change that, at minimum:

1. selects the packages and registry explicitly;
2. replaces or qualifies every cross-repository path dependency for the chosen
   distribution channel;
3. verifies package contents from a clean archive and a clean Cargo home;
4. adds provenance, checksums, signed/tagged release evidence, dependency and
   license review, and supported-platform declarations;
5. updates this policy, the changelog, and installation documentation in the
   same reviewed change; and
6. changes the fail-closed release gate rather than bypassing it.

Until all of those conditions are met, crates.io publication remains disabled.
