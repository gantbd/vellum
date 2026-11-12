# Workspace lifecycle contract

Vellum separates per-client view lifecycle from workspace-membership
retirement. Neither claims that a filesystem resource, engine document,
history group, retained save task or provenance owner was disposed.

## Closed view algebra

The native protocol admits two commands:

- `OpenView` returns `ViewOpened { document, status }`, where `status` is
  exactly `Opened` or `AlreadyOpen`.
- `CloseView` returns `ViewClosed(status)`, where `status` is exactly `Closed`
  or `AlreadyClosed`.

The document identity returned by `OpenView` is scoped to the service's
`WorkspaceIdV1` and uses the existing monotone document projection. Opening an
already-open view preserves its selected document. Opening a closed view
selects the workspace's first document; an empty workspace refuses with
`NoActiveDocument`. Closing is total for a known client and releases only that
client's selection projection. After close, source commands refuse with
`NoActiveDocument` until `OpenView` or `SelectDocument` establishes a view.
Client witnesses intentionally survive this transition: they attest to a
captured source revision and are not authority or presentation-view handles.
After a view is re-established, ordinary document and version checks decide
whether such a witness is still current.

Both operations use ordinary request replay. An exact replay returns the
retained receipt without reapplying the transition or reconsulting authority;
a changed command under the same request identity is `RequestConflict`.
Fresh operations require the closed `ManageView` authority action, which is in
the read-only ceiling because it cannot mutate shared source or membership.

Operational counters distinguish the four transition outcomes. They are
saturating, content-free, passive evidence; document IDs are not metric
dimensions and counter state never participates in admission.

## Workspace-membership retirement

`RemoveDocument` and `RemoveRoot` retire bounded, non-reused membership slots;
they do not dispose the native substrate or delete files. A known tombstone
returns `AlreadyRemoved`; a foreign or never-issued identity is `UnknownDocument`
or `UnknownRoot`. Re-admission receives a fresh high-water identity and remains
bounded by the workspace incarnation limit.

Document removal refuses `DocumentInUse` while any client view selects the
document. On success the service revokes all of that document's retained
witnesses. Exact replay still returns the retained old reply, including an old
snapshot reply, but a fresh command cannot consume the revoked witness.
The public receipt intentionally omits the revoked count: another client's
capture activity is not disclosed. Only a content-free saturating internal
counter records aggregate revocation work.

Root retirement never cascades. The current model does not own a root-to-file
provenance relation, so hiding documents would invent one. Native source,
history groups, retained save tasks, resource handles and provenance all remain
owned until a future reclamation contract can prove their individual release.
Neither service operation performs source-adapter I/O or filesystem deletion.
The persistent standalone daemon wraps an accepted fresh retirement in a
separate identity-metadata checkpoint before its reply; that checkpoint removes
the active locator binding while preserving the non-reused ordinal high-water.
Failure or uncertainty fences and yields no semantic reply rather than claiming
that the in-memory retirement rolled back.

Both commands require the distinct `RetireMembership` authority action. It is
absent from the default read-only ceiling and independent of file/directory
admission. Inventory generation advances exactly once for a new retirement,
making prior page cursors stale; `AlreadyRemoved` does not advance it.

## Threat and privacy notes

- Spoofed or foreign document IDs remain unusable because IDs are workspace
  scoped and projections are owner-held.
- View close does not revoke witnesses or persistence authority and must not be
  represented as such by a connector.
- Outcomes contain no locator or source content. They disclose only process
  local opaque identity and whether the caller already held view state.
- Removal receipts do not expose witness counts, client identities, save-task
  counts, history shape, locator data or native-resource state (LINDDUN
  minimization/linkability boundary).
- No partial/unknown view outcome exists because the transition has no external
  effect and completes synchronously under the serialized service owner. If a
  future view acquires external obligations, this algebra must be revised
  before composition.
