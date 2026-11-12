# Vellum wire protocol V1

Status: canonical transport-neutral codec profile, implemented 2026-11-09.

`vellum-wire` is the anti-corruption boundary between closed native protocol
values and bytes. It does not select a stream, socket, daemon, authentication
scheme, authorization policy, retry policy, or log format.

## Canonical representation

Every integer is unsigned and big-endian. Every frame begins with version byte
`01` and one frame-kind byte. Text is strict UTF-8 prefixed by one unsigned
32-bit byte length. There are no optional fields, alternate representations,
padding bytes, ignored members, or trailing data.

| Frame | Kind | Remaining fields |
| --- | ---: | --- |
| Connect request | `01` | none |
| Execute request | `02` | client u64, request u64, command |
| Disconnect request | `03` | client u64 |
| Durable execute request | `04` | operation-ref 2×u64, client u64, request u64, stable durable intent |
| Durable receipt lookup request | `05` | operation-ref 2×u64, client u64, request u64, identical stable durable intent |
| Durable receipt retirement request | `06` | operation-ref 2×u64, client u64, request u64, identical stable durable intent |
| Connected reply | `81` | client u64 |
| Connect-refused reply | `82` | refusal u8 |
| Executed reply | `83` | protocol reply |
| Disconnected reply | `84` | none |
| Disconnect-refused reply | `85` | refusal u8 |
| Durable executed reply | `86` | stable durable outcome |
| Durable receipt looked-up reply | `87` | complete outcome, conflict, pending, absent-or-foreign, or unavailable |
| Durable receipt retired reply | `88` | retired, absent-or-foreign, conflict, outcome-unknown, or unavailable |

Commands use tag `01` for Snapshot. Replace uses tag `02`, followed by witness
u64, start u64, end u64, and inserted text. Protocol replies use tag `01` for
Accepted and `02` for Refused. Accepted outcomes use tag `01` for Snapshot and
`02` for Replaced; both carry witness u64, revision u64, and source text.

WorkspaceChanges uses additive command/outcome tag `0d`. Its bounded V1 page
contains at most one content-free event and an exact workspace-scoped
continuation; its alternate accepted Gap form makes retention loss explicit.
ForeignChangeCursor and InvalidChangeCursor use additive refusal tags `1b` and
`1c`. See [change-feed.md](change-feed.md).

Exact file and directory admission receipts use additive accepted-outcome tags
`0f` and `10`. Tags `04` and `05` retain their legacy whole-inventory decoders
but are no longer emitted by the service. A file receipt carries status,
workspace-scoped document, and an optional same-workspace root. A directory
receipt carries a same-workspace root, added/joined counts, and the complete
ordered added-document identities. Those local u64 identities are concatenated
into one canonical length-delimited field; its byte length must equal
`8 * added`, every identity must be nonzero and distinct, and the field remains
subject to the normal frame/field-byte ceiling. Packing bounds field count; it
does not make the identities paths, capabilities, or current-membership proof.

Refusal tags are explicit and independent of Rust declaration order. The
original sixteen tags remain `01` UnknownClient, `02` ClientCapacity, `03`
RequestCapacity through `10` hexadecimal CounterExhausted. Tag `11`
hexadecimal is ClientLifetimeCapacity. Adding or reordering a Rust enum variant
does not change assigned octets.

Zero is invalid for client, request, and witness identities. Unknown versions,
kinds, commands, outcomes, replies, or refusals are closed decoding errors;
V1 never guesses that an unknown value has old semantics.

Durable intent is a separate closed sum. Membership forms carry the exact
locator or scoped target. Save and replace-and-save carry an explicit scoped
document and source revision; they never encode a selected view or live
`VersionWitness`. The operation reference is fixed at 128 bits and encoded as
two big-endian u64 fields. Stable receipts contain historical document/revision
evidence but no witness. Six save evidence flags are canonically packed into a
single byte so the largest durable reply remains within V1's sixteen-field
bound. The journal codecs encode only stable intent/outcome, excluding the
ephemeral client/request envelope.
Durable-execute outcome tag `03` is the absorbing `Retired` terminal for an
exact delayed attempt after a permanent retirement tombstone. Changed intent
continues to conflict; this tag is never stored as an active receipt.

## Bounds and allocation

Every encode/decode call receives explicit maximum frame bytes, text bytes, and
field count. Encoding checks text and field ceilings before reserving the exact
frame. Decoding uses `keystone-codec::Decoder` for input, field, count,
truncation, and length-overflow checks, then applies Vellum-owned identity,
UTF-8, schema, and trailing-data checks.

Keystone supplies mechanics only. It does not own Vellum tags, versions,
semantic limits, compatibility, disclosure, or refusal policy. The semantic
`vellum-protocol` crate remains dependency-free.

## Compatibility

V1 is exact. A future incompatible schema receives a new leading version and a
separate decoder. A transport must negotiate or select supported versions
before interpreting requests; it must not retry bytes under another version.
Matching V1 values have one encoding, and decoding followed by encoding returns
that encoding.

The codec consumes exactly one complete frame. It is intentionally not stream
framing. `vellum-transport` adds one bounded unsigned-32-bit outer frame length,
rejects partial/oversized frames before payload allocation, and passes exactly
one payload to the codec. `vellum-daemon` uses one such request/reply exchange
per Unix connection.

## Golden and adversarial evidence

Golden fixtures live under `protocol/vellum-wire/tests/fixtures` for the
supported command and reply profiles, including exact admission receipts.
Tests verify exact octets and semantic
round trips. They also cover every closed reply variant, every truncation point
of a Replace request, trailing data, frame/text/field ceilings, unsupported
versions, unknown tags, zero identities, and invalid UTF-8.

`Debug` output is not a wire dump: inserted and snapshot text are redacted and
opaque identities do not reveal their numeric values. Wire payloads themselves
intentionally contain source and therefore require authenticated transport,
authorization, metadata/log review, and retention analysis before remote use.
