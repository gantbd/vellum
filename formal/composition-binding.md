# Primary only accepted composition binding

The existing F11 accepted-composition-commit variant accepts one explicit
replacement at the original primary input target. Temporary preedit, focus,
native surrounding-text index conversion and cancel-before-command ordering
remain UI responsibilities under specification §§20.1/28.29. The engine does
not reconstruct committed text from keys or own an IME session.

## Request and facade binding

`composition-commit/1`, with profile `primary-composition-replacement/1`, has exact
fields contract, operation, invocation, document, view, sourceRevision,
expectedSelection, profile, replacement and text. Operation is
accepted-composition-commit. Replacement has exactly lo, hi and expected: one
original-snapshot half-open UTF-8 byte range and its complete expected bytes.
Text is literal Unicode scalar text, including empty text. The range can be an
accepted surrounding-text replacement rather than the current primary selection
extent; the supplied bytes are not authority to alter another document or view.

The native declaration `compositionInputForm()` supplies this field inventory.
`normalizeCompositionArguments` binds typed/dynamic F11/version-1 tuples to the
same complete bounded request, original seven-field execution context and three
argument limits. Native constructors use reference family indices; public Rust
names/identifiers are not assigned here. Unknown versions, fields, executable
profiles and variants refuse. This is normalized native input, not a hostile
byte decoder or wire format.

`prepareInvocationComposition`/`submitInvocationComposition` bind the original
issued domain/ticket and bounded canonical capture before native preparation.
Current ordinary-edit admission grants, actual remaining owner capacity and
source/view context remain required. An existing preparation cannot be replaced
by another facade wrapper, different range/text/limits/context, another ticket
or another owner. Matching typed/dynamic retries reuse the original job and charge;
they do not follow focus or resample targets. Generic accepted-registry metadata
can describe this declaration without a new fixed builtin discovery version.
Actual descriptor/schema/handler correspondence remains an installation premise.

## Primary target and publication

The producer validates current document/view incarnations, source revision,
selection generation, exact expected content, scalar boundaries and all source/
view/edit limits. It derives one edit only. Secondary selections and all attached
peer views map through that edit using the existing positional-selection policy.
The original primary selection is replaced by a caret immediately after inserted
text, retaining its affinity, before normalizing the resulting selection set.
Normalization can merge mapped selections and change the primary index; using
the original index after that merge would be incorrect.

The private plan retains the original composition request as well as the lowered
edit and complete companion candidates. The shared selection publisher rechecks
current source/selection/annotation-set and all attached-view signatures, current
authority, stop, versions and capacity. Source, actual annotation mapping, history
and view changes publish through the existing single local decision. Changed
source produces one coherent history group. Equal source bytes do not create
source history; an actual caret/annotation change can still be an applied local
decision. A complete semantic no-op remains unchanged.

The five existing invocation terminal pairs and current-recipient outcome release
apply, with effect none: this local command dispatches no provider work. No new
operation-specific result reference or U01 retrieval choice is needed. Source/
annotation/view projections reuse existing observation boundaries; complete
cross-family event assembly remains separate work.

Stop before publication prevents the local edit without making the ticket fresh.
Stale current state refuses rather than redirecting or refreshing the request.
Canonical capture refunds on failed preparation; an unaccepted provisional native
plan is discarded if the complete native envelope does not fit. Protected terminal
retirement releases the retained capture/plan once, never lifetime issuance.
These are logical normalized-value bounds, not physical allocation or GC claims.

## Conditional arguments and evidence

Assume finite normalized input, truthful current host grants/reservations, one
authoritative root and serialized non-reentrant trusted installation. The exact
single range constructs one edit; the ordinary splice equation therefore cannot
broadcast insertion to secondary selections. Reconstruction uses original primary
identity, followed by the existing total normalization/refusal policy. Retained
full intent plus single-ticket claim and the pending/running/done relation prevent
matching retries from executing again. Existing final-context and companion guards
either publish the complete current candidate or retain the previous root.

Finite closed request fields and bounded selection/view traversals provide
conditional per-call termination. No arbitrary script, UI callback, provider call
or fresh execution mode enters the path. This is not whole-engine categorical
completeness, a global bounded lattice, an unbounded code proof, crash refinement
or independent Cleanroom certification.

The focused checker has sixteen controls, ten omitted-field cases and eight
service-permission cases. Six coherent copied-graph mutations challenge broadcast,
wrong primary, missing expected-content checking, lost original intent, missing
secondary mapping and wrong primary caret. Controls cover primary-index changes
after mapping/merge, both directions of competing ordinary/F12/replacement
preparation, cross-owner plan refusal, explicit surrounding-text ranges, Unicode,
empty/equal-byte edits, stale actual view changes, stop, native envelope and exact
owner-headroom boundaries, replay and protected retirement. Full frozen
qualification and adoption remain required.

## Instrumentation and remaining gates

Existing IF-01/03/08 and INS-01/02/03/06/07/09/13 cover normalization/admission,
single-edit preparation, current-context refusal, publication/no-op, recipient
release, source/history, annotation mapping, view companions and retained discharge.
Separate caller refusal, stale conflict, stop, capacity refusal and invariant fault;
measure bounded preparation/service duration and logical capture pressure without
content, replacement ranges, document identities or session identifiers as labels.
UI INS-10/11 covers native input ordering/presentation independently.

Rust API/decoder/installation, actual scheduling/allocation/crash behavior, native
IME integration and independent assurance remain their existing gates. No new
engine component, network flow, resource provider or numeric deployment default
is selected. Exact-batch, accepted paste/cut, N03 and the whole L02 facade are not
closed by this binding.
