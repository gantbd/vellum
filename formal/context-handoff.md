# Configuration and command context handoff

Specification §22.3 requires in-flight work to retain its admitted interpretation
while rechecking current rights and affected dependencies at acceptance. PS-CH
checks this reference pathway across preparation, context activation, unrelated
cosmetic changes, source changes, revocation, cancellation and diagnostics.
It gives PH-09 bounded reference support, not a concrete schema or implementation
refinement. All 11 concrete handoffs remain open.

## Reference state and transitions

`ContextHandoff.tla` has one pending text-changing command, one document, two
interpretation contexts and a six-transition exploration horizon. The initial
source is three characters over two ASCII symbols. It imports the actual
`EditorContracts` splice, point-map and inverse operators; it does not import
that module's complete temporal transition relation or inherit its proofs.
The source/point/history root is a representative coherent participant vector,
not a selected Rust struct or public serialization.

Prepare captures the current interpretation, source revision, resolved edit and
coherent candidate. Activation can change the interpretation context while the
old admitted context remains retained. Commit checks current grant, exact source
revision, retained context and compatible meaning before publishing that candidate.
Rejection preserves the current root; cancellation stops the pending command.
An unrelated edit is a separate environmental action, not a bypass granted to
the pending request. Context retirement cannot discard a prepared command's hold.

The two context profiles either resolve the fixed logical request to the same
edit, or to different edits. Equality here is exact within that representative
request; it is not a general schema-compatibility algorithm. Cosmetic changes
are classified as unrelated by the model, not by a proved production classifier.
Only text-changing preparations are explored; adopted D01 identity is modeled
separately in PS-NO, not retroactively proved by this slice. No concrete command ID, keybinding, version format, fallback policy,
capacity, lease or transport is adopted.

## Properties and evidence roles

| Boundary | Positive properties | Fault controls |
|---|---|---|
| Prepared interpretation | Captured edit agrees with its context; capture stays immutable; admitted context remains held | Rewrite the captured context or discard it while prepared |
| Final publication | Current grant, exact source revision and compatible meaning; used edit remains the captured edit | Bypass grant/revision/compatibility, or reinterpret under the new context |
| Coherent participant root | Source, mapped point and inverse history equal the staged candidate; rejection changes none | The reinterpretation control exposes use of different admitted meaning; broader split-state controls remain in PS-EC/BR |
| Diagnostic boundary | Off/full/fault paths preserve root, phase, capture and grant; terminal outcomes stay terminal | Diagnostic write or a fault retroactively rejecting a committed command |

Five positive profiles cover incompatible and compatible interpretation changes,
and diagnostics off, full and fault. Eight weakened controls must fail the named
property. The reinterpretation control deliberately also bypasses compatibility:
the compatibility gate otherwise blocks that incompatible reinterpretation. It
does not establish isolated sensitivity to reinterpretation behind an intact gate.

Twelve witnesses require actual reachable paths: ordinary application, cosmetic
change accepted, compatible version change accepted, incompatible/source-stale/
revoked work refused, old context held after activation, pending cancellation,
normal diagnostic capture, diagnostic drop, diagnostic failure and a diagnostic
fault after publication. Admission/publication facts are frozen at the boundary:
later activation or revocation cannot manufacture acceptance evidence. In particular
the post-commit fault witness requires that the fault was absent at publication
and present afterward, not merely that both events occurred somewhere.

The 25 cases and property links are declared in `slices.mjs` and the PH-09
`boundedEvidence` record. The assumption checker validates evidence roles,
configured property names, module, horizon and complete selected-case linkage.
Its 25 structural controls additionally reject missing/mistyped bounded evidence,
wrong horizons and witnesses passed off as safety runs. Tool errors are not valid
counterexamples; only the intended property failures satisfy controls.

## Security and instrumentation

The pathway uses existing IF-01/03/07/09 and INS-01/02/09/12 placements. Preparation,
activation, final validation, publication, rejection and diagnostic loss/fault are
distinct semantic events. Relevant context/source changes, stale work, revocation,
retained old contexts and late diagnostic faults need separate observable classes
under the protected observation contract. No raw arguments, source contents or
context/recipient identities become approved metric labels through this model.

STRIDE challenges stale/rewritten interpretation, partial or unauthorized
publication and diagnostic-triggered mutation. LINDDUN and recipient projection
remain separate release obligations; this slice has no actual sink or disclosure
surface. USE/RED and golden-signal instrumentation still need concrete units,
schemas, pending-age and overhead evidence. Diagnostic actions consume a model
step while stuttering authoritative state: this is local state isolation, not
equal wall-clock latency, resource noninterference or fairness across profiles.

## Trusted host command binding preservation argument

Specification §22.2.3 defines the required command binding separately from native
resource registration. Its immutable descriptor/handler incarnation, bounded
capture, current applicability and retained code/asset hold instantiate the
existing interpretation contract. The binding runs above the coordinator, not
inside its transition or through a second mutable root.

[CodeMirror's dispatch/update implementation](https://raw.githubusercontent.com/codemirror/view/main/src/editorview.ts)
provides precedent for transaction-mediated changes and rejecting update reentry.
Its keymaps belong to view support, not Vellum's symbolic kernel. This precedent
does not supply Rust ownership, Vellum registration/disclosure or worker guarantees.

Assume truthful host ownership/current-policy/executor/retention facts, a coherent
initial registry and the independently specified facade contracts. Activation
validates an exact expected binding/absence and installs one complete immutable
pair, so lookup cannot mix a new descriptor with old code. A capture retains that
pair and does not resolve its name again; replacement therefore cannot reinterpret
it. A stale or inapplicable capture refuses rather than adopting the replacement.
Actual lifetime retention until handler exit and independent hold discharge is a
host refinement obligation, not inferred from a state label.

For each engine request emitted by a handler, ordinary facade validation is still
necessary. Induction over the finite admitted requests preserves the facade's
publication/authority/retention invariants; a host callback has no additional
transition rule for authored roots. Failure before admission frames that root;
failure after admission leaves the actual operation's recorded outcome and effect
knowledge intact. A multi-call workflow obtains no extra atomicity. Unknown emitted
semantics refuse before effects. These are conditional interface-composition
arguments, not a proof that an arbitrary trusted native callback returns or that
a malicious embedding cannot violate its own host contract.

The existing PS-CH held-context, reinterpretation, current-right/revision and
post-commit-fault checks support those abstract laws under their original finite
scope. They do not instantiate a native host command registry or discharge PH-09.
Required concrete binding acceptance must challenge duplicate/stale activation,
descriptor/handler substitution, replacement during a retained call, withdrawal
before invocation, copied/foreign captures, early code unload, callback reentry,
late failure after actual publication and protected once-only release. L02 retains
public binding/discovery/result assembly; E01/E02 retain actual Rust/host execution.
The new contract/proof argument does not count those scenarios as executed.

## Proof boundary and remaining gates

TLC explores the declared finite prefix; it proves neither unbounded induction
nor progress/termination or native callback behavior. Identity allocation, repeated
activations/ABA, multiple commands/documents, dynamic compatibility, real ownership,
adapters, codecs and public outcomes remain outside this slice. The two contexts
are never reused during a run. There are no physical effects or late provider
receipts here; their captured contracts remain governed by existing save models.

PH-09 remains an unbound concrete handoff in the assumption audit: a checked
reference relation does not establish that actual PA-08 producer outputs and
PA-06 lowering consume the same context representation. D33–D35 still own those
schemas, validators, lifetime/fallback and compatibility rules. No added shared
premise is used to relabel the concrete handoff as proved. Other R-01–R-12 gates
and the whole v1 specification objective remain open.
