# Portable Unicode provider

An in-process provider for the engine's existing Unicode 18 / UAX #29 revision
49 boundary profile. It neither publishes source nor implicitly selects UI policy.
Both reference apps explicitly opt into its source movement/deletion profile via
the [shared editor](../vellum-model/src/app/unicode.rs); rendering, form input
and widget word/line gestures remain separate profiles.

- ICU4X 2.4 supplies the stable Unicode 18 grapheme implementation. The adapter
  composes the revision-49 GB9c delta (`Linker Extend* × Consonant`) over its
  candidate boundaries; the pinned production gate passes all 853 official cases.
- Default words follow the normative revision-49 rules using pinned ICU4X 2.4
  Word_Break and Extended_Pictographic data. Two monotone iterators and RI parity
  avoid rescanning ignored-character and regional-indicator runs.
- Significant intervals use pinned Alphabetic and numeric-category properties,
  not Rust's compiler-dependent character tables or dictionary word status.

The stable ICU4X 2.4 word constructor passes the canonical word corpus but groups
Thai differently from this engine profile; it is therefore not used here. Its
stable grapheme constructor predates revision 49 and misses exactly ten GB9c
cases. The adapter implements the published rule composition, not an input list
or corpus-specific exception.

`Provider::segment` takes authorized exact text and returns a complete owned
table or an explicit refusal. `attest_capture` obtains the original engine
capture and lets the engine revalidate it at acceptance. Failed calls do not
retire that capture, rewrite source, authorize disclosure or issue replacement
identities. The caller owns retry/retirement. Source is not retained between calls.

Caller-selected limits bound source bytes, output endpoints, significant words
and reserved Rust-vector payloads. Logical accounting excludes allocator overhead
and the caller/engine's source copies; it is not an RSS or latency guarantee.
Allocation refusals are explicit where safe Rust exposes them; allocator aborts
are not recoverable Rust errors. No subscriber, external callback, native loader,
or subprocess runs.

Instrumentation separates offers, admissions, completions and refusal categories;
counters saturate with a sticky overflow flag. Optional process-service timing
and bounded diagnostic loss do not change output or engine acceptance. Diagnostics
carry no source, paths, handles or per-document identity.

Run the focused tests and full pinned production corpus gate without native
libraries or environment selection:

```sh
cargo test --release --offline --locked -p vellum-adapter-unicode-icu
cargo run --release --offline --locked -p vellum-unicode-reference --features native-conformance --example portable_qualification
cargo run --release --offline --locked -p vellum-adapter-unicode-icu --example service
```

UI migration, glyph geometry and product performance budgets remain separate.
The Unicode data dependency is licensed under Unicode-3.0; no native binaries or
dynamic libraries are required or vendored.

The service example characterizes 256 warm samples per workload/capture mode,
including owned-result drop. It excludes engine evidence acceptance, UI/rendering,
cold initialization and native distribution. It enforces no product SLO and does
not establish a causal observer-overhead comparison.
Earlier native-provider measurements are not attributed to this implementation;
rerun the executable before adopting any product SLO.
