# Reference annotation snapshot schema and staging

Specification §29.4.1 defines the version 1 UTF-8 JSON reference-host format.
annotation-snapshot.schema.json describes its structural shape; semantic checks
add budgets, exact numeric domains, unique live identities, range relations,
source association and current scalar boundaries. No schema engine, parser or
storage service is added to the authoritative editor core.

## Standards and bounds

[RFC 8259](https://www.rfc-editor.org/rfc/rfc8259.html) supplies JSON syntax and
interoperability guidance on member uniqueness, numeric precision, UTF-8 and
Unicode strings. Vellum makes duplicate decoded names and unpaired surrogates
errors, and uses canonical decimal strings rather than rounding JSON numbers.
[JSON Schema 2020-12 validation](https://json-schema.org/draft/2020-12/json-schema-validation)
provides the declarative shape vocabulary; it is not an RFC and cannot alone
detect duplicate names discarded by a parser or establish host/source truth.
All `$ref` entries are local; the verifier supplies its pinned dialect metaschema.
This selects neither remote schema retrieval
nor runtime interpretation of `$schema` or document strings.

[Web Annotation](https://www.w3.org/TR/annotation-model/) separates bodies,
targets and source context. The Vellum record separately preserves authoredTarget
and current placement, allowing detached bodies and historical intent without
claiming a current location. Its byte ranges/affinities are Vellum's existing
contracts, not imported character-position semantics or JSON-LD interoperability.

The approved version 1 draft requires the existing `position` or `content` profile
on every point, in both authoredTarget and located placement.target. Affinity alone
does not encode that distinction. Missing, unknown, wrong-case or coerced profiles
are invalid; neither source content nor a host default fills one in. This revision
does not migrate earlier profileless draft records or rewrite rejected input.

## Conditional arguments

For the supported finite syntax, each scanner iteration advances through input;
recursive descent consumes depth/value budgets. A per-object decoded-name set
rejects duplicates before assignment, including escaped spellings. Standard
JSON string parsing plus a scalar check rejects malformed escapes and lone
surrogates. Exact decimal grammar followed by integer comparison avoids rounding;
its true end assertion also rejects a terminal newline. Bounds apply before a
snapshot can be accepted. These are authored arguments about this reference,
not proof-assistant verification or production memory/CPU accounting.

Shape/semantic validation precedes staging. Failed association, current source,
identity restoration, target-evidence or annotation-set checks leave current state
unchanged. Accepted staging owns a frozen copy plus captured source/document/set
dependencies. Later input mutation cannot rewrite it, and preparation does not
publish. Actual final authority/source/set/companion checks and atomic publication
remain necessary; staging is not a lease. Historical target fields and serialized
versions remain data unless the actual host establishes their admissibility.

Profile preservation is a representation property: JSON string encoding of the two
distinct ASCII enum values is injective; strict member/enum validation, exact
decoding and the immutable owned copy preserve the chosen value. Thus, for accepted
reference values, projecting either point-profile field after wire/decode/stage
returns its original value. Unknown/missing values refuse instead of selecting a
meaning. Authored and located profiles are independently preserved as input data;
this does not establish their semantic agreement, source truth or repair eligibility.
The existing scanner budgets charge the added field/name/string before acceptance;
no new numeric product default or uncharged extension bag is introduced.

## Executable evidence

annotation-snapshot.mjs checks 828 valid combinations of 23 target shapes, three
placement states, current/historical authored context and six body values. It
preserves exact decoded data and input frames. There are 256 staging decisions
over six current host gates and matching/mismatching document/source association.
Seventy-five controls cover duplicate/escaped/nested names, malformed UTF-8, BOM,
surrogates, numeric coercion, fields/versions, identity collisions, decimal forms,
range/affinity errors, all eight resource dimensions, scalar splits, detachment,
large integer preservation, owned immutable preparation and captured set context.
Current-source authored targets also receive scalar checks; a contradictory length
for the same source domain/token cannot evade them by masquerading as history.
Separate examples preserve an astral scalar and decomposed text without normalization.

Both authored and placed profiles survive independently, without silently equating
the two fields. Four witnesses run the actual existing mapping reference after
decode/stage: under both affinities, replacement across a point detaches `content`
while `position` remains located. This demonstrates why profile loss changes
meaning; it is not an implementation of native snapshot restoration. Five loaded,
assertion-failing mutations challenge unknown profiles, missing-field defaulting,
content-to-position coercion and erased/swapped staged profiles. Their unmutated
dynamic baseline passes; syntax/import errors do not count as mutation detection.

The reference parser accepts only the object/array/string subset needed by this
format and relies on standard JSON.parse for individual string tokens. It is a
schema-specific design reference, not the chosen production decoder or a general
JSON Schema interpreter. Its fixture depth ceiling 32 and numerical limits are
private exploration parameters, not product defaults. Current host state/gates
are trusted inputs. Tests are same-author reference expectations, not independent
Cleanroom certification or exhaustive hostile-input coverage.

`--schema-check` uses pinned Python jsonschema 4.25.1 to check the schema against
its Draft 2020-12 metaschema and 28 selected structural positive/negative cases.
These checks are required for a complete current suite; absence is not a pass.
They check structure, not cross-field ordering/uniqueness, duplicate wire names,
resource/host truth or equivalence between the schema and every parser behavior.
Existing full-run hash/worker/output requirements remain. A new harness control
prevents a schema-only request being misclassified as generation-only work.

## Integration and instrumentation

Section 16.2.1 now supplies a selected actual native-to-storage capture over the
managed inventory; see [annotation-artifact.md](annotation-artifact.md). It keeps
complete disclosure, injective host/source association, old authored evidence and
immutable bytes separate from native restoration, provider writes and host truth.
The original decoder/shape checks do not by themselves establish that composition.

Actual host registry/source evidence, fresh runtime restoration and complete
history/retention ownership must refine the supplied gates. Real code must prevent
counter rollback, retired-incarnation revival, implicit deletion and concurrent-set
overwrite, not merely accept this JSON shape. The file is not a history archive or
authority source. No location, rename continuity, external-change guarantee,
crash durability or joint source/annotation persistence is proved here.

INS-01 distinguishes decode/budget/association/identity validation and admission;
INS-02 observes real atomic publication, not staging; INS-03 gates snapshot/body/
source-token release; INS-06/07 retain history and persistence distinctions.
INS-12 records bounded diagnostics/loss. Use approved coarse failure categories,
not bodies, tokens, raw parser excerpts or paths as labels. Existing STRIDE/
LINDDUN/STPA/FMEA lenses cover duplicate-key confusion, false source/identity,
integer truncation, target repair, resource floods, stale overwrite and disclosure.
Profile omission/coercion/tampering is a semantic integrity failure, not a cosmetic
display change. Decode/encoding/refusal classes at IF-13 / INS-07 distinguish these
failures without bodies, source tokens, offsets or raw input as ordinary labels.
