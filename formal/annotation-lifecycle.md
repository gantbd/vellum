# Annotation inventories through document lifecycle

Current integration: §11.2.7 additionally routes scratch creation and registered
view/workspace relations through the actual native owner, conditional explicit
profiles, private closed-delta certificates and existing floors; see
[native-live-lifecycle.md](native-live-lifecycle.md). Native registration, resource/
export/rebind/SaveAs and public products remain open. Earlier exclusions below
retain their original slice scope.

Specification §11.2.4 composes retained annotation inventories with actual scratch
creation, initial source activation/known-owner joins, view/workspace relations,
disposal, binding release and provenance release. New documents acquire known-empty
inventories at construction. Existing inventories are validated and preserved;
missing state never becomes permission to manufacture an empty history.

## Precedent and derived boundary

[Neovim's buffer API](https://neovim.io/doc/user/api/#api-buffer) distinguishes
buffer validity from loaded contents; its deletion operation separately supports
unloading without deleting the buffer. This supports separating owner identity,
content availability and final deletion. It does not establish Vellum's retained
annotation obligations or prescribe Vellum's disposal semantics. Those derive from
the already adopted current-identity, non-reuse and retained-disposal contracts in
§§11.2.2/13.9.1, not copied editor code.

The reference does not adopt Neovim buffer reload as an identity-restoration rule.
Disposed Vellum owners stay closed to mutation, and alias opens cannot resurrect
them. Last-view detachment, authored annotation deletion, document disposal and
record retirement remain different operations with different obligations.

[RFC 8259](https://www.rfc-editor.org/rfc/rfc8259#section-4) treats JSON objects as
unordered name/value collections. Identity and uniqueness therefore depend on the
typed authority/id/incarnation fields, not their insertion order. Inventory checks
use semantic equality; relation/observer keys use explicit field tuples; compound
keys render those fixed fields consistently. This is a private key construction,
not adoption of a canonical wire format or normalization of authored source bytes.
Mixed-order source and annotation participants must share the same edit context.

## Activation and complete inventory

An explicit private membership limit profile, or any existing document inventory,
requires a complete valid inventory on every document in the supplied root. An
empty authority root can establish that profile because it has no unknown existing
documents. An existing unprofiled document, even with an empty current record list,
cannot be upgraded by guessing that it has no retained or retired identities.
Legacy migration and persisted annotation-snapshot activation require their own
explicit evidence and are not implemented here.

Both actual creation paths construct an empty inventory only for the newly
allocated document. They independently validate the input root and completed
candidate against the declared membership root/document/entry/retained/record
budgets, in addition to their existing source/lifecycle budgets. Source activation
does so before setting the task's authoritative activation marker. A refused
candidate consumes no root identity counter, owner row, publication or activation
marker; a later eligible attempt may still activate the retained source candidate.

Known-owner joins and repeated activation validate but do not reinitialize the
owner's authored or retained inventory. Source equality is not owner identity.
The existing codec, resource-key registry and independent source-baseline evidence
continue to determine source activation; this slice does not load annotation data.

Earlier isolated unprofiled fixtures retain their own supplied premises. Their
compatibility is not a second user-facing editing mode or an excuse for a v1
implementation to omit the logical inventory. The explicit fields are private
reference representation, not a new public setting, persistence format or API.

## Relations disposal and retained provenance

View attachment/detachment and workspace membership preserve document records,
all three annotation phases, floors, retained payload and history. They retain
their existing authority/precondition checks; inventory validation adds no
annotation-write grant requirement to a relation-only operation.

Disposal preserves inventory and records even when an authorized discard
disposition ends future mutation. Present means present in that retained document's
collection, not that its disposed owner is still live. The existing live=false
marker fences actual native create/delete/body/target preparation and compound/
history publication. Record retirement also refuses a disposed document in this
profile. There is no implicit annotation phase change, floor reset, identity reuse,
payload purge or inference that all historical references have vanished.

Binding/provenance release preserves the same inventory. Existing actual writer
barriers and independent read/write/history/snapshot/provider discharge premises
still govern release. Final disposed-record reclamation needs a separate complete
retirement planner. Publication growth itself counts against the membership root
budget, so even disposal can refuse before installing a partial tombstone or
relation change. Bounded logical retention is not proof of physical reclamation.

## Conditional preservation argument

Assume a trusted serialized root boundary, authentic current host authorization/
coverage/discharge facts and the stated finite budgets. For an active inventory
profile, prevalidation establishes complete correspondence between current records
and present slots, well-formed absent payload/floors, absorbing retired identities
and aggregate bounds.

Fresh source/scratch construction extends that root with one new document and
empty inventory. No existing row or slot is synthesized or overwritten. Whole
candidate validation establishes the same invariant before publication or read-task
activation is installed. Failure returns the original root and leaves the read-task
activation marker untouched. Join paths preserve root identity and validate before
their task bookkeeping, rather than replacing current authored state with disk data.

Every relation/disposal/release branch copies existing annotation fields unchanged.
Its root prevalidation and final budget check therefore preserve identity phases,
floors and payload by framing, not by interpreting discard as record retirement.
Even an unchanged relation still requires valid incoming inventory. The existing
concrete disposed marker blocks mutation/reversal/record retirement independently
of positive supplied permission flags. This composes with existing version-floor
laws; no new SMT axioms or cross-language refinement theorem is added.

Induction over these reference transitions preserves the invariant under those
premises. Each finite call uses bounded arrays, source-codec steps and existing
closed lifecycle branches; it performs no user callback or search for missing
history. Neither the authored argument nor finite tests prove arbitrary native
execution, hostile decoding, actual concurrency or categorical/lattice completeness.

## Executable evidence and limitations

`annotation-lifecycle-checks.mjs` supplies 7,168 lifecycle gate combinations over
the seven existing operations, 2,048 initial source-activation combinations,
fourteen semantic controls and fifteen rejected mutations. The unmodified dynamic
composition must pass before counting mutants; only assertion failures count.
Independent unchanged validators check candidate inventories, not mutated oracles.

Controls cover root birth, later scratch creation, actual source candidates/joins,
native annotation editing after creation, incoming unknown/inconsistent inventory,
profile downgrade, relation authority/framing, all-phase disposal/release framing,
live-positive versus disposed-negative mutation/reversal/retirement, and final
root/document/entry/retained/record pressure without partial activation.

Native checks use fresh fixture invocation domains for independently installed
roots; they do not prove live lifecycle/invocation handoff or the complete F17
dispatcher. The retired-slot seed is a valid supplied inventory state, not a proved
retirement history. The retirement fence control compares valid live/disposed
fixtures after explicit test-only history release; it does not implement or prove
a history-floor eviction planner. No unbounded state-space closure is claimed.

Identity-order controls reject semantically duplicate documents/workspaces/views/
members, preserve one history frontier across reordered references and map a
before-source annotation target through a source participant with differently
ordered identity fields. Three mutations independently restore order-sensitive
inventory, relation and compound-context checks. The observer's existing context
controls also reject reordered duplicate document/view scopes with an additional
order-sensitive-key mutation; see collection-observation.md.

## Instrumentation and remaining gates

PATH-01/03/04/10/12 and IF-01/03/07/08/12 retain the existing creation, provider,
root/history and release boundaries. INS-08/02 distinguish resolution, validation,
candidate pressure, atomic publication, known-owner join and disposed fencing.
INS-04/07 separate source read evidence/baseline from annotation persistence;
INS-06/13 distinguish held history, discharge and useful completion; INS-03/12
gate current disclosure and optional diagnostics. MELT/RED/golden signals track
offered/admitted/useful completion, refusal/fault and phase latency; USE covers
inventory/retained bytes, relation/root pressure and held provenance. STRIDE checks
invented emptiness, alias replacement, stale mutation and counter/retirement reuse;
LINDDUN checks retained-body/linkage and disposal-status disclosure. Ordinary labels
contain neither private identities/targets/bodies nor hidden collection counts.

Remaining gates: legacy inventory migration, annotation snapshot activation and
point-profile storage, actual host identity/coverage/discharge, full disposed-record
retirement/reclamation, general retained target mapping, full native result/event/
invocation composition and actual concurrent/physical/production/independent
refinement. The all-sections goal and broader lifecycle/annotation families remain open.
