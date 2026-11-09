# Owned viewport intent and geometry reveal

Specification §19.5 refines F15 through viewport-edit/1, the integer-viewport/1
reference profile. These are accepted view-owner intents, not native scroll/frame
acknowledgments. Source coordinates and source ownership do not change.

## Precedent and choices

[CodeMirror's view source](https://raw.githubusercontent.com/codemirror/view/main/src/editorview.ts)
exposes independent nearest/start/end/center axes and explicit margins; center
ignores margins. [Helix view mode](https://docs.helix-editor.com/keymap.html)
permits scrolling without moving selection. These support explicit view commands,
not implicit source edits or universal key bindings.

[CSSOM View's scroll-position algorithm](https://drafts.csswg.org/cssom-view/#determine-the-scroll-into-view-position)
provides alignment/containment precedent. It is an Editor's Draft, not a W3C
Recommendation or an adopted DOM dependency. This reference does not implement
writing modes, snap, animation, nested scrolling or browser conformance.

| Choice | Benefit | Cost and disposition |
|---|---|---|
| Publish a bounded viewport intent | Deterministic local view state; raw scrolling can proceed while measurement lags | Accepted intent is not physical application; selected reference product |
| Wait for native acknowledgment before publishing | Can report applied host geometry | Introduces host timing/failure into the intent path; remains a separate adapter/effect obligation |
| Reveal an owned accepted projection | Reuses F09 context/target ownership without guessing coordinates | Requires current geometry; unavailable coverage refuses rather than estimating |

## Exact reference contract

Common fields bind contract, operation, invocation, view/document incarnations,
sourceRevision, expectedViewport generation, profile and exact metric context.
The context names unit and configuration/metrics/projection generations. Actual
view state must own matching context and a nonnegative viewport generation;
missing values are not zero defaults. This profile is not a production migration
policy. Older geometry-only normalized fixtures without an owned generation still
depend on their declared trusted current-context premise; new F15 state must carry
it explicitly. When present, F09 also checks it, including after equal-value ABA.

scroll supplies signed integral x/y deltas. viewport-intent supplies either an
exact set rectangle or reveal with owned layout/target, two alignments and
nonnegative margins. Rectangles have positive dimensions and bounded safe-integer
coordinates in the declared unit. Negative origins are permitted. No content
extent is inferred and no undocumented native clamp or animation policy follows.

For one axis let p be the origin, e the extent, [a,b] the target band and m the
margin. start = a-m; end = b-e+m; center = floor((a+b-e)/2) in the declared unit.
nearest clamps p to the interval between start and end. Thus a fitting target
already inside the margin-adjusted viewport, or a viewport contained in an
oversized target, does not move. Require 2m < e even for center. All intermediates
use exact wide integers; out-of-domain results refuse, not round or saturate.
The floor rule is an explicit reference-profile choice, not a native convention.

Reveal resolves the retained stop index, not the first equal source/affinity pair.
It binds accepted geometry's view/document/source/viewport and metric generations
and rechecks the same projection at publication. It need not change selection;
this does not solve F12's inability to store every richer visual-caret identity.
The projection helper is a trusted internal composition facet, not a public
reflection verb/plugin ABI or arbitrary hostile in-process boundary.

Preparation stages an owned opaque plan and charges request/plan JSON bytes.
Current grants/read/write/scope continuity/liveness/stop/interpretation/companion
gates and limits are required again at publication. For reveal, current geometry
release gates apply too. Ownership, target context, source and the captured
viewport signature must still agree. Complete identity returns the same root
without needing version capacity; change requires available safe counters.
Publish once from the current root, replacing only the target viewport and
incrementing its viewport generation and the root publication. Preserve current
selections, other views, annotations, source revisions and history. A viewport
round trip increments generation twice and cannot revive old geometry/plans.
Raw scroll/set do not require a layout, and do not restore intervening selection
or annotation changes; reveal's annotation dependency remains current.

Admission/query current-authority checks now precede geometry relation traversal.
Evaluation-order tracing controls check this guard order, not constant-time privacy
or hostile decoder isolation. Logical byte/work limits are not physical accounting.

## Evidence and instrumentation

The executable checks cover 648 independently enumerated alignment cases, 512
scroll-gate and 65,536 reveal-publication combinations, 44 controls and thirteen
load-success/assertion-failing mutants. Reveal tests use the actual accepted
geometry/target helper; frame tests include an actual intervening set-selection.
Paired geometry mutants share the same private ownership maps with the consumer;
import failures do not count as kills. Controls cover set/scroll/reveal, exact
rounding, margins, missing context, quotas, version exhaustion, source/annotation
staleness, ABA and duplicate source projections. This is finite scoped reference
evidence, not universal completeness or an implementation-refinement proof.

INS-10/11 separate intent preparation/publication, geometry invalidation, native
application and frame acknowledgment; INS-01/03/08 cover interpretation/current
release and resource pressure. MELT/golden/RED distinguish refusal, identity,
applied intent and adapter failure, with bounded queue/prepare/publish/ack latency.
USE tracks concrete plan/relation/cache/native-resource capacity. STRIDE includes
forged projection/plan, stale/ABA reuse, wrong-view publication and overflow;
LINDDUN includes reading position, document existence and viewport trajectories.
Use bounded reason/budget classes, not coordinates, source, IDs or raw context
tags as labels. Actual signals, privacy/overhead and host measurements need evidence.

F25/F28 provider admission, native clamping/application/paint acknowledgment,
shaping/bidi/font/cell-width, richer selection identity, actual policy/issuer/
retention truth, hostile decoding, replay/outcome/event composition, complete
companion state, concurrency and physical accounting remain separate gates.
Section 19.6 separately covers scoped geometry F25/F28 admission and revocation
through actual prepared reveals; see [geometry-work.md](geometry-work.md). It does
not change this artifact's isolated scope flags or prove native measurement.
No dependency, host matrix, annotation storage policy or measured default changes.
