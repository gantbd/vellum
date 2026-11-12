# Official MCP client interoperability qualification

This nonshipping package qualifies the exact built `vellum-daemon` and
`vellum-mcp` binaries through the official TypeScript MCP client. It is not a
runtime dependency of Vellum or of the extracted MCP peer.

The profile pins MCP `2026-07-28` and `@modelcontextprotocol/client` `2.4.0`.
One connection uses automatic discovery and must settle on the modern era;
another pins the modern revision and therefore has no legacy fallback. Both
must observe `server/discover`, exact protocol negotiation, tools, resources,
one document read, one exact locator resolution, one snapshot call, one atomic
workspace edit, one exact-revision durable save and clean stdio EOF/disconnect.
The ordinary catalog is exactly eight tools and excludes every
restart-stable durable tool; the authenticated catalog adds exactly execute,
receipt lookup, retirement and content-free status.

## Locked acquisition

Dependency acquisition is deliberately separate from source preparation. From
the Vellum repository root, create the candidate lock without lifecycle
scripts:

```sh
npm install --package-lock-only --ignore-scripts --no-audit --no-fund \
  --prefix qualification/mcp-official-client
```

Then review the complete `package-lock.json`, emit and review the exact
CycloneDX candidate, and add both with the source change:

```sh
node qualification/mcp-official-client/verify-source.mjs --emit-sbom=-
```

After the reviewed lock and `source-sbom.json` are present, acquire only that
locked closure and run the qualification:

```sh
npm ci --ignore-scripts --no-audit --no-fund \
  --prefix qualification/mcp-official-client
make check-mcp-official-client
```

`verify-source.mjs` refuses Git/link dependencies, lifecycle scripts,
non-registry sources, missing SHA-512 integrity, missing licenses, drift of the
reviewed official client/core identities, an incomplete SBOM or any unlocked
component. The generated `node_modules` directory is never a release input.

The official client source is the verified `v2.4.0` release of
`modelcontextprotocol/typescript-sdk` (`951fe56`), licensed Apache-2.0. Its
published client and core tarball identities and integrity values are frozen in
`source-policy.json`.

The reviewed lock and SBOM now retain 13 exact registry components. The
qualification passes automatic and pinned modern negotiation, eight ordinary
and twelve authenticated tools, locator resolution without path disclosure,
resource read, snapshot, atomic workspace edit, exact-revision durable save and
clean EOF/disconnect.
`make qualify-mcp-official-client` remains the sole interoperability gate and
fails closed when any reviewed acquisition evidence or installed locked closure
is absent.
