# Terminally ineligible annotation baseline holds

Specification §16.5.10 separates an impossible future successful-baseline promise
from retained evidence that a write applied. It adds one private control operation
to discharge only the former, using actual writer/claim/pool facts. It does not
turn synchronization failure into nonapplication, success or evidence retirement.

## Eligibility in the existing evidence lattice

The captured required acknowledgment is computed from immutable request and
registration fields: explicit synchronized, or ordinary with captured sync support,
requires synchronized; write-only and ordinary without captured support require
written. Applied bytes alone cannot satisfy a captured synchronized requirement.

The existing synchronization vocabulary is unknown, succeeded, failed and
not-requested. Unknown is lower information. Distinct known atoms conflict rather
than overwrite one another. Once accepted synchronization is failed or not-requested,
a consistent later receipt cannot turn that same attempt into succeeded. Lower
information preserves the known atom; conflicting information quarantines the
attempt/destination and still cannot qualify it. Version metadata, stop and later
registration changes do not alter its captured requirement or write identity.

This gives a bounded ineligibility condition without inventing a timeout: actual
applied write, captured sync requirement and a known incompatible synchronization
atom. A write-only request with failed synchronization is deliberately excluded,
because its written acknowledgment may already be satisfied. Unknown information
is also excluded; a later valid receipt can still supply the missing guarantee.

## Actual cleanup boundary

releaseInvocationIneligibleBaseline resolves the actual native domain and its
attached pool. The pool resolves the exact original-domain claim/link/hold through
a bounded control view, without reading/copying document bodies or future-baseline
payloads, and
requires current owner control plus the existing hold obligation/retention/drain
facts. The private claim consumer independently rechecks actual live ticket facts:
dispatched, written, captured synchronized requirement, failed/not-requested sync,
safe disposition, quiescence, no contestation or destination fault, and actual
release of any separate export-destination reservation.

Only the pending hold is deleted and its ledger charge debited. The actual claim
link is consumed and transfer becomes terminal-ineligible. Neither lifetime
issuance counter is decremented; the transfer label fits the existing maximum
claim frame. Root/publication, baseline, producer pins, writer ticket and charge,
codec artifact, knowledge and applied-write evidence remain unchanged.

The control operation needs neither current body disclosure nor a provider call.
Stopped admission and native execution fault do not by themselves prohibit safe
cleanup, as in existing nonapplication/control paths. Current control facts remain
mandatory. Destination quarantine still refuses, including when another ticket
caused the fault while the target itself remains uncontested.

Hold-retention/delivery facts in this operation concern that capacity bookkeeping;
they do not silently discharge the writer artifact, recipient observations or
external effects. Writer retirement is separately gated and still checks actual
current native baseline/history. Without a newer qualifying retained producer or
other established release route, the written evidence remains pinned. A subsequent
successful ordinary save may supersede it, permit actual checked retirement and
preserve the current producer. A single pending-hold ceiling therefore need not
prevent a new save solely because a terminally ineligible promise occupied it.

Later contradictory receipts are still accepted as conflict evidence under the
existing receipt authority rules. They quarantine retained state, not recreate a
capacity promise or manufacture savedness. Replay of the cleanup handle/link cannot
release twice, recycle issuance or bind the discharged claim to a new hold.

## Conditional preservation arguments

For finite normalized inputs, immutable captures, confined producer factories,
truthful current control/effect facts and serialized bounded transitions:

1. A captured synchronized requirement and known failed/not-requested atom make
   qualification false. Joining lower or equal information preserves that atom;
   joining a different known atom yields contestation/quarantine, also excluding
   qualification. Induction over such consistent/conflicting receipt steps cannot
   create a successful baseline for that attempt. Different new attempts are not
   steps of this attempt's evidence lattice.
2. Actual domain/pool/claim/hold ownership and private ticket rechecks prevent a
   caller-supplied failure label from authorizing release. Required disposition,
   quiescence, destination discharge and quarantine guards preserve independent
   effect-lifetime obligations rather than inferring them from synchronization.
3. Deleting one exact hold reduces its charged ledger and future publication-unit
   demand. It changes no native root and adds no competing promise, so remaining
   byte/counter floors cannot become harder to satisfy. Issued, producer references
   and writer/codec charges remain unchanged.
4. Ending the link and removing the hold makes release single-use. Existing writer
   evidence, actual-native-root retirement pins and artifact ownership remain;
   capacity release cannot erase a known applied write or adopt a saved baseline.
5. A later installed qualified producer can satisfy the existing strict serial/
   binding/history retirement conditions for the older failed attempt. Retirement
   still requires its own quiescence, retention and delivery facts, without
   deleting the current producer or recycling consumed lifetime issuance.

These are authored conditional finite-state/lattice/accounting arguments and actual
reference checks, not proof-assistant induction, new SMT conditions, arbitrary
concurrency proof, independent Cleanroom certification or whole-engine closure.

## Executable evidence

annotation-terminal-hold-checks.mjs covers fourteen native annotation variants
across capture, undo and redo, using both incompatible sync atoms: 42 stages.
Each admits the actual ordinary save, records applied-but-unsatisfied evidence,
discharges only its hold, preserves charged evidence, rejects baseline adoption
and premature retirement, then admits and installs a successful replacement and
retires the superseded failed record while preserving the current producer.

Seventy-two fact cells cross nine application/strength/evidence profiles with
disposition, quiescence and stop. All sixteen four-fact release-gate combinations
have exact refusal frames and a positive sibling. Thirty-two actual receipt cases
cross both incompatible initial atoms with nonapplication, lower information at
zero/interior/full observed-byte classes, and written results across all sync atoms
and absent/same/different version classes. They preserve the original known atom
or quarantine contradiction; no case satisfies the requested strength. These
representative classes do not enumerate every byte value or arbitrary history.

Twelve controls cover evidence retention/no adoption; progress under a one-hold
ceiling; foreign/cloned handles; exact owner control without content/provider
availability; cleanup after native fault/stopped admission; later contradiction;
actual destination release for claimed exports; distinct nonapplication cleanup;
bounded transfer metadata and callsite confinement; destination-wide peer
quarantine; stopped-prelaunch abandonment; and explicit synchronized requests.
The export control does not prove native installation of the legacy supplied
export-reservation root or every binding/history route.

Twenty-four assertion-detected semantic mutants follow a passing rewired claim,
pool/native/artifact/codec/writer baseline. They weaken requested-strength, terminal
sync, application, disposition/quiescence, contestation/quarantine, destination and
current control guards; remove discharge/link/hold/debit; recycle issuance; refund
claim storage or erase a writer ticket; relabel capacity discharge as installation;
drop written-evidence retirement pins; unnecessarily block safe control; downgrade
known evidence; or fail to quarantine a contradictory sync atom. The contestation
mutant weakens both redundant attempt/destination guards; peer quarantine is
challenged separately. Import, syntax, tooling and arbitrary runtime failures do
not count as killed semantic mutants.

## Abandonment and remaining integrations

Queued stopped work retains its existing prelaunch discharge path. Confirmed
nonapplication retains its existing disposition/quiescence path. Qualified written
work retains actual baseline installation/current/superseded transfer. Unknown or
contested post-dispatch effects stay held until actual resolution or a separately
specified recovery/retention policy: stop, expiry, process/caller loss and elapsed
time do not establish nonapplication or justify silent promise/evidence waiver.
This closes the terminal-ineligible capacity slice, not unknown abandonment.

All legacy/cross-context/binding/history and retirement routes, annotation load,
public invocation/outcome/events, actual host/policy/package truth, full global/
origin/physical costs, concurrent/crash refinement, supervisor recovery and
independent assurance remain gates. Requirements, numeric limits, public verbs,
components and native job kinds are unchanged. Solver totals remain 234 conditions
across fourteen manifests.

PATH-05/12 and INS-01/02/03/04/07/08/13 distinguish terminal promise discharge,
still-held writer evidence, requested versus achieved strength, actual nonapplication,
qualified savedness, contradictory late receipts and refused cleanup. MELT,
RED/USE/golden and STRIDE/LINDDUN cover admission blockage, stuck/held demand,
premature refunds, orphaned evidence, false savedness, control abuse and private
save-activity correlation. Installed signals, numeric SLOs and timing noninterference
remain unproved. Content, paths, contexts, origins, handles, identities and version
tokens are not metric labels; trusted supervision counts are not recipient status.
