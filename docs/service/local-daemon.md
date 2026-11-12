# Vellum local daemon

Status: bounded macOS Unix-socket profile implemented 2026-11-09; unpublished.

`vellum-daemon` is the standalone composition root for one serialized Vellum
service. It selects a shared Workspace, bounded multi-client service limits,
canonical V1 wire encoding, and one-request Unix-socket exchanges. The default
service ceiling is read-only. The qualified local-filesystem discovery adapter
is installed only when its corresponding action ceiling is explicitly selected.
No semantic crate depends on the daemon or concrete adapter.

The installed binary enables durable workspace-scope reservation in the same
private directory. Before binding the data socket it loads the bounded identity
record, constructs the service, materializes the startup inventory, and commits
the next high-water reservation and exact startup locator bindings. Restart
preserves the workspace scope and reuses a document/root ID only for its
validated exact locator; it never infers a binding from inventory order. See
[workspace identity recovery](../workspace-identity-recovery.md).

Run from the Vellum checkout:

```sh
cargo run --locked -p vellum-daemon -- \
  --socket PRIVATE_DIR/vellum.sock \
  --control PRIVATE_DIR/vellum-control.sock \
  [--allow-add-file] [--allow-add-directory] [--allow-remove] \
  [--allow-edit] [--allow-save] \
  [--] \
  [EXISTING_FILE_OR_DIRECTORY...]

cargo run --locked -p vellum-daemon -- \
  --shutdown PRIVATE_DIR/vellum-control.sock

cargo run --locked -p vellum-daemon -- \
  --status PRIVATE_DIR/vellum-control.sock
```

The fixed action flags form an explicit trusted-local ceiling over the
read-only base (`InspectWorkspace`, `ReadSource`, and `SelectDocument`):

- `--allow-edit` adds `EditSource`.
- `--allow-save` adds `PersistSource`.
- `--allow-add-file` adds `AdmitFile`.
- `--allow-add-directory` adds `DiscoverDirectory` and installs the bounded
  descriptor-anchored discovery adapter.
- `--allow-remove` adds `RetireMembership` for document/root retirement.
- `--allow-retire-receipt` adds only `RetireReceipt`; fresh authentication is
  still independently required.

Omitting both flags is read-only. Each flag is independent; for example,
save authority does not silently imply edit or admission authority. The `--`
separator permits a workspace root whose name begins with `--`. These are
static ceilings for the local single-OS-user profile, not authentication,
delegation, a bearer grant, or an Aegis policy decision.

The default `OwnerOnlyOffline` invocation cannot satisfy that fresh
authentication requirement. Receipt retirement becomes reachable only when
the daemon is also started with an explicit credential path; authentication
and the `--allow-retire-receipt` action ceiling remain conjunctive.

Durable receipt retirement retains a permanent operation tombstone under a
separate bounded monotone capacity. Active receipt capacity releases only
after validated active-file cleanup and directory synchronization following
tombstone commit. The passive in-process snapshot exposes both gauges;
the byte-compatible V1 external control-status frame does not yet project the
new tombstone gauge.

The persistent standalone composition retains its identity metadata port for
runtime membership. The v3 record separates exact document/root issued prefixes
from reservation ceilings; recovery allocates at issued plus one, and removal
never lowers the issued marks. Any fresh accepted command which changes the
identity generation or either issued prefix, including inventory projection,
must store the complete next binding generation before writing its accepted
wire reply. Exact replay does not repeat the checkpoint.
A refused or indeterminate store sends no semantic reply and fences the daemon,
so the caller observes `OutcomeUnknown` and no later request can use the
unconfirmed live state. No rollback is claimed. On restart, exact active
locators must match whichever complete generation the adapter proves, while
retired ordinals remain reserved and are never reused. The explicitly
transient embedded `LocalDaemon::bind_with_action_ceiling` retains runtime
membership semantics without making this restart-stability claim.

No workspace roots is valid and starts an empty service. Explicit regular files
are opened directly. Explicit directories are expanded through the same
bounded, descriptor-anchored `LocalDirectoryDiscovery` used by the native
`AddDirectory` command. Startup inspects the root without following a symbolic
link, then the selected file-open or directory-discovery path revalidates it;
symbolic links and special resources refuse the complete startup composition.
The published workspace inventory contains only scoped opaque identities and
root kinds, never these filesystem locators.

The socket parent must already exist, must be a real directory rather than a
symlink, and must grant no group or other permission bits. After binding, the
daemon sets mode `0600`. Clean Rust shutdown removes only the same socket
inode/device created by the daemon; replacement paths are not removed.

The standalone persistent profile records a bounded exact path plus socket
device/inode for both endpoints in the already locked `0600` ownership file.
After a crash, startup removes an endpoint only when it has the exclusive
persistent lock, the complete record decodes, path/type/euid/mode/link count
and identity all match, a connection fails specifically as refused, and a
second ownership/identity check still matches immediately before removal.
Missing, truncated, extended, conflicting, live, replaced, or otherwise
unproven endpoints fail closed without unlinking. Updating the data record
preserves the prior control record until control recovery completes. The lock
file is synced after each record update and the parent directory is synced
after recovery removal. A crash during in-place record update may leave a
malformed record and require operator intervention; no repair is inferred.
Transient library composition has no predecessor evidence and continues to
refuse every existing path.

The persistent ownership lock is also first-install evidence. A missing
identity record is accepted only when this startup exclusively created that
lock. If the lock already existed, missing, malformed, conflicting, unsafe, or
outcome-unknown identity state refuses before socket publication. The daemon
does not delete, repair, or regenerate either identity evidence. Embedded users
may still choose the explicitly transient `LocalDaemon::bind` constructor;
the standalone CLI uses `bind_with_persistent_identity_and_action_ceiling`.

The explicit default authentication profile is `OwnerOnlyOffline`: endpoint
ownership and reachability are enforced, but no human, agent, or process
principal is inferred. Embedded standalone compositions may instead call
`LocalDaemon::require_authentication` with the existing transport-neutral host
port and its adapter-owned context. That selects fixed-size local handshake
protocol V1 before the ordinary Vellum V1 frame. Exact evidence is required for each
connection and retained only through its one request; rejection, provider
unavailability, replay, and malformed or transplanted evidence are externally
coalesced and fail before service decoding. The authenticated context grants no
service action and is never substituted for the independent authority lattice.
One safe OS-CSPRNG daemon incarnation plus a domain-separated monotone ordinal
prevents reference reuse across connections and fresh restarts; randomness or
ordinal exhaustion fails closed without a metadata/PID/time fallback.
The CLI remains owner-only offline unless `--credential PRIVATE_CREDENTIAL` is
explicitly selected. With that option it loads the descriptor-anchored local
credential before workspace discovery or socket publication and requires the
fixed-size handshake for every exchange. Its lifetime authentication-attempt
budget is a dedicated finite daemon limit, categorically separate from the
authority-decision bound; exhaustion fails closed. Authentication still grants
no action. Operators can create a
new exclusive record with `--provision-credential PRIVATE_CREDENTIAL` or stage
a same-subject, next-generation record with
`--stage-credential-rotation CURRENT_PRIVATE_CREDENTIAL NEXT_PRIVATE_CREDENTIAL`.
These commands accept paths only and never print credential fields or paths.
See [authentication and session establishment](../authentication-seam.md).

The persistent composition also installs a bounded local request journal. It
is inert under `OwnerOnlyOffline`; only a required-authenticated subject may
create or retrieve an effectful operation receipt. Intent is synchronized
before service execution, membership identity is checkpointed before receipt,
and the exact reply is synchronized before transport publication. Any pending
or indeterminate record fences/refuses rather than retrying an effect. The
plaintext/source-bearing format, non-reassigned client ordinals, no-eviction
capacity, subject non-disclosure rule, and generation compatibility are
specified in [durable-request-replay.md](durable-request-replay.md).

The required-auth composition may additionally install one exact
create-exclusive export policy before endpoint publication. Its canonical
`vdo1` operation, document ordinal, revision and absolute UTF-8 destination
must all match the durable intent. The daemon consumes the fresh authenticated
session, joins it with the operation-derived authority resource, installs the
pinned local destination object only after that exact match, and executes the
real `RENAME_EXCL` effect. Owner-only operation and unmatched intents remain
closed. Recovery uses the host canonical validator rather than a daemon-local
approximation; a future-generation, cross-sort, wrong-document/revision or
noncanonical record refuses startup before socket publication.

Lost-reply qualification discards a completed export response, restarts the
daemon and retrieves the synchronized historical receipt without a second
effect or publication. Pending export remains `OutcomeUnknown`. The public
receipt and operational evidence omit destination and source, while the exact
intent and permanent retirement tombstone necessarily retain the destination
in private plaintext storage.

Handshake V1 is plaintext local transport coordination, not confidentiality,
mutual isolation from a claim-observing process, credential design, remote peer
authentication, or propagation into operation authority.

After any required handshake, every connection carries exactly one
unsigned-32-bit-length-prefixed V1 frame and receives at most one reply. Frame,
text, field, request, witness, client,
and lifetime-client populations are bounded. Read and write timeouts bound a
connected peer that stops making progress. Malformed requests close only their
connection and cannot become semantic refusals.

Clients explicitly disconnect. Disconnect is idempotent through bounded retired
client tombstones and releases that client's retained requests and witnesses.
Lost disconnect delivery remains an unavailable transport outcome; it is not
reported as successful cleanup. The daemon's client-lifetime ceiling prevents
unbounded reconnect identity growth.

## Operational evidence

The standalone process installs the closed, content-free operational event
boundary described in [operational events](operational-events.md). Embedded
compositions may supply another sink. Event delivery is passive: failures and
unwinds are contained and counted and never alter authority, replay, request,
or lifecycle outcomes.

Daemon counters report accepted, completed, accept-failed, request-failed, and
deadline-exhausted I/O connections with sticky saturation. A
deadline-exhausted count means an accepted exchange encountered
an OS `TimedOut` or `WouldBlock` read/write result under the configured daemon
I/O timeout; the transport continues to expose only its closed `Io` class. The
counter does not distinguish read from write. Required-authentication evidence
adds only aggregate handshake started/completed/refused/I/O-failed populations,
issued/closed/live session gauges, and identity/counter saturation. Reasons and
opaque references never become metric labels or operational events.
Transport counters separately report
exchange completion and I/O, limit, malformed, or service-access failures.
Service counters retain semantic accepted/refused/replay/conflict/stale/limit
populations and live/issued/retired client utilization. None of these counters
authorizes an edit or proves delivery, durability, authentication, or cleanup.

The daemon library also exposes a content-free operational probe. Its closed
lifecycle is `Running -> Draining -> Stopped`; loss of the pinned socket,
directory, or ownership lock fences any phase. Health reports only healthy or
fenced, readiness is true only while running below the configured in-flight
bound, and the status carries bounded in-flight utilization without paths,
client identities, or document content.

Mechanical connection obligations use Keystone's bounded
`ObligationLedger`: Open maps to Running, Draining maps to Draining, and Closed
maps to Stopped. Vellum's Fenced state is deliberately orthogonal because it
means endpoint ownership was lost, not that Keystone discharged or closed the
ledger. The reference daemon is serialized at exactly one obligation, so a
single reusable private obligation token is injective over the live set. Drain
forbids later registration, unresolved close remains Pending without deleting
the obligation, exact discharge permits close, and unknown discharge remains a
Vellum lifecycle-invariant failure. Keystone does not own readiness, health,
refusal mapping, counters, endpoint verification, or shutdown control.

`operational_snapshot` is the passive, atomic-in-process observation contract.
One shared borrow projects lifecycle/readiness, daemon counters, service
counters, fixed-cardinality utilization gauges, closed saturation flags, and
content-free identity-startup counters/disposition.
It contains no paths, socket or client IDs, source, locators, request values,
document names, or dynamically chosen labels. Sampling neither verifies
endpoint ownership nor changes counters, service state, admission, or lifecycle
state; ownership is revalidated by admission and lifecycle transition methods.

The signal mapping is exact:

- USE utilization is in-flight connections, active and lifetime-issued
  clients, retained requests, retained witnesses, and each corresponding
  capacity. Retired clients are an additional bounded lifetime gauge.
- USE saturation is a closed boolean for each resource population at capacity,
  plus separate sticky daemon-counter and service-counter saturation and their
  aggregate. Required authentication adds a fixed one-live-session capacity and
  sticky identity-exhaustion saturation.
- USE errors are daemon accept/request failures and the service's closed
  refusal/error-category counters.
- RED request/traffic evidence is daemon accepted/completed connections and
  service request/accepted/refused counters. Error evidence is the same closed
  failure/refusal set plus coalesced authentication refusals and handshake I/O
  failures. Duration is the optional fixed-bucket distribution of
  actual admitted request exchanges described below.
- Golden-signal traffic is the RED traffic evidence; errors are the USE/RED
  error evidence; saturation is the explicit closed saturation projection.
  Latency reuses the same admitted-request distribution.

The reference daemon enables a Keystone-owned five-bound duration histogram
with closed upper bounds at 1 ms, 5 ms, 25 ms, 100 ms, and 1 s, plus an
overflow bucket. Timing starts only after the data connection is accepted and
admitted, immediately before timeout setup, any required authentication, and
the native request/service exchange; it stops when that exchange returns. Time blocked in `accept`,
startup, shutdown control, and drain completion is excluded. Successes,
semantic refusals, malformed exchanges, I/O errors, and timeouts are all in the
admitted population; pre-admission capacity, readiness, ownership, and shutdown
refusals are not. The snapshot carries bounds, per-bucket and overflow counts,
total count, duration sum, and sticky fidelity-loss evidence. Observation can
be disabled with `request_latency: None`; disabling or saturating it cannot
change admission, service outcomes, counters, or lifecycle transitions.

Keystone supplies only the bounded histogram algebra. It reads no clock and
exports nothing. The daemon selects `Instant`, owns this timing scope, and
passively records the result after the exchange. Histogram arithmetic
saturates rather than wrapping; after fidelity loss, the retained snapshot is
frozen and explicitly marked rather than claiming later samples were counted.

The snapshot is diagnostic evidence, not an authorization, audit record,
health-check side effect, linearizable cross-process metrics transaction, SLO,
end-to-end or user-visible latency measurement, or proof of endpoint ownership. It introduces no metric
label registry or exporter. The fixed histogram is a shared Keystone primitive;
its clock boundary and signal meaning remain daemon-owned.

The installed binary exposes one separate owner-only control socket in the same
pinned private directory. Its bounded local protocol preserves the original
one-byte V1 shutdown request and adds the two-byte status request `[2, 1]`
(operation, version). Status returns one fixed-size V1 reply with liveness,
lifecycle, health, readiness, the same aggregate utilization gauges described
above, and their closed saturation predicates. It contains no paths, source,
subjects, sessions, client/request/document/root IDs, arguments, per-user
counts, dynamic labels, raw errors, or debug rendering. The shipped
`vellum-daemon --status PRIVATE_CONTROL_SOCKET` renders that closed reply as
one stable JSON object.

Status sampling copies the last projection published by the serialized daemon
at admission, request completion, fencing, and drain transitions. It does not
call the ownership probe, increment `operational_probes`, revalidate authority,
or mutate readiness. Thus a successful reply proves only that the private
control process is live and reports its current published state; it is not a
principal-authentication, authorization, dependency-health, or end-to-end SLO
claim. The control client validates socket type, effective-user ownership,
exact `0600` mode, single link, and stable device/inode across connect. This is
still same-OS-user isolation, not cryptographic peer authentication: another
process under that account can impersonate the endpoint.

Malformed operations and versions are refused without changing lifecycle, and
slow/partial clients are discarded at the daemon's existing exact control I/O
timeout. Status replies leave the control loop available for later status or
shutdown requests. A valid shutdown request sets a monotone shutdown flag and
makes a wake connection to the data socket.
The accept path checks the flag before admitting that connection, so the wake
is not service traffic. It then closes admission, revalidates ownership, drains
all admitted obligations, and prints `vellum-daemon stopped` only after the
closed lifecycle reaches `Stopped`. The shutdown client acknowledgment means
the request and wake were accepted, not that quiescence has already completed;
the supervising process must also observe successful daemon exit. Existing
drain counters and the stopped lifecycle snapshot are the content-safe
deterministic evidence.

The reference driver is deliberately serialized and has no asynchronous
cancellation runtime. Once a connection is admitted, shutdown cannot preempt
an in-progress read, service dispatch, or write: the configured read/write
timeouts bound a peer that stops making I/O progress, after which the
connection obligation is discharged and shutdown can begin draining. The OS
may apply timeout granularity, so the deadline is a lower-bound classification,
not a wall-clock upper-bound promise. A peer that continuously makes progress
is bounded by frame size and finite service work, not by a total exchange
deadline. Capacity is exactly one admitted obligation; readiness becomes false
while it is held, a second lifecycle admission is refused as capacity, and a
drain with that obligation remains pending. Because the public driver owns one
mutable serialized loop, it cannot accept that second socket concurrently;
kernel listen-backlog occupancy is neither admission nor daemon capacity
evidence.

The standalone process installs safe `SIGINT`/`SIGTERM` observation and routes
the first notification through the same owner-only control socket. It neither
performs filesystem work in a signal handler nor bypasses endpoint ownership,
the admission fence, wake, or drain algebra. Later notifications are replay
evidence. Observed, successfully selected, replayed, and failed signal-shutdown
populations are separate saturating counters. `SIGKILL`, abort, host failure,
and power loss still bypass Rust destructors; the exact predecessor evidence
above is the only automatic crash-residue recovery path.

For launchd, run the binary directly (not through a shell), pass absolute data
and control paths in one pre-created private directory, and let launchd's
ordinary `SIGTERM` initiate the bounded I/O/drain path. Successful process exit
after the `vellum-daemon stopped` line is lifecycle completion; the `ready`
line only means endpoint construction completed and is not a launchd health or
transactional readiness protocol. Automatic restart is safe only for exact
recorded crash residue; evidence refusal must remain a visible failed start,
not an `rm` preflight. `KeepAlive` policy, restart throttling, log retention,
resource limits, session selection, and authenticated health signaling remain
deployment decisions. The GitHub candidate now ships an inert, separately
qualified [LaunchAgent plist renderer](../launchd-distribution.md) that fixes
this direct, read-only composition and validates exact private paths. It does
not install or activate the plist, provide launchd socket activation, or add an
authenticated health contract.

The [Unix ownership reuse audit](../unix-ownership-reuse-audit.md) compares
these guards with Chronos, Moira, and Keystone and records why extracting a
shared mechanism is currently unsafe.

## STRIDE/LINDDUN boundary

- The private socket directory and `0600` socket provide local OS-user
  isolation. They do not authenticate a human, agent, process, or Aegis
  principal; any process under the same OS account may connect. The optional
  required profile establishes only what its injected adapter verifies and
  makes no credential-strength or remote-peer claim.
- Canonical framing, strict decoding, opaque identities, version witnesses,
  replay handling, and exact bounds address malformed/tampered/replayed input
  within the selected profile. They do not provide cryptographic integrity.
- Replies intentionally disclose source. Socket payloads, traffic shape, source
  files, process inspection, and operator logs remain disclosure/linkability
  surfaces. The daemon prints no source, identity, file path, or socket path.
- Create-exclusive export receipts, Debug and telemetry omit the destination,
  but its canonical intent and absorbing tombstone remain linkable private
  plaintext. Filesystem side effects, backups and same-account inspection are
  independent disclosure surfaces; encryption and erasure are not claimed.
- Control status reduces STRIDE tampering/spoofing exposure with exact version,
  width, enum, reserved-bit, ownership, mode, link-count, and endpoint-identity
  checks. It limits LINDDUN disclosure and linkability to fixed aggregate
  utilization/saturation and coarse lifecycle state. Poll timing, process
  existence, load changes, and same-account endpoint impersonation remain
  observable; no authentication or non-repudiation claim is made.
- Volatile service-register receipts and disconnect tombstones disappear on
  restart. The bounded plaintext operation journal survives restart, but is
  still neither durable audit nor non-repudiation evidence.
- Safe signal observation reuses the explicit control path. Crash cleanup is
  limited to exact predecessor evidence under the exclusive persistent lock.
  Because any process under the same OS account can modify that private state,
  this is same-user operational evidence, not cryptographic authentication or
  protection from a compromised account.
- Consent, purpose, retention, principal authorization, and cross-user or
  remote networking remain outside this profile.

This daemon is suitable for local integration and coordinated GitHub source
distribution, not a crates.io or general remote-service release. `vellum-mcp`
is the peer inbound adapter over `vellum-client`; it calls the same socket
contract as any remote client and receives no daemon internals or host
capabilities.
