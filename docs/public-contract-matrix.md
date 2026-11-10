# Northbound contract closure audit

Current native delivery disposition: the approved v1 functional C1/C4 crosswalk
is closed; see `engine-test-plan.md`'s C1/C4 closure checkpoint for current source
and test evidence. Earlier untyped attachment lookup and distinct provider-context
activation gaps below are superseded by the user's typed-native/adapter-lookup
and binding-only decisions. Historical formal/staged notes retain their original
scope; they neither reopen implemented routes nor certify current release builds.
C6 actual-path/performance qualification is closed within its declared scope;
C7 integrated qualification is also closed: both full 88-target debug/release
runs pass 2,880 tests, with strict workspace lint, formatting, documentation,
explicit pinned Unicode conformance and the runnable native handoff passing.
The release-only performance test was explicitly run, not waived. No new formal
certification, transport/identity codec, UI or disabled analysis capability is
inferred from these implementation checks.

The qualified baseline has 71 engine variants across 23 families, five integration
variants across three families and eleven presentation variants across six
families. Families can span owners; all 87 labels and their original family
indices remain. This is a routing inventory, not a complete public contract.
The adopted clipboard partition places workflow verbs in the headless integration
facade, not the semantic engine or UI.
Existing native contracts supply substantial
argument, execution and result semantics. The remaining work is to bind those
contracts into one facade and close the specific control and event gaps below.
The user selected operation-specific receipt delivery through the retained-outcome
facet; adopted 0.219 qualifies that binding at the eight native kinds listed under N02.

This audit supports L02 in the [closure ledger](specification-closure-ledger.md).
It does not introduce commands, select a transport or freeze Rust layouts. The
qualified baseline is specification 0.224/model 0.288. Pending candidates
are not accepted evidence. This working file is outside that qualification's input
inventory; its inventory and links are checked separately.

Adopted 0.224 passes 802/802 full frozen checks over 1,018 inputs, exact source/
worker/root comparison, selected registered predicates and retained archive
integrity. It includes the original load/save parent-task bindings, separate
native-operation observation profile, twelve-class protected stop relation,
registry command/historical receipt/discovery composition and lifetime-funding
corrections. These are selected native facade closures, not full L02 or F18.
The 0.225/0.226 inspection drafts are not adopted; their reproduced accounting
defects are repaired in the separately verifying 0.227 successor.

Staged 0.226 §17.4.4 now composes actual original shutdown consent, parent funding,
the ordinary fence and dedicated retained F17 disposition. Its focused registered
predicate passes nine readiness/ten consent groups and eighteen semantic mutants;
generic disposal remains fenced. This is an internal shared-writer binding, not
public F18 argument/result/report completeness. Complete owner-obligation inventory,
service/cutoff/report/control retirement and full qualification/adoption remain
L09/L02 work. U05/U06 are unchanged pending user decisions.

Adopted [specification 0.218 §18.8.4](../specification.md) binds the selected
existing-owner F08 queries and generic accepted-registry discovery. Its complete
793-check frozen suite and source/worker/root/archive integrity audits pass over
1,004 inputs. Targeted read-only second-agent review found no material defect in
that F08 scope. This accepts the selected binding, not complete L02 or independent
certification.

The adopted batch connects the F01 native result guard, F08 actual query result/
workspace guard, source-only F11 exact-batch and primary-only accepted composition
binding, and repeated-source grouping. It does not close the full conditional F08
facade or L02. Adopted 0.210 adds compound receipts, mixed authoring and
standalone F13/F14 invocation bindings. Its 768-check full suite and source/worker
and root endpoint audits pass over 970 inputs; retained archive integrity also passes.
Adopted 0.211 adds actual-parent temporary workspace accounting for all fourteen
existing F17 forms; 769 checks pass over 973 inputs, with source/worker/root and
archive integrity audits. Adopted 0.212 adds actual native observer-claim binding:
770 checks pass over 976 inputs, with source/worker/root and archive integrity
audits. Adopted 0.213 qualifies the three-owner partition with 770 checks over 977
inputs and matching source/worker/root and archive audits. Adopted 0.214 checks
the current engine/integration/presentation partition against native discovery
metadata, with 771 checks over 978 inputs and matching endpoint/archive audits.
Adopted 0.215 binds the native analysis owner to its actual invocation domain and
current root, with 772 checks over 981 inputs and matching endpoint/archive audits.
It does not complete the F08/F24/F28 facade or F16 provider activation. Adopted
0.216/model 0.280 qualifies the actual-domain admission-fence primitive with 773
checks over 984 inputs and matching source/worker/root and archive audits. It
closes the four selected bypasses while preserving eligible pre-admitted save
drain; it does not establish complete owner inventory or shutdown acceptance.
Adopted 0.218 also qualifies selected actual owner attachments, aggregate logical
funding, native publication-observer ownership, terminal-ticket local summaries,
existing-owner admission guards and dependency-discharge fixes. Complete factory
coverage, shared-writer retirement and shutdown-controller composition remain open.
The historical per-family integration notes below must be
read with these current dispositions, not as a request to rebuild adopted bindings.

## Existing contract boundary

[Specification §22.1.1](../specification.md) makes F-identifiers and variant labels
design references, not assigned public identifiers. The header and decoders in
[operation-catalog.mjs](../formal/operation-catalog.mjs) explicitly implement
family/reference routing, not argument parsing or the Rust API. Successful routing
therefore cannot establish a variant's complete input or output contract.

The matrix identifies contracts to reuse, not contracts to rewrite. A native
argument, receipt or private preparation port is not automatically a public entry
point. Its opaque ownership, current authority, retention and release conditions
must survive facade binding. Typed and dynamic invocation share semantic meaning;
their concrete syntax need not be identical.

## Variant and contract matrix

Every qualified-baseline engine variant appears once below. Required integration
workflows have their own matrix immediately afterward. The last column identifies remaining
integration work using the six bounded work items in the following section. It
does not assert that every cited reference covers every variant in its row.

| Family | Engine variants | Existing contracts to reuse | Remaining integration |
| --- | --- | --- | --- |
| F01 | list, inspect, advisory-enablement | [Arguments](../formal/discovery-arguments.md), [owner gateway](../formal/discovery-gateway.md), [results](../formal/discovery-results.md); specification §22.1 discovery profiles | Adopted 0.207 connects the native result guard before tentative selector/issuance publication on native and decoded routes. The prior result/fault composition defect is closed; original-owner fault containment and protected cleanup remain in force. Self-description /13 is a separate adopted contract. Actual installed descriptor/handler correspondence and production refinement remain external gates, not another missing result schema. |
| F02 | bounded-range, line, snapshot-handoff | [Source products](../formal/source-products.md), [read gateway](../formal/source-read-gateway.md), [acquisition gateway](../formal/read-acquisition-gateway.md), [released values](../formal/source-read-results.md) | Existing logical query bindings accepted below for the supported current/retained single/coherent profiles. L04 lifetime design is accepted; production ABI/conformance and L18 final consistency remain. No new public issuer verb is required. |
| F03 | enumerate, inspect, target-location | [Annotation gateway](../formal/annotation-query-gateway.md), specification §§12.8 and 22.1.12–17 | Existing logical query bindings accepted below, including acquisition-to-query composition. Native alias installation is not rolled back by later facade failure. L04 lifetime design is accepted; L18 final consistency and release evidence remain. |
| F04 | frontier, availability, permitted-blocking-detail | [Arguments](../formal/history-query-arguments.md), [gateway](../formal/history-query-gateway.md), [results](../formal/history-query-results.md) | Existing logical query bindings accepted below, including exact selector roles, capture ceilings and current permitted detail. L07 history design is accepted; advice does not authorize reversal. |
| F05 | literal-matches, next-match, previous-match | [Search gateway](../formal/search-gateway.md), specification §§15 and 22.1 | Synchronous exact-literal products are bound. Qualified §15.4.4 resolves L06's cancellation crosswalk: one bounded call is a cooperative quantum, with entry and later-use checks. No mandatory async mode or mid-scan preemption is inferred. Navigation query does not apply selection. |
| F06 | status, bounded-observation-wait, retained-outcome, minimal-control-status | [Observation arguments](../formal/invocation-observation-arguments.md), [decoder](../formal/observation-decoder.md), [results](../formal/invocation-observation-results.md) | Existing four logical bindings accepted below, including trusted host-driven bounded-wait continuation. U01 is resolved: adopted 0.219 qualifies the typed retained-outcome receipt facet at N02's eight native kinds. Receipt access is not implied by summary/detail permission. Recipient event transport, complete F19 and cross-family N03 assembly are not inferred. |
| F07 | effective-configuration, permitted-origin, capabilities | [Settings query arguments](../formal/settings-query-arguments.md), [gateway](../formal/settings-query-gateway.md), [registration query arguments](../formal/registration-query-arguments.md) | Configuration-query/1 and the selected resource-catalog-query/1 capabilities binding are accepted below. They remain distinct profiles with different original context roles. Generic accepted-registry metadata supplies the latter's logical discovery binding without widening builtin /9. Other conditional domains, N03 and full L02 remain separate. |
| F08 | supported-analysis, supplied-evidence-inspection | [Analysis products](../formal/analysis-work.json), [analysis owner](../formal/analysis-work.md), [actual engine owner binding](../formal/analysis-owner-binding.md), [native producer binding](../formal/native-analysis.md) | Adopted 0.218 §18.8.4 accepts bounded typed/dynamic argument binding, original snapshot/view and caller-scope correlation, current native release and generic accepted-registry discovery for both existing query variants. Decoder and native workspace overlap in actual owner accounting. These immediate queries need neither a new execution machine nor U01's retained command-receipt route. F16 activation, F24/F28 invocation/task/evidence assembly and installed provider/production truth remain separate; no other analysis profile is inferred. |
| F10 | advisory-validation | [Proposal validation](../formal/proposal-validation.md), specification §22.1.26 | Existing logical query binding accepted below for native-compound-current-checks/1, including the adopted normalization bridge and discovery /12. Advice reserves nothing and cannot authorize later publication. Other job/provider validation profiles are not inferred. |
| F11 | exact-batch, ordinary-edit, replace-current, replace-all, accepted-composition-commit | [Exact batch binding](../formal/exact-batch-binding.md), [mixed authoring](../formal/atomic-authoring.md), [ordinary gateway](../formal/ordinary-gateway.md), [replacement gateway](../formal/replacement-gateway.md), [composition binding](../formal/composition-binding.md), specification §§6 and 20 | These five variant bindings are accepted at their stated logical/scoped-reference level. Adopted 0.210 includes mixed source/annotation ingress and internal receipt vectors. N03 and U01 remain distinct. Clipboard workflow continuations belong to the integration matrix; no keymap/preedit policy enters the engine. |
| F12 | set-selection, logical-movement, select-search-match | [Selection products](../formal/selection-products.md), [gateway](../formal/selection-gateway.md), specification §22.1.25 | Existing logical bindings accepted below for four native forms/three semantic variants. Cross-family outcome/observation assembly remains N03/F06/F19, not a reason to invent an operation-specific selection receipt. Visual movement is presentation-owned. |
| F13 | create, update-body, delete, reattach | [Native invocation binding](../formal/native-local-command-binding.md), [authoring products](../formal/annotation-operations.json), [released results](../formal/annotation-results.md), specification §§13.6–13.9/14.6.2 | N01/N06 accepted below for all four standalone native forms and their mixed-authoring composition. Original allocation, independent body/target versions and both point profiles are retained. N02/U01 public receipt retrieval, complete N03 and installed dispatch remain separate. |
| F14 | undo, redo | [Native invocation binding](../formal/native-local-command-binding.md), [reversal products](../formal/history-reversal.json), [released results](../formal/reversal-results.md) | N01/N06 accepted below for undo/redo with original group/scope-frontier selection. Execution cannot refresh the selection or replay external effects. N02/U01 public receipt retrieval, complete N03 and installed dispatch remain separate. |
| F16 | semantic-settings, supported-registry, compatible-analysis-or-provider-context | [Settings owner](../formal/engine-settings-state.md), [schema discovery](../formal/schema-discovery.md), [result distinctions](../formal/registration-result-values.md), specification §22.3 | N01/N06 accepted below for the eleven selected settings/registration forms, using their actual settings envelopes or trusted native registration adapters. Compatible analysis/provider-context binding remains open. N02/U01 result assembly, N03/F19 observations and operational gates remain separate; host/scoped/created-reference results cannot be conflated. |
| F17 | create-scratch, activate-loaded-candidate, attach-detach-view, membership, rebind, dispose, view-container, workspace-container | [Lifecycle products](../formal/lifecycle-products.md), [basic arguments](../formal/lifecycle-arguments.md), [activation arguments](../formal/lifecycle-activation-arguments.md), [owner envelope](../formal/lifecycle-envelope.md), [load invocation](../formal/load-invocations.md) | N01/N06 request and native execution bindings are accepted below for all eight variants through the fourteen existing forms. Qualified actual-parent workspace and discovery contracts compose with the two argument/envelope paths. Adopted 0.219 qualifies N02's selected receipt facet. The staged 0.220 selected native outcome/receipt/event composition passes focused checks but is not yet qualified; complete N03 remains open. Initial birth/join is not in-place reload; U02 adds no mandatory reload surface. |
| F18 | request-cancellation, shutdown-disposition | [Native cancellation](../formal/invocation-control.md), [scope flags](../formal/invocation-control.json), specification §17 | Rust scoped cancellation now has exact typed/dynamic `invocation-control/1` input and installed discovery → current authorized native owner dispatch. Copied descriptors and inert tickets refuse; direct protected control is independent of discovery. Native settlement is exercised separately. Complete shutdown/public assembly and whole-engine qualification remain open; stop intent, settlement and quiescence stay distinct. |
| F19 | establish-bounded-stream, stop-observation, authorized-resynchronization, revoke-observation-grant | [Read contexts](../formal/read-contexts.json), [publication observations](../formal/publication-observation.md), [registration observations](../formal/registration-events.md), specification §§12.3/12.9–10 | Native stop bindings compose current owner classes. Source, named-facet, collection and registry establishment/resync now join installed Rust request/replay and independently authorized original receipts. Collection commands consume only their original visibility packet; known failure and replay do not restore or replace it. Historical receipts preserve original projections/references. Exact-context revocation has separate installed ingress. Broader heterogeneous attachment/control composition and complete N03 remain open. Stop does not cancel a producer. |
| F20 | source-load-candidate, annotation-load-candidate | [Source resource read](../formal/resource-read.md), [annotation resource read](../formal/annotation-resource-read.md), specification §16.1 | Native request/task bindings are accepted below. Current Rust installed discovery joins original reader-owned parent/matching-task recovery and independently authorized native operation observation; see the current projection. Legacy raw origin reuse is not owned recovery. Complete N03/event assembly and operational qualification remain separate. Completed reads do not activate or restore content. |
| F21 | captured-source-save, captured-annotation-save | [Source save](../formal/source-save.md), [native source save](../formal/native-source-save.md), [ordinary annotation save](../formal/ordinary-annotation-save.md), specification §16 | Native ordinary-save bindings are accepted below, including immutable capture, one writer and distinct destinations. Current Rust installed discovery joins original writer-owned parent/matching-task recovery and independently authorized native operation observation. Complete N03/event assembly and operational qualification remain separate; external write acknowledgment is not baseline adoption. Separately authorized export is not an ordinary save. |
| F24 | enabled-parse-query-work | [Analysis work](../formal/analysis-work.md), [native analysis](../formal/native-analysis.md), specification §18 | N01/N03/N05/N06: bind conditional requests, submission lease, task results and independent quiescence. Optional use retains the required Markdown/child reference integration. |
| F26 | timer, task-cancellation, authorized-diagnostic-drain | [Diagnostic drain](../formal/diagnostic-drain.md), specification §§17 and 23.2 | N01/N04/N06 accepted below for owned-diagnostic-drain/1, including matching retry, local service and independent result/control facets. Timer/task cancellation still need their advertised facility contracts under N05. No mandatory timer service, exporter or all-F26 closure is inferred. |
| F27 | progress, acknowledgment, failure, quiescence, unknown, contradictory-or-duplicate-evidence | [Read evidence](../formal/resource-read.md), [save evidence](../formal/source-save.md), specification §§16–17 | N01/N03/N06: bind supported evidence at its owning resource/integration edge to the generic engine request/evidence relation. Authentication and exact historical attribution precede duplicate/conflict handling. Clipboard-specific envelope interpretation remains integration work, not a clipboard protocol inside the semantic engine. |
| F28 | enabled-analysis-result | [Analysis intake](../formal/analysis-work.md), [native binding](../formal/native-analysis.md), specification §18 | N01/N03/N05/N06: bind contextual derived evidence and current applicability, not external-write truth. Geometry results belong to presentation support. |

## Current Rust closure crosswalk — 2026-11-07

This is the finite C1/C2 reconciliation of the 71 variants above, not another
specification or a claim that the reference bindings are implemented. Counts are
variant inventory, not effort percentages. Paths below are relative to
`engine/vellum-engine/src/coordinator/`; cited tests already exist and are reused.
“Installed” means a private handler-origin check plus independent owning service,
not a copied descriptor. Partial rows remain required work; absence is not closure.

| Family / count | Actual Rust owner, input/result route and existing checks | Delivery disposition |
| --- | --- | --- |
| F01 / 3 | `discovery/arguments.rs`, `ownership/decoder.rs`, `ownership/queries/results.rs` and builtin `discovery_query.rs`: actual input declarations, native reply-form guard, explicit self-description and sealed typed/dynamic dispatch; `tests/discovery_behaviors/self_description.rs` and owner tests | Functional native self-description/dispatch implemented over the existing query/decoder owners. Input fields come from the normalizer's declarations; result forms and metadata whitelists are shared with actual reply validation. Original selectors, captured disclosure ceilings, independent current recipient release and protected cleanup remain mandatory. Preparing descriptions neither acquires contexts nor enables decoding; copied metadata has no handler. This native explicit source is not a JSON identity codec or a claim that reference /13 is installed. |
| F02 / 3 | `source_context/facade/arguments.rs`, `facade/acquisition/arguments.rs` and builtin `source_read.rs`: range/line and current/retained single/coherent acquisition, with original opaque roles, owned argument capture and result guards; facade and installed/public consumer tests | Installed discovery now reaches both source queries and all six acquisition ports on typed/dynamic routes. Independent metadata/native release, copied-metadata refusal, optional capture and protected cleanup are exercised. Functional F02 integration complete; release performance and whole C1 remain open. |
| F03 / 3 | `source_context/facade/annotations.rs`, `annotations/arguments.rs` and builtin `inspection_read.rs`: enumerate, inspect, location with original single/coherent targets; facade and public annotation consumer tests | Installed typed/dynamic binding complete. Native disclosure ceilings, context-local record/continuation references and current whole-projection release remain mandatory; copied metadata cannot install a handler. |
| F04 / 3 | `source_context/facade/history.rs`, `history/arguments.rs`, `history/inspection.rs` and builtin `inspection_read.rs`: frontier, availability and permitted detail; facade and public history consumer tests | Installed typed/dynamic binding complete. Original context/scope and filtered native products remain guarded; retiring discovery neither retires history nor grants reversal authority. |
| F05 / 3 | `source_context/search/{arguments,binding,navigation,result_contract}.rs`, `source_context/facade/search.rs`, builtin `literal_search.rs`; public `search_argument_behaviors/{binding,installed}.rs` and existing search consumers | Selected native binding/service/installed integration implemented for all three operations. Host-established correspondence issues bounded borrowed opaque native-owner captures, with no resolver, second search store or new payload pin. Typed/dynamic envelopes and private-origin discovery dispatch preserve logical reference, literal-exact/1 profile and request generation; unsupported profiles do not fall back. Read/result permissions remain separate; original-role checks precede absorbing correspondence loss. Reply-header budget is checked before native issuance/hold, and protected result retirement uses the actual managed-search owner. Eleven final public scenarios pass, with 16 owner-unit and 52 existing public search regressions; formatting and strict focused all-feature Clippy pass. Explicit native discovery source does not change older builtin profile rows or implement a JSON identity codec/reference /4 protocol. Whole C1/C7 qualification remains separate. |
| F06 / 4 | `invocation/observation/{ingress,receipt_ingress,ordinary,wait}.rs`, `resource/observation{,/arguments,/decoder}.rs`, builtin `invocation_observation.rs` and `native_operation.rs`; observation, retained-release, source-read and save-operation consumers | Local invocation observation/wait and V10 typed receipt facet are installed. A separate sealed native-operation source now connects the four actual source/annotation read/save parents to their existing funded decoder, retained receipt, protected control and bounded wait. Owning declarations bind the accepted fields/contracts; current discovery and native class-specific release remain independent. No task or invocation store is added. C2's remaining producer crosswalk stays explicit; these dispatchers do not need further speculative proofs. |
| F07 / 3 | `settings/reader/ingress.rs`, `resource/catalog/read/ingress.rs`, builtin `settings_read.rs` / `catalog_read.rs`; public settings/catalog recipient scenarios | Both adopted configuration and resource-catalog profiles have native/decoded/installed functional integration. Release workload qualification remains C6. |
| F08 / 2 | No native analysis owner, enabled analysis-provider registration or installed analysis query handler in this crate; analysis is optional under the authoritative-core boundary in requirements.md | Conditional facility unimplemented, disabled and not advertised by native installed sources. This does not certify a future enabled provider. |
| F10 / 1 | `proposal_advice.rs`, `proposal_advice/ingress.rs`, builtin `compound_advice.rs` and shared `transaction_candidate.rs`; native/closed-header/installed and public advice scenarios | Functional native profile integration complete: both borrowed routes preserve original plan/proposal pairing, request/root/context/policy basis and independent complete observability. Sealed discovery reaches the actual non-reserving validator; copied descriptions cannot install it. Private candidates spend no identities/journal reservations and future publication revalidates. Outer schema coverage is structural only; no nested logical-plan JSON or wire codec is claimed. |
| F11 / 5 | `invocation.rs` compound/search-replace, `invocation/{compound_results,exact_batch,atomic_authoring}.rs`, `invocation/managed_search/replacement{,/arguments}.rs` and owning argument modules, ordinary/composition producers and installed `builtin/{ordinary_dispatch,composition,exact_batch,atomic_authoring,replacement}.rs`; atomic/ordinary/search/selection consumer tests | Composition, source-only exact batches, mixed authored batches and returned-search replacement have native/closed typed-dynamic/sealed installed routes through existing owners. Mixed batches require original ordered component bindings and fresh F13 rights; generic run cannot bypass them. Before/candidate targets, private stamps, current-service views, one linked publication and original allocations/receipts are preserved. Canonical partitions do not multiply participants/funding. Actual parent prepays preparation plus the existing receipt hold; stop/forget do not refund, protected retirement does once. The mixed producer reaches the existing typed F06 atomic-annotation facet under independent common/native release. Returned-search replacement binds actual released result identity, original capture/limits, separate parent capture/reply charges, current native service and protected once-only cleanup. Both replacement variants reach installed dispatch and the existing F06 ordinary retained-outcome projection with independent current release. Copied metadata has no handler. Complete cross-family C2/events and release qualification remain separate. No UI, logical-ID wire codec, logical F05 resolver or arbitrary-script authority is claimed. |
| F12 / 3 | `invocation/selection{,/arguments}.rs`, builtin `selection.rs`, existing F06 `observation/{projection,ordinary,ingress}.rs` and search consumers; selection, occurrence and Unicode argument consumer tests | The four existing forms/three semantic variants join native/installed admission, original one-shot execution and typed/dynamic F06 retained-outcome release. The adopted metadata-only result exposes current-permitted applied/unchanged facts, not coordinates or a new selection receipt. Original-context policy, current publication withholding, transient result-read refusal, stable retry and observer/policy discharge are exercised. Metadata does not authorize mutation; logical view state has its separate owner. Visual movement stays outside the engine; other-family C2 qualification remains separate. |
| F13 / 4 | Scoped annotation owner, `invocation/annotation_arguments.rs`, installed `annotation_dispatch.rs`, mixed `atomic_authoring.rs`, original annotation receipt profiles; annotation/mixed-authoring tests | Standalone and mixed forms reuse actual annotation allocation/staging, independent versions and original receipts. The mixed producer is exercised through the existing typed F06 facet, including independent common/native denial and cleanup. Complete cross-family assembly remains C2; no new annotation issuer or source-to-annotation grant. |
| F14 / 2 | Native reversal arguments/owner, installed `reversal_dispatch.rs`, original reversal receipt profile; reversal tests | Installed supported reversal forms implemented; do not replay external effects. |
| F16 / 3 | Native/scoped settings, definition, serialization and registration owners; installed `context_dispatch.rs` ten-form closed sum and original result profiles; `resource/catalog/consumer{,/annotations,/writer}.rs` original correspondence captures | Approved binding-only v1 is implemented and focused-qualified; distinct document/analysis activation is explicitly deferred. The two semantic operations retain installed native forms; host-established correspondence binds immutable actual native targets without provider startup, interpretation change, semantic publication or authority expansion. All 13 consumer tests pass, including four capture modes with unchanged root/generation/task/funding, exact capture bounds, independent current host facts and stale/foreign refusal. Provider execution remains host/integration-owned; analysis stays disabled. C7 final integrated qualification remains separate. |
| F17 / 8 | Native lifecycle, activation, view/workspace containers, rebind and disposal; builtin `lifecycle.rs`, `containers.rs`, `source_rebind.rs`, `retained_release.rs`; lifecycle/container/disposal tests | Supported installed lifecycle routes exist. Final cross-family result/control assembly belongs to C2/C4/C5, not another lifecycle owner or speculative API version. |
| F18 / 2 | `invocation/control/arguments.rs` and installed cancellation; `shutdown/` inventory/fence/disposition/progress/wait/attempt/ingress owners; builtin `shutdown_disposition.rs` | Cancellation integrated. Native C5 shutdown is focused-qualified through actual saves, consent, late settlement/quiescence, disposition, retained report, reclamation and final control retirement. Original historical report return/retirement never discharges producer or document ownership. Fresh complete census gates final release; an absorbing tombstone prevents reopening. Installed `shutdown-disposition/1` now binds original fence/document handles to the accepted native plan through closed typed/dynamic ingress, sealed discovery origin and separate current native facts. Copied descriptors, missing accepted consent and stale repeated disposition refuse. Protected native progress survives discovery teardown. Optional normalization/dispatch and five report/control service boundaries instrument actual execution. The 52 built-in discovery tests pass, including native/copied × typed/dynamic × four capture modes through final zero funding. Complete C1 and C7 remain separate; no safe process-exit or crash-persistence claim. |
| F19 / 4 | Installed source/facet/collection/registry establish/resync and historical receipts; `observation_events.rs`, stop V4 and revocation V2; owning event/control/discovery tests | C4 functional integration is closed for the four implemented publication families. Original command/stream/generation and receipt projections remain correlated; current release, loss/overflow/gap, original opaque-role validation, separate revocation authority, sibling isolation and protected retirement remain at native owners. Typed heterogeneous event framing fits before queue/certificate consumption. Generic-ID lookup is explicitly adapter-owned, not an erased engine registry. F07 snapshots do not imply event streams. Focused publication/observation/registry checks pass; C6 performance and C7 whole-engine qualification remain separate. Diagnostic records never substitute for event products. |
| F20 / 2 | Reader-owned source/annotation load requests, original tasks and native operation observation; installed `source_load.rs` / `annotation_load.rs`; load-facade tests | Installed supported load-candidate paths implemented. Completion does not activate a candidate; activation remains F17. |
| F21 / 2 | Shared writer ordinary source/annotation saves and independently authorized observation; installed `ordinary_save.rs`; save-facade tests | Installed supported ordinary-save paths implemented. Preserve original writer/task ownership and distinct destinations. |
| F24 / 1 | No enabled native parse/query task owner or installed handler | Conditional analysis facility disabled/unadvertised. No scheduler or provider work is implied by metadata-only resource catalogs. |
| F26 / 3 | `diagnostic_capture{,/types,/shutdown_inventory}.rs`, `diagnostic_capture/drain{,/types,/arguments}.rs` and builtin `diagnostic_drain.rs`: actual scoped marker queue, owned one-shot tasks, current copied result/protected control, closed typed/dynamic input and sealed installed hostRequest dispatch; public/private queue/bridge tests and actual parent census | The diagnostic-drain functional profile is implemented. Original generation/limit replay never drains again; eligible service selects the current prefix once. Missing readiness defers; known refusal, performed (including empty) and unknown remain distinct. Parent prepays actual queue-bound batch/result/control capacity; known consumers block retirement. Controlled faults/unwind fence only the optional bridge and preserve historical performed knowledge; current release/generation governs payloads separately. Timer/analysis-task cancellation stay disabled/unadvertised. Specification §23.2.2 supplies this profile's separately authorized ordinary result and protected control; it does not require another invocation/F06 wrapper. Operational qualification remains separate. No exporter/UI, native invocation kind or implicit mapping of richer timed records is added. |
| F27 / 6 | `resource/evidence.rs::ProviderEvidenceV1`, `coordinator/resource/evidence.rs::ingest_provider_evidence_v1`, original source-read/annotation-read/save evidence owners; `tests/provider_evidence_behaviors.rs` and existing native evidence tests | Supported native resource intake is joined through an exhaustive typed facade. Existing opaque evidence preserves original engine/task/provider/attempt authentication before reconciliation; duplicate/conflict results and bounded witnesses remain at their actual owners. Late intake after stop stays possible; completion, no-future-effects and quiescence remain separate. Fixed optional primary ingress timing and secondary duplicate/conflict tags carry no payload labels. This is a trusted provider edge, not an ordinary-recipient command, remote authentication mechanism, new effect runner or clipboard workflow. |
| F28 / 1 | No enabled analysis-result intake owner or installed handler | Conditional analysis facility disabled/unadvertised. Future activation must add its own applicability/conformance, not silently reuse external-write evidence. |

F27's six semantic perspectives map to existing native evidence, not six new
mutually exclusive commands:

| Perspective | Read evidence | Save evidence |
| --- | --- | --- |
| Progress | Original start and ordered bounded chunks | Partial observed bytes with application still unknown |
| Acknowledgment | Original end/version/count produces a candidate/capture, not activation/restoration | Written/application, synchronization and version facts remain independent; no automatic baseline publication |
| Failure | Original typed terminal read failure | Explicit nonapplication or synchronization failure; an adapter I/O exception alone cannot assert either |
| Quiescence | Original dispatch quiescence | Original quiescence is separate from no-future-effects and application |
| Unknown | Missing completion remains unknown; no invented success/failure atom | Original unknown application/synchronization remain unresolved, including after stop |
| Duplicate or contradiction | Existing exact-atom reconciliation and capped conflict witnesses | Existing result/causal-edge reconciliation and capped witnesses; neither duplicates nor conflict results execute an effect |

Intake records one `ProviderEvidenceIntake` population per generic call. Fixed
duplicate/contradiction tags are secondary outcomes, not additional requests.
Native owner spans and ingress spans overlap and must not be summed. Provider
elapsed time, pending age, current recipient observation and protected retirement
remain separate existing interfaces; ingress timing is not an external-I/O SLO.

### C2 native producer/result crosswalk

The native result composition is the existing owning query, not one universal
task ID, receipt store or wire envelope. The following current implementation
inventory supersedes older generic “cross-family assembly remains” notes for C2;
it does not waive C1's outstanding F05/F16 decisions or C4/C5/C6/C7 deliverables.

| Existing producer | Owning retained delivery | Public composition evidence |
| --- | --- | --- |
| Source edits, composition, exact batches and search replacement | F06 ordinary retained-outcome; exhaustive `observation/projection.rs` classification and `ordinary.rs` current release | `selection_invocation_behaviors/{composition,exact_batch}.rs`, `settings_activation_behaviors/source_context/search/consumers/returned_arguments.rs` |
| Four logical selection forms | Same metadata-only F06 outcome; view coordinates are a separate read, not an invented receipt field | `selection_invocation_behaviors/arguments.rs`, `search_selection_behaviors/arguments.rs`, `unicode_invocation_behaviors/selection_arguments.rs` |
| Standalone/mixed annotation authoring | V1 annotation / V4 indexed atomic-annotation receipt, original allocation and native release | `annotation_behaviors/results.rs`, `annotation_behaviors/compound_observation.rs`, `annotation_history_behaviors/atomic_authoring.rs` |
| Reversal | V1 reversal facet with original journal/provenance and current native release | `history_behaviors/reversal_results.rs`, `discovery_behaviors/reversal_dispatch.rs` |
| Settings/definition/lifecycle/serialization activation | Ordinary F06 outcome; created layer identities use the separately admitted V1 created-settings facet | `settings_activation_behaviors/{context_dispatch,created_results}.rs` |
| Host/scoped registration | Distinct V1 host-registration and scoped-registration facets; no stamp/reference conflation | `resource_registration_behaviors.rs`, `scoped_registration_behaviors.rs` |
| View/workspace allocation and relations | V1 relation allocation / V5 live-relation facet | `relation_invocation_behaviors/discovered.rs`, `live_relation_invocation_behaviors/observation.rs` |
| Scratch, disposal, retained release and rebind | V6 scratch, V2 disposal, V3 release and V7 rebind facets | `scratch_receipt_behaviors.rs`, `retained_disposal_invocation_behaviors.rs`, `retained_release_observation_behaviors.rs`, `source_export_behaviors/rebind_facade.rs` |
| Source/annotation candidate activation and SaveAs | V8 source-load, V9 annotation-load and V10 SaveAs original historical result facets | `source_read_behaviors/load_facade.rs`, `annotation_restore_behaviors/load_facade.rs`, `source_export_behaviors/saveas_facade.rs` |
| Four owned resource read/save parents | Separate F06 native-operation observer/receipt profile and installed `builtin/native_operation.rs`; original actual parent/task | `source_read_behaviors/observation.rs`, `save_operation_behaviors/{observation,read_observation,decoder}.rs` |
| Source/facet/collection/registry observation commands | Their original command-retained receipt facets; event payload release is independently authorized | Owning `publication_stream/commands/receipts`, `facet_publication/commands/receipts`, collection and registry receipt tests; installed `builtin/{source_publication,facet_publication,collection_publication,registry_receipt}.rs` |
| Diagnostic drain | §23.2.2 ordinary copied result and protected control at `diagnostic_capture/drain.rs`, independent of invocation identity | `diagnostic_capture_behaviors/drain.rs`, owning `diagnostic_capture/drain/tests.rs` |
| Retained native search and read snapshots | Their existing original opaque result/context queries and protected retirement; no invocation is introduced for a query | `settings_activation_behaviors/source_context/search/consumers.rs`, read/search/annotation/history facade consumers |

`receipt_ingress.rs` exhaustively maps the existing V1–V10 native requests to
their closed facet/index selectors, checks original invocation/context/profile,
then calls the native release chain. Shared admission prepays the complete wrapper
and original payload ceiling; shared release meets original/captured/current
permission before the family owner checks its own grants and provenance. Receipt
faults fence that facility, not the producer or independent protected control.
Observers, policy captures, producer results and source/search holds discharge
through their own existing owners, not by replay, stop or a duplicate result read.

Immediate query results and F27 evidence dispositions do not create a command
receipt obligation. F27 updates the original resource parent's retained knowledge;
optional disabled analysis facilities have no enabled producer to join. C1's
unselected logical search/provider activation contracts remain required decisions,
not missing implementations silently relabeled as optional or as completed C2
producers. New adopted producers must preserve this crosswalk when implemented.

Current Rust F19 stop projection: a closed first-party stop port now composes the
existing read-stop owners with invocation/native-operation observers and discovery
contexts. Exact native ownership/scope/control checks and typed errors remain at
their owners. Stop never implies producer cancellation, funding discharge or
retirement; native fault containment is preserved. Full opaque attachment lookup,
stream establishment/correlation, revocation and authorized resync remain open.

The additive Rust V3 sum also composes the actual source-publication (through V2),
named-facet, annotation-collection, resource-registry stream and registry-receipt
owners. Both stop routes retain `read-context/1`; older exhaustive Rust enums are
unchanged. Original opaque-role matching precedes one native protected stop.
This completes stop wiring for the currently implemented native owner classes,
not full F19: heterogeneous establishment/resync/event ingress and original-command
correlation, required observer classes without native owners, opaque attachment
resolution/installed ingress and U06 revocation remain acceptance work.

Registry-specific F19 establishment/resync now have installed /1 discovery sources
and exact typed/dynamic native request lowering to their original command owner.
Private origin/variant matching and current discovery release precede independent
native admission; generic metadata cannot install that route. First execution and
protected identity-only replay preserve the original command/result relation, with
payloads separately authorized through F06. This closes that registry-specific
ingress gap, not other observer-family commands or full heterogeneous F19.

Source publication likewise now connects installed discovery to both original
establish/resync commands and separately authorized F06 historical source receipts.
Closed typed/dynamic ingress and actual result-identity validation reuse native
owners; copied metadata and cross-kind selectors cannot execute. Heterogeneous
receipt control, remaining observer families and full gate closure remain open.

Current native event delivery now has a closed heterogeneous port/result over
source, named-facet, collection and registry stream owners. Actual release guards,
issuer generations and projections are preserved. Complete reply-header funding is
checked before consuming an event or registry certificate. This is bound native
delivery, not the missing opaque attachment resolver or establishment/replay for
other required observer families.

Current Rust F20 projection: installed source/annotation read-admission variants
now dispatch from scoped discovery /1 to their original reader-owned parent and
child. Private origin/family/effect-request checks and independent native facts
remain required. Matching recovery preserves original target/catalog provenance
after edits or withdrawal without rereading; changed attempt and spent identities
refuse. The original task joins existing native-operation observation for current
authorized evidence/control. This does not activate or restore content, substitute
a new invocation/task store, or close F19 and whole C1/C2 acceptance.

Current Rust F17 projection: installed scratch/attach/detach/membership/disposal
routes are now joined by the four view/workspace container create/retire forms.
The container source reuses the existing scoped discovery owner, original native
relation invocation and separately authorized F06 result; it does not widen the
older built-in inventories or establish arbitrary-recipient execution authority.
The three existing retained-disposal release forms now also connect installed
discovery to the original native owner and F06 V3, without a new parser or verb.
Loaded-candidate source activation now has bounded native owned admission,
fresh conditional create/join service, original replay and separately authorized
historical identity/disposition release with protected cleanup. The fixed installed
source and F06 V8 now join that native route, preserving old discovery inventories
and receipt profiles. Both annotation restore/adopt-baseline forms now likewise
join fixed installed discovery and F06 V9 over their owned native invocation;
strict phase/origin matching and independent current release remain required.
Acknowledged SaveAs activation now joins its installed discovery /1 source to
the original native task-bound invocation and F06 V10 historical result. This
route does not admit or dispatch a second write; original capability and current
independent release remain required. Published V1–V9 profiles are preserved.
Final storage reclamation and other routes remain separate obligations. This does
not close all F17 or C1/C2.
Source rebinding now has its Rust native owned invocation and independently
authorized historical result: immutable original roles/limits, fresh native
service, actual writer checks and staged terminal correlation are implemented.
The selected native route now also connects fixed installed discovery and F06 V7,
preserving the older inventories/receipt profiles. This is not recipient
authentication or complete C1/C2 closure.
See the current
[engine acceptance plan](engine-test-plan.md#current-implementation-evidence-by-gate)
for Rust evidence; the matrix's historical accepted bindings remain reference/
specification evidence, not full implementation closure.

The required headless integration workflows remain open at their original semantic
and conformance scope; moving ownership does not remove transfer behavior:

| Family | Integration variants | Existing contracts and remaining binding |
| --- | --- | --- |
| F11 | accepted-paste, validated-cut-deletion | [Local continuations](../formal/clipboard-invocations.md): original task/request, attributed accepted payload or export, exact current target and one normal validated engine edit; matching retry cannot dispatch another effect. |
| F22 | plain-or-supported-structured-read | [Clipboard products](../formal/clipboard-products.md): advertised formats, original target, task, dispatch and replay binding. A provider receipt is not paste publication. |
| F23 | copy, cut-export-stage | [Clipboard products](../formal/clipboard-products.md): include copy's independent effect outcome and cut's export-before-delete relation. Refused deletion can coexist with confirmed export. |

F09, F15, F22, F23 and F25 have no engine variants. Presentation also owns
F12/supported-visual-movement; F26/dialog, native-input-accessibility and
host-preference; and F28/enabled-geometry-result. All eleven remain accounted for
in L11's separate [UI example stream](reference-ui-design.md), not as engine API
work or engine-completion blockers. Layout/decoration producer contracts and native
UI support are not missing engine products. The engine supplies behavior, logical
state, effects and authorized data projections for UI and automation consumers.

## Six integration work items

These are subdivisions of L02, not new goal deliverables or additional proof
programs. Close each by reusing a sufficient existing contract, completing a
specific missing contract, or recording a decision that genuinely prevents closure.

### Accepted source and annotation query bindings

The existing logical facade binding is accepted for all six F02/F03 variants
under their declared supported profiles. Specification §§12.8, 22.1.10–17 and
the native argument/gateway/result declarations already supply the contracts.
This accepts existing design and qualified scoped reference evidence, not new
public identifiers, Rust types or codecs. No source changes or additional proof
program are needed for this disposition.

| Variant | Exact existing request product | Existing released product |
| --- | --- | --- |
| F02 bounded-range | `source-query/1`: contract, operation, snapshot, document, profile, lo, hi | Content: status, ref, range, unit, source, context; or status-only unavailable. |
| F02 line | `source-query/1`: contract, operation, snapshot, document, profile, line | Line: status, context, ref, unit, line, body, delimiter; or status-only unavailable. |
| F02 snapshot-handoff | `read-context/1`: contract, operation, document, selection, data, profile; or `read-version-set/1`: contract, operation, authority, participants, selection, profile | Single: status, context, data; coherent: status, context, participants with document/snapshot/data rows; or status-only unavailable. |
| F03 enumerate | `annotation-query/1`: contract, operation, snapshot, document, profile, limit, continuation | Page: status, snapshot, records, continuation; or status-only unavailable. |
| F03 inspect | `annotation-query/1`: contract, operation, snapshot, document, profile, annotation | Record: status, snapshot, record; or status-only unavailable. |
| F03 target-location | `annotation-query/1`: contract, operation, snapshot, document, profile, annotation | Location: status, snapshot, reference, location; or status-only unavailable. |

The exact nested products and guards remain the source of truth:
[source products](../formal/source-products.json),
[annotation products](../formal/annotation-queries.json),
[acquisition arguments](../formal/read-acquisition-arguments.mjs),
[acquisition results](../formal/read-acquisition-results.md),
[source results](../formal/source-read-results.md) and
[annotation results](../formal/annotation-query-results.md). The table is an
index, not permission to omit nested fields, bounds or native binding checks.

| L02 criterion | Accepted contract and evidence scope |
| --- | --- |
| Exact arguments and versions | Shared immutable native declarations bind every field to its variant and contract version. Handoff selects current capture or an original retained reference; data classes are an explicit nonempty subset of source/annotations. Coherent participant rows are exact and unique. Unknown versions/fields or unsupported meanings are not silently upgraded. |
| Exact results | Closed content/line, acquisition and page/record/location products have construction and facade guards. Native staged aliases/candidate rosters are checked independently of returned shape. Source units are UTF-8 bytes. Optional annotation facets remain withheld, not fabricated empty or detached values. |
| Authority and dependencies | Original snapshot/member/record/continuation roles are preserved. The actual native owner supplies captured source and current authority; a structural copy is not ownership. Coherent member use revalidates the whole required parent scope. Source-only acquisition cannot promote itself into annotation access. |
| Bounds and control | Parent classification plus eight acquisition or seven query frames are reserved/refunded separately from native retained contexts, replies and selectors. Capture byte/node/depth, native page/reply, roster and lifetime issuance limits remain independent. Owner stop, independent retained copies and protected discharge retain their specified lifetime meanings. |
| Typed and dynamic agreement | Both routes reconstruct the same ordinary request meaning and original opaque roles before actual native dispatch. The adopted acquisition-to-annotation-query corpus covers all four route combinations, 24 current/retained owner chains and 72 page/record/location products, plus twelve source-only promotion refusals. This does not require fresh aliases from separate calls to be pointer-equal. |
| Discovery and capability boundaries | Explicit discovery /5 describes the existing input forms; /6 adds the existing result products while preserving earlier profiles. Metadata is structural, not permission or proof of producer truth. Advertised concrete read profiles remain subject to native supported/currentness checks. |
| Outcomes and events | These are bounded queries, including acquisition of derived read/accounting state, not authored commands. Their immediate products do not require a second invocation terminal receipt, an authored publication event or U01's operation-result route. Subsequent subscription/resynchronization is F19, not an implicit side effect of a read. |
| Fault and release behavior | Preinstallation native faults publish no aliases; a later facade fault preserves already-installed contexts/selectors, consumed cuts and spent issuance. Refusal cannot claim rollback. Current release and native protected cleanup remain required; authored roots are unchanged by these read paths. |
| Instrumentation | Existing IF-01/02, PATH-02 and INS-01/03/05/08/12/13 distinguish argument capture, native installation, current release, ordinary refusal, invariant fault, temporary refund and retained pressure. Raw content, object identities and handles are not ordinary labels. |

The adopted 0.201/0.265 report passes the argument, owner gateway, composition,
result shape, result-invariant fault, selected substrate and handler-bound
discovery checks for these products. The source/report inputs match. These checks
are scoped finite evidence and conditional ownership arguments; they do not
establish actual host policy, physical allocation, concurrent/crash refinement,
all historical-root lookup or independent certification.

L04's cross-family lifetime design is accepted in the closure ledger; L11/L12
retain edge-contract review and L18 final reconciliation. E01–E05 retain production API,
host/platform, measurement and assurance execution. Those gates do not require
recreating these six already specified logical query bindings. A concrete
contradiction may reopen the corresponding binding; a historical generic phrase
such as "complete public products remain" is not, by itself, a new missing field.

### Accepted history query bindings

The three existing F04 logical bindings are also accepted under specification
§§14.5–14.5.4 and 22.1.18. Their exact native request declaration is
[history-queries.json](../formal/history-queries.json): `history-query/1` with
contract, operation, context, scope, direction and profile. Availability and
permitted-blocking-detail additionally require group; frontier has no group
field. Direction is the closed undo/redo sum. Null group retains its declared
query meaning and is not a command to choose a new reversal target.

| Variant | Existing immediate result |
| --- | --- |
| frontier | status, context, group; established empty has explicit null group. |
| availability | status, context, value; value is empty, eligible or blocked. |
| permitted-blocking-detail | status, context, blockers; dense bounded unique scope/group rows in the declared order. |

Each operation also admits status-only unavailable. The actual result guards
bind the original context and operation, independently retained permitted aliases,
captured detail ceiling, reply bound and copied value integrity. Shape-valid
availability is not proof of history eligibility. Whole-report current release
and native history-basis truth remain separate requirements.

Both routes share original opaque context identity and copied ordinary meaning;
the actual owner's eight-field authority scope includes policy. The gateway
reserves classification plus seven request frames against that history owner's
headroom, excludes reentry and refunds only temporary workspace. Its native base
reservation, contexts and spent issuance survive faults. Recognized authority
loss is absorbing; unknown facts withhold without proving revocation. Stop and
fault are separate from protected individual close/retirement and never cancel
an independently owned reversal.

The existing qualified report passes native history, decoded argument,
result/fault, owner gateway/substrate and explicit discovery /7 checks. The
argument corpus includes 24 native typed/dynamic compositions across six
operation/direction forms; the gateway includes 24 native route/profile cases,
fixed fault/reentry cases and 27 assertion-detected mutations; the result corpus
has 31 mutations. These existing checks supply N01's local binding and the
corresponding scoped N06 audit, not full linked-history or producer/host truth.

These are immediate queries: a second invocation receipt, new publication event
or U01 choice is not applicable. INS-01/03/06/07/08/12/13 already distinguish
workspace admission, native advice/release, invariant fault and protected refund.
The exact version/refusal, conditional disclosure, bounds and lifetime contracts
are therefore already specified for this binding. L04/L07 design acceptance is
recorded in the closure ledger; L18 and E01–E05 retain final reconciliation, actual
implementation/policy/retention and independent assurance. No new history API or
proof program is introduced.

### Accepted settings configuration query profile

Current Rust boundary (2026-11-07): `settings::project_settings_v1` implements the
pure bounded projection-building component over an already privileged
`CompiledSettingsV1`. It requires complete per-key facts, checks effective origin
subsets, preserves accepted schema order and separately selects owned values,
per-field origins and declarations without default fields. Sensitivity supplies
no grant. The helper returns privileged data, including selected private origin
identities; it is explicitly not an F07 recipient API or authenticated permission
check. Projected values/types use owned flat graphs and borrowed traversal, not
recursive engine copy/drop or recursive JSON-shaped return objects. The scoped
native reader now implements separately funded original/current one-use cuts,
eight-field recipient scope, non-widening current release, opaque context-local
origin references, absorbing invalidation, protected cleanup and lifetime issuance.
Its borrowed typed operation enum covers all three configuration-query meanings;
the public Rust consumer scenario is
`tests/settings_behaviors/settings_recipient.rs`. The same consumer now exercises
native, typed and dynamic calls. `query_decoded_settings_read_v1` captures ordinary
meaning with separate original opaque roles, while
`prepare_native_settings_read_discovery_cut_v1` and
`query_discovered_settings_read_v1` supply the sealed installed query route.
Closed result frames carry original context/target/key and owned filtered payloads.
Decoder workspace is separately leased from the actual shared parent; native
logical capture ceilings are not reference-model JSON or physical-byte bounds.
Decoder faults stay pool-local and protected cleanup remains available. This
closes functional ingress/discovery integration for this configuration-query/1
profile, not all F07 profiles, C1, release-performance qualification or formal
refinement. Existing `SourceCoordinator::settings_read` stays privileged.

F07's existing `configuration-query/1` profile is accepted for effective-
configuration, permitted-origin and capabilities. It does not stand for every
F07 capabilities profile. Its exact fields come from the immutable
[input declarations](../formal/settings-query-declarations.mjs) and
[result declarations](../formal/settings-query-results.mjs).

| Operation | Exact request fields | Immediate successful result fields |
| --- | --- | --- |
| effective-configuration | contract, operation, context, target, profile, key | status, context, target, key, value |
| permitted-origin | contract, operation, context, target, profile, key | status, context, target, key, coverage, origins |
| capabilities | contract, operation, context, target, profile | status, context, target, coverage, capabilities |

All three also admit status-only unavailable. Original context/target and
context-local origin identities remain separate opaque roles. Native guards
compare released values, origins and accepted declarations with the retained
permitted projection, not just with a plausible shape. Origin and capability
coverage is permitted-only. Empty output does not prove no hidden contributions;
capabilities describes permitted accepted setting declarations, not all engine
features, successful probes or authority. Defaults and private layer identities
are not incidental output.

The actual settings owner reserves its pool against its shared parent. The
decoded gateway separately reserves classification plus seven request frames,
captures ordinary arguments/current facts, excludes reentry, preserves original
roles, dispatches to the same native reader and refunds only temporary workspace.
Known projection/authority loss is absorbing; transient pressure withholds.
Native context/result faults, decoder-pool fencing and execution-domain faults
retain their different scopes. Protected cuts/context/pool discharge remain
available without refunding spent issuance or manufacturing a healthy context.

Explicit discovery /9 describes the existing closed products and bounded
recursive setting-type syntax. It is neither an evaluator nor a permission grant.
The current qualified report passes native reads, exact arguments, result/fault,
owner gateway, discovery and selected substrate checks. Queries have no authored
effect, asynchronous event or separate operation-specific terminal receipt in
this profile; U01 is not needed to retrieve these immediate values. Existing
IF-01/02 and INS-01/03/05/07/08/12/13 cover capture, release, pressure, faults
and protected discharge. This accepts the configuration profile by reuse;
L04 lifetime and L05 configuration design are accepted; L18 and E01–E05 remain separate.

### Accepted resource catalog capabilities binding

Current Rust boundary (2026-11-07): `resource/catalog/read.rs` now supplies a
separately parent-funded native recipient owner, complete actual-root visibility
preparation, acquisition, list/inspect and protected lifetime controls. Original
opaque context/entry/continuation roles, captured ordering/descriptor ceilings,
stable page retry and staged alias publication are executable. Descriptors include
the writer identity-known flag but exclude native identities, locators and stamps.
Current loss, narrowing and changed permitted metadata invalidate only the bound
context; transient pressure does not refund issuance or invalidate siblings.
The public consumer is `tests/resource_catalog_behaviors/recipient.rs`.
`query_decoded_resource_catalog_read_v1` now supplies the distinct closed
resource-catalog-query/1 typed/dynamic envelope; its normalizer retains original
roles separately from owned inert captures. The actual parent intent ceiling
intersects input bounds and actual parent headroom funds classification plus
seven request frames. `prepare_native_catalog_read_discovery_cut_v1` and
`query_discovered_resource_catalog_read_v1` bind installed metadata to the exact
catalog handler without widening the configuration /9 source. Copied descriptors
cannot install handlers. Current discovery and catalog release remain independent.
Final reply checks bind actual installed aliases and captured metadata. Busy/
decoder-fault fences retain native obligations and permit protected cleanup after
temporary leases discharge. This completes functional integration of the accepted
catalog profile, not JSON/wire conformance, measured performance or whole C1.

Do not extend configuration-profile acceptance to all of `resource-catalog-query/1`
merely because both use F07/capabilities. Qualified/adopted 0.202 §22.2.2.5
resolves the earlier actual-parent funding defect. The native registry reserves
its retained cost before installation; bounded decoder workspace borrows actual
parent headroom and refunds only temporary units. Final registry discharge
refunds its reservation once without reopening lifetime issuance. All 760 checks
pass, including the reader's 28 controls/49 mutations, four decoder fault stages
and nineteen selected registered integration checks. This is logical reservation
evidence, not a physical-memory or production-provider guarantee.

The selected F07/version-1 capabilities alternative is now bound to the actual
registrationReadInputForm / registrationQueryArgumentInputForm declarations,
queryDecodedRegistrationRead and queryRegistrationRead. It does not borrow a
configuration-query/1 context or settings-result meaning:

| Product | Exact existing meaning |
| --- | --- |
| Outer request | contract, operation, context, profile, request; contract resource-catalog-query/1, operation capabilities, profile scoped-resource-catalog/1. |
| List alternative | request contains exactly operation, limit, continuation; operation list, positive bounded integer limit, null or the original context-local continuation. |
| Inspect alternative | request contains exactly operation and entry; operation inspect and the original context-local entry. |
| Immediate results | Page with original context, captured-permitted coverage, bounded entry references/read-write table labels and original continuation; registration with original context/entry and permitted descriptor; or status-only unavailable. |

Context establishment uses the existing owned registry/visibility/acquisition
ports and actual parent reservation. Those setup/cleanup ports need not become
new discoverable commands. The logical facade retains the original context,
entry/continuation role, visibility packet, scope/profile and limit products.
Current visibility/release, captured ceilings and absorbing known loss remain
native conditions. A metadata grant grants no resource-content access, write
authority or handler execution.

Discovery uses the existing generic accepted-registry bounded field metadata,
linked to this exact declaration and handler. Preserve contract/profile
discrimination wherever capabilities has multiple alternatives; do not infer it
from a shared variant label. Native builtin /9 still describes configuration,
not this resource alternative. No new builtin profile or public contract string
is required. Actual accepted-registry/schema/handler correspondence remains a
trusted-producer premise, not a conclusion of metadata validation. Separate
bounded review found no other semantic exception for this selected binding.

The native reader validates page meaning before selector installation and checks
the released reply against the actual installed maps. Inspection compares the
disclosed value with its captured permitted row. Valid reply pressure is ordinary
unavailable; a contradictory native product invalidates only that context while
retaining its slot, spent issuance and registry/parent obligations. The charged
decoder separately fences its pool on fault and refunds only temporary frames.
Protected stop, cut/context discharge and final registry refund remain available.

These queries create no authored publication, operation-specific terminal receipt
or mandatory async event. U01 does not govern their immediate values; resource
registration observation remains a distinct F19 contract. This accepts N01/N06
for this existing profile, not every possible F07 conditional domain or full L02.
Actual installed handler/provider truth and E01–E05 remain separate; no source,
model, profile or verifier change is needed for this binding disposition.

### N01 Bind the existing semantic products

For each row, connect the exact supported argument alternatives to the owning
operation, logical version, original opaque roles, authority/dependencies,
immediate result and applicable retained result. Define unsupported, malformed,
stale, unavailable and withheld behavior at the owning boundary without inventing
an externally observable precedence among simultaneous private refusal reasons.
Use the existing native field sets where adequate. Record a genuine unsupported
profile instead of silently broadening a native reference.

Logical product and version binding suffices for this design pass; selecting Rust
type spelling, an RPC envelope or token codec is not a prerequisite. Structural
validation is not native ownership or current authority. Private preparation and
protected-discharge substeps need explicit facade access where necessary, but
need not each become a separately discoverable editor command.

#### Normalization bridge found by the publication audit

Requirements §5.7 and specification §4.2.1 require shared admission of unambiguous
unsorted original-snapshot edits. [Batch normalization](../formal/batch-normalization.md)
already defines and checks that behavior. The private
[compound evaluator](../formal/annotation-compounds.mjs) expressly consumes
normalized plans and validates ordered edits. Before 0.201 the invocation owner's
read-plan normalizer canonicalized read sets and scopes, not source edit order.
Neither isolated reference alone established the public-to-private bridge.

A direct read-only reference probe used source `abcd` with replacements
`[0,1) -> X` and `[2,3) -> Y`, correct expected bytes and otherwise identical
current context. `decideAnnotationPlan` accepted their ordered list with candidate
`XbYd`, but returned the unchanged root for the reversed list. Both calls preserved
their input state. This confirms the private normalized-input boundary, not a
defect in a production facade that does not yet exist.

The adopted 0.201/model 0.265 owner now normalizes before view preparation,
immutable intent/retry comparison and candidate evaluation. Complete records,
boundaries, original contexts and expected-content checks remain intact. The
full frozen suite passes all 60 permutation/profile/route cases and all 28
invocation mutations; source, worker and adopted-root report/input audits pass.
The private ordered evaluator and public catalog vocabulary are unchanged.
This closes the normalization bridge and supports L03's accepted scoped contract,
not production decoding, concurrent installation or the whole L02 facade.

### Accepted proposal validation query binding

F10's existing `proposal-validation/1` logical binding is accepted for
`native-compound-current-checks/1`. The exact [declaration](../formal/proposal-validation.json)
requires contract, operation and plan. Typed/dynamic route envelopes carry route,
request and proposal; actual catalog routing selects F10/advisory-validation,
not an arbitrary evaluator. The nested native compound `/1`–`/5` products retain
their closed fields, source/read/scope/annotation meanings and native validation.
The outer metadata schema does not claim to validate those nested values.

The result is exactly advice with status, fixed profile and Boolean
checksSatisfied, or status-only unavailable. The original observation basis binds
actual current root, complete recipient context and original request. Complete
proposal-readability/validation-observability and ordinary current gates precede
candidate work; caller labels or positive metadata cannot supply them. The shared
owner normalizes records before the same source/annotation/view/read checks used
for publication. Advice mutates no authored root, invocation accounting or committed
source journal and returns neither candidate root nor execution authority.

Actual owner intent/root/reply limits and finite candidate/view/dependency profiles
bound the synchronous query. Current stop still withholds advice; no admitted
execution, wait handle, provider launch, terminal invocation receipt or authored
event needs cancellation or discharge. Unexpected query failure fences the owner
and returns no verdict rather than proving the proposal invalid. Protected
responsibility for already admitted work remains. U01 is not needed for this
immediate query; later execution is a separately admitted, fully revalidated intent.

The adopted report passes native advisory validation, fixed declaration, all five
advisory metadata mutation banks and selected substrate checks, plus the shared
normalization/publication composition. Explicit discovery /12 describes the
existing outer input and advice/unavailable outcomes; /13 preserves that profile.
INS-01/02/03 distinguish candidate work, current release and actual publication;
advice cannot count as a commit. Private plans, bases and influencing policy facts
are not ordinary diagnostic labels. This accepts N01/N06 for the supported query,
not every job's validation, provider truth, hostile-runtime containment, production
ABI/conformance or the complete L02 facade. L04 lifetime design is accepted;
L09/L14–L18 and E01–E05 retain their cross-cutting obligations.

### Accepted selection and logical navigation bindings

F12's three engine variants are accepted at the existing logical binding level
under specification §§6.5.2.2, 15.5, 22.1.25 and 26.1.1–3. This reuses the
qualified 0.202 contracts and evidence; it selects no new identifier, Rust layout,
receipt, event family or UI behavior. Four forms implement three semantic verbs.

All forms bind contract, operation, invocation, exact view/document incarnations,
sourceRevision, expectedSelection and profile. The existing declarations remain
authoritative for nested validation and closed vocabularies:
[selection-edit/1](../formal/selection-products.json),
[boundary-command/1](../formal/boundary-commands.json),
[argument capture](../formal/selection-arguments.mjs) and
[native gateway](../formal/selection-gateway.md).

| Variant and native form | Additional exact request fields | Supported meaning |
| --- | --- | --- |
| set-selection; selection-edit/1 | selections, primary | Explicit anchor/head/affinity sets under explicit-positional-selection/1. |
| logical-movement; selection-edit/1 | unit, extend | Line/document start/end; explicit extension, no visual approximation. |
| logical-movement; boundary-command/1 | boundary, unit, direction, count, extend | Grapheme/word-start/word-end under unicode-logical-edit/1, using the original retained boundary handle. |
| select-search-match; selection-edit/1 | match, direction, affinity, selectionMode | Original owned occurrence and explicit replace-all/replace-primary/add-primary selection mode; not a raw range or fresh query. |

| L02 criterion | Accepted contract and scoped evidence |
| --- | --- |
| Exact arguments and versions | Native fixed declarations and normalizer bind each tuple to its contract/profile/variant and exact fields. Unknown fields, versions, profiles and mixed forms refuse; metadata does not validate nested selection meaning or authorize a handle. |
| Owner and execution | Original issued domain/ticket owns one bounded canonical capture and native candidate. Typed/dynamic retries retain the same meaning before, during and after execution; changed arguments/limits or opaque roles refuse. Native preparation/service use the same local publication coordinator as ordinary edits. |
| Authority and dependencies | Current source-read/view-write, source/selection revisions and native occurrence/boundary context revalidate. Source-write is not required for view-only publication. Current documents, authored groups, unrelated views and later comments are preserved. |
| Results and refusal | Existing invocation settlement is settled or faulted, with separately authorized retained publication/control facts: applied/done, unchanged/done, notApplied/rejected, notApplied/stopped, unknown/faulted. These are the existing terminal pairs, not a guaranteed rich response. No separate created-reference or selection receipt is specified, so U01 does not block this binding. Missing/withheld outcome remains unavailable, not known nonapplication. |
| Applicable observations | Existing publication-observation contracts project currently permitted named views; no implicit subscriber, callback or operation-specific event is added. Cross-family F06/F19 context, outcome and event assembly stays N03 and cannot be inferred complete from this acceptance. |
| Bounds and control | Complete canonical capture reserves actual remaining owner headroom before native preparation. Native request/view/selection/boundary/publication bounds remain independent. F18 pending stop cannot free or refresh the plan; protected retirement releases held logical capture without refunding lifetime issuance. No numeric default, physical reclamation or native preemption follows. |
| Faults and current release | Busy preparation excludes competing preparation/submission/retirement. Retained-preparation faults fence admission and preserve cleanup; publication faults do not permit another execution. Current recipient release remains separate from historical publication and cannot widen its captured ceiling. |
| Discovery | Explicit /11 describes the four outer forms and existing terminal dispositions, preserving /1–/10; adopted /12–/13 retain that meaning. Discovery selectors/advice are not native boundary, match or invocation authority. |
| Instrumentation | IF-01/02 and INS-01/02/03/05/10/13 separate admission/replay, publication, release, native evidence, view intent and discharge. INS-11 is a frontend-input edge where present. Content, paths, raw handles and identities stay out of ordinary metric labels. |

The adopted full report passes 72 normalization boundaries/eleven argument
mutations; 32 gateway controls, 156 permission cases, 48 capture boundaries/ten
gateway mutations; eight retained-preparation/publication fault cases; nineteen
declaration mutations; four six-mutation discovery banks and selected substrate
checks. Shared F18/context/observer coverage includes these forms in its actual
38-form/twelve-kind corpus with twelve mutations. Older references' smaller
counts and broad exclusions retain their historical scopes, not new requirements.

This accepts N01/N06 for these existing native profiles. N03/F06/F19 assembly,
the remaining L02 families, L11 UI edges and E01–E05 remain open. No new model
or full-suite rerun is required merely to repeat this adequate qualified evidence.

### Accepted ordinary edit binding

F11/ordinary-edit's logical binding is accepted for its four existing native
forms under specification §§6.5.2.1, 22.1.8 and 26.1.1–3. This is not acceptance
of every F11 ingress. Exact fields come from the same native declarations used
by the [normalizer](../formal/ordinary-edit-arguments.mjs) and
[discovery descriptor](../formal/ordinary-edit-descriptor.mjs), not a second
argument grammar.

All four forms require contract, operation, invocation, document, view,
sourceRevision, expectedSelection and profile. Operation is ordinary-edit.

| Native form and profile | Additional exact fields | Native meaning |
| --- | --- | --- |
| selection-edit/1; explicit-positional-selection/1 | text, selectionAfter | Literal scalar text at the original selection set; mapped or carets-after selection disposition. |
| line-edit/1; explicit-prefix-line-edit/1 | action, settings | Explicit indent-lines/outdent-lines/insert-newline and the applicable unit/tabWidth or delimiter/autoIndent record. |
| boundary-command/1; unicode-logical-edit/1 | boundary, unit, direction, count, extend | Supported grapheme/word deletion through original retained boundary evidence, not a structural copy or visual approximation. |
| configured-line-edit/1; configured-prefix-line/1 | action, configuration | Same simple line actions with exact target/key binding to owned current configuration; no caller-fabricated configuration truth. |

Typed/dynamic routes capture identical complete meaning under the original
domain/ticket, with one canonical charge and one retained native plan. Matching
retries cannot resample text, carets, configuration or boundary evidence; changed
data/limits/handles refuse. At preparation and service, current source-read,
view-write, source-write/all-target-write, exact source/selection/annotation/
configuration and all-attached-view companion checks remain applicable. Native
guards, not outer schemas, decide nested validity and read-only/no-op/conflict.

The common coordinator stages source, annotation placement, linked history and
attached-view selection changes for one local publication. It preserves current
peer state and does not perform file I/O or claim persistence. Outcomes reuse the
five qualified terminal pairs and current-recipient release described for F12;
there is no separately created result reference requiring U01. Applicable source/
annotation/view observations use existing publication projections. Their complete
cross-family N03/F06/F19 assembly is still open.

Actual remaining owner headroom funds complete canonical capture before native
preparation; native settings/boundary/view reservations remain independent.
Retained-preparation faults and pending stop preserve original eligibility and
protected obligations. Retirement refunds held logical capture once, not lifetime
issuance or another owner's assets. No callback, arbitrary script, template
evaluation or new numeric default enters publication.

Qualified 0.202 passes all four argument forms, 43 omitted-field cases, 72
byte/node/depth boundaries and thirteen argument mutations; 35 gateway controls,
162 permission cases, 48 capture boundaries, eleven gateway mutations and two
post-lease fault cases; the configured consumer's 28 controls/eleven mutations;
and the existing /3 declaration/discovery corpus with seventeen mutations and
selected substrate checks. Later adopted discovery profiles preserve /3's meaning.
Shared cancellation/context/observer coverage also contains these forms. Existing
INS-01/02/03/06/07/09/10/13 cover admission, publication, release, source history,
annotation mapping, interpretation, view companions and protected discharge.

This accepts N01/N06 for ordinary-edit, with E01–E05 retained. The two replacement
bindings are accepted separately below through the generic registry contract;
accepted-paste/cut are not declared complete merely because their eventual
mutation can reuse this core. Composition has its own accepted binding below.
Exact-batch retains its distinct original-
snapshot proposal/dependency contract and is not passed through selection
normalization. No new behavior or proof program is introduced by this audit.

### Accepted composition commit binding

F11/accepted-composition-commit cannot simply alias the existing literal
ordinary-edit form. Specification §20.1 requires one accepted replacement at the
primary target, with secondary selections mapped rather than receiving duplicate
insertion. The native ordinary producer correctly edits every selection.

The [native counterexample](composition-binding-counterexample.mjs) starts with
`abc`, a secondary selection over `a` and the primary selection over `c`.
Submitting ordinary literal `X` through the actual invocation gateway publishes
`XbX`; primary-only replacement requires `abX`. This identified the former missing
composition binding, not an ordinary-edit defect. The working counterexample is
outside the qualified input inventory; the adopted binding below supplies the producer.

The binding retains
the original document/view incarnation, source and selection versions, exact
replacement range and expected content, authority and bounded invocation capture.
It must stage the primary caret and mapped secondary/peer selections with the
source, annotation and single history publication. Reusing the private derived-edit
port alone is insufficient: its carets-after path constructs one caret per edit,
not preservation of the original multi-selection set around a primary-only edit.
The original invocation's replay, stale-context refusal, stop, capacity and
retirement obligations remain applicable. Existing ordinary profiles must not
silently acquire new meaning.

Preedit sessions, focus, native index conversion, cancel-before-command ordering
and candidate geometry remain UI responsibilities. An accepted replacement range
must not be restricted to the current selection merely to simplify this binding:
§20.1 also accounts for native surrounding-text replacement ranges. No engine IME
session or new presentation state is required.

Adopted specification 0.207/model 0.271 contains the separate composition-commit/1
normalizer, primary-only producer and original-owner facade binding. Sixteen
controls, ten omissions, eight current-service permission cases and six coherent-
graph mutations pass within the qualified 765-check batch. The bounded review
found no remaining must-fix after merge, competing-wrapper and capacity cases
were added. Ordinary/selection gateway and native editing regressions also pass.
This accepts the selected composition binding for N01/N06, not whole L02 or UI
preedit integration.

### Exact batch ingress and repeated source entries

The private owned compound plan is not itself a complete public F11 exact-batch
request. Specification §§3/13.9 requires explicit source targets/dependencies and
allows preparation to compose source and separately authorized annotation intent.
Native source components also carry engine-owned annotation-set expectations;
requiring a source-only caller to discover private annotation state would cross
the release boundary. Public preparation must retain the caller's complete intent
while capturing those engine-owned dependencies once, not refresh them on retry
or expose a generic execute-anything job constructor. The source-only binding is
accepted below; mixed F11/F13 composition retains its separate qualification and
result-route boundaries.

A native canonicalization gap was corrected in 0.206 and adopted in 0.207.
The former owner admitted two consistent entries for one document but then the
private evaluator refuses duplicates: replacing `a` and `c` in `abc` with `X` and
`Z` leaves `abc`/NotApplied instead of the required combined `XbZ`. Shared owner
and F10 normalization now group equal exact document incarnations under agreeing
source/annotation-set headers, preserve all records, then apply existing ordering
and ambiguity checks. First-occurrence participant order remains; read predicates,
write scopes and authored annotation operations retain their own contracts.

Independent grouped edit arrays prevent alias-based self-extension. Ten
profile/route cases plus three added controls/mutations pass inside the existing
owner checker, now 27 controls/37 mutations, including grouped/partitioned replay,
header conflicts, duplicate overlap and non-elided empty expected conditions.
The bounded reviewer found no must-fix. The actual registered predicate and full
0.207 qualification pass. This accepts repeated-source ingress canonicalization;
it does not imply complete mixed-authoring ingress or whole-family closure.

Adopted 0.207 supplies the source-only exact-batch binding
through exact-source-batch/1, without exposing the private compound plan. Its
exact public fields are contract, operation, invocation, profile, sources, reads
and scopes; source entries have document, expectedSource and complete edit records.
Canonical public intent is captured once, then engine-owned annotation-set stamps
are captured privately once. Replays retain those stamps. Current service views
map without an accidental admission-time selection dependency. Shared terminal/
retirement/discovery contracts apply; no new created result needs U01.

Fourteen controls/seven omissions/sixteen service-permission cases/six semantic
mutations pass the actual registered predicate and full 0.207 qualification.
Separate bounded review found no must-fix. This accepts source-only exact-batch
for N01/N06. Mixed atomic F11/F13 authored composition remains required and is
not replaced by sequential calls; its later candidate is described below.
This does not close the whole F11 family or L02.

The [mixed source and annotation design](atomic-authoring-design.md) maps the
remaining public proposal to existing §13.9 authority, version, same-record,
candidate-target, allocation, history and fault contracts. Its [native probe](mixed-target-binding-counterexample.mjs)
shows why standalone F13 live-source targets cannot substitute for same-proposal
candidate targets: append plus reattachment publishes once, preserving body and
body version. The private probe alone is not evidence of a public facade. Adopted
0.210 now supplies the mixed binding and internal receipt composition below;
public receipt retrieval and complete cross-family observations remain separate.

The [combined native reference](atomic-authoring-reference.mjs) now exercises
source plus all four annotation operations through the actual shared invocation
owner, with five changes/four history scopes, five whole-plan conflict controls
and an Unknown/faulted protocol case. Four receipt-policy checks use the exact
published root and individual admission ceilings; they do not establish the
coordinator-owned receipt vector, aggregate funding or public result retrieval.

The 0.208/model 0.272 work, now included in adopted 0.210, adds that internal coordinator-owned vector
and actual-parent logical reservation, with retained historical release and once-
only protected retirement. Eleven controls/nine semantic mutants and seven adjacent
regressions pass; the exact registered validator, static boundary and 121 clean
entrypoints pass. Full 0.210 qualification, endpoint adoption and retained archive
satisfy that batch's acceptance gate. The vector alone does not supply mixed
ingress/per-participant checks or select U01's public retrieval decision.

The 0.209/model 0.273 work, now included in adopted 0.210, supplies the logical mixed ingress and
per-component current authority binding. It preserves a distinct explicit contract
from source-only F11, derives before/candidate native requests, captures original
allocations and private stamps once, and joins all source/annotation/view/history/
receipt effects through the actual coordinator. Sixteen controls/thirteen semantic
mutants, eight omissions, 26 permission cases and twelve adjacent regressions pass;
the exact registered predicate and 123-module boundary/clean-entrypoint checks pass.
Bounded review found no must-fix. Full 0.210 qualification, matching source/worker/
root endpoints and retained archive integrity pass. This accepts the selected
mixed input/execution binding and internal receipt vector, not live discovery/
enablement, N03 or U01. Standalone F13/F14 bindings have their separate acceptance below.

### Accepted literal replacement bindings

The existing F11/version-1 replace-current and replace-all logical bindings are
accepted by inspection of specification §§6.5.2.3/15.4 and the adopted 0.202
native declarations/gateway. F-identifiers and names remain design references,
not newly assigned public identifiers. Both typed and dynamic routes bind the
same original issued invocation and existing native handler:
`prepareInvocationReplacement` / `submitInvocationReplacement`, followed by the
common invocation service and `prepareReplacement` / `publishReplacement`.

| Variant | Exact request fields | Separate owned binding and eligibility |
| --- | --- | --- |
| replace-current | operation, index, replacement, invocation; operation equals the variant; index is a nonnegative integer | The original returned literal-search result by identity; the selected occurrence must exist and remain current/authorized. A known subset may supply that occurrence. |
| replace-all | The same four fields; operation equals the variant; index is null | The original returned complete-within-scope literal-search result by identity; the full selected set must remain current/authorized. A partial set cannot establish completeness. |

replacementInputForms is the immutable guard-owned declaration source. The
replacement string is literal scalar text, including empty text—not a template
or script. The original result remains a separate native role, never a fabricated
field, JSON clone, new query or interchangeable equal-result reference. Complete
captured context/limits, source revision, annotationSet, selected-match eligibility
and actual attached-view/annotation/history companions survive preparation,
replay and final publication. No UI focus or keymap grants source mutation.

Discoverability uses the existing generic accepted-registry cut and bounded
`{contract, variant, fields}` metadata. Bind the accepted handler's F11/version/
variant identity to those guard-owned fields, original-result role and current
requirements. The metadata's contract identity belongs to that accepted registry
binding; this audit invents no new public contract string. Literal metadata cannot
install a handler, validate a request, grant rights or replace native checks.
Actual accepted-registry/schema/handler truth remains a supplied implementation
premise. Builtin discovery profiles 1–13 are not asserted to include replacement;
a new builtin profile is not a requirement for this generic logical binding.
F01's separately tracked result-guard composition exception is not waived.

Immediate submit/service and retained local outcomes reuse the common invocation
relations and five qualified terminal pairs. No new operation-specific result
reference or U01 route is needed for these mutations. Existing authorized
publication projections can observe actual source/annotation/view changes;
complete cross-family F06/F19/event assembly remains N03. No external effect or
persistence outcome is introduced by replacement.

The qualified report supplies two forms, eight omissions/eight controls, 36 argument
boundaries and twelve argument mutations; three footprint cases/three mutations;
25 gateway controls, 108 permission cases, 24 capture boundaries, twelve gateway
mutations and six staged faults; plus 143 selected substrate wrappers with zero
baseline intercepted calls and three injected-call mutations. Canonical capture
and the actual returned reply frame remain separately funded; protected retirement
releases those holds once without retiring the original query or refunding issuance.
Current root inputs match that report. A separate bounded review confirms that
requiring a new fixed builtin descriptor was unnecessarily strong.

This accepts these two N01/N06 bindings, not full F11/L02, actual handler
installation, raw ingress/wire/ABI, physical accounting or independent refinement.
Accepted-paste and validated-cut-deletion remain unfinished, alongside
N03/U01/other-family work.

### Accepted execution observation bindings

F06's four existing logical bindings are accepted under specification
§§23.1.2/4–7 and adopted discovery /8, including the existing trusted host-driven
wait continuation. This does not select U01's operation-specific receipt route
or a recipient event transport. Observation reads the original invocation ledger; it never
executes, stops or retires that producer.

Every existing execution-observation/1 request has contract, operation, invocation,
context and profile. Typed/dynamic normalization preserves the original opaque
observer context, exact invocation label and copied complete authority frame.
Actual observer lookup, not a shaped context, binds the original producer ticket.

| Variant | Exact existing available product | Independent condition |
| --- | --- | --- |
| status | execution: status, phase, observation; optionally outcome only in terminal phase | Current ordinary release/control and actual retained outcome where included. Missing rich facts need not erase permitted phase observation. |
| retained-outcome | outcome: status, observation, outcome | Actual retained projected summary and current release; no operation-specific annotation/history/settings/lifecycle receipt is implied. |
| minimal-control-status | execution: status, phase, observation, never outcome | Independent protected control reservation/readiness and current control authority; ordinary rich capacity/issuance is unnecessary. |
| bounded-observation-wait | waiting: status, wait; private host continuation can return waiting, observed with ordinary execution result, or observation-ended | Original ordinary after-reference, positive admitted budget, actual waiter capacity/retention and valid declared clock/service. No control-reference promotion. |

All four also return status-only unavailable under their guard. Phases are the
existing pending/terminal sum. Nested outcome fields and bounds remain exactly
those in [the native result declaration](../formal/invocation-observation-results.mjs):
observation, publication, control, effect, reasons, participants, count, correlation
and recovery. These are scoped reference products, not newly selected public
explanation/wire codes. Recovery text is not retry authority; withheld or forgotten
facts are not known nonapplication, successful persistence or quiescence.

| L02 criterion | Accepted contract and scoped evidence |
| --- | --- |
| Arguments, result correlation and versions | Shared native declarations fix the four field sets and variant/profile meaning. Wait-start adds after/budget; other requests use the common fields. Original observer/reference roles remain opaque; operation-specific result guards run before reference installation and at completion. Foreign/copied references cannot borrow authority. |
| Authority and disclosure | Original invocation policy, captured observer ceiling and current recipient/epoch/grant intersect. Known loss is absorbing; unknown/mismatched frames refuse without lending loss to another context. Minimal control does not copy rich outcome payloads or widen ordinary wait authority. |
| Bounds and ownership | Explicit prepaid decoder support charges the actual shared envelopes, native context and eight input frames before opening. Reads use that paid workspace, with nonreentrant reservation ownership. Ordinary/protected issuance, reply/context/wait bounds and consumed work/fanout stay independent. Protected reads work with zero ordinary spare capacity and stopped new admission. |
| Outcomes and control | Immediate observation changes only derived observer/reference state. No authored event, external effect or second command terminal receipt is created. Observer stop/drain/retirement cannot settle the producer, refund its effects or grant replay. A producer's forgotten result cannot make its invocation fresh. |
| Fault and discharge | Staged result faults fence the observer and end waits without changing producer facts. Late installed references remain spent. Protected waiter drain/native context retirement precede shared refund; fault/unavailable alone cannot release the reservation. |
| Discovery and instrumentation | /8 describes the three exact queries and their bounded result products, with later profiles preserving meaning. IF-01/02 and INS-01/03/04/07/08/12/13 distinguish ordinary/protected release, native result construction, observer loss, pressure and actual discharge. No raw invocation/context/outcome payload is an ordinary metric label. |

Qualified 0.202 includes the argument corpus with sixteen route compositions,
122 refusals, 96 boundaries and 21 mutations; 519 native result products with
29 result mutations/eight fault stages; and the prepaid decoder's 23 compositions,
29 selected refusals, eighteen capacity/profile boundaries, 23 mutations, twelve
fixed faults and 24 reentries. Native observation/shared admission regressions and
handler-bound discovery /8 also pass. These are finite reference checks and
conditional ownership arguments, not real clock/policy/physical/host truth.

The existing continuation is a trusted in-process host facet, not another
recipient command. Pooled admission returns the original native context and shared
reservation after funding native reply/wait/reference maxima. The host retains
that pair through actual waiter drain and native context retirement. The exact
existing continuation parameters are context, wait, event, complete authority
context, current policy and trusted facts. Original context/wait lookup and the
closed event sum bind every transition; a copied handle supplies no ownership.

| Host event | Existing transition and authority boundary |
| --- | --- |
| poll | Revalidate ordinary current disclosure and original producer identity. Return a permitted change/terminal result or remain waiting; polling does not advance time or run the producer. |
| tick | Require valid declared clock evidence; decrement the original finite rank or end observation. A tick is trusted host service, not a recipient's assertion of elapsed time. |
| expire | Require owner control; end observer waiting only. The host's expiry scheduling/clock truth is a premise, not inferred from the event name. |
| stop | Require owner control; end this waiter without producer cancellation, effect settlement or obligation refund. |

Continuation uses the already funded native bounds directly. It does not acquire
the read/wait-start decoder's busy guard or copy another decoder workspace. The
existing serialized, inert, callback-free host seam prevents competing retirement
inside a native transition; production synchronization remains E01. Terminal or
invalidated waiter eligibility cannot revive. Ended records and paid shared
capacity remain held until protected waiter drain/native retirement precede
refund; consumed work/fanout and producer obligations stay unchanged. The shared
admission corpus already exercises native continuation on pooled contexts and
blocks premature refund. Separate bounded review verified that connection and
the inspected source hashes against the qualified report.

This closes the existing logical host-driven wait binding without adding a poll
verb, acquisition codec or event transport. Actual monotonic clock/service/suspend
truth remains E02/E03; a separately advertised recipient-facing continuation
protocol would need its own contract rather than borrowing host control authority.

U01 still decides retrieval of operation-specific retained receipts. The accepted
summary query supplies no operation-specific receipt access or result/capability-
reference minting. Its existing observation references still advance when the
permitted phase/outcome projection changes. Cross-family
outcome/event assembly, complete F19 control, L02 as a whole and E01–E05 remain
open. No new behavior, default or proof program is added by this disposition.

### Accepted annotation reversal and mixed authoring bindings

Adopted 0.210/model 0.274 accepts N01/N06 input/execution bindings for the four
standalone F13 operations, F14 undo/redo with their existing group/scope-frontier
selectors, and atomic mixed source/annotation authoring through F11 exact-batch.
The internal component receipt vector is accepted at its native retained-release
scope; this does not choose a public retrieval route.

| Product | Binding and preserved contract |
| --- | --- |
| Standalone annotation-authoring/1 create/update-body/delete/reattach | Exact bounded public data selects the existing native operation. Separate host annotation roles supply actual owner/profile/dependencies/limits and original create allocation once. Source-write authority is not required or substituted for annotation rights; body/target versions and both point profiles remain independent. |
| Standalone history-reversal/1 undo/redo | Preparation fixes the original group or scope-frontier selection and native history capsule. Service cannot refresh the frontier, change participants or replay external effects. Current history authority/dependencies and the common publication path remain required. |
| atomic-source-annotation/1 with bounded-atomic-authoring/1 | Original source/read/scope intent and ordered F13 components bind before/candidate target meaning, private companion stamps and separate per-component authority. The private source candidate is not an intermediate publication. Source, annotations, current views, linked history and original receipts install through one coordinator decision or not at all under its reference premises. |
| Internal compound receipt vector under §13.9.6 | Actual parent funding, original plan/component correspondence and individual capture ceilings govern staging. Historical release uses the original component index and current grants, not newer rows. Forgetting clears results without refreshing eligibility; separate protected retirement refunds the hold once. |

The [standalone binding](../formal/native-local-command-binding.md) uses the actual
shared typed/dynamic coordinator, retained native capsule and role-specific current
checks. Its complete finite native profile and actual remaining parent headroom
are checked before construction, including larger native ceilings without rejecting
a valid small intent solely for that difference. The [mixed binding](../formal/atomic-authoring.md)
adds actual per-component and final aggregate limits; the
[receipt owner](../formal/atomic-annotation-results.md) does not create a separate
independent budget or accept a caller-supplied second receipt-target list.

The qualified batch includes twelve standalone controls/fourteen semantic mutants,
sixteen mixed-authoring controls/thirteen mutants and eleven internal receipt
controls/nine mutants, with their exact malformed/authority/boundary cases and
adjacent regressions. Bounded review found no must-fix before qualification.
All 768 registered checks pass over 970 matching source/worker/root inputs; the
byte-exact archive's read-only integrity verifier also passes. This is scoped
reference evidence, not physical/concurrent/crash refinement or independent
Cleanroom certification.

Generic accepted-registry descriptions remain descriptive declarations of these
logical handlers, not an extra builtin discovery version or proof of live
installed enablement. N02/U01 must still bind standalone and component receipts to
public retrieval without widening native capture ceilings or minting references
at query time. Complete N03/F19 assembly and other L02 families remain open.
Existing IF-01/03/08 and INS-01/02/03/06/07/09/13 retain whole-operation admission,
publication, history/mapping, receipt pressure/release and protected discharge;
components do not multiply useful-success counts or become raw telemetry labels.

### Accepted settings and registration input execution bindings

F16's eleven selected native forms have sufficient adopted 0.207 evidence for
their N01/N06 logical input/execution bindings. This acceptance reuses their
existing entry points; it does not add a uniform request wrapper, mandatory
transport or public Rust spelling.

| Variant and contracts | Original authority and execution binding |
| --- | --- |
| semantic-settings-activation/1 and scoped-semantic-settings/1 | Ordered set/remove/reset intent uses the owned settings job. Scoped writes retain the original separate write bases and current first-party lowering; F07 read selectors cannot substitute. See [settings](../formal/engine-settings-state.md) and [scoped writes](../formal/scoped-settings.md). |
| engine-settings-layers/1, scoped-engine-settings-layers/1 and basis-engine-settings-layers/1 | Layer creation/removal/reassociation preserves incarnation issuance, registry rights and retained interpretation. Scoped lifecycle and per-layer bases remain distinct original handles. See [layer lifecycle](../formal/engine-settings-layers.md), [scoped lifecycle](../formal/scoped-lifecycle.md) and [basis composition](../formal/basis-lifecycle.md). |
| source-serialization/1 and scoped-source-serialization/1 | Intrinsic document output metadata uses its own target/generation or scoped basis, without changing source/history or performing I/O. See [serialization](../formal/source-serialization.md) and [scoped serialization](../formal/scoped-serialization.md). |
| engine-settings-definition/1 and scoped-engine-settings-definition/1 | Supported-registry definition/restriction/association changes use the settings owner without replacing authored layers or inferring registry rights from a layer grant. See [definitions](../formal/engine-settings-definitions.md) and [scoped definitions](../formal/scoped-definition.md). |
| registration-invocation/1 and scoped-registration-invocation/1 | Supported-registry resource metadata uses the existing trusted native adapter, registration job and current certificate/floor commit. The scoped adapter retains its original issuer, scope and basis array. See [native registration](../formal/registration-invocations.md) and [scoped registration](../formal/registration-scoped-invocations.md). |

Settings and serialization consume their existing exact typed/dynamic tuples.
Registration uses its existing guarded native/scoped request adapters and common
typed/dynamic invocation submission, not a newly selected decoded envelope.
Original owner/domain/ticket/context/profile and opaque bases bind the plan.
Current service rechecks mandatory plus declared dependencies and affected rights,
merges against the current root and publishes through the common terminal path.
Matching submission observes the same job; changed intent or replacement bases
cannot refresh it. Validated no-ops, pending cancellation, logical parent bounds,
protected retirement and nonrefundable issuance retain their existing contracts.

The handler-bound discovery /2 producer describes all eleven contracts from their
actual private guards. Independent literal field expectations and a standard
Draft 2020-12 validator check eleven valid products, 74 omissions, eleven extra-
field cases and nine runtime-context counterexamples: 105 cases. Seven declaration
mutations challenge the source/descriptor binding. This is outer-schema evidence,
not complete nested semantic validation or authorization; the actual native
handlers supply those checks under their finite normalized-input premises.
The adopted full suite includes these discovery and native mutation checks.
Fresh focused reruns of schema discovery, native settings and scoped settings
also pass. Native settings has 24 controls and seventeen mutations; scoped
settings has 21 controls and seven mutations.
Second-agent readback found no additional logical binding blocker at this scope.

This does not close full F16. Compatible analysis/provider-context activation
remains open. Adopted 0.219 binds the distinct host-registration, scoped-
registration and created-settings results through the selected N02 retained-
outcome facet; full qualification/adoption passes. The stateless result-shape
checker alone is not installed production validation. Complete
F19/event assembly and installed all-family dispatch remain separate. UI keymaps,
host applicability/policy truth, hostile ingress, physical/concurrent/crash
refinement and E01–E05 are not established by these logical bindings.
Existing INS-01/02/03/08/09/12/13 retain admission, publication, release,
configuration, pressure and protected-discharge attribution; acceptance adds no
telemetry stream or content-bearing labels.

### Native load and ordinary save request bindings

F20/F21's four existing native request/admitted-task execution products are
sufficient at their adopted design/reference scope. This acceptance reuses the
qualified 0.211 contracts, not a new uniform envelope, public identifier, transport
or Rust API. Complete N01/N06 includes the original-invocation/retry relation
below and remains open for these variants.

| Product | Exact existing boundary and authority |
| --- | --- |
| source-load-candidate | [resource-read/1](../formal/resource-read.json) declares the request, origin, provider/resource, registration generation, profile and requested read guarantee. The native source reader owns its actual domain and obtains the current root at admission/submission. Its returned task belongs to that reader; caller-supplied replacement roots and truthy native grants cannot substitute. |
| annotation-load-candidate | [annotation-resource-read/1](../formal/annotation-resource-read.json) additionally binds the existing document, source/set, ordinary-pair and annotation-binding generations. The [native reader](../formal/annotation-native-load.mjs) confines its raw owner and obtains the actual domain root. Current target/binding/registration checks precede submission and candidate release. |
| captured-source-save | The native ordinary-admit adapter consumes the existing [source-save/1](../formal/source-save.json) request. Actual domain/writer, codec artifact, current ordinary binding and retained read/write baseline provenance bind its final ticket issuance. Admission does not publish an authored root or create a copy-export destination. |
| captured-annotation-save | [Ordinary annotation admission](../formal/ordinary-annotation-save.md) consumes the closed annotation-save/1 ordinary request. Actual domain, attached baseline pool, codec preparation, annotation binding, writer ticket and retained claim commit together under the serialized-reference premises. It is neither source permission nor export authority. |

Known original tasks already support native submission/dispatch, inspection,
evidence attribution, cancellation and protected retirement under their respective
contracts. Immutable captures do not refresh when source, annotations or bindings
later change. Requested read coherence, write acknowledgment, current candidate
applicability, baseline adoption and producer quiescence remain independent facts.
Initial F17 activation is a separate authorized command, not a read completion.

All four admissions retain a consumed-origin set. Reusing an origin refuses;
it does not compare the original request and return its existing task. They do
not themselves bind origin.operation to an issued, scope-bound original invocation.
The
remaining facade must preserve the original invocation, immutable request and
issuer/task relation across a matching retry; a changed attempt label is not
recovery of the original request. No task-lookup, safe replay or external
deduplication guarantee is inferred from this narrow acceptance.

Requirements §§3.3/4.1 and specification §6.5 require cohesive behavior, equivalent
routes and bounded at-most-once intent, not one homogeneous issuer, dispatcher or
execution machine for local publication and provider effects. The accepted F26
diagnostic-drain owner is an existing separate-issuer precedent. A supported
per-profile facade may retain its actual native owner/task and original scope;
it must still establish issued identity, immutable-intent comparison, serialized
first admission, current authorized matching-result release, protected control
and non-reusable retirement. Origin strings or a common unavailable response do
not prove those properties. This removes an unnecessary implementation constraint,
not the remaining binding/retry gap, and selects no new public identifier format.

Separate staged 0.221 §16.1.3 implements native source-load effect-attempt recovery
through its actual native reader and typed/dynamic catalog routes. Matching
requests recover the original task without new admission/submission; current
scope/release and spent retirement remain enforced. Nine controls/ten targeted
mutations, exact registration and selected regressions pass; read-only challenge's
numeric-operand finding is repaired. A reproduced lifetime-origin funding gap is
also fixed by a separate safe parent reservation, with conditional SMT induction
and its omission counterexample. Neither the adopted 0.219 nor running 0.220
inputs include this fix. Full qualification/adoption, annotation-load/save retry
bindings, discovery and effect/receipt/event assembly remain; no all-F20/F21
acceptance follows from this staged native relation.

The same staged candidate's §16.1.4 adds annotation-load's counterpart: original
task lookup survives a real source edit without reviving candidate applicability.
Eight controls/ten guard mutations, both original annotation regressions and
bounded read-only review pass. Its analogous spent-origin underfunding is
reproduced (898,432 retained key bytes versus a 760,000-byte task envelope) and
fixed with the same validated parent bound. Both original-request load bindings
are now implemented in the candidate, not qualified/adopted. Parent-operation
ownership/admission, parent-command deduplication and authorized retry composition
remain open for both adapters: changing an attempt label may admit fresh work
under native gates. Neither adapter resolves origin.operation to an actual owned
parent in the earlier attempt-only implementation. The updated candidate now
atomically admits a parent-operation record with the original issuer/context,
immutable request and actual child. One attempt per parent is explicit; changed
effect/attempt labels and raw bypass refuse, old raw work cannot be retroactively
adopted, and guarded child retirement spends the co-owned parent. Source thirteen
controls/sixteen mutations and annotation twelve/sixteen pass. Both load owners
now bound and privately capture request/context before current-fact reads;
catalog routing reads no permission supplier. Reproduced supplier mutation of
caller input no longer retargets fresh/matching typed/dynamic/native calls.
The exact registered validators pass. Earlier bounded review found no concrete
defect in the preceding parent relation, not this later repair. Independent parent records and
origin keys have separate safe lifetime reservations. Qualification/adoption and
cross-issuer identity remain; this adds no global dispatcher or automatic retry.
The save parent counterparts are implemented below; discovery,
downstream effect/evidence/receipt/event relations and complete L02 remain open.

Staged §16.1.5 now implements both ordinary-save parent counterparts through the
actual shared writer. Parent identity binds its original domain/context,
immutable request and child; annotation intent also retains its full validated
artifact context and identity/source-association binding. Raw bypass/adoption,
attempt relabeling, recapture and pool/profile migration refuse. Guarded child
retirement spends the co-owned parent; actual annotation baseline claims must
discharge separately. Bounded private input/scope capture precedes current-fact
reads. Eleven controls/fifteen coherent-graph mutations and bounded
review pass, as do the original writer and native save regressions. An independent
lifetime parent-store envelope is funded once by the shared sponsor. Exact
registration, boundary checks and qualification govern acceptance. Admission or
matching lookup is not dispatch, persistence or baseline adoption; no receipt
kind, network protocol or UI/substrate workflow is introduced. All four selected
load/save parent relations are implemented, not yet qualified/adopted; downstream
discovery/receipt/event/control assembly and whole L02 remain open.

Current Rust ordinary-save ownership refinement: both existing parent owners now
stage their parent before child admission, with exact native capture/task matching.
Artifact/charge, task/linkage, optional own-chain credit and parent installation
share a fault fence. Interrupted installation preserves installed obligations and
fences ordinary execution; it does not return an admission success or attest
external nonapplication. Native child admission and parent admission/recovery have
optional bounded synchronous timing. These changes reuse the current one-attempt
writer parents; the installed route described next does not complete all F21
effect/event assembly or whole-engine acceptance.
The installed Rust F21 source now routes both captured-save variants on scoped
discovery /1 to those original owners, with exact private origin/family/effect-
request kind checks and independent current native admission/release. Optional
catalog bindings are retained; matching recovery after edits/withdrawal does not
recapture or issue a second task. The returned original child joins existing
native-operation observation for separately authorized evidence/status/control.
This closes that selected discovery-to-owner-to-result route, not external
dispatch/persistence/baseline adoption or the remaining whole-engine gates.

Staged §23.1.11 now supplies the normalized native observation relation for all
four owned parent/task kinds: F06 status, typed retained-evidence facet, minimal
control and bounded waits. It uses separately versioned native-operation contracts,
not a fabricated publication outcome or an extension of execution-receipt/1.
Original actual owner/task/context, captured/current grants, private input,
prepaid parent funding, protected progress and observation/producer lifetimes
are checked. Read rows/chunk-conflict bytes and export/baseline workflow state
are excluded. Seventeen controls/fifteen facade mutations, 24 cross-route
deliveries, exact registration and static/cold boundaries pass; read-only challenge
found no concrete defect in this selected relation and did not execute tests.
Full qualification, accepted-registry schemas/decoder, full F19 event/context
and F18 shutdown assembly remain. This closes the selected normalized result/
control/wait binding in working source, not N03 or whole L02 acceptance.

Successor 0.222 §23.1.12 now implements the selected accepted-registry/decoded
relation for these four native observation kinds. An explicit versioned profile
uses the existing pool/eight frames, actual domain binding, opaque roles and
native retirement before refund; older profiles do not gain native support.
Two descriptive records supply four closed argument/result schemas and private
wait continuations without granting authority or creating an event transport.
The handoff's reproduced opaque-progress sizing traversal is repaired in this
successor, not in frozen 0.221. Eleven controls/twelve coherent mutations, 24
route deliveries, 45 standard-schema cases, exact registration, legacy decoder/
pool/receipt regressions and static/cold boundaries pass. Bounded read-only
challenge found no concrete defect. Full qualification/adoption, full F19/F18
and operational/independent gates remain; no whole N03/L02 closure is claimed.

The adopted report contains passing native source-read, native annotation-load,
native ordinary-source-save and ordinary-annotation-save checks against the exact
current inputs. Fresh shortened reference runs pass 27, 14, 30 and 19 controls,
respectively; these reruns do not replace the qualified full mutation suites.
Separate read-only code review confirms the native bindings and the specific
retry gap. N03's complete effect/outcome/event relations, F18 shutdown, U01's
operation-result retrieval, actual host/provider truth and E01–E05 remain open.
Existing capture/admission/dispatch/evidence/release instrumentation and protected
resource contracts remain in force; this acceptance adds no telemetry path.

### Accepted diagnostic drain binding

F26/authorized-diagnostic-drain is accepted for N01/N04/N06 at its existing
owned-diagnostic-drain/1 logical facade scope. Specification §23.2.2 and
[the native gateway](../formal/diagnostic-drain.md) already supply the complete
closed request, original queue/domain/scope binding, issued opaque task, matching
immutable retry intent, at-most-once fixed local service, retained terminal fact,
separate ordinary-result/protected-control release and actual parent-hold discharge.
Both typed and dynamic requests select the exact F26 variant through the existing
catalog router. No additional envelope or operation-specific result schema is needed.

The gateway's native result and control facets are already defined;
U01's generic operation-receipt route is not needed to establish them. Their
public spelling/encoding and installed dispatch remain implementation obligations.
Draining optional diagnostic records does not publish authored state and needs no
fabricated source publication or change event. Performed, notPerformed and unknown
retain their declared meanings; an absent batch is not nonperformance.

The qualified report includes the sixteen-control gateway suite, forty coherent
semantic mutants, 84 typed/dynamic drains and 1,555 traces/5,910 edges, plus the
actual queue/hold suite and eight BASE-DRAIN conditions. A fresh shortened run
passes all sixteen controls and 43 traces/78 edges through horizon two; it does
not replace the qualified horizon-four or mutation evidence. Separate read-only
review found no missing logical binding. Existing MELT/golden/RED/USE and privacy/
security attribution remains IF-01/08/09, PATH-11 and INS-01/03/04/12/13.

This does not accept timers, general task cancellation, other F26 facilities,
export delivery, durability or audit completeness. Host grants, clock/scheduling,
installed instrumentation, physical/global accounting, concurrent/crash refinement
and independent assurance remain their named gates. No full F26/L02 deliverable
closes from this variant acceptance.

### Accepted F17 request and native execution bindings

The existing five lifecycle forms and nine activation/rebind/container forms
together cover every F17 variant. The first path covers scratch creation,
attach/detach, membership and disposal. The second covers initial source and
annotation activation, annotation-baseline adoption, source SaveAs/pure rebind,
and view/workspace container create/retire. Release-binding and release-provenance
cleanup operations do not become additional public variants.

The [lifecycle envelope](../formal/lifecycle-envelope.md) and
[activation envelope](../formal/lifecycle-activation-envelope.md) bind both routes
to the original actual domain/ticket, captured semantic request and native
authority/profile ports. Existing submit/run relations compare retained intent,
publish through the common coordinator and preserve protected retirement. The
[workspace contract](../formal/lifecycle-workspace.md) supplies actual-parent
temporary funding for all fourteen forms; earlier local-ceiling exclusions do
not override it. Native nested validation remains mandatory after bounded capture.

[Discovery /10](../formal/lifecycle-discovery.md) describes all fourteen forms
and eight variants without substituting shape/advice for native authority.
These compositions and their existing fault, lifetime, route and resource checks
pass in the adopted 0.218 full report. This closes N01/N06's request/native-binding
gap for these supported profiles by reusing qualified contracts, without a new
decoder, model, public verb or executor.

N02 receipt delivery is implemented in the unqualified working draft below;
N03 outcome/event assembly remains open. No in-place reload, additional
profile, provider/platform truth or production refinement is inferred.

### N02 Bind operation result retrieval

Current Rust increment (2026-11-06): `query_decoded_invocation_receipt_v10`
connects borrowed typed/dynamic F06 `execution-receipt/1` input to the existing
native V10 retained-result release. It validates the exact tuple and six fields,
original opaque invocation/context, admitted profile and closed facet/index before
reading current release facts. The native trusted release request stays separate
from recipient data. Its exhaustive mapping reuses the existing V1–V10 producers;
it does not widen their enums or create/replay a command. Native lowering uses the
already issued `InvocationId`, not a new string-label registry. This borrowed Rust
edge is not the reference model's owned/copying decoder or a wire deserializer.
Public integration tests exercise retained-release delivery, repeat query,
malformed/foreign refusal before loss, attributed loss and optional/saturated
instrumentation. The four ordinary/protected/wait forms now also have borrowed
native typed/dynamic ingress and sealed installed discovery. A second installed
F06 entry exposes the separate receipt contract; both sources keep metadata
release and current native authority independent. Generic copies cannot acquire
the private handler origin. Direct protected reads remain available independently
of ordinary discovery readiness. These selected F06 connections are implemented;
this is not whole-C1/C2, whole-family result qualification or whole-facade closure.

The user selected U01's separately bounded, typed, permission-filtered facet of
the existing retained-outcome query. The route decision is resolved. Adopted
specification 0.219 §23.1.10 and the existing observation owner bind the selected
execution-receipt/1 contract to eight original native receipt kinds. Both routes
use the existing prepaid decoder. Explicit captured facet ceilings, current
ordinary and native-specific grants, original opaque roles, complete reply bounds,
receipt-local fault quarantine and protected control remain distinct. Handler-owned
accepted-registry metadata describes this contract without widening builtin profiles.
Focused composition, eight targeted mutations and the existing argument/observation/
decoder regressions pass. The wrong-original native fault remains structurally valid
and is independently refused by the facade; summary and protected control survive.
Read-only second-agent challenge found no reachable safety defect at this selected
scope. All 794 checks over 1,005 unchanged inputs, source/worker/root endpoint
audits and retained archive integrity checks pass. N02 is accepted for these
eight native kinds, not unspecified future results or the complete facade.
Original-command/replay return
is not selected as an alternative receipt protocol. Receipt observation never
executes or replays the command.

The selected route must preserve original issued references, native current
release and capture ceilings, reserved combined-reply capacity and protected
retirement. It must not mint result references at query time, JSON-clone opaque
references, infer receipt permission from a generic details flag or couple
minimal-control availability to optional rich result buffers. F13/F14/F16/F17
native receipt existence alone would not establish this facade binding.

Working successor 0.224 §12.3.6 now specifies the remaining registry-baseline
receipt relation: a distinct opt-in F06 contract, original command/result binding,
separate receipt-context lifetime, funded historical entry-to-slot associations,
exact current authorization including removed slots, and all-or-unavailable
delivery of the original complete baseline. Neither command replay nor current
catalog visibility substitutes for that authorization. Existing receipt profiles
are not widened. This is a design contract awaiting native/decoder/discovery and
retention/funding composition, not an accepted or implemented binding. The frozen
0.223 run does not contain or qualify this successor contract.

That successor now supplies the native receipt ports within the existing registry:
funded original historical associations, actual receipt-context command claims,
bounded exact-slot/current-root certificates, all-or-unavailable original delivery
and protected context stop/retirement. Eight control groups/ten targeted unsafe
variants and the exact registered predicate pass; the legacy registry suite and
existing stop composition also pass. Read-only challenge found no concrete defect
in those selected ports. The successor now also binds the selected public F06
receipt on both routes, revalidates original native content/opaque roles, and
quarantines invalid replies without changing command settlement. Two handler-owned
accepted-registry descriptors cover the two command forms and receipt; twenty-five
standard schema cases and actual authorized discovery pass without widening old
profiles or granting authority from metadata. Receipt-context stop routing extends
the existing dispatcher to twelve native classes, with twenty-four deliveries per
route, 180 bound/62 local refusals and all nine existing mutations detected. The
exact integrated predicates, ownership boundary and isolated imports pass; bounded
second-agent read-only review found no concrete defect. Full frozen qualification/
adoption, explicit F19 grant revocation, complete facade/shutdown and operational
gates remain. This closes the selected binding at focused scope, not whole N02/L02
or physical/policy truth or independent certification.

### N03 Assemble command outcome and event relations

Bind each supported command/effect to its admissible immediate, retained and
observed products. Keep local publication, external acceptance/persistence,
historical evidence, current applicability, recipient disclosure and quiescence
as separate facts. A command need not generate an event merely because it has a
result; a query need not gain a retained command outcome. Record not-applicable
facets explicitly.

Existing publication and registration observers supply concrete projections,
gap/resynchronization and release rules. Assemble their supported context union
and effect-specific relations rather than replacing them with one lossy global
status enum. Notifications stay downstream of publication and cannot roll it back.

Reuse the local invocation's ownership/capture/retry/release pattern, not its
publication-specific lifecycle, for the remaining effects. The actual
runInvocation path settles a local plan and root outcome; its clipboard branch
consumes an already acknowledged effect for a local continuation. It does not
admit a fresh general host request or settle copy-only completion. A nonpublishing
effect must not acquire a synthetic root commit or use notApplied to mean that
no local publication was requested. Bind the original operation to the actual
native task and preserve that task's effect knowledge, applicability and release
rules through the facade's closed profile-specific products.

Neovim's job-control contract assigns job identities and keeps stream closure
separate from job termination. This supports distinct owned lifetimes, not
Vellum's stronger authorization, deduplication, quiescence or persistence claims.
[Neovim job control](https://neovim.io/doc/user/job_control/)

The original working [lifecycle composition reference](lifecycle-outcome-event-reference.mjs)
compares actual native execution, retained receipts and recipient projections for
all fourteen existing F17 forms on both typed and dynamic routes. Its 28 success
cases correlate the receipt's original invocation, operation, status and target
with the admitted request and committed root. Publication does not imply a change
to the selected source projection: disposal produces a gap; the other selected
fixtures leave that projection unchanged. Twenty-eight pending-stop cases preserve
the original root and settle without a receipt. Fourteen disclosure cases keep
writer publication intact when receipt or observer access is lost.

Two additional annotation-restoration cases select a named annotation before
execution and deliver its exact committed body/version and target placement/
generation, even when command-receipt disclosure is independently withheld. Six
controlled unsafe composition alternatives are rejected, including foreign
invocation, wrong operation, wrong target and wrong status receipts. These are
working composition checks, not production mutation coverage or adopted full-suite
evidence. They close the receipt-correlation review finding and establish these
selected relations; complete F19 context/control assembly and other-family
relations remain open. U01's selected public receipt binding is implemented in
adopted specification 0.219; its selected qualification is recorded under N02.

Staged 0.220 reuses that composition as lifecycle-outcome-event-checks.mjs in
the formal inventory. It now calls the adopted actual-domain observer wrapper,
not the raw supplied-root helper, and compares every available receipt through
the selected retained-outcome facet with its original native result. Both
observation owners are admitted and funded before command execution; querying
does not acquire contexts. The same four controls and six unsafe alternatives
pass, with all original fourteen-form/two-route cases retained. Its exact harness
registration passes. Read-only delta review found no concrete defect and confirmed
the selected scope; it did not execute tests or provide independent certification.
Full qualification/adoption remains pending; this is selected
F17 outcome/receipt/projection correspondence, not complete F19 or all-family L02.

#### Clipboard request and continuation lineage

The [native lineage reference](clipboard-lineage-reference.mjs) passes five
controls against adopted 0.210. For paste and cut it follows the original issued
invocation through native task admission, dispatch, attributed acknowledgment,
owned continuation, shared publication and matching typed/dynamic local retries.
Each trace has one task issuance, one local start and one publication. Rejected
foreign task/origin/capture evidence leaves the original task uncontested. This
is native composition evidence, not a public clipboard facade or provider-truth
proof; the working reference is outside the frozen qualification inventory.

Three remaining bindings are now explicit:

| Boundary | Existing behavior | Required facade integration |
| --- | --- | --- |
| Original request retry | Re-admitting the same native origin refuses; it does not retrieve the original task. | Bind matching retries to the original owned request/task, with exact semantic comparison and current permitted disclosure. Do not report a duplicate refusal as a new execution failure. |
| Attempt identity | A different attempt can create another native task even with the same invocation label. | Do not mint a new attempt as an implementation of matching retry. A separately authorized new attempt is not the same logical request. |
| Copy completion | Attributed export acceptance does not edit source, prepare a local editing continuation or terminalize the shared invocation. Provider quiescence remains independently required for retirement. | Bind copy's immediate/retained external-effect outcome without pretending there was a source publication or borrowing cut/paste's editing continuation. |

The native duplicate-admission refusal is not a defect: task creation and facade
retry lookup are different responsibilities. F11's two clipboard continuations,
F22/F23 request binding and N03 therefore remain open at precisely these seams;
the existing once-only local continuation need not be rebuilt. No new public
verb, provider retry policy, receipt-retrieval choice or UI interaction is selected.

The [clipboard integration boundary](reference-ui-design.md#clipboard-integration-boundary)
keeps UI gestures, workflow orchestration and native clipboard mechanisms outside
the semantic kernel. The behavior facade still has to preserve its promised
capture/edit/effect contracts. An integration binding must use the actual engine
issuer and original task, not merely equal authority/invocation strings; moving
that binding to an edge does not waive it. The proposed clipboard registry inside
the invocation owner is unadopted and held for reconciliation. No public routing
row is removed and no whole-engine closure follows from this ownership clarification.
The user selected moving workflow verbs to the headless integration facade.
Adopted 0.213/model 0.277 moves F22/F23 and F11's accepted-paste/validated-cut
aliases to that owner, retaining all required transfer behavior. The resulting
catalog has 71 engine, five integration and eleven presentation variants. It is
qualified and adopted after 770 checks over 977 inputs and matching endpoint/
archive audits; the matrices above use the current three-owner catalog.
Original task/retry/effect binding remains required integration work, not UI work
or evidence that the semantic kernel needs a clipboard-specific registry.
Those five integration variants do not gate L02's 71-engine-variant closure;
the engine's authorized capture, exact editing and generic effect/evidence
contracts remain required in their own engine rows. Integration conformance
remains separately tracked, not waived or counted as completed engine work.

### N04 Complete the control facade

U06's target scope was selected on 2026-11-06: one exact original local context,
with independent current revocation authority. See [the selected contract](observation-revocation-decision.md).
The direct Rust bound native transition and closed typed/dynamic request ingress
and installed discovery/dispatch are implemented; complete untyped attachment
resolution remains open. Stop permission alone does not authorize revocation. External policy and
grant-wide withdrawal remain outside this selected context-local operation.

Bind cancellation, supported shutdown dispositions, observer stop/revoke/resync
and supported task cancellation to exact targets, current control authority,
protected capacity and retained obligations. The current cancellation scope flags
explicitly leave public control schema and external shutdown unproved. Distinguish
missing semantic shutdown behavior from real scheduler/provider service evidence:
the former needs design closure; the latter remains E02/E03/E05.

Stopping an observer does not stop its producer. Requesting cancellation does not
settle work, prove quiescence or refund all holds. Unknown obligations remain
owned even when a rich result is unavailable.

### Accepted protected read context stop bindings

F19 stop-observation has an accepted native read-context/1 product with exactly
contract, operation and context. This N01/N04/N06 disposition reuses the adopted
0.207 native owner bindings for source, annotation/combined and coherent version
sets, including their retained-copy profiles. It does not add a decoder or a new
control verb.

| Owning context | Existing native binding and control effect |
| --- | --- |
| Single source capture or retained source copy | stopInvocationSourceContext resolves the original domain/pool and exact issued context, checks the captured scope and closes further delivery. Existing internal holds remain separately owned. See [native source contexts](../formal/native-source-contexts.md). |
| Annotation-only or combined source/annotation capture or retained copy | stopInvocationAnnotationContext uses that context's original issuer/scope. All dependent record/continuation delivery closes; source access through the combined context cannot outlive it. See [native annotation contexts](../formal/native-annotation-contexts.md). |
| Coherent version-set parent or one member context, current or retained | stopInvocationVersionSetContext resolves the parent/member role to the original owning parent and invalidates the whole set. Record/continuation aliases are not control targets. See [native coherent sets](../formal/native-version-set.md) and [retained sets](../formal/native-retained-version-set.md). |

Each binding requires actual owner/domain/pool/context correspondence, matching
captured scope and strict current ownerControlAllowed. Token possession, copied
identity, a read grant or one peer's permission cannot supply that authority.
No content-release grant, ordinary-work admission, producer invocation or retained
command receipt is needed to stop the context. This acceptance is for the existing
normalized native adapters; it does not infer a public cross-issuer lookup or
trial-dispatch algorithm from similarly shaped handles.

Stop prevents later handoff through that context. It does not retract transferred
bytes, change source/history, cancel producer work, settle an external effect or
prove quiescence. Repeated stop has idempotent state effect while the context
exists; retirement may make later requests unavailable. Regrant does not reset
the stopped identity. Fresh acquisition creates its own bounded context; stopping
one retained copy does not close an independently acquired copy. A coherent
member, by contrast, is a facet of its parent and cannot be detached by stop.

Native source controls exercise stop with an outstanding root hold, ordinary
capacity exhaustion, faulted-owner cleanup and nonrefundable issuance. The native
annotation registry's bounded FSM compares stop/loss/query/retirement sequences
with actual root and parent accounting. Coherent query/lifetime controls reject
record aliases as control selectors, close the whole parent through a member and
retain actual hold/discharge obligations. These checks belong to adopted 0.207;
fresh source, annotation-query and coherent-query reruns pass. Second-agent
readback found no missing logical binding requirement at this scope. Finite
traces do not establish arbitrary concurrent handoff or physical reclamation.

Five additional existing protected-stop products are accepted at native logical
binding scope, using the qualified 0.212 evidence rather than new wrappers:

| Owning context | Existing native binding and control effect |
| --- | --- |
| History inspection context | closeHistoryInspection binds the original context and recipient. Its actual owner retains the captured scope; the caller does not resupply the whole scope. stopHistoryInspection instead stops the whole inspection owner. |
| Settings read | stopNativeSettingsRead and stopInvocationSettingsRead bind the actual domain, pool, issued context and captured scope. Faulted-owner cleanup and separate retirement remain available. |
| Discovery context | stopNativeDiscovery binds the original owner/context and scope, invalidating its selectors without refreshing issuance. |
| Resource-catalog read | stopRegistrationRead binds the original context/scope and closes aliases. Preparation, context and pool discharge remain separate. |
| Registration stream | stopRegistrationObservation binds the original stream/scope and makes stop absorbing. Outstanding preparations remain retained until protected retirement. |

All five require strict current ownerControlAllowed; none relies on renewed
ordinary delivery permission. Fresh focused runs pass fourteen history, twenty-two
settings, twenty-three discovery, twenty-eight resource-read and twenty-five
registration-event controls. Separate read-only review found no missing native
logical stop relation under the trusted-control-fact premises. These are not
proofs of physical release, producer cancellation or heterogeneous facade dispatch.

Registration establishment and authorized resynchronization also already bind
the actual domain/root, original scope, issued preparation/head and bounded stream
or generation. Reused preparation packets refuse; a new preparation can allocate
a fresh stream or resynchronization generation. F19 is a local-command family:
this behavior does not provide original-command matching-result lookup. The
original invocation must still resolve to its existing stream/baseline result
without replaying allocation or clearing the queue again. Fresh query acquisition
semantics cannot substitute for that N06 command-retry obligation.

Heterogeneous F19 facade dispatch, unsupported context classes, the establishment/
resynchronization matching-command lookup, revoke-observation-grant and N03 remain open. Revocation cannot
be inferred to mutate external host policy or erase retained producer evidence.
Existing INS-03/08/12/13 distinguish stop, refused delivery, retained holds and
actual retirement; no new diagnostic payload or metric identity is introduced.

Staged 0.223 §12.3.4 now supplies the closed first-party lowered stop dispatcher
for those native classes plus publication, execution and native-operation
observers. Actual domain/attached-owner identity, native reference membership,
original scope and strict control facts select exactly one native port. It adds
neither public cross-issuer lookup nor a new registry. Seven controls/six semantic
mutants, 22 deliveries/165 refusals and bounded read-only challenge pass at this
scope; full qualification/adoption remain. Public lookup/ingress, establishment/
resync original-command matching, explicit revocation and complete N03 remain
open. The adopted normative baseline is unchanged.

The same working successor now adds the local opaque-reference resolution entry:
one prepaid actual attachment inventory, pure issuer-owned membership and exactly
one final stop. Ten controls/nine semantic mutants cover 22 bound/22 resolved
deliveries and 165 bound/57 resolved refusals; bounded read-only follow-up found
no concrete defect. Selected local lookup is therefore implemented at focused
reference scope, not wire encoding, complete factory coverage or batch adoption.
Stream-command matching, explicit revocation and complete N03 still remain.

Working §12.3.5 now supplies original command ownership for the two registry
stream forms through that existing owner, distinct from snapshot preparations.
It retains the exact original basis/result, one native call and matching
control-only lookup without reallocating a stream or resetting a later queue.
Both routes resolve the actual local owner. Thirteen controls/seventeen semantic
mutants, four routes/twenty refusals and post-native fault injection pass;
faults retain/fence their uncertain registry scope rather than becoming ordinary
unavailable or an automatic restart. Historical baseline payload release and
accepted-registry descriptions remain explicit N02/N03/N05 work, as do revocation,
shutdown and full qualification/adoption. This does not settle U05's unrelated
host-defined handler-registry placement.

### Publication observer attribution prerequisite

The publication-observation /2 helper is a trusted internal observer, not a public
scope validator. Its current-context failure intentionally fences the observer,
including a changed recipient, purpose, destination, policy or epoch. Forwarding
unbound caller data to that trusted port would let a foreign or malformed request
close an otherwise valid stream. The native resource-registry observer already
shows the safer ordering: original stream/scope binding precedes attributable
current-state loss processing.

The facade must therefore resolve the actual issuer-owned observation reference
and retained original scope, reject malformed/foreign/wrong-role caller claims
without changing observer state, then obtain independently attributed current
authorization/policy/continuity/epoch facts for that original observation. Only
then may it invoke the trusted helper. Actual loss remains absorbing; a caller
cannot impersonate a changed epoch, and the guard must not suppress a real one.
This requires a binding contract, not a new policy service or caller-supplied
truth flags. Existing native trusted suppliers may remain premises, with their
actual identity/policy truth retained in E02.

The [working ordering reference](observer-attribution-reference.mjs) uses an actual
committed invocation and queued native publication projection. It checks twenty
foreign scope claims, eight copied/foreign/wrong-role references, eight separately
attributed loss cases and two unguarded-adaptation counterexamples. Native-call
counts and exact observer usage establish refusal before trusted loss handling;
the queued permitted event remains deliverable. Genuine loss clears the queue,
cannot be undone by regrant/resync and preserves producer root/outcome. The two
routes exercised are producer routes, not an F19 typed/dynamic facade. Separate
review found no must-fix in this narrow evidence.

The original /2 helper and adopted input set are unchanged. The working guard is
not an installed gateway, hostile decoder, complete context sum or authoritative
fact supplier. Full F19 acceptance still requires this ordering in its actual
guarded assembly and frozen qualification. INS-03/12/13 must distinguish caller-
binding refusal from attributed context loss, queue invalidation and protected
retirement; neither diagnostic classification may expose raw scope or identities.

The 0.212/model 0.276 successor now integrates this ordering into two actual native
event/minimal-outcome release ports, without changing the old trusted helpers.
Its five controls/eight targeted semantic mutants pass: 44 foreign/malformed
claims, 28 role substitutions, sixteen real losses, sixteen temporary-pressure
cases and six policy cases. Six adjacent regression suites, the exact registered
predicate, 124-module boundary and 124 cold entrypoints pass; bounded review found
no must-fix. The lowered claim is a first-party native value, not public payload
requiring private annotation IDs. The complete public issuer/root/domain/context
assembly is still open. The 976-input snapshot passes all 770 checks, source/worker/
root endpoint audits and byte-exact archive integrity verification; it is adopted.
Session 91050 is terminal exit 0 and must not be polled again. No whole
F19/L02 closure is claimed by these native ports.

### N05 Bind conditional capabilities without expanding scope

For supported analysis, clipboard formats and host facilities, bind advertisement,
accepted versions/profiles, current dependencies, disabled/unsupported behavior
and activation/invalidation. Preserve the required mixed-language reference
integration. Do not add automatic cross-file semantic discovery, a universal
grammar/schema engine, network transport or arbitrary plugin execution.

### N06 Check the integrated contract once

Audit every matrix variant against N01–N05 and its existing evidence. Reuse native
gateways, finite traces, mutation controls, logical bounds and instrumentation
contracts where they cover the adopted meaning. Add evidence only for a specific
uncovered semantic relation or detected defect. A fresh private validator for
every field, metadata profile or row is not an acceptance criterion.

Require the normal frozen full suite and matching endpoints for adopted normative
changes. Keep physical allocation, real policy/provider truth, Rust refinement,
platform measurements and independent assurance in their named release gates.
Do not claim that passing this matrix's inventory check proves behavior.

## Audit disposition

The variant inventory and source-to-contract crosswalk account for all 71
engine variants across 23 families and all five integration workflows separately.
F02/F03/F04, F07's configuration-query/1 and selected resource-catalog-query/1
profiles, F08's two existing analysis-query/1 products and F10's
native-compound-current-checks/1 query bindings are accepted above.
L02 itself remains open: N01–N06 are not closed across the full
facade. N02 is accepted for the eight selected native receipt kinds. This narrows the next work to remaining bindings and
specific gaps; it does not authorize replaying the existing native proof program.

Next, bind original invocations to effect tasks and matching retries, assemble
supported evidence/events, and complete protected control/shutdown composition.
N03/N04 are not blocked by receipt-route selection. Other remaining deliverables
can proceed where independent of U05 and the remaining bindings. L18 reconciles the final normative
documents, model, instrumentation and qualified evidence after those closures.
