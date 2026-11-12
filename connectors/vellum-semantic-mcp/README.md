# Vellum Semantic MCP

`vellum-semantic-mcp` is the bounded agent-facing semantic peer for Vellum. It
projects symbol navigation, bounded incoming/outgoing call hierarchy, diagnostics,
type information and rename planning
over MCP `2026-07-28`; it cannot apply edits or acquire provider authority.

The useful local composition requires explicit native and semantic service
sockets, private credential files and one bounded language label:

```sh
vellum-semantic-mcp \
  --semantic-socket PRIVATE_SEMANTIC_SOCKET \
  --semantic-credential PRIVATE_SEMANTIC_CREDENTIAL \
  --native-socket PRIVATE_VELLUM_SOCKET \
  --native-credential PRIVATE_VELLUM_CREDENTIAL \
  --document-language rust
```

The binary keeps native snapshots synchronized at MCP request boundaries and
fails closed on uncertain mutations. `--unavailable` is a qualification mode,
not a useful semantic backend. No remote network, implicit write, crates.io
publication or provider confinement claim is made.

Build and test from the Vellum workspace with:

```sh
cargo test --locked --offline -p vellum-semantic-mcp --all-targets
```
