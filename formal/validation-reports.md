# Named assertions and validation reports

Specification §26.4 defines semantic subject/key/report shapes without choosing
public Rust names, wire encoding, schema runtime or new v1 structural commands.
It refines §26.3's positive-only fixture by distinguishing source/candidate subjects,
named definitions/options, conformant/nonconformant/indeterminate verdicts and
order-independent satisfaction of required assertions.

## Subjects and applicability

A source subject identifies exact retained source context and target/scope.
A candidate subject identifies the exact prepared operation/candidate, base source
contexts and complete relevant proposed authored change. Candidate identity does
not claim live publication or permission. Actual construction/encoding must bind
the immutable prepared value and all relevant companions; a label alone is not
that proof. Public identifier, fingerprint and comparison procedures remain gates.

An assertion key names its supported claim kind, namespaced assertion definition,
applicable standard/schema/grammar version/dialect/options, tagged subject,
target/scope/query and relevant dependency context. Same text under a different
schema is a different assertion. Definition names do not load code or authorize
network access. Imported definitions and validation options need bounded authorized
resolution and tracked closure/context, not silently mutable URI dereferencing.
Required assertions and permitted issuers belong to the accepted descriptor/
intent/current host policy; a report cannot grant itself authority or lower them.

The core first validates report shape, interpretation, attribution/admissibility
and context. A report with no applicable key cannot satisfy or refute that
requirement. Unknown/malformed/stale/untrusted evidence is not automatically a
negative validation verdict. Its safe intake disposition follows its own contract;
recipient errors/counts/provenance remain whole-envelope projected.
The admissible-issuer/trust interpretation belongs to the captured applicability
context. Its change invalidates the assertion/evidence set or requires explicit
bounded reconciliation; it cannot silently filter out old negative facts and
reuse a positive report under the unchanged key. Current recipient revocation
can withhold a report without changing the retained facts.

## Verdicts and satisfaction

Conformant describes supported conformance within stated coverage. Nonconformant
is a definitive violation under the accepted assertion interpretation, not a
timeout, parser fault or malformed input envelope. Indeterminate supplies no
definitive answer. Coverage remains separate: partial positive evidence cannot
establish a required complete assertion. A partial counterexample refutes only
where the accepted claim profile explicitly supports that meaning.

For a fixed required assertion/current applicability context, let P mean at least
one matching admissible complete conformant fact and N mean at least one matching
admissible definitive nonconformance fact. The satisfaction relation is:

| P | N | Internal satisfaction |
|---|---|---|
| false | false | Insufficient |
| true | false | Established |
| false | true | Refuted |
| true | true | Contested |

Only established required assertions pass this evidence gate. Ordinary authority,
dependency, resource, cancellation and publication gates still apply. This does
not make evidence status a public outcome enum, effect receipt or validation
runtime. Additional optional reports cannot create new required assertions.

Set membership gives permutation invariance and equivalent-duplicate idempotence:
reordering/repeating facts cannot change P or N. Adding contradictory evidence
cannot silently become established. This is not a last-writer-wins register or
a total ordering of verdicts. Reusing a report identity with different payload is
a protocol fault, not an equivalent duplicate. Retained provenance/contradiction
and bounded reconciliation remain real ownership obligations, not implied storage.

## Conditional evidence and remaining gates

`validation-reports.mjs` evaluates normalized trusted fixtures for two subject tags
and two definition names. Each checks all 16 subsets of complete positive, partial
positive, complete negative and indeterminate facts against an explicit four-way
truth table: 64 report cases. All subset permutations and duplicate sequences
check order/idempotence; direct controls vary named key/subject/profile fields,
issuer admission, closure freshness, contradiction order, unrelated negative
reports and declared/unsupported partial counterexamples. Inputs are framed.

The fixture does not parse hostile reports, authenticate issuers, validate actual
schemas, resolve imports, construct immutable candidates or prove provider truth.
Its three verdict labels, two coverage labels and concrete fields are private
model values, not the complete public type catalog. Candidate labels stand in for
sound subject binding; Boolean partial-counterexample support stands in for an
accepted claim-specific interpretation. Complete/partial are a selected coverage
abstraction, not a waiver of unavailable/unsupported execution dispositions.
Duplicate identity/content conflicts, bounded provenance storage and reconciliation
are specified obligations but not implemented by this predicate.

The conditional set/truth-table argument is authored reasoning, not a new solver
theorem or automatic composition with exact publication. Existing structural and
source-only references retain their own scopes. General claim/schema decoding,
actual context suppliers, all-field recipient projection, provider failure
containment and full operation/companion/adapter refinement remain open.

INS-05 observes report admissibility versus verdict/satisfaction and context loss;
INS-01 observes required-claim admission; INS-02 observes actual publication;
INS-03 gates report/target/reason release. MELT/RED/USE/golden observations distinguish
refutation, insufficient/contested evidence, work failure, saturation and unknown
visibility without raw assertion names/URIs/source as labels. STRIDE addresses
issuer/key spoofing, definition substitution, stale closure, verdict confusion and
report floods; LINDDUN covers subjects/schema locators/provenance/correlations;
STPA/FMEA targets wrong-subject validation and arrival-order conflict erasure.
Actual schemas/names/units/limits and adversarial runtime evidence remain gates.
