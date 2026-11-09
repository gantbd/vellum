# Native view and workspace container lifecycles

Current invocation integration: §11.2.12 adds owned view/workspace lifecycle
invocation, current native staging, cancellation/replay and bounded current identity
result release; see [relation-invocations.md](relation-invocations.md). Remaining
F17, registry/event/remote products and host/physical/independent gates remain.
Earlier exclusions retain their slice scope.

Specification §11.2.11 supplies logical container creation and retirement at the
existing local native owner. Documents, resource bindings, retained producer work
and presentation allocation have separate lifetimes. Removing the last view or a
workspace with members never disposes those documents or claims persistence.

## Precedent and architectural derivation

[Neovim](https://neovim.io/doc/user/windows/) distinguishes a buffer from its
viewports and assigns windows session-stable identifiers. [Emacs window deletion](https://www.gnu.org/software/emacs/manual/html_node/elisp/Deleting-Windows.html)
separates removal from the display tree from the lifetime of referenced window
objects. Neither editor's focus, fallback-buffer, last-window, callback, layout,
identifier encoding or undo policy is adopted. Workspaces remain Vellum's logical
cross-domain groupings, not copies of editor tabs, projects or filesystem roots.

Requirements §§3.4/3.7 already require separate view, membership, ownership,
residency and outstanding-operation dimensions. The existing native lifecycle can
attach/detach views and change membership only after their rows exist. This slice
closes the native logical container birth/retirement gap without changing document
disposal or introducing a new public invocation/job or presentation dependency.

## Closed control and publication

The trusted product is exactly {kind: view | workspace, action: create}, or
{kind, action: retire, target, expected}. View retirement expects the exact current
attachmentGeneration, selectionGeneration and viewportGeneration. Workspace
retirement expects its current generation. Unknown fields,
caller-assigned identities, foreign/missing targets and stale expectations refuse.
Missing targets are unavailable, not authorization-free idempotent success;
invocation replay is a separate layer. No implicit current view/workspace is used.

Creation uses the existing identityAllocation namespace/next/last allocator, with
the existing identity triple representation and incarnation 1 at birth. It checks
safe increment, bounded scalar UTF-8 labels and collisions against current document,
view/workspace and retired-document identities before consuming next once.
No scan-to-next-free, wrap, rewind or reset is allowed. A new view is detached, with
sourceRevision 0, empty selections, primary 0, explicit-positional-selection/1 and
attachment/selection/viewport generations 1. A workspace is live with generation 1
and no members. Creation does not attach, select, focus, clone or join a document.
Birth constants describe known-empty state, not defaults for unknown imported rows.

Retirement removes the exact addressed row, including its logical attachment or
membership edges, in one publication. It can close an attached last view or remove
a populated workspace without first emptying it through intermediate publications.
It does not alter its documents, owned text/annotations, history, bindings,
resource owners, saved baselines, resource registration, producer ledgers or peer
containers. It does not move focus, recall disclosed copies, stop workers, unload
provider code or reclaim their retained state. Independent retained evidence and
charges survive under their owners; container absence is not a discharge receipt.

Twelve strict shared facts cover authorization/scope/continuity/liveness/stop,
resources/companions/versions/reply, owner control/preparation and complete relation
coverage. Creation adds creationAllowed; retirement adds retirementAllowed and
retainedConsumersSafe. The latter is a current host/companion premise about safe
fencing and retained ownership, not an observation that all old work has stopped.
Missing, numeric or truthy substitutes refuse. The existing exact lifecycle,
membership and binding profiles bound request/root/row/tag domains; actual native
root/reply caps and current annotation-baseline promises also constrain publication.
No new numeric envelope or host/platform default is selected.

Private certificates bind the actual native owner/domain/before-root and frozen
request/profile. Final validation reconstructs candidate and copied result and
rechecks current facts, envelopes and promises. Successful publication freezes the
root, consumes the certificate and advances publication once; creation alone
advances identityAllocation.next. Existing committed-source journals carry without
reissuance. Rejection leaves root, allocator and producer accounting unchanged.
There is no I/O, arbitrary callback or await inside this boundary. Inspector/raw
certificate descriptors are trusted composition/proof instrumentation, not public
handles or disclosure APIs. The returned {status: created | retired, kind, target}
is an internal semantic result; scoped public reference encoding remains separate.

## Conditional preservation arguments

Assume a supported known initial native lifetime, truthful current host/coverage/
retention facts, existing serialized native-root composition and no out-of-band
allocator or container mutation. Initial and future identity allocation must not
adopt unknown prior lifetimes. Imported/legacy registry recovery is not established.

1. Each native creation consumes the current bounded allocator coordinate exactly
   once. Existing source/scratch birth and unrelated publications preserve its
   namespace and never decrease next. Retirement preserves next. Induction over
   this supported native lifetime gives no reuse of a natively issued container
   identity after removal. Finite counter exhaustion refuses rather than recycling.
   This is not a proof of arbitrary imported identifier history or durable recovery.
2. Exact owner/domain/before-root ownership and complete candidate reconstruction
   exclude peer-content, ownership, producer or allocator substitution. Current
   request expectations exclude stale retirement. Both roots obey declared limits,
   and actual native promise validation prevents spending capacity promised to a
   later baseline publication. The synchronous commit is locally all-or-nothing
   under its preparation/resource premises, not allocator or crash atomicity.
3. Created views/workspaces compose with the existing native attach/membership
   products. After retirement, current lookup fails; a new creation cannot restore
   that identity. Actual old selection publication and membership mutation refuse.
   The control touches no producer store, so it does not itself cancel work,
   rewrite historical evidence or refund retention. Complete provider/UI/config/
   observation consumer fencing still requires their actual contracts and host truth.
4. Two kinds and two actions form a closed vocabulary. Every call finishes through
   finite bounded scans/copies/validation and one commit or refusal. This proves
   conditional per-call halting, not provider termination, whole-engine categorical
   completeness or a lattice for arbitrary container replacement/removal.

These are authored conditional construction/induction arguments, not proof-assistant
induction or independent Cleanroom certification.

## Evidence and instrumentation

relation-lifecycle-checks.mjs checks 42 annotation/history stages, 252 actual native
publications, all 49,152 factorized gate vectors, 162 strict-value refusals and
thirteen controls. Twenty semantic mutants exercise allocation, retirement frames,
expectations, owned/current/spent certificates, exact candidate/reply reconstruction,
native caps/promises/faults, source journals and strict authority. Each must fail
assertions against a passing coherently rewired multimodule baseline; import,
syntax and TypeErrors do not count as kills. Existing 234 SMT conditions/fourteen
manifests remain unchanged. Finite test coverage is not general induction.

IF-01/03/06/07, PATH-01/03/05/10/12, DAT-01/02/03/07/09 and
INS-01/02/03/05/07/08/09/10/12/13 cover create/retire admission, capacity,
publication/refusal, stale consumers, retained work and privacy-safe observation.
MELT/RED/golden signals distinguish queue/preparation/publication/release latency,
logical controls from attempts and container removal from actual work discharge.
USE covers row/identity/request/root/reply and retained-consumer pressure.
STRIDE/LINDDUN challenge forged identity, stale expectations, surprise cascade,
premature refund, counter exhaustion and cross-scope activity linkage. Use bounded
kind/action/phase/reason categories, not IDs, generation counters, names, handles,
paths, membership cardinalities or contents as ordinary telemetry labels.
Actual installed signals, performance bounds and privacy-safe public projections
remain implementation/measurement and contract gates.

Public lifecycle/discovery/invocation/event products, command/provider callback
registration, imported/legacy/contested recovery, full retention/consumer ownership,
all host/provider/policy truth, global physical accounting, real concurrency/crash
refinement, frontend allocation and independent assurance remain. No whole
D20/D28/D33/F17/all-sections goal closes. Earlier per-slice exclusions retain their
historical scope; native logical view/workspace creation and retirement are now
covered here rather than assumed preinstalled.
