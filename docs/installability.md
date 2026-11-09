# Cargo installation contract

Status: accepted release constraint; migration blockers are bounded by
[`release/installability.json`](../release/installability.json).

The eventual user path is:

```sh
cargo install vellum --locked
```

That installs the default `vellum` terminal application. The qualified desktop
application is opt-in so terminal users do not compile a GUI stack:

```sh
cargo install vellum --locked --features desktop
```

The desktop binary is `vellum-desktop`. The initial qualified platform profile
is macOS; unsupported platforms must fail clearly rather than compile a binary
that cannot provide its declared storage or Unicode behavior.

The workspace now contains that exact distribution package, default binary,
`desktop` feature and feature-gated desktop binary. Registry publication and
portable native prerequisites remain separate gates below.

## Packaging invariants

- Registry packages use exact Vellum dependency versions. Local `path` entries
  may coexist only for workspace development.
- The distribution package owns binaries and composition only. Engine, SDK,
  model and adapter crates remain libraries.
- A published package cannot depend on a consumer-side workspace patch. Patched
  Iced/AppKit behavior must be upstreamed, carried by separately published fork
  packages, or removed before release.
- `cargo install` must not require setting `PKG_CONFIG_PATH`. The selected
  Unicode provider must be discoverable or bundled under an explicit supported
  profile, with its license and platform behavior qualified.
- `--locked` is part of the supported command. Release qualification tests both
  a clean registry-style package and the installed binaries.
- Install-time code must not contact a network except through Cargo's normal
  registry/source retrieval. Runtime network services remain outside Vellum.
- Cargo metadata includes an exact MSRV, SPDX license expression, repository and
  content-free package descriptions.

## Release qualification

Before removing the final blocker, qualification must prove:

1. `cargo package` succeeds for every library in dependency order.
2. The packaged `.crate` files, unpacked outside the workspace, build offline
   against only their declared registry dependencies.
3. `cargo install vellum --locked` succeeds in a clean Cargo home without
   sibling repositories or workspace patches.
4. The installed TUI passes a bounded real-file create/edit/save/reopen probe.
5. The desktop feature builds from the packaged source and passes its existing
   opt-in synthetic-window qualification.
6. Installed `--help` output identifies the platform profile and unsupported
   capabilities without overstating filesystem, clipboard, Unicode or desktop
   assurance.

`make check-installability` is monotone: removing an admitted blocker is
progress; adding or renaming one fails the check.

## Current baseline

On 2026-11-09, the canonical distribution package passed both clean local
offline install shapes and both installed help paths:

```sh
PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig \
  cargo install --path crates/vellum-tui --locked --offline \
  --root /tmp/vellum-install
/tmp/vellum-install/bin/vellum --help

PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig \
  cargo install --path crates/vellum-tui --locked --offline \
  --features desktop --root /tmp/vellum-desktop-install
/tmp/vellum-desktop-install/bin/vellum-desktop --help
```

The second command installed both `vellum` and `vellum-desktop`. This proves
the final package, feature and executable topology locally. It does not yet
prove registry installation or portable native dependency discovery.

Earlier on 2026-11-09, the component package passed an offline release install
and executed its installed help path:

```sh
PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig \
  cargo install --path crates/vellum-tui --locked --offline \
  --root /tmp/vellum-cargo-install-smoke
/tmp/vellum-cargo-install-smoke/bin/vellum-tui --help
```

This proves the current package entry point, release build and installed Mach-O
binary. It does **not** close the registry contract: the command still uses the
monorepo, workspace patches and explicit ICU discovery. The installed binary
links Homebrew ICU4C 79 dylibs, so `native-icu4c-discovery` remains a real
installation blocker rather than a documentation issue.
