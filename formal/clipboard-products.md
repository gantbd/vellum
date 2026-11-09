# Owned clipboard transfer and local publication

Specification 0.213 §1.1 assigns the workflow verbs and clipboard-specific task
state to a headless integration facade, not the semantic engine. The module below
is combined integration/engine composition evidence. Its use of actual editing
primitives does not establish a clipboard-free core package or permit integration
to write coordinator-owned state directly. All original capture, authority,
evidence, local publication and protected-discharge requirements remain.

Specification §21.1 refines existing F22/F23, selected F27 evidence and F11 paste/
validated-cut products. The reference composes actual selection, source/annotation
publication and native F14 undo/redo modules. It is not an OS clipboard adapter,
whole-family proof, production implementation or independent Cleanroom review.

## Precedent and adopted semantics

[Neovim nvim_paste](https://neovim.io/doc/user/api/#nvim_paste()) distinguishes bulk
text insertion from mapped input. Adopt that separation, not its mode, dot-repeat,
line conversion or streaming behavior. Vellum plain paste broadcasts the entire
literal payload to each captured target; no key replay, newline splitting or implicit
indentation occurs. Structured fragments pair only when their count matches targets.

The [W3C Clipboard working draft](https://www.w3.org/TR/2026/WD-clipboard-apis-20260624/#clipboard-actions)
separates copying from source-changing cut/paste and discusses access permission.
It is work in progress, not a binding native platform standard. Its mutable-focus
event semantics are not adopted: Vellum retains the originally identified target
and rejects stale revisions/selections instead of using a later focus. No browser,
DOM, HTTP or clipboard synchronization service is introduced.

## Exact products and execution phases

`clipboard-transfer/1` names `copy`, `cut-export-stage` or `paste-request`, exact
document/view/source/selection context, invocation and effect origin, named provider/
capability/registration and `plain` or session-local `structured` format. The actual
closed registration distinguishes system clipboard, platform selection and session
clipboard, read/write capabilities and the provider's declared acknowledgment profile.
No universal transfer/durability guarantee follows from that profile string.

Admission validates current source/selection identity, normalization, rights, full
registration, task/issuance/root/capture/payload/held limits and finite schemas.
Copy requires disclosure/read rights, not source or view write rights. Cut and
paste require edit rights. Capture selected fragments in normalized document order,
preserving their contents verbatim. Plain export joins them with one LF separator;
all-empty copy/cut is unchanged without clipboard clearing or whole-line expansion.
No source/history publication occurs at copy admission, dispatch or acknowledgment.

Dispatch rechecks the actual full registration and current capability/launch rights;
export separately needs disclosure authority. Paste also revalidates its captured
target before requesting data. Only the current named provider recipient with
release/disclosure authority obtains copied export bytes. Payload handles and
provider dispatch do not expose raw source documents or private annotation/history
roots. Provider-reported read data is insertion data, never executable input.

Receipts correlate the actual owned task, full immutable capture and origin/provider/
capability. Validate exact bounded shape, authenticity and independent attestation
before accepting a result or quiescence. Result is accepted or known-not-applied;
missing result remains unknown. Equal duplicates are idempotent; incompatible
authoritative facts mark contested and retain capped witnesses. Do not use last
arrival as truth. Late contradiction cannot roll back an already published source.
Quiescence releases work only; cancellation stops unapplied local intent, not a
late accepted fact or outstanding provider work.

For cut, only accepted declared export evidence can precede renewed exact target,
current registration, edit rights and companion/history/resource validation. Then
delete the original selected ranges through the actual ordinary-edit publisher.
For paste, prepare all replacement edits against the same captured source and
selection; validate the full expanded work before one local publication. Current
annotations and every attached source-view selection participate. A changed focus
cannot redirect the edit; changed revision, selection, incarnation, profile, liveness
or authority refuses. Failed/unknown export leaves source intact. Accepted export
with failed/cancelled local intent remains copied-not-cut; no compensation retracts
disclosure and no joint external/local atomicity is claimed.

Successful local application records its Applied or Unchanged disposition.
Section 6.5.3 and clipboard-invocations.md separately join acknowledged cut/paste
to the invocation ledger through owned pins and staged task/root/fact commit.
That is not full request/dispatch replay or event refinement. Actual native F14 undo/redo
reverses source and maps current views without dispatching clipboard work or
restoring a prior clipboard value. Section 6.4.1 now composes actual equal-byte
annotation mapping without source revision/history changes; see equal-byte-companions.md.
Other companion profiles remain refinement obligations, not universal paste proof.

## Session fragments and bounded retention

Structured external results are refused. Session structure requires the exact owned
handle, same active registry/provider/capability/registration and current release/
session-retention rights. Text equality, a cloned handle or another owner cannot
recover fragment boundaries. Each capture has one idempotently issued fragment
handle. Receiver tasks copy accepted fragments into their own admitted capacity.
Retirement invalidates the producer handle, not a previously accepted receiver copy.

Reserve declared capture plus canonical/conflicting payload/envelope maxima and
one terminal-stage envelope with checked arithmetic; maxConflicts participates in
the charge. Full invocation stages additionally charge four root maxima; see
clipboard-invocations.md. Bound issuance for
the owner's lifetime, never recycling origins after retirement. Actual physical
allocation and hostile object/decoder costs remain unproved. Unknown/contested
effects cannot retire. An accepted cut/paste additionally remains held until local
publication or explicit stop, or the known terminal refusal recorded by §6.5.3.
Live capsule/stage pins and unknown local dispositions prevent retirement.
Quiescence, retention release, delivery drain and
session-reference discharge are independent retirement premises; actual host truth
and native provider retention procedures remain integration work.

## Conditional arguments and executable evidence

Assume a valid serialized root, complete current scope/identity/companion coverage,
truthful policy/provider/retention attestations and correct normalized inputs.
Immutable capture plus exact current validation prevents focus redirection or stale
deletion. The accepted-result guard prevents unknown/failed exports from deleting
source. Actual ordinary publication frames source/annotations/history/current views
as one local result; refusal leaves the supplied root unchanged. Private publication
state prevents repeated local application in this serialized reference, while late
facts remain historical. Bounded task/witness/issuance stores and finite gate/variant
inventories bound reference processing, not arbitrary native provider execution.

The checks cover 1,536 admission, 768 dispatch, 1,024 publication and 128 release
decisions, 25 controls and fifteen assertion-rejected semantic mutants. Controls
exercise real source/annotation/view and native undo/redo composition, stale target/
focus/cancel races, unknown/contradictory receipts, owned versus forged structure,
limits, pinned local intent, counter exhaustion and actual equal-byte mapping publication.
Finite matrices and these authored conditional arguments are not general induction,
new SMT laws, categorical completeness or independent certification.

## Instrumentation and remaining gates

PATH-06/DF-11 retain IF-04/03/08 boundaries. INS-04 separates admission/dispatch/
acknowledgment/quiescence, INS-01/02 local publication, INS-03 recipient release,
INS-06 history and INS-11/13 native capability/retention/shutdown responsibilities.
MELT/RED/golden signals distinguish provider latency, current-validation refusal,
copied-not-cut, unknown age and useful local completion. USE tracks admitted tasks,
expanded edits, payload/witness/session holds and saturation in declared units.
STRIDE covers forged receipts/handles, capability substitution, stale-target deletion
and text-as-command injection. LINDDUN covers involuntary disclosure, retained content
and cross-device/activity linkage. No source/clipboard text, raw identity, token or
path is a metric label. Actual collectors, clock domains, physical costs, side-channel
noninterference and measured profiles remain evidence gates.

Open: actual clipboard acknowledgment/ownership/retention/release profiles and native
hosts; equal-byte nonidentity and all other native companion coverage; complete public
invocation/replay/outcomes/events; actual concurrent root/private-intent installation;
hostile decoding, physical accounting, production refinement and independent assurance.
No new engine component, network path, public operation family/variant or kill ring
is selected. The approved point representation is §29.4.1; native persistence integration remains separate.
