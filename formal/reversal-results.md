# Native reversal decision receipts and current disclosure

Specification §14.6.1 selects the normalized F14 decision result over actual owned
undo/redo and the shared invocation ledger. Capture a small immutable receipt with
the root/coarse-fact decision; never reconstruct an old reversal outcome from the
current frontier. The historical result is separate from F04 current advice and
does not reserve or authorize another reversal.

## Precedent and selected product

[Neovim undo blocks](https://neovim.io/doc/user/undo/#undo-blocks) group a command's
changes into an undo step. This supports the existing command/group boundary, not
Vellum's linked-object authorization, result grants or receipt implementation.
The receipt mechanism reuses the already adopted §6.5 ledger and §13.6.4 bounded
decision-cache pattern; it adds no transport, database or external execution effect.

The outer normalized result is exactly reversal-result plus a result, or common
unavailable. It is not a selected JSON/Rust/wire format. Known inner decisions are:

| Decision | Mandatory facts | Optional protected fact |
|---|---|---|
| applied | status, original invocation, undo/redo operation, causal publication reference | exact original group or scope-frontier target, under captured and current target disclosure |
| not-published | status, original invocation and operation | none; no candidate target, publication, participant, version or existence facts |

The publication reference is the once-only reversal invocation's causal anchor,
not the original authored group, raw root counter, history context or read selector.
Applied states a historical completed reversal; it does not report current eligibility,
frontier position, document liveness or redo availability. There is no F14 unchanged
result: that would be a surprise, not a fabricated successful no-op.

The mandatory product is permitted only through the original/current F06 publication
ceiling, epoch, complete invocation context and common current release gates. Target
disclosure is not inferred from write authority. In contrast to F13's identity-bearing
record product, the minimal F14 decision is useful without a target field; absence
of a detailed grant therefore retains the minimal decision, not target disclosure.
Pending, unknown/faulted, forgotten and retired observations are unavailable, not
guessed not-published. Reply loss after publication cannot change the decision.

## Target disclosure and ownership

Admission accepts an optional exact trusted reversalResultGrant containing invocation,
authority, the original typed target and a Boolean discloseTarget. Bind all of them
to the owned request and check recipient/execution-profile/authority agreement.
Malformed, substituted or extended grants refuse admission; absence captures false.
The ceiling is copied/frozen. Matching retry does not replace it.

For applied decisions, current release requires any supplied grant to bind the same
invocation/authority/original target. Emit target only if both captured and current
capabilities are true. An absent current grant means false; malformed current grants
make detailed observation unavailable rather than accepting substituted scope. The
Boolean capability is a two-element bounded distributive lattice; release uses meet,
not join or an engine-wide completeness argument. This independent target product
does not enable raw participant details in F06.

The original target contains the already selected group alias, or scope alias and
expected group. These are historical request references, not private internal group
IDs, newly issued F04 selectors or a promise that the aliases still resolve now.
Current grant truth must permit their historical disclosure. Closing the old history
context does not rewrite the receipt or recreate its alias binding. Later commands
must use current valid bindings and ordinary complete F14 preparation/authority.

Owned admission goes through the same native reversal producer's private provenance
map as preparation/publication. A frozen structural clone is not an owned plan.
Complete invocation/intent equality and spent eligibility remain the shared ledger's
responsibility; neither cache loss nor returning to equal frontiers makes an old
invocation fresh.

## Capture and reservation

Before pending execution reserve the complete private receipt and largest permitted
public frame against maxResultBytes, including the original target even if withheld.
The full private receipt stores copied decision scalars, authority and original target
only. It retains no root, source bytes, inverse journal, participant graph, view,
authored version, private group ID or raw publication counter.

The existing per-slot five-result-maxima logical charge remains sufficient: three
coarse policy/result/frame units and one operation-specific private receipt/release
pair. Annotation and reversal jobs are mutually exclusive. Predeclared slot fields
do not create two retained receipts for one invocation. The common before/candidate
root, context and four-intent-maxima reservation is unchanged. This is logical
accounting, not heap/RSS, transient physical peaks or concurrent reservation proof.

After actual native candidate validation and the outer root cap, capture either
minimal confirmed nonpublication or the original applied decision. An applied
receipt checks the same retained original group and every actual participant's
movement to the opposite-direction frontier. Do not validate only the visible scope
or prepared candidate before final outer-root refusal. The publisher still owns the
actual complete reversal/content/view checks; the receipt is not a second publisher.

Validate closed fields and size before final installation. Trusted assignments pair
the root, coarse fact, native receipt and done phase. Missing/malformed/oversized or
unexpected receipts fault/fence with Unknown and no receipt, not confirmed refusal.
Final assignments contain no host callback. Actual concurrent/native installation,
physical allocation and crash persistence are separate gates.

Later native work, history reversal, observer/query closure or a later domain fault
cannot change the retained receipt. Current release still rechecks permission and
the original coarse ceiling. Forget clears coarse and operation-specific receipt
caches only after drain/retention conditions; retirement removes the owned ticket.
Neither resets issuance, rebinds the target or reexecutes the native evaluator.

## Conditional arguments and executable evidence

Assume the existing serialized owner, truthful initial/native history and alias/grant/
retention facts, finite admitted limits and actual reserved capacity. Owned preparation
and admission bind the original operation/target; actual native publication validates
the complete group. Final root-cap resolution and receipt validation establish a
matching historical local decision before paired installation. Confirmed refusal has
no candidate facts; a protocol surprise yields Unknown instead.

Induction over later edits/history/queries/forget/retire preserves the installed
receipt or removes its availability, never changes it to a current evaluation or
restores spent eligibility. Captured/current target meet prevents amplification;
the independent coarse/context/epoch gates still bound the mandatory result. This
is an authored conditional argument over selected serialized transitions, not a
proof-assistant-checked implementation induction or independent certificate.

Native receipt capture/release runs bounded owned field and participant scans under
the existing root/work profiles and closed undo/redo vocabulary. No parser, network,
external effect or arbitrary callback is added. Finite reference execution does not
prove hostile decoder termination, native scheduling or physical halting.

reversal-result-checks.mjs composes actual native annotation creation/body/reattachment/
deletion with all six target forms, owned history contexts, both F14 target forms,
undo/redo and the actual shared ledger. It checks 224 captured/current receipt cases,
512 release cases, thirteen controls and fourteen assertion-rejected semantic mutants,
with a passing unmodified dynamic baseline. Controls also cover actual two-document
source reversal, independent context closure, original-versus-current facts, malformed
grants, private ownership, preadmission capacity, outer-root refusal, fault/Unknown,
forget/retire, once-only retry and copy isolation. Load errors do not count as kills.
This is selected integration, not exhaustive reachable histories or all companions.

Six local scoped-release conditions cover target meet, original scope, historical
direction and known nonpublication, with SAT controls for unsafe current-only grants
and inferring refusal from unknown/lost state. Current totals are 27 scoped conditions
and 217 suite conditions across thirteen manifests. These concern declared predicates,
not automatic JS/TLA+/Rust refinement or host truth.

## Instrumentation and remaining gates

PATH-01/02/03/04/12 and IF-01/03/08/09/12 distinguish reversal preparation, publication,
receipt capture, current result release, historical advice and cache retirement.
INS-01/02 track decisions and paired receipt installation; INS-03 release/withholding/
eviction; INS-06/08 retained history/context ownership; INS-07 discharge; INS-12
diagnostic loss; INS-13 useful completion. Private invocationUsage adds reversal
receipt count/serialized bytes; those are supervision data, not public F06 fields
or physical measurements. Check annotation/reversal receipt exclusivity per slot.

MELT/RED/golden signals distinguish applied reversal, confirmed refusal, Unknown,
withheld/lost receipt and phase latency. USE distinguishes root/intent/receipt/slot
pressure. STRIDE challenges substituted grants, wrong direction, missing participant
validation, false receipt publication and replay; LINDDUN challenges historical alias
linkage, private graph/content leakage and misattribution of old results to current
history. No raw target aliases, invocation IDs, roots, counters or content in ordinary
metric labels. Observation does not grant another action.

Public wire/history-alias bridges, actual grants/aliases/retention, full retirement/
general target restoration/point persistence, other family/effect result products
and physical/concurrent/native/decoder/production/independent refinement remain.
The selected F14 native decision product is no longer missing, but no entire family,
D-row or all-sections goal is closed by this receipt slice alone.
