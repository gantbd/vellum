# Vellum

Vellum is a headless, agent-friendly Rust editing engine for code and prose.
Its engine owns authoritative source and history state, bounded admission,
retained outcomes, and content-safe instrumentation. The repository also owns
the adapter-neutral host, native protocol/service/client profile, qualified
first-party adapters, and non-UI composition roots. Inbound connectors are
independently versioned peer repositories. These remain
separate crates with enforced dependency direction.

The workspace is organized by architectural role rather than packaging type:

- `engine/` — categorically closed `vellum-engine` semantics
- `host/` — trusted ports, session/workspace authority, and editor controller
- `protocol/`, `client/`, `service/`, and `transport/` — native service contract
- `adapters/` — qualified outbound effect interpretations
- `applications/` — non-UI executable composition roots such as the daemon
- `qualification/` — unpublished conformance and live-platform probes
- `sdk/` — zero-authority embedded and client distribution facades

Presentation projects remain independent:

- `vellum-tui` — terminal composition root and `vellum` executable
- `vellum-gui-macos` — native macOS GUI composition root

The consolidated Git graph retains the former `vellum-sdk` and
`vellum-adapters` histories. See [project boundaries](docs/project-boundaries.md)
for the dependency law. Superseded layouts remain audit evidence, not active
package boundaries.

## Verify

```sh
make check
```

Development and CI use the toolchain pinned by `rust-toolchain.toml`. See the
[release policy](docs/release-policy.md), [security policy](SECURITY.md), and
[changelog](CHANGELOG.md) for the source-distribution contract.
Pre-public byte-exact verification snapshots are intentionally excluded from
the source tree; their scope, nonclaims, and private recovery identities are
recorded in the [legacy evidence catalog](docs/legacy-evidence.md).

Shared Keystone mechanisms, including the canonical wire codec, resolve from
one exact Git revision. A sibling checkout is used only by local qualification
to inspect that pinned object; ordinary Cargo builds do not compile sibling
working-tree bytes. The engine crate itself has no upward
Vellum, UI, filesystem, clipboard, ICU, service-runtime, or
application-framework dependency.

Install the Vellum daemon from one source checkout with:

```sh
cargo install --locked --path distribution/cargo-install
```

That installation-only package contains no engine behavior. It installs only
the `vellum-daemon` service. MCP connectors are independently owned peers. The
equivalent command for a remotely available Vellum release commit is:

```sh
cargo install --locked --git https://github.com/gantbd/vellum --rev RELEASE_COMMIT vellum
```

The Git form is locally qualified against immutable Git objects. Remote
installability remains deliberately unclaimed until the pinned Keystone object
and the Vellum release commit are both reachable from their declared URLs.

An exact GitHub candidate download can instead be verified and installed into
one new private local root without Cargo or crates.io:

```sh
mkdir -m 0700 /absolute/private/vellum
node scripts/install-release-candidate.mjs \
  --bundle-file=/absolute/download/vellum-release-candidate.tar \
  --install-root=/absolute/private/vellum
```

Run the installer from the exact Vellum source revision that produced the
candidate. It installs only `bin/vellum-daemon`; it does
not alter `PATH`, install a plist, invoke launchd, activate a service, fetch a
dependency, overwrite an installation, or install the packaged deployment
tools. See the [artifact lifecycle](docs/artifact-lifecycle.md).

Run the headless example with:

```sh
cargo run --offline -p vellum-engine --example headless_edit
```

Every crate in this repository, as well as the UI repositories, is currently
source-distributed and explicitly non-publishable. Neither UI is part of this
workspace or a dependency of the other. No crate is being released to
crates.io yet; `make check-release-contract` fails if that policy drifts.
