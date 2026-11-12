# Workspace identity recovery

Vellum preserves one workspace scope, reserves document/root allocator ranges,
and restores an identifier only when a private record names its exact locator
and the local adapter revalidates that locator's admission provenance.
Inventory order is never recovery evidence: reversing startup order preserves
each locator's identifier, while a same-shaped inventory at different locators
refuses construction rather than rebinding an ordinal.

The host owns `WorkspaceIdentityMetadataV1` and its bounded binding contracts.
The local-filesystem adapter owns the encoded bytes and stores a bounded v3
record in a caller-supplied 0700 same-user directory. The private record
contains locator bytes, protocol ordinals, and separate document/root
`issued_through` and `reserved_through` marks, but no source text; diagnostics
contain none of them. The adapter requires a singly linked
0600 regular record, refuses links and unsafe directories, bounds every read,
uses an exclusive stage plus atomic rename, and reports directory-sync failure
after rename as `OutcomeUnknown`. Load/store counters are saturating,
content-free, and non-authoritative. Every binding requires the named
resource's expected kind and the same containing-directory device/inode.
Ordinary save may atomically replace a file, so file-inode equality is not
durable identity. Rename, removal, kind change, or containing-directory
retargeting refuses recovery. A same-name, same-kind external replacement in
the unchanged directory is locator identity under this policy and cannot be
distinguished across a stopped interval.

The standalone daemon now enforces the initial composition precondition. It
acquires and pins the existing private daemon directory and lifetime lock,
refuses an existing socket, loads the record, constructs either the initial or
recovered service, materializes its startup inventory, and durably commits the
next reservation before binding the client socket. A refused or unknown commit
therefore cannot publish a client endpoint. On recovery, the exact workspace
scope is retained, validated locators recover prior document/root identifiers,
new allocation begins at the exact prior issued prefix plus one, never at the
reservation ceiling, and the next reservation is committed before service.

`Missing` is initial state only when the lifetime lock was exclusively created
by this startup. Once that persistent lock exists, a missing identity record is
evidence of loss rather than a virgin installation and startup refuses it. This
prevents record deletion followed by process restart from silently reusing the
process-local initial scope or resource identifiers. It also means a directory
previously used by the transient embedding constructor cannot be silently
upgraded to persistent identity; migration requires an explicit future tool.
Malformed, unsafe, limited, I/O, conflict, and `OutcomeUnknown` states remain
distinct fail-closed build results. None is repaired or converted to missing.
Legacy v1 and v2 records both refuse recovery. Neither reservation-only schema
is reinterpreted as issued evidence; either needs an explicit future migration
which proves the exact issued prefix.

The operational snapshot reports only content-free disposition and saturating
counters: initial versus recovered, load/store attempts, successful commits,
refusals, and unknown outcomes. It contains no scope, high-water value,
locator, content, request identity, or record bytes and is not an authorization
input.

Consequently this slice establishes:

- restart-stable workspace scope after a committed record;
- fail-closed, exact-locator and non-ordinal startup resource recovery;
- stable startup bindings across ordinary atomic save replacement;
- exact document/root issued-prefix non-reuse, independently of reservation;
- explicit missing, malformed, limited, unsafe, I/O, conflict, and unknown
  recovery outcomes.

The committed reservation prepays the service's complete fixed document/root
inventory capacity, while the distinct issued prefix proves the allocator's
historical extent. Every live binding is at or below its sort's issued mark,
which is at or below its reservation. A generation transition may advance an
issued mark only when every ordinal in the newly issued contiguous tail is
present in the candidate bindings. Removal may remove live bindings but never
lowers either issued mark. The persistent daemon also retains the
metadata port as a pre-reply transaction boundary for explicitly authorized
`AddFile`, `AddDirectory`, `RemoveDocument`, and `RemoveRoot` commands. For one
fresh request the serialized service first authorizes and applies the semantic
transition, publishes its bounded change, and retains its exact reply. It then
projects the complete active binding set plus issued and reserved marks and
stores generation `n + 1` before transport may encode the accepted
reply. An accepted semantic no-op is checked against the exact already-stored
generation. Exact request replay returns the retained reply without repeating
the checkpoint.

If candidate projection, generation succession, or metadata storage refuses,
the daemon sends no semantic reply, reports the exchange as outcome-unknown,
and fences before another connection can be admitted. `OutcomeUnknown` after
rename receives the same treatment: neither rollback nor commit is invented.
The in-memory mutation/change/replay may exist in the fenced process, while a
restart recovers whichever complete generation the adapter can prove. Startup
must reconstruct the exact recovered active locator set, so missing, renamed,
retargeted, or unexpectedly present resources fail closed. Removed ordinals
remain at or below the exact issued prefix and are never reused; the current
record does not retain removed locator history. Recovered ordinary and durable
intents and receipts, including retired-operation tombstones and every
directory-admission document, are checked against the correct document/root
issued mark rather than the larger reservation.

This slice does not establish rename identity, same-name external replacement
detection, automatic reconciliation after an unknown commit, Save As,
overwrite authority, or remote authentication. The transient embedded service
still supports runtime membership but makes no restart-recovery promise for it.
