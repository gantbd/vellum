# Accepted geometry queries and visual selection

Specification §19.4 supplies F09 geometry-query/1 and the F12 visual-selection/1
consumer. The declared-caret-geometry/1 reference profile consumes bounded accepted
provider relations; it does not measure native glyphs, shape text or redefine the
symbolic kernel's source coordinates. Provider truth and actual frontend support
remain separate evidence gates.

## Precedent and profile boundaries

[CodeMirror's view method documentation](https://raw.githubusercontent.com/codemirror/view/main/src/editorview.ts)
distinguishes logical/visual positions, precise/estimated coordinate lookup and a
retained horizontal goal for vertical movement. Its
[commands 6.8.1](https://raw.githubusercontent.com/codemirror/commands/6.8.1/src/commands.ts)
compute page movement from viewport geometry and margins.
[Emacs 30.2's display manual](https://raw.githubusercontent.com/emacs-mirror/emacs/emacs-30.2/doc/emacs/display.texi)
has configurable page overlap and screen-position behavior.
[Helix](https://docs.helix-editor.com/keymap.html) distinguishes visual-line/page
commands and a view mode for scrolling without changing selection. These support
explicit semantic choices, not a universal keymap, page overlap constant or
cross-frontend geometry algorithm. The current view source is precedent, not a
pinned dependency/conformance implementation imported into Vellum.

| Choice | Benefit | Cost and disposition |
|---|---|---|
| Accepted stop/hit relation | Deterministic bounded queries and current-context checks independent of a native callback | More bounded relation data and a declared coordinate profile; selected reference product |
| Provider-owned query execution | Can use native shaping and complex geometry directly | Requires bounded request/result ownership, re-entry and admission evidence through F25/F28; not a hidden synchronous callback in F09 |
| Host-specific page/binding policy | Fits native conventions and viewport constraints | Must be explicit and context-bound; no global pixel/line constant or binding is inferred |

## Geometry ownership and query products

An accepted normalized packet binds exact view/document incarnations, source bytes/
revision, annotation set, layout/decoration/projection/configuration/metrics/viewport
generations, the viewport, unit/profile and positive page distance. Current source,
view and viewport must still agree. Current identity/grant/scope/continuity/capacity/
liveness/retention/stop suppliers remain premises; this helper does not implement
the complete F25 measurement or F28 pending-task admission families. Raw packet
possession cannot install native geometry or authorize source access.

Rows are a finite contiguous prepared fragment with ordered vertical bands. Stops
carry scalar-safe source position, explicit affinity and declared horizontal
coordinate. Source order need not equal visual order. Left/right neighbor links
are explicit, reciprocal and acyclic; edge and unavailable are different closed
values. Half-open hit rectangles target a source stop or a virtual identifier.
Where one source hit exists at a declared caret representative, it must agree
with that caret's source/affinity. This is a conditional inverse domain, not a
universal pixel/source bijection or proof that native measurements are correct.

geometry-query/1 has three closed variants. Caret/hit conversion returns known
caret projections or source, virtual, ambiguous or unavailable hit results. No
prepared caret/hit coverage is unavailable, not an empty complete answer or an
estimated source position. Overlapping targets remain ambiguous; a choice must
match the current offered alternatives. Virtual output never carries a source
selection reference. Bounded-layout returns requested retained rows and explicit
edge coverage, not incidental global totals or a claim that a fragment is the
whole document. Successful data replies have explicit byte caps; the minimal
unavailable control response does not depend on a data payload fitting.

Visual-navigation-target takes an explicit origin index, closed unit, positive
bounded count, layout-bound horizontal goal and per-step choices. Units are
left/right, row-start/end, row-up/down and page-up/down. Horizontal motion follows
the provider graph; row endpoints are physical row edges, not logical line starts.
Vertical motion chooses nearest declared row stops to the retained goal, exposing
ties for explicit resolution. The goal is not reset by a short row and is not
silently reused with another layout. Counts accumulate page distance from the
origin band rather than rounding every step to a row top. Integer arithmetic uses
exact wide intermediates for distance/tie and page-coordinate checks; floating
rounding cannot silently choose a different caret.

Page distance is an accepted positive amount no larger than the captured viewport
height in the declared units; the provider/host supplies its policy. Missing
prepared rows cannot be treated as document ends. Known complete edges may
saturate. Oversized rows can retain a caret while page/scroll intent is separate.
F15 viewport publication and native scrolling are not implemented by this slice;
querying/moving a selection does not implicitly scroll or launch measurement.

Source replies deliver owned layout-bound target references, cached once per exact
query under a finite per-layout quota. Every cached handoff rechecks current
context, rights and limits. A copied/foreign reference cannot reconstruct local
provenance. Reference JSON placeholders are not a chosen serialized token format.

Section 19.4.1 checks a complete prospective source reply against its current byte
budget before installing a new target. Withheld hit/navigation replies do not
consume undisclosed persistent cache slots. Withholding an existing reference
does not revoke it or refund issuance. See geometry-reply-admission.md for native
boundary/lifetime/mutation evidence and the remaining public/physical gates.
This remains presentation-owned reference behavior, not an engine facade route
or dependency.

## Current selection consumer and representability

visual-selection/1 binds invocation, exact view/document/source revision, expected
selection generation, accepted layout, owned targets, explicit mode and extension
intent. move-heads requires one navigation target per current selection and exact
origin head/affinity correspondence. replace-primary and replace-all consume one
explicit source target, including a resolved source hit. Extension preserves the
chosen current anchors; ordinary normalization retains primary/direction laws.
No implicit focus, source write authority, geometry-based range widening or source
history follows. Query origins/targets are explicit; keyboard collapse/binding
policy does not silently replace the logical command contract.

The current source selection record stores position and affinity, not an arbitrary
visual-stop identity. Queries may report multiple projections for that pair.
The consumer refuses an unrepresentable target or ambiguous move-head origin
rather than reporting false identity or guessing a visual occurrence. Richer
visual-caret identity, beyond what the accepted source/affinity profile represents,
is an explicit integration/refinement gap; no new presentation state is silently
inserted into the symbolic kernel.

The actual set-selection helper validates current view/source/selection context
and performs one complete pure-view publication or identity. Target-view viewport
changes invalidate the geometry plan; unrelated current viewport changes survive
without restoration. Source/annotation/history and peer selections remain intact.
Current geometry predicates, target quotas, total steps, request bytes and captured
selection limits revalidate. All targets stage before one publication; a failed
participant cannot leave a partial multi-selection result.

## Finite arguments evidence and instrumentation

Finite source/row/stop/hit/target counts, request/reply/layout bytes, coordinate
domain, navigation count and validation-work caps bound the reference operations.
Reciprocal graph checks and bounded cycle detection prevent malformed neighbor
relations from becoming unbounded traversal; admitted counts also bound repeated
identity steps. A finite relation does not prove native measurement halts, actual
allocation/retention, progress/fairness or categorical completeness. Integer-safe
coordinates are a reference semantic representation, not a chosen wire/Rust or
universal pixel/cell format. Trusted internal helpers are not a public plugin ABI
or malicious in-process JavaScript isolation boundary.

The suite checks 2,550 independent synthetic-grid query/navigation cases, 128 query
gate combinations, 65,536 selection-publication combinations, 87 controls and
nineteen load-success/assertion-failing mutants. Controls cover variable-height
page accumulation, exact extreme-coordinate distances, non-source visual order,
explicit ambiguity/virtual outcomes, coverage, duplicate projections, quotas,
current context/bytes, multi-head/primary normalization, source/history frames and
unrelated viewport preservation. Mutants rerun focused oracle/guard controls;
the unmutated gate enumeration remains complete for its selected predicates.
Synthetic geometry is not actual font/shaping/bidi/cell-width or independent
Cleanroom certification. Actual provider/current-policy/issuer/retention truth,
F25/F28/viewport/native integration, replay/outcome/event and complete companion/
physical/concurrent refinement remain gates.

Section 19.5 separately composes F15 viewport intents with the retained target
index; see [viewport-products.md](viewport-products.md). F09 checks an explicit
owned viewport generation when supplied, including equal-rectangle ABA. Older
normalized geometry-only fixtures without that field retain their trusted current-
context premise; F15 does not accept missing generation/context as zero. Admission
and query current-authority guards now precede relation traversal. Two tracing
controls and corresponding mutants establish guard order, not constant-time
privacy, arbitrary JavaScript isolation or hostile decoder conformance.

Section 19.6 separately supplies geometry F25/F28 task admission and acceptance;
see [geometry-work.md](geometry-work.md). Its trusted retirement facet invalidates
actual issued layouts/targets/prepared consumers after cancellation, supersession,
authenticated contradiction or retirement. Geometry-only checks retain their own
limited provider-premise scope; native measurement and analysis work are not proved.

INS-10 separates preparation/measurement, acceptance, query reuse/coverage and
ambiguity; INS-11 distinguishes target handoff and selection publication; INS-01/03
cover interpretation/current release and INS-08 retained relation/target/work
pressure. MELT/golden/RED separate unavailable/virtual/ambiguous/unchanged/applied
dispositions and queue/query/publication latency; ambiguity is not automatically
a provider fault. USE tracks concrete relation/cache/native-resource pressure.
STRIDE includes forged/stale geometry, virtual-to-source conversion, cyclic
neighbors, coordinate overflow and guessed hits. LINDDUN includes view existence,
layout/word density, language/style, annotations and selection streams. Use bounded
status/reason/budget classes, not source, glyphs, rectangles, offsets, IDs or raw
metric-profile tags as labels. Actual signal installation/release/overhead remains
separate evidence. Annotation point persistence and the host matrix are unchanged.
