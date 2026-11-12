# Vellum semantic MCP repository extraction

Status: public SDK boundary and deterministic isolated extraction are qualified;
the local sibling repository was materialized from Vellum
`25afc5a38af99963c15df4488cb42f93786be3ff` and committed as
`0c713411c26caefa683cba11ba762fd2b9c72ce7`. No remote or publication is
configured.

`vellum-semantic-mcp` is an inbound peer beside the native MCP, TUI and GUI. It
depends on three Vellum distribution boundaries only:

- `vellum-sdk-client` for native workspace snapshots and change events;
- `vellum-sdk-semantic-client` for semantic protocol, witnessed sessions,
  bounded transport and the explicit unavailable qualification composition;
- `vellum-adapter-local-authentication` for private-file credential proofs.

The semantic SDK facade adds no behavior or role. The peer cannot depend
directly on the engine, editor, semantic implementation crates, LSP adapter or
semantic-local application.

The machine contract is
[`release/vellum-semantic-mcp-extraction.json`](../release/vellum-semantic-mcp-extraction.json).
Run:

```sh
node scripts/verify-semantic-mcp-extraction.mjs
```

The verifier copies only declared regular non-symlink files into a fresh
archive, builds and tests it against the two SDK facades and credential adapter,
runs warnings-denied Clippy, path-installs the binary and smokes `--help`.

After committing the Vellum source boundary, materialize exact Git-bound peer
source with:

```sh
node scripts/emit-semantic-mcp-peer-source.mjs \
  --output=.peer-staging/vellum-semantic-mcp
```

The emitter reads bytes from one committed tree, pins the exact Vellum and
Keystone revisions, generates a lockfile, tests and lints the peer, then
qualifies `cargo install --locked --git ... --rev ...` through local URL
rewrites. Those rewrites are qualification mechanics; emitted manifests retain
the public Git identities. Provider process qualification, confinement and the
native/semantic service integration remain Vellum-owned release gates.

Extraction does not authorize deleting the in-tree connector. Removal requires
a reviewed sibling commit, compatibility parity, an official-client
interoperability run and coordinated distribution changes. No crates.io package,
GitHub Release, signing, activation or remote service is claimed.
