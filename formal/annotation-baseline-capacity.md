# Annotation baseline capacity envelopes

Specification §16.5.4 defines the sizing prerequisite for ordinary annotation-save
admission. It refines §16.5.3's explicit baseline publication and §17.2.2's adopted
finite-capacity admission contract: promised downstream obligations need capacity
before effects launch. The actual reservation ledger and its enforcement on every
competing publication are separate required integrations, not supplied host flags
that this helper claims to verify.

## Fixed capture and future evidence

The helper accepts finite normalized baseline descriptors and declared logical
byte limits. It is not a hostile-object decoder, authority check, write qualifier
or public API. The actual native producer supplies exact bytes; admission must
separately establish complete ordinary binding, registration, context, source/set
and requested-strength attribution. The envelope freezes those fields and bytes.
Only future external version, dispatch serial and acknowledgment vary. Those
variables are sized, not predicted or authenticated.

Let S(x) denote UTF-8 byte length of the compact JSON reference representation.
P is the captured baseline with external version null, dispatch serial
Number.MAX_SAFE_INTEGER and acknowledgment synchronized. Let T be maxTagBytes,
the bound on the entire serialized non-null version object, not its token alone.
The computed baseline bound is:

`B = S(P) + max(0, T - 4)`

For any admitted future baseline A with the same fixed fields and octets:

`S(A) = S(P) + S(A.resourceVersion) - 4 + serialDelta + acknowledgmentDelta`

Both deltas are nonpositive: a natural safe serial has at most the maximum's
decimal width, and written is shorter than synchronized in this encoding.
Version size is four for null or at most T otherwise. Therefore S(A) ≤ B.
The implementation rejects unsafe sums and B above maxBaselineBytes. Null remains
valid for small T; this does not admit a non-null object that exceeds T. Exact
byte arrays and escaped strings are measured through the actual encoding rather
than estimated from raw payload length or character count.

For the complete current root, R0 replaces the addressed baseline with null and
sets publication to its largest safe width. Inserting a previously absent field
includes its name, punctuation and value overhead. For any candidate baseline A
and one available publication advance:

`S(candidate) ≤ S(R0) - 4 + B`

The helper rejects a missing/duplicate/foreign/disposed target, an exhausted
publication counter, an already over-limit input root, unsafe arithmetic or an
over-limit complete candidate. It reports the bound, nonnegative additional root
bytes and one publication unit without changing any input. Existing large baseline
replacement may reduce occupancy; sizing old data does not validate its eligibility.

## Reservation integration obligations

An envelope can be copied and checked for sizing consistency; it is not a bearer
grant or evidence of a held resource. The current helper does not issue writer
tasks, spend codec issuance, dispatch effects, change roots or debit a reservation
ledger. Actual capture/disclosure/retention/effect authority remains with the
existing producer, codec, shared writer and baseline publisher.

The root bound must be reevaluated as edits, views, histories, reservations and
other companions change. Ordinary-save admission must hold all promised staging,
evidence, root-slot and publication resources in the shared capacity discipline.
Every competing publication must preserve held floors atomically. A frozen
document, stale root estimate or later size check after writing is not equivalent.
Unknown effects, stop, quiescence and safe obligation discharge stay distinct.
This evidence does not choose an ordinary-save request schema or claim those
complete integrations already exist.

## Executable evidence and proof scope

annotation-baseline-capacity-checks.mjs uses the actual native producer and shared
writer/baseline fixture, preparing envelopes before each write. Fourteen annotation
variants across capture, undo and redo give 42 native cases. Each checks twelve
future-evidence combinations, for 504 cases: two serial widths, two acknowledgment
widths and null, escaped Unicode/control and maximum-size external versions.
The actual subsequently published baseline fits its earlier capture envelope;
independent export reservation changes trigger a fresh current-root calculation.

Ten controls cover exact baseline and complete-root limits, missing/null/large
prior slots, concurrent-root growth without freezing edits, fixed-field and byte
substitution, serialized-version limits, small-token budgets, integer overflow,
envelope integrity, invalid targets/counters and unchanged writer/root state.
The maximum-size ASCII token witness exactly reaches the baseline bound, and a
valid maximum-width publication candidate exactly reaches the root bound. These
are normalized reference witnesses, not an unbounded native reachability proof.

Twelve assertion-detected mutants follow a passing dynamically loaded baseline.
They remove future version/serial/acknowledgment growth, baseline/root ceilings,
publication width, missing-slot overhead, fixed capture checks, version limits,
envelope integrity, complete-root accounting and counter availability. An unsafe
addition check is not claimed independently mutation-sensitive when the separate
safe finite ceiling already rejects every overflowing sum.

The inequalities are authored conditional arguments over the declared encoding;
the executable checks are finite actual-code evidence, not a new SMT theorem,
whole-engine categorical closure or independent Cleanroom certification. The
existing total stays 234 SMT conditions across fourteen manifests. Actual shared
reservation and all-publication enforcement, ordinary-save admission/load,
native atomic installation, host/provider/policy/discharge truth, global/physical
costs, public invocation/events, hostile decoding, production/concurrency and
independent assurance remain open.

INS-01/02/07/08/13 and PATH-05 separate sizing refusals, logical reservation holds,
root publication, acknowledged effects and real discharge. MELT, RED/USE/golden
signals, STRIDE and LINDDUN cover quota undercharge, stale roots, forged attribution,
counter exhaustion and private equality/occupancy correlation. No installed
instrumentation or timing claim is made. Raw bytes, contexts, paths, origins,
identities and version tokens are not ordinary metric labels.
