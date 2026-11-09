# Owned source reads and initial owner activation

Current registration integration: §22.3.3 adds atomic native resource metadata
activation and removal/reinstall generation fencing; see
[resource-registration.md](resource-registration.md). Current-row checks stale future
work without rewriting captured late evidence, bindings or producer obligations.
Public registration products, imported lifetimes and host/physical/independent gates
remain; earlier exclusions retain their original slice scope.

Current integration: §11.2.10 adds domain-bound source-read work, actual native
initial activation/alias join with producer markers and paired-writer guards, plus
selected actual-root/peer/queued-consumer read retirement; see
[native-source-read.md](native-source-read.md). Imported/legacy/multi-registry or
contested recovery and remaining persistence/host/public/physical/independent gates
remain. Earlier exclusions below retain their original slice scope.

Specification §§16.1.2/11.2.1 compose actual source-codec streams with F20
source-load work, selected F27 receipts and F17 initial-owner activation. The
reference rechecks current resource registrations and complete binding coverage,
preserves existing owners, and constructs a fresh source owner only after valid
EOF. It does not implement a filesystem provider, annotation loading, source
replacement/reload, full resource writes or concurrent production root installation.

## Captured resource and identity contract

resource-read.json closes normalized requests, registration records, event
variants and finite profiles. A request names origin/effect/attempt, provider and
resource incarnations, expected registration, codec and required read guarantee.
An authority-bound registry captures the entire actual registration: declared
identity domain/key/knowledge, locator, generation, codec, atomic-read capability
and generated-newline policy. The LF reference default is explicit; an accepted
host registration may select CRLF/CR without converting loaded source.

Admission and submission check current authority/scope/continuity/liveness/stop,
identity rights, complete binding coverage, read grant, provider availability,
resources and launch permission. A stale registration body cannot survive merely
because its numeric generation was not updated. Unknown identity is never known
distinction. Locators and equal source bytes do not establish resource equivalence.
The existing resolveOwner relation is used with the actual current binding map.

captured-read means a coherently identified received byte bundle, not a proved
single physical resource revision or current external contents. An atomic-resource-
snapshot request requires a declared capability and supplied trusted attestation;
it cannot silently accept captured-read. Equality of start/end epoch/token alone
does not establish atomicity, ordering or absence of an intervening write. Provider
identity/coherence/attestation truth remains a host premise, not a fact proved by
these field checks. No path/metadata heuristic or actual provider capability is
invented by the reference.

lookupLoadedResource can resolve an authorized already-live source owner without
new I/O, even when provider/work capacity is unavailable. It still requires current
identity/coverage and independent access to that owner. A binding owned in a
different artifact role is not silently absent or joined as source: this reference
refuses unsupported/inconsistent role mappings. The supplied coverage premise
must cover all relevant writable bindings, not merely a filtered source list.

## Ordered receipt and codec composition

Only an owned submitted task can receive the closed start/chunk/end/failure/
quiescence variants. Check authenticity, evidence authority, ingress capacity,
exact task/origin/provider/resource and envelope shape before protected payload
service. Start supplies the attributed resource version/coherence and opens the
actual private source-codec stream. Chunks require exact next sequence/file offset;
end repeats the version/coherence and exact received chunk/byte counts. Gaps,
wrong counts or unrelated attempts are not repaired by guessing or reordering.
Only actual successful codec EOF yields a usable source candidate. Invalid UTF-8
or exhausted codec capacity is candidate-refused, not an engine defect, provider
quarantine, partial source publication or external-read quiescence.

Validate octet arrays and bounded scalar counters before duplicate/conflict
classification. An authenticated but malformed chunk/EOF cannot manufacture a
trusted contradiction, quarantine a valid attempt or poison its loaded baseline.

Exact duplicate retained rows are idempotent and do not allocate another stream.
Contradictory rows retain the original transcript plus bounded new witness data,
contest the task and disable candidate use. Terminal failure and successful EOF
are distinct witnesses. The store reserves two bounded witness slots; another
distinct witness explicitly returns evidence-full. Transcript exhaustion refuses
the candidate rather than marking incomplete text loaded. Ordered payload assembly
and this capped witness store are not arbitrary-union lattice closure.

Cancellation/supersession disables applicability but cannot refund submitted work.
Pure new chunks may be discarded after stop without a claim that their content
was processed. Required terminal facts and contradictions of retained rows remain
separate. A ready stopped task never launched work and can retire without provider
quiescence. A submitted task needs separately authenticated attested quiescence;
EOF, failure, caller stop and wait cutoff do not provide it. Quiescence may precede
already-buffered ordered receipts, and does not imply delivery drain or free
retained transcript/codec state.

## Current whole root activation

Activation receives the authority's current serialized normalized root. It
rechecks the complete registration, current authority/identity/coverage, source
candidate, companion/version/reply availability and actual live owner registry.
The state argument and private bookkeeping are one logical reference transition;
production installation/linearization is a separate refinement obligation.

If a known source owner now exists, independently authorize joining it and return
that owner unchanged. Two staged opens, including different known aliases, cannot
install two writable roots when applied in either serial order to the current
root. Dirty source, comments, history, views and memberships survive; disk bytes
do not replace the current owner. Joining needs no fresh identity or creation
capacity. Existing-owner lookup is not a disclosure or write grant by possession.

Otherwise require current creation capacity and allocate a fresh non-confusable
logical document identity from the root's declared namespace/counter. Check actual
live and retired identities, safe counter advancement and root publication limits;
do not wrap or silently skip a collision. Construct source, BOM/profile generation,
generated-newline policy, empty initial annotations, binding/registry entry and
loaded-capture baseline in one bounded new root. Advance the allocation/publication
counters once. Do not attach a view/workspace implicitly, create source-edit undo
history for materialization, infer annotation storage or alter existing owners.

The baseline owns the actual received raw bytes and original read origin/provider/
version/coherence, binding/source/serialization generations, profile and BOM choice.
It records what was loaded, not a write acknowledgment, durability or current
external freshness. Actual serialization of the activated source reproduces the
received BOM/text bytes in the tests. Inspection returns owned copies; caller
mutation cannot change the candidate used for activation.

Repeat activation can return the same live owner only while its binding generation
still matches. A disposed/rebound target cannot be resurrected by an old task.
After committed activation, later trusted contradiction contests the read evidence
and makes loadedBaselineUncontested false; it does not reverse published source,
erase later edits/comments/history or rewrite the already delivered result.
That predicate checks exact baseline bytes and original binding/version/strength;
it does not authorize saving or promote a loaded capture to acknowledged persistence.

Section 16.3.1 additionally uses resourceReadBaselineEvidence to validate retained
historical loaded provenance against the actual private transcript, independently
of the current root's baseline pointer. It does not replace that pointer or assert
current external truth. loadedBaselineUncontested remains a current-root wrapper;
both predicates exclude later contradictory read evidence. This is not a transfer
of the read task's existing live-binding provenance hold.

An active created binding pins its producing read/provenance record. Quiescence
releases work only. Safe task retirement additionally requires recipient-independent
retention/drain obligations released and no live binding still naming that read.
Rebind/disposal integration and transfer of required historical holds remain
separate obligations; the reference does not invent such a transfer or evict a
live owner under pressure. Origin issuance is not recycled on retirement.

Section 11.2.2 extends the producing-read pin through retained disposal and key
release, using the historical retained binding until independent provenance discharge.
That extension is checked in lifecycle-products.md; quiescence/drain/other consumer
retention still govern actual retirement. It does not erase evidence on disposal.

Section 11.2.3 separately composes explicit destination intents and rebind. Actual
producing-read retirement also recognizes old bindings in bounded bindingHistory,
until independent history/snapshot/read/write/provider discharge. That reference
does not prove native consumer transfer or discharge truth; see source-export.md.

## Conditional argument and executable scope

Under trusted identity/policy/coverage/coherence facts and a valid current root:

1. Only current admitted/submitted tasks issue a provider capture. Private task
   ownership and exact correlation prevent another attempt from feeding its codec.
   Preflight reserves a codec context including the largest admitted version field;
   registry profiles cover all retained task streams and complete candidate handoff.
2. The actual streaming codec plus exact counts/version binds a candidate to that
   captured read. No invalid/partial/stopped/contested result can become source.
3. The final owner lookup either joins one live source root unchanged or adds one
   fresh identity to an absent complete key. Induction over serialized activations
   preserves at-most-one owner for that declared identity domain/key; incomparable
   evidence does not establish global physical alias equivalence.
4. Creation constructs source/metadata/binding/baseline together, preserving every
   old root participant. Private single-use activation and live/tombstone checks
   prevent stale resurrection. Later evidence changes provenance knowledge only.
5. Work, retained records, active binding holds and spent origins remain separate;
   no caller stop, terminal fact, quiescence or duplicate can refund another class.

These are authored conditional trace arguments, not checked unbounded induction.
resource-read-checks.mjs executes 2,048 admission/2,048 submission/2,048 activation
gate combinations, eight intake combinations, 60 controls and twenty load-success/
assertion-failing mutants. Composition uses actual source-codec, identity resolver,
source serializer and ordinary source publication modules. Controls cover BOM/
mixed source, alias races/dirty joins, complete cross-artifact coverage, changed
registrations, stronger capability refusal, duplicate/conflict/cancellation/late
receipts, real UTF-8 refusal, fresh allocation/tombstones, baseline promotion,
active provenance holds and all selected capacity dimensions. Scope is normalized
source opening, not annotation-load or complete F17/F20/F27 family conformance.

Finite profiles bound tasks/outstanding/issued origins, captures/envelopes/
transcripts/witnesses, live task charges, decoder reservations, registry scans,
new documents/bindings, root/candidate sizes and metadata tags. Spent origin storage
has its own finite issuance/capture bound; live-task held-byte metrics are not RSS
or a claim that retirement erased every tombstone. No service-time/fairness,
actual coherent filesystem snapshot, host identity/permission truth, physical
retention/accounting, hostile object/wire decoding, actual concurrent installation,
full invocation/replay/event, all companions, source replacement/reload, resource
write or independent Cleanroom refinement follows. Current full formal/results.json
and matching live/frozen hashes govern execution freshness.

## Instrumentation and precedent

INS-08 covers identity resolution, known/unknown/role refusal, owner joins/creation,
allocation and pinned load provenance; INS-04/07 capture/launch/start/chunk/EOF/
codec refusal/end/failure/late contradiction/quiescence; INS-02/01 final activation;
INS-03 scoped candidate/baseline release; INS-09 captured registration/policy;
INS-13 protected drain. Distinguish received bytes, decoded bytes, completed
candidates and useful created/joined owners. RED covers attempts/refusals/duration;
USE covers task/decoder/transcript/witness and pinned-record saturation; golden
signals include useful activation latency, stale work and unresolved quiescence/
drain age. Use local monotonic clocks and protected correlation, not origin,
resource/document identity, locator, token, paths or content as metric tags.
STRIDE includes forged attempts, wrong alias/role, unsupported guarantee promotion,
stale activation and exhaustion. LINDDUN covers owner-existence/alias linkage,
loaded content and retained provenance. Probe coverage is not installed telemetry.

Section 11.2.4 separately checks known-empty inventory at fresh source activation,
input/final membership budgets before task activation, and joins preserving actual
authored/retained inventories; see [annotation-lifecycle.md](annotation-lifecycle.md).
This is not annotation-file loading or migration of unknown legacy inventories.

[Neovim validity/residency](https://neovim.io/doc/user/api/#nvim_buf_is_loaded())
distinguishes valid buffers from loaded contents. This supports separate identity,
materialization and view lifetimes, not adoption of Neovim's transport or buffer
policies. [Rust metadata](https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.modified)
describes a file's known metadata and platform-dependent modification-time support;
it supplies no atomic-read or universal alias oracle. Vellum's existing §§11/16
and adopted single-owner/current-publication rules govern this construction.
