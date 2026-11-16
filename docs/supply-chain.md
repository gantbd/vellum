# Dependency and source-SBOM policy

Status: offline source-closure gate implemented; the locked graph has no source
identity or pinned-API blockers. Remote provenance remains unverified.

`release/dependency-policy.json` is the fail-closed machine policy for the
locked Cargo graph. It permits only the canonical crates.io registry source,
the reviewed SPDX expressions currently present in the lock closure, Vellum
workspace paths, and the four named Keystone packages at one exact reviewed Git
revision. Every other Git source is forbidden. Registry packages require Cargo
SHA-256 checksums; missing licenses, unknown registries, extra path packages,
path escapes, lock/metadata disagreement, and unapproved Git sources fail.

Run the policy and its adversarial fixtures without network access:

```sh
make check-supply-chain
```

Generate the deterministic CycloneDX 1.5-style source inventory on stdout:

```sh
make sbom > vellum-source-sbom.cdx.json
```

The generated document contains package names, locked versions, approved
license expressions, registry checksums, normalized source classes and the
Cargo dependency graph. It omits timestamps, absolute paths, usernames and
Cargo cache locations. The legacy `MIT/Apache-2.0` declaration is retained as
declared evidence and normalized to `MIT OR Apache-2.0` in the CycloneDX license
expression. `release/source-sbom.manifest.json` pins the lock digest,
SBOM digest, component count and blockers so dependency drift requires an
explicit review and manifest update.

This is a source inventory, not a legal conclusion, binary SBOM, signature,
attestation, vulnerability scan or proof of reproducible bytes. License
expressions are policy inputs and still require accountable human review.

The optional local-authentication adapter uses pinned `getrandom`, `hmac` and
`sha2` crates; RustCrypto's HMAC closure adds `subtle` 2.6.1. The cached crate
metadata declares exact SPDX `BSD-3-Clause`, and its bundled license text was
reviewed before that expression was admitted to policy. Randomness comes from
the OS CSPRNG through `getrandom`; failure is closed and Vellum does not fall
back to timestamps, socket metadata, process IDs or a weaker pseudo-random
source. Neither credential material, random bytes nor raw OS errors are logged
or projected as operational evidence.

The durable-operation journal adds no registry, Git, or Keystone dependency.
Its adapter uses the standard library and existing locked `rustix` features.
Journal bytes may contain source and locators; they are private deployment data
and never enter the source SBOM, release candidate, metrics, or build
provenance.

## Keystone source closure

`keystone-codec`, `keystone-lifecycle`, `keystone-observe`, and
`keystone-replay` are consumed through six manifest edges from
`2e4faf10ec0a70805aa1db1555f93c77fcf0a1e4`. Cargo.lock records that full source
identity and the policy admits exactly those four packages. Dirty sibling bytes
cannot enter compilation.

The pinned Keystone commit is publicly retrievable from the repository URL in
policy, and Vellum's remote-only source qualifier verifies that exact object.
This establishes source reachability only; it is not a signature, attestation,
release artifact, or crates.io publication claim.

The checked-in source-SBOM manifest closes over the exact locked workspace
components and records zero blockers. The source-policy gate separately verifies that every
required manifest and the `keystone-observe` histogram/saturation API are
present in the pinned commit.

The excluded semantic-provider qualifier has an independent lockfile and an
independent CycloneDX digest manifest. The supply gate evaluates its 72-component
closure separately from the shipping daemon closure; qualification-only
dependencies therefore cannot silently enlarge a release artifact's claimed
SBOM. This records source closure only and does not attest the separately
downloaded provider executable or its transitive toolchain behavior.

## GitHub qualification

The coordinated CI workflow checks out the exact Vellum workflow SHA and
manifest-pinned Keystone SHA for independent policy inspection. Cargo still
resolves Keystone by exact Git source. CI installs the pinned Rust toolchain and
fetches the locked registry graph once,
then runs formatting, Clippy, all-target/all-feature tests, documentation tests,
and all dependency, effect, boundary and release gates offline. The workflow
has read-only repository permission and makes no lone-checkout, publication,
signing or attestation claim.

The cargo-install qualifier uses a newly created private `CARGO_HOME`, install
root and target directory, deletes ambient `CARGO_*`, Rust-flag and compiler-
wrapper authority, and runs Cargo offline. It never copies a developer or CI
cache into that home.
This closes the verifier's ambient-cache confusion but intentionally exposes a
remaining distribution gap: the Vellum source archive does not contain the
crates.io source closure needed by a genuinely empty-cache offline install.
The local exact-Git synthetic fixture qualifies the verifier; it is not a
third-party dependency-availability claim. Ordinary GitHub
installation is intended to fetch one exact Keystone Git revision and the
locked registry graph online, then run the gates offline. A genuinely
empty-cache offline install instead requires a reviewed vendor/source bundle
or prebuilt binaries; the current source archive does not provide it.
