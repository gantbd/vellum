# Editor-precedent adoption and proof ledger

Accepted baseline: 2026-10-24, specification §27. Every adoption requires proof;
the user did not waive implementation refinement, platform tests or the remaining
material decisions. Requirements §19 already adopted most underlying mechanisms.
This ledger distinguishes new scoped algebra evidence from existing bounded
models and from still-open proof obligations. No whole-engine proof or independent
Cleanroom certification is asserted. No upstream implementation source is copied;
the mathematical encodings are authored from the declared contracts. Reading
public design/API documentation is not a source-isolated certification claim.

The subsequent proof pass supplies [general abstract arguments](precedent-proofs.md)
for all nine mechanisms and 29 additional solver conditions (19 positive local
laws, 10 faulty-alternative controls). Finite-list/trace inductions are authored
mathematical arguments supported by the local lemmas, not proof-assistant-checked
inductions. Concrete implementation and unspecified-profile gates remain open.
The [assumption audit](assumption-audit.md) now makes 18 selected premises and
11 open handoffs explicit, with a fingerprinted registry and 25 sensitivity
controls. It assigns existing owners without claiming premise truth, and keeps
the configuration-to-command context handoff visibly unbound.
Its PH-09 record now also links PS-CH's 25 bounded reference cases; concrete
producer/consumer refinement remains open. See [context-handoff.md](context-handoff.md).

## Precedent and exact adoption boundary

| ID | Primary precedent and adopted mechanism | Not imported |
|---|---|---|
| PA-01 | [CodeMirror system guide](https://codemirror.net/docs/guide/): functional immutable state, transactions and imperative view boundary | UTF-16 coordinates, DOM, concrete storage or arbitrary extension execution |
| PA-02 | [Helix usage](https://docs.helix-editor.com/usage.html): explicit multi-selections; [CodeMirror guide](https://codemirror.net/docs/guide/): batch positions in the original document | Modal UI, incidental insertion order or implicit focus targets |
| PA-03 | [Neovim API](https://neovim.io/doc/user/api/): edit-mapped extmarks, explicit gravity and namespaces | Namespace as capability; universal anchor identity/invertibility |
| PA-04 | [Emacs atomic changes](https://www.gnu.org/software/emacs/manual/html_node/elisp/Atomic-Changes.html): multi-buffer accept/cancel and separate undo amalgamation | Proof of observer isolation, external rollback or disk atomicity |
| PA-05 | [Neovim undo](https://neovim.io/doc/user/undo.html), Emacs atomic changes: explicit blocks and joining/amalgamation | Branch/selective history, persistent undo or timer-only cross-author merging |
| PA-06 | Neovim API: revision-bearing updates, discovery metadata and explicit evolution rules | RPC requirement, scripting runtime or identical support policy |
| PA-07 | [Helix injection guide](https://docs.helix-editor.com/master/guides/injection.html): explicit embedded source regions and explicit combined mode | Combining independent fences, ownership transfer or execution of embedded content |
| PA-08 | CodeMirror guide: setting-specific facets and compartment reconfiguration | Generic deep merge, dynamic plugin ABI or unbounded context retention |
| PA-09 | [Neovim options](https://neovim.io/doc/user/options/): synchronization enabled by default | Universal provider CAS/crash durability or a particular filesystem write procedure |

Sources were checked in the preceding research pass. They justify choices, not
our formal conclusions. In particular Neovim documents same-character replacement
marking a buffer modified. D01 now deliberately adopts Vellum's validated
unchanged-success policy (§6.4 / PS-NO), not a claimed universal editor default.

## Scoped proof support and mandatory residual gates

SMT positives first require satisfiable premises, then an unsatisfiable negated
conclusion. Controls require a satisfiable counterexample. They trust Z3 4.15.4;
there is no independently replayed proof object. See `results.json` for actual
execution, exact input hashes and freshness. A ledger row alone is not a pass.

| Adoption | Current property support / fault sensitivity | Proof still required before full closure |
|---|---|---|
| PA-01 | `PRE-SNAPSHOT-PUBLISH` and reuse control; authored fresh-root induction. Earlier functional-frame and bounded snapshot/retention checks remain | Actual storage ownership/alias/lifetime refinement, complete companion-state coherence and all adapter mutation paths |
| PA-02 | `PRE-BATCH-PREFIX-STEP`/`INVERSE-STEP`, unshifted control and authored arbitrary finite-list induction. Adopted D02 adds [ordering normalization](batch-normalization.md), 8,008 finite permutations and seven local SMT conditions. Earlier two-edit separated/adjacent checks remain | Concrete canonicalizer/list/inverse refinement, caller-resolved tie lowering, multi-selection resolution and current-revision/authority integration. Existing bounded batch TLC still excludes adjacency |
| PA-03 | `PRE-NAMESPACE-FRAME`, `PRE-ANCHOR-RANGE-ORDER` and controls; earlier point/survivor/affinity laws remain. Adopted D04 adds [profile evidence](mapping-profiles.md), finite byte/provenance checks and nine local SMT conditions | Concrete point/replacement/history/geometry maps, actual namespace storage/authorization and authored-annotation coordinator refinement; D45 interaction defaults remain separate |
| PA-04 | `PRE-ATOMIC-ROOT-READ`, abort frame/torn-read control and authored interleaving argument. Bounded Transactions/History/Bridges remain separate support | Concrete multi-object staging/root capture/refinement, recipient linearization and every required participant; external effects stay outside the theorem |
| PA-05 | `PRE-GROUP-APPEND-STEP`, `CLOSE-STABLE`, omitted-plan control and authored arbitrary-size group lifecycle induction; bounded History frontier checks remain | Concrete group lifecycle/typing policy, arbitrary linked-history graphs, retention/pin/floor and complete-group eviction refinement |
| PA-06 | `PRE-COMMAND-REJECT-FRAME`, surface equivalence and discovery-as-authority control; existing finite Execution/mediated-release checks remain | Actual public verbs/events/fields, schema totality, metadata noninterference, exact version compatibility, registration and actual UI/agent lowering/refinement |
| PA-07 | `PRE-REGION-INDEPENDENT-FRAME`, unique membership, suffix preservation and concatenation control; authored finite partition/frame arguments | Actual independent-fence discovery, delimiter/crossing/precedence profiles, real parser/query assets and source-map refinement. Valid-partition laws are not a parser proof |
| PA-08 | `PRE-CONFIG-UNRELATED-SETTING-FRAME`, dependency frame/missing-dependency control and authored activation induction using the existing coherent-pair law | Actual typed combiner laws/conflicts and dependency completeness, compatibility validator correctness, activation/refinement, bounded generations/exhaustion and chosen fallback |
| PA-09 | `PRE-SYNC-EVIDENCE-STEP`, failure/write preservation, out-of-order retention and premature-ack control; existing bounded storage/dispatch evidence models remain | Concrete provider write/sync/attribute/ack/failure profiles and physical crash/fault tests. No theorem here proves `fsync`, filesystems or hardware |

PA-01/PA-02 additionally have finite executable byte-edit expectations in
[text-oracles.md](text-oracles.md): three constructions, exact source preservation,
inverse recovery and adjacent-range coalescing with literal byte fixtures and
targeted mutants. These do not change the table's concrete refinement gates or
establish independent certification, insertion-tie policy or adopted D01's
complete semantic classification/validation; that has separate PS-NO evidence.

The initial five positive conditions are local algebra/guard laws, not complete
proofs of five features. For PA-05/08 the legality/compatibility guards are
assumptions of the encoding; proving callers, classifiers and activation actually
enforce them is explicitly outstanding. The controls explain why removing each
constraint can matter, not statistical fault-detection coverage.

## Derived arguments and next proof work

For adjacent intervals `[a,b)` and `[b,c)`, with insertion lengths `k,l`, applying
the right replacement first leaves the left coordinates intact. Applying the left
first shifts the right range to `[a+k,c+k-(b-a))`. Both yield the same prefix,
first insertion, second insertion and suffix. The SMT condition checks this
pointwise over arbitrary arrays/lengths, including either inserted string empty.
The all-empty result has no in-range points and equality is immediate by length;
arbitrary content equality needs no byte alphabet bound. UTF-8 boundary validity
is checked separately at admission, not inferred from this integer-array law.

The existing strict-gap composition law and adjacency law supply independent
two-edit checks. The subsequent finite-list argument in precedent-proofs.md
uses a prefix/cursor invariant and arbitrary-sequence step/inverse lemmas; it
is not a proof of the actual canonicalizer. Coincident insertions must not be
smuggled into the adjacency extension.

For pure insertion at `p` with length `k`, an anchor before `p` stays fixed, one
after `p` shifts by `k`, and one at `p` selects `p` or `p+k` according to declared
affinity. The new condition proves bounds and these cases, not deleted identity.

For coherent configuration, valid pair `C_old` or valid pair `C_new` selected as
one unit remains valid. Independently selecting components can yield an invalid
pair despite both complete configurations being valid. This is why staging and
one activation boundary are required; actual compatibility checking remains a
trusted supplier obligation until refined.

Next proof priorities are concrete finite-list batch refinement, full attachment
profiles, concrete history/region refinement and concrete closed API/
configuration validators once their remaining schemas/profiles are selected.
Physical provider/native obligations require integration evidence in addition
to mathematical models. Neither prior art nor a passing bookkeeping audit may
be substituted for these gates.

## Instrumentation and assurance integration

Specification §27.2 reuses existing model interface/pathway observation coverage:
current authority at publication/release; mapping/group/context identity under
protected diagnostic projections; stale-result and activation disposition; bounded
queue/cost/loss; and dispatch/receipt/sync acknowledgment evidence. No new network
edge, identity issuer, content asset or telemetry exporter is introduced. PA-06
discovery metadata and PA-03 namespaces must pass the same recipient-safe release
checks as other output; they are not exceptions to STRIDE/LINDDUN review.
Native, security and instrumentation conformance remain open alongside the exact
metric/field catalog. The nine adoptions are not nine new whole-system claims in
the separate four-claim assurance graph.
