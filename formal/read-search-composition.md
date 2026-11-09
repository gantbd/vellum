# Retained read context and native search composition

This scoped reference composes an already acquired F02 source context with the
three existing F05 literal-search operations. It removes the second, independently
supplied source snapshot from this path. It does not select a new public request,
issuer, token encoding, query profile or asynchronous job protocol.

## Source and query boundaries

`searchCapturedReadContext` bounded-copies the complete request, current source
facts, current query facts and query limits before evaluation. The actual native
search declaration supplies the request fields, contract and operations. Unknown
fields, accessors and malformed scope rows cannot reach the evaluator. Scope rows
must be normalized numeric ranges before native sorting; a bounded data copy alone
does not establish that condition.

The selected context must remain active, include source data and match its captured
document, recipient, purpose, destination, policy, continuity and profile. Current
source release facts must be exactly true. A zero-range source-product validation
checks the retained source and captured source profile internally; it is not a new
source snapshot delivered to the recipient. Parent version-set permission remains
a trusted upstream obligation, not something this helper derives from one member.

Source-read permission is not query permission. The six native query facts must
separately be exactly true, and the original current query request reaches native
validation unchanged. The source profile bounds root, input and reply bytes even
when the supplied query limits are larger. Existing native search preserves exact
revision, scope, completeness, navigation and result ownership semantics.

## State and lifetime

The helper returns the identical functional registry state on success or refusal.
It does not issue selectors, acquire or release holds, refund issuance, invalidate
contexts, or turn observed loss into a durable lifecycle transition. Explicitly
closed and invalidated contexts deny subsequent search. An existing hold retains
the root until release, while retired identities remain spent.

A previously returned native search result can remain in memory after its source
context stops. Its owned identity and measured reply footprint do not establish
permission to reuse it for F11 replacement or F12 selection. Actual issuer binding,
absorbing authority loss, child-result lifetime and current-use checks must compose
before that wider lifecycle can close.

## Executable evidence

- `node formal/read-search-checks.mjs`: six native cases, 54 denial cases and ten
  controls, including separate grants, captured ceilings, stale query requests,
  held-root retirement and getter-free bounded input.
- `node formal/read-search-path-checks.mjs`: six observer-capture/read-context/native
  search cases, nine exact byte/node/depth boundary cases and five controls. A later
  source-root change does not replace the retained search input; annotation-only
  visibility cannot grant source access. Observer state remains unchanged. An
  actual acquired version-set member refuses independent search without its parent
  permit, and stopping the set prevents a stale permit from reviving the member.
- `node formal/read-search-mutations.mjs`: a coherent positive module graph, eleven
  semantic mutation controls and one syntax-construction control excluded from
  semantic detection counts. The controls include all three off-by-one limits.

Existing read-context, version-set and observer-read regression commands pass in
the same isolated source graph. These are serialized normalized reference checks,
not native context issuance, actual policy or authority-root certification,
asynchronous lifetime, physical/global allocation, concurrent/crash refinement or
independent Cleanroom certification. Specification §15.4.1 and model revision 0.210
record this scope; the batch requires a fresh complete frozen-input verification run.

## Instrumentation boundary

Existing INS-05/01/02/06 boundaries distinguish capture refusal, query admission,
evaluation, result release and later use. Query text, source bytes, exact scopes
and recipient identifiers must not become default telemetry labels. Metrics and
traces report bounded reason classes, useful versus stale work, budget exhaustion,
held-root pressure and lifecycle refusal without turning observations into grants.
No installed instrumentation, clock truth or measured overhead is established by
these reference checks.
