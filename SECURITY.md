# Security policy

## Supported versions

Vellum has not made a production release. The `main` branch is the only line
currently considered for security fixes; historical snapshots and untagged
source archives are unsupported.

## Reporting a vulnerability

Do not disclose a suspected vulnerability in a public issue, discussion, pull
request, chat room, or test fixture. Use GitHub's private vulnerability
reporting for this repository when it is enabled. If that facility is not
available, contact the repository owner privately through their GitHub profile
and provide only enough information to establish a confidential channel.

Include the affected revision, impact, preconditions, a minimal reproduction,
and whether secrets or user content may have been exposed. Avoid attaching real
user data, credentials, tokens, or unredacted filesystem paths.

Receipt of a report is not a promise of a response or remediation deadline.
Coordinated disclosure timing will be agreed after the report is reproduced and
its scope is understood.

## Security boundary

Vellum is an editing engine and local-service composition, not an
authentication provider or sandbox. Host applications must establish caller
identity and authority, constrain local resources, and select deployment
controls appropriate to their threat model. The concrete
[STRIDE/LINDDUN threat model](docs/security-threat-model.md) defines current
trust boundaries, residual risks and profile-specific security release gates.
See [`docs/release-policy.md`](docs/release-policy.md) for distribution limits.
