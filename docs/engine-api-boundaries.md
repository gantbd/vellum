# Engine API boundaries and compatibility review

Parallel consumer implementation starts from the [SDK automation and UI handoff](sdk-automation-ui-handoff.md).

The engine exposes editing behavior, authoritative logical state and owned effect
contracts. UI, resource mechanisms and optional providers consume or implement
that boundary; they are not alternate mutation engines. This L13 working review
records the packaging requirements and the user-selected API policy. It is not
a qualified normative amendment, Cargo layout or tested support
matrix. Adopted 0.219 §25 reconciles the selected enum policy; full frozen
qualification, endpoint/adoption audit and archive integrity checks pass.

## Public type exposure map

These are contract roles, not selected Rust item spellings or crate names.
Requirements §16 and specification §§1.2/25/28.43–44 govern the boundary.

| Boundary | Permitted engine-facing types and responsibility | Types or authority that stay outside |
| --- | --- | --- |
| Engine library facade | Versioned requests/outcomes, logical coordinates/revisions, exact proposals, resource profiles, capabilities and owned opaque identity/context roles | UI frameworks, concrete filesystem/SDK/credential types, executor-runtime or telemetry ecosystem types |
| Core and coordinator | Distinct document/workspace primitives, ownership, admission, complete publication, history and retained-obligation responsibilities | Public primitives that bypass the coordinator while an object belongs to its engine; duplicate per-UI semantics |
| Resource modules | Engine-owned resolution/effect/evidence contracts and declared identity/capability/guarantee profiles; local filesystem first | OS paths/handles, object-store clients/credentials, syscall mechanisms and provider truth; opaque locator evidence does not become document identity |
| Clipboard integration | Authorized snapshot/selection captures, explicit original-target edits, effect identities/evidence and bounded outcomes through the behavior facade | Clipboard-specific workflow orchestration, system/session provider mechanisms, native formats, shortcut/focus policy and platform acknowledgments; no required clipboard registry in the semantic kernel |
| Serialization edge | Engine-owned fidelity/schema/context contracts, bounded source/artifact bytes and attributed preparation evidence | Parser/serializer library values crossing the engine API; file I/O or implicit format migration during an edit |
| Optional language providers | Captured source/context, supplied-evidence coverage, task/result correlation, leases and explicit retirement | Concrete native parser nodes/SDK types becoming engine authority; automatic asset loading or deferred structural semantics |
| Host execution and clocks | Engine-owned work/evidence/control values, declared clock domains/epochs and completion meaning | A required executor, timer service, native preemption or clock truth inferred from a Boolean/profile label |
| Observation adapters | Authorized bounded owned projections and declared measurement/diagnostic values | Global subscribers, exporter/SDK types, callbacks inside publication or diagnostics granting authority |
| UI adapters and examples | Engine requests, logical selections/targets, snapshots and authorized projections | Keymaps, layout/painting, UI geometry policy, native input presentation and framework-specific types |

One coherent public engine boundary does not require three published crates for
core/coordinator/facade. Separately reusable document primitives may exist, but an
attached object has one authoritative publication path. The in-memory source
backend is a separate internal representation choice, not a resource module.
Headless use does not require a filesystem, parser, UI, exporter or network.
Ratatui/iced adapters and thin applications remain wider-editor obligations,
not conditions for completing the engine API.

## Clipboard contracts without clipboard mechanisms in the kernel

Requirements §§3.1/3.4/10.1 and specification §1.2 already separate semantic
authority from host mechanisms. The model places typed effect/evidence ownership
in CMP-09, host service integration in CMP-10, gestures in CMP-15 and authoritative
local publication in CMP-03. PATH-06 and IF-01/02/03/04 compose these roles; they
do not require a new kernel-owned clipboard registry. F22/F23 are effect-request
contracts, not local source-edit commands or a mandate to import provider code.
The user selected a separately wired, headless-capable integration facade for
those workflow verbs. Adopted 0.213/model 0.277 implements the 71 engine/five
integration/eleven presentation partition; 770 checks over 977 frozen inputs and
source/worker/root/archive audits pass.
Integration is not UI, and no required transfer behavior is removed.

A separately wired integration may orchestrate the workflow for a UI or automation
client. It must obtain an authorized capture, retain the original operation/task
relationship, request the named host effect, preserve attributed evidence, and
submit any later edit through the existing behavior facade. It cannot replace
engine validation with an enablement flag, mutable focus, text equality, a new
effect attempt or an unvalidated acknowledgment Boolean. No integration gets a
second writer into coordinator-owned objects or permission to inspect private
roots merely because it implements clipboard behavior.

| Operation | Engine contract retained | Integration and provider work |
| --- | --- | --- |
| Copy | Authorized exact source/selection capture; no local source publication or text undo entry. | Payload serialization under the selected profile, external export and its actual acceptance/unknown/quiescence evidence. |
| Paste | Explicit captured target/revision and bounded literal or supported fragment edit; complete current validation and one local publication. | Request data, preserve its original task/capture and pass inert accepted content to the edit contract; no key replay or focus retargeting. |
| Cut | Exact current deletion authority, original target/dependencies, atomic companions/history and local disposition. | Stage the original export and pass its attributed outcome into the supported continuation; export success does not imply deletion or a compensating retraction. |
| Retry and cleanup | Original identity/eligibility, bounded retained responsibility, no re-execution or fabricated quiescence. | Retrieve the original task under the promised contract; do not re-admit or create another attempt as matching retry. Preserve provider and session-fragment obligations until their actual discharge. |

This layering has direct precedent. Neovim delegates OS clipboard access to
providers and explicitly distinguishes it from paste insertion. Adopt that
separation, not its provider auto-detection, line conversion or register defaults.
[Neovim provider and paste contracts](https://neovim.io/doc/user/provider/)
Helix's clipboard provider implementation lives in `helix-view` and contains
platform/command/terminal access. This is implementation-placement precedent,
not evidence for Vellum's authorization, cut ordering or failure guarantees.
[Helix clipboard provider source](https://github.com/helix-editor/helix/blob/master/helix-view/src/clipboard.rs)

The existing combined reference modules test logical task/evidence/edit composition;
their import graph is not a required production package graph. The unadopted
invocation-owned clipboard-registry draft remains held rather than becoming an
engine requirement. Original issuer/task/retry binding, copy's truthful effect
assembly and the two F11 continuations remain in the northbound audit. Moving
mechanisms outward does not waive those contracts or choose U01's public result
route. Actual package separation and provider truth remain E01/E02 conformance
gates. INS-04 measures the integration/provider phases separately from INS-01/02
engine admission/publication; neither phase's success implies the other's.

## Feature combination contract

Supported feature combinations must preserve the meaning and safety of existing
engine profiles. Compiling a feature cannot grant rights, select a provider
instance, load an asset, download data or start a service. Enabling an integration
must not change another integration's profile interpretation or native guarantees.

Keep authority-bearing operation/control vocabulary stable across supported
feature combinations. An absent integration reports its declared unsupported or
unavailable disposition; it does not change an existing enum's meaning. Additional
versioned types/profiles can be explicit extension points. Do not use mutually
incompatible semantic feature switches as an implementation of one promised
profile. Cargo's feature unification makes additive feature design important;
supported combinations still need actual builds and conformance evidence.
[Cargo feature guidance](https://doc.rust-lang.org/cargo/reference/features.html)

This does not promise no_std, a particular feature list, all combinations of
third-party dependencies, or a zero-dependency implementation. U03/E01 retains
the actual package/features/MSRV/dependency and support matrix.

## Selected enum policy

The user selected exhaustive authority-bearing executable enums within explicitly
versioned behavioral profiles. Downstream exhaustive handling and explicit
vocabulary evolution match the engine's fail-fast, finite-vocabulary contract.
Adding executable variants or changing payload meaning requires an explicit
source/API and behavioral-version compatibility decision, not a silent same-profile
extension. Rust's non_exhaustive attribute changes downstream construction and
matching rules; it does not itself admit unknown execution semantics.
[Rust type-system reference](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute)

The selection applies to authority-bearing operation/control/outcome types,
not arbitrary descriptive metadata. Known Unknown effect
or knowledge states remain ordinary named cases, not an Unknown(payload) escape
hatch for executable extensions. Cargo treats additions to exhaustive public
enums as source-breaking; package versioning and behavioral-profile compatibility
must both be checked.
[Cargo SemVer guidance](https://doc.rust-lang.org/cargo/reference/semver.html#major-adding-new-enum-variants-without-non_exhaustive)

## Compatibility and migration procedure

Carry these existing obligations into the API specification regardless of the
selected enum policy:

1. Classify source/API compatibility and behavioral refinement separately.
   Defaults, coordinates, outcomes, ordering, bounds and evidence milestones count
   as behavior even if downstream code still compiles.
2. Preserve the existing profile's assumptions and guarantees. New executable
   meaning needs an explicitly accepted version/capability; unknown dynamic
   requests refuse before effects rather than entering a generic execution path.
3. Keep queued/in-flight values under their captured interpretation. Retain
   required assets/contexts or refuse activation; migration cannot erase holds,
   reset spent issuance, relabel historical evidence or silently restart effects.
4. Document intentional breaks, affected profiles, the replacement and migration
   steps. Do not remove/degrade a promised API as a compatible change. Deprecation
   periods and support dates need an actual release policy, not guessed numbers.
5. Version exposed persisted formats separately. Source/annotation fidelity and
   unknown-version refusal remain fixed; any migration needs explicit acceptance,
   preservation of original inputs and failure without destructive replacement.
6. Treat security-driven restrictions honestly as intentional contract changes
   where applicable. Do not call a restriction transparent merely because necessary.
7. Before release, execute downstream compile/conformance checks over the declared
   minimal headless and promised integration combinations, plus material-boundary
   model/adversarial review. No stable native ABI or future wire protocol follows.

## Evidence and remaining decisions

U05 remains an actual ownership decision: §22.2.3 places trusted host-command
bindings above the coordinator and assigns execution affinity/serialization to
the host binding adapter, but does not explicitly place the arbitrary descriptor/
handler registry inside or outside the engine facade. The recommended boundary
is the headless integration adapter, preserving the engine's closed builtin
behavior vocabulary. User approval is pending; no placement is inferred from
CMP-02 or CMP-10. Either placement must preserve PH-09's original descriptor/
handler/context correspondence, current authorized discovery, retained code/assets
and validation of each emitted engine request. An adapter label alone proves none
of those handoffs. Missing arbitrary Rust handler implementation is not by itself
a missing engine mutation rule.

### Contract reconciliation

The following crosswalk audits existing normative contracts, not a proposed
package implementation. It supplies the package-map reconciliation for L13;
it records the selected enum policy but does not establish build conformance.

| Required design facet | Existing normative contract | Disposition |
| --- | --- | --- |
| Separable engine, providers, adapters and apps | Requirements §16.1; specification §§1.2, 25, 28.43, 29.5 | Architectural contract covered. The exposure map above assigns crossing types without requiring a particular crate count. Actual manifests and dependency isolation remain E01. |
| Headless core and modular resource substrates | Specification §25; requirements §16.1 | Covered: no required filesystem, network, UI, parser or exporter; local filesystem is the first separately wired resource module. In-memory text storage is a different boundary. |
| Optional features and activation | Requirements §§16.1–16.2; specification §§25, 28.43 | Covered: compilation/import grants no authority, loads no assets and starts no service. Supported combinations require builds; no all-combinations claim follows. |
| Behavioral profiles and evolution | Requirements §16.2; specification §§25, 28.44 | Covered: explicit versions, closed executable vocabulary, unknown-variant refusal, no stronger caller obligations or weaker guarantees under the same assumptions. Adopted 0.219 reconciles the user-selected exhaustive versioned executable enums; qualification and endpoint/adoption audit pass. |
| Retained interpretation and migration | Requirements §16.2; specification §§22, 25 | Covered procedurally: preserve captured interpretation and obligations, document intentional breaks and migrations, and verify adapter semantics. Concrete format migrations and release/deprecation schedules require an actual format/release policy; no dates are inferred. |
| Assets, provenance and supported environments | Requirements §16.1; specification §§25, 29.5 | Acceptance obligations covered. Production redistribution inventory, toolchain/MSRV, dependency and OS/procedure combinations remain U03/E01–E02; accountable support commitments remain U04/E04. UI choices are separate. |

Existing finite boundary checks support ownership, selected execution and
unknown-profile refusal only at their stated scopes. They cannot prove a Rust
package graph, downstream compilation, license rights or installed platform behavior.
No additional reference model is needed merely to restate these package roles.
Implementation evidence must exercise the actual exported API and package graph.

Existing engine/UI and selected substrate checks support their stated reference
boundaries, not this unimplemented Rust export graph. Catalog/version controls
support unknown-variant refusal and profile semantics, not downstream compilation.
Once the actual API exists, include exhaustive consumer
tests, feature-unification builds, incompatible-profile/format refusal, retained
interpretation and migration-failure scenarios. A logical proof cannot establish
MSRV/platform compatibility or redistribution rights.

L13's normative/package-map crosswalk covers the listed boundaries, crossing
types/features, version evolution and support choices requiring U03. The selected
enum policy is reconciled in adopted 0.219. The subsequent U05 review identified
one remaining placement ambiguity: the arbitrary host-command registry. L13's
design acceptance therefore still needs that ownership decision reconciled.
Full 0.219 qualification and endpoint/adoption audit pass; it does not require a second
package model or invented support matrix. Actual crate names/manifests, Rust versions/MSRV,
supported OS/procedure combinations and bundled redistribution inventory stay
U03/E01–E02. Accountable reporting/support commitments stay U04/E04. UI framework
choices remain in the separate UI stream. No new plugin ABI, runtime module loader
or cloud provider is selected by this review.
