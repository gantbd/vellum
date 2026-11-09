# Adopted v1 reference defaults and evidence

Specification §29 adopts the reference profile approved on 2026-10-27. The
selection closes defaults for execution, logical text, history, comments, assets,
configuration, observation and packaging. Concrete schemas, native support and
measured guarantees remain distinct obligations. reference-profile.json is the
checked selection record, not a complete executable configuration or Rust API.

## Precedents and selected behavior

| Source | Selected mechanism | Boundary |
|---|---|---|
| [Neovim API metadata](https://neovim.io/doc/user/api/#api-metadata) | Typed versioned discoverability | No RPC or copied extension/authorization policy |
| [Ropey line-break feature profiles](https://docs.rs/ropey/latest/src/ropey/lib.rs.html) | LF/CRLF/CR is an established selectable line domain | Vellum uses byte columns; no Ropey backend or scalar coordinates selected |
| [Unicode UAX 29 revision 49](https://www.unicode.org/reports/tr29/tr29-49.html) | Unicode 18.0.0 default extended grapheme and word boundaries | No implicit tailoring, dictionary completeness, bidi or layout guarantee |
| [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html) | UTF-8 JSON for versioned reference-host annotation storage | Stricter duplicate-name/scalar/exact-number rules are Vellum's chosen profile; JSON alone does not provide them |
| [W3C Web Annotation](https://www.w3.org/TR/annotation-model/) | Separate bodies, targets and source context | JSON-LD, remote contexts and a mandatory core persistence backend not imported |
| [OpenTelemetry metrics model](https://opentelemetry.io/docs/specs/otel/metrics/data-model/) | Compatible units, populations and measurement adapters | No core exporter/runtime dependency or measured-overhead claim |

History grouping, selection, typed configuration and package separation retain
their accepted mechanisms in §§28.13/28.34/28.43/28.45 and baseline-contracts.md.
The profile does not choose a complete peer editor's mode, bindings or defaults.
Required iced/ratatui apps and Markdown/child integration remain required.

## Logical line argument

For valid UTF-8, ASCII CR/LF bytes cannot occur inside a multibyte scalar. Scan
left-to-right; consume CRLF as one delimiter and lone CR/LF as one. Each consumed
delimiter closes the current body and starts the next. Source spans are adjacent
and disjoint by construction; after a finite byte scan, the final body ends at
EOF. Concatenating body-plus-delimiter spans yields exactly the input bytes.
Appending a final row also handles empty text and trailing delimiters without a
special missing-line state. Each scan step advances, so finite input terminates;
this is an authored argument, not proof-assistant or production-index refinement.

Within a row, adding its start to a valid body byte column and subtracting that
same start are inverse operations. Rows' representable body-caret domains do not
overlap: the following row starts after a nonempty delimiter. A CRLF-interior byte
is in neither body domain. Scalar-boundary checks reject multibyte splits; no
projection rounds an unsupported point into a different target. Canonical byte
edits remain independently valid at their own supported boundaries.

## Executable scope

reference-profile.mjs scans UTF-8 bytes and compares with a separate string/regex
delimiter oracle. It enumerates all texts of lengths zero through five over
`a`, `é`, CR, LF and U+2028: 3,906 text cases and 33,594 byte-position cases.
It compares exact line spans, byte reconstruction, representable inverse maps,
body-column domains and input frames. Seventeen controls cover empty/trailing
lines, CRLF, non-delimiter Unicode content, a literal BOM, invalid UTF-8,
out-of-domain numbers, wrong split/clamp expectations and annotation staging.
These are reference semantics, not a streaming/incremental rope or indexed query
complexity proof. Inputs are normalized byte arrays; arbitrary array coercion is
outside the reference domain. This earlier fixture executes no Unicode segmentation;
§26.1.3's separate [Unicode evidence](unicode-commands.md) now executes the pinned
reference corpora/provider and logical commands. Its reference-only conformance
does not change the production/provider refinement boundary.

The annotation fixture checks 256 normalized combinations of six supplied gates
and matching/mismatching document/source evidence. Refusal preserves current state;
successful staging owns a copy and still does not publish. Six supported scalar
body examples round-trip through JSON stringify/parse, including empty text,
multibyte/control/quoted text and a fenced body. A caller mutation cannot change
the staged candidate. The exact document/source objects and equality are fixed
normalized fixture values; no host identity oracle, public field schema, hostile
JSON decoding, duplicate-name detection, exact-number decoding, metadata, migration,
physical retention, atomic persistence or current-publication refinement is proved.

For a real validated lossless codec, serialize/parse preserves each supported
record field; if staged association/source/current authority/resource checks hold,
the existing shared coordinator may publish the complete declared participants.
JSON parsing itself neither proves the premise nor authorizes publication. Actual
decoder/conformance and the existing complete companion gates must discharge that
composition before implementation closure.

## Remaining evidence and observation

The reference profile is adopted, not provisional architecture. Numeric capacities
remain unvalidated until measured; no arbitrary numbers are installed here.
Actual production assets/dependency/license and host matrices, native/incremental UAX conformance,
public catalog/types and diagnostic fields/units, production JSON decoder and
host association/restoration procedure remain technical/integration work. The
subsequent §29.4.1 snapshot schema and reference checks are in annotation-snapshot.md;
they do not retroactively expand this earlier normalized fixture's scope. Real security
owners/support commitments and independent acceptance remain operational work.

INS-01/02 observe coordinate resolution, admission and publication distinctly;
INS-05/09 report the effective interpretation/profile and activation without
inventing a current result on missing support. INS-07 distinguishes staged JSON,
source versus annotation save and provider acknowledgment. INS-03 protects body/
target/association/profile disclosure, and INS-12 records bounded capture loss.
Aggregate bounded categories, not raw JSON/source/paths as labels. Existing
STRIDE/LINDDUN/STPA/FMEA lenses cover stale association, version downgrade,
duplicate-member confusion, coordinate rounding, retention pressure and hidden
body/existence leakage. Observation coverage is design intent, not installed code.
