# Abstract storage-outcome bridge

PS-SB connects actual `Persistence` and `SaveComposition` transition relations.
It replaces the free stored-result argument **inside this profile** with the
abstract host result, preserving the general composite's typed result interface.
This is an abstract-provider proof, not an actual filesystem test.

## Adapter and ordering boundaries

One composite attempt A maps to Persistence's Source attempt. Resource R,
incarnation 1 and serialization profile 0 are fixed. Payload, binding and profile
relations are checked explicitly; the richer persistence snapshot retains its
own revision and external-state token. This is not a complete wire-schema or
annotation-persistence refinement.

| Event | Actual persistence transition | Actual composite transition |
|---|---|---|
| Offer | Capture Source | Offer A |
| Supported/current conditional launch | Dispatch Source | Launch A |
| Unsupported/stale conditional launch | Dispatch to Rejected | Cancel undispatched entry; refund |
| Provider work | HostWrite Source | Stutter |
| Accepted result | Receive Source | Receive Completed, stored iff host result is OK |
| Contradictory report | Conflict Source | Receive Completed with negative result grade |
| Invalid tuple | BadReceipt Source | Receive mismatched tuple; no resolution/refund |
| Caller stop | Stop Source | Stop A; retain unresolved reservation |
| Source change | EditSource | Edit; preserve captured representation |
| Host binding transition | Rebind, guarded by quiet | Invalidate observed binding generation |

The abstract atomic compare/write can fail after valid dispatch if external
state changes. Earlier admission does not turn that failure into success. If
capability/current precondition is insufficient at launch, no dispatched unknown
obligation is created. The queue's generic Cancelled label is not a public
success/cancellation assertion: the persistence rejection retains the host-side
reason for a future typed API.

Host-side quiet and accepted core evidence are different observations. Rebind
invokes the existing abstract host guard; the composite sees invalidation and
retains its unresolved reservation until the result arrives. This is not blanket
permission for editor binding activation during unknown effects, nor proof that
an actual host supplied the required quiet evidence.

## Outcome and baseline agreement

Success acknowledgments must be backed by the abstract written snapshot. Resolved
coordination must be backed by provider quiet. Positive and negative outcome sets
agree with persistence facts. Current baselines agree on presence/absence,
binding, profile and captured source representation.

The composite retains a monotone negative-result set alongside positive
acknowledgments. A contested attempt cannot establish a current baseline, even
after a duplicate positive result. Both observations survive: no last-message-wins
rule removes the contradiction. This connects the existing persistence Conflict
contract. Broader provider quarantine/fault policy remains follow-up work, not an
automatic policy choice made by this model.

Fifteen cases include three safety runs, three controls and nine witnesses.
Six-transition exhaustive profiles check capable/unsupported hosts. A six-step
script writes, receives success, receives a contradiction and receives success
again. Controls invent success from a failed host result, inject completion
before any host result, or ignore contradiction during baseline selection.
Each must fail its named cross-model property.

Witnesses cover success, dispatch-to-write conditional failure, prelaunch refusal,
unsupported-host refusal, late success after stop, historical receipt after binding
change, an old captured revision, malformed input retaining reservations, and
retained contradiction without a baseline. Actual Next-or-stutter correspondence
is checked for both modules.

## Limits and instrumentation

The inherited persistence profile has one attempt per artifact, symbolic payloads
and one external-change token; successful HostWrite does not model per-write token
consumption. Multiple source attempts/token reuse/order are the separate PS-CS
profile. No actual codec, provider authentication, physical durability, arbitrary
aliases, joint annotation save, real-clock bound or recovery guarantee is supplied.

IF-04/INS-04 separates rejection, conditional failure, success, contradiction and
quiet-based reconciliation. Duplicate traffic is not a second completion/refund.
IF-10/INS-08 distinguishes host quiet, observed invalidation and authorized binding
activation. IF-13/INS-07 preserves snapshot/profile grades without raw payload/token
logging. Malformed-input work and evidence storage need bounded envelopes separate
from caller fuel and outstanding-work reservations.

Actual supplier conformance, public schemas, codecs, dynamic identities/attempts
and complete lifecycle/result-grade relations remain generalization gates. This
strengthens scoped design evidence, not production I/O or clean-room certification.
