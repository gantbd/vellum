# Rust engine testing guide

Executable Rust behavior tests are the primary implementation acceptance surface.
Use a failing scenario, the smallest implementation change and focused regression
checks. JS/formal reference checks are distinct evidence, not Rust coverage.
Do not add another status document for each slice.

## Development checks

Run the affected test target first, widening coverage when shared owners,
authorization, publication or lifetime behavior changes:

```sh
cargo test --offline -p vellum-engine --test headless_delivery_behaviors
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
```

Tests must exercise public behavior through actual owners, not merely descriptors
or helpers. Cover original intent/replay, independent result authorization,
stale/foreign identities, bounded resource exhaustion, fault/refusal behavior,
atomic publication and protected cleanup wherever affected. Ignored/empty tests
do not count as implemented acceptance. Keep UI gestures and substrate mechanisms
in their own integration tests.

`cargo test --release --offline -p vellum-session --example file_edit` exercises
the runnable local-file workflow itself, including synchronized-save checks,
lossless reopen and explicit tracked-document reclamation. A refused edit must
leave disk unchanged; failure is not reported as rollback or completed cleanup.

## Instrumentation and performance

Test capture disabled, enabled, timed and saturated against the same behavior.
Diagnostics must not change authorization, execution, results or cleanup. Preserve
explicit loss/overflow and unknown-clock states; unknown is not zero.

Apply these perspectives to actual owning boundaries, not every function:

| Perspective | Useful acceptance questions |
| --- | --- |
| MELT / golden signals / RED | Are request rates, outcomes, errors, duration and pressure observable? Distinguish offers, admissions, retries/replays and completion; avoid duplicate populations. |
| USE | Are bounded ownership, reservation/retention utilization, saturation, exhaustion and recovery tested? Logical reservations are not physical RSS or CPU utilization. |
| STRIDE | Do authority, identity, evidence and interface tests challenge substitution, tampering, replay, disclosure, denial of service and privilege escalation? Include diagnostics. |
| LINDDUN | Can data or telemetry create unwanted correlation, identification or disclosure? Test minimization, current recipient permissions and retention/release. Tests are not legal certification. |

Keep synchronous service, engine-owned pending/queue age and first attributed
result timing separate. Protocol start, unknown-write progress and quiescence are
not automatically meaningful results. Substrate I/O, scheduling and network
measurements belong at their actual boundaries. Do not collect document content
or identity labels merely to observe the engine.

Use the existing [release workloads](performance.md), build once and repeat the
binary when appropriate. Report distributions and sample populations, diagnostic
loss and measurement exclusions. Enforce only explicitly adopted budgets; no
default product SLO or causal observer-overhead claim is implied.

## Integrated qualification

At integration/release checkpoints:

```sh
make check
cargo test --workspace --all-targets --release --offline --quiet --no-fail-fast
cargo run --offline -p vellum-engine --example headless_edit
```

The separate Unicode reference gate requires Java and the pinned ICU4J 79.1 jar:

```sh
VELLUM_ICU4J_JAR=/absolute/path/to/icu4j-79.1.jar make check-unicode
```

`VELLUM_JAVA` may name the Java executable. Missing prerequisites, incorrect pins,
protocol errors and corpus mismatches fail this gate rather than silently skip.
Reference-provider execution is outside engine commit and is not a production
provider choice. Separately run the release-only ignored performance experiment
as described in [performance.md](performance.md).

The isolated production-candidate probes are separate from that engine reference
gate and do not migrate either UI:

```sh
make check-unicode-icu4x
PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig make check-unicode-icu4c
```

Both use the same SHA-256-pinned public Unicode 18 corpus (853 grapheme and
1,944 word cases), report public corpus line numbers rather than text, and return
failure on mismatches. The stable ICU4X 2.4 grapheme constructor fails 10 cases;
its stable no-complex-script word constructor passes all word cases. The actual
ICU4C 79.1 root character iterator passes all grapheme cases through
`rust_icu_ubrk` 5.8.0, with checked UTF-16-to-UTF-8 coordinate conversion. The
native gate requires pkgconf/pkg-config, libclang and the selected ICU headers
and libraries; inspect actual linked libraries as well as configuration version.
Those remain isolated comparison probes, not runtime dependencies of the engine
or applications. The production `vellum-adapter-unicode-icu` composes Unicode 18
ICU4X data with revision-49 GB9c and direct default-word rules. Its portable gate
passes all 853 grapheme and 1,944 word corpus cases:

```sh
cargo run --offline --locked -p vellum-unicode-reference --features native-conformance --example portable_qualification
```

ICU4X's word constructor groups Thai differently despite passing the canonical
word corpus and is not selected. Whole-workspace and UI builds require no system
ICU, headers, loader, or `PKG_CONFIG_PATH`.

`VELLUM_ICU4J_JAR=/absolute/path/to/icu4j-79.1.jar make check-unicode-native`
compares all 2,805 complete native tables with the pinned oracle, including
significant intervals, then runs native tables through real engine capture,
movement, replay, deletion and undo. An additional 1,602 contextual cases challenge
ignored characters and complex scripts. Stale/foreign captures, failure ownership,
input/output/logical bounds, full 64 KiB source, optional timing and saturated
capture have focused tests. This qualifies that adapter/profile, not a UI migration,
rendered geometry, native distribution or a product performance guarantee.

The shared editor's optional `native-unicode` feature adds explicit
`enable_native_unicode` configuration; both reference applications enable it. Focused
native editing tests cover Unicode-18-only clusters, capture modes, version-cache
reuse, multi-selection history, readonly policy, provider-refusal ownership and
cleanup. A presentation failure after source publication preserves the actual
authored state and discard requirements; its status must not claim nonapplication.
The `interaction` example accepts `--native-unicode` only when built with that
feature and checks cache regeneration and retirement as well as source/frames.
Neither the session facade nor the engine selects or invokes a provider implicitly.
The iced bridge tests also cover explicit source-gesture projection versus exact
SDK/literal targets, stale attachment ABA, incremental native cluster edits,
readonly edge no-ops, refused previews and actual widget named-key events.
Its `iced-interaction --unicode-profile native` workload distinguishes preview
and evidence service populations, checks bounded capture loss and verifies cleanup;
word/line gestures, shaping, form profiles and OS IME remain separately scoped.

## Fixed remaining v1 acceptance gates

All seven gates closed on 2026-11-07 under the approved provisional native scope.
The historical heading is retained for existing links; this is not an open queue.

| Gate | Accepted scope |
| --- | --- |
| C1 Behavior facade | Native discovery/request/owner/result routes and explicit disabled conditional capabilities |
| C2 Retained results | Original invocation/allocation binding, independent current disclosure, replay and once-only cleanup |
| C3 Lifetimes | Document reclamation, retained-target mapping/pins and protected discharge; not physical erasure |
| C4 Observations | Supported heterogeneous events, attribution, revocation/resynchronization, loss and retirement |
| C5 Shutdown | Admission fence, actual obligation inventory, disposition, settlement and final control retirement |
| C6 Instrumentation | Actual-path populations, capture-mode equivalence and scoped release workloads |
| C7 Qualification | Integrated workspace checks, reference gate and runnable handoff |

## Current implementation evidence by gate

The [native crosswalk](public-contract-matrix.md#current-rust-closure-crosswalk--2026-11-07)
identifies supported profiles and owners. Final qualification recorded:

- Debug and release: each 88 targets, 2,880 passed; one ordinary-suite ignored
  release-only experiment separately qualified.
- Formatting and strict all-feature workspace lint: passed.
- Doc-test command: passed across four crates, with zero executable snippets;
  not additional behavior coverage.
- Pinned Unicode: two tests passed, 2,805 native tables and movement/replay/deletion/undo paths.
- Release workloads: five examples, 81 cohorts / 41,472 measured operations;
  separate receipt-control experiment, 48 cohorts / 6,144 first closures.
- Headless handoff: edit/replay/authorized result and all session owners released,
  zero diagnostic drops.

These are recorded results, not an assertion that checks were rerun for subsequent
documentation edits. Product latency SLO, MSRV and production Unicode-provider
selection remain unselected; no physical-persistence/RSS or renewed independent
formal certification is claimed.

## Current engine closure reconciliation

No mandatory engine deliverable remains in the completed provisional v1 goal.
UI/clipboard/automation integration and storage mechanisms remain separate work.
Use current code and tests when planning changes; intermediate slice caveats in
the [archived acceptance plan](engine-test-plan.history.md) are historical, not
instructions to reopen completed gates. The archive preserves detailed original
acceptance criteria and run history without requiring continued maintenance.
