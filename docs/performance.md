# Engine responsiveness gate

## Current release refresh — enabled publication commands

One offline release build of the five examples, followed by sequential direct
binary runs (`--samples 512 --warmup 32`), passes **81 cohorts and 41,472 measured
operations**. The explicitly invoked receipt-control release test separately
passes **48 cohorts and 6,144 first closures**. Current CSVs:

- [Source/edit/query](performance/qualified-responsiveness-2026-11-07.csv)
- [Invocation](performance/qualified-invocation_responsiveness-2026-11-07.csv)
- [Raw/enabled-command observation](performance/qualified-observation_responsiveness-2026-11-07.csv)
- [Search](performance/qualified-search_responsiveness-2026-11-07.csv)
- [Shutdown](performance/qualified-shutdown_responsiveness-2026-11-07.csv)
- [Receipt control](performance/qualified-receipt-control-2026-11-07.csv)

The observation example now has separate raw and enabled-command profiles. The
latter measures original resynchronization-command admission, native execution,
privileged retained-result inspection/full payload oracle, identity-only replay
with original-result pointer equality, and command retirement. Per-boundary
counts distinguish one issuance, two requests, two inspections and one retirement
per measured iteration. Pool teardown refunds the actual parent. Raw rows do not
export fictitious zero-duration command samples. Five observation example/harness
tests, focused strict all-feature Clippy and workspace formatting pass.

Observed total p99 ranges after the first-result instrumentation change:
source/edit/query 1.750–53.375 µs; invocation 2.334–5.417 µs; raw/enabled source
observation 2.625–707.291 µs (including payload oracles up to 1 MiB); search
14.250 µs–1.678 ms; shutdown 3.916–67.167 µs. This final-artifact refresh retains
the declared sample populations; observed variation is not assigned a cause.
These are declared-workload results, not universal latency acceptance, an adopted
numeric SLO, physical-memory qualification or isolated causal overhead estimates.
Named-facet/collection/registry behavior remains covered by native functional
tests, not falsely represented as measured source-command performance. The
receipt workload explicitly measures source/facet/collection first control
closures; it is not provider execution, scheduler or network timing.

Binary SHA-256 for this refresh:

| Artifact | SHA-256 |
| --- | --- |
| responsiveness | `529d168519ef7125d09b3b7667f06af6e7934013f2b48797699030d4721c29fc` |
| invocation_responsiveness | `fd770e16f5596d87796214d717d3d7cd00612f7ecad3e29659bc20932437746a` |
| observation_responsiveness | `70b7c24e841fc7892db6ab26189f38d021596bb8f09e694e3a970d005e716d3c` |
| search_responsiveness | `e679a788d6c214da8cfdf3e97a6ae41feba478e2ba1531bcb222bec539f34a62` |
| shutdown_responsiveness | `036edbd38271c1d75979e591068aa465859998c543bd1816021b1c74e3e1e2ec` |
| receipt-control release test binary | `0bc7829958c25270e3d6a4bac3648350c60fd42f7f5d13e37802f71d22e2b54f` |

C6 first-result instrumentation is implemented at the original source-read,
annotation-read and shared save owners. The existing opt-in profile now preserves
dispatch -> first accepted attributed payload/terminal-result claim intake,
explicit observed-with-unknown-clock state, active censored ages and retired-
without-result populations. Reads' protocol Start, saves' unknown-application byte
progress, causal edges, no-future effects and quiescence do not count as results.
The first observation is absorbing, including after duplicate/late evidence and
an unavailable clock. Metrics remain privileged, fixed-size, independent of
Recorder loss and separate from effect truth, provider CPU/wire time and result
disclosure. Six timing unit tests, eight public queue/result scenarios and 119
source/annotation/save/provider regressions pass; strict affected-target Clippy
and formatting pass. The CSVs/hashes above were refreshed after this change.
C6 is closed under its declared workload/measurement scope; final C7 workspace
qualification also passes, as recorded in the delivery checklist. No new product
latency threshold or physical-RSS acceptance is claimed.

## Initial native shutdown workload (earlier binary)

`cargo run --offline --release -p vellum-engine --example shutdown_responsiveness -- --samples 512 --warmup 32`
completed **24 cohorts and 12,288 measured shutdowns** over 1, 8 and 64 alternating
code/Unicode-prose documents, native/typed-decoded disposition, and capture off,
enabled, timed and saturated. The [retained run](performance/delivery-shutdown_responsiveness-2026-11-07.csv)
contains all eight phase distributions and exact populations. Total p99 ranges
were 4.084–6.042 µs (one document), 6.666–12.583 µs (eight), and 36.250–69.209 µs
(64). These are observed workloads, not a selected product latency SLO or causal
instrumentation-overhead claim.

This measures actual original-consent disposition, provenance release and payload
reclamation, accepted-plan retirement, returned report creation/disclosure/
retirement and final control retirement. Every iteration requires full native
coverage, no unexplained funding, zero final parent reservation and an absorbing
closed engine. Setup/consent, diagnostic drain and final Rust Drop are excluded;
reclamation timing includes provenance release. No worker, substrate, network,
queue wait or crash-persistence result is inferred. `max_checkpoint_reserved_bytes`
is the maximum observed parent reservation at the declared checkpoints, not an
allocator/RSS bound or continuously sampled peak.

All external samples survive capture pressure. Native service populations are
checked per boundary before sample acceptance; only the intentionally saturated
cohort may lose diagnostic records. Four example/harness tests, strict
all-feature example Clippy and workspace formatting pass. Counts/durations/loss
cover applicable MELT/golden/RED and logical USE perspectives; unexpected errors
fail the run instead of becoming fast samples. No external scheduler utilization
or I/O saturation is claimed. STRIDE/LINDDUN boundaries retain separately supplied
native authority and fixed optional records without content/identity labels;
this privileged local performance output exposes declared workload size/timing,
not recipient-authorized document data. Behavioral security/privacy refusals are
qualified by the engine tests, not by timing success.

Measured release binary SHA-256:
`4a1f1ec54f6dec2897397a8afc915ed6df1818ff21e2c876fd1472f9ebe55925`.
Example source SHA-256:
`8ef89591175d4256a37e825500f6f6f74d1c5c5c42942368ffe52a39b00877e2`.
Earlier CSVs retain their original binary scope; this does not refresh them or
qualify other publication-command/provider paths. C6/C7 remain open.

Performance is an implementation acceptance concern, not a consequence of finite
work bounds or green correctness tests. This first gate exercises the actual
trusted local source owner. It does not qualify the entire recipient facade,
an editor UI, a storage provider or a scheduler.

## Delivery release workload checkpoint — 2026-11-07

One offline release build of the four existing examples, followed by sequential
512-sample/32-warmup runs, completed all 45 cohorts and 23,040 measured operations.
The existing receipt-control workload also passed all 48 cohorts/6,144 measured
first closures in its release test binary. Every declared correctness, sample
population, capture-mode and cleanup oracle passed. No product SLO was supplied.

| Warm-process total | Current p99 across cohorts/modes |
| --- | ---: |
| 4 KiB source edit/query | 1.000–1.250 µs |
| Two 64 KiB documents, 16 edits each | 17.833–18.167 µs |
| 1 MiB source edit/query | 48.125–48.959 µs |
| Invocation plus result/observer query | 2.250–3.333 µs |
| Unicode prose search, 64 matches | 9.625–10.083 µs |
| Captured-scope code search, 1,536 matches | 115.834–120.333 µs |
| Mixed search, 16,384 matches | 1.601–1.634 ms |
| Source observation, 4 KiB / 64 KiB / 1 MiB | 2.208–2.834 / 29.417–29.833 / 328.167–349.708 µs |
| Receipt stop/revoke, 48 cohorts | 0.333–1.125 µs; maximum 7.458 µs |

Exact binary hashes, commands, all distributions, external example process times
and capture losses are retained in [source](performance/delivery-responsiveness-2026-11-07.csv),
[invocation](performance/delivery-invocation_responsiveness-2026-11-07.csv),
[search](performance/delivery-search_responsiveness-2026-11-07.csv),
[observation](performance/delivery-observation_responsiveness-2026-11-07.csv) and
[receipt control](performance/delivery-receipt-control-2026-11-07.csv).
The receipt test is explicitly invoked, not ignored correctness acceptance.

These uncontrolled sequential runs qualify the declared workloads' semantics and
populations against their recorded binaries, not a regression baseline, causal
capture overhead, physical allocation/RSS or full-interface latency. In particular,
the larger current multi-edit number is visible; the older non-isolated run is
not sufficient to attribute that difference to a code regression or a host cause.
Observation totals include the payload oracle; receipt samples exclude setup,
envelope/oracle/drain/drop. UI, substrate, agent and external waiting times are
not engine service time. New discovery/native-task interfaces are functionally
instrumented, but these examples do not measure their latency. C6's remaining
interface/population review and resource queue-age instrumentation stay open;
this checkpoint does not claim C6/C7 or latency-budget acceptance.

## Previous release workload checkpoint — 2026-11-06

Receipt-control workload: the opt-in native library test
`receipt_control_release_workload` measures first-time source, named-facet and
collection receipt closure through decoded stop/revocation, with typed/dynamic
input and off/capture/timed/saturated diagnostics. Each of 48 cohorts has 16 warmups
and 128 measured operations against fresh receipt state. Every sample verifies
one closed receipt, a still-available sibling and unchanged retained funding;
enabled capture must include exactly one successful facade closure and timed
capture must supply service timing. External timing keeps all 6,144 samples even
when diagnostics drop. Setup, envelope construction, oracle, draining and owner
drop are excluded. Retained results are not falsely reported as discharged.

Run `cargo test -p vellum-engine --offline --release --lib
receipt_control_release_workload -- --ignored --nocapture` as one command.
This explicitly invoked workload is not an ignored correctness acceptance test.
It measures resident direct control only: installed discovery, shutdown drain,
physical memory, sustained admission pressure and causal capture overhead remain
outside this experiment. It enforces semantics/populations, not an invented SLO.
The retained [receipt-control run](performance/receipt-control-2026-11-06.csv)
records the binary hash and all 48 cohorts: p99 0.292–1.125 µs, maximum 9.458 µs
over 6,144 measured first closures. The larger maximum remains visible rather
than being hidden by percentiles. This single sequential run is not a causal
overhead comparison or a full-engine performance qualification.

The additional `observation_responsiveness` example exercises the actual source
publication owner through the heterogeneous event-delivery facade. It measures
sample, projection-event release, empty release and unchanged sample separately
for 4 KiB, 64 KiB and 1 MiB documents, with off/capture/timed/saturated diagnostics.
Every measured iteration checks original generation continuity, complete payload,
no authored publication from observation, stable parent funding and empty queue.
Each cohort must stop/retire its stream and discharge its actual parent reservation.
The executable also requires the exact captured event population in enabled modes,
timing presence in timed mode and honest losses under saturation. External timing
retains all samples regardless of diagnostic pressure.

The retained observation CSV was refreshed after the source original-command
ordinary-admission/delivery gate was added. All 12 cohorts again complete with
their exact external populations, payload/generation oracle and parent cleanup.
The command table is unconfigured in this existing workload; it measures the raw
publisher path through that new gate, not enabled-command admission, native-call
settlement, protected replay, command retention pressure or their overhead. Those
new interfaces still require their own C6 release workload qualification.

Run `cargo run --offline --release -p vellum-engine --example
observation_responsiveness -- --samples 512 --warmup 32` (as one command).
The retained [observation run](performance/observation-events-2026-11-06.csv)
contains all 12 cohorts and 6,144 measured iterations. Total includes the full
payload oracle; individual event phases exclude that oracle. Authored edits,
setup, diagnostic drain and cleanup are outside per-iteration timing. This is
source-observation coverage, not named-facet/collection/registry performance,
shutdown service, queue waiting, physical allocation/RSS or a causal overhead
comparison. An optional `--max-p99-ns` enforces only an explicitly supplied total
budget; no product threshold is invented. Four focused example tests pass,
including the real capture-pressure/cleanup scenario and three shared harness
controls. Strict all-feature example Clippy and formatting pass.

One offline release build of all three existing examples, followed by sequential
runs with 512 measured operations and 32 warmups per cohort, completed all 33
cohorts (16,896 measured operations). Every phase retained its complete external
sample count, including saturated diagnostic capture. The harness correctness,
population and applicable final cleanup checks succeeded. This is not latency-SLO
acceptance: no product budget was supplied or invented.

| Actual workload | Total p99 range across observer/capture modes |
| --- | ---: |
| 4 KiB source edit/query | 0.917–1.042 µs |
| Two 64 KiB documents, 16 edits each | 12.333–14.292 µs |
| 1 MiB source edit/query | 47.709–70.375 µs |
| Invocation plus result/observer query | 2.000–3.792 µs |
| 3,520-byte Unicode prose search, 64 matches | 9.750–10.417 µs |
| 70,656-byte captured-scope code search, 1,536 matches | 116.167–126.458 µs |
| 1,114,112-byte mixed search, 16,384 matches | 1.566–1.647 ms |

Raw source, invocation and search distributions are in
[source](performance/responsiveness-current-2026-11-06.csv),
[invocation](performance/invocation-current-2026-11-06.csv) and
[search](performance/native-search-current-2026-11-06.csv). Each artifact includes
the measured binary SHA-256, compiler/target, command, diagnostic drops and external
process times. Source/invocation/search spent approximately 0.119/0.014/3.435 s in
main; `/usr/bin/time` reported 0.32/0.19/3.61 s wall time and 0.11/0.01/3.43 s user
CPU time. These clocks have different endpoints; the difference is not engine
service time or evidence of a particular OS/host cause.

This uncontrolled single sequential run is not a causal overhead comparison,
regression baseline or full-engine performance qualification. It excludes the
new observation/discovery interfaces, storage, UI, agent reasoning and external
waiting. Allocations/RSS and host load remain unmeasured; optional hardware-detail
inspection was refused by the environment. The artifacts qualify these existing
workloads against this binary only. C6 still requires remaining actual-path/
population coverage and any explicitly selected product budget; C7 stays separate.

## Run without rebuilding unnecessarily

```sh
make perf-build
make perf-run
make perf-run PERF_ARGS='--samples 1000 --warmup 64'
make check-perf PERF_MAX_P99_NS=<explicit-positive-nanosecond-budget>
```

Select `PERF_EXAMPLE=invocation_responsiveness` on the same targets for the
invocation/cohort experiment described below; the default stays `responsiveness`.
Select `PERF_EXAMPLE=search_responsiveness` for native retained literal-search
evaluation/release/navigation/cleanup across synthetic Unicode prose/code/mixed
content. Build once, then reuse `perf-run` or the existing executable.

Replace the placeholder before running the last command. `perf` builds then runs;
`perf-run` runs the existing release executable only. `check-perf` refuses a missing
budget before building. `PERF_BIN` can select the binary in a custom Cargo target
directory. The harness refuses debug execution, malformed/duplicate arguments,
fewer than 100 or more than 100,000 samples, and warmup above 100,000 iterations.
The defaults (512 measured samples, 32 warmup operations) are experiment settings,
not deployment limits or an adopted latency SLO. No new runtime dependency is used.

For launch/build attribution, run these separately:

```sh
cargo build -p vellum-engine --example responsiveness --release --offline --timings
/usr/bin/time -p target/release/examples/responsiveness
```

Cargo's timing report, outer process wall/user/system time, the executable's
`entered-main` marker and `in_process_elapsed_ns`, and individual measured phases
are distinct observations. A large outer/main gap is outside the measured Rust
main episode; it does not by itself identify security scanning, scheduling,
tool orchestration, loader or host failure. Do not call it engine service latency
or infer a specific OS cause without further evidence.

## Fixed workload and observation endpoints

| Cohort | Decoded bytes per document | Documents | Original-coordinate edits per document |
| --- | ---: | ---: | ---: |
| resident-small | 4,096 | 1 | 1 |
| agentic-multi | 65,536 | 2 | 16 |
| resident-large | 1,048,576 | 1 | 1 |

All content is synthetic ASCII valid UTF-8. Each iteration alternates replacement
values, prepares the complete transaction, commits once, then queries every
participant and verifies the new revision, length and every edited byte. Every
operation must succeed with all requested participants changed; a refusal or
incomplete cohort fails the run rather than disappearing from the denominator.
This is not yet a benchmark of refusal or overload latency.

For every cohort the harness runs capture disabled, capture without clocks,
capture with native timing, and an intentionally full diagnostic queue. It
preallocates four bounded sample vectors. External monotonic samples include every
measured operation even when native diagnostic records are dropped. Warmup never
enters the measured sample counts. No document content or identity is output.

- Total latency begins with an existing owned request and ends after the committed
  source queries/checks. It excludes request construction, initial document creation,
  queues, I/O, rendering and agent reasoning.
- Preparation, synchronous commit service and query/check durations are reported
  separately. Timer reads/checks are included; overhead is not subtracted.
- Throughput can be derived from the measured cohort count and elapsed time, which
  also includes bounded diagnostic draining and per-iteration bookkeeping.
- CSV reports exact sample counts, nearest-rank p50/p95/p99, maximum, retained timed
  record count and diagnostic drops. A missing native sample is not zero latency.
- The supplied budget applies to total p99 for **every** cohort and capture mode.
  All cohorts are reported before a budget failure exits nonzero. It is an absolute
  gate, not yet an automated comparison to a stored baseline.

The native source owner's preparation, commit and query now populate existing
service-time fields when opted in. The existing `Admission` marker describes
synchronous commit validation/publication service, **not queue waiting**. Publication
markers do not reuse that duration as if it were a separate publication-only span.
Default-off and full-queue clock suppression retain their existing semantics.

## Retained-search service coverage

```sh
make perf-build PERF_EXAMPLE=search_responsiveness
make perf-run PERF_EXAMPLE=search_responsiveness PERF_ARGS='--samples 100 --warmup 4'
make check-perf PERF_EXAMPLE=search_responsiveness PERF_MAX_P99_NS=<explicit-positive-budget>
```

The search experiment runs three fixed workloads: 64 repeats of Unicode prose,
1,536 repeats of a Unicode code/comment unit with two adjacent independently
captured scopes (supplied in reverse order), and 16,384 repeats of mixed Markdown/
code/prose. Each unit has one exact literal UTF-8 match. Actual byte/match counts
are reported, with no payload or identity labels. A standard-library matching
oracle is prepared outside the measured cohort. All four diagnostic modes run.

Each iteration evaluates through the existing native retained-context owner,
releases and checks the complete original-version result, navigates and checks
the first occurrence, drops copied replies, then protectedly retires the result.
Total and separate evaluation, release/check, navigation/check and retirement/
check distributions include their clock/check overhead. Total includes intervening
reply drops. There is no request construction, initial context acquisition, input
normalization, `/4` facade, UI, queue, provider or final pool cleanup in total.
Cohort throughput time includes sample bookkeeping and diagnostic drain. Source
holds must return to zero each iteration; final search/context/pool cleanup must
leave zero parent reservation. Refusal, partial results, incorrect results or an
incomplete population fail the experiment, not disappear from its denominator.

Warmup is excluded from percentiles but consumes real nonrefunding issuance.
Lifetime pressure grows through warmup plus measured work; no hidden registry
reset flattens its cost. The experiment caps that sum at 4,096 before setup and
rejects empty populations. Explicit logical fixture ceilings (including 256 MiB
parent headroom, 64 MiB per pool and 4 MiB search capture/reply bounds) are benchmark
settings, not new product defaults, measured RSS or deployment recommendations.
External samples survive full diagnostic capture. The optional positive total-p99
budget applies to every workload/mode; all cohorts report before budget failure.
The shared parser/distribution/budget tests plus search-specific population,
UTF-8/oracle/cleanup/capture tests qualify the harness, not an adopted SLO.

Five focused example tests, formatting and strict offline all-feature Clippy for
the changed example pass. The optimized local smoke (`100` measured, `4` warmup)
completed all twelve cohorts and all final cleanup checks. Across modes, total
p99 was approximately 8–20 µs for 3,520-byte prose, 120–144 µs for 70,656-byte
captured-scope code and 1.70–1.75 ms for 1,114,112-byte mixed content. Timed modes
retained 400 service records per measured cohort; saturated modes dropped 900
markers while retaining all 100 external samples per phase. Raw output is in
[`native-search-smoke-2026-11-05.csv`](performance/native-search-smoke-2026-11-05.csv).
A separate same-binary run with 100 samples, no warmup and an intentionally
impossible 1 ns budget reported every cohort then exited 1 as required; this is
a gate negative control, not a selected engine budget. The smoke took about
0.72 s inside main, including setup/reporting. It is uncontrolled single-run
evidence, not a regression baseline, causal observer-overhead result, physical
memory/CPU qualification or whole-engine acceptance. Counter/gauge checks are
included in the measured total and may scan lifetime rows; they are not free
exclusive engine service. Build/launch/host interference remain separate.

Subsequent native implementation removes lifetime-row scans from managed-search
result-capacity admission and live/invalidated usage gauges. Private checked
gauges update with row installation, first absorbing query loss and protected row
removal; spent issuance and tombstones remain unchanged. Invalidated retains its
old meaning of retained query-stopped rows, not every currently ineligible result
or a permission oracle. Source-context usage and
other paths can still scan their inventories. Cold pool retirement deliberately
reconciles against actual rows before discharging funding; impossible bounds or
underflow fault without erasing the original result/hold. The larger private header
is included by `required_pool_bytes`.

The local invariant is live = retained rows and invalidated = retained stopped
rows: initialization establishes both; installation adds one live nonstopped row;
first loss adds one stopped contribution; retirement subtracts the addressed row
and its contribution. Refusal installs none. Exclusive ownership serializes these
closed mutation sites. Tests compare a direct row oracle across all three issuers,
temporary refusals, repeated loss, peer release and protected retirement; injected
contradictions test conservative fencing and debt retention. This is source-level
invariant/complexity reasoning plus finite Rust tests, not a new machine-checked
proof, physical bound or measured speedup. The earlier CSV predates this change
and is not qualification of the new binary or whole-search constant-time behavior.

Native retained-search configuration, evaluation, release, occurrence resolution,
navigation, result retirement and pool retirement now time their actual bounded
synchronous bodies when process-monotonic capture is selected. Existing complete,
partial, refused and unknown classifications are preserved. Wrappers delegate to
one measured owning path; nested context/hold/result-validation markers are not
assigned invented independent durations or counted as disjoint work. Durations
can overlap enclosing facade spans and must not be summed as exclusive CPU time.

These are service spans, not ingress wait, end-to-end recipient delivery, a new
population ledger or a search performance SLO. Default-off and saturated-queue
clock suppression remain unchanged; missing/dropped observations are not zeros.
Actual-path correctness/capture-equivalence tests do not replace workload-qualified
search latency, physical-resource, observer-overhead or regression gates, which
remain open. Source-label binding and `/4` discovery are separate unfinished work.

## Native invocation pending-age observation

Native trusted-owner status/outcome queries, immediate stop, forget and retirement,
plus scoped stop request and actual stopped settlement, now time their existing
service bodies under the same optional process-monotonic profile. Off, disabled
and already-full capture retain clock suppression. The common no-stop poll takes
no clocks and emits no fictitious settlement. Request acceptance still is not
settlement, and unknown/interrupted jobs are not converted into clean stops.
These spans preserve the existing `Outcome`, `Control`, `InvocationStopRequest`
and `InvocationStoppedSettlement` classifications; the first two are shared
categories, not separate per-operation population ledgers. Nested run/control
spans can overlap and must not be added as exclusive time. They do not measure
accepted-stop-to-service wait, prove quiescence, or qualify control latency under
load; workload-qualified budgets, resource/overhead measurements and those
independent populations remain open.

`configure_invocation_measurements_v2` explicitly selects the existing process-
monotonic timing profile for the invocation cohort; V1 remains counter-only and
cannot be silently upgraded or downgraded. Both profiles remain privileged native
host controls, not recipient-authorized telemetry or new semantic commands.
Configure the ordinary parent retention budget first. The V2
`required_pool_bytes(maximum_issued, timing)` helper includes the counter pool,
fixed per-lifetime-ticket timing storage and output wrapper; one byte below that
requirement refuses without installing an observer or charging the parent.

The timing pool now also prepays one native index per possible admitted member.
Snapshots traverse admitted members, not never-admitted reserved capacity. The
index grows only on first admission without allocation, and retains retired
terminal members; it is not a live-job list or a reset of cohort history. Existing
unknown-clock/interrupted/unclassified meanings remain intact. Recompute native
budgets with `required_pool_bytes`: the additional index and vector header are
included; physical allocator/RSS costs are not asserted. Work is still linear in
the admitted cohort, including retired history—not constant time in all lifetimes.

The affected 19 native measurement tests and public responsiveness BDD pass. A
follow-up release invocation run again completed all nine cohorts (512 measured,
32 warmup each) and final cleanup. Its total p99 ranged 2.084–4.000 µs across all
modes; the timed observer reserved 18,960 logical bytes for 544 possible tickets.
[Raw output](performance/invocation-indexed-2026-11-06.csv) pins the changed binary.
The earlier checkpoint above measures the pre-index binary. These uncontrolled
runs do not establish a causal latency improvement or a selected product budget.

`invocation_responsiveness_v1` reports one bounded owner-ledger snapshot:

- First admissions after observer activation; old admitted work is excluded.
- Current pending count, unknown pending-age count and oldest **known** pending
  age. If any pending age is unknown, the known maximum is not the true oldest.
- Local terminal count, unknown terminal-duration count and maximum known
  admission-hook → terminal-hook duration, including host delay and service.
- Interrupted/unknown executions and unclassified membership, separately from
  known terminal outcomes; membership completeness is separate from clock coverage.

Snapshots obey `admissions = pending + terminal + interrupted + unclassified`.
Completed durations include refusals, no-ops and stops, not only successful edits.
Unknown/missing/regressed clock samples never become zero or a completed endpoint.
An actual zero duration is valid. The result is neither queue-only wait nor a
guarantee of useful progress, eligibility, result delivery, effect quiescence or
persistence. The existing service probes remain separate. Native `Duration` and
bounded cardinalities are not new generic engine-measurement/1 series; the V2
`values` profile still governs the existing counter series, not an age deadline.

The timing profile is independent of Recorder capture. Explicitly enabling V2
timing retains ages even with `Recorder::disabled()` or a full diagnostic queue.
Admission and terminal hooks have constant observer work and no per-hook storage
growth; snapshots scan at most the configured lifetime issuance bound. No clock,
background scheduler, provider callback or polling loop is installed by V1/Off.
No raw stamps, ticket/document identities or source content are returned. Counts
and timings can still reveal activity: this is not a recipient-safe disclosure
product or privacy certification. Returned copies are historical observations,
not continuously current or revocable facts.

Original native replay never restarts age or admits another cohort member.
Forgetting/retiring a terminal result does not erase the retained aggregate or
reuse its lifetime slot. Existing protected stop makes timing Off; protected
observer retirement discharges its complete parent hold and removes timing state
without stopping original jobs or recovering a faulted engine. Observer loss
marks coverage incomplete rather than selecting publication or fabricating safety.

Verification passes 179 focused invocation unit tests and ten public measurement
scenarios. The new profile first failed on absent APIs; four new units cover exact
funding/current host control, activation/replay, Off/legacy profiles, deterministic
unknown/regressed/zero clocks and interrupted publication. The public scenario
compares unconfigured/V1/V2-Off/V2-timed across disabled/full/available capture,
with replay, stop and committed source checks; existing settings/relation coverage
also exercises the timed common hooks. Strict engine linting and formatting pass.
No independent assurance, formal refinement or performance-overhead qualification
of this new observer is claimed.

| Testing perspective | Evidence in this slice and remaining limits |
| --- | --- |
| Golden signals | Actual pending ages, known terminal maxima, missing samples and diagnostics-saturation independence; not whole-engine percentiles or queue/service separation |
| RED | Existing unique admission/disposition counters reused; replay/no-op/refusal/stop remain distinct from offers and attempts |
| USE | Prepaid finite storage, exact pool boundary, original job/retention preservation and protected refund; no physical allocation/RSS claim |
| STRIDE | Missing/denied host control, legacy-profile substitution refusal, original ticket/replay invariants and interrupted-publication fault fencing; native owner possession remains privileged |
| LINDDUN | Explicit opt-in, no source/identity/raw-stamp fields, sensitive-payload check and protected retention/discharge; generic recipient projection/export and broader privacy analysis remain open |

## Invocation and observer-overhead gate

```sh
make perf-build PERF_EXAMPLE=invocation_responsiveness
make perf-run PERF_EXAMPLE=invocation_responsiveness
```

This second native Rust experiment runs the actual issued-ticket admission →
service → retained-outcome/source query → observer query path. It compares no
cohort, V1 counters and V2 process timing, each with diagnostic capture disabled,
timed/available or saturated: nine combinations. Each operation atomically edits
four original-coordinate sites in each of two synthetic 4,096-byte documents.
It verifies changed participants, coherent versions/generation, every edited byte,
terminal populations and current counter denominators. Refusal, unknown or
incomplete work fails rather than leaving the sample population.

Total latency includes an observer snapshot **after every operation**, as well as
result checks and renewal of the caller's next request. Admission, service,
result-query and observer-query distributions are separate. Issuance, job cleanup
and diagnostic drain are outside that total but inside cohort throughput time.
Initial document/observer setup and final observer retirement are outside cohort
throughput. The supplied total-p99 budget therefore also bounds this deliberately
intensive observation cadence; it is not directly comparable with the first
source-only gate's endpoints.

The lifetime bound equals warmup plus samples; the native observer scans that
bound. This experiment refuses more than 4,096 lifetime tickets before running,
to bound repeated scan work. That is a benchmark limit, not a new engine limit.
Reported observer admissions include warmup, while percentile populations do not.
Unconfigured observation is `unmeasured`, not zero. Active logical observer holds
are reported separately from retained jobs; physical allocations/RSS remain
unmeasured. Every job/result is forgotten/retired and the observer is protectedly
stopped/retired, with zero remaining parent reservation required before success.

The [2026-11-05 warm smoke](performance/invocation-smoke-2026-11-05.csv) covers 4,608
measured operations (512 per combination, 32 warmup each). V2-timed total p99
ranged from 3.208 to 5.750 µs across capture modes; its observer-query p99 was
1.083–1.167 µs with a 544-ticket bound. The active V1/V2 logical holds were 1,400/
14,584 bytes respectively, not measured RSS. The entire process took 0.03 seconds
outer wall time and 0.0137 seconds in main. Fixed ordering and uncontrolled host
load prevent causal overhead percentages or a qualified performance guarantee.

Five focused example tests pass: two new scenario/work-limit tests plus the three
shared profile/percentile/budget controls. They first failed on the absent harness.
Formatting, strict engine linting and release build pass. The Make gate with
`PERF_EXAMPLE=invocation_responsiveness`, 100 samples/two warmups and a deliberately
impossible one-nanosecond budget reports all nine combinations and fails (binary
exit 1 propagated as Make exit 2). Neither that control nor the smoke selects a
product SLO. The default/source and selected/invocation Make build routes were
also inspected independently.

Golden signals/RED cover actual phase tails, useful successes and exact cohort
denominators here; saturation cannot remove the external samples. USE covers
finite scan work, observer holds and verified release, not physical host memory.
STRIDE/LINDDUN evidence is limited to fixed synthetic inputs, no source/identity/
raw-stamp output and fail-closed incomplete-cohort reporting; this harness is not
an adversarial input suite or recipient-policy proof. Existing native correctness
tests retain those responsibilities. Controlled repetition, capacity scaling,
overhead budgets and other engine paths remain open.

## Interpretation and remaining coverage

The [2026-11-05 sequential smoke capture](performance/responsiveness-smoke-2026-11-05.csv)
contains all 6,144 measured operations across 12 cohorts, plus explicit native
diagnostic loss. Across the four capture modes, total p99 was 0.792–1.041 µs for
resident-small, 12.750–13.666 µs for agentic-multi and 46.000–69.292 µs for
resident-large. These are measurements of these fixed synthetic native workloads,
not arbitrary document/semantic-command bounds. All edits and revisions were
checked. Rust main took 0.121 seconds, while outer process wall time was 58.31
seconds (user CPU 0.12 seconds). This exposes a large outside-main delay without
claiming its cause. No host isolation or CPU/frequency qualification was performed.
An unchanged-binary follow-up using
`/usr/bin/time -p make perf-run PERF_ARGS='--samples 100 --warmup 2'` completed
1,200 measured operations in 0.04 seconds outer wall time, with 0.024 seconds
inside main (user CPU 0.02 seconds). This confirms a fast warm development gate
on this host, not a universal startup guarantee or diagnosis of the slow launch.
The earlier smoke overlapped another performance control and is not the retained
sequential capture or a qualified baseline.

The two timing scenarios first failed on missing preparation/commit/query service
durations. After wiring the existing optional timer, 25 targeted tests pass: 16
coordinator regressions, three capture contracts, two timing scenarios and four
benchmark tests. Budget equality/refusal and nearest-rank tails are unit tested;
an actual release run with a one-nanosecond budget reports every cohort and exits
1. This deliberately impossible control tests enforcement, not an adopted target.

Run on a declared host with controlled load, repeat runs and retain the CSV plus
host/build context before adopting budgets. Fixed ordering, caches, frequency,
background processes and small sample populations affect tail comparisons. One
uncontrolled smoke run is not a speed guarantee or evidence that instrumentation
has negative overhead. Concurrent performance runs are not a qualified baseline.

Native resource queue observation now covers the four source/annotation read/save
owners through `configure_resource_queue_timing_v1`,
`resource_queue_responsiveness_v1` and `resource_queue_attempt_timing_v1`.
Opt in before task issuance. Fixed inline task stamps and retired aggregates
require no sample buffer; diagnostic capture saturation does not suppress them.
Pending ages are censored observations, not completed waits. Dispatch ends queue
wait, not provider work; stopping a dispatched task cannot rewrite its wait.
Unknown clocks/count saturation remain explicit. These are host-only observations
without document content or identity labels, not authoritative effect evidence.
The previously recorded release CSVs qualify their recorded binaries only.

Still open: approved workload-specific latency/memory/overhead budgets, regression
baseline comparison, whole-engine oldest eligible
pending age beyond these resource and invocation populations, refusal and
control responsiveness under sustained contention, allocation/RSS measurement,
larger documents/workspaces, Unicode-heavy workloads, configured semantic facade
costs, cold opening, and cancellation/shutdown timing. UI frame/presentation and
provider I/O measurements belong to their respective adapters and need correlation
without moving those responsibilities into the engine.

Runtime overload observation also remains distinct: the bounded diagnostic queue
may drop records and suppress clocks when full. This benchmark's external sampler
prevents that blind spot **in this gate**; it does not install a complete independent
runtime latency/oldest-pending population. The separate native invocation profile
above now covers its actual admitted cohort, not all ingress, queues or services.
Neither descriptor catalogs nor this scoped observer establish the missing whole-
engine populations or operational overhead budgets.
