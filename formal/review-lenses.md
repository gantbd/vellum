# Complementary-lens contract review

This is a selected architectural review under requirements §12.5.3, not exhaustive
STPA/FMEA/LINDDUN analysis, certification or installed instrumentation. Existing
contracts are normative. No public outcome schema, scheduler, physical memory
unit, numeric SLO or new runtime is selected. [assurance.json](assurance.json)
links four scoped claims to configured checks, controls, assumptions and limits.
Only a current complete passing [results.json](results.json) establishes that
those checks ran. An evidence link alone never establishes a claim.

## Resources, requests and useful outcomes

USE reviews resources; RED reviews request boundaries. Reuse the existing
[observation index](observability.json) rather than count the same activity twice.

| Resource / contract boundary | Useful questions | Current formal boundary |
|---|---|---|
| Staging/publication, IF-01/03 | Offered/admitted/rejected; preparation versus publication; participants and reserved staging | PS-TX/BR/EC check selected atomic/current boundaries, not physical staging size or timing |
| Save queue/completion reservations, IF-04/13 | Held count and abstract cost; pending/unknown age; actual safe discharge | PS-AC imports PS-SA transitions and checks cumulative count/cost ledgers; fixed one-shot requests and units, not actual metrics |
| Worker-held source, IF-05 | Root/shared holds versus allocations; cancellation versus actual worker release | PS-DR covers two source cells only; result/history/native storage is excluded |
| Result buffers and history, DAT-04/07 | Useful accepted/current results, retained cost, obsolete work and eviction constraints | Acceptance/history properties are partial; comprehensive memory/ownership accounting remains a specification residual |
| Read/event/diagnostic release, IF-02/08/09 | Queued/current projections, unknown/lost/off status, current recipient gate | PS-PV extends PS-DI for one diagnostic profile; PS-EC checks a selected event release; actual sinks/schemas remain open |
| Background/control work, IF-05/13 | Pending versus disposed work, eligible service, oldest eligible age | PS-UP checks a closed two/three-job cooperative backlog under explicit weak fairness; not host service bounds |
| Native/file/display services, IF-04/06/11 | Declared capability, actual observable handoff and unknown completion | Abstract provider/platform assumptions remain; do not infer physical utilization or display from local call success |
| Diagnostic capture/drain, DAT-10 | Bounded pressure, capture loss, consumer status and interference | Existing intent/authority constraints; actual callback cost, nonblocking behavior and overhead require implementation evidence |

The row labels are architectural references, not frozen metric labels. Verify
asset semantics against the catalogs before assigning concrete accounting units.
Per-operation identities remain private bounded correlation, not metric series.

Candidate outcome contracts, without targets:

- Edits: separate logical offers, valid eligible requests, admitted operations
  and applied/unchanged outcomes. Capacity shedding and prolonged pending work
  remain visible; valid permission/conflict rejection is not an invariant fault.
- Persistence: distinguish captured logical saves, physical attempts, safe
  disposition and success-backed exact receipts. Unknown/timeouts are not success
  or confirmed failure; pending ages remain visible. Late evidence stays scoped.
- Analysis: distinguish submitted work, completed work, current accepted results
  and superseded/cancelled outcomes. Dependency coverage remains declared complete
  within scope, partial, unsupported or unknown—not inferred from acceptance.
- Control: observe eligible backlog and actual dispositions under stated host
  service assumptions. Eventual drain does not imply every job was applied, a
  numeric response bound or fairness under continuous arrivals.

PS-AC's ghost counters are a specification instrument, not a telemetry-derived
ledger or independently measured production SLI. PS-UP terminal outcomes partition
identity-level jobs; cancellation is intent until actual service disposition.

## Privacy review — LINDDUN

| Perspective | Architectural risk / constraint | Evidence and residual |
|---|---|---|
| Linking | Stable correlation or activity sequences join private identities/documents across scopes | PS-PV detects deliberately exposed differing private tags in a fixed paired profile; universal unlinkability/correlation lifetimes remain unproved |
| Identifying | Identity can be inferred from fields, content, paths or small aggregates | Existing allowlisting and scoped projection apply; PS-PV tests one explicit tag, not statistical reidentification or anonymity |
| Unwanted non-repudiation | Persistent attribution can itself harm privacy even when integrity is correct | Balance necessary authoritative evidence against minimization; no mandatory globally attributable audit is introduced; policy/retention detail remains open |
| Detecting involvement | Hidden existence/count/activity can be inferred from response, cache or pressure | PS-DI/PV compare paired projections and detect hidden count exposure; same public schedule only, no timing/resource side-channel theorem |
| Excessive data disclosure | Authorized collection can still retain/process too much | Allowlist before capture and gate new drain/release. PS-PV checks regrant/disposal cases; general field policy, disposal and physical erasure remain open |
| Unawareness / unintervenability | Host or user cannot understand or control diagnostic collection and disclosure | Explicit host configuration/disclosure direction applies; actual UI/control conformance is planned, not modeled |
| Non-compliance | Data lifecycle/governance does not meet declared obligations | Record purpose, scope and retention policy for later contracts; no jurisdictional legal-compliance or certification claim |

PS-PV permits only a fixed public health class and constant scoped correlation;
hidden identities/existence are not permitted operational disclosures in that
profile. This is compatible with §15.6's broader explicit disclosure limits, not
a promise to hide all operational interference. It does not revoke bytes already
delivered. Imported revocation also changes the epoch; isolated grant-check
sensitivity cannot be inferred from its epoch-bypass control.

Revision 0.42 strengthens the delivery witnesses after a fresh separate-agent
challenge: grant, disposal, epoch/captured epoch, hidden divergence and output
kind are frozen at drain and checked for stability/output agreement. Later core
changes cannot retroactively satisfy the gap/disposal/masking witnesses; masking
requires authorized Health delivery, not merely equal denials. Four scripted
regressions with endpoint witnesses cover those false-positive paths. The earlier
saved traces already had the intended order; this closes regression sensitivity,
not a newly discovered unauthorized-release defect.

## Interaction losses, unsafe controls and representative failure modes

Selected losses are authored-data loss/incoherence, unauthorized disclosure,
false persistence assurance, stranded obligations and misleading diagnostic or
service conclusions. Controls must address missing actions, unsafe provision,
incorrect timing/order and overlong/too-early cessation where relevant.

| Interaction / failure | Constraint and detection/containment | Evidence / remaining work |
|---|---|---|
| Cancel/timeout mistakenly discharges a running effect | Retain reservation/evidence until actual safe disposition; expose unknown rather than retry as if quiescent | PS-AC premature/double/wrong-cost controls; existing PS-SC/SE/DR controls. Real worker/provider truth remains assumed |
| Missing admission or terminal bookkeeping | Offers partition into admission/rejection; admitted reservations balance actual discharges; pending and terminal jobs partition identities | PS-AC missing-admission and PS-UP silent-drop controls; no claim of general physical-resource accounting |
| Regrant or policy change between prepare and release | Gate each new release on current grant/disposal/continuity; cache preparation is not permission | PS-PV epoch-bypass control and gap/disposal witnesses; existing PS-DI/RB/EC checks; arbitrary subscriber/sink profiles remain open |
| Wrong recipient correlation or private aggregate | Minimize before capture; scope correlation and projection; diagnostics cannot grant authority | PS-PV private-tag/count controls; no statistical/timing proof or generic support-bundle implementation |
| Stale derived result reported as useful success | Classify stale/superseded separately and check acceptance-time context | PS-UP stale-success control plus PS-DR current-evidence gates; boolean context is not a semantic dependency oracle |
| Cancellation treated as job completion, or silent removal | Cancellation intent alone leaves pending obligation; service gives one stable terminal disposition | PS-UP cancellation-discharge and silent-drop controls, with cancelled-but-pending witness |
| Service omitted indefinitely | Finite pending rank decreases on actual service; make host service assumption explicit | PS-UP eventual drain under weak fairness; removing fairness gives an infinite counterexample. No bound on response time or open workloads |
| Consumer blocks/reenters during publication | Capture must not call arbitrary downstream code; contain diagnostic failure outside publication | SCN-08 remains planned; no executable capture/consumer implementation exists to inject faults into |
| Cut deletes source before export is acknowledged | Preserve source on failed/unknown export and renew source/permission checks | SCN-03 explicitly plans two-boundary clipboard tests; no new clipboard proof is claimed |
| Duplicate/contradictory/late effect evidence | Preserve attributable facts, reject false success and do not erase unresolved obligations | Existing save/dispatch/storage/recipient controls; arbitrary attempts and real provider fault integration remain residual |

These are abstract fault controls, not production fault injection. None permits
automatic remediation outside ordinary capability/current-context validation.
Safety/disclosure invariants are never paid for with an SLO error budget.

## Claim argument and proof boundaries

- ACS-01: PS-AC restricted imported-admission count/cost conservation.
- ACS-02: PS-PV fixed-profile privacy and current diagnostic release.
- ACS-03: PS-UP conditional temporal drain; explicit service fairness is a premise.
- ACS-04: PS-UP identity partition and current useful-outcome classification.

Evidence references must name a positive configured check and an actual checked
property; negative controls/witnesses retain their distinct expected types.
Assumptions, conditions and limits cannot be omitted. SHACL/generator sensitivity
checks establish structural consistency only. Evidence validity additionally
depends on trusted tools, current source hashes and recorded outcomes. This
argument graph is not independent review, a deductive proof kernel, a complete
Cleanroom process or exhaustive architectural coverage.
