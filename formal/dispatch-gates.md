# Dispatch and historical-evidence gates

PS-DG discharges a narrow part of the outstanding boundary work in
[save admission](save-admission.md). It does not adopt Busy/FIFO, reservation or
single-flight policy for v1. It is a separate reference model; full composition
with the admission, execution, persistence and concurrent-save models remains
an obligation, not a consequence of similar state names.

## Different gates, different authority

| Boundary | Required basis in this model | Must not be inferred |
|---|---|---|
| Final local launch | Current caller grant; captured binding and provider-context epoch still match | Admission-time permission is permanent; an old capture may target a new binding |
| Incoming observation | Established writer provenance and exact historical attempt/resource/incarnation/binding/epoch/profile/payload tuple | A claimed attempt ID or trusted-looking label authenticates a message |
| Resolve outstanding coordination | Accepted observation plus safe-disposition kind and a supported provider guarantee | Progress, cancellation intent, source authenticity alone or caller timeout means no future write |
| Retain historical observation | Valid attempt-scoped evidence, independently of current caller grants/current binding | Retention authorizes public release, saved-baseline reassignment or new binding activation |

The reference snapshot keeps its admitted source representation. A later source
edit alone does **not** invalidate saving that exact older capture. Current
launch authority, destination identity/fences and provider context are separate
from a “save latest live text” contract. Real declared dependencies, serialization
profiles, expected external-state tokens and final policy checks still need the
full operation/host contract; the three launch predicates are not their discovery
or implementation. Conditional storage races are modeled separately in PS-CS.

`InvalidateBinding` is an environment invalidation of the currently observed
binding generation. It is not an editor rebind operation or permission to bypass
the existing unknown-write/rebind prohibition. An old observation can remain
historically valid while being ineligible to establish the current saved baseline.
The provider epoch similarly fences launch context; it is not a blanket rule to
trust revoked/compromised issuer keys. Provenance must actually validate under the
declared security contract, including any applicable compromise handling.

## Authentication is not effect semantics

The model uses `Writer` versus `Stranger` as an **oracle** for established
provenance, not as a cryptographic mechanism. Merely receiving the string Writer
in an implementation cannot satisfy that oracle. A supplier must justify issuer
identity, integrity, intended audience, attempt scope, replay handling and bounded
parsing before an observation reaches authoritative reconciliation.

`Completed` and `Quiescent` are finite representatives of provider-supported
safe dispositions. They do not encode success versus failure, exact durability
strength, or every platform-specific outcome. `Quiescent` means the provider
contract establishes no future effect from this attempt. `Progress` and
`CancelIntent` never establish that fact. Even an authentic Completed/Quiescent
claim cannot discharge the obligation without the supported guarantee profile.
The unattested-provider profile retains authenticated observations without
inventing safe resolution. It is an evidence-conservatism test, not approval to
dispatch through an unsuitable provider in a real requested-guarantee profile.

| Assurance construction | Benefit | Limitation / recommendation |
|---|---|---|
| Validated typed local host boundary plus explicit provider contract | Fits local v1 without imposing federation machinery | Establishes only the declared local trust boundary; isolate untrusted adapters and test supplier conformance |
| Authenticated inter-process/remote channel and scoped credentials | Can establish remote provenance when remote execution is in scope | Does not by itself prove no future write, authorization freshness or durability; mechanism remains deferred with federation |
| Provider-enforced scoped authorization and conditional/fenced writes | Can enforce constraints nearer the actual effect | Capability-dependent; revocation/fence timing and atomicity require an actual provider guarantee, not a core-side check-then-write assertion |

These constructions are composable, not substitutes for one another. No new
transport, credential format, signature scheme or remote-v1 scope is selected.
[NIST SP 800-207](https://csrc.nist.gov/pubs/sp/800/207/final) supplies the
resource-focused zero-trust anchor: location/ownership alone does not establish
trust, and authentication and authorization are distinct decisions. The exact
receipt/disposition rules here are derived editor contracts, not claims that
NIST prescribes this protocol or certifies it. Reference checked 2026-10-23.

## Machine-checkable scope and controls

`DispatchGate.tla` explores two one-shot attempts, one canonical resource,
five-transition horizons, binary observed binding/provider epochs and two
symbolic source versions. Incoming events vary eight tuple cases (exact, wrong
resource/incarnation/binding/epoch/profile/payload/attempt), two provenance classes
and four disposition classes. These are category representatives, not a parser
or an enumeration of arbitrary malicious bytes.

EndQueued now supplies explicit Cancelled/Expired terminal states for the
admission lifecycle composition. It applies only before dispatch; neither action
can cancel an already-dispatched physical obligation. Terminal stability checks
include both states.

Twenty cases comprise two exhaustive safety profiles, eight weakened controls
and ten witnesses. Safety covers authority/binding/epoch at local launch, exact
trusted retained observations, retention of valid historical observations, safe
resolution, single-flight, immutable captures, monotone evidence and stable
terminal/stopped control. Identical observations union idempotently; this does
not bound malicious duplicate processing cost or prove a transport replay cache.

Witnesses establish accepted safe disposition, rejected revoked authority,
rejected stale binding/epoch, permitted older source capture, ignored bad input,
progress without resolution, historical evidence after binding invalidation or
caller revocation, and late safe resolution without restarting stopped control.
Each safety run explores malformed/provenance classes; the bad-input witness is
not by itself exhaustive adversarial testing.

Controls separately remove each launch predicate, trust a foreign issuer, compare
only attempt ID, promote progress to resolution, assume an unsupported provider
guarantee, or incorrectly gate historical retention on current caller/binding
state. Each must fail the named property rather than merely produce a tool error.

## Feedback into requirements and instrumentation

At IF-04/INS-04 distinguish denied/stale launch, rejected provenance/tuple,
accepted nonterminal observation, supported safe disposition and unsupported
guarantee. Rejected input must not consume authoritative completion reservations
or be reported as successful resolution. Separate ingress work/loss accounting
from accepted evidence. Flood resistance, bounded parsing, protected capacity and
actual service still need the §7.5/§12.7 contracts and implementation evidence.

At IF-10/INS-08 distinguish observed identity invalidation from authorized binding
activation and historical baseline ineligibility. At IF-13/INS-07 retain exact
profile attribution without logging payloads or raw provider tokens. Internal
attempt correlations are not public identity or priority credentials. Current
recipient projection still applies to all externally released diagnostics.

STRIDE: spoofed/altered receipts cannot release obligations; current grant/fence
checks prevent stale launch authority; retention must not be suppressed merely
because caller permissions changed; private historical evidence must not leak
through error classes, metrics, callbacks or timing claims. A provider saying it
cancelled is not a proof that its worker/write is physically quiescent.

## Remaining supplier and refinement gates

1. Prove/refine the final local gate and handoff atomicity. This model does not
   guarantee that revocation after launch prevents an already-dispatched write.
2. Establish actual issuer/channel validation, compromise/revocation policy and
   provider-specific safe-disposition semantics. The oracle assumptions are not
   discharged by checking the receiver's conditional statements.
3. Refine dependency/fence/profile matching, exact serialization and actual host
   conditional writes. Source revision ordering alone is not freshness evidence.
4. Generalize the [bounded composition](save-composition.md), which invokes both
   actual module relations, to the complete resource/cost/completion lifecycle,
   persistence baseline and recipient projections. No current-binding check may
   accidentally erase historical evidence; no retained fact may bypass release
   authority or create a fresh grant.
5. Supply lifecycle/service/retention and independent assurance evidence before
   any whole-system, clean-room certification or implementation-correctness claim.
