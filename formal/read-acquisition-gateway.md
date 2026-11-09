# Decoded read context acquisition

Section 22.1.16 validates closed single/coherent acquisition results and actual
native staged identities/candidate meaning. Preinstallation faults preserve native
and prepared state. Late facade faults retain actual installation, spent issuance
and consumed cuts, fence admission and refund only temporary decoding workspace.
Shape does not authenticate fresh aliases; see read-acquisition-results.md.

The selected F02 `snapshot-handoff` facade preserves the existing single-document
and coherent-set acquisition meanings through typed and dynamic requests. It
creates no new operation, UI mode, transport, filesystem mechanism or owner.
The implementation is a scoped design reference, not a frozen production ABI.

## Requests and ownership

`read-acquisition-arguments.mjs` derives its two input declarations from the actual
single-document and version-set catalogs. Each supports current capture and
retained acquisition. Exact inert records carry document identities, admitted
data classes, profile and selection. Class sets canonicalize; participant order
does not. Duplicate document slots, including different incarnations of one slot,
are refused. Typed and dynamic envelopes produce the same canonical meaning.

A retained reference is an original opaque capability, kept outside copied
arguments. Its presence is not authentication or a read grant. The actual native
owner checks it and current authority independently. No arbitrary historical-root
lookup or fallback to the current root is introduced.

The gateway selects a fixed native port by the original pool handle. Source-only
current acquisition captures through its existing owner. Current annotation and
coherent-set acquisition require the original private preparation pool and cut.
Before native validation, a private boolean check binds the request's document,
classes and profile, or ordered participants and classes, to that cut. A changed
request cannot consume or poison another prepared cut, even when accompanied by
loss facts. The check returns neither captured content nor authority. Native
installation still checks scope, current facts and one-time transfer. The captured
root is the root selected by preparation; handoff does not silently recapture a
newer publication.

Retained acquisition requires no preparation and cannot accept a superfluous cut.
Native copies own their contexts independently; retiring an original prevents its
reuse without invalidating an already acquired copy. An available original must
pass its native whole-original authority checks before resource admission.
Known loss remains absorbing even when capacity or transient resources refuse;
unknown facts and foreign scope frames do not invalidate another owner's context.

## Temporary workspace and failure

The actual invocation parent reserves classification bytes and eight logical
frames, each bounded by the lesser of the input byte ceiling and the owner's
intent ceiling. Those frames cover the envelope, authority, canonical record,
limits, descriptor, separate current/facts packet, producer request and preparation
binding. A busy fence excludes reentry. Protected cleanup refunds only this
temporary reservation and clears its observations; native preparation and installed
context reservations retain their existing owners and issuance rules.

Unexpected pre-handoff failure fences the invocation owner. The fixed probes
exercise classification and workspace faults before native acquisition or
installation. They establish no post-install crash rollback. Raw ingress,
descriptor enumeration, physical allocations, global accounting, concurrent host
execution and provider truth remain separate obligations.

## Executable evidence

Run the following scripts from the project root:

- `node formal/read-acquisition-argument-mutations.mjs`: four argument shapes,
  116 malformed refusals, 48 capture boundaries, nine actual native compositions,
  nine controls and 23 assertion-detected semantic mutations.
- `node formal/read-acquisition-gateway-mutations.mjs`: eighteen route/native
  acquisition cases, six parent-headroom boundaries, 62 refusals, seven controls
  and twenty assertion-detected semantic mutations.
- `node formal/read-acquisition-gateway-fault-checks.mjs`: baseline,
  classification fault, workspace fault and reentry, each across nine branches.
- `node formal/read-acquisition-gateway-substrate-checks.mjs`: 143 callable
  filesystem/process exports intercepted after import, no selected baseline calls,
  and three injected call types detected across nine branches each. Temporary
  workspace is refunded, owner admission fenced, and prepared/native state remains
  unchanged on each injected pre-handoff failure.

Mutation construction failures do not count as semantic detection. Copied graphs
first pass the coherent baseline. These checks do not establish complete F02/F03
public results, discovery, asynchronous lifetimes, production import isolation,
all-host containment, measured bounds or independent assurance. Full frozen-suite
verification and requirements/model reconciliation remain necessary for adoption.
