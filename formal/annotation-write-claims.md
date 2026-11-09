# Annotation write tickets and baseline holds

Specification §16.5.6 binds §16.5.5's actual native reservation pool to the shared
writer's actual annotation tickets. The accepted precedents are the existing
private clipboard continuation, retained ticket/artifact ownership and separate
result/disposition/quiescence contracts. A supplied discharge Boolean alone cannot
release a linked hold. Ordinary-save atomic admission and qualified-baseline
installation/hold transfer remain separate required integrations.

## Claim provenance and bounded ownership

Only claimAnnotationWriteReservation in the checked source writer calls the
package-private claim constructor. It first resolves the actual private owner and
task, checks three exact-true current control/preparation/historical-artifact facts,
and requires a queued, unstopped annotation ticket with its original ordinary
annotation association and actual native domain. Existing claims are idempotent;
first-time postdispatch construction cannot retrofit capacity onto an effect.

The draft uses actual retained codec bytes, source/set, original seven-field
context, origin, full binding/registration and requested acknowledgment. Receipt
binding generation and ordinary annotation binding generation stay distinct.
The writer preflights the future baseline envelope under its own finite version/
baseline limits and charges the stored descriptor against its actual shared
held-byte headroom. Source/annotation admissions share the resulting pressure.
Claim copies returned by trusted inspection cannot mutate actual ticket facts.
The declared descriptor charge includes maximum link/control metadata, not only
the byte array. Scalar dispatch/discharge checks do not recopy artifact bodies;
authorized binding and trusted inspection materialize the bounded descriptor.

The package-private facet retains references to the actual writer/task record.
It reads that record's phase, stop, application, disposition, quiescence,
contradiction, destination fault and destination-reservation state. It verifies
that the actual task map still contains that exact record. No caller-supplied
status object, artifact descriptor or task-shaped copy creates those facts.
The artifact's native domain is retained privately, never added to provider handoff.

A static callsite check confirms the sole constructor caller in the current
non-test source tree. The reference boundary assumes that internal facet is not
an untrusted factory. Enforcing production package/module visibility is not proved
by JavaScript export syntax, the callsite scan or the claim handle's shape.

## Binding and launch

The actual native owner resolves its unique attached pool. Binding requires the
same domain object as the artifact capture, not merely equal authority/engine
strings. It revalidates the whole original annotation binding and provider
registration, complete current pair and prelaunch source/set. The pool and writer
must agree on the serialized version limit; otherwise a valid future writer
receipt could exceed the held envelope. The pool builds that envelope from the
private claim. A copied claim or another domain's pool cannot bind it.

Each claim binds one live reservation. Repeating the same binding returns its
existing handle without duplicate rows, charges or issuance. A generic sizing
hold with matching bytes does not establish a ticket link. Once claimed, actual
writer dispatch requires a live link and a nonfaulted original native domain.
Refused binding cannot degrade into an unreserved launch. A later edit after
binding does not refresh the old artifact, set or receipt attribution; a stale
unbound capture instead refuses binding.

The two-step selection is explicit: claim creation may charge writer storage and
pool binding may later refuse. A stopped never-dispatched attempt can release its
destination reservation and retire safely if no hold was installed. This is not
an atomic ordinary-save admission contract or an automatic change to old exports.

## Discharge and retirement

The native pool retains its four exact-true release facts, but for linked rows it
also checks the actual writer record. Export-destination reservation release must
already have happened. A queued ticket must be stopped; a dispatched ticket must
have application not-written, safe disposition, quiescence, no contradiction and
no destination-wide quarantine. Other tickets' authoritative contradiction at the
same destination therefore blocks release of an otherwise resolved ticket too.

Unknown and known-written attempts keep their holds, including after stop or
quiescence. Known-written attempts need the future qualified-baseline installation/
transfer path; they are not silently abandoned to make reclamation pass. A bound
claim pins actual writer ticket/artifact retirement. After a valid nonapplication
or stopped-prelaunch discharge, the link becomes released and cannot rebind.
Actual ticket retirement removes the claim and releases its separately charged
writer storage. Both writer and pool lifetime issuance remain consumed.

Receipt, disposition and quiescence authenticity/provider truth remain premises of
the existing evidence observer. The claim checks those observed facts; it does not
prove that a provider physically stopped, wrote no bytes or honored synchronization.
An actual native fault fences new linked launch while protected safe cleanup remains
available. No external effect is undone and no authored root is rolled back.

## Conditional preservation arguments

For finite normalized inputs, immutable artifacts, the checked constructor's
package-private confinement, truthful supplied policy/provider facts and serialized
component transitions:

1. A claim originates in an actual owned annotation ticket and its retained bytes.
   Binding preserves full capture identity and selects its original native domain.
   Nominal handle lookup prevents copied public shapes from selecting another row.
2. Writer storage grows only after Q ≤ C−U and finite descriptor/envelope checks.
   Pool admission independently charges its own retained envelope and future
   baseline demand. The distinct copies are not treated as one shared allocation.
   Refusal preserves the existing charges; retirement removes only its actual row.
3. A claimed ticket has no dispatch path before actual binding. Repeated claims/
   binds do not add another hold, and released or stale/unbound claims cannot
   reactivate launch. An actual native fault overrides positive host launch flags.
4. Linked release implies either stopped prelaunch or actual confirmed nonapplication
   with separate disposition, quiescence, destination release and uncontested scope.
   Thus unknown/written effects do not become discharged merely through caller stop
   or supplied obligation flags. The writer's own effect and destination obligations
   remain independent and may continue after a native capacity release.
5. A bound claim prevents artifact/ticket removal. Removing the link only after safe
   discharge permits subsequent writer retirement without dangling a native hold.
   Historical producing baselines still use their independent existing pins.

These are authored conditional arguments and actual-code checks, not proof-assistant
induction, a new lattice theorem, physical accounting or independent certification.
The new selection does not add a public operation or make the whole engine finite.

## Executable evidence

annotation-claim-checks.mjs covers 42 actual native artifact-stage/ticket-binding
cases: fourteen annotation variants across capture, undo and redo. It checks eight
claim, 32 reserve and sixteen release Boolean combinations, plus sixteen actual
application/disposition/quiescence cells spanning unknown, nonapplication, written
and contested states. All six result/disposition/quiescence orders exercise actual
destination release, linked hold release, writer retirement and nonrecycled issuance.

Sixteen controls cover unbound launch, idempotent retries, foreign/cloned handles,
retroactive/missing association, whole registration/binding/version-limit checks,
stopped prelaunch discharge, stopped unknown/written effects, later edits, shared
writer claim pressure, copy/strict-grant safety, failed-bind cleanup, native faults,
generic-hold substitution, stale unbound capture, destination-wide quarantine and
the checked constructor callsite. The sixteen state cells do not enumerate every
possible receipt history or arbitrary interleaving.

Eighteen assertion-detected mutants follow a passing dynamically rewired claim,
pool, native domain, producer, codec and writer baseline. One domain mutant removes
both component checks to expose the cross-domain error. Whole-binding, stopped
and disposition guards are retained, but not claimed independently mutation-
sensitive where current registration/pair/destination-release guards already imply
the same refusal on reachable test states. No import, syntax or tool failure counts
as killing a semantic mutant. Solver totals remain 234 across fourteen manifests.

## Remaining root and operational gates

The existing explicit export still returns a destination-reservation root candidate
through its supplied-root contract. A claim link neither installs it into native
authority nor proves complete coherence between those publication routes. Atomic
ordinary-save admission, qualified baseline installation/hold transfer, every
non-invocation route, public invocation/events and final package visibility remain
open. Actual host/provider/discharge truth, full global/origin/physical and temporary
preparation accounting, hostile decoding, production/concurrency and independent
assurance are not established.

PATH-05/12 and INS-01/03/04/07/08/13 distinguish claim creation/binding, stage and
held occupancy, refused launch, result uncertainty, known writes/nonapplication,
destination release and actual retirement. MELT, RED/USE/golden, STRIDE and LINDDUN
cover forged/cross-domain handles, stale capture, fallback, native fault, quarantine
bypass, premature refunds and private activity correlation. No installed telemetry,
timing noninterference or new numeric SLO is claimed. Source/annotation bytes,
paths, contexts, origins, identities and version tokens are not metric labels.
