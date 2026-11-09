# Settings query discovery products

The three existing F07 queries have closed declarative input and reply products:
effective configuration, permitted origin and capabilities. These describe engine
behavior, not key bindings, UI presentation, filesystem procedures or a transport.
The descriptor introduces no query, context acquisition or asynchronous event.
Explicit discovery profile /9 appends only F07 to /8. Existing profiles /1–/8
retain their audited products. Full frozen qualification and endpoint
reconciliation remain adoption gates.

## Type grammar and native authority

Inputs retain separate context and target roles, a nonempty profile, and a key for
the first two operations only. Successful replies retain their operation-specific
fields; unavailable has only its status. Origin replies are permitted-only arrays
or field maps of context-local references. Capabilities contain permitted-only
declarations without library defaults.

The setting grammar has six constructors: boolean, text, bounded safe integer,
nonempty enum, list and map. Lists and maps contain further setting types. Immutable
field declarations and domain, scope and composition vocabularies are consumed by
the native definition guards and the descriptor. The descriptor refuses an
unrecognized declaration partition instead of guessing how to describe it.

Capabilities describe the recursive syntax with fixed local references under
`$defs`. Exact metadata recognition admits only the generated tree and its known
local reference string. It does not load schemas, evaluate callbacks or add a
generic schema interpreter to the engine. The verification tool, not the engine,
uses an external JSON Schema validator.

The published JSON Schema 2020-12 dialect defines
[`$defs` and `$ref`](https://json-schema.org/draft/2020-12/json-schema-core)
and [array uniqueness](https://json-schema.org/draft/2020-12/json-schema-validation#section-6.4.3).
The recursive metadata applies that vocabulary to the adopted engine type syntax;
it is not a claim that an Internet-Draft is a ratified IETF standard. Enum
uniqueness uses JSON instance equality, not source bytes or object member order.
Merge-map composition requires a map type. Scope arrays are nonempty and unique.

Native checks still establish integer-bound ordering, unique declaration keys,
typed defaults and current values, current applicability, original identities,
origin membership and order, declaration order, disclosure permissions and
captured ceilings. Standard schema shape is deliberately insufficient for these
claims. In particular, unique array items cannot establish uniqueness of one
property across otherwise different declarations.

## Closure and termination limits

The six constructors form a closed signature, not a finite universe of all
settings and not a demonstrated bounded lattice. Finite lists, maps, strings and
JSON enum values admit arbitrarily many instances before an actual profile bounds
them. The native validator uses an explicit work stack with schema node and depth
ceilings. Normalized input capture separately bounds bytes, nodes and depth and
rejects malformed strings and non-JSON values. Local recursive references consume
child instances; they do not authorize schema execution or remote resolution.

Metadata cannot prove that a returned value belongs to the current key or that a
syntactically legal declaration fits an actual runtime budget. Original handles
cannot be authenticated from their JSON shadows. No unbounded termination,
categorical completeness, physical/global accounting or independent refinement
claim follows from these products.

## Instrumentation and evidence

At IF-01/02 and PATH-02, distinguish descriptor construction/recognition from
native admission, capture, current release and reply delivery. Existing
INS-01/03/05/07/08/12/13 coverage carries MELT and golden/RED stage populations,
latency and refusal/fault outcomes; USE tracks declared ownership, saturation and
discharge. STRIDE, LINDDUN and privacy/hazard checks include schema substitution,
remote reference injection, role spoofing, false complete-registry claims and
configuration/origin linkage. Raw setting keys, values, handles, declarations and
facts must not become ordinary telemetry labels or fault payloads. No new signal
threshold or deployment default is selected here.

The targeted corpus passes 391 standard-schema cases, fourteen actual-parent
typed/dynamic query products and twelve additional type/route products covering
all six constructors, nested prose and code-shaped data. It includes 319 syntax
denials, twenty recognition denials and six runtime counterexamples: reversed
integer bounds, repeated declaration keys, a wrong current value, a copied role,
an invalid Unicode key and a runtime depth ceiling. Two mutation banks detect
twenty-one changes against an unmodified copied-graph baseline. Syntax,
construction and process/tool failures do not count as detected semantic changes.
The earlier native settings regression retains all seventeen mutation controls;
its primitive-field construction target follows the shared declaration refactor.

Fourteen owned discovery controls include the eight pinned older-profile
fingerprints, captured disclosure ceilings, inherited read/history/observation
recognition, absorbing withdrawal, private builtin origin, advice separation and
eleven native authority refusals. Thirty integration mutations challenge those
controls and thirty-nine exact boundaries across thirteen resource dimensions.
Reply-size probes use independent fixtures so cumulative selector pressure does
not masquerade as the intended reply boundary. The observed metadata depth is 22;
the test fixture explicitly admits depth 64 without changing product defaults.

Selected post-import substrate probes wrap 143 filesystem/process callables.
Eighteen preconstructed discovery fixtures and two settings-reader fixtures
execute the fourteen controls with no observed baseline substrate calls and detect
three fixed read/rename/process injections. These are selected execution checks,
not import isolation or general host containment. Unknown-profile regression
probes now use /10; older recognition mutation targets retain their original
profile-specific obligations after the new profile joins inherited guard sets.

The complete frozen suite remains required before adoption. This evidence does
not close other operation families or F07's other resource/created-reference
products, hostile ingress, supported hosts, physical accounting, operational
measurement or independent assurance.
