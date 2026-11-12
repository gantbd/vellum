# Vellum MCP repository extraction

Status: boundary prepared and archive fixture qualified; sibling repository not
created.

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

Creating `/Users/brian/Workspace/github.com/gantbd/vellum-mcp` is outside this
repository's current write authority. Extraction therefore stops at the
verified archive contract. No sibling directory, Git repository, remote, or
history rewrite has been created.
