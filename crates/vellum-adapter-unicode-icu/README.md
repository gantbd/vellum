# Native Unicode provider

An in-process provider for the engine's existing Unicode 18 / UAX #29 revision
49 boundary profile. It neither publishes source nor implicitly selects UI policy.
Both reference apps explicitly opt into its source movement/deletion profile via
the [shared editor](../vellum-model/src/app/unicode.rs); rendering, form input
and widget word/line gestures remain separate profiles.

- ICU4C 79.1 supplies root character boundaries through `rust_icu_ubrk` 5.8.0.
- Default words follow the normative revision-49 rules using pinned ICU4X 2.4
  Word_Break and Extended_Pictographic data. Two monotone iterators and RI parity
  avoid rescanning ignored-character and regional-indicator runs.
- Significant intervals use pinned Alphabetic and numeric-category properties,
  not Rust's compiler-dependent character tables or dictionary word status.

The stable ICU4X 2.4 word constructor passes the canonical word corpus but groups
Thai differently from this engine profile; it is therefore not used here. The
stable ICU4X grapheme constructor also fails ten pinned grapheme cases. No
unstable constructor, dictionary substitution or per-input boundary patch is used.

`Provider::segment` takes authorized exact text and returns a complete owned
table or an explicit refusal. `attest_capture` obtains the original engine
capture and lets the engine revalidate it at acceptance. Failed calls do not
retire that capture, rewrite source, authorize disclosure or issue replacement
identities. The caller owns retry/retirement. Source is not retained between calls.

Caller-selected limits bound source bytes, output endpoints, significant words
and reserved Rust-vector payloads. Logical accounting excludes allocator overhead,
the caller/engine's source copies, binding-owned UTF-16 copies and native ICU allocations; it is not an RSS or
latency guarantee. Native errors and allocation refusals are explicit where the
safe bindings expose them; host allocator aborts and native crashes are not
recoverable Rust errors. No subscriber, external callback or subprocess runs.

Instrumentation separates offers, admissions, completions and refusal categories;
counters saturate with a sticky overflow flag. Optional process-service timing
and bounded diagnostic loss do not change output or engine acceptance. Diagnostics
carry no source, paths, handles or per-document identity.

Build against the explicitly selected ICU4C installation:

```sh
PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig cargo test --release --offline -p vellum-adapter-unicode-icu
PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig VELLUM_ICU4J_JAR=/absolute/path/to/icu4j-79.1.jar make check-unicode-native
PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig cargo run --release --offline -p vellum-adapter-unicode-icu --example service
```

The build refuses ICU versions other than 79.1. The host must maintain the
qualified native libraries/loader; this is not runtime authentication of swapped
dylibs. Native deployment/bundling, UI migration, glyph geometry and product
performance budgets remain separate. Dependencies use Apache-2.0 (Rust bindings)
and Unicode-3.0 (ICU4X/data); ICU4C's accompanying Unicode license remains required
for any native redistribution. No native binaries are vendored in this crate.

The service example characterizes 256 warm samples per workload/capture mode,
including owned-result drop. It excludes engine evidence acceptance, UI/rendering,
cold initialization and native distribution. It enforces no product SLO and does
not establish a causal observer-overhead comparison.
An initial local run measured provider-only p99 around 0.11–0.19 ms for
4–4.7 KiB inputs and 1.55–1.58 ms for 64 KiB ASCII across the three modes.
These are warm measurements on the current qualification host, not UI latency
or a guaranteed budget; rerun the executable when integration or hosts change.
