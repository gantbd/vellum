# Mutation construction and semantic detection

Specification §24.1 requires setup failures to fail verification, not masquerade
as semantic counterexamples. Sixty-three existing reference harnesses now finish
candidate source construction and module loading before entering the catch that
accepts assertion failures from their semantic checks. Engine predicates, mutation
identities and scoped claim boundaries are unchanged.

## Repaired targets

The annotation-baseline mutation `current-baseline-corruption-repaired` previously
named the anchor call without its now-required domain argument. Its missing-target
assertion was incorrectly caught as a detected mutant. The target now names the
actual domain-bound call; the existing corrupted-baseline control detects removal
of that predicate.

The lifecycle mutation `prepared-slot-submits-another-job-kind` previously named
an obsolete two-part preparation guard. The current guard includes settings and
registration preparations too. Its target now removes only the lifecycle-prepared
clause, retaining other participants' guards. The existing foreign-job control
detects that weakening.

The reversal mutation `forget-keeps-reversal-receipt` previously assumed the
reversal receipt was the last field cleared before returning from forget. Later
receipt kinds invalidated that source target. It now removes only reversal-receipt
clearing from the exact forget sequence, retaining common/annotation clearing and
all following receipt cleanup. The existing retained-receipt count/byte assertions
detect the leak even when common outcome release already withholds the result.
All three mutations retain their original names and banks.

These are corrected verification defects, not evidence that the production
predicates themselves failed. The original stale-target results cannot establish
semantic detection of those three mutations; fresh execution is required.

## Regression checks and scope

Run `node formal/mutation-harness-contract-checks.mjs`. Eight executable control-flow
fixtures distinguish setup assertions, import-like syntax/runtime failures,
semantic assertions, surviving mutants and non-assertion semantic errors. Four
lexical controls challenge the selected synchronous/asynchronous/spread shapes and
allow the registration baseline's outer cleanup block. The checker independently
lists all 63 reviewed harnesses, checks their selected construction separation and
retained cleanup, and verifies all three repaired targets occur in current source.

The lexical checks cover selected source shapes; they are not JavaScript AST or
arbitrary control-flow proofs. They do not establish all mutation match
cardinalities, all intended oracle relationships or completeness of every harness.
Actual native mutation execution and the full frozen-input suite remain required.
The two newer serialization harnesses separately retain their missing/ambiguous
preimage controls. No solver condition, runtime component, API, I/O, UI policy or
instrumentation path is added. Independent assurance and full C9 traceability
remain open.
