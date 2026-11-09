# Unicode reference conformance and logical commands

Specification §26.1.3 connects owned exact-source segmentation evidence to F12
logical-movement and F11 ordinary deletion. The named Unicode 18/UAX #29 revision
49 profile is unchanged. Reference conformance now has executable evidence;
production/incremental provider selection and native conformance remain gates.

## Pinned standard and reference provider

[UAX #29 revision 49](https://www.unicode.org/reports/tr29/tr29-49.html) defines the
adopted default extended-grapheme and word rules. The unmodified official
[grapheme](https://www.unicode.org/Public/18.0.0/ucd/auxiliary/GraphemeBreakTest.txt)
and [word](https://www.unicode.org/Public/18.0.0/ucd/auxiliary/WordBreakTest.txt)
corpora and their license are retained as flat formal inputs, with version/count/
SHA-256 pins in unicode-boundaries.json. The verifier parses expected UTF-8 byte
offsets independently from corpus markers; reference-tool UTF-16 offsets require
an explicit scalar-safe byte conversion. Source text is never normalized.

[ICU 79.1](https://unicode-org.github.io/icu/download/79.html) supplies Unicode 18
property data and the root extended-grapheme iterator for this reference tool.
The external jar is pinned by SHA-256, its runtime ICU/Unicode versions checked,
and its digest/version recorded in results. It is not an engine dependency choice.
The local Node runtime's Unicode 17 and cached unicode-segmentation 1.13.3 Unicode
17 data are not substitutes for the adopted version. Re-evaluate provider versions
explicitly rather than treating the newest installed library as matching evidence.

ICU's [word iterator includes automatic dictionary segmentation](https://unicode-org.github.io/icu/userguide/boundaryanalysis/).
Although its root iterator passes the official finite word corpus, supplementary
Thai/Japanese fixtures distinguish it from the untailored default. The reference
therefore evaluates the ordered default word rules directly over pinned ICU
properties, including ignored characters, punctuation lookaround and regional-
indicator parity. It does not import ICU's dictionary word result as the default.
This is a standards-derived reference evaluator, not a newly selected engine
algorithm, independent implementation oracle or proof of every Unicode string.

The Java tool has fixed batch/scalar/input/output/time limits. Its source, raw
corpora, license and metadata are captured and hashed with other formal inputs.
The complete suite requires VELLUM_ICU4J_JAR; omitting the tool cannot silently
produce a complete pass. Java capture/tamper and tool-only nongeneration controls
extend the evidence harness to 46 checks. The verifier never downloads the jar.
The reference calls a subprocess; no such execution belongs in the editing path.

## Word endpoints and explicit engine commands

[Rust UnicodeSegmentation's public word contract](https://docs.rs/unicode-segmentation/1.13.3/unicode_segmentation/trait.UnicodeSegmentation.html#tymethod.unicode_words)
provides the significant-word precedent: retain default word segments containing
an Alphabetic or General_Category Number character. This classification precedent
does not adopt that crate's older Unicode data or implementation. Punctuation-only,
space-only and symbol-only segments are not significant words. Word-start and
word-end traversal are separate explicit products; adapters choose bindings rather
than silently importing Emacs, Vim, Helix or platform group-motion conventions.
Identifiers and dictionary/language tailoring remain separately named profiles.

boundary-command/1 under unicode-logical-edit/1 binds exact view/document
incarnations, source revision, expected selection generation, invocation, owned
boundary evidence, unit, direction, positive count and extension intent. Units
are grapheme, word-start and word-end; directions are forward/backward. Both BOF
and EOF are terminal caret targets. Choose the next strictly directional target,
or that terminal edge if none remains. Counts include a first nonextending
nonempty-selection collapse; remaining steps then traverse. Extending movement
keeps the original anchor, primary association and explicit affinity. This follows
the distinction between collapse and extension in
[CodeMirror commands 6.8.1](https://raw.githubusercontent.com/codemirror/commands/6.8.1/src/commands.ts),
not its entire word-group or binding scheme.

Segmentation and movement are different contracts. One degenerate flag/ZWJ
fixture has a raw default word boundary inside an extended grapheme, despite
matching both corpora. Character motion uses only the grapheme grid. Significant-
word endpoint vectors must be compatible with that grid or refuse the inconsistent
profile result; raw word boundaries are neither automatically intersected nor
substituted for character endpoints. No universal subset theorem is claimed.

Collapsed movement can leave an explicitly selected scalar position inside a
cluster by choosing the next directional endpoint. Implicit character/word
deletion from such an interior refuses instead of deleting part of a cluster or
guessing an expanded target. A caller can move to a valid boundary or supply an
explicit exact selection/batch. Nonempty deletion removes that exact selection,
with no extra word/character step, preserving the distinction between explicit
byte-edit intent and derived human commands. Delete rejects extend=true.

Derive one canonical simultaneous deletion batch. Overlapping/tied derived ranges
refuse as a whole; neither selection display order nor arrival order resolves the
conflict. Origin carets finish at deletion starts. The actual ordinary source/
annotation/all-view coordinator supplies current dependencies, permissions, bounds,
one root/group and mapped peer contexts. Pure motion publishes only a selection
effect, never a source revision/group. Identity validates without fabricated
bookkeeping; current viewport changes survive. Native F14 reverses actual source
bytes and maps current views, not historical caret snapshots.

## Evidence ownership bounds and arguments

Boundary evidence is owned, bound to exact source bytes/incarnation/revision and
the pinned profile. Copied values do not recreate provenance. Shape checks bound
ordered scalar-safe full-extent boundary vectors and exact significant intervals.
Current profile/liveness/retention and the complete ordinary view/source gates
revalidate at publication. Trusted snapshot and issuer/policy/retention suppliers
remain premises; internal constructors/resolvers are not a plugin ABI or a claim
of malicious in-process JavaScript isolation. No selector grants source access.

Finite source extent, boundary/word counts, boundary JSON bytes, total steps across
all selections and derived JSON bytes have explicit caps. Captured selection
limits cannot be reinterpreted, and tightened current work limits cannot expand
the plan. Grapheme counts do not replace byte/work limits: a single cluster can
contain many scalars. The simple word evaluator/UTF-16 conversion may rescan finite
input; these reference bounds are not incremental complexity, wall-clock service,
native layout, physical memory or lifetime-reclamation proofs.

Directional targets are monotone within a finite ordered vector and saturate at
BOF/EOF. The admitted count gives an independent finite loop horizon, including
identity steps at an edge. Source framing follows canonical batch emission;
whole-cluster implicit deletion additionally requires both endpoints in the
accepted grapheme grid. Exact selection deletion follows its explicit source
extent instead. Current acceptance/publication arguments remain conditional on
the existing coordinator's complete-companion and single-owner premises. Closed
units alone do not establish global termination or categorical completeness.

## Executed scopes and instrumentation

The provider checks all 853 official grapheme cases and 1,944 word cases, eight
controls and five compiled/load-success/assertion-failing mutants. Mutants include
UTF-16-as-byte offsets, ignored characters, regional-indicator parity and dictionary
substitution; the last passes the official corpus but fails supplementary fixtures.
Finite published conformance data is a test target, not a universal proof or
independent Cleanroom certification. The pinned property implementation/compiler
and manual word evaluator remain trusted parts of this evidence chain.

Actual command composition checks 19,200 grapheme movements/extensions, 4,800
grapheme deletions from official-fixture boundaries, 96 hand-written prose-word
movements, 48 word deletions, 2,048 final gate combinations, 43 controls and eight
mutants. Expected character endpoints come from the official markers, not the
candidate segmenter. Controls cover exact selection semantics, CRLF, read-only,
source/profile/ownership/selection/annotation/peer staleness, total step/resource
limits, coherent source/view publication and actual native undo.

INS-01/09 observe command/profile interpretation, INS-05 segmentation/evidence
availability, INS-02/07 publication/mapping, INS-06 reversal, INS-11 selection
handoff and INS-08 retained boundaries/work pressure. MELT/golden/RED separate
prepare/refused/unchanged/applied and latency; USE observes concrete input/boundary/
participant pressure. STRIDE includes forged/stale profiles, scalar substitution,
unbounded clusters/counts and partial deletion. LINDDUN includes language/script
fingerprints, selections, boundary density and hidden views/comments. Use bounded
status/reason/budget classes, not text, offsets, vectors, identities or plans as
metric labels. Actual diagnostic installation/release/overhead remains a gate.

Page/visual motion, shaped width/bidi, native and incremental/chunked provider
conformance, production package selection, replay/outcome/event composition,
complete companions and physical/concurrent refinement remain open. No annotation
storage policy, default keymap or globally supported host matrix is selected.
