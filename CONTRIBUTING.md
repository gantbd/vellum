# Contributing to Vellum

Vellum is currently distributed as source and has not made a production
release. Contributions should preserve the boundaries and nonclaims described
in [the release policy](docs/release-policy.md) and
[the project-boundary guide](docs/project-boundaries.md).

## Development setup

Vellum currently targets macOS and uses the Rust toolchain pinned by
`rust-toolchain.toml`. The workspace also depends on the exact Keystone Git
revision recorded in `release/github-source.json` and `Cargo.lock`.

Fetch the locked dependency graph, then run the complete local gate:

```sh
cargo fetch --locked
make check
```

The gate checks release and supply-chain policy, architectural and effect
boundaries, compatibility fixtures, formatting, Clippy, all workspace tests,
and documentation. Some adversarial tests create Unix-domain sockets and
provider subprocesses, so they must run in an environment that permits those
operations.

## Change discipline

- Keep dependency direction aligned with `docs/project-boundaries.md`.
- Keep crates non-publishable unless the release policy is deliberately
  changed in the same reviewed contribution.
- Add focused tests for behavior changes and preserve content-safe diagnostics.
- Treat formal models, compatibility fixtures, release manifests, and retained
  evidence as source artifacts rather than disposable generated output.
- When an intentional source or wire change trips the compatibility gate,
  review `node scripts/check-compatibility-baseline.mjs --emit=-` and update
  `release/compatibility-baseline.json` only for the reviewed delta.
- Do not commit credentials, private user data, unredacted local paths, or
  machine-specific build output.

Before submitting a change, ensure `make check` passes from the exact proposed
tree and summarize any release-policy, wire, security, or compatibility impact.

## Security reports

Do not report suspected vulnerabilities in a public issue or pull request.
Follow [SECURITY.md](SECURITY.md) instead.
