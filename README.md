# Vellum adapters

Qualified interpretations of Vellum host ports: local filesystem, annotation
JSON, ICU Unicode boundaries, and macOS clipboard. These crates provide no
semantic source/history authority and select no application policy.

`vellum-adapter-local-fs` also provides bounded, sorted workspace discovery for
application shells. It expands explicit file/directory inputs without opening
resources, follows no symbolic links, and returns either the complete file set
or a content-free refusal. Frontends choose the limits and retain workspace
lifecycle policy.

The patched AppKit surface is confined to the unpublished live-qualification
runner. With explicit operator approval, run it on macOS with:

```sh
cargo run --locked --manifest-path qualification/vellum-clipboard-macos-qualification/Cargo.toml -- --allow-private-pasteboard
```

It creates only unique private pasteboards; production adapter packages resolve
the released `objc2-app-kit` crate and require no consumer patch.
