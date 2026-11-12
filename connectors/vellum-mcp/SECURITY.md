# Security policy

Vellum MCP has not made a production release. Only the current `main` branch is
considered for security fixes.

Report suspected vulnerabilities through GitHub private vulnerability
reporting for the `gantbd/vellum-mcp` repository when available, or establish a
private channel with the repository owner through their GitHub profile. Do not
include credentials, real user content, or unredacted filesystem paths in a
public issue, discussion, transcript, or fixture.

Include the affected revision, impact, preconditions, and a minimal synthetic
reproduction. Receipt does not promise a response or remediation deadline.

This process is a local stdio projection over an already-running Vellum daemon.
It is not an authentication provider, authorization authority, sandbox, remote
MCP service, or durable audit system. The invoking harness and daemon operator
retain responsibility for identity, authority, process supervision, private
socket ownership, transcript retention, and filesystem exposure.

Resource identifiers are closed `vellum://workspace/{id}/document/{id}` or
`vellum://workspace/{id}/root/{id}` forms. They cannot name filesystem paths or
carry authority. Inventory pages and change events expose workspace shape and
revision activity but not locators or document content; reading an explicitly
selected document intentionally releases its text to the invoking harness.
Membership locator inputs are sent to the daemon but are never reflected in MCP
results or metric dimensions.

Authenticated create-exclusive export carries its exact destination in the MCP
request and durable native intent so later lookup can detect changed arguments.
That destination may therefore remain in the invoking harness transcript and
the daemon's private plaintext journal. It is absent from export receipts,
discovery, errors, debug output and metric dimensions. The connector does not
turn syntax support into authority: the daemon must independently match its
exact authenticated operation, document, revision and destination policy.

After a durable edit reports `service_outcome_unknown`, only a retry on the
same supervised stdio process lifetime carrying its opaque operation handle and otherwise identical
arguments is admitted. It reuses the exact retained native request identity and
payload. Treating an MCP JSON-RPC id as an idempotency key, changing arguments,
or starting another operation does not resolve the uncertainty fence.
