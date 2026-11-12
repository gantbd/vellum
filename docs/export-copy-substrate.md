# Local export-copy physical substrate

The local macOS adapter has a deliberately narrow authenticated copy-export
surface. It does not expose Save As, change a document binding, accept ambient
authority, or add a generic filesystem command.

The physical create-exclusive writer, source-capture handoff, atomic composite
authorization, pinned local destination object, authenticated service/daemon
composition, durable-operation recovery, real process-loss qualification and
authenticated MCP projection are qualified together.

## Fail-closed exposure gate

Every service, wire, daemon, SDK or MCP route executing this effect must retain
all of the following together:

- serialized bytes come from a non-forgeable, one-use codec-owned capture bound
  to the exact workspace document, revision, resource, and serialization;
- copy export has its own `ExportSourceCopy` action rather than borrowing the
  broader and semantically different `PersistSource` action;
- source/action and destination decisions form one atomic product, so refusal
  of either factor cannot consume the other factor's bounded decision budget;
- authenticated subject/session, authority context, document/revision, durable
  operation, and receipt ownership are congruent;
- destination policy names the same pinned directory object that receives the
  publication, with explicit object-capability semantics across ancestor rename;
- public receipt construction and recovery reject every impossible
  persistence/failure combination; and
- bounded service/daemon USE, RED, and golden-signal projections disclose no
  path, source, byte length, or principal label.

The required-auth daemon installs the interpreter only for one exact
operator-configured `vdo1` operation, document, revision and absolute UTF-8
destination. It consumes the fresh connection session and executes through the
host-owned affine effect port. Ordinary and synthetic service constructors,
owner-only daemon operation and the ordinary MCP catalog remain closed.

The authenticated MCP peer adds no generic export tool. Its existing
`vellum.durable_execute` and read-only `vellum.durable_receipt` unions accept
the exact create-exclusive intent. Discovery discloses only syntax support,
not the installed policy. The response carries document, source revision and
closed persistence evidence, but never the destination, source, operation,
subject, session or authority evidence.

The substrate now uses one `ExportSourceCopy` composite decision. A mismatch
refuses before its one-use source/destination object is consumed, so there is no
partial authority-budget state. The plan binds the exact engine-origin
`DocumentVersion` and serialization state before codec capture; attachment
refuses a capture from any other source or serialization. Source bytes and the
destination object remain internal effect material; public receipts remain
locator- and content-free.

The adapter resolves every absolute parent component through directory
descriptors with `NOFOLLOW`. The final parent and existing file must be local,
regular/qualified where applicable, singly linked, and owned by the effective
user. Create uses an exclusively created private stage and atomic
`RENAME_EXCL`; an exact `EEXIST` is known nonapplication and never overwrites.
Overwrite captures and rechecks identity, bytes, mode, owner/group, flags,
ACL, and bounded extended attributes before ordinary atomic replacement. Both
paths full-sync the private stage before publication and the directory after
publication.

The local composition root installs a one-use, type-erased destination object
containing the qualified parent descriptor and exact leaf name. The effect
adapter accepts only its own object type and never reopens the parent from the
display locator. Deterministic tests rename and replace an ancestor both after
authorization and after preparation; publication remains in the originally
pinned directory and never crosses into the replacement namespace.

Receipts classify persistence as `Durable | NotDurable | Unknown |
NotAttempted`. Construction is closed: `Durable` has no failure or cleanup
failure, while every other constructible receipt carries a failure through the
separate failed-persistence sum. A failure after publication is `Unknown`;
there is no automatic retry, rollback, compensating overwrite, or binding
claim. The current synchronous profile does not emit `NotDurable`.

The daemon synchronizes the canonical intent before the effect and the checked
reply before transport publication. Recovery re-decodes and canonically
re-encodes both sides, then delegates owner/scope, generation, issued-prefix,
receipt-sort, document and revision checks to the host recovery algebra. Real
Unix-socket tests discard the completed reply, restart, and retrieve it without
a second publication; a pending crash cut remains `OutcomeUnknown`. The MCP
qualification repeats this through the production daemon and authenticated
connector, though the MCP server and daemon are composed in-process rather
than launched as two shipped executables.

Security and privacy review:

- STRIDE: the required atomic authority product addresses spoofing/elevation;
  one-use codec evidence and descriptor-object congruence address tampering/replay;
  opaque receipts support repudiation-safe correlation without locator or
  content disclosure. Same-user external writers remain a documented race and
  produce conflict/unknown evidence rather than an atomic-CAS claim.
- LINDDUN: paths and source bytes are absent from receipts, debug output,
  counters, observations and discovery. The trusted policy, canonical durable
  intent and adapter necessarily see the destination; the intent and absorbing
  tombstone retain it in private plaintext storage. At-rest secrecy,
  unlinkability, backup erasure and unlinkability from filesystem side effects
  are not claimed.
- RED/golden signals: bounded observations separate validation, staging,
  publication, and synchronization latency/disposition. Content-free counters
  distinguish attempts, preparations, each persistence class, cleanup failure,
  and saturation.
- USE: stage-attempt and byte limits expose bounded capacity; counters expose
  utilization and saturation without resource names.

The editor's older convenience path still constructs trusted engine facts from
static booleans. It is not promoted to a public receipt and remains outside the
authenticated agent/service route. Successful copy export is never Save As,
activation, rebinding, overwrite authority or evidence that the destination is
the document's current backing resource.
