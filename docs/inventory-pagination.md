# Bounded inventory pagination

`WorkspaceInventoryPage` is the preferred V1 inventory contract. It returns at
most two entries in canonical document-then-root order. This conservative
cross-transport bound uses the larger root representation, so every permitted
page plus continuation metadata fits the canonical sixteen-field wire profile.
`next: None` is the
only end marker; a present opaque cursor is scoped to the workspace and exact
structural inventory generation.

The generation changes only when workspace membership or root structure is
published. Source edits and saves do not change it. A foreign, stale, forged,
or out-of-range cursor is refused as a whole: the service does not resync,
return a partial page, or publish a partial identity projection. Page size zero
or above the cross-transport bound is `InputLimit`. Each fresh page request is
authorized as `InspectWorkspace`; exact request replay returns retained output
without a second authority consultation or page-counter increment.

The cursor and inventory identities are content-free live-service values. They
contain no paths, source text, or durable restart promise, and their debug
forms redact all numeric components.

## Compatibility

The original `WorkspaceInventory` command, tags, and golden fixtures are
unchanged. It remains an all-at-once compatibility command under the existing
workspace capacity ceiling. Legacy `FileAdded` and `DirectoryAdded` outcomes
retain their V1 codecs for old retained frames, but the service no longer emits
them. Add-file and add-directory now return exact admission receipts under
additive outcome tags `15` and `16`; callers enumerate unrelated current
membership only through inventory pages.

The canonical wire adds command/outcome tag `10`, refusal tags `22` through
`24`, and two new golden fixtures. This is additive for decoders that negotiate
the closed V1 vocabulary; older exhaustive decoders must be upgraded before
accepting these frames.
