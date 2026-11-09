# Superseded annotation read discharge and attachment detach

Current integration: §16.5.16 separately retires original annotation reads and
ordinary acknowledged-write producers after actual native provenance release,
under actual work/known-consumer/retention checks; see
[disposed-annotation-producers.md](disposed-annotation-producers.md). Remaining
all-profile lifecycle/export/rebind/history and contested-recovery gates persist.
Earlier exclusions below retain their original slice scope.

Specification §16.5.15 permits logical retirement of an obsolete producing read
after an actual acknowledged checkpoint, and separately detaches unused writer
relations. It preserves authored state, the checkpoint producer, current/retained
baseline dependencies, live launch obligations and lifetime issuance. It does not
complete disposal/rebinding, contested-baseline recovery or physical reclamation.

## Precedent and ownership boundaries

The direct implementation precedent is the existing source-read producing pin and
the annotation writer's independent disposition/quiescence/retention rules. The
[Linux RCU overview](https://docs.kernel.org/RCU/whatisRCU.html#rcu-overview)
distinguishes removing references from reclaiming objects still used by readers.
[Rust shared ownership](https://doc.rust-lang.org/std/sync/struct.Arc.html)
similarly ties value lifetime to remaining owning references. These support the
ownership separation, not selection of RCU, Arc, a language, a grace-period timer
or a physical-memory proof for Vellum.

Every writer attachment now has one actual bidirectional relation: the writer's
domain→read-owner entry and the reader's writer→relation entry. Bound both sides'
row counts and nominal relation size before assignment. Each side independently
reserves its existing maxCaptureBytes against its own held-byte limit. Reader
tasks compete with those relation charges in the actual reader ledger; there is
no uncharged reverse map or attachment bypass around reader headroom. Current
exact attachment allocates/charges nothing again; inconsistent half-links refuse.

The relation object itself is a fresh private incarnation. Prepared writer guards
bind that exact object, so detach/reattach of the same handles cannot revive an
old preparation. This is local ownership continuity, not a credential. Both maps
commit in one serialized reference transition with no callback or await; production
concurrent/crash installation and allocator behavior remain refinement gates.

## Checkpoint and actual consumers

Producing-read retirement obtains the actual native root and an actual registered
writer's bounded private checkpoint. The writer reconstructs the current
acknowledged baseline from its real retained ticket/codec artifact, not supplied
bytes or a kind string. Private producer/domain/capture/artifact/knowledge/serial
identities remain checked. The checkpoint must be qualified, uncontested and not
quarantined, with actual disposition and quiescence. Its full ordinary annotation
binding must match the consumed read's binding, and its source revision/annotation
set must still name the current live native state. Valid native transition/version
invariants connect those identifiers to the actual captured authored state; this
is not a claim that version labels alone prove arbitrary byte equality.

A current loaded baseline cannot be discharged by a timer or an acknowledgment
label. Scan the declared current and retained annotation-baseline slots across
the actual bounded root. Named references to the original read block release.
Unknown baseline/history/lifecycle shapes and inconsistent duplicate identities
refuse, including reordered identity-object keys. A missing history field means
the accepted initial normalized profile's known empty history under complete
coverage, not migration of arbitrary legacy roots. Retained-slot fixtures check
these fences; they do not implement the complete native binding-history producer.

Inspect every actual known writer relation, not only the checkpoint writer.
Destination quarantine and captured loaded-baseline dependencies block release
while work is still launchable or unresolved. An absorbing stopped, unlaunched
ticket no longer needs its launch premise. A dispatched dependent requires actual
known application, disposition, quiescence, destination release and no contestation.
Original captured baseline data and historical writer outcomes remain unchanged;
settled writer evidence is not erased when its old read dependency is discharged.

Fourteen exact-true current facts independently cover owner control, retirement,
retention/delivery/resource readiness, binding/writer/baseline/retention coverage,
and history/snapshot/read/write/provider evidence discharge. Their subject is
the original read's remaining obligations, not permission to discard the current
writer checkpoint. Unknown external consumers must remain covered by their real
owners; host discharge truth is a premise, not a result of setting booleans.
Those facts never override named actual-root/writer references or actual work.

## Retirement and detach transitions

After the checkpoint and all known-consumer fences pass, actual reader retirement
removes that task/transcript/candidate charge. Remove its retained restoration and
adoption records in the same serialized reference transition. Keep lifetime origin
issuance and absorbing application-consumed identity. Old tasks cannot restore,
adopt or issue their origin again; old historical result inspection becomes
unavailable after authorized retention discharge. Delivered copies are not recalled.
No native root, publication counter, authored record, version floor, group, binding,
baseline, writer receipt, writer claim or codec artifact changes.

The original read may be contested after an independent valid checkpoint was
installed; that does not undo the independent write. Current checkpoint contestation
or known destination quarantine instead refuses release. Cleanup can run after a
native execution fault, stop, disclosure revocation or provider unavailability
under its separate current owner-control facts. It needs neither content disclosure
nor a spare publication generation. Dirty state requires a fresh checkpoint first.

Detach is a separate checked transition. It requires no retained producing-read
records, no declared root reference and no live captured-read dependency across
known relations, plus the same current discharge facts. Remove exactly one writer
entry and its corresponding reader entry/charges once. Detach does not retire
unconsumed read work or refund issued origins, writer work, claims or codec data.
Future loads must reattach explicitly before adoption, using a fresh relation;
current independent acknowledged-write comparison remains valid without the old
reader attachment. No reopening of the native read registry or identity recycling
is introduced.

## Conditional arguments and finite-state scope

For confined factories, immutable valid native roots, finite normalized ledgers,
truthful current host discharge facts and serialized transitions:

1. Bidirectional relation construction checks both ledgers before assignment;
   each task/attachment competes in its actual held budget. Detach subtracts only
   its two known charges and never reduces lifetime issuance.
2. A checked actual checkpoint plus absence of known baseline/live-launch
   dependencies removes the original read's modeled role. Additional consumer
   obligations require their independently true discharge facts. Removing only a
   convenient visible subset is not admissible release.
3. Reader quiescence and its existing retirement gates precede task removal.
   The original authored root and current checkpoint producer are framed; a
   superseded producer's retirement cannot be reported as undo or write failure.
4. Application consumption is absorbing even when its retained result disappears.
   Relation incarnations similarly prevent old preparation from becoming current
   after detach/reattach. No task or guard obtains authority from a copied label.
5. Each call scans bounded root, relation, ticket and artifact domains. No provider
   callback, time-based reclamation or interpreter enters the final transitions.
   Reactive engine lifetime, provider completion and wall-clock service time are
   not proved to terminate.

For a fixed known consumer set C, dependency sets form the bounded powerset lattice
P(C); release requires its empty known dependency set plus independently discharged
outside obligations and a valid checkpoint. This limited set algebra does not
prove that all future consumers are known, that all editor states form one lattice,
or that physical storage is reclaimed. These are authored conditional arguments
and executable witnesses, not proof-assistant induction or independent Cleanroom
certification. Solver totals remain 234 conditions across fourteen manifests.

annotation-read-discharge-checks.mjs exercises 42 native variant/capture/undo/redo
stages through read, baseline, checkpoint, retirement and detach; all 16,384
retirement and 16,384 detach gate vectors; fifteen controls and sixteen semantic
mutants after a passing dynamically rewired baseline. Cases cover retained/unknown
inventories, actual quiescence, other known writers, dirty state/new checkpoints,
contestation, fault-safe control, exhausted counters, relation ABA, reverse-ledger
caps and cloned/foreign/stale checkpoint inputs. Paired fact/quiescence mutants
weaken redundant fences together; they are not independently sensitive claims.
Import/syntax/runtime failures do not count as killed semantic mutants.

## Instrumentation and remaining gates

PATH-05/12 and INS-01/02/03/04/06/07/08/09/13 separate current checkpoint validation,
named dependency refusal, host-obligation readiness, actual task retirement,
historical-result unavailability, bidirectional detach and retained writer ownership.
MELT/RED/golden signals distinguish useful cleanup latency, refusal/error classes
and unresolved age; USE counts original-read work/storage, reader/writer relation
rows/charges, spent origins and independently pinned writer artifacts. STRIDE
covers forged checkpoints, ignored consumers, stale generations and quota theft;
LINDDUN covers historical-content and activity linkage. Content, paths, origins,
identities, handles, tokens and generations are not ordinary metric labels.

Active-checkpoint retirement and unused attachment detach are scoped reference
closures. Full disposal/rebind/unbind and retained-history production/release,
uncheckpointed recovery, contested-baseline replacement, all public products,
actual host/provider/retention/package truth, engine-global/temporary/physical
accounting, installed probes, production concurrent/crash refinement and independent
assurance remain gates. A host assertion cannot silently turn any of them into proof.
