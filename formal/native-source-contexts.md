# Native current source context ownership

The selected native reference issuer binds current source capture to the actual invocation
owner, reserves its declared pool envelope against that owner's remaining budget,
and issues opaque context identities. Its selected route implements existing F02
`snapshot-handoff` with `capture-current` and exactly `data: ['source']`, followed
by existing bounded-range or logical-line source queries. Section 12.8.6 additionally
supports retained selection of an original available source context; see
native-retained-source.md. Annotation/combined contexts use §12.8.5. Broader
historical and version-set products remain separate native integration work;
unsupported requests refuse rather than silently narrowing.

## Ownership and authority roles

Only the invocation owner's private ledger reaches the native helper. The engine
facade does not accept a caller-supplied owner or root. A pool binds one actual
owner/domain and immutable finite profile; a native WeakMap resolves its handle.
Context and hold maps use original object identity, not their visible tag or a
structural copy. These tags are internal reference representations, not a selected
wire encoding or a transferable issuer certificate.

The context claim contains the existing document, recipient, purpose, destination,
policy, continuity and interpretation-profile fields. It must match the captured
scope on every source handoff. Such a match establishes binding, not authorization.
The separately supplied release facts are accepted host observations for that bound
request. Their `scope` and `continuous` facts include current policy/profile and
continuity eligibility. The host must report accepted loss; merely presenting a
different scope claim is not a trusted policy-change notification. Actual caller
identity, policy interpretation, completeness of loss notification and issuer truth
across process restarts remain premises.

Validate the recognized request, native selector, complete caller binding, source
profile and numeric UTF-8 boundaries before applying bound loss. An unknown,
foreign, malformed or copied selector/request cannot close a different context.
Getters and non-string operation coercions reject without executing ordinary
accessors. This normalized-data boundary is not a proxy/native-code sandbox.

## Capture and logical budgets

Opening a pool validates the finite profile and debits a fixed logical charge from
the actual parent ledger before source capture. The charge covers declared profile,
context capture/scope/reply maxima and context/hold selector maxima. Bounded
request/copy workspace has its own reservation even with zero retained-context
capacity; the actual parent separately owns its declared bounded root frames. Local pool
capacity cannot replace the independent actual-parent headroom check. Arithmetic
overflow, incompatible limits and insufficient headroom refuse without a charge.
These are declared reference units, not measured physical allocation or complete
global/host accounting.

Acquisition reads the owner's actual current root, selects exactly one matching
live document and copies only its exact source/revision under the captured profile.
There is no second root or candidate argument. Existing source-product validation
checks canonical source and its zero-range envelope before installing the entry.
Complete scope, root, capture, selector and acquisition-reply limits apply before
one context is published; refusal installs nothing and spends no context identity.
Request and profile copies separately obey byte, node and depth limits.

The captured source remains unchanged across later native source publications.
Fresh capture sees the newer revision; an old context is not relabeled or refreshed.
Empty content, UTF-8 text, a literal BOM and mixed line endings retain their source
meaning through the existing range/line products. Successful results contain newly
owned bounded data while preserving the original opaque context reference.

## Invalidation and retained obligations

For a valid bound query, an actual faulted owner, actual owner disposal or accepted
exact-false authorization/scope/continuity/liveness/retention/source-read fact
invalidates delivery. Unknown facts and truthy substitutes deny without establishing
loss; ordinary resource, reply or transfer refusal likewise does not close a valid
context. Once invalidated or explicitly stopped, an identity cannot become active
again. Resource refusal cannot hide an otherwise accepted bound loss.

Native disposal is checked from the current owner root even if supplied grants
still say allowed. It prevents both delivery and new internal holds. Private
invalidation is recorded when a valid query observes disposal; this is not eager
bulk notification or proof that every reader receives a lifecycle event.

Owner stop uses the existing F19 product and protected owner control, without
ordinary content or capacity permission. Internal hold acquisition is a trusted
engine lifetime operation, not a public pin/renew verb or a grant from selector
possession. A stopped context keeps its actual source snapshot while any such hold
remains. Separate owner-control, retention-discharge and delivery-drain facts are
required to release a hold and retire its row. Foreign or duplicate hold releases
cannot refund another owner. Context retirement leaves the pool reservation charged;
only empty-pool retirement releases that reservation once. Issuance stays spent,
and the same owner cannot reopen the pool to reset it.

## Finite execution and evidence

The per-context state follows prepared capture → active → invalidated/stopped →
retired, with no resurrection edge. Context/hold counts and monotonically spent
issuance have finite declared ceilings. Each operation uses finite catalog checks,
bounded data copying and bounded source/root scans; it starts no job, callback,
retry loop, filesystem operation, provider discovery or network effect. This is a
conditional finite reference argument, not categorical completeness of the engine.

- `node formal/native-source-context-mutations.mjs` checks 13 controls, six bound
  losses, twelve unknown-fact cases, four transient refusals, 36 acquisition-gate
  denials and 33 exact boundaries across eleven dimensions. It rejects 28 semantic
  mutants after a coherent native-kernel baseline. One syntax-construction control
  is excluded from semantic mutation counts.
- `node formal/native-source-context-disposal-checks.mjs` composes actual native
  retained disposal, source delivery and held-root discharge. Two mutation controls
  execute complete copied module graphs so all helpers share the same native owner.
- `node formal/native-source-context-fsm-checks.mjs` compares eight lifecycle
  actions with a separate state oracle over all 4,681 traces through horizon four
  and 18,056 executed edges. Three mutation controls challenge active-row retirement,
  early refund and held-root loss. This horizon is not unbounded induction.
- `node formal/native-source-context-substrate-checks.mjs` wraps 143 filesystem/
  process callables after setup. The baseline's controls and boundaries attempt no
  such calls; three injected calls are detected before context installation while
  preserving the root, context/issuance state and existing parent reservation.

The existing invocation-publication suite passes in the same isolated source graph.
Import-time I/O, all host APIs, general fault containment and independent assurance
are not established by the selected substrate check. Specification §12.8.4 and model
revision 0.211 record this scope; complete frozen-input verification is required.
Native annotation/combined/historical/version-set paths, F05 result-child
lifetime and later F11/F12 current-use integration remain open.

## Observation and threat coverage

IF-02 and PATH-02 separate pool admission, source capture, selector handoff, query
release, observed loss and protected discharge. INS-01/03 record bounded admission
and release outcomes; INS-08 records retained roots/holds; INS-13 records protected
stop and retirement. Source-context installation is not authored publication under
INS-02. MELT/golden/RED distinguish useful handoff, refusal and stale work; USE
tracks occupied contexts, holds, spent issuance and parent/pool headroom separately.

STRIDE covers forged issuer/selector/scope, stale root use, malformed-input poisoning
and premature refunds. LINDDUN covers source disclosure, identity correlation and
unnecessary retained data. Ordinary labels contain no source bytes, raw document or
recipient identifiers, context/hold handles or policy values. Supervision counts
are trusted diagnostic products, not ordinary recipient existence oracles. Installed
instrumentation, clock/discharge truth, physical reclamation, concurrency/crash
refinement, public protocol and independent Cleanroom evidence remain gates.
