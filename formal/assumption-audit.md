# Precedent assumptions and composition audit

The nine adopted mechanisms depend on 18 selected premises and 11 selected
handoffs between abstract arguments. Each premise now has an architectural owner
or assurance role, validation boundary, required failure behavior and explicit
remaining evidence. The registry checks their traceability; it does not establish
their truth. All 11 concrete handoffs remain open refinements.

[precedent-assumptions.json](precedent-assumptions.json) is the manually reviewed
inventory. [precedent-assumptions.mjs](precedent-assumptions.mjs) validates source
fingerprints, model references, claim coverage, local evidence roles, handoff
references and acyclic proof support. Twenty-five sensitivity controls challenge
missing/stale references, omitted boundaries, wrong evidence roles, circular
support and unsupported claim promotion. The 29 precedent solver conditions are
all linked; the two earlier context conditions retain their separate manifest.
Six controls specifically check bounded handoff evidence roles, configured
properties, module, horizon and complete selected-case linkage.

## Findings requiring further evidence

| Finding | Existing owner and boundary | Consequence for the proof claim |
|---|---|---|
| Freshness must cover all reachable retained identities, not only the current root | Coordinator and lifecycle registry at identity allocation/publication | The arbitrary pairwise root law cannot prove the real allocator never reuses a retained identity |
| Equal lowered requests are a premise of surface equivalence | Facade, shared views and adapters at target/command lowering | Common dispatch proves conditional equivalence, not correct UI/agent lowering or native event behavior |
| A grouping key must include all incompatibility, including unrelated plans | History and coordinator at join/close | The group theorem cannot establish completeness of the eventual key or authorize joins |
| Valid independent regions are supplied to the frame theorem | Language analysis and coordinator at discovery/result acceptance | Unchanged suffix bytes do not prove correct fence discovery, language interpretation or ownership |
| Complete dependencies and valid combiners are supplied to the configuration theorem | Registry and affected consumers at resolution/invalidation | Frame preservation cannot prove the dependency declarations or typed combiners correct |
| Receipt attribution and fixed attempt scope precede evidence joins | Effects, host ports and binding registry at ingestion/reconciliation | Monotone facts cannot authenticate a provider or prevent cross-attempt mixing without those checks |
| Current recipient release is distinct from retaining a snapshot or historical receipt | Access gate and diagnostic bridge at each new release | An immutable root or true historical fact does not authorize its new disclosure |
| Configuration activation and command interpretation lack a shared semantic context binding in the selected theorem relation | Registry/facade/coordinator at request interpretation and final validation | PH-09 has only an assurance-trust premise in common. Its registry-version/context correspondence remains unbound |

These are assurance gaps exposed by the abstraction boundaries, not reports of
bugs in production code. No production implementation exists here. The findings
refine existing D02–D04, D09–D16, D24–D26, D29, D33–D35, D40, D45 and D48 work;
they do not adopt new public fields, grants, fallback rules or defaults.

## Handoffs and circular reasoning

The selected support direction is:

- Retained roots supply original-snapshot batching, coherent reads and save captures.
- Batch content/inverse laws supply attachment parameters, history content evidence
  and independent-region framing.
- Shared command interpretation supplies a candidate to final atomic publication.
- Coherent publication supplies the required history companions and historical captures.
- Configuration supplies compatible contexts to command interpretation and analysis.

Every edge requires a correspondence argument between the producer's actual
output and the consumer's assumed input. Matching names or sharing a broad
assurance premise is insufficient. PH-09 is reported as an unbound semantic
handoff rather than repaired by inventing a shared version schema. The other
edges also stay open: shared premises are necessary traceability, not proof of
representation equality, synchronization, completeness or supplier truth.

PH-09 now also has [bounded reference support](context-handoff.md): PS-CH checks
25 cases linking captured interpretation to final publication, using the actual
finite text/mapping/inverse operators. Its five positive profiles, eight controls
and twelve witnesses are separately linked and source-fingerprinted. The concrete
producer/consumer representation relationship remains unbound; no new shared
assumption or public version schema is inserted to claim closure.

The checker rejects cycles in this selected proof-support relation. It does not
claim that runtime execution, model interfaces or configuration feedback must be
acyclic. A mutually dependent proof would require an explicit joint invariant
and an induction argument; links cannot make each claim prove the other.

## Failure behavior and ownership

Architectural owners identify existing responsibilities in model §4, not new
processes or staffing assignments. AP-17 records the assurance maintainer/reviewer
obligation without inventing a runtime component. Environment and profile premises
remain conditions until their supported contracts and evidence exist.

Each premise records what must not happen when its required evidence is absent:
no silent identity reuse, ambiguous edit ordering, stale-context reuse, partial
publication, unauthorized release, receipt promotion or false durability claim.
Where exact refusal/fault/fallback behavior remains a specification choice, the
registry says so; its failure field does not select a public outcome enum.

The registry's proof fingerprints bind the inventory to the current authored
arguments and SMT sources. The full verifier additionally hashes the registry,
checker and narrative. These freshness checks cannot detect every semantic change;
material changes still need review of the mapped premises and handoffs.

## Remaining assurance work before specification decisions

This pass completes a selected assumption/ownership and composition-gap inventory,
not the whole assurance workstream or v1 specification. Next work remains:

1. Mechanically replay the authored inductions, with an explicit trusted-tool
   policy and no axioms that silently assume the desired result.
2. Strengthen mutations and positive witnesses for cross-boundary event histories,
   including instrumentation off/full/drop/fault and late evidence.
3. Extend the algorithmically distinct byte-edit expectations in text-oracles.md
   with independent process/review and remaining profile/edge oracles. Their
   finite same-author comparisons do not supply independence. Atomic-clause
   traceability beyond the existing source-unit census now includes 41 selected
   publication clauses (all ten §5.3 bullets); whole-document decomposition and
   semantic/implementation correspondence remain open.
4. Challenge the argument, premise coverage and harness with a fresh reviewer.

Actual UI/API/attachment/configuration/provider profiles and implementation,
native-platform and physical-storage evidence remain their separate gates.
The broad specification goal cannot be completed by marking this inventory green.
