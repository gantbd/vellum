# Reference UI boundaries and remaining release work

This is the presentation workstream for the ratatui editor example and the iced
adapter/example. It is outside engine completion. Requirements §§1.1/8–9.5 still
require both integrations for the wider editor release; that does not make their
UI policies part of the engine or require a UI to use the engine.

Both reference consumers now exist: [ratatui](../crates/vellum-tui/src/main.rs)
and [iced](../crates/vellum-iced/src/gui.rs), over the shared
[Editor](../crates/vellum-model/src/app.rs) and one `LocalHost` across files.
They implement native editing/history/save, search/replacement, explicit
destination confirmation, editing locks, session comments and guarded close.
Both UIs expose explicit comment-file save/reopen over the shared Editor, with
confirmed existing destinations and bounded host UUID/source-witness retention.
Source save and comment save remain independent; historical JSON without actual
managed witnesses refuses activation rather than guessing a relocation.
See the [README](../README.md) for current controls and reference profile limits.
Headless framework tests and live terminal checks do not qualify the desktop,
OS input methods, accessibility or production Unicode/geometry profiles.

These consumers are exemplars for the other engines: retain domain behavior in
the engine, orchestration in integration and effects in substrate/provider
adapters. Reuse the boundary and qualification practices, not Vellum's document
semantics or a mandatory common UI framework. The iced source and comment forms
now share independent viewports with wrapped comment controls and scrollable
auxiliary content; a full-window framework regression guards against a comment
draft collapsing the source viewport. This is not desktop qualification.

## Ownership boundary

The engine exposes explicit-target behavior, logical source/selection state,
validated commands, versioned read projections, annotations/analysis and typed
effects/outcomes. UI input resolves to those contracts. UI implementations cannot
bypass authority, currentness, atomic publication or disclosure rules.

The UI owns keymaps/chords/modes, focus, gestures, preedit interaction, painting,
layout, shaping, bidi/cell-width policy, hit testing, visual navigation, scrolling,
decoration precedence/clipping/style, virtual hits and native accessibility/input
bindings. A visual hit becomes an explicit validated logical target before an
engine command. The engine needs neither painted geometry nor a UI framework to
perform ordinary editing or serve automation.

### Clipboard integration boundary

Clipboard interaction spans UI, integration and provider work; it is not all UI
and does not require an OS clipboard implementation inside the semantic engine.

| Layer | Responsibility |
| --- | --- |
| UI | Gestures, shortcuts, menus, focus-to-explicit-target translation and user feedback. |
| Host integration | Orchestrate export/read requests, attributed receipts and subsequent cut/paste requests through the behavior facade; preserve the original operation, capture and target. Automation may use the same integration without a UI. |
| Clipboard provider | OS or session clipboard mechanisms, formats, access permissions and actual acknowledgment/quiescence evidence. |
| Engine behavior and authority | Authorized source/selection capture, immutable logical targets, exact edit validation, local atomic publication, history/annotation/view companions, and truthful separation of external effects from local publication. |

Integration orchestration cannot grant edit authority, manufacture provider
acceptance, refresh a stale target or bypass the existing joint local task/root/
outcome decision. Copy has no source publication; accepted export does not imply
cut deletion, and accepted read data is inert insertion data. Engine insertion
semantics do not depend on whether text originated from a clipboard, file or agent.

The native headless [transfer integration](../crates/vellum-session/src/host/copy_transfer.rs)
and [Editor facade](../crates/vellum-model/src/app/clipboard.rs) implement copy,
acknowledged cut and literal paste over original native invocations, captures and
publication/history. They retain unknown/contested work and separate provider
acceptance from local application. The selected next provider is eager plain-text
AppKit, with acceptance distinct from durability or cross-app delivery. The
`vellum-adapter-clipboard-macos` I/O layer now implements bounded eager plain text,
exception-to-unknown handling, change-count-checked reads and content-free
outcome/timing counters. Foundation encoding tests preserve literal leading
U+FEFF, CR/LF, NUL and combining text; they do not access a pasteboard. Joining
this provider to original native transfer tasks is implemented by
`EditorClipboard`: one pending operation, one provider service, retained receipts
across ingress refusal, independently checked publication, and explicit native
retirement. Its API takes current host premises rather than granting authority;
unknown effects and interrupted provider work cannot be discarded or retried.
Provider receipt summaries remain separate from native local outcomes, including
rejected exports that may already have cleared prior clipboard contents.
`ReferenceClipboard` now supplies the common example workflow to both consumers:
lazy binding at source gestures, delivery-only retries, independent publication
and retirement, and all-tab unsaved preflight before retiring an idle provider.
Iced source-focused Cmd-C/X/V and buttons, and ratatui Ctrl-C/X/V with F12 retry,
use this integration. Host premises are explicit trusted-local-owner policy,
not credential authentication. The 64 KiB plain-text and 262144-byte logical
capture limits are example budgets, not product capacity or latency guarantees.
General-clipboard and live UI qualification remain open. The widgets'
independent clipboard paths remain disabled; no clipboard registry was added to
the semantic engine. Historical F22/F23/F11 design references are not evidence
that the remaining provider/UI work is complete.

An opt-in `vellum-adapter-clipboard-macos` example prepares isolated live qualification:
`cargo run --offline -p vellum-adapter-clipboard-macos --features private-pasteboard-qualification --example private-pasteboard -- --allow-private-pasteboard`.
Run it only after explicit operator approval. It creates unique private AppKit
pasteboards (never General) and temporary source documents, using the same I/O
implementation and native controller as the apps. It checks exact raw text,
bounds, cut/paste, retained receipt delivery, undo/replay and owner cleanup across
three capture modes. Success explicitly clears test data and calls AppKit's
private server-resource release; failures or process
termination do not promise cleanup or physical erasure. Ordinary tests, `--help`
and missing/invalid arguments do not access any pasteboard. A prepared or compiled
probe is not a live qualification result; even a passing private-board run would
not qualify general-board permissions, desktop gestures or cross-app delivery.

The live private-board probe passed on the qualified macOS host on 2026-11-09,
across capture capacities 0, 1 and 256. Exact raw text, bounds, native cut/paste,
retained receipts, undo/replay, native-owner cleanup, explicit server-release calls
and post-release refusal passed. The temporary source files were removed; General
was never acquired. The pinned AppKit binding needs one missing method declaration,
documented in [the vendor patch](../vendor/README.md); first-party unsafe-code
prohibitions remain intact. This is isolated native I/O qualification, not desktop
or cross-app qualification.

## Remaining release work

The initial Unicode candidate probe is now executable via
`make check-unicode-icu4x` and `make check-unicode-icu4c` (see the
[testing guide](engine-test-plan.md)). Stable ICU4X 2.4 passes all 1,944 default
word cases but fails 10/853 grapheme cases. The installed ICU4C 79.1 root
character iterator passes all 853 grapheme cases. The separate `vellum-adapter-unicode-icu` adapter now
integrates the ICU4C grapheme fallback and normative default-word rules over pinned
ICU4X properties with original engine captures. ICU4X's word constructor is not
used: it groups Thai differently from the raw default profile despite passing
the canonical word corpus. The native adapter's complete tables match the pinned
reference across all 2,805 inputs and 1,602 added contexts, and drive real engine
movement/replay/deletion/undo. The terminal reference now explicitly opts into
that provider through the shared editor/session: native movement/deletion,
one exact-version evidence/cache entry, refusal-safe original capture retention
and explicit close cleanup. Terminal frame iteration uses that same grid;
width tables and form editing remain separate. Iced now explicitly enables the
same native horizontal movement/deletion profile; source edits retain incremental
widget patches and version-bound native selection. New visual gestures resolve
through the native grid using the existing 'after' convention, before a logical
target is created; exact SDK points and literal match ranges do not take that
route. Scroll preserves exact partial targets. Widget word/line gestures, shaping,
form input, secondary geometry and OS input-method qualification remain separate.
Provider work counts include both candidate previews and exact-version evidence
construction, not one offer per edit; the UI exposes these populations without
content or document identity. The generic bridge retains legacy behavior unless
its host explicitly opts in. A PTY check of the
built terminal app deleted one Unicode-18-only cluster, saved exact `Z` plus LF,
and quit cleanly using a temporary document; no clipboard access occurred.
Deployment, glyph geometry and desktop qualification remain separate.

| UI item | Work retained for the examples/adapters |
| --- | --- |
| Layout and geometry | Terminal cell/selection mapping and iced's lossless source-to-widget projection exist. Narrow pinned Iced patches expose actual renderer caret/range geometry without moving the primary cursor or reshaping. Iced now paints bounded, clipped secondary ranges/carets and delegates its supported multi-selection edits to native commands; actual widget draw, atomic history and lossless save are tested. Other multi-selection gestures refuse explicitly. Glyph/width/bidi behavior and desktop viewport qualification remain separate; headless rendering tests are not desktop qualification. |
| Decorations and annotations | Both UIs expose native comment operations, separate locks/history and explicit JSON save/reopen. Qualify discoverability, styling, clipping, accessible text and authorized projections; current trusted-host APIs are not untrusted recipient endpoints. |
| Input and interaction | Keymaps, selection, native history and attachment-bound iced preedit are implemented. Actual OS IME, candidate/surrounding-text geometry, native focus/announcements and the remaining gesture/feature parity need qualification. Headless preedit tests are not physical input-method support. |
| Accessibility | Bind semantic engine text/selection/actions to the native accessibility interface. Native focus, announcements and geometry remain UI/platform responsibilities. Do not equate keyboard access with verified screen-reader support. |
| Host/framework support | First qualification target is this exact macOS 26.6.2 arm64 / Rust 1.95 configuration, not a broader matrix or minimum supported version. The qualified ICU4C 79.1 fallback serves source commands in both examples; glyph/width/form profiles and native redistribution remain separate. The rebuilt local GUI bundle passes signature/plist/host-dylib checks. The opt-in real-window probe passes native select/edit/CRLF-save/undo/save/close, four source-region-validated readbacks and inspection of its own synthetic screenshot. It neither injects physical input nor accesses a clipboard workflow or other windows. Physical keyboard/mouse, OS IME, accessibility and general clipboard qualification remain open. Product capacity/performance budgets still need explicit adoption; warm controller or whole-probe timings are not cold-start/presentation latency. |
| Example packages and conformance | Keep both apps thin consumers and qualify the selected release profile through actual integrations. Compare equivalent explicit semantic requests, not pixel equality or identical keymaps. Existing focused native/frontend tests do not close untested desktop capabilities. |

## Existing work to reuse

Retain the presentation-owned portions of specification §§19–21 and
§§28.28–28.34 here, without moving their engine-owned semantic contracts.
Clipboard effect contracts and local publication, host command registration,
semantic configuration and current authorized release remain engine responsibilities;
clipboard mechanisms and workflow orchestration follow the separation above.
The ownership catalog assigns F09/F15/F25,
visual movement, UI host facilities and geometry-result acceptance to presentation.

The [terminal keymap](../crates/vellum-tui/src/terminal.rs),
[iced input routing](../crates/vellum-iced/src/gui.rs),
[attachment guard](../crates/vellum-iced/src/attachment.rs) and shared Editor are
the implementation starting points. Formal keymap/configuration/geometry
references remain historical design inputs, not a second implementation or a
reason to repeat their evidence campaign. The
[engine/UI boundary](../formal/engine-ui-boundary.md) and
[ownership catalog](../formal/engine-ui-ownership.json) preserve the separation;
no engine geometry vocabulary, keymap state or UI service is added.

The bridge's `rendered_geometry` is a UI-only read over the last completed widget
layout. Content actions invalidate readiness; unavailable/unshaped caches, invalid
UTF-8/source projections, changed versions/attachments, mismatched viewport sizes
and insufficient region capacity refuse without partial results or target repair.
Queries reuse the renderer's primary calculation and explicit-position `Before`
affinity. Cached off-screen geometry may be returned; the consuming widget owns
padding, translation and clipping. Results must not outlive their source,
attachment or layout. This does not certify bidi or arbitrary shaping profiles.
The source UI now offers **Select matches** over the existing complete native
search result. Secondary ranges/carets are painted after the source widget using
its actual completed renderer layout, shared 12-pixel padding and explicit
16-pixel font size/default line height. Painting clips to the text viewport and
parent viewport, with a total ceiling of 4096 regions per draw; an unavailable or
over-budget query withholds the entire secondary overlay, never changes targets
or substitutes guessed positions. Exact source/version/attachment/primary-cursor
checks bind the immutable paint snapshot. No engine query or shaping runs inside
the secondary draw query.

Successful **Select matches** returns source keyboard focus through an opaque
widget ID unique to that visual attachment, not a native engine identity. The
focus operation first checks that its target still exists before running Iced's
focus traversal; an obsolete request cannot blur the current source, query or
comment form. These are UI tasks, not new engine verbs or authority. Framework
key-event tests exercise immediate native multi-edit and stale-focus refusal;
the live probe forwards the real focus task before capturing its next frame.

Multi-selection literal insertion, newline, deletion, horizontal movement and
horizontal extension delegate to existing native commands, preserving atomic
history. Select-all explicitly returns to one selection; new search/navigation
can also replace the selection. Scrolling preserves it. Other multi-selection
widget gestures currently refuse rather than silently edit only the primary.
Clipboard and comment workflow qualifications remain their existing profiles;
painting does not broaden their authority or supported selection semantics.

`geometry_usage` and the separate `secondary_geometry_usage` report offers,
completions, refusals and emitted regions with a
sticky saturation flag, without content/identity labels or a fabricated latency
sample. The [pinned patch notes](../vendor/README.md) identify the exact upstream
sources. No common UI framework is imposed on the other engines.

The [closure ledger](specification-closure-ledger.md) records this as L11's
separate presentation stream. Moving it here does not assert UI completion or
silently remove the wider release's adapter/example obligations.
