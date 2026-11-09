# Scoped serialization metadata evidence

Specification §22.3.1.9 adds a metadata-only conditional basis and private lowering
into the existing document serialization participant and common settings job.
The basis retains context, document incarnation and serialization generation,
not source, an old owner root or a configuration registry. Source-read permission
is not required; current target-bound serialization write permission remains.

This refines the existing intrinsic metadata contract and editor precedent in
[source-serialization.md](source-serialization.md), not a new filesystem mechanism.
Codecs and external reads/writes remain edge implementations. A condition handle
is neither a resource capability nor a claim of authenticated host policy.

## Conditional basis and lowering

The pool reserves normalized profile bytes plus finite context slots against the
actual parent owner. Capture has finite occupancy, complete-record byte bounds and
nonrefunding lifetime issuance. Stop retains charge; protected release and final
pool closure work after faults without reopening the issuer. Seven-field context,
owner/domain/profile and actual-root facts bind before recognized loss can affect
a retained condition. Foreign or malformed facts cannot poison another condition.

The six-field scoped-source-serialization/1 request supplies no document identity
or guessed generation. A separately supplied basis privately derives those native
arguments. Full bounded semantic/dependency validation precedes stateful condition
observation. A plan retains its original basis identity and both scoped/native
intent; service checks them again through the unchanged native publisher. Raw and
scoped preparation share one slot. Separately authorized recipient contexts remain
fully bound without requiring the domain's opening recipient.

## Executable checks

- `node formal/serialization-basis-checks.mjs`: eighteen controls, seven context
  cases, eight recognized-loss cases, six malformed-grant cases and six exact
  reservation boundaries. No configuration registry or source payload is needed.
- `node formal/scoped-serialization-checks.mjs`: sixteen controls, both invocation
  routes and thirteen malformed products, including original-handle binding,
  current dependencies, cancellation, immutable intent, no-op/counter exhaustion,
  actual intervening source publication/journal framing and unknown fault cleanup.
- `node formal/serialization-basis-mutations.mjs`: nine exact one-preimage mutants;
  only assertion failures after a coherent baseline count as detection.
- `node formal/scoped-serialization-budget-checks.mjs`: nine exact parent-wrapper,
  captured-plan and candidate-root boundaries plus three assertion-killed mutants.
  Owner accessors exist only in generated test graphs, never the runtime module.
- `node formal/serialization-basis-disposal-checks.mjs`: actual native disposal
  invalidates a retained basis, refuses recapture and preserves protected release
  and spent issuance.
- `node formal/scoped-serialization-writer-checks.mjs`: an admitted old save keeps
  original bytes/generation; late old acknowledgment remains historical; a new
  capture uses changed BOM intent without changing source revision. Disposition
  and quiescence remain separate. Execution evidence is supplied, not a file write.

Discovery has ten guarded outer forms and 94 standard-validator instances.
Cancellation covers thirty forms/twelve kinds, 480 gate/240 context cases, sixty
races and 7,770 traces/21,780 edges; earlier controls and mutations remain.

## Limits and observation

Reference byte charges do not prove physical heap peaks, concurrency, crash
durability or measured cost. Public basis/result/event codecs, complete F16,
actual host policy, production package separation and independent assurance remain
open. No solver condition, network hop, UI preference or filesystem procedure is
added. INS-01/02/03/07/09/12/13 distinguish condition lifecycle, invocation lifecycle
and persistence evidence. Separate their occupancy, lifetime issuance, queue/service
and acknowledgment populations; ordinary labels exclude payloads and identities.
