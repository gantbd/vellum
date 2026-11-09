# Current read acquisition after publication observation

Specification §12.8.2 separately checks actual native F13/same-ledger F14 through
this current-read path; see [native composition](annotation-reads.md). The earlier
checker below retains its own normalized scope and exclusions. No direct event or
receipt reference conversion is introduced.

Specification §12.8.1 composes the existing observer, annotation projector, read
preparation and read registry for a fresh current capture. Event generations and
annotation references remain non-grant stream identifiers, not historical root,
context, record or continuation selectors. Enumerate the acquired read context to
obtain its record selectors; do not reinterpret a stream reference as one.

## Precedent and selected boundary

[Neovim buffer update events](https://neovim.io/doc/user/api/#api-buffer-updates)
and [scoped extmark lookup](https://neovim.io/doc/user/api/#nvim_buf_get_extmark_by_id())
separate update notification, object identity and subsequent API calls. This is
precedent for keeping those products distinct, not evidence for Vellum permission
truth or historical retention. The existing F02 `read-context/1` request/result
and F03 queries suffice for a new current capture. No new verb, request field,
transport or implicit event-to-snapshot conversion is introduced.

`capture-current` selects the actual root supplied at the serialized capture
point. It can be newer than the latest event already delivered. Its context owns
that captured data; it does not claim equality with the event's generation.
Exact historical reads still require an independently retained authorized
snapshot reference under §12.8, not an event reference or a fallback to current.

## Reference composition

The trusted host routes the existing F02 request with a live owned observer and
actual current root. Read preparation completeness/reservation are checked before
copying the capture or consuming visibility preparation, not only at later registry
admission. `capturePublicationRead` checks the observer's unchanged
recipient, purpose, destination, policy, continuity and profile, liveness/stop,
coverage, projection, resource and current-release gates. A gap refuses this
composed path until recovery; independent F02 acquisition retains its own contract.
Source requests require the captured source ceiling. Annotation requests require
the complete collection mode, not a named subset mislabeled as the document's
complete permitted collection. Combined requests cannot silently drop a denied
class. Source-only acquisition remains available under an explicit source scope.

Reuse the actual collection projector with fresh owned current-root visibility
preparation. Intersect captured and current row body/target permissions before
constructing read rows. Invisible records never enter the read collection.
Candidate stream reference allocation remains temporary and subject to the
observer's existing staging bounds; this conservative reference can refuse at
that limit. It does not install bindings, advance a generation or queue an event.
The observer's real baseline, ledger and issuance remain unchanged on success or
refusal. Visibility preparation consumption still follows its single-use contract.

`acquireObservationRead` checks all seven read-scope dimensions against the
observer scope, then calls actual `prepareReadContext` and `acquireReadContext`.
The ordinary read registry independently admits context count, units, selectors,
issuance, retention and transfer. Failed preparation/installation leaves the
registry unchanged and exposes no partial context. Annotation-only acquisition
does not require source-read authority: source retained privately for coherent
ownership is not included in its admitted public data classes. Actual host
reservation and physical retention remain premises.

The acquired context captures one immutable permitted collection. Subsequent
F03 enumeration issues the existing context-specific record/continuation
selectors; stream references are only private intermediate row identifiers in
this composition. Later permission cannot widen the captured body/target facets.
Every query still checks current scope/grant/liveness/continuity/transfer.
Stopping the observer prevents new composed captures but does not close an
already acquired independent read context. Its own owner closure retires all
dependent query selectors under §12.9; neither operation revokes bytes already
transferred or replaces current policy checks.

## Conditional argument and evidence

Assume coherent serialized normalized roots, truthful accepted policy/visibility,
non-confusable identities and actual bounded preparation/retention/transfer.
The shared projector bounds read data by the observer ceiling and current facts.
Read preparation copies that permitted domain into a new immutable capture, and
registry installation checks the same complete scope before minting a selector.
Existing read queries cannot return uncaptured facets. Independent selector
namespaces and lifetime lookup prevent an event token from being accepted as a
read selector. No step installs the temporary observer ledger; read success and
refusal preserve stream state. These are conditional composition arguments,
not general implementation induction or categorical completeness.

The checker exercises 512 read-gate and 2,048 observer-release combinations,
16 facet intersections, 64 selected hidden-world pairs, twelve controls and ten
assertion-rejected semantic mutants. Controls cover a newer current root,
annotation-only and combined capture, exact scope, historical/extended requests,
owned single-use preparation, registry refusal, current query release, separate
stop/retirement, temporary reference staging, complete versus named scope and gap.
The actual observer, collection projector, read preparation/registry and F03
enumeration/inspect/location/closure paths execute; no alternate model stands in
for them. Finite worlds do not prove timing or resource noninterference.

## Instrumentation and remaining gates

PATH-02 and IF-02/08/09 separate notification, current capture, selector installation,
query handoff and closure. INS-03 records current scope/facet/release decisions and
read-registry/staging pressure; INS-08 tracks retained snapshot/lifecycle obligations;
INS-13 tracks protected closure/control service. Context installation is read activity,
not INS-02 authoritative publication traffic. MELT/RED/golden
signals distinguish useful query delivery from observation delivery and recovery.
USE tracks held contexts, selectors, units and staging. STRIDE covers forged owner,
stale context, selector-kind confusion, partial installation and ceiling expansion.
LINDDUN covers hidden membership and falsely attributing current data to an older
event. Public labels contain no content, raw identity, private counters or selectors.

Historical event lookup and direct event-to-record-selector conversion are not
provided or proved. Host root/policy/order/reservation/retention truth, all lifecycle
and multi-document event products, invocation/replay integration, actual concurrent
installation, hostile decoding, physical costs, timing/resource privacy, measurement,
production refinement and independent assurance remain gates. The point-storage
decision is unchanged. This closes only the fresh-current normalized composition,
not whole F02/F03/F19, all sections or an independent Cleanroom certification.
