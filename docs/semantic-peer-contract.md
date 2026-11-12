# Semantic peer contract

Status: bounded host contract, provider-neutral synchronization/routing,
partition-bound asynchronous client facade, no-process-authority LSP adapter,
qualified local process supervisor, embedded local composition facade, exact
deadline scheduler, staged provider replacement, bounded MCP projection and
stdio driver, and background-session protocol/client/broker/actor substrate are
implemented. The semantic MCP executable has both an explicit unavailable
qualification composition and an authenticated local rust-analyzer composition
that serves bounded startup workspace snapshots. No remote network semantic
transport, PSI, nREPL, automatic provider restart policy or post-start file
watching is claimed.

Vellum's useful semantic surface is a peer capability, not an editor wrapper.
`vellum_host::semantic` defines the outbound contract that a trusted embedded
composition can place beside Vellum. The existing `vellum-sdk-embedded` facade
already exposes it through its `host` re-export. Remote and agent-facing
projections must remain sibling clients of Vellum and the semantic service;
they must not route language authority through the editor engine.

`vellum-semantic-service` now owns the sovereign in-memory synchronization and
routing state. `vellum-semantic-client` is its zero-authority facade. They are
categorically separate from the native Vellum service/client pair: adding this
substrate does not add a Vellum wire command or make semantics editor state.

The V1 query algebra contains nine operations:

- bounded symbol search;
- definition, references and implementations at a byte offset;
- incoming and outgoing calls for the exact symbol prepared at a byte offset;
- diagnostics;
- type-at-point;
- rename planning.

Rename returns bounded text edits as a proposal only. It cannot apply an edit,
reserve history, claim atomicity or carry Vellum edit authority. A composer
that accepts a plan must obtain fresh Vellum witnesses, authorize every target
and select a separately documented multi-document transaction policy.

`vellum-semantic-apply-service` supplies the first such trusted policy. It binds
an admitted semantic snapshot to one exact presentation-free workspace
document only when revision and source bytes agree, preserves client partition
ownership, rechecks current provider provenance/generation and every source at
application time, obtains one exact workspace `EditSource` decision, and
derives expected replacement bytes from the fresh workspace snapshots. It then
publishes all documents through the existing single prepare/commit and linked
history group. The receipt always says persistence `NotAttempted`; filesystem
bytes are outside this transaction and no cross-file disk atomicity is claimed.

## Binding and ownership

Every request binds an opaque fixed-width request reference (a correlation fact,
not a numeric ordinal, credential or grant), opaque document reference,
Vellum-owned source revision (where zero is the valid initial revision),
optional peer-owned index generation, bounded language label and one closed
query. A complete response must echo the exact request and source target,
satisfy the index-generation precondition, and return the result variant
belonging to that query. The host gate rejects a transplanted or malformed
response. `Stale` discloses neither the peer's current generation nor a current
source revision.

Locations use half-open byte spans. An empty span is deliberately valid for a
point location or insertion proposal; reversed spans are unconstructable.

## Exact snapshot synchronization

The service admits a bounded, whole immutable UTF-8 snapshot for open/change
and an exact target for close. A change must name the same opaque document and
a strictly newer Vellum source revision. Equal, older, wrong-owner and unknown
document updates fail closed. A provider accepts the update before the service
publishes it; unavailable/refused synchronization leaves the prior snapshot
current. This is deliberately not an incremental text-delta, filesystem-watch,
buffer, CRDT or LSP `didChange` claim.

Newer accepted source revisions cancel requests for the older snapshot and
deliver `Stale`. A newly published peer index generation likewise cancels
requests bound to the old generation. Provider completions carry their exact
request reference and may arrive out of order; the service validates request,
target, index generation, result family and output ceiling before routing them
only to the admitted client partition that submitted them. Cancellation is
bounded and late provider completions become content-free orphan counts.

Provider readiness is a closed `Unavailable | Indexing | Ready(generation)`
state paired with a finite capability set. It is read from the privately owned
provider rather than published beside it; a real transition invalidates pending
work once and an equal observation is inert. It is not a health probe or
provenance attestation. A submit may be accepted immediately before a provider
transition, in which case the service can immediately deliver `Stale`. The
provider port is injected;
neither semantic crate discovers an executable, reads project configuration,
opens a socket, starts a process or chooses workspace trust. Client references
partition state but are not credentials, grants or proof of a principal.

Rune/Dendrite or another language-semantic peer remains sovereign over index
and language meaning. Vellum owns source/history state and ordinary mutations.
The trusted composition root owns authentication, authorization, disclosure
choice, clocks, deadlines and transport/process selection. The semantic port
adds none of those authorities.

The finite capability lattice separates index reads from rename planning.
Runtime attenuation is meet; join is reserved for trusted composition. Even
the rename capability authorizes only asking for a plan.

## LSP adapter baseline

`vellum-adapter-semantic-lsp` owns bounded `Content-Length` framing, strict
CRLF header parsing, JSON-RPC correlation, negotiated UTF-8/UTF-16/UTF-32
position conversion, virtual-document synchronization, and conversion of full
pull-diagnostic reports, hierarchical document symbols and strict rename plans.
Symbol search is a
case-sensitive filter over one exact document; the adapter intentionally does
not issue workspace-wide symbol queries.

The adapter receives an already-owned nonblocking byte transport and a
non-reusing index-generation issuer. It cannot discover or launch an
executable, choose arguments, read workspace configuration, open a network
connection, or apply an edit. Virtual URIs are path-free and injective across
registered documents. The standalone rust-analyzer profile instead binds every
document to a distinct workspace-confined file URI; the first ordered startup
member is `src/lib.rs`, the fixed linked-project root, and later members receive
injective generated paths beneath `src/`. Initialization grants `ReadIndex` only when the server
statically advertises both pull diagnostics and document symbols, but
initialization alone remains `Indexing`. Only a privately owned,
server-profile-specific readiness port may promote the adapter to `Ready`; each
promotion after reindexing receives a fresh non-reused generation. Unsupported
queries remain explicit.

Static `renameProvider` support contributes `PlanRename` only when read-index
support is also present. The adapter emits exact `textDocument/rename` requests
and accepts only versioned, text-only `documentChanges` for currently admitted
virtual documents. It rejects changes maps, resource operations, annotations,
unknown or stale versions, foreign URIs, invalid UTF positions and overlapping
or same-start edits. An LSP `null` result is represented as an empty proposal;
it grants no edit authority.

Call hierarchy is a bounded two-step LSP operation. The adapter first sends
`textDocument/prepareCallHierarchy`, requires zero or one valid item bound to a
currently admitted document, and passes that exact provider-owned item,
including bounded opaque `data`, to `callHierarchy/incomingCalls` or
`callHierarchy/outgoingCalls`. It never substitutes symbol text. Multiple
prepare items refuse as ambiguous; item bytes, result count, ranges, URIs and
snapshots are bounded and checked. Results project only caller/callee selection
locations, while the ordinary reply retains exact source revision, index
generation and composition-installed provider provenance.

Requests use bounded non-reused numeric LSP correlations internally while the
public request identity remains opaque. Results bind the source target and the
generation captured at submission. Cancellation removes correlation before a
best-effort `$/cancelRequest`; late replies are orphans. Full diagnostic reports
and hierarchical symbols are converted without truncation. Invalid envelopes,
ranges, severities, symbol kinds, containment, response families, or UTF
positions fail closed. Output, traversal depth, pending work, frames, documents,
and aggregate retained source bytes are bounded.

Provider identity, profile and version are installed by trusted composition,
bounded, path-free and carried in provider state and complete replies. They are
never learned from an LSP message. The host and semantic service reject
provenance-transplanted replies, and a provenance transition invalidates
pending work exactly once.

Graceful protocol shutdown is explicit and send-before-state. It is busy while
documents, requests or completions remain live; otherwise it sends `shutdown`,
accepts only the exact correlated null result, sends `exit`, closes the byte
transport and records graceful closure. Malformed/mismatched replies fail, and
a separate `abort` transition records composition-owned deadline escalation.
Dropping the adapter is only emergency byte cleanup and is not graceful
evidence.

The semantic service can replace its privately owned provider through a
composition-only transition. A candidate must be ready with fresh provenance
and generation, accept deterministic replay of every retained exact snapshot,
and keep its projected state stable throughout. Failure returns that candidate
without changing the installed provider, snapshots, pending requests or
deliveries. Success swaps once, invalidates old pending work once through the
returned old provider, and returns that provider for composition-owned graceful
shutdown. Per-client delivery capacity is preflighted before candidate effects.

Successful replacement returns a bounded retirement product rather than a bare
old provider. It immediately attempts deterministic exact document closes,
retains every refused/unavailable close, and exposes one-attempt-per-target
retry plus explicit escalation extraction. Only an empty remaining plan permits
drained extraction; neither swap success nor process exit is called graceful
shutdown by itself.

Deadlines remain composition-owned. `expire(client, request)` consumes no clock;
it records exact external deadline evidence as `TimedOut`, distinct from
cancellation, and refuses pre-effect when the destination delivery queue is
full. A later provider completion for expired work is an observed orphan.

This is an LSP 3.17-compatible subset, not general LSP conformance. Dynamic
registration, push diagnostics, unchanged diagnostic reports, flat
`SymbolInformation`, related-document disclosure, incremental edits,
workspace symbols, automatic query retry, an automatic background clock driver,
automatic provider restart policy and provider sandboxing remain outside this
baseline.

## Local process supervisor substrate

`vellum-semantic-local` is the only current process-launch caller for language
semantics, and remains excluded from production release artifacts. A trusted
operator constructs its path-redacted profile
from a canonical absolute executable, fixed argv, cleared environment with an
explicit variable map, canonical effective-user-owned `0700` private cwd outside
the canonical workspace root, launch/grace/kill-and-reap deadlines, and bounded
command/stdout/stderr queues. Qualification rejects loader-affecting environment
names, group/world-writable executables and executable owners other than root or
the effective user. It hashes a bounded executable and binds its canonical path,
filesystem identity, cwd identity, provenance, argv, explicit environment,
deadlines and limits into opaque redacted artifact/profile identities. The
single-use qualification product is revalidated before worker admission and
again at the final process-launch boundary.

The supervisor clears ambient environment, installs `RLIMIT_CORE=0` and a
`077` umask on supported Unix hosts, uses a process group, ordinary
Rust-owner-drop cleanup, bounded stderr accounting, graceful stdin closure and
deadline escalation. Its fixed support evidence classifies byte/path and cwd
identity as observed-only, environment clearing and core-dump suppression as
enforced on the supported Unix profile, and filesystem/network/subprocess
confinement as unsupported. The compatible profile constructor also selects an
explicit unbounded physical-resource policy. A separate strict constructor can
require one whole-tree memory ceiling, process-count ceiling and CPU quota; the
three exact values are profile-bound, and qualification refuses before spawn
unless ProcessKit reports a whole-tree mechanism capable of all three axes.
Process-group creation is the final fail-closed enforcement gate and reports an
unenforceable policy rather than launching without it. A receipt for a
successful strict launch classifies the three axes independently as enforced;
qualification alone is not process-application or utilization evidence, and it
does not upgrade filesystem, network, subprocess or abrupt-parent confinement.
The pinned macOS rust-analyzer profile remains unbounded and reports all three
physical-resource enforcement axes as unsupported. While a provider is live,
the supervisor samples the selected ProcessKit backend every 250 milliseconds
and retains only fixed-cardinality success/failure, availability and high-water
observations for active units, CPU time, memory, I/O and peak units. Unsupported
backend fields remain absent rather than becoming zero; on the POSIX
process-group backend, an active unit is a live process group rather than an
exact descendant count. These observations are measurement, not enforcement or
confinement. Lifecycle observations separately distinguish accepted whole-group
hard kill from confirmed finish/reap of the supervised leader; they do not claim
that Vellum reaps descendants or that macOS abrupt parent death cleans up the
provider.

Production entry is a separate affine promotion, not a caller-selected launch
flag. Its closed requirement product contains executable and working-directory
identity, ambient-environment clearing, core-dump suppression, filesystem,
network and subprocess confinement, abrupt-parent cleanup, and whole-tree
memory, process-count and CPU enforcement. Every member must be `Enforced` to
mint the token accepted by the production launch entry points. Refusal retains
the exact ordinary qualified launch for explicit probe use and exposes only the
typed support disposition for each fixed axis. The current macOS profile cannot
be promoted: observed identity is not immutable launch, and ProcessKit does not
provide the required confinement, abrupt-parent cleanup or whole-tree ceilings
there.

Launch admission is serialized. If the bounded launch wait expires or cleanup
cannot be confirmed, the worker join handle remains owned behind a permanent
process-local fence. The call returns within the launch bound, no later
generation may overlap the unresolved generation, and production V1 has no
in-process reset or automatic retry; process restart is the only fence reset.

Live qualification uses a compiled hostile child to prove exact argv/cwd/env,
byte round trip, stderr saturation without stdout blockage, monotone close and
abort, bounded stdin backpressure, graceful EOF/reap, inherited-pipe cleanup,
nonzero/crash distinction, confirmed post-start refusal cleanup and owner-loss
cleanup. The crate does not read workspace configuration, use a shell or PATH
lookup, open a network connection or apply edits.

Its reusable embedded facade preflights compatible adapter/service limits,
launches the supervised byte transport, starts the adapter and privately owns
the semantic service. Trusted composition may admit a client, bind one
partition-only SDK handle, schedule same-clock exact pending-request expiry,
perform one bounded fair drive, stage a replay-validated provider replacement
and own the returned retirement obligation through graceful or forced cleanup.
Ordinary clients see only readiness, capabilities and their own deliveries;
provider provenance, clock evidence and lifecycle control do not cross the
facade. This supplies no clock source, authentication, background loop,
automatic restart policy, wire protocol or daemon. The current binary remains
only a content-safe probe.

`vellum-semantic-local` also exposes a single-flight restart controller and an
opt-in embedded composition that consumes it. The composition accepts one
bounded clock, follows the finite explicit backoff schedule, launches a fresh
provider, waits for independent readiness, and uses the semantic service's
replacement boundary to replay every exact workspace snapshot before the
single authoritative swap. It withholds its runtime surface until the
displaced provider's exact terminal cleanup receipt is reconciled. A failed or
budget-exhausted cleanup, uncertain clock, or outcome-unknown launch
permanently fuses that controller instance; retained cleanup owners are
returned rather than erased. Every restart fixes pending query disposition to
invalidation without replay. This composition is not installed in the binary
by default, has no reset route, and does not weaken the production-promotion
requirements above.

The authenticated ordered actor now has the corresponding replacement seam
without pretending that the embedded facade is its runtime. A command/control
queue-quiescent worker can be suspended into its exact broker, timed port,
snapshots, pending provider requests and deadline graph. Replacement replays
retained snapshots, publishes one ordered provider epoch, terminalizes pending
queries as stale without replay, clears their exact deadlines, and returns the
displaced provider affinely. Resumption preserves accumulated actor usage and
permanently fences every old sender. A physical-swap/broker-reconciliation
failure yields a non-resumable fused owner. Fresh real-process candidate launch
derives initialization from a separately qualified workspace and admits only a
fresh-started, pinned-ready provider. The installed runtime remains hidden until
the displaced provider has terminal cleanup evidence; candidate refusal retains
the exact refusal and cleanup authority, while the resumed current provider has
its own terminal cleanup path. The standalone probe binary exposes one explicit
composition: `--restart-on-sighup` classifies only SIGHUP as
`OperatorRequested`, waits until no authenticated stream is active, applies a
finite backoff and readiness deadline, requires terminal displaced cleanup, and
then resumes the same broker/session state with a fresh sender. It is disabled
by default, never replays queries, and exits permanently on uncertain progress.

This substrate does not prove an immutable executable, interpreter,
dynamic-library or plugin closure: spawning remains path-based and same-authority
replacement can race the last observation. It does not close inherited handles,
install a provider sandbox, or enforce physical CPU/RSS/cache/FD/process limits.
Artifact hashing and final-boundary revalidation are synchronous work outside the
configured asynchronous launch deadline, so that deadline is not a total
wall-clock bound over qualification. Those are release blockers before this
crate may enter a production background-service artifact. Environment names are
bounded and loader-affecting names are rejected, but the remainder is not a
closed semantic safe-name allowlist.

## Asynchronous client boundary

`vellum-semantic-client` defines a boxed `Send` future port suitable for an
embedded application or a later connector. Ordinary operations carry no client
identifier because the port is already bound to one composition-selected
partition; the sovereign service is accessible only through a private binding
adapter. A separate trusted runtime-driver port owns exact expiry and one
bounded pump. Client provider observation projects only readiness and
capabilities, never identity/profile/version. These interfaces select neither
authentication nor transport and do not constitute a remote SDK.

`vellum-semantic-session-client` separately implements the background-session
client ledger over an injected phase-aware exchange port. It permits no implicit
mutation replay: outcome-unknown mutations retain the exact command and can be
resent only through the explicit replay operation under a fresh nonzero exchange
correlation. The operation identity and semantic intent remain exact, while a
reply must bind the fresh exchange. `NotSent` during replay does not replace or
clear the last possibly-sent exchange. Operation-less Describe and Collect may proceed
while a mutation is unresolved, but their replies cannot resolve it. A changed
incarnation invalidates the volatile membership, request, witness and uncertainty
ledgers.

Join acceptance independently rechecks that the granted role product is
non-bottom and attenuated by both the requested roles and the last described
subject ceiling. Its remaining-operation budget must be nonzero and no larger
than the described membership limit. The client then consumes that budget
monotonically on each fresh definitive mutation operation; a matching replay is
inert and no later reply can increase the local budget.

Accepted synchronization is transactional. A Begin from Ready seeds a bounded
stage from the committed witness projection, fences queries, and admits only
one synchronization effect at a time. Open/Change/Close preflight projected
document count, aggregate source bytes and compact-witness retention before the
exchange. Only an accepted Complete with the exact compositional manifest root
atomically publishes the stage. Refusal discards the stage without publishing;
an unknown effect retains both the committed projection and the exact stage
behind the replay fence. Epoch identities are incarnation-global and
non-reusable, including across membership turnover. Retained committed
witnesses are never sufficient to accept a Complete query result while the
membership is unsealed, synchronizing, or provider-unready; the result remains
unknown and its reservation remains held.

The client retains no source copy afterward: each private,
redacted witness contains exact language, target/revision, byte length, canonical
UTF-8 boundary bitmap and the protocol's collision-resistant document
commitment. Commitments are not categorical source equality or authority.
Witness, exact replay-content and worst-case logical result-payload budgets are
independent and checked before effects. Complete replies revalidate exact
request/target/generation/result family, every location/edit against the current
witness projection, target-local diagnostics and globally nonoverlapping rename
edits. Provider provenance is structurally closed but is not preselected unless
composition supplies a separate expectation.

A query-only client is explicitly unsealed unless trusted composition installs a
same-incarnation, exact-ready-epoch witness store into a query-capable
membership. `vellum-semantic-client` now owns the only bridge from that
witness-validated session product into its private sovereign-validation seal.
The bridge rechecks the granted query role and validation state, exposes no
synchronization methods, retains transport-unknown Submit intent for identical
caller-initiated replay, and admits complete deliveries only after the session
client's nested source/span validation. It does not treat authentication,
membership, canonical decoding or opaque document identity as the seal. The
session client still implements no wire codec, socket, authentication, broker
storage or automatic query retry.

`vellum-semantic-transport` supplies the client-side local Unix byte exchange.
It lazily connects to one canonical private socket path, rechecks the
effective-user-owned socket and owner-private parent identity around connect,
applies separate connect/write/read deadlines, runs one application-installed
authentication prelude before any semantic byte, and exchanges bounded big-endian
length-prefixed frames on one persistent stream. Connect and pre-byte write
failure are `NotSent`; any lost or malformed reply after transmission is
`OutcomeUnknown` and permanently fences that connection object. A later call
does not reconnect a possibly live semantic membership. The observations are
fixed-cardinality logical I/O counters. Private path identity is not
authentication; the credential adapter establishes local attribution but no
confidentiality, physical-resource or latency evidence. Path identity remains
observed-only because pathname connect is not descriptor-anchored.

## Agent-facing MCP projection

`vellum-semantic-mcp` is a sibling of the native-daemon `vellum-mcp`, not an
extension of it. It is a bounded protocol projection over an already
partition-bound `vellum-semantic-client`, plus a sequential newline-delimited
stdio driver. The library is not a remote transport or authentication boundary;
the binary is an explicit composition root over the native SDK, semantic Unix
transport and local credential adapter. Its stable surface mirrors the seven
closed query forms, one explicit terminal-result release operation, and two
content-safe catalog resources. Provider indexing changes query outcomes;
committed native membership and source-revision changes replace the catalog at
the next request boundary. It never depends directly on the editor,
apply crate or LSP adapter. The library projection and stdio driver remain
service-neutral. Cancellation and composition-owned deadline
expiry retain their distinct outcomes. Rename application stays unadvertised
until a server-side durable plan handle and outcome-unknown-safe operation
receipt exist.

The workspace binary refuses a default invocation and requires explicit private
semantic/native socket and credential paths plus one registered language. Its
current real profile authenticates one persistent semantic connection, joins
with the synchronization/query role product, synchronizes exact public native
snapshots as one bounded uniform-language startup manifest, waits for
pinned-provider readiness, and publishes a locator-free multi-document catalog.
The native connection remains live while stdio is served. After each complete
frame arrives but before it is executed, the composition affinely pauses the
query port, drains the bounded workspace-change feed, performs exact
resynchronization and resealing after events or retention gaps, polls again to
close the synchronization race, and replaces the catalog only after
convergence. Eight refresh rounds and 4,096 feed pages are hard
ceilings; sustained mutation refuses closed. A transport-unknown query mutation
retains its exact replay path and temporarily prevents synchronization instead
of being retried under a new identity. Clean EOF explicitly retires retained
query results and the semantic membership, then disconnects the native client.
This is request-bound service-mutation synchronization, not idle background or
external filesystem watching.

Composition lifecycle/synchronization correlations occupy the lower half of
the 128-bit identity space and query correlations occupy the corresponding
upper-half partition; both start with at least one quarter-space of monotone
headroom. They therefore cannot collide while request-bound synchronization and
queries advance independently. Before executing an arrived frame, the peer
also reserves enough ordinary operations for one worst-case complete epoch plus
that frame. When the membership falls below this reserve, it uses the separate
retirement tail to leave, rejoins under a fresh membership, resets the local
synchronization coordinator, opens a complete current epoch, rebinds the MCP
catalog, and continues the same stdio lifetime. Successful rollovers are a
content-safe exit metric. If a fresh Join instead receives definitive global
`ReplayCapacity`, the peer completes Leave, extracts a sessionless affine MCP
owner, closes the retired transport, authenticates a fresh connection,
synchronizes and witness-validates a new session, and then rebinds that same MCP
owner and stdio lifetime. Pending collections tied to the retired membership
become exact replayable `cancelled` receipts. This path is never entered for
`NotSent`, `OutcomeUnknown`, an unresolved mutation, or a non-quiescent
session. Reconnect and rollover totals are fixed-cardinality, content-safe
metrics. Ordinary rollover does not compact the broker's historical global
join/membership replay ledger or rotate its service incarnation.

The explicit `--unavailable` mode admits one process-local partition to a real sovereign
service whose provider is permanently unavailable. It is useful for MCP host,
catalog, framing and failure-path qualification only: capability discovery says
`unavailable`, no snapshot can be admitted through MCP, and queries fail closed.
It must not be presented as a production semantic backend.

For an MCP-host framing or catalog qualification run:

```sh
cargo run --locked --offline -p vellum-semantic-mcp -- --unavailable
```

The host owns the child lifetime and sends one JSON-RPC object per newline on
stdin. `--metrics-stderr` adds exactly one content-safe JSON record after clean
EOF. `--probe` and `--help` do not enter MCP mode.

The stdio loop retains at most the configured request ceiling plus one byte,
drains an oversized frame before continuing, flushes every response, does not
respond to notifications, and treats a nonempty EOF fragment as truncated even
if the fragment is otherwise valid JSON. Clean EOF is the current graceful stop
boundary. Optional exit metrics are fixed-cardinality and content-free.

The persistent listener/dispatcher now authenticates once, derives the
synchronization/query role product independently, and owns the actor and pinned
provider runtime. Trusted synchronization input comes from composition-owned
Vellum snapshot products, not an MCP argument or workspace manifest. The
connector does not mint the seal, infer membership from document ids, or
automatically replay an uncertain mutation. MCP callers can explicitly release
terminal replay payloads; the broker retains a bounded non-reusable identity
tombstone. The semantic-local process observes SIGINT/SIGTERM, interrupts a
blocked authenticated connection, projects that connection's exact
subject/session/policy epoch as lost authority, drains fenced memberships,
joins actor recovery, retires retained client source, and drives the provider
through protocol shutdown with bounded forced escalation. Successful exit
reports only the closed provider terminal class. Signal receipt is host
lifecycle authority, not semantic query authority, and does not make a
same-user process boundary into principal authentication. The semantic MCP
source is now closed behind the zero-authority semantic and native SDK facades,
and its deterministic sibling extraction is qualified without publication.
Remaining synchronization work excludes automatic external-file merging:
native Vellum workspace state remains authoritative. Membership-local operation exhaustion rolls over
without consuming the broker/client retirement tail, and definitive global
replay exhaustion reconnects through a fresh service incarnation without
ending MCP stdio; historical global replay evidence is still never silently
deleted in place.

The same exact authority-loss projection now occurs on every authenticated
connection exit, not only signal-driven shutdown. The listener's idle loop
drives retirement between clients, preventing a disconnected or malformed peer
from indefinitely retaining membership capacity or synchronization ownership.
The content-safe admission/completion observation makes refusal explicit; it is
not evidence that provider cache or historical replay state was erased.

Before accepting the next authenticated client, the service then uses an OS-
CSPRNG identity and a trusted actor control to rotate the broker only after the
old incarnation is quiescent and the provider port reports no cleanup
obligations. Rotation atomically drops the fully retired replay domain, resets
broker-local session identities, and preserves the current provider epoch and
projection. Same-incarnation, active-membership, queued-command and external-
obligation cases refuse without partial compaction. Actor usage records
content-safe rotation/refusal totals. The MCP composition now closes a
definitively retired exhausted connection so this quiescent rotation can occur,
then binds its sessionless owner only after the fresh session is fully
synchronized and witness-validated.

Construction requires the semantic client's sealed sovereign-validation port,
not merely the public generic service-port shape. This preserves the service's
proof that every nested location belongs to the caller partition and exact open
source revision, that spans are in bounds and on UTF-8 boundaries, and that
rename ranges are non-ambiguous. A later remote adapter must reproduce those
checks before it can mint equivalent evidence; the MCP projection does not
duplicate or guess unavailable workspace authority.

The capability resource deliberately omits provider provenance. A successful
query result deliberately includes the bounded opaque provider id, profile and
version as a semantic-behavior fingerprint. Deployments that treat those fields
as sensitive must attenuate the result at a separate policy boundary; they are
correlation metadata, not authentication evidence.

## Bounds and observation

Text, result lists, rename edits, request lifetime, per-snapshot bytes and
aggregate retained snapshot bytes have hard V1 ceilings; deployment limits may
only narrow them. Both constructed values and provider output are checked at
the host gate. The host gate and service retain only
saturating, fixed-cardinality counters for requests, completions, unsupported,
stale, unavailable, refused, cancellation/invalidation/timeouts, snapshots,
provider replacement/replay and orphaned completion totals. Debug output redacts queries,
names, diagnostic text, type text and replacement text.

These counters cover bounded RED request/outcome volume and logical USE
capacity, including current/high-water snapshot bytes. Supervised local-process
usage additionally reports content-free ProcessKit resource samples with
per-axis observation counts, so an unavailable axis cannot masquerade as a zero
measurement. The samples do not provide request latency, queueing, provider
health or cross-platform-comparable CPU/RSS semantics, and a strict launch
policy remains enforcement configuration rather than measurement. Applications
that claim complete golden-signal coverage must add a caller-supplied clock and
health model without placing source, paths, symbols, identities or language
labels in metric dimensions.

## Closed failure vocabulary

`Unsupported`, `Unavailable` and `Stale` are distinct outcomes. Refusals
separate authority, input limit, output limit, invalid request and malformed
provider reply. None implies a retry is safe, an index was refreshed, or an
edit occurred. This crate deliberately supplies no automatic retry or refresh.

## Security and privacy

The peer is a separate disclosure domain. Opaque identifiers do not anonymize
source or diagnostic meaning. A real adapter must explicitly address peer
provenance, project configuration/plugin execution, workspace isolation,
cancellation, bounded stderr/output, index retention/deletion and transport
confidentiality. Runtime evaluation is outside this contract and requires an
independent execution authority and sandbox profile.

Local signal supervision prevents an idle socket read from orphaning the
provider, but it does not authenticate the sender of an OS signal, survive
SIGKILL, prove provider-cache erasure, or provide a remote shutdown protocol.
Those remain separate deployment and privacy obligations.

Selected executable evidence covers construction ceilings, capability
attenuation, stale outcomes, request/revision/index transplant rejection,
provider over-output, exact snapshot ordering, provider unavailability,
unsupported operations, cancellation, out-of-order completion routing,
cross-client isolation, aggregate snapshot capacity, LSP framing, virtual-URI
injectivity, exact position conversion, diagnostic/symbol correlation and
content-safe observation. It is not general LSP conformance, semantic
correctness, process qualification or whole-system STRIDE/LINDDUN certification.
