# Shared native observation admission and retirement

Section 23.1.7 adds explicit retained decoder metadata support and optional copied
decoder profiles at context admission. That route prepays eight logical request
frames alongside native charge in both existing envelopes. Native-only bootstrap
retains the earlier profile. See observation-decoder.md; the original admission
scope below does not itself establish decoder integration.

Specification §17.2.3 connects actual observation contexts to the existing D06
finite-capacity model. One owned ledger per actual invocation authority charges
all contexts routed through this selected pool before opening them. Context/wait
termination is not a reservation refund. This is shared declared accounting, not
a proof of whole-engine physical memory or all origin hierarchies.

## Precedent and fixed scope

[Owned semaphore permits](https://docs.rs/tokio/latest/tokio/sync/struct.OwnedSemaphorePermit.html)
couple permits to a particular semaphore and lifetime; the
[semaphore API](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)
provides nonwaiting acquisition attempts as well as waiting acquisition. These
support owned admission and explicit failure. Vellum does not adopt Tokio,
semaphore fairness, automatic Drop as an external-discharge witness, or a queue.
Its existing §§17.2.1–17.2.2 complete-coordinate and protected-envelope rules govern.

The selected reference has one fixed declared host/origin accounting domain, using
the actual capacity-admission.mjs units and occupancy/consumption distinction. It
does not infer an authenticated origin from a recipient, document, purpose or
workspace, and does not implement dynamic multi-origin ancestry. Actual identity/
origin provenance and whole-host accounting remain separate gates. All routed
observation contexts share both envelopes; changing ticket/file/workspace or local
profile does not create a fresh ledger or erase spent budget.

An actual invocation domain can open this pool only once. Opaque native ownership
rejects structural clones and foreign tickets. Stop cannot reopen it or reset its
consumption, and compatible capacity activation cannot change the declared clock
context. Native authority incarnation/uniqueness still relies on the existing
domain contract. There is no new service, network quota protocol, per-file engine,
public reservation/acquisition verb or selected wire encoding.

Pool bootstrap and profile activation require explicit current capacity-activation
authority as well as owner control. Ordinary context admission neither requires nor
grants that administrative permission. Do not infer quota-changing rights from
observation, possession of a reservation or protected stop authority.

## Producer charge and protected promise

The observation producer exposes its one canonical local charge calculation. Both
the lower-level producer and the pooled path use it; no caller estimate, missing
demand or null value becomes zero. Before calling the actual opener, reserve that
whole local charge, one ordinary context slot and one protected completion promise
in both declared envelopes. The local charge already includes ordinary replies,
protected reply/two phase-reference units, context, wait and ordinary-reference
maxima from §23.1.4. The protected slot is the container's completion promise, not
a substitute for those byte charges or a measured service guarantee.

Each granted admission consumes one declared admission step and child grant. These
are logical reservation-domain units, not CPU milliseconds or an assertion that
every API/native/provider instruction is charged. Consumed work/fan-out does not
refund when an observer disappears, a setup attempt fails or an operation retries.
All-owner CPU/native/provider accounting remains required and unproved here.

Pool metadata is precharged in both byte envelopes at startup. Its bound includes
two maximal fixed-shape profile/ledger frames, bounded live reservation records and
supervision counters. Safe finite arithmetic and a separate bookkeeping ceiling
are checked before installing the owner. Maximal numeric representations cover
later compatible capacity activation; live-record count remains bounded by the
original pool profile. Actual allocator overhead, transient physical peaks and
host ingress work are not established by these serialized logical units.

Use the actual admission helper's current headroom subtraction for all coordinates
and both scopes, including the protected completion partition. Failure refuses
before native context creation, without a waiting queue or partial promise. Once
reserved, call the actual native observation producer and compare its resulting
declared charge with the reservation. The owned local registry now supplies the
lower-level registry-ready premise for this selected route; it does not manufacture
host retention, current rights, clock or physical reservation truth.

## Failure, retirement and activation

Known setup refusal returning no context releases only the reserved bytes, ordinary
slot and completion promise. Its admission step and child grant remain consumed.
No producer invocation is run, cancelled or refunded. An exception or reservation
mismatch marks the record Unknown and fences further pool admission, retaining
its charge and bounded reservation identity. Do not infer safe absence or silently
retry through another pool. Unknown recovery/authority teardown still needs an
independently supported actual discharge path; this slice does not invent one.

The native context retains its parent invocation-authority handle, not a separate
historical source snapshot. Its existing ancestor owner must keep shared authority
and root charges live until child references are discharged. Releasing this pool's
local occupancy does not establish ancestor release or physical reclamation.

Successful admission binds the opaque reservation to the exact actual observation
context. There is no caller-supplied replacement context at release. Retirement
requires current owner control, delivery drain and retention discharge. It first
retires that native context, or recognizes that it has independently retired, and
verifies that the native context owner is absent. Outstanding waiter records block
native retirement and therefore block shared refund. Stop/expiry/context close alone
does not satisfy this predicate. Independently retiring a context does not cause an
automatic or implicit shared refund.

Only then remove the reservation and subtract its reusable byte/slot/completion
charges from both envelopes. Keep startup bookkeeping, consumed admission/fan-out
and monotone issuance. Copied, foreign, duplicate or unknown reservation references
cannot debit another record or manufacture headroom. Host drain/discharge truth is
still a premise; observing null local ownership is not a physical memory certificate.

Activation uses actual activateCapacity against the retained ledger. Incompatible
byte, slot, protected-floor or consumed-quota shrink preserves the old profile/state.
Compatible changes preserve spent consumption, so remaining quota is new ceiling
minus spent, not a reset. Soft targets do not supply admission rights. Clock/context
changes and pool-limit migrations require separate supported contracts. Stopping
new admission retains all charges and does not disable already admitted protected
status or stop unrelated native producer work.

## Conditional proof and executable evidence

Assume the trusted serialized native domain, truthful fixed accounting scope and
host permission/retention/service facts, valid complete profiles, and producer charge
refinement. Existing D06 admission checks each demand against remaining headroom,
so it preserves both accounted envelopes and their nonborrowing ordinary/protected
partitions. Startup metadata is charged before pool installation. A successful
owned reservation precedes native observer opening, and charge equality relates
the actual producer to that reservation.

For known refusal or successful retirement, subtract only the reusable coordinates
of that one owned record after the corresponding absence/discharge predicate.
Consumed coordinates and other records remain unchanged. Induction over admission,
refusal, quarantine, close/stop, retirement and compatible activation preserves the
declared ledger bounds and non-resetting consumption. Unknown paths retain rather
than fabricate release. Existing resource/capacity solver conditions supply the
coordinate lemmas; opaque ownership/native coupling is separately exercised here,
not automatically proved by those arithmetic predicates.

observation-admission-checks.mjs runs actual capacity admission/activation and native
invocation/observation/context/wait/retirement functions. It checks 30 scoped boundary
cases, all 24 orderings of close/stop/wait-retirement/reservation-retirement (96
observer edges), twelve controls and fifteen assertion-rejected semantic mutants,
with a passing unmodified dynamic baseline. Independent prospective ledgers check
base plus actual producer charges, live ordinary/completion use and preserved
consumption. Tests include two actual tickets/different local sizes, both envelopes,
protected pressure, known/Unknown failure, wait retention, independent native
retirement, compatible/incompatible activation, fixed clock context, duplicate
authority ownership and producer completion despite observer stop/fault.

These are finite integration cases, not exhaustive concurrent histories. No new
solver conditions are added: totals remain 223 across thirteen manifests. The
existing coordinate, scope and occupancy-lattice arguments remain conditional;
there is no new whole-engine lattice/completeness, physical-accounting, fairness,
timer, Rust/native or independent certification claim.

## Instrumentation and remaining gates

PATH-01/02/12 and IF-01/03/08/12 distinguish pool startup, resource admission,
native context creation, protected promises, known refusal, quarantine and discharged
retirement. INS-01/03 record offered/admitted/delivered observations, not an extra
authored publication; INS-04/07 retain producer/provider discharge distinctions;
INS-08 tracks retained contexts/waiters; INS-12 diagnostic loss; INS-13 useful
completion/control. Private observationAdmissionUsage exposes declared ledger,
profile, live/unknown count and spent issuance for trusted supervision only.

MELT/RED/golden signals distinguish capacity refusal, native setup failure, Unknown,
useful delivery, retained age and phase latency. USE separates reusable occupancy,
consumed quota and protected promises. STRIDE challenges quota resets, duplicate
owners, wrong-scope admission, premature release and hidden charge edits; LINDDUN
challenges resource-attribution and private observer counts. No raw identities,
reservation/context handles or private counters as ordinary public metric labels.
Telemetry does not establish demand bounds, safe release or grant authority.

Physical global accounting, all engine owners, true origin hierarchy/identity,
actual clock/service/retention/drain, unknown recovery, public reservation/encoding
and concurrent/native/decoder/production/independent assurance remain. The lower-level
unpooled observer is a trusted producer primitive, not an alternate public bypass
of required shared admission. No whole D06/D35/F06/F19 or all-sections goal closes.
