# Compatibility baseline

Status: deterministic conservative change detector; not a semantic-version
proof.

Vellum keeps the reviewed release baseline in
[`release/compatibility-baseline.json`](../release/compatibility-baseline.json).
`make check-compatibility` verifies it without downloading a tool or contacting
a registry. The gate is part of coordinated CI and the GitHub source archive
contract.

## What is closed

The baseline records and compares exactly:

- the sorted Cargo workspace-member inventory;
- workspace and effective per-package version, edition, minimum Rust version,
  license, repository, and publication setting;
- each package's declared feature names, exact feature-table digest, and inferred
  library/binary target names and paths;
- a deterministic SHA-256 Merkle-style digest over every regular `.rs` file in
  each library package's `src` tree, including its sorted path and content hash;
  and
- the complete sorted inventory, encoded byte length, and exact content hash of
  every canonical `.hex` golden in both `protocol/vellum-wire/tests/fixtures`
  and `protocol/vellum-semantic-wire/tests/fixtures`.

Inputs must be repository-relative, canonical, regular files and directories.
Symbolic links, path escape, malformed baseline JSON, unknown checker arguments,
noncanonical fixture text, an empty fixture inventory, and any baseline mismatch
fail closed. The adversarial test changes Rust source, a golden frame, package
publication metadata, and workspace membership, injects a source-tree symlink,
and replaces the baseline with an incomplete object; every mutant must refuse.

The baseline is canonical JSON. After an intentional compatibility review, print
the candidate replacement with:

```sh
node scripts/check-compatibility-baseline.mjs --emit=-
```

Review the complete diff and update the checked-in file in the same change as the
contract, version, migration, and changelog evidence. The checker never rewrites
the baseline itself.

## Current reviewed change

The November 2026 refresh records 34 packages, the complete native golden
fixture directory, and the separately frozen semantic-session command/reply
fixtures. The
review classified the changes as follows:

- the nonpublishing `vellum-launchd-activate` distribution package is additive,
  source-built operator tooling. It is absent from both the two-binary Cargo
  install closure and the release-candidate payload, and its test interpreter
  injects every platform effect;
- native command/outcome tag `17` adds the bounded atomic in-memory workspace
  edit. Existing frames are unchanged; older exhaustive V1 peers must be
  upgraded before receiving the additive tag;
- semantic query-kind tags `8` and `9` add provider-native incoming and
  outgoing call hierarchy. Existing frozen semantic frames remain unchanged;
  older closed-vocabulary peers must be upgraded before receiving them;

- the nonpublishing `vellum` distribution package is additive and contains no
  library API; it installs the existing daemon and MCP peer binaries in one
  Cargo invocation, while their owning packages remain independently buildable;

- inventory pagination adds command/outcome tag `10` and refusal tags `22`
  through `24`; membership retirement adds command/outcome tags `11` and `12`
  plus refusal tags `25` and `26`;
- those tags do not alter the bytes of the retained goldens, but they are only
  additive for peers that reject or negotiate unknown closed-vocabulary tags;
  an older exhaustive V1 peer must be upgraded before receiving a new frame;
- `RetireMembership` widens the host's closed authority-action lattice while
  remaining outside the default read-only ceiling;
- reducing the inventory page maximum from four to two is a source and
  behavioral incompatibility for consumers of the intermediate unreleased
  implementation. It was corrected before a Vellum release so that every
  permitted page fits the canonical sixteen-field wire profile; and
- the adapter's private identity record advances from fixed v1 reservations to
  bounded v2 locator bindings. This is an internal persistence migration, not
  a public protocol-frame change. Legacy v1 records decode only without
  bindings, and populated legacy deployments require an explicit migration.

The later persistent-membership checkpoint slice changes the unpublished
standalone-daemon composition behavior: action ceilings containing
`AdmitFile`, `DiscoverDirectory`, or `RetireMembership` are now accepted when
the persistent metadata port is retained, and the CLI again exposes the three
corresponding opt-in flags. No command, reply, refusal, or wire tag changes.
An accepted fresh mutation now adds a synchronous pre-reply metadata store;
store refusal or uncertainty produces a transport `OutcomeUnknown` and fences
the daemon. Exact replay does not repeat the store. This replaces the
intermediate unreleased blanket build refusal; because no released contract
exposed it, the unreachable variant was removed rather than retained as
misleading public taxonomy.

The subsequent semantic-peer slice is additive Rust source API in
`vellum-host`: a new `semantic` module and its V1 outbound consumer types. It
now also exposes public deployment-ceiling validation predicates used by the
provider-neutral substrate. The additive `vellum-semantic-service` and
`vellum-semantic-client` packages introduce exact-snapshot synchronization,
bounded asynchronous routing and a zero-authority client facade. They do not
add or alter a native Vellum command, wire tag, golden frame, daemon behavior or
existing SDK dependency edge; `vellum-sdk-embedded` continues to expose the
host module through its existing `host` re-export. The contract is not a
provider, LSP or remote-interoperability claim. Adding the two workspace
members and their exact source digests requires an intentional coordinated
baseline refresh before `make check-compatibility` can pass; it is not a claim
that package-inventory changes are invisible or Rust-semver compatible.

The local-authentication slice is also additive Rust API: `vellum-daemon`
provides an optional required handshake composition plus profile/usage
projections and one new daemon serve/refusal class. The opaque local claim,
provider seam and authenticated connector now live in `vellum-transport` and
are consequently available through `vellum-sdk-client`; the daemon retains
source-compatible re-exports. No concrete credential provider ships. The
default constructor and shipped CLI
retain their prior owner-only framing with no pre-frame bytes, so existing
local clients are unchanged unless an embedding explicitly opts into the
required profile. The native V1 request/reply codec and golden fixtures do not
change. The daemon reads its incarnation from the Unix OS CSPRNG device using
existing standard-library and rustix filesystem capabilities, without adding a
locked package or feature.

The bounded change-feed slice adds command/outcome tag `13`, refusal tags `27`
and `28`, and two canonical golden frames. Its Rust protocol types, service
limits/usage fields, and client method are additive only for peers that reject
or negotiate unknown closed-vocabulary tags; exhaustive older V1 decoders must
upgrade before receiving these frames. Feed cursors are deliberately volatile
and cannot resume across restart. This is not a Hermes interoperability or
durability claim.

The durable-edit composition adds command/outcome tag `14`, two canonical
golden frames, an additive client method, and additive protocol receipt types.
It does not alter ordinary `Replace` or `Save` bytes or semantics. As with the
other additive closed-vocabulary commands, an exhaustive older V1 decoder must
upgrade before receiving tag `14`. The receipt deliberately exposes partial and
unknown persistence facts; treating every accepted outcome as durable is an
incompatible consumer assumption, not a promise of this contract.

The launchd distribution slice changes no Rust library API or native V1 frame.
Its current private GitHub candidate manifest schema 4 retains the two product
binaries, adds the checksum-closed inert renderer and explicit activation
executor as distinct deployment artifacts, and captures both source closures.
Existing schema-2/schema-3 candidate directories and qualification evidence
must be rebuilt under schema 4; there is no migration that treats an unlisted
helper as trusted payload. Qualification help-smokes the activator without its
explicit effect switch and therefore performs no launchd effect.

The cargo-install qualification slice likewise changes no Rust package API or
wire frame. It tightens distribution evidence from an ambient path-install
smoke to the documented exact-revision Git install under `--locked --offline`
in fresh Cargo/install/target roots, with an exact daemon+MCP binary closure
and adversarial filesystem/environment checks. This is a release-process
compatibility constraint: source payloads
that relied on an ambient registry cache, extra binaries, symlink binaries or
writable installed executables now fail rather than receiving release credit.

The MCP workspace-surface slice remains on MCP `2026-07-28` and adds advertised
resource capability, `resources/list`, `resources/read`, and two tools to the
former three-tool catalog. Legacy `initialize` remains rejected because this
modern version removed it. It is additive for clients that discover and list
capabilities, but incompatible with a client that treats the old exact catalog
as closed. Resource/change cursor strings are connector-owned opaque values,
not native wire encodings or durable cross-process tokens. Existing native
commands and wire bytes are unchanged. The durable-edit retry correction is a
behavioral fix: after `OutcomeUnknown`, an identical tool call now replays the
exact retained native request instead of attempting a new snapshot that the
native uncertainty fence must reject.

The subsequent durable-operation journal adds an adapter-neutral host port,
local-filesystem adapter, daemon-limit field, daemon build refusal, and trusted
injection constructor. These are Rust source/API changes to unpublished crates;
the native V1 wire tags and golden frames are unchanged. The persistent daemon
reserves recovered receipt client ordinals and, only under required
authentication, retains effectful request/reply bytes across restart. The
owner-only profile preserves volatile replay behavior. The private journal
format is V1 local state with no promise beyond its documented fail-closed
recovery contract.

The exact-admission-receipt correction keeps legacy `FileAdded` and
`DirectoryAdded` outcome tags `4` and `5` decodable and leaves the existing
add-file/add-directory request fixtures byte-identical. The service now emits
additive outcome tags `15` and `16` carrying operation-scoped file and directory
receipts. Directory document identities use one canonical length-delimited
packed field, so the complete configured 256-document admission remains within
the reference sixteen-field profile. Two new golden replies freeze those
shapes. Older exhaustive V1 decoders must upgrade before receiving the new
outcomes; no locator or unrelated inventory is added to the result.

The durable-operation resume slice adds native request tags `0x04` and `0x05`
and reply tags `0x86` and `0x87`. Both requests encode the fixed 128-bit
operation reference before the ephemeral client and request identifiers, then
encode the restart-stable effect intent. The durable outcome is a separate
stable receipt algebra and never serializes a session `VersionWitness`.
Retained ordinary V1 frames and goldens remain byte-identical, but an older
exhaustive peer must upgrade before receiving these additive closed-vocabulary
tags. The local adapter persists these records only in the separately versioned
subject-namespaced `.vellum-durable-operation-v2` relation; it neither treats
historical raw request records as operation records nor silently migrates
between formats. Presence of the superseded operation-v1 directory refuses
recovery and writes until an explicit migration is supplied.
Pending records remain inspectable as `OutcomeUnknown` after restart and are
never retried by lookup. The added wire fixtures, workspace source inventory,
and package source digests require the coordinated baseline refresh that
accompanies this reviewed change.

The semantic-session actor slice adds one unpublished service package and two
host authority actions, `SemanticQuery` and `SemanticSynchronize`. The actions
are distinct lattice atoms and remain outside the default read-only set. The
actor adds a Rust composition surface for bounded volatile serialization,
private affine-effect interpretation and exact admission/control phase truth;
it adds no native or semantic wire tag and changes no frozen fixture bytes.
Consumers of the intermediate unpublished host action lattice or workspace
inventory must refresh against this reviewed additive source contract.

The following semantic frame-transport slice adds one unpublished transport
package. Its Rust API defines a one-attempt canonical byte exchange and
content-safe usage counters; no existing dependency edge, socket profile or
wire tag is replaced. The native and semantic golden fixture bytes remain
unchanged. The package is additive, but consumers treating the workspace
inventory as an exact closed set must refresh to the 30-package baseline.

This review does not claim Rust semver compatibility or exhaustive V1
interoperability. It records the deliberate contract consequences that
accompany the exact source and golden change detector.

## Explicit nonclaims

This is intentionally stricter and less expressive than Rust semver analysis.
It hashes all library source—including private implementation and test modules
under `src`—so a compatible internal edit can require a reviewed baseline update.
It does not parse Rust visibility, types, trait coherence, layout, feature
interactions, platform-specific expansion, macros, or downstream compilation.
It also does not inspect binary-only source as a Rust library API.

Frozen wire goldens prove only that the named examples retain identical bytes.
They do not establish exhaustive encoder/decoder compatibility, unknown-field
behavior, protocol negotiation, or interoperability with historical clients.
Those remain the responsibility of protocol tests, malformed-input/property
tests, client interoperability suites, version policy, and eventually fuzzing.

Accordingly, this gate is useful release evidence because it prevents silent
drift. It is not permission to label an unreviewed change compatible, and it is
not a substitute for a future rustdoc/API-semver tool once such a tool is pinned,
vendored, and qualified for offline use.
