# Source read and exact edit contracts

Specification §22.1.2 connects F02 bounded-range reads to F11 exact-batch
publication. Reads can use a retained historical context under current release
authority. Exact edits still validate current live incarnations, revisions,
declared dependencies, rights, scope, companions and bounds at publication.
This is optimistic concurrency with immutable reads, not a read lock or lease.
Sections 12.8–12.9 now select native mediated acquisition/owner-stop products and
selector retirement. read-contexts.mjs composes this bounded read reference with
the annotation query reference; actual root/history truth and native implementation
remain gates. F02's broader design label is read-document; source bytes still
require explicit source scope and do not grant annotation access.

Specification §22.1.4 now selects tagged bounded-range/line fields and captured
source delivery profiles in source-products.md, composed with the actual single/
version-set registries and this readRange helper. This earlier source/edit suite's
counts and source-only publication limits remain unchanged.

## Required semantic fields and results

| Boundary | Required request information | Permitted result meaning |
|---|---|---|
| Bounded source read | Supported interpretation; host-established recipient; exact authority/document/incarnation/revision; UTF-8 byte range; source projection; destination/purpose; delivery and retention profile | Exact owned bytes for the complete permitted range with its context/unit/coverage, or a recipient-safe non-content disposition |
| Mediated snapshot handoff | Same release context and explicit retained extent/lifetime | Bounded non-grant selector; later access is a new currently validated handoff |
| Raw snapshot handoff | Authority for every byte and field readable through the value | Disclosure at ownership transfer; no retroactive revocation or claim of narrower mediation |
| Exact batch | All specification §3 fields; complete records normalized under §4.2.1; all actual write/read participants and typed contexts; explicit semantic companion obligations | Applied publication or validated unchanged decision; prepublication rejection/stopping; integrity faults and observation availability kept distinct |

These are mandatory semantic fields, not a selected serialization or Rust layout.
One logical field may be represented by a validated retained context rather than
repeated payload. A selector does not establish a grant. Public identifiers,
version negotiation, typed constructors, malformed-input decoding, cursor/token
formats and actual numerical profiles remain D33/D34/D43/D44 gates.

Content results do not carry unrestricted source handles as an incidental field.
They disclose no annotation body, hidden participant, global generation, total
document length or correlation unless separately permitted by the projection.
Non-content dispositions follow the whole-envelope policy in §12.6/§23.1.1;
internal reasons do not require distinct public error codes. This contract does
not invent an observable ordering among multiple simultaneous refusal causes.
Request validity itself is not permission to reveal that a protected context exists.

An exact batch's authoritative commit evidence and its recipient report are
different values. Publication can succeed while later status, notifications or
snapshot access is withheld or unavailable. Retained replay identity prevents
re-executing retained intent under §6.5, not indefinite exactly-once execution.
A new request after status loss requires the existing reconciliation/fresh-intent
rules; it is never an automatic retry of an unknown application.

## Conditional composition argument

Assume immutable exact snapshots, sound current-release validation serialized
with transfer, correct original-position normalization/splicing, complete current
dependency/authority validation, and atomic publication of all actual companions.
These are supplier/refinement obligations, not conclusions of schema validation.

For a valid range `[lo, hi)`, slicing the identified snapshot returns exactly its
bytes without changing that snapshot. Valid UTF-8 boundary checks make this a
canonical source slice; they do not normalize graphemes, CRLF or FEFF. Current
release at each handoff prevents an old read authorization from serving as a
lease. A lost snapshot cannot satisfy the exact-context premise, so substituting
current content or returning fabricated empty content is not an allowed success.

At edit publication, either every declared dependency/permission/companion/bound
is valid at the same final boundary, or no publication occurs. An earlier read
does not alter that predicate. Exact source revision inequality rejects even if
the expected replacement bytes still match; an unchanged write target cannot
hide a changed read-only dependency. For an accepted normalized batch, the
original-position splice equation in §4.2 yields the candidate bytes. Atomic
publication makes that candidate and its required companions visible together.
Unrelated current participants are framed from the immediately preceding state,
not restored from a stale preparation copy. With no semantic effect, §6.4 frames
the complete authored projection without generating edit bookkeeping.

This composes the existing conditional release/normalization/publication/no-op
arguments at a named context bridge. It is reviewed mathematical reasoning, not
a machine-checked refinement theorem or independent Cleanroom certification.
No categorical completeness, general lattice of command states, arbitrary host
termination, cross-authority transaction or semantic-agent correctness follows.

## Executable evidence and exclusions

`source-edit-contracts.mjs` exercises private finite fixtures. Read expectations
are independently constructed from scalar subsequences, including empty source,
emoji, combining marks, literal FEFF and CRLF. Every valid range is checked under
all 16 combinations of four supplied release facts; returned owned bytes cannot
mutate the retained input. Exact source-only decisions cover all 64 combinations
of six supplied admission facts over a two-write/one-read-only dependency plan.
Successful byte results have literal expectations and a right-to-left splice
comparison against the original-position emitter. Inputs are framed throughout.

Direct controls exercise identity/revision/authority mismatch, retention loss,
scalar-boundary and extent errors, expected bytes, stale read-only dependencies,
cross-authority and invalid second participants, insertion ambiguity, duplicate
participants, bounded normalization exhaustion, historical reads, an intervening
relevant change, unrelated-publication preservation, validated no-op, cancellation
and missing companion evidence. Useful successful reads and multi-document edits
prevent reject-all vacuity for the exercised domain.
Every invalid-request control also checks the returned/input state and request
against their pre-call snapshot. Deliberate generation/source mutations on refusal
must trip that frame check. An unsorted original-position batch has a literal
successful result; missing document and reused incarnation cannot enter editing.

The fixture assumes already normalized distinct document participants, with a
counterexample check for duplicates; real admission must normalize unambiguous
repeated entries under §3 rather than categorically reject them. Private integer
revisions/generations and four-edit fixture bounds are not product encodings,
non-wrap proofs or selected resource limits. Boolean gates stand in for policy,
scope, continuity, capacity, stop ordering and companion suppliers. In particular,
the fixture models only semantically unchanged companions: it cannot decide a
full operation's identity from equal source bytes. It supplies no new solver
theorem, token/decoder proof, concurrency scheduler, physical retention, actual
multi-participant history/mapping implementation or complete result projection.

## Instrumentation and remaining variants

INS-03 at IF-02/08 observes authorized handoff, unavailable context, bounded read
duration/bytes and retention pressure. INS-01 at IF-01 observes proposal decoding,
normalization/preparation and admission; INS-02 at IF-03 observes the actual
publication/no-op/rejection boundary; INS-06 observes actual history outcomes.
Count proposed writes, accepted publications, resulting snapshot handoffs and
provider acknowledgments separately. A read byte count or commit participant
count is not a globally visible metric label or permission to disclose totals.

MELT/RED distinguish request rate, bounded preparation/decision/handoff duration,
successful/no-op/refused/stopped/fault categories and observation loss. USE covers
staging/retained-source utilization, reservation saturation and exhaustion; golden
signals use their actual local clock/denominator and never treat unavailable as
zero. Diagnostic perturbations must preserve authoritative decisions. STRIDE
targets selector/context spoofing, target tampering, authority confusion, stale
dependencies and bounded-work floods. LINDDUN targets source/identity/correlation
leakage in payload, errors, lengths and pagination. STPA/FMEA covers omitted final
validation, wrong-snapshot handoff and partial/wrongly reported publication.
Runtime names/units/limits, installation and timing/privacy evidence remain gates.

F02 line interpretation and continuation formats need their declared profiles.
F11 ordinary edits, replacement, paste, composition and cut must supply their
own target/evidence lowering before this shared exact boundary; those complete
variant contracts remain open. Multi-object coherent capture, full companion
schemas/identity predicates, public outcomes/events, malformed/late/replay totality
and both adapters' refinement also remain explicit work. No required variant is
waived by this source-boundary evidence.
