# Vellum

A headless, agent-friendly Rust editor engine for code and prose.

## Current native baseline

The provisional v1 engine implements lossless source handling, bounded
cross-document edits, annotations, logical views, linked history, retained
queries/search/results, semantic settings and typed publication observations.
Resource operations preserve original ownership and separately attributed
evidence; cooperative shutdown preserves unresolved obligations until discharge.
Instrumentation is optional, bounded and content-safe.

Its diagnostic FIFO storage is supplied by the sibling
[`keystone-observe`](../keystone/crates/keystone-observe/src/lib.rs) crate. Vellum
retains event meanings, opt-in timing and measurement accounting; shared capture
never authorizes or settles an operation. Build with the sibling `keystone/`
checkout present. This first-party mechanism introduces no async runtime or exporter.

UI, keymaps, OS clipboard and automation ID lookup belong above the engine.
Storage mechanisms belong in modular substrate adapters below it. Analysis and
distinct provider-context activation remain disabled; catalog correspondence is
binding-only. Supported native profiles are in the
[contract matrix](docs/public-contract-matrix.md#current-rust-closure-crosswalk--2026-11-07).

## Run and test

```sh
cargo run --offline -p vellum-engine --example headless_edit
make check
```

The example discovers an installed command, edits `Hello, world!` to
`Hello, agents!`, verifies replay without re-execution, reads a separately
authorized retained result, and releases its session owners. It needs no UI,
filesystem adapter, network runtime or external service. Its trusted-host facts
are explicit premises, not a credential system or policy evaluator.

For an actual local file, the new macOS integration example accepts one explicit
byte-range replacement and verifies edit → undo → redo → synchronized save →
reopen → explicit tracked-document cleanup:

```sh
cargo run --offline -p vellum-session --example file_edit -- --help
# Writes the named existing file; example-host input limit is 64 KiB:
cargo run --offline -p vellum-session --example file_edit -- path/to/file 0 5 Hello Hallo
# Opt in to the same workflow through a native logical view/selection:
cargo run --offline -p vellum-session --example file_edit -- --logical path/to/file 0 5 Hello Hallo
# Opt in to original native invocation/replay verification in this process:
cargo run --offline -p vellum-session --example file_edit -- --invocation path/to/file 0 5 Hello Hallo
cargo test --offline -p vellum-session --test local_workflow
cargo test --offline -p vellum-session --example file_edit
```

The integration crate provides one [`LocalHost`](crates/vellum-session/src/host.rs)
for multiple files, atomic cross-document edits and linked history. `LocalSession`
is its single-file convenience wrapper, not a separate engine implementation.
With explicit `CoordinatorConfig::invocation` limits, the host also exposes actual
native invocation issuance, exact-intent submission, once-only execution, retained
outcomes, stop/forget/retire and logical usage. Matching replay reuses the original
ID; it never reruns source or allocates a new attempt. These are trusted-host APIs,
not recipient authorization, cross-process idempotency or a clipboard task ledger.
`capture_text_transfer` adds a bounded, source-neutral capture of an original
native invocation/view/selection context. Verbatim fragments retain document order;
plain data uses LF separators, and all-empty ranges mean no export. Logical byte
counts and redacted diagnostics do not imply provider acknowledgment, structured
handle authority or cumulative host-memory bounds.
Integration-owned `claim_invocation` pins an original issued lifetime. Ordinary
submit/run/stop/forget/retire refuse while claimed; only its opaque non-cloneable
claim can control it. Source-neutral claims cannot execute source work. Native
outcomes remain the publication truth, and discharge retires rather than reuses
the ID. Drop never unlocks a claim. Content-free claim gauges/counters expose
retention and guard refusals without document or identity labels.
The optional [plain-transfer gateway](crates/vellum-session/src/host/copy_transfer.rs)
adds bounded provider registrations, original task admission/replay, one dispatch,
current named-provider payload release, authenticated host-attributed receipts,
stop and guarded retirement. Acceptance, quiescence and conflicting evidence stay
distinct. All-empty copy cannot dispatch or clear a clipboard; copy never changes
source or history. `copy_usage` separates per-operation offers/refusals, task/work
pressure, acknowledgment latency and unknown/work ages; timing Off is `None`.
Unknown/contested work cannot be retired or bypassed by unpacking the host.
`admit_cut` retains the same export task with a private publication-capable native
claim. `continue_cut` requires accepted uncontested export, checks current grants
and registration, and deletes only the original ranges through native literal
editing. Permission/staleness/capacity refusals are spent terminal decisions;
retries never reacquire focus or reapply a cut after Undo. Native source, annotation,
peer-view and history publication remains shared. `cut_report` joins current effect
evidence with the retained local decision, reporting copied-not-cut only when known.
`configure_paste_transfers` adds explicit target/plain/expanded-byte limits over
that same task/provider owner. `admit_paste` captures target identity and selections,
not source fragments; dispatch revalidates them before requesting data. Authentic
bounded plain receipts retain a canonical body and capped conflicting witness.
`continue_paste` broadcasts the whole literal at every original target through the
native publisher, without focus refresh, newline splitting or key replay. Empty
data is a real paste, not missing evidence. Retained native outcomes distinguish
Applied, Unchanged and spent refusal; undo never repeats provider work. Admission
reserves receipt/native text buffers; owned native literal admission consumes the
preallocated command text. Logical reservations are not physical RSS guarantees.
This is a trusted provider-facing boundary, not an OS clipboard adapter, selected
production acknowledgment/retention profile or session-structured transfer registry.
The tests use an explicitly test-only provider profile and never read or modify
the OS clipboard.
The shared `Editor` now retains original clipboard plans across tab changes,
adopts each native undo group once and refuses close while tasks/provider controls
remain undischarged. The iced bridge binds initiation to its original visual token
but routes late completion through the captured plan, refreshing only a newly
applied active target. Matching replay does not rebuild the widget; background
completion does not steal focus. These provider-neutral routes require explicit
limits, native invocation configuration and trusted host facts. Both reference
apps now use the eager AppKit plain-text provider through a shared integration
workflow, lazily bound at an explicit source gesture. Retained delivery retries
never repeat platform work; uncertain work blocks new transfers and close.
Live OS clipboard qualification remains open; there is no session-clipboard fallback.
Caller-selected logical-view and line-edit profiles expose original native
view/context handles, normalized multi-selections, logical endpoints, literal
replacement and explicit prefix/newline transformations. Source, initiating/peer
selections, annotations and history use the existing shared publisher. No focus,
geometry, implicit context refresh or Unicode provider is introduced. The
`file_edit --logical` route checks its expected fragment and discharges its view.
Both reference UIs now attach original native primary views, publish validated
logical selections and use native selection-derived literal edits when applicable;
The terminal also supports native multi-selection editing from complete search
results; iced secondary-selection visuals and provider-backed movement parity
remain open.
With caller-configured `SearchLimits`, `LocalHost` also exposes native literal
search, navigation and replacement through original session/result handles.
Partial results remain explicit; stale/foreign/superseded results refuse action,
and replacements use the same publisher and undo history. Both reference
frontends use this route for literal find/navigation and current/all replacement.
With caller-configured `AnnotationLimits`, the SDK also exposes exact native
compound source/comment publication, reads/snapshots, separate annotation policy,
annotation-frontier undo/redo and guarded absent-record retirement. Both point
profiles (`position` and `content`) remain explicit. These are trusted-host APIs;
the shared editor and terminal now expose session comments. F10 comments each
native selection (empty targets: content points); Shift-F10 chooses position points.
Enter adds LF in the multiline form, Ctrl-S commits it and Esc cancels. F11 lists,
edits, deletes and reattaches comments, with native U/R history and separate L lock.
Source saves/Save As do not persist comments; close requires explicit
discard while comments remain unsaved. The iced Comments pane exposes the same native
create/edit/delete/reattach, separate lock and note-history operations. Its bounded
multiline form preserves raw body bytes behind the display projection; Cmd-S
applies to the session, while Cmd-Z/Shift-Z only undo/redo unapplied form edits.
Stale source/form attachments refuse Apply, and unapplied forms also gate close.
Both apps expose explicit comment-file save/reopen: terminal F11 then S/O, or
iced Save JSON/Load JSON. Select an existing, distinct JSON destination and confirm
the initial association (terminal Y); entering a path alone never writes it.
The host assigns opaque UUIDv4 document/record labels, not authority. Fresh-owner
loads verify exact current source evidence; missing historical witnesses refuse
activation and fence Save, preserving the original JSON. No automatic sidecar or
durable source-history store is implied. Native metadata reads avoid source snapshots for dirty
state, and unchanged body caches survive location mapping.
`bind_annotations` accepts an explicit existing, distinct destination. The SDK's
JSON encoding/queue/service/baseline routes preserve original save attempts and
host-supplied identity associations. The shared `Editor` now exposes explicit
`bind_comment_file`, `capture_comments`, `save_comments` and separate comment-save
outcomes for an embedding that supplies those identities and source witnesses.
Successful acknowledged writes establish an independent dirty baseline; deletions
must be saved even when no records remain. Source/set changes conservatively mark
that baseline dirty. `load_comments` reads an explicitly associated file into an
untouched annotation set using caller-approved identities and managed source
witnesses. It retains native read/restoration facts even after a decoding,
activation or baseline-adoption failure. Loaded baselines are not write receipts;
adoption can retry independently without re-reading or re-restoring. Malformed
JSON, missing historical witnesses and existing authored comments refuse without
overwriting either file. Explicit failed-read/checkpoint retirement and close
discharge the actual native owners. Drop native captures before close. Optional
`enable_comment_files` supplies the reference host's bounded UUID/witness policy;
explicit external identity mappings remain supported independently.
Annotation bindings reject changed destinations rather than silently adopting a
new probe; only an actual attributed written observation advances their guard.
Source saves do not imply annotation saves,
and saving both resources is not a two-file atomic commit. The SDK save path is
tested against real files. The SDK also exposes attributed read attempts,
bounded decoding and native restoration with explicit approved identities/source
witnesses. Loaded-baseline adoption is separate from decoding/restoration;
retained reads remain queryable and can refuse retirement while still pinned.
Fresh-owner round trips preserve both point profiles without reviving old handles.
Restored read attempts can retire after an independent current checkpoint, or
after explicit document disposition and provenance release. Real decoded-data
loans still block those routes; endpoint detachment is separate. The tested
disposal path reclaims native payloads without deleting either persisted file.
Both are trusted native embeddings, not untrusted automation sandboxes. Original
save tasks survive service errors, and current baseline tasks can remain retained.
The host exposes explicit disposal, source-binding/history/provenance release
and guarded reclamation; caller-supplied premises cannot override actual pins.
These steps are not file deletion, physical erasure or whole-engine shutdown.
Process exit is not a completion receipt; advanced shutdown retains the original
owners through `into_parts()`. Unpacking now returns `Result`: outstanding
integration claims or copy-provider registrations return the intact host/session
in `Err`, not a detached engine
that could bypass them. Retire the original claims explicitly before retrying.

SDK source copy export and Save As also use the shared writer. Keep the original
preparation/attempt, supply current host authority facts, and target an existing
supported file. Save As write acknowledgment and binding activation are separate;
activation returns a new binding-aware document handle. Old handles are not
silently redirected. Missing files refuse—this is not an exclusive-create profile.
Both frontends expose these routes with an explicit version-bound overwrite
confirmation. TUI: F2 Save As / F7 copy export, enter a literal existing path,
then Y confirms or Esc cancels. GUI: use the path bar and Export copy / Save As,
then Confirm overwrite. An intervening edit/context change cannot refresh the
original confirmation. The sample binding-history limit is 16, not a selected
production budget. Copies leave original saved-state unchanged.

### Terminal editor

```sh
cargo run --release --offline -p vellum-tui -- path/to/existing-file another-file
```

The ratatui example uses one host across tabs: type/paste committed text,
arrows/Home/End, Backspace/Delete, Ctrl-S save, Ctrl-Z/Y undo/redo, F4 editing lock and F6 next
file. Shift-arrows/Home/End extend a highlighted selection; Ctrl-A selects all.
Typing/paste and Backspace/Delete replace selected text through one native edit;
plain Left/Right collapse the selection. Keybindings, grapheme projection and
clipped painting stay in presentation; native views own the validated logical
selection and source publication maps it with history. Ctrl-O opens another
existing file through a literal UTF-8 path prompt
(4096-byte frontend input bound, no shell expansion; Esc cancels). Reopening an
owned file preserves unsaved edits. Non-UTF-8 paths can use startup arguments.
F5 indents selected/current lines; Shift-Tab outdents. Both UIs use native
four-column prefix transformations, including column-aware mixed tabs/spaces;
ordinary terminal Tab remains literal insertion. Only edited prefixes change.
Ctrl-F enters a literal query, F3/Shift-F3 navigate, Ctrl-R replaces the original
current match and F8 replaces all complete results. Empty replacement deletes;
Esc cancels entry. The single-line terminal prompts refuse control characters.
Search must be explicitly rerun after edits; stale targets are not refreshed.
F9 selects complete current matches within the native reference budget (16 input
selections). The count and all visible ranges/carets are rendered; typing/paste,
deletion, movement/extension, indentation and history operate on the whole native
set. Stale, partial or over-budget results refuse without a primary-only fallback.
Selecting a new search match or Ctrl-A replaces the set explicitly. This terminal
slice does not qualify iced secondary-selection geometry or native Unicode18 movement.
Ctrl-Q quits; unsaved tabs require a second press to discard, and Esc
cancels. Save refusals are visible and never trigger automatic retry or overwrite.
Normal quit explicitly discharges tracked document/history/save owners before
terminal restoration; this is not physical erasure or a whole-engine certificate.

The reference application profile is macOS/local-filesystem only, 64 KiB per
file, finite history/resource budgets and LF for generated newlines. It preserves
existing BOM and mixed endings. The terminal reference explicitly enables the
qualified Unicode 18 provider: horizontal source movement and deletion use native
engine commands; vertical projection, painting and post-edit caret projection
share its exact-source grapheme grid. One retained evidence/cache entry is reused
until the source version or document changes, then retired; close discharges it.
Pinned terminal-width tables and form-input behavior remain separate. The iced
reference also explicitly enables native Unicode 18 source horizontal movement
and deletion, with incremental original-coordinate widget patches. New visual
gestures resolve to the native grapheme grid using the existing 'after' convention;
exact SDK positions and literal match ranges are never rounded or repaired.
Widget word/line gestures, shaping and form editing retain their separate profiles.
Controls/bidi overrides are escaped for terminal
display without altering source. Ctrl-C/X/V invoke source copy/cut/paste through
the native integration; F12 retries retained delivery, not platform I/O. Ctrl-Q
remains quit. Mouse, live OS clipboard, IME preedit, bidi layout,
screen-reader support and full engine-feature UI parity are not yet qualified.

Run `cargo run --release --offline -p vellum-editor --example interaction` for
the bounded warm-process insert/frame/backspace/frame workload. It checks content,
diagnostic loss and owner cleanup across off/enabled/saturated capture. This
measures presentation preparation plus engine service—not terminal drawing,
filesystem write time, cold startup or an adopted product latency SLO.
With `PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig`, add
`--features native-unicode` before `--example interaction` and
`-- --native-unicode` after it to measure the native editor path. The initial
warm insert/frame/backspace/frame run measured roughly 4.9 ms p99 at 63,996 bytes,
with actual source, evidence-cache and cleanup checks; terminal drawing is excluded.

### Iced GUI reference build

`PKG_CONFIG_PATH=/opt/homebrew/opt/icu4c/lib/pkgconfig make app-macos` builds the
local, ad-hoc-signed `target/Vellum.app` against the qualified ICU4C 79.1 install.
Native dylibs remain host prerequisites; this is not a portable bundle. It starts
with an existing-file path entry when no CLI arguments are supplied. The GUI
uses the same host/history/save owners, a software renderer and a mapped LF-only
display projection; original source endings are never serialized from the widget.
Cmd-S/Z/Shift-Z, toolbar actions, selection and explicit discard operate through
the existing engine. The path entry can open additional files in the same host;
reopening an owned file preserves unsaved edits. F4 or the toolbar toggles the
engine-enforced source editing lock in either frontend. This lock is not an OS
permission change; the native policy refuses edits, undo/redo and ordinary saves
until explicitly unlocked. Navigation remains available.
The actual widget tree is attachment-bound: file/history/context changes cancel
retained preedit/drag state, and window blur clears source preedit. Headless
framework-event tests cover inert preedit, committed Unicode through native
history/save, late-commit refusal and keyboard-focus continuation. They do not
qualify an OS input method, candidate-window geometry or desktop interaction.
Literal search/replacement fields and buttons use the same engine results;
F3/Shift-F3 navigate. Search again after edits before another replacement.
Reference search limits include a 1024-byte query, 1,048,576 comparisons and
4096 matches; exhaustion is explicit and does not authorize navigation or
replace-all from partial results. These are sample-host budgets, not a selected
production release profile. Replacement fields are bounded at 64 KiB.
Source-focused Cmd-C/X/V and source toolbar buttons use the native clipboard
integration, not the widget's independent clipboard backend. Note forms and path
fields cannot retarget a source transfer. Retry delivery does not repeat platform
I/O. The reference host explicitly supplies trusted-local-owner premises; this
is not credential authentication or an untrusted-client authorization service.
Live OS clipboard qualification remains open. This is not a notarized distribution or a production
Unicode/IME/accessibility/bidi qualification; desktop interaction testing still
requires approval to launch this newly built application.

Run `cargo run --release --offline -p vellum-iced --example iced-interaction` for the
headless widget/controller workload. It checks exact source behavior, saturation
and the absence of full-buffer refresh during ordinary typing; it does not measure
window drawing, event-to-screen delay or cold startup.
Experiment flags (`--source-bytes`, `--samples`, `--warmup`, `--workload`) permit
bounded short-line, long-line and Unicode-prose characterization without changing
application limits. See `--help`.
`--unicode-profile native` measures the native source path with preview/evidence
population and cleanup checks; `legacy` preserves the previous baseline. The
initial warm native 63,996-byte insert/backspace round trip measured about
9–13 ms p99 across capture modes, excluding drawing, OS events and disk writes.
The status line reports aggregate Unicode provider work, not an edit count.
The example has a unique target name to avoid overwriting the shared editor's
`interaction` binary. CSVs separate controller insert/backspace service from
full-buffer oracle readback; their percentile columns are not additive. Attachment
and explicit close service have separate clocks, not hidden setup costs.

On the current arm64/Rust 1.95 host, 512-sample/32-warmup synthetic short-line
cohorts across capture off/enabled/saturated observed controller p99 below
88 µs / 1.0 ms for 63,996-byte / 1,048,572-byte inputs, respectively, with native
logical views enabled. The 1-MiB run's whole-process peak RSS was about 117 MiB;
this is not per-document attribution or a memory guarantee.
The 1-MiB run used 15.2 CPU seconds across all three cohorts. Bounded literal
attachment batches avoid both eager full-document shaping and the pinned widget's
quadratic large multiline paste. The rejected one-paste experiment saved memory
but used 65.1 CPU seconds; it is not the shipped attachment path.
Earlier pre-native-view 1-MiB Unicode-prose/long-line runs passed the same exact-content and
cleanup oracles; controller edit p99 stayed below 0.79/0.84 ms, with process peak
RSS about 63/38 MiB. These are synthetic shape observations, not isolated causal
overhead measurements. Attachment/close costs are not included in edit percentiles.
The hot path validates selected/changed lines and adjacent joins/splits over its
private widget buffer; attachments and workload oracles retain full-buffer checks.
These observations are not screen latency, an adopted SLO or permission to raise
the reference apps' 64-KiB limit. Larger-file memory and real viewport qualification
remain open.

`Editor` and the iced bridge expose a bounded, content-free native trace window.
Engine capture drops, frontend-window overwrites and full-projection refreshes
are separate observations; trace volume is not an authoritative operation count.

`make check` runs formatting, strict Clippy, workspace tests and doc tests.
Use focused crate/test targets while developing; reserve full qualification for
integrated changes and release checkpoints. See the
[testing guide](docs/engine-test-plan.md) for additional gates and instrumentation.
Cargo checks run offline with dependencies already cached; `Cargo.lock` is kept.
The development toolchain is Rust 1.95.0, not a selected MSRV.

All seven native delivery gates closed on 2026-11-07. Debug and release each
passed 2,880 tests across 88 targets; the sole ignored release-only experiment
was separately qualified. This is a provisional native baseline, not a wire
protocol, production-provider certification or adopted product latency SLO.

## Workspace and boundaries

| Crate | Responsibility |
| --- | --- |
| [vellum-engine](crates/vellum-engine/src/lib.rs) | Semantic owners, behavior facade, authorization guards, retained outcomes and bounded instrumentation; no third-party dependencies |
| [vellum-session](crates/vellum-session/src/host.rs) | One trusted host for multiple local files; atomic edits, linked history, explicit save/lifecycle operations and bounded diagnostics; no UI or policy evaluator |
| [vellum-clipboard-macos](crates/vellum-clipboard-macos/src/lib.rs) | Main-thread AppKit plain-text I/O and original-operation Editor adapter; shared reference workflow wired into both UIs, retained receipts, explicit outcomes, lossless bytes and content-free measurements; live OS qualification remains open |
| [vellum-editor](crates/vellum-editor/src/app.rs) | Shared presentation/interaction state above the host; cursor, viewport, status and explicit application-owner cleanup |
| [vellum-tui](crates/vellum-tui/src/terminal.rs) | Ratatui projection and terminal-only key bindings; no independent source or history owner |
| [vellum-iced](crates/vellum-iced/src/bridge.rs) | Iced widget projection, original-coordinate edit translation, attachment tokens and GUI-only bindings; no independent source/history authority |
| [vellum-local-fs](crates/vellum-local-fs/src/lib.rs) | Separate first substrate adapter; conservative native macOS filesystem profile |
| [vellum-annotation-json](crates/vellum-annotation-json/src/lib.rs) | Separate reference annotation persistence codec |
| [vellum-unicode-reference](crates/vellum-unicode-reference/src/lib.rs) | Pinned reference-provider conformance adapter, not an engine dependency or production-provider selection |
| [vellum-unicode-icu](crates/vellum-unicode-icu/README.md) | Bounded in-process Unicode 18 adapter: qualified ICU4C graphemes, normative default words and pinned properties; opt-in shared-editor integration enabled by both reference apps; rendering and native distribution remain separate |

### First local-filesystem adapter

Filesystem guarantees belong to the adapter, not the semantic engine. Extend
the existing module rather than building a second one; its platform tests define
the supported profile. See the [adapter handoff](docs/filesystem-adapter-handoff.md)
and [substrate design](docs/substrate-module-design.md).

## Where to look

- [Requirements](requirements.md): intended behavior and constraints.
- [Specification](specification.md): detailed adopted contracts.
- [Architecture/model](model.md): ownership, dependencies, flows and formal model.
- [API boundaries](docs/engine-api-boundaries.md): engine versus consumer/substrate responsibilities.
- [Native contract matrix](docs/public-contract-matrix.md): supported profiles and owner bindings.
- [Testing guide](docs/engine-test-plan.md) and [performance workloads](docs/performance.md).
- [SDK/automation/UI handoff](docs/sdk-automation-ui-handoff.md): consumer-layer work.

Code and executable tests establish implemented behavior; design documents do
not by themselves establish implementation or certification. Change the relevant
contract when behavior changes, rather than copying progress reports into every
document. Keep routine development history out of active guides. Earlier slice
notes are retained in [README history](README.history.md) and
[acceptance-plan history](docs/engine-test-plan.history.md) for reference only;
their intermediate “open” statements are not a current work queue.
