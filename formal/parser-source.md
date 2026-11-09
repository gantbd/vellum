# Committed parser source bridge

Specification §18.5 supplies tree-sitter-utf8-source/1: bounded owned source
transformation receipts and exact parser input products. The actual reference
source/annotation, ordinary-selection and view-aware history coordinators mint or
carry these receipts. This is the source-input portion of analysis preparation,
not completed F24/F28 admission, a native parser or a mandatory grammar dependency.

## Precedent and coordinate domain

[Tree-sitter basic parsing](https://tree-sitter.github.io/tree-sitter/using-parsers/2-basic-parsing.html)
defines raw byte offsets, zero-based LF rows and byte columns. Only LF advances
the parser row. CR remains a byte in that row, including the CR of CRLF. Therefore
the parser can represent the scalar boundary inside CRLF which the editor's
logical-line converter deliberately refuses. UTF-16, grapheme and display columns
are not substitutes. Literal U+FEFF remains source, not a stripped file signature.

[Tree-sitter editing and concurrency](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)
specifies TSInputEdit and cheap tree copies before mutable edit/reparse work.
The source bridge constructs the edit records; actual parser/tree ownership and
incremental/fresh equivalence still need the pinned integration. Tree-sitter's
native byte/point fields are uint32_t, so this adapter bounds coordinates to at
most 4,294,967,295. That does not choose the authoritative core's integer width or
a measured document-size default. A smaller declared profile may refuse earlier.

| Input choice | Benefit | Cost and disposition |
|---|---|---|
| Owned committed transformation chain | Preserves exact transaction edits, revisions and parser starting snapshot | Requires retained bounded journal; selected incremental input |
| Reconstruct a diff from equal/current text | Can omit transformation retention | Does not establish the actual committed transformation; not used to invent missing edits |
| Fresh identified source | Works without required history and after incompatible baseline | More parsing work, subject to fresh-work admission; selected fallback |

## Publication and receipt ownership

Journal preparation uses explicit finite document/source/batch/edit/serialized-byte
limits and a reservation predicate before source handoff. It validates each
canonical batch against its actual intermediate source, exact resulting bytes and
the fresh source revision. A private staged token is not yet a committed receipt.
Commit binds that token to the exact completed candidate root and matching source
frames, then consumes it once. Root lookup and receipts are owned opaque values;
cloning a root or reference cannot forge an association.

The source coordinator registers a journal only after all source/annotation/history
checks succeed. Multi-document publication keeps independent identified receipts
under the one publication. Ordinary editing and current-view reversal carry the
association through their private view-aware result roots only when publication
and source frames agree. Failed complete publication returns no new receipt on the
returned current root. Undo/redo use the actual retained forward/inverse sequences,
not a reverse diff or restored historical revision. A metadata/view-only change
does not fabricate a source transform.

This journal is optional derived evidence. Missing reservation, absent/invalid
profile or exhausted journal limits does not reject otherwise valid ordinary
editing. The parser must then use an identified fresh snapshot if its separate
capture/work budget permits, or report unavailable. Journal failure does not
authorize an unbounded native parse. The reference copies flat source/receipt data;
aggregate retention, allocator behavior and accepted/native trees require their
own accounting. No production rope, event log or persistent journal is selected.

```mermaid
flowchart LR
  A[Private candidate and bounded journal] --> B[Complete source and view publication]
  B --> C[Owned committed source receipts]
  C -->|Exact retained chain| D[TSInputEdit stream and captured source]
  C -->|Missing or over budget| E[Fresh identified source]
  D -. Separate F24 admission .-> F[Optional parser worker]
  E -. Separate F24 admission .-> F
```

## Parser input products

Source snapshots name an exact document incarnation, revision and canonical UTF-8
bytes. Current authority/read/continuity/liveness/stop/capture predicates and a
complete finite input profile precede preparation. Product modes are incremental,
fresh and source-only unchanged; fresh reasons are no-base, incompatible-base,
missing-journal or journal-budget. Invalid target bytes, same-version conflicting
bytes or an output that cannot fit refuse instead of being normalized or rounded.

Incremental input requires an owned contiguous same-document chain: every receipt's
from revision and before bytes equal the current intermediate, and its final
revision/bytes equal the target. Missing, copied, reordered, duplicate or unrelated
receipts cannot be treated as a complete chain. Equal bytes at a newer revision do
not become unchanged; a retained undo chain is still applied, or fresh input is
required. Source-only unchanged says nothing about grammar, query, registry,
discovery, annotations or configuration freshness.

Apply each canonical source batch from highest original byte offset downward.
Earlier high-offset edits do not move lower original prefixes. For every actual
step emit start_byte, old_end_byte, new_end_byte, start_point, old_end_point and
new_end_point in its then-current source. Inserted LF counts and trailing UTF-8
bytes determine the new endpoint; CR never resets its column. Bound intermediate
sources, receipt/batch/edit counts, native coordinates and complete plan JSON bytes.
If incremental preparation cannot fit, fresh input must independently fit; it does
not inherit unlimited budget. Target bytes are always preserved exactly.

## Arguments checks and limits

For one batch, descending disjoint edits preserve each unprocessed lower prefix;
the step's old endpoints therefore address its current source. Replacing that
range yields the exact new endpoint and source. Induction over descending edits
establishes batch equivalence; induction over owned receipts with matching before/
after versions and bytes establishes chain equivalence to committed publication.
Point conversion counts LF in the prefix and bytes after its last LF. Its inverse
selects that LF row and checks byte-column/scalar boundary, giving the conditional
round-trip law over supported canonical boundaries. These are authored construction
arguments, not proof-assistant-checked native parser correctness.

Executable evidence covers 930 byte/point cases, 996 single-edit and 22,770 paired-
edit cases, 27 three-publication chains and 64 capture-gate combinations. Fifty-one
controls and twenty-one load-success/assertion-failing mutants cover actual forward/
ordinary/history composition, coalescing, LF/CRLF/CR/multibyte/U+FEFF distinctions,
version continuity, missing-history fallback, staged/committed ownership, frame
carry, budget exhaustion and refusal without authoring failure. Mutated producer,
compiler and wrapper graphs share their actual private ownership maps; failed
imports do not count as kills. Existing source/history/search checks remain in
the full suite. This is not source-isolated independent Cleanroom certification.

Native Tree-sitter execution, actual parser ownership/tree copying, grammar/query/
discovery/region contexts and assets, analysis F24/F28 lifecycle, malformed-source
tree behavior, highlighting and mixed-language conformance remain gates. So do
all publication paths, actual policy/capture/retention truth, hostile decoding,
complete companions, physical/concurrent refinement and measured defaults.

Section 18.6 separately executes scoped pinned native parser/query comparisons
and mixed-language reference cases; see [tree-sitter-reference.md](tree-sitter-reference.md).
It does not widen this source bridge's isolated scope flags or prove full native
semantics, analysis task admission or production integration.

## Instrumentation and threat coverage

INS-02/07 identify committed transformation handoff; INS-05/09 distinguish journal
capture, input preparation, incremental/fresh/unchanged/unavailable and subsequent
parse/acceptance. INS-01/03/08 retain interpretation/release/resource coverage.
MELT/golden/RED track capture/compile/fallback and useful later parse outcomes in
consistent populations; a fast fresh-source plan does not prove fast parsing.
USE tracks retained source/journal/plan bytes and edit/batch pressure separately
from actual native tree/memory/work. STRIDE includes forged/gapped/reordered
receipts, wrong document/revision, coordinate overflow and budget bypass. LINDDUN
includes source disclosure, retained deleted text and edit cadence. Use bounded
mode/reason/budget classes, not source, positions, document IDs or history tokens
as labels. Actual signal installation, privacy and overhead still need evidence.
Internal publication/inspection helpers are trusted composition facets, not a
public plugin ABI or protection against malicious same-process code. Requirements,
annotation point persistence, host matrix and production dependencies are unchanged.
