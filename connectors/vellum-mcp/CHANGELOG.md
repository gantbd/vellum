# Changelog

## Unreleased

- Closed the peer extraction and official-client catalog contract over the
  exact six ordinary tools plus four authenticated durable additions. The
  ordinary catalog now includes bounded atomic `vellum.workspace_edit`; the
  authenticated catalog also includes content-free `vellum.durable_status`.
- Added an authenticated-transport-only projection of native durable execute,
  pure receipt lookup, and exact receipt retirement. Harnesses supply and
  persist canonical nonzero
  128-bit `vdo1` references before the first effect; changed intents conflict,
  pending remains fenced, and absent/foreign lookup never executes. Retirement
  delegates only to the native retirement port and does not grant the daemon's
  distinct `RetireReceipt` authority.
- Added content-free durable outcome counters and real daemon/client restart
  qualification, including retirement followed by absorbing absence.
- Added create-exclusive document-copy export to the authenticated durable
  execute/receipt/retirement intent union. Exact daemon policy remains
  authoritative;
  responses omit destination and source bytes, and malformed cross-intent
  receipts fail closed.
- Added optional path-only local credential loading. The credential-enabled
  binary uses the authenticated connector/catalog; omission remains owner-only
  offline, and authentication never grants a Vellum action.

All notable Vellum MCP changes will be recorded here. No public release has
been made.

## [Unreleased]

### Changed

- Prepared the connector to consume Vellum only through the public client SDK.
- Preserved the bounded MCP `2026-07-28` stdio profile and content-free
  instrumentation.
- Added current-version per-request discovery, paginated content-free
  workspace resources, self-contained document selection, bounded change polling,
  and authority-mediated membership through the existing native SDK contract.
- Preserved exact native request identity when an identical durable-edit retry
  follows `service_outcome_unknown`; different work remains fenced.

### Compatibility

- The protocol version remains `2026-07-28`; legacy `initialize` remains
  unsupported because this version removed it. The resource capability and two
  additional tools are additive for capability-discovering clients. Clients
  that hard-code the former three-tool catalog must update; unsupported
  versions remain rejected with the exact supported-version list.

[Unreleased]: https://github.com/gantbd/vellum-mcp/commits/main
