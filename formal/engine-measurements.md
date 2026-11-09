# Engine measurement semantics

engine-measurement/1 defines a finite engine measurement catalog and closed logical
value shapes. It refines specification §23.3 and model §11, not the public command
facade, a wire format, an exporter or an installed instrumentation claim. The JSON
catalog supplies exact descriptors; this contract supplies their common meaning.
The existing boundary-marker/1 queue remains unchanged and cannot carry these
richer values by adding fields under its old profile name.

## Coverage and ownership

The catalog covers engine INS-01–09/12–13. UI INS-10/11 remain in the UI example
stream. Each supported placement binds applicable descriptors to its actual
producer, owner/accounting scope, provenance and collection/release permission.
An unavailable supplier or inapplicable phase has an explicit unavailable or
not-applicable placement disposition; it does not synthesize an observed zero.
Not-applicable describes a placement, not a successfully observed data point.
Existing observability.json retains the interface/path/identity/data-flow crosswalk.

One logical producer owns each series. A series is a descriptor plus its exact
allowed finite dimension tuple and an authorized accounting scope. Private owner
instances, documents, users, paths, bucket/key names, registry names, profiles,
versions, attempts and correlation IDs are not metric dimensions. Multiple owners
cannot be merged merely by deleting distinguishing metadata; aggregate only under
a declared comparable population and permission covering the whole aggregate.
An authorized subset cannot be reconstructed by subtracting hidden totals.

The descriptor's id, site, type, unit, population, boundary and allowed dimensions
are exact semantic metadata. The type selects its aggregation rule. Unknown
descriptors, fields, dimension names or dimension values refuse the measurement,
not the editing operation. No free-form dimension or formatted host error is allowed.
Adding meanings requires an explicit compatible/new profile, not silent widening.

request.dispositions resolves each ingress offer as first-admitted, replay-observed,
refused, stopped, unavailable, unknown or fault. This includes refusals before any
logical invocation exists. Offers minus first admissions is not a refusal count:
it also includes replay observations and unresolved offers. Invocation disposition
counts remain a separate admitted population; no propagated failure is counted
as an additional original invocation failure.

Derived classifications preserve accepted/partial/stale/superseded/unsupported
versus ordinary refused/stopped/unavailable, unestablished unknown and actual fault.
A fresh well-formed result refused for authority or capacity is refused, not stale
or fault; stopping is stopped. When detail cannot be established or disclosed,
use unavailable or unknown as applicable rather than inventing the cause. These
are diagnostic classes under their collection grant, not new engine outcomes.

## Closed point shapes

Every point has exactly the fields listed by pointFields in the JSON catalog.
profile and measure select the declaration. generation is a bounded owner-issued
opaque series-generation reference; it is not an exported label or authority.
sequence is a bounded monotonically increasing observation ordinal in that series,
not a clock or operation ID. dimensions contains exactly the descriptor's names
and values from the finite dictionaries. The other fields have these meanings:

| Field or type | Exact value rule |
| --- | --- |
| status | observed, off, unavailable or unknown. Only observed carries a non-null value. |
| freshness | current, stale or unknown. Current means the snapshot was current at its declared observation boundary, not that it stays current at later receipt. |
| coverage | complete, sampled, inexact or unknown for the stated population/interval. Record delivery coverage is not engine execution coverage. |
| counter value | Exactly total and exact. total is a bounded nonnegative integer; exact is Boolean. Sampled/inexact/unknown coverage cannot claim exact totals. |
| gauge value | Exactly amount, a bounded nonnegative integer at the declared boundary. No implicit addition across time, overlapping scopes or resource meanings. |
| duration value | Exactly seconds and endpoint. seconds is finite, nonnegative and within its declared ceiling; endpoint is completed, pending or censored. |

Non-observed points have freshness and coverage unknown and value null. Observed
zero is therefore distinct from off, unavailable, unknown and missing data.
Point validity establishes shape and declared bounds only. An authentic producer,
truthful scope, actual observation, current disclosure and funded capture remain
separate requirements; caller-supplied metadata cannot establish them.

The generation resolves an owned retained provenance record: producer/accounting
scope, collection/release scope, descriptor/dimension binding, interval start and
observation cut, applicable clock domain/epoch, continuity disposition and captured
limits. These are restricted existing owner/clock/scope facts, not extra metric
labels or a new recipient query. Without this context the point is uninterpretable,
not independently authentic. Delta derivation additionally requires established
continuous accumulation across the whole interval; endpoint shapes and matching
generation labels alone cannot establish it.

Each activated measurement profile supplies finite ceilings for point bytes,
series, generations, observation ordinals, integer values, duration values,
capture/aggregation workspace, retained points/links and retention. These are
logical resource obligations under §§17.2/23.2, not selected production defaults.
Refuse unsupported limits before activation. Compatible activation retains live
charges, spent series/generation issuance and the interpretation of queued values;
incompatible shrink refuses without confiscating commitments. No counter or
sequence wraps. Counter saturation retains the bounded total and makes exactness
false; it does not silently reset or infer zero. New generations explicitly mark
resets/gaps and do not replenish admission/collection quotas.

## Units and resource populations

Durations use seconds, encoded-output sizes use By, and integer populations use
the catalog's annotated count units. This follows [OpenTelemetry's unit guidance](https://opentelemetry.io/docs/specs/semconv/general/metrics/)
without requiring its SDK or ecosystem types in the core. The special coordinate
unit denotes a tagged sum, never an interchangeable scalar:

| Coordinate | Unit and meaning |
| --- | --- |
| normalized-bytes | By of the explicitly declared normalized logical accounting representation; not heap/RSS or allocator peaks |
| source-octets | By of source bytes in the declared source representation; not output bytes under another encoding |
| slots | {slot} in the declared owner pool |
| completion-promises | {promise} for protected completion obligations |
| work-steps | {step} of the declared nonrefundable work profile; not CPU seconds |
| child-grants | {grant} of the declared nonrefundable fan-out profile |

Occupancy and commitments use reusable coordinates only; consumed uses work-steps
or child-grants only. Capacity may describe either kind, but utilization/headroom
requires the same coordinate, effective profile, owner scope and observation cut.
Commitments and occupancy may overlap and must not be added as independent use.
Unknown denominator/capacity yields unavailable, not zero utilization. Physical
memory, native CPU, network service and provider-reported costs need separately
supported substrate/host measurements; this engine profile does not invent them.

## Timing and aggregation

Every duration binds the descriptor's named start/end or cutoff milestones to
the existing trusted host clock contract. Both endpoints must be valid, ordered
and in the same monotonic domain/epoch with a known conversion to seconds. Missing,
invalid, reversed or cross-epoch evidence produces unavailable. Do not subtract
wall clocks, enqueue/export timestamps or timestamps from another host. Preserve
clock resolution/validity as restricted provenance; numerical precision does not
assert clock accuracy. Sequence ordinals cannot supply elapsed time.

The descriptor boundaries separate admission, preparation, final publication,
release, effect queue/dispatch/evidence observation, configuration and retained
discharge. effect.first-result.duration ends once, on the first authentic attributed
acknowledgment, failure, unknown or contradiction result observation after dispatch.
Application unknown and failed synchronization still end this first-result phase;
they do not establish successful persistence. Progress, quiescence and duplicates
do not end it. Quiescence before a result leaves the phase pending/censored, and
later evidence cannot create another completed first-result sample. Without the
original episode/first-result tracking evidence, the duration is unavailable.
Dispatch-to-receipt observation includes provider time plus observation delay and
is not physical service time. Synchronous
execution does not require an invented queue phase; its placement can be inapplicable.

Each duration descriptor admits completed observations and separately classified
pending ages/censored observations from the same start milestone. A cutoff does
not claim completion, failure or quiescence. Keep pending population and censoring
visible when interpreting completed distributions; do not merge pending samples
into completed-duration histograms or turn fast refusals into useful throughput.

Counters are cumulative within one series generation; gauges are current values;
durations are individual observations. Rate/delta derivation requires the same
descriptor, dimensions, producer/accounting scope and generation, increasing
observation order, complete exact counts and no reset/gap. A rate additionally
requires a valid positive same-domain elapsed interval. Unknown/mismatched evidence
refuses derivation. Histogram aggregation, temporality conversion and workload
buckets belong to an explicit adapter/profile; none is selected by a point alone.
These distinctions follow [OpenTelemetry's data model](https://opentelemetry.io/docs/specs/otel/metrics/data-model/),
not an adoption of OTLP timestamps or transport. Workload-dependent thresholds,
resource defaults, tail objectives and service claims still require E03 evidence.

## Correlation and safe capture

Bounded owner-issued opaque event/trace links may relate already authorized
episodes and fan-out/fan-in. They are not metric fields, authority, document
identity or proof of causality. Their permitted relation kinds are parent,
retry-of, coalesces and evidence-for. Bind scope, purpose, recipient/destination,
generation and lifetime; omit the link when those facts are unavailable or its
release would expose hidden activity. Imported caller IDs or baggage are not
forwarded. No network propagation/header or richer disclosure grant is introduced.

Use the existing callback-free bounded capture/current-release rules. Optional
measurement pressure may drop new observations with sticky coverage/loss status;
it cannot erase an authoritative outcome, consume protected completion capacity,
wait for a sink, execute a formatter or trigger automatic remediation. Loss of
delivery is separate from inexact accumulation: a retained exact cumulative
counter may bridge a missed delivery only with actual same-generation continuity
evidence. No such continuity is inferred from silence or the descriptor.

STRIDE challenges forged producers, tampered values, loss/attribution, disclosure,
cardinality pressure and diagnostic privilege. LINDDUN challenges linking and
inference from dimensions, timing, aggregates and opaque links. USE/RED/golden
signals reuse these populations and dimensions rather than creating parallel
counts with different denominators. Collection/export is not an authority path.

## Evidence scope

The declared point vocabulary is a closed tagged sum, not a claim that the whole
engine or all observation qualities form a bounded lattice. Exact field sets and
finite dimension dictionaries exclude extra semantic fields; each type's value
rule excludes its other branches. Bounds are supplied finite profile premises.

For a counter state 0 <= total <= maximum, adding an admitted nonnegative amount
uses remaining headroom. If it fits, the sum stays within maximum; otherwise the
result is maximum and exactness becomes false. Exactness is a conjunction with
its old state and therefore cannot recover without an explicit new generation.
Induction over these steps preserves the bound and excludes wrap/reset. This
logical counter argument does not replenish the owner's consumed quotas.

The first-result episode has two abstract states, waiting and seen. An authentic
attributed result in the named set changes waiting to seen and supplies one
completed first-result observation. All other evidence preserves the state;
seen never produces another completed sample. Observation cutoff records pending
or censored age without changing the effect's application, synchronization,
quiescence or baseline state. This finite phase argument assumes truthful
attribution and retained episode state; the reference predicate does not implement
or prove their actual provider/runtime composition.

The reference checker exercises the finite descriptor/field vocabulary, point
shapes, tagged resource coordinates, saturation, safe delta derivation and local
clock conversion on normalized inert owned values, not hostile proxies/getters or
unbounded raw ingress. Its supplied generation/clock/provenance values are not actual
host attestations or a public codec. Descriptor/coverage checks do not prove
installed probes, allocator costs, physical accounting, private-path containment,
actual clock accuracy, full runtime composition or independent certification.
Implementation, current grants, funded aggregate/capture state and real workload
measurements remain explicit E01–E05 gates. The profile resolves measurement
semantics; it does not turn those gates into model-proved facts.
