# Actual domain ordinary admission fence

Specification §17.4.1 implements a first-party admission primitive in the actual
invocation-domain owner. It closes selected native bypasses that remain possible
when a working shutdown composition keeps its fence only in an outer wrapper.
It is not the complete F18 shutdown controller or a public command spelling.

## State ownership and transitions

The domain retains at most one opaque absorbing fence, its original actual root
and one declared root-frame charge. Installation requires exact original context,
strict protected-control facts, exact current root identity and sufficient actual
parent headroom. Copied/foreign roots and domains cannot establish correspondence.
The required charge is maxRootBytes plus the fence token's serialized units;
installation debits the actual domain, not a separately asserted budget. This is
logical serialized-frame accounting, not an allocation or RSS measurement.

A matching retry returns the same fence using the original root identity, even
if permitted baseline bookkeeping later advances the current root. It does not
reserve again, refresh consent, reopen admission or establish a new wait budget.
There is no fence-reset operation. Complete owner discharge and engine retirement
remain separate composition obligations; retiring an invocation does not release
the retained fence frame.

One actual-domain native admission section spans selected fact reads, preparation
and commit. It excludes installing a fence mid-operation and unwinds in finally.
Only its original opaque section token clears it. Running or preparing invocations
also exclude installation. These are serialized reference relations, not a mutex
or arbitrary concurrent installation proof.

| Native path | Fence behavior |
| --- | --- |
| Invocation issuance and first admission | Refuse new acquisition/admission. Already-admitted matching retry can return pending or observe-existing without another execution. |
| Pending authored invocation | Refuse ordinary execution. Protected stop allows existing nonpublication settlement and later retirement. |
| Ordinary source and annotation save admission | Refuse before installing a new writer task, artifact or baseline claim. |
| Source-destination preparation crossing installation | Check the actual sealed operation before consuming/committing its token. A preparation made before fencing cannot install a new admission afterward. |
| Protected source-destination bookkeeping | Existing baseline and release operations retain their actual native guards. No general lifecycle/SaveAs/rebind exception is selected. |
| Already-admitted source and annotation saves | Preserve captures, current dispatch guards, authentic evidence, independent qualified baseline installation and separate quiescence. The primitive does not decide the shutdown controller's eligible-task inventory. |

Fault containment is independent. A fault refuses installing a new fence; late
authentic evidence can still be retained privately. Faulted-domain guards may
refuse baseline installation, which is why faulting is not an implementation of
the admission fence.

## Scoped preservation argument

Under the existing trusted normalized facts, fixed first-party calls and serialized
execution assumptions, only the installation function creates the retained fence.
Its guard precedes debit/insertion, and the duplicate branch cannot change state.
Thereafter the selected issuance/first-admission and native writer guards refuse
new work. Source-destination commit checks the sealed operation before consumption;
the current root need not change for that admission to be blocked. A pending
authored invocation cannot enter execution without a stop request, whose existing
nonpublication branch preserves the root.

The admission section and running/preparing checks prevent successful installation
between a selected operation's guard and native commit. Exception cleanup clears
only the actual section, not a fence, task, issuance record or retained obligation.
Protected settlement and native baseline/evidence transitions retain their existing
guards and cannot reset admission. Independent actual domains use different private
owner records. These are conditional source-level arguments supported by scoped
executions, not machine-checked induction or whole-engine categorical closure.

## Evidence and limits

admission-fence-checks.mjs covers twelve controls, twelve strict-control cases,
eleven scope/identity cases, three exact parent-headroom boundaries and four
active-operation reentry cases. It exercises the four actual native bypasses,
prepared-token crossing, both save completion orders, observational matching retry,
stop/retirement, exception unwinding and fault-versus-fence behavior.

admission-fence-mutations.mjs runs an unchanged coherent-module baseline before
fourteen semantic mutation alternatives. Redundant guards are removed together
where necessary to challenge the same path. Mutation construction/parse/import
failures are not semantic detections. Only validated passing runs release their
verifier-owned transient capture scope; rejected runs retain diagnostic captures.

Complete authority-owner inventory and future owner acquisition, shutdown consent
and acceptance, eligible-dispatch inventory, other native admission/root-install
paths, public F18 codec/discovery, real service/clock/provider truth, physical
refinement and independent certification remain open. No primitive query can report
shutdown accepted. The implementation is unadopted until full frozen qualification,
endpoint/hash reconciliation and ordered adoption complete.

## Instrumentation boundaries

Existing IF-01/03/04, PATH-12 and INS-01/02/03/04/08/12/13 distinguish offered versus refused
ordinary work, protected progress, held frame units, admitted effects, settlement
and separate quiescence. Use the existing measurement populations and continuity/
loss rules; do not label metrics with control tokens, documents, destinations or
other private identities. Supervision-only fence inspection is not a recipient
receipt, an installed probe or evidence of successful shutdown.
