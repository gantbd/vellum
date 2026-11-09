# Bounded invocation replay

D03 adopts bounded incarnation-scoped deduplication and explicit retirement in
requirements §5.7 and specification §6.5. An identity binds immutable intent;
matching retries observe rather than execute again. Changed intent rejects.
Execution eligibility, retained status and unresolved obligations are distinct.
Neither cache absence nor loss of observation establishes fresh intent or known
nonapplication. This is a local engine contract, not a network protocol.

## Conditional safety argument

For each valid identity, let `started` initially be false. The only first-execution
transition tests eligibility and atomically consumes it while setting `started`.
All duplicate, mismatch, completion, status-loss and retirement transitions leave
`started` unchanged. Retirement never restores eligibility; identity reuse must
not alias an obsolete admission domain. By induction over any finite admitted
trace, there is at most one first execution for each identity. Publication is
permitted only once by its pending execution's final validated decision; terminal
outcomes cannot reenter preparation. This composes conditionally with the existing
publication contract, not by assuming every production participant is present.

This argument requires a truthful identity/intent comparison, serialized admission,
non-confusable validity/retirement and complete execution state. Random IDs and a
bounded result cache alone do not supply these premises. With one cache entry,
`A; B; retry A` executes A twice if eviction makes a miss fresh. After forgetting
A, cache state is identical to the history containing only B; admission needs
additional validity/fencing information or weaker retry guarantees.

Status deletion changes observation availability, not publication history. Release
checks current policy without rewriting historical facts. Retirement, epoch change
and observation loss preserve unresolved effect/worker charges; only their own
supported discharge transition may release them. A trusted owner failure may
destroy observation or the admission domain; it does not prove a past write absent.
These are authored induction/frame arguments, not proof-assistant replay or an
unconditional exactly-once/termination theorem.

## Finite reference exploration

`replay-reference.mjs` explores the entire reachable graph of a private fixture:
two incarnations, two generations in each, one slot, two opaque intents, two submit
route labels, one current grant, free/pending/done/retired control states, four
terminal dispositions and four independently discharged effect obligations.
It checks 48,664 distinct states and 2,092,552 labeled transitions, without a
transition-depth cutoff. The 200,000-state exploration cap is a tooling failure
guard, not a production capacity. No runtime can retain this proof's ghost history
as if it were a proposed unbounded tombstone design.

Ghost first-execution/publication/dispatch counts check at-most-once execution and
dispatch in this fixture. Duplicate/mismatch/status-observation transitions frame
the entire state; nonpublication outcomes preserve publication counts. Old or
unissued tickets reject; wrong-ticket completions cannot finish a newer request.
Revocation withholds even a retained result. Forget/retire/advance/incarnation
transitions preserve publication counts and unresolved obligations; generations
do not wrap. Both submit routes use the same authoritative transition function.

Seven reachable witnesses cover duplicate pending, intent mismatch, forgotten
terminal status without reexecution, retired-but-held effects, generation
exhaustion, obsolete incarnation rejection and retained-result withholding.
Nine direct sensitivity controls expose cache-miss admission, changed-intent
rebinding, forgotten-applied confusion, early charge discharge, cached-result
release bypass, generation/incarnation aliasing, stopped restart and late
completion redirection. They are selected fault examples, not mutation adequacy.

The model uses an opaque publication counter, not actual text, D01 classifiers,
D02 ordering or full multi-document publication. `finish` abstracts final validation
and supplies terminal outcomes; it does not establish actual dependency validation
or integrity-fault containment. `discharge` assumes justified provider evidence.
Its one-slot representation, bounded integer ticket notation, equality of intent
symbols, internal observation labels and administrative epoch transition are test
notation, not adopted API fields, error precedence, clock/restart semantics or
public token/disclosure profiles. It does not model actual active-request timeout
retirement, crash recovery, fairness, response latency or physical effects.

## Local symbolic checks

`replay-laws.smt2` has five positive conditions, each with SAT premises then an
UNSAT negated conclusion: duplicate frame, intent mismatch binding, retired-domain
fencing, release gating and the at-most-once count step. Four SAT controls show
that forgotten status can coexist with publication, cache-miss admission duplicates
execution, retired status can coexist with an outstanding obligation and matching
identity does not imply matching intent. Intent arrays denote exact abstract
semantic values, not hashes or an adopted serialization. Freshness consumption
and fencing validity are premises, not implementation facts established by Z3.
The exact ordered `replayCases` manifest uses trusted Z3 4.15.4 without independent
certificate replay. Together with the same-author finite reference, these checks
support the conditional design argument, not independent Cleanroom certification.

## Precedent and integration

[AWS's idempotent API guidance](https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/)
uses explicit caller intent identities and parameter mismatch handling.
[NFSv4.1 session replay mechanisms](https://www.rfc-editor.org/rfc/rfc8881.html#section-2.10.6)
demonstrate bounded slot/sequence reply caching and separate session-loss handling.
These are mechanism precedents, not editor equivalence or adoption of their
transports, encodings, counter-wrap rules or durable execution guarantees.

IF-01/03 and PATH-01 retain shared authoritative admission/publication; INS-01/02
observe attempts versus first admissions and actual outcomes. IF-02 / INS-03
retains current status-release gating. Existing profiles cover dedup hits,
mismatch/retirement, status occupancy/loss and unresolved obligations without
identity-valued metric labels, intent contents or raw tokens. STRIDE/LINDDUN risks
include identity spoof/rebinding, late-message aliasing, status-existence/linkage
disclosure, replay floods, reservation pressure and diagnostic-authority confusion.
Protected observation is not the dedup ledger or a receipt of physical effects.

Remaining refinement includes actual admission/identity/intent schemas, equality,
owned memory, no-wrap/fencing truth, finite retained/admitted/obligation budgets,
retention guarantees and pressure behavior, clocks and supported host incarnation
domains, current recipient projections, publication/effect integration and actual
conformance. D05/D06/D09/D12/D22/D33 and supplier/implementation gates stay open.
No new runtime component, network service or mandatory persistence is introduced.
