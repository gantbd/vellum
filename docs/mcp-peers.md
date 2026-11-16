# MCP peer ownership

Vellum owns the native daemon, protocols, SDK facades, and coordinated
compatibility qualifications. It does not own MCP connector production source.

The independently versioned inbound peers are `vellum-mcp`, the ordinary
workspace and durable-operation projection, and `vellum-semantic-mcp`, the
bounded semantic-query projection. Their exact reviewed source revisions and
Vellum-facing contracts are recorded in
[`release/mcp-peers.json`](../release/mcp-peers.json).

Each peer owns its manifest, lockfile, source tests, payload contract, install
command, and release lifecycle. Vellum consumes the peers only through public
SDK and authentication facades. Vellum deliberately retains the qualifications
that cross the ownership boundary: the native daemon/live MCP journey, the
official-client profile, the semantic local application, and semantic provider
qualification. These gates may build an explicit sibling checkout, but
Vellum's shipping workspace, source-install package, candidate artifact
closure, and activation state do not contain or install an MCP executable.

No crates.io publication, remote reachability, coordinated clean-tree release,
signing, or activation is claimed by this record.
