# Simple line indentation and generated newlines

Section 26.1.2.1 additionally composes these transforms with actual owned semantic
settings through configured-line-edit/1. See [configured-line.md](configured-line.md)
for explicit bindings, current native generations, retained interpretation and
scoped evidence. The explicit product below remains usable without that service;
its supplier premises do not describe the newer configured product's ownership.

Specification §26.1.2 refines F11 ordinary-edit with owned line-edit/1 products.
The explicit-prefix-line-edit/1 profile works on prose, code and mixed documents
without requiring a grammar. It lowers exact source edits into the existing
source/annotation/all-attached-view coordinator, not a parallel mutation pathway.

## Precedent and profile choices

[CodeMirror commands 6.8.1](https://raw.githubusercontent.com/codemirror/commands/6.8.1/src/commands.ts)
separates selected-line shifting from syntax-assisted indentation. Its line helper
avoids repeating a line and excludes a nonempty selection's terminal line when
the upper endpoint is that line's start. Configured indentation units, tab-column
outdent and preserving common prefix bytes provide useful precedents. Vellum
retains its own UTF-8 coordinates, LF/CRLF/CR recognition, annotation mapping and
explicit multi-selection conflict rules. This is not adoption of upstream code,
all whitespace classification, keybindings or newline heuristics.

| Choice | Benefit | Cost and selected boundary |
|---|---|---|
| Explicit space/tab prefix shifts | Deterministic without language/geometry support; existing bytes outside exact edits stay intact | Not syntax-aware and a prefix insertion before tabs need not add a fixed number of columns; selected named base profile |
| Grammar-driven indentation | Can respect nested language structure | Requires accepted grammar/configuration/evidence and bounded provider work; optional separate profile, not inferred from simple shifts |
| Whole-file formatting or detected newline normalization | Can impose a uniform style | Alters unrelated authored bytes; never a side effect of these commands |

No universal two/four-space preference is chosen. Indent settings are an explicit
nonempty run of ASCII spaces or one tab, plus a positive tab width. They are
accepted invocation values, not arbitrary script expressions. Tab expansion for
indentation columns advances to the next declared tab stop; these columns are
not UTF-8 byte columns, rendered glyph widths or a general geometry API. A tab
key may invoke a line shift, insert literal text, or transfer native focus under
the adapter's explicit binding; this profile does not silently choose among them.

## Exact command products

line-edit/1 carries ordinary-edit, action, invocation, exact view/document
incarnations, source revision, expected selection generation, named profile and
settings. The three closed actions are indent-lines, outdent-lines and
insert-newline. Current effective settings must match at preparation/publication;
the existing interpretation-current supplier premise still carries configuration
identity/applicability truth. Value comparison is not a proved settings registry.

Every selection endpoint must be an exact logical-line caret. CRLF interior is
unavailable, not rounded. A collapsed selection targets its containing line,
including the empty EOF line. A nonempty selection targets intersected logical
lines but excludes a line beginning exactly at its upper endpoint. Reverse ranges
select the same lines. A shared line is edited once, even for several carets.

Indent prepends the configured unit unchanged. Outdent measures only leading
ASCII space/tab columns and reduces their count by the unit's width, floored at
zero. A tab unit emits tabs plus residual spaces; a space unit emits spaces.
Keep the longest byte-identical leading prefix and replace only the differing
suffix. Other line bytes, delimiters and unrelated lines remain exact. An empty
outdent is identity. Indent followed by outdent is not generally a byte inverse
for mixed prefixes; retained source history, not an assumed algebraic inverse,
recovers the actual original bytes.

Generated-newline settings declare LF, CRLF or CR, and none or copy-prefix
indentation. Replace each exact selected range with the chosen delimiter and,
when requested, the starting line's leading ASCII spaces/tabs before the lower
insertion endpoint. Do not copy whitespace after that endpoint, strip existing
indentation, fetch a grammar, trim trailing spaces or normalize other newlines.
Collapse origin selections after their actual inserted bytes. Paste/literal
ordinary insertion remains literal; the new-document LF policy is already in
requirements §5.9, not inferred by this helper. A loaded document's insertion
policy must still be supplied explicitly.

## Publication and finite argument

The trusted internal derived-edit helper shares the actual ordinary-edit
normalizer, canonical simultaneous-batch validation, current annotation dependency,
all-attached-view capture/mapping and complete-root publication. It is not a new
public verb or plugin ABI. Colliding derived insertions refuse wholly rather than
invent ordering or silently coalesce an agent batch. Current permissions, source/
selection generations, peer membership/profile, stop, companion and capacity
gates still apply. Viewport changes are preserved; pure identity creates no group.
Nonidentity edits create one local publication and one source group. Native F14
undo/redo maps current views through retained transforms; it does not restore old
cursors. Equal-byte annotation/nontext classification remains a separate gate.

Finite admitted source/selection extents bound scanning. Distinct-line count,
total copied/scanned prefix bytes, actual before/after indentation columns, unit
bytes and derived request/edit JSON bytes have explicit finite caps. Existing
selection profiles also cap inserted text, edit bytes, views and selections.
Measure actual resulting prefix columns: tabs make a simple current+unit formula
unsound. Publication checks current work caps and the captured selection profile.
Reference UTF-8 JSON charges are logical accounting, not a chosen native/wire
layout or an allocator, latency, reclamation or whole-plan ownership theorem.

For accepted shifts, the derived edits change only targeted prefixes. Canonical
batch emission preserves every source byte outside those edit intervals in order.
Outdent rendering has exactly the requested floored column count because full
tabs contribute the declared width and residual spaces are strictly fewer than
that width. Newline inserts only the explicit delimiter/copied prefix. Existing
publication frame arguments then apply conditionally to all supplied companions.
These arguments do not infer arbitrary language indentation or global termination
from a closed verb list; all loops also rely on finite admitted inputs.

## Evidence and instrumentation

Actual-module composition checks 256 prefix/oracle cases, 96 newline cases,
512 final gate combinations, 55 controls and eleven load-success/assertion-failing
mutants. The independent prefix oracle expands tabs to space strings rather than
using the candidate column helper. Checks cover forward/reverse/overlapping
selection lines, same-line carets, EOF, mixed delimiters, non-ASCII content,
common-prefix annotation survival, current settings/resources, peer/view/annotation
staleness, input immutability and actual native undo/redo with coherent views.
This is same-author scoped reference evidence, not independent Cleanroom proof.

INS-01/09 observe command/settings interpretation, INS-02/07 publication and source/
annotation/view mapping, INS-06 reversal, INS-11 selection handoff and INS-08 work/
retention pressure. MELT/golden/RED distinguish prepare/refuse/unchanged/applied
and queue/service/publication latency; USE tracks scanning, derived buffers and
participant pressure. STRIDE covers stale settings, write-scope bypass, skipped
views and expansion exhaustion. LINDDUN covers whitespace/style fingerprints,
selection streams and hidden annotation/view existence. Use bounded reason/budget
classes, not source, prefixes, coordinates, settings values, IDs or plans as metric
labels. Actual signal installation, release policy and measured overhead remain gates.

Full movement/Unicode/geometry, grammar indentation, native hostile decoding,
settings supplier truth/replay/outcome/event integration, all companions and
physical/concurrent implementation refinement remain open. Point-annotation
storage and public configuration key names are unchanged.
