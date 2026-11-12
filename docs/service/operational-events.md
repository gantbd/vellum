# Local daemon operational events

The daemon exposes one passive, content-free event port. The standalone binary
installs the reference stderr sink; embedded compositions may install another
`OperationalEventSink`. Delivery is synchronous, at most once, and bounded to
one sink call per event. Sink errors and unwinds are caught, counted, and never
become request, authority, replay, lifecycle, or shutdown inputs.

The closed algebra is:

- lifecycle: `ready`, `drain_started`, `drain_replayed`, `drain_pending`,
  `drain_completed`, `fenced`;
- request: `accepted`, `completed`;
- refusal: `shutdown`, `not_ready`, `capacity`, `ownership`, `accept`,
  `transport`, `lifecycle_invariant`.

`request.accepted` means that a connection entered the daemon's single
in-flight obligation. It does not assert authority approval or a semantically
accepted Vellum reply. Service-level refusals are deliberately not decoded by
this boundary: doing so would couple transport observation to service content
and authority evidence.

Each `serve_next` attempt receives a process-local monotonically allocated
correlation number. Request and refusal events from that attempt share it.
Lifecycle events have no request correlation. Sequence and correlation
exhaustion is sticky and visible in `OperationalEventUsage`; correlation
exhaustion drops correlation and sequence exhaustion stops delivery rather than
reusing an identity. Neither identifier is stable
across restarts or derived from a principal, client, document, request ID, path,
or source.

## STRIDE/LINDDUN field policy

Allowed fields are the fixed schema marker, monotone process-local sequence,
optional process-local correlation, and a closed enum value. Paths, root names,
source text, edit payloads, locators, protocol frames, client/principal IDs,
tokens, secrets, authority evidence, refusal details, timestamps, hostnames,
PIDs, and free-form error strings are forbidden.

This policy reduces spoofable labels and log injection (STRIDE), prevents the
event stream becoming an authority channel, and limits linkability,
identifiability, disclosure, and detectability (LINDDUN). Correlation still
reveals the ordering and coarse success/failure shape of local activity, so
access to stderr or any replacement sink remains privileged operational access.

## Limits

- The port provides no durable delivery, backpressure, retry, batching, remote
  exporter, tracing span propagation, or cross-process correlation.
- A sink implementation can retain its fixed event inputs without bound; the
  daemon bounds calls and payload cardinality, not third-party sink storage.
- Rust's panic hook runs before `catch_unwind`, so a panicking custom sink may
  produce its own panic-hook output even though the daemon outcome is preserved.
- Process aborts, double panics, and sinks compiled with `panic=abort` cannot be
  contained. Production sinks should return the opaque error instead of panic.
- Timing remains in the separate fixed histogram. No wall-clock value is added
  to events because it would increase linkability and clock trust requirements.
