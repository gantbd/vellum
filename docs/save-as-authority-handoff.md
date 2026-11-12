# Save As authority handoff

Existing-destination Save As is intentionally not exposed by the native service yet.
It overwrites an existing filesystem destination and changes the document's ordinary
binding. Those effects require explicit user approval and a destination-scoped policy;
the broad persistence roadmap is not itself approval to overwrite an arbitrary path.

The service preflight is prepared for a bounded two-action plan. Save As must obtain
both `PersistSource` and `RebindSource` decisions, validate each decision against its
exact host-established context, resource, and action, and complete both checks before
preparation, probing, writing, task admission, or rebinding. A partial grant has no
filesystem or editor effect. `--allow-save` must remain Persist-only; a future explicit
Save As composition may join Persist and Rebind without widening the existing flag.

The engine's trusted export facts must not be manufactured from those two decisions.
Their required provenance is:

| Fact group | Required source |
| --- | --- |
| authorized, scope, write allowed, destination allowed | Exact Persist decision plus a destination-scoped host policy |
| binding activation allowed | Independent exact Rebind decision |
| continuous, live, not stopped | Current service/client lifecycle state |
| source readable, versions available | Exact active-document witness and engine capture |
| binding/writer coverage, identity allowed | Engine ordinary-pair and alias checks |
| provider available, resources ready | Selected adapter/resource registration state |
| launch allowed, reply ready | Preflighted bounded service/request retention state |
| probe attested, resource continuity attested | Fresh adapter probes around write and activation |
| companions ready | Explicit composition policy; never inferred from a path |
| overwrite allowed | Explicit existing-destination approval |
| create allowed | Implemented only by the separate `ExportCopy` create-exclusive substrate; not installed in this Save As flow |
| disposition readable, reservation release allowed | Current retained-task/control state |

The current Save As flow supports replacement of an existing qualified file only.
The separate local `ExportCopy` substrate can publish a new destination with
create-exclusive semantics, but no codec-owned source-capture bridge, service
command, durable receipt, destination-policy composition, or rebind activation
connects it to Save As. Missing destinations therefore remain a closed refusal in
this flow. No rollback or compensating overwrite may be claimed after a successful
write whose later activation fails.

## Native surface closure review

The editor and local-filesystem substrate already implement the important physical
sequence: capture the exact source, reserve an existing destination, re-probe before
the write, obtain a synchronized write receipt, re-probe again, and activate the
binding separately. That is not yet enough to expose `SaveAs` through
`vellum-protocol`, `vellum-wire`, or `vellum-client` truthfully.

The following gaps are release-blocking for that public surface:

1. `vellum-service` currently consumes ordinary authority decisions in
   `authorize_plan` and retains no exact evidence for construction of the engine's
   trusted export and activation facts. Turning successful `PersistSource` and
   `RebindSource` checks into raw `Some(true)` values would erase provenance.
2. The service has no installed destination-policy interpreter. A locator carried by
   an untrusted command cannot authorize its own overwrite. `vellum-host` now owns
   an affine `SaveAsAuthorityRequestV1` product over exact Persist, Rebind, source
   version, and overwrite-existing destination decisions. Its authorized plan accepts
   the engine capture only after the complete product is validated, and its adapter
   handoff retains a congruent one-use destination object. This closes the portable
   authority mechanism, not the retained service lifecycle or an exposed command.
   `vellum-session` now bridges this product before native preparation/capture and
   retains the authorization with the exact preparation for later one-use service.
3. `vellum-session` now owns a portable retained Save As coordinator over opaque
   preparation, attempt, and activation tokens. Admission, write, activation,
   release, and retirement errors preserve the exact reconciliation token; no
   recovery path can repeat admission, write, or activation. A non-effecting native
   recovery API reconstructs the activated document handle when the original task
   proves binding publication completed. `vellum-editor::Workspace` now owns this
   coordinator per stable document, excludes conflicting edits/saves/removal until
   activation, and commits only the private locator/native-handle swap after durable
   write plus activated binding evidence. Workspace identity, document identity,
   active views, and inventory generation remain unchanged; subsequent ordinary
   saves route through the new handle. The coordinator ports are stage-specific,
   allowing each workspace call to borrow only its authority, execution, activation,
   or cleanup dependencies.
4. Save As activation is a fault-fenced engine publication. `vellum-host` now owns
   the closed native receipt sum with separate `Durable | NotDurable | Unknown |
   NotAttempted` physical and `Activated | NotActivated | Unknown` binding
   projections. Invalid products are unrepresentable, every unknown installs an
   explicit fence, and only `Durable + Activated` permits a stable binding change.
   The workspace now retains the locator-free authority evidence across native task
   retirement and constructs this receipt from the retained operation's actual
   physical, binding, and lifecycle state. In-flight work always carries a retained
   task fence; write or activation uncertainty adds its corresponding fence; only
   release plus retirement produces `TerminalReplayable`. A changed destination is
   covered by a target-neutral test that observes the original task without retrying
   the write or changing the binding. Content-free workspace gauges expose bounded
   capacity, issued/retained/live/fenced tasks, physical and binding classes, and
   completion. Service-level RED counters and stage latency remain required before
   protocol projection.
5. The durable request journal must encode the eventual command and receipt before
   wire publication. A crash after the external write but before a completed journal
   record must replay/fence as unknown; it must never repeat the overwrite under a
   new request identity. The in-process service now resolves Save As through the
   stable protocol document projection and advances the existing workspace-identity
   metadata generation exactly once when the private binding commits. Consequently,
   a composition can detect and durably checkpoint the changed locator association
   before considering a receipt publishable. Release and retirement fences have
   explicit reconciliation calls. The durable operation intent/completion join and
   crash-recovery policy are still not connected, so no public command is admitted.

Until all five laws are implemented together, no command tag, client convenience
method, wire fixture, daemon flag, or MCP tool is advertised. In particular,
`--allow-save` remains ordinary Persist-only and MCP remains unable to widen it by
accepting a destination string.

## Required public receipt

The eventual bounded receipt must retain, without returning a filesystem locator:

- the exact client source witness and workspace document identity;
- the host destination-policy decision witness and requested disposition;
- separate Persist and Rebind decision witnesses, bound to the same request;
- closed physical persistence evidence (`Durable`, `NotDurable`, `Unknown`, or
  `NotAttempted`);
- closed binding evidence (`Activated`, `NotActivated`, or `Unknown`); and
- whether the retained task is terminal/replayable or fenced for reconciliation.

Only `Durable + Activated` may change the workspace's stable locator association.
`Durable + NotActivated` means the destination bytes may be externally visible while
the original binding remains authoritative. Any `Unknown` component forbids an
automatic retry or implicit locator change.

## Security, privacy, and operations

STRIDE controls require exact host-installed destination policy, symlink/type/link and
identity revalidation, independent Persist/Rebind checks, request replay protection,
and no implicit overwrite or create. The unavoidable residual is a same-user or
external-writer race outside the qualified local writer; the contract must keep that
as conflict/unknown rather than claim atomic compare-and-swap.

LINDDUN controls require locator redaction from debug, metrics, wire receipts, and
MCP results. Host policy necessarily observes the literal path, and a durable request
journal would retain it; at-rest encryption and unlinkability are not claimed. Metrics
should expose content-free RED counters for attempts, policy denied/unavailable,
physical durability class, activation class, replay/conflict, and fences. USE signals
should cover retained export-task and journal capacity. Golden-signal latency must
separate policy, write/synchronization, and activation stages so an external write is
never hidden behind one aggregate “save failed” counter.
