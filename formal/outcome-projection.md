# Layered outcomes and safe explanations

D05 adopts requirements §4.4 and specification §23.1.1's separation of truthful
authoritative outcome facts from current recipient-safe explanations. Projection
governs the whole report, not only message text. Hidden causes, participants,
counts, correlation and recovery hints cannot become alternative existence oracles.
Status unavailability is not nonapplication; observation failure cannot undo a
known publication or discharge an unresolved external effect.

## Conditional projection argument

For recipient profile r, let V_r contain the permitted facts, including any
expressly allowed operational disclosures and implications of recovery guidance.
The released explanation is a deterministic bounded function F_r(V_r), not a
function of raw hidden causes or their evaluation order. Consequently equal V_r
under the same profile produces equal explanations. This is the relevant
noninterference/quotient argument, not a theorem that every arbitrary mask or
provider implementation enforces a valid policy. Field policies must account for
dependencies: allowing a recovery hint may itself disclose a milestone or safety
basis even when the full evidence record is withheld.

Filter permitted observed reasons first, deduplicate their declared safe classes,
then apply the operation/profile's bounded ordering and secondary-detail limit.
Changing only hidden reasons, their multiplicity or their container order does
not change this permitted reason set. No additional protected check is required
to manufacture a complete diagnostic set. Determinism is conditional on the same
permitted observed facts, not identical outcomes for arbitrary execution schedules.

Authoritative facts frame under projection/release. Delivered reports are owned
immutable observations; later evidence may produce a different new authorized
report but cannot rewrite an old one. Current recipient/grant/lifecycle checks and
the captured disclosure ceiling govern queued release. A failed release returns a
safe unavailable observation, not an invented publication/rejection fact. Absence
of status similarly reveals no historical nonapplication. Safe recovery guidance
requires both its supported verified basis and permission to disclose the guidance;
it does not confer execution authority. These are authored conditional arguments,
not proof-assistant replay, a complete information-flow type system or a provider
safe-retry proof.

## Finite reference checks

`outcome-projection.mjs` uses nine explicit publication/control/effect templates
and 64 Boolean field-projection profiles. It checks 9,216 comparisons varying
hidden reason values, presence/multiplicity, participant IDs and internal trace
values; 3,456 reason-order permutations; and 3,456 release cases for revocation,
epoch/recipient change, absent status, capture-ceiling broadening and narrowing.
It frames authoritative/queued facts and freezes reports. Visible publication and
effect facts remain truthful, including committed edits with failed saves and
unknown outcomes with ended caller observation. Visible participant count means
only visible participants; withheld/unavailable count is null rather than zero.

Nineteen direct sensitivity controls cover first-failure leakage, hidden top-level
reason distinctions/counts, private participant counts, false zero, raw correlation,
retroactive report mutation, save failure relabeling an edit, unknown/absent status
called nonapplication, blind retry, reasons treated as retry authority, dedup versus
new proposal, hidden recovery bases, invalid basis/code/product admission and
correlation aliasing through report freezing. Correlation must be a primitive
string in this fixture; rejected object inputs retain their mutability rather
than being frozen through the report. This selects no public correlation schema.
These are selected fault examples, not general mutation adequacy or independent
Cleanroom evidence. The two fixed visible reasons plus at most two hidden reasons,
two participants and two returned reason classes are private fixture bounds, not
adopted numeric limits. Missing or unknown runtime facts are not filled from logs.

`projectOutcome` is a trusted-side pure projection helper with assumed valid
recipient context, not a public release operation. `releaseOutcome` is the gated
reference release path; `prepareOutcome` copies/fixes private authoritative fixture
facts and a capture ceiling. It is not a proposed telemetry buffer retaining raw
diagnostics, an in-process sandbox or a proof of callbacks/serialization/native I/O.
Same-author construction, structuredClone/freeze, code validation and the test
enumerator remain trusted. Reason visibility and verified retry bases are supplied
premises rather than implemented authorization or provider attestations.

Private labels, one fixed precedence list and template combinations are not the
final public codes, operation-specific precedence tables, full outcome product or
adopted recipient schemas. Observed reasons in the fixture may describe ancillary
or postpublication conditions; they do not assert that a failing publication gate
can coexist with that same gate admitting a commit. The product validator rejects
selected impossible combinations, not every invalid v1 execution state. Actual
reason applicability/phase/context and complete product validity remain refinement
obligations. A profile allowing recovery guidance explicitly permits its causal
implications; hiding an effect field alone is not a no-inference guarantee.

`observe-existing`, `prepare-new-proposal` and `retry-protected-effect` illustrate
different supported bases; they do not restart a rejected/stopped invocation or
promise current host guarantees. A verified nonapplication basis is admitted only
for known nonpublication with no effect in the fixture. The actual D03 lifetime,
provider conditional/idempotent protection, identity and current mutation grants
are not established here. Unknown effects without such a basis require reconciliation,
not a generic automatic retry. Suggested actions still enter normal validation.

## Local symbolic checks

`outcome-laws.smt2` has five positive conditions with SAT premises then UNSAT
negated conclusions: projected-tuple noninterference, reason-filter idempotence,
current release gating, observation/publication framing and recovery-basis gating.
Four SAT controls expose raw-reason distinctions despite equal permitted sets,
hidden top-level kind, denied observation coexisting with applied publication and
an error reason without a retry basis. Reason sets use arrays and a disclosure
intersection; full bounds, sorting/truncation and real policy implication closure
are not encoded. The frame and eligibility definitions are abstract contracts,
not implementation proofs. The exact ordered `outcomeCases` manifest uses trusted
Z3 4.15.4 without independent certificate replay.

## Interfaces observation and residuals

Existing IF-01/03 and INS-01/02 retain authoritative admission/publication;
IF-02/08 and INS-03 retain current projection/release. INS-04 retains actual effect
evidence and INS-12 diagnostic loss/health, not a replacement outcome ledger.
Count logical invocations, projection attempts, successful/withheld/unavailable
releases, dropped observations and provider failures separately. Observe bounded
projection/reason-selection cost and queue/status pressure without raw reasons,
private policy IDs, paths, correlation tokens or participant-valued metric labels.
STRIDE/LINDDUN include forged status, outcome relabeling, diagnostic repudiation,
reason/count/action linkage, stale rich caches, replay floods and hints promoted
to authority. Existing capture/current-drain/resource contracts remain in force.

Actual result products/codes, per-operation precedence, scoped field/token/recovery
policies, safe timing/operational disclosures, retained queues, clock/identity
truth, complete release surfaces and provider/implementation refinement remain
D05/D09/D12/D23/D33/D36–D42 work. Existing PS-DI/RB/PV models are separate scoped
evidence, not complete composition with this helper. No constant-time, physical
resource secrecy, durable status, exactly-once or cryptographic guarantee follows.

Mechanism precedents are [Emacs error conditions](https://www.gnu.org/software/emacs/manual/html_node/elisp/Error-Symbols.html)
and [gRPC typed error categories](https://grpc.io/docs/guides/error/).
[RFC 9110's existence-hiding rule](https://www.rfc-editor.org/rfc/rfc9110.html#section-15.5.4)
supports safe concealment, not our entire outcome/privacy model. No HTTP transport,
status mapping, gRPC dependency or policy engine is adopted.
