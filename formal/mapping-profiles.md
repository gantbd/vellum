# Purpose specific attachment mapping

D04 adopts specification §4.3.1 and requirements §3.8's purpose-specific profiles
over one change algebra. Positional continuity, content-target survival and
applicability are distinct. Nonempty content ranges exclude pure insertion at
outer boundaries; collapsed ranges use one explicit point affinity. Partial
target edits can leave affected location evidence; complete original-target
deletion/replacement detaches without erasing the authored body. Mapping never
refreshes exact-revision proposal preconditions or authorizes execution.

## Conditional arguments

For a single valid replacement `[l,h)` with inserted length k, the representative
positional map leaves points before l unchanged and shifts points after h by
`k−(h−l)`. Replacement outer edges map to l and `l+k`; interior points collapse
to either boundary according to an explicit profile parameter. A pure insertion
maps its coincident point before or after the new text according to affinity.
This is one reference replacement rule, not selection of all production point/
replacement profiles. Each case maps valid input points into the new source's
bounds. UTF-8 scalar boundaries remain valid when input edit/target boundaries
and inserted text are valid; no grapheme, glyph or bidi conclusion follows.

Composition through a finite valid batch follows by induction over these bounded
maps, preserving each edit's logical boundary. Right-to-left original-coordinate
application and left-to-right delta-adjusted application provide two reference
constructions. This does not justify coalescing adjacent edits with different
mapping evidence; D02's boundary-preservation rule remains authoritative.

For nonempty content ranges, start-right/end-left affinity excludes pure insertion
at their outer boundaries. If at least one original target byte survives, the
reference endpoint maps remain ordered. Complete removal is instead detached,
not a potentially inverted range. Original-byte survival equals membership outside
the union of original replaced intervals: induction over sorted nonoverlapping
intervals proves the coverage test, including several adjacent replacements.
Equal inserted values do not preserve those removed identities. These byte-origin
labels are reference evidence, not a proposed runtime per-byte identity store.

A collapsed range maps one point once and shares its location for both ends.
Mapping a start-right and end-left pair independently at the same inserted point
would invert it; silently sorting would instead manufacture a selected span.
Neither is the adopted point contract. Missing history and detached states do not
become located merely because coordinates or a matching quote can be produced.
Authored body/version frames independently of source-driven location changes.
Exact expected revision and current authority remain required even if a preview
maps successfully. Restoration of source bytes is not an inverse of target identity.

These are authored conditional arguments, not proof-assistant replay, upstream
editor verification, semantic-target correctness or complete implementation
refinement. Persistence/history/lifecycle evidence must supply actual context,
source maps and supported restoration, not derive them from this algebra alone.

## Finite reference checks

`mapping-profiles.mjs` enumerates all 40 strings of zero through three scalars over
`a`, `é`, `🙂`. Batches include empty, single insertion at every scalar boundary,
and all nonoverlapping nonempty replacement batches of one or two edits, including
adjacency. Inserted values are empty, `X` or `🙂`. There are 2,338 batches,
18,056 point evaluations and 88,504 target evaluations. Both affinity parameters
and positional/content reference profiles are checked for each valid point/range.

Point checks compare reverse and delta-adjusted forward constructions and verify
result bounds/scalar boundaries. Target checks frame input/source/body/version,
check order/bounds, keep collapsed targets as points and compare complete range
detachment to a separate original-byte provenance emission oracle. The oracle
also reproduces result bytes from the existing byte emitter. They share source
admission, codec and author; provenance traversal is not independent certification.
This corpus does not enumerate mixed insertion/replacement batches, three-edit
batches, arbitrary Unicode or successive revision/reattachment histories.

Fifteen direct sensitivity controls exercise outer-boundary absorption, universal
clamp-as-survival, body erasure, equal-byte replacement, adjacent complete removal,
partial applicability, empty insertion, opposed collapsed affinities, silent
point widening, automatic undo/quote reattachment, missing history, stale proposal
refresh and unsupported profile/implicit affinity. These are selected fault cases,
not general mutation adequacy. A mapped preview is checked against both current/
stale expected revisions and revoked authority; other coordinator gates remain
premises, not modeled by that local eligibility predicate.

Fixture records, evidence labels, representative replacement edges/interior collapse
and strict-interior point-context detachment are private reference notation, not
chosen public schemas, point-context defaults, error precedence or UI behavior.
The target's old byte coordinates belong to the supplied starting source; output
location is separate. This is not a persistent target implementation or a claim
that replaying that output against another revision supplies valid context.
Authored document-wide targets, arbitrary history loss/reloads, reattachment
validation, direction/primary-selection rules and actual layouts are not simulated.
An equal-byte replacement's declared mapping effects can make it nonidentity under
D01; the fixture does not prove D01 classification or manufacture a publication.

## Local symbolic checks

`mapping-laws.smt2` adds five positive conditions, each SAT premises then UNSAT
negated conclusion: representative point bounds, surviving nonempty range order,
outer-boundary insertion exclusion, no automatic detached-state resurrection and
exact-revision proposal rejection. Four SAT controls expose opposed collapsed
affinities, valid geometry despite lost content, absent history despite numeric
mapping, and a current mapped proposal without authority. They use unbounded
integers for local coordinates, not a concrete overflow/allocator/schema proof.
The exact ordered `mappingCases` manifest uses trusted Z3 4.15.4 without independent
certificate replay. Complete source-context truth and explicit authorized
reattachment remain assumptions; the symbolic detached flag is not a verified
production identity/discovery algorithm.

## Interfaces observations and remaining work

IF-03 / INS-02 and PATH-01/03 retain coherent source/attachment publication.
INS-05/07 observe derived applicability versus authored body state; INS-03 retains
current release policy. Count targets traversed, affected/detached/unavailable
dispositions, mapping cost and retained evidence separately from source commits
and authored comment mutations. No body, quote, private topology or identity-valued
metric labels are added. STRIDE/LINDDUN cover forged source contexts, stale-target
redirection, manufactured applicability, private annotation linkage/counts,
mapping fan-out pressure and observations used as authority. Existing quotas and
release profiles apply; this adds no network path or diagnostic-authority shortcut.

Actual profile schemas, point-context and replacement-boundary choices, history/
reload/reattachment validation, native geometry, resource units/limits and whole
coordinator/implementation refinement remain open. D45 owns caret/selection
interaction defaults, not this adoption. The earlier PA-03 sources and fingerprints
remain unchanged; their opposed-affinity counterexample remains valid rather than
being relabeled a proof of these concrete profiles.

Mechanism precedent includes [Emacs marker affinity](https://www.gnu.org/software/emacs/manual/html_node/elisp/Marker-Insertion-Types.html),
[CodeMirror mapping modes](https://github.com/codemirror/state/blob/main/src/change.ts),
[Neovim extmark gravity/invalidation](https://neovim.io/doc/user/api/#nvim_buf_set_extmark()),
and [Helix's separate collapsed-selection handling](https://github.com/helix-editor/helix/blob/master/helix-core/src/selection.rs).
Their mechanisms inform the adopted separation, not identical behavior, code reuse
or proof of upstream editors.
