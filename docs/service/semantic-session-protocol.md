# Semantic session protocol V1

`vellum-semantic-protocol` is the provider-neutral domain shared by the local semantic
service, transport-neutral session client, pure serialized broker and future MCP peer.
`vellum-semantic-wire` is its strict canonical byte representation.
Neither crate owns sockets, process lifecycle, authentication, source persistence or native
editing commands.

V1 addresses one singleton workspace session. A service restart creates a new opaque incarnation.
Every post-join command and reply carries the exact incarnation and membership; every exchange
carries an independent opaque correlation reference. Correlation is never authority. Only
mutations carry a durable operation reference, whose canonical intent and reply remain replayable
for the operation-budgeted membership lifetime. There is no silent replay eviction or
operation-reference reuse. A lost Join reply replays the same server-issued membership.
`OutcomeUnknown` is a transport disposition, not an application reply.

Roles form the independent, attenuating `Synchronize x Query` product. A Join requests roles; the
trusted broker grants only the meet with installed policy. V1 permits exactly one active
synchronizer and synchronization epoch. Begin fences query readiness; from Ready it seeds a
transactional stage from the committed projection so Change/Open/Close can form the next epoch.
Complete atomically publishes only the exact declared canonical manifest. Epoch identities never
repeat within one incarnation, including after Leave/rejoin. Synchronizer loss or Leave performs bounded epoch cleanup, blocks
replacement until cleanup closes, leaves readiness Unavailable, and requires a fresh full sync.
A partial prior epoch is never resurrected.

One broker-owned sovereign semantic-service client/partition owns the shared workspace snapshot
set. External memberships are multiplexed above it; they do not map one-to-one to service clients.
The broker alone enforces membership, role and request isolation. An external request identity is
unique only within its membership, so the broker maps `(membership, external request)` to a
collision-free internal service request and rewrites completion identities back. Raw external
request identities never enter the globally keyed service, and collisions never become
cross-membership existence oracles.

Collect is an exact, non-destructive lookup into bounded per-membership retention, never a FIFO
scan. Absent and foreign requests coalesce. A terminal delivery remains until explicit Release;
Release may drop result bytes but retains a non-reusable tombstone for the membership lifetime.
Cancel Accepted means admission only: the caller must still Collect the terminal delivery. Pending
and Complete request identities must equal the collected request, and Unsupported must match the
submitted request kind. Membership operation and request budgets are explicit; exhaustion requires
orderly Leave and rejoin (plus full resynchronization for a synchronizer).
The advertised operation count is ordinary-work headroom only. Each accepted membership retains a
closed retirement tail equal to the maximum synchronized-document plus pending-request ceilings,
and replay-byte admission preserves the corresponding fixed Leave intents across the concurrent
membership ceiling. Ordinary exhaustion therefore cannot consume the ability to cancel and close
the membership. Each fresh Leave step must reduce its bounded retirement obligations; exact replay
remains inert. Retired replay records are not silently compacted, so incarnation-global replay-byte
exhaustion still requires an explicit service restart/rotation rather than an invented rollover.

The manifest uses domain-separated SHA-256 document leaves over document identity, exact revision,
language length/bytes and source length/bytes. Its root covers sorted fixed-width document/leaf
pairs, count and aggregate source bytes. Duplicate document identities refuse. This lets the pure
broker retain bounded source-free seals while the sovereign semantic service remains the sole
source owner. The commitment is equality only in the digest-induced quotient under the explicit
SHA-256 collision-resistance assumption: it is integrity/correlation evidence, never injective
proof, literal source equality, replay identity or authority. Exact replay retains the literal
immutable intent; source-bearing snapshot clones share `Arc<str>` storage and accounting charges
logical retained bytes rather than pretending each structural clone allocates another copy.

The session broker is a pure reducer. Composition supplies an already authenticated subject,
session, allowed-role ceiling and policy epoch with every command. The reducer returns either a
reply or one typed affine effect with a non-cloneable continuation; an interpreter owning the
single sovereign semantic-service partition consumes the effect and returns its exact outcome as a
later event. Replay intent and result reservation enter state before effect publication. The core
does not call a provider, pump a queue, retry, read a clock, authenticate, or perform I/O.

The first background composition is a transport-neutral serialized actor, not a socket server.
It owns the reducer and interprets at most one affine effect at a time through a synchronous,
nonblocking provider port. A command is either rejected before mailbox admission as `NotSent`,
accepted and resolved as one exact reply, or accepted without a trustworthy reply as
`OutcomeUnknown`. Queue saturation, draining and closure are pre-admission facts; a worker or
effect-port failure after admission is never rewritten as `NotSent`. The actor never retries an
effect, and dropping a reply ticket neither cancels nor rolls back accepted work. Trusted provider
delivery, provider state, policy loss and retirement drive use a separate bounded control path so
client saturation cannot suppress authority revocation or cleanup. Draining closes command
admission immediately; quiescence requires the broker's exact membership, effect and cleanup
gauges to close rather than being inferred from a closed channel.

The actor is volatile in V1. Restart creates a new incarnation and requires rejoin plus full
resynchronization. It does not persist memberships, sessions, roles, replay entries or result
receipts. Durable semantic recovery requires a separate journal and a protocol-level
cross-incarnation lookup/rebind algebra; an editing-operation journal cannot silently supply that
meaning, and a transient authenticated session must never be restored as durable authority.

A later local socket composition requires one persistent authenticated channel. The existing
editing connector's one-request-per-connection handshake creates a fresh authority session for
every exchange and therefore cannot carry a semantic membership across Describe, Join, Submit,
Collect and Leave. A semantic connection must authenticate once, retain that exact host-issued
session for its bounded lifetime, and process strictly sequential length-prefixed frames with one
request in flight. Authentication establishes attribution only. Allowed semantic roles are the
meet of the operator ceiling, authenticated policy grant and requested session attenuation; they
must not default to top, and Query never implies Synchronize. EOF, timeout, malformed framing,
policy revocation and shutdown cause policy loss followed by bounded retirement.

Transport phase truth remains explicit. Canonical encoding or a pre-send capacity refusal is
`NotSent`. Any write/read failure, EOF, timeout, oversized response, malformed/noncanonical reply
or reply-binding mismatch after transmission is `OutcomeUnknown`. Reconnect never implies replay:
every attempt uses a fresh exchange correlation, and only the client ledger may explicitly replay
the same retained mutation operation and literal intent. Without a separately specified
authenticated session-resumption protocol, connection loss can leave an uncertain mutation
permanently unknown to that client. Plain Unix-domain bytes and HMAC attribution provide neither
confidentiality nor peer-process isolation.

`vellum-semantic-transport` realizes only the transport-neutral one-attempt part of that contract.
It encodes one canonical command, calls an injected frame exchange at most once, and accepts only a
canonical reply with exact kind, exchange, operation, incarnation and membership binding. Its
fixed-cardinality counters report attempts, logical request/reply frame bytes and closed outcome or
refusal classes without identity or content labels. The adapter necessarily trusts the injected
port's `NotSent` claim; it cannot observe independently whether a lower transport emitted bytes.
It owns no listener, authentication handshake, connection/session lifetime, server dispatch,
reconnect or retry policy.

The legacy local semantic runtime and session broker both terminalize requests and therefore remain
deliberately non-composable. A separate ordered local mapper now coordinates a fresh-only bridge
service, its one sovereign client, broker genesis and the sole provider epoch/projection owner as
one affine prepared value. Every operation is fallible, construction and thread-launch refusal
retain exact cleanup ownership, and no raw service or mismatched broker/port pair can escape. This
base composition is explicitly injected-provider and deadline-disabled. A production local-LSP
composition now also has fresh-started adapter admission, a specialized cross-limit proof, a
distinct timed scheduler typestate, a qualified supervised-process constructor and consuming
checked/forced provider terminal receipts. The first initialize admission is linearized against
Vellum's supervised stdout pump; this does not claim that an OS or ProcessKit buffer was physically
empty. Disabled and timed construction retain every prelaunch input, while postlaunch failures
retain transport, adapter or bridge cleanup ownership. A normalized affine recovery owner joins
actor/build origin, broker/actor usage, deadline disposition, client retirement, extraction mode
and the consuming terminal receipt. The legacy probe is not silently upgraded.

Launch timeout and cleanup uncertainty are not ordinary refusals. An affine unresolved-launch
owner retains the worker and singleton fence and resolves nonblockingly to confirmed no
application, confirmed cleanup or restart-required indeterminacy. Only confirmed absence or cleanup
may clear the fence. Dropping the owner transfers it to an exact-once recoverable process-global
slot; restart-required evidence leaves the fence set. This is conservative process-local recovery,
not automatic provider restart or proof of descendant cleanup.

A lawful bridge requires distinct ordered service transition receipts. A fresh-only bridge service
type, not an optional flag on the ordinary service, prevents compositions from mixing legacy
synthetic delivery semantics with broker-owned terminalization. One Change or Close can observe a
provider transition before synchronization, a source admission, and another provider transition
after synchronization; provider transitions may occur even when source admission is unavailable or
refused. Its report must therefore preserve the fixed product
`pre-provider × source-admission × post-provider`, with separate exact bounded canonical receipts
and causes, rather than relabeling their union. The broker records its expected set when it emits the
effect and validates the entire report against a shadow live-request set before atomically
publishing success. Missing, extra, duplicate, unordered, foreign or wrong-cause evidence is fatal
composition divergence: the
broker latches poisoned, the actor fails while preserving its broker and port behind a separate
affine recovery capability, and the exchange remains outcome unknown. It is not an ordinary
quarantined delivery after which the actor may keep serving.
Close cause does not choose the external terminal outcome; the broker remains the external
lifecycle owner.

The ordered product is operation-specific rather than one generic admission enum. Open has no
source-invalidation receipt. Change can accept only a `SourceChanged` receipt; Close can accept
only `SourceClosed`. Submit observes a pre-provider transition, then submission, then a
post-provider transition; an accepted request may therefore be installed and immediately
invalidated by the post transition. Cancel and deadline expiry order their named terminal action
before any post-provider transition and do not synthesize a bridge delivery. Expiry is a sixth
serialized actor effect whose exact outcome is Expired, pre-transition-satisfied or known
not-pending; only that path can produce TimedOut. One successful provider poll is
`pre-provider × poll × post-provider × disposition`: if the post transition invalidates the polled
request, the completion remains quarantined rather than becoming an ordinary delivery. Physical
provider delivery is Peer-only and cannot fabricate cancellation or timeout. Provider transition
and completion cannot be split across controls or interleaved with a command. Raw provider-state
and delivery events are not part of the actor or broker event surface.
Every public bridge operation must use these receipt-producing paths; exposing a legacy method
through the wrapper would silently reintroduce synthetic terminalization.

The timed local profile owns a bounded exact `request -> due` relation and its
inverse under an injected monotonic clock and fixed relative budget. Both indexes and due values
are validated before every service effect; reservation precedes Submit, installation occurs only
for an accepted request still live after post-transition, and exact receipts clear obligations
infallibly. One drive selects at most one canonical earliest due request. Actor stop observes a
mandatory fixed-cardinality obligation projection and refuses live, inconsistent or poisoned
scheduler state. Retirement yields a must-use Disabled, exact discard or Unresolved receipt;
Chronos-backed scheduling remains a separate future composition that must join Chronos retirement
evidence rather than treating local deletion as cleanup.

A provider transition stage contains the first state that triggered the observation and the
settled state after all cancellation signals caused by draining its pending set. Pending work is
drained once, so settling cannot duplicate invalidation. The complete stage maps to one monotone
composition epoch and remains present across ABA even if settled state equals the entry state;
neither internal observation is independently publishable. Operations following a pre-provider
stage use the settled state. Poll report debugging exposes only closed disposition classes and
receipt counts, never provider metadata, request identity, source revision, reply structure or
result cardinality.

The broker owns a separate composition report rather than depending on the semantic-service
crate. Its report binds the exact provider epoch/projection before the effect and maps each atomic
service transition stage to one exact-successor epoch carrying observed and settled projections.
The broker captures the canonical service-live request set when emitting the effect, validates the
whole ordered report against a shadow set, and only then commits exact receipt identities. One
global affine service-effect lane prevents another membership, retirement drive, policy event or
provider drive from changing that expectation. Any mismatch latches fatal divergence;
the actor reports outcome unknown, fails, reconciles admitted uncertainty and returns the exact
broker and port only through its must-use recovery capability.
Composition must initialize the moved effect port and broker from the same epoch/projection owner.
Independent provider-state publishers intentionally cause the next report to diverge.

Provider transition is one ordered coalgebra value containing runtime-owned provenance, readiness,
capabilities, a monotone composition epoch and the same invalidation receipt. It is installed before
any later ordinary delivery. Bridge construction must consume a fresh sovereign runtime type state
that proves one client, one provider poll per drive and no prior snapshots, pending work,
deliveries, quarantine or binding. Public asynchronous client futures are not assumed Ready.
After actor termination, a separate must-use recovery owner nonblockingly reclaims the exact broker
and effect port after the worker has actually exited. Clean stop, owner loss, provider failure,
fatal divergence and caught provider panic all preserve this cleanup authority. Reclamation is not
client or provider retirement: the bridge client must retire, checked provider extraction must
prove that no client, document, pending request or queued delivery remains, and the resulting
retirement-only provider capability still requires typed graceful or forced terminal evidence.
Bounded terminal correlation may cross extraction only coupled to that provider and forbids provider
reuse. Dropping any of these values is not retirement evidence.

Active stop is also nonblocking. A consuming admission is legal only from `Draining`, atomically
fences later commands as `Stopping`, and returns one affine observation ticket. Polling preserves
the ticket while pending. Completion is a closed sum: clean stop; refusal plus the exact owner
restored in `Draining`; or outcome unknown when the acknowledgement channel disconnects. Explicit
or implicit loss of a live ticket increments abandonment evidence and never implies termination.
The independent recovery owner is still required to establish worker termination and recover the
runtime. The managed ordered composition binds owner and recovery at spawn time, so evidence from
different actors cannot be paired. Refusal restores the exact managed product with bounded history;
Clean, OutcomeUnknown, Abandoned and FailedBeforeStop witnesses advance nonblockingly to recovery.
Clean requires clean recovery and FailedBeforeStop requires failed recovery. The joined witness is
retained by normalized cleanup and its terminal provider/process receipt; an impossible mismatch
still retains exact cleanup authority and an explicit mismatch disposition. Deadline-enabled actors
use the same product, retaining scheduler authority until exact retirement or explicit abandonment.

The local acceptance tier includes a compiled standalone LSP process over the qualified supervisor
and framed stdio path. It qualifies bootstrap projection plumbing, ordered membership and
synchronization, nonempty diagnostics, nested symbols, versioned rename plans, graceful shutdown,
and a post-ready transport failure with terminal cleanup. Its readiness source and semantic replies
are deterministic fixture evidence, so they do not establish a production language server's index
state or semantic quality.

The LSP adapter accepts a typed construction profile joining provenance, framing bounds, semantic
bounds and a closed initialization policy. A pinned protocol profile additionally binds a nonzero
profile digest, exact expected `serverInfo`, and one audited readiness-notification method. Missing
or mismatched identity fails startup. Fixed capabilities advertise workspace folders and versioned
`documentChanges`; only a validated closed health × quiescent notification reaches the readiness
observer. Notification-bound profiles cannot be promoted by independent polling. Responses,
notifications and unsupported inbound server requests are disjoint, and generic initialize success
remains Indexing. The supervised composition byte-encodes and binds the exact qualified launch
workspace as its root before process effects. Terminal retirement captures content-safe protocol
evidence and aggregate LSP usage from that exact owned adapter, so another instance's observations
cannot be paired with its process cleanup.

The first real-provider qualification uses an explicit rust-analyzer standalone initialization
variant rather than changing generic pinned LSP behavior. The variant binds canonical
`src/lib.rs` before launch, serializes its absolute filesystem path in `linkedProjects`, and uses
the corresponding injective `file://` URI for document synchronization. Its exact closed options
disable Cargo discovery, build scripts, proc macros, sysroot discovery and check-on-save. The
pinned 2026-11-09 macOS ARM64 run derives Current from validated server-status notifications,
completes diagnostics, nested symbols and a versioned two-edit rename, and joins clean actor stop,
same-spawn recovery, deadline retirement, checked extraction and graceful process termination.
The receipt explicitly records zero inbound server requests, malformed messages, orphan messages
and saturation. It also records that filesystem, network, subprocess and abrupt-parent confinement
are unsupported; qualification of those deployment properties remains separate.

If normal client retirement cannot close, forced bridge abandonment is a separate consuming sum,
never a relabeled graceful result. It records exact live-state counts and bounded cancellation/close
attempt, skip, refusal, unavailability and unknown evidence. A provider panic is caught, marks the
effect unknown and prevents every later provider call. The forced-only provider parts are consumed
by a retirement driver that joins abandonment with adapter, transport and process terminal proof;
its terminal result is only ForcedConfirmed or CleanupFailed. The ordinary path may additionally
produce GracefulConfirmed, including a genuine graceful close observed before an abort effect.

Actor provider polling is fallible. A physical-poll failure is itself an ordered evidence product:
provider-before and any lawful pre-provider transition are validated and committed before the actor
fails, while no post-provider observation is claimed. Mapper failure before such a product closes
is separately fatal and preserves the recovered cleanup port. The LSP adapter preserves cancelled
correlation in a bounded tombstone set so a consumed late response remains an exact completion
rather than fabricated
absence; cancel-send and transport failures latch closed failure classes. A content-safe poison,
runtime-unavailable or delivery-refused result transitions the actor to Failed, acknowledges the
admitted drive as `OutcomeUnknown`, reconciles queued/in-flight uncertainty and preserves its port
for recovery. Only a successful no-event disposition means one completed physical poll with no
event.

Wire decoding rejects unknown or duplicate/reordered fields, unknown tags, future versions,
malformed identities and UTF-8, trailing/truncated bytes and every declared limit violation before
allocation. Accepted frames re-encode to identical bytes. Golden command and reply fixtures are
part of the repository compatibility baseline.

Privacy rule: source, language, document, manifest digest/count/size, incarnation, membership,
exchange, operation, request and provider fingerprint must never become logs, error text or metric
labels. Only bounded aggregate counters and closed reason enums are suitable telemetry dimensions.
