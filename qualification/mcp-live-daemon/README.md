# MCP live-daemon qualification

This nonshipping package is owned by Vellum because it coordinates the native
daemon, local persistence adapters, and the independently owned `vellum-mcp`
peer. It contains no connector production source and is excluded from Vellum's
shipping workspace.

The harness must provide `VELLUM_MCP_BIN` naming the already-built sibling peer
binary. The tests retain Vellum's exact daemon/MCP interoperability, restart,
durability, uncertainty, and authenticated-receipt evidence without making the
connector part of the engine repository.
