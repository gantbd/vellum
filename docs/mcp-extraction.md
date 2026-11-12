# Vellum MCP repository extraction

Status: deterministic peer repository payload materialized and qualified; local
sibling Git repository established without a remote.

`vellum-mcp` can be extracted as an independently versioned peer once the
current Vellum work is committed and remotely addressable. Its production
crate consumes the zero-authority public `vellum-sdk-client` facade and the
narrow `vellum-adapter-local-authentication` credential interpreter. The
facade supplies the client, protocol DTOs, canonical wire codec and Unix
transport without giving MCP engine, service, editor or daemon authority. The
adapter supplies only descriptor-anchored credential loading and proof
construction; it grants no Vellum action.

The machine-readable extraction closure is
[`release/vellum-mcp-extraction.json`](../release/vellum-mcp-extraction.json).
Run `node scripts/verify-mcp-extraction.mjs` to copy that closure into a fresh
archive, compile and test it against only the public SDK facade, source-install
the binary, and exercise `--help`. This fixture uses a local SDK path solely to
qualify the boundary; it does not claim that an uncommitted Vellum revision is
available through Git.

Run `node scripts/emit-mcp-peer-source.mjs --output=.peer-staging/vellum-mcp`
to materialize the independent repository tree from committed Vellum bytes.
The emitter writes exact Vellum and Keystone Git revisions, generates a
lockfile, runs the peer tests, initializes a disposable Git stand-in, and
qualifies the documented `cargo install --locked --git ... --rev ...` shape.
Git URL rewrites point qualification at the local Vellum, Keystone, and staged
peer repositories; they do not weaken the emitted public Git identities.

## Current coupling audit

- Production source and ordinary conformance tests need `serde_json`,
  `vellum-sdk-client`, and `vellum-adapter-local-authentication`; the machine
  closure includes the adapter's exact transitive Vellum dependencies.
- There are no external fixtures; the JSON-RPC cases are constructed in the
  conformance test.
- `tests/live_daemon.rs` directly composes the local filesystem, editor,
  service and daemon. It is intentionally excluded from the extraction
  closure. Before deleting `connectors/vellum-mcp` from Vellum, retain this as
  a Vellum-owned coordinated daemon/MCP qualification using built binaries.
- The extracted manifest must replace workspace-inherited metadata and the
  SDK path with an exact Git dependency on the committed Vellum revision. Its
  lockfile must be committed. Its transitive `keystone-codec` edge is already
  pinned to the same qualified full Git revision as Vellum's shipping graph.
  Independent installation still waits for committed, remotely reachable MCP
  and Vellum revisions.
- MCP `2026-07-28` compatibility and stdio behavior move with the connector;
  native request/reply compatibility remains owned by `vellum-sdk-client`.

## Release workflow transition

The present Vellum release-candidate workflow still builds daemon and MCP from
one revision and must remain so until extraction completes. Afterwards Vellum
builds only the daemon. MCP's candidate workflow will need exact MCP and Vellum
revisions; Vellum's lock then carries the immutable Keystone source. A coordinating bundle may
combine independently checksummed artifacts, but no workflow may silently
follow a branch or claim signing, attestation, crates.io publication, or a
GitHub Release.

The coordinated Vellum cargo-install qualifier currently installs MCP as the
second exact peer binary from the Vellum source archive because the declared
extraction still depends on the Vellum public SDK. Its isolated install/root checks do not collapse MCP back into the
daemon or claim that the future sibling repository is independently installable.

The ignored `.peer-staging/vellum-mcp` tree was refreshed from Vellum
`02f2670fd2a886c58c5188dd014958049fbe392d` and copied to the local sibling at
`../vellum-mcp`. Its current reviewed commit is
`8a96a983396aa4bb419340c079b2ac83e9671722`; the locked sibling suite, exact
source-contract test and strict Clippy pass against the Vellum and Keystone
inputs recorded in `release/source.json`.
No remote or publication has been configured. Coordinated live-daemon
qualification remains owned by Vellum until the peer release lifecycle is
available from immutable remote inputs.

## Official-client interoperability

`qualification/mcp-official-client` is a nonshipping qualification package,
not a Vellum or extracted-peer runtime dependency. Its TypeScript dependency
exists only to drive the built Rust daemon and MCP binaries through the
official client API. `make check-mcp-official-client-profile` runs offline and
adversarially qualifies the lock/SBOM policy plus the current seven ordinary
and four authenticated tool expectations. It does not claim interoperability.

The committed npm schema-3 lock and generated CycloneDX source SBOM close 13
exact registry components under SHA-512 integrity; they contain no Git/link
source or lifecycle script. `make qualify-mcp-official-client` verifies that
source identity, requires its already-installed locked closure, builds both
Rust peers offline, and exercises automatic and pinned modern negotiation,
discovery, tool/resource enumeration, exact locator resolution, resource read,
snapshot, atomic workspace edit, and clean EOF/disconnect. CI alone performs
the separate `npm ci --ignore-scripts` acquisition from the exact lock.

The official `@modelcontextprotocol/client` 2.4.0 qualification passes against
the shipped Rust surfaces: automatic and pinned negotiation remain on
`2026-07-28`, the ordinary catalog contains seven tools, the authenticated
catalog contains eleven, both workspace-edit paths complete, and both stdio
children disconnect cleanly. This qualification exposed and closed missing
modern cache metadata on resource reads and non-object durable input schemas.
