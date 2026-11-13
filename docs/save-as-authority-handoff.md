# Save As authority handoff

Save As now has a portable native/client/wire/MCP operation surface, but no default
service interpreter. Submission therefore fails closed as unavailable until a trusted
harness installs the complete retained lifecycle. Overwriting or exclusively creating
a filesystem destination and changing the document's ordinary binding require explicit
user approval and destination-scoped policy; carrying a destination string is never
authority to write or rebind it.

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
| create allowed | Explicit create-exclusive destination decision; no fallback from a failed overwrite and no default adapter installation |
| disposition readable, reservation release allowed | Current retained-task/control state |

The portable intent distinguishes overwrite-existing from create-exclusive and the
same retained lifecycle carries either effect. Concrete destination probing and
publication still belong to an installed adapter/authority composition; the default
service port supplies neither. No rollback or compensating overwrite may be claimed
after a successful write whose later activation fails.

## Native surface closure review

The editor and qualified substrate implement the important physical sequence: capture
the exact source, reserve a destination under the selected effect, re-probe before the
write, obtain a synchronized write receipt, re-probe again, and activate the binding
separately. The portable surface exposes only submission, observation and receipt
retirement; it deliberately does not expose those privileged lifecycle controls.

The following laws remain release-blocking for installing a runnable service owner:

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
   completion. The service adds saturating RED counters and accumulated monotonic
   latency for the closed Begin, Admission, Persistence, Activation, Release, and
   Retirement stage sum; reconciliation calls remain classified under the stage
   whose obligation they close. These observations never participate in semantics.
5. The durable request journal must encode the eventual command and receipt before
   wire publication. A crash after the external write but before a completed journal
   record must replay/fence as unknown; it must never repeat the overwrite under a
   new request identity. `DurableSaveAsTransactionV1` now joins the existing journal
   and workspace-identity ports in exactly that order: it synchronizes the frozen
   intent before returning authority to execute, accepts only the exact successor
   identity in which the target document locator alone changed, checkpoints that
   identity, and completes the journal only from a terminal replayable receipt that
   proves the stable binding changed. Intermediate crash cuts remain pending and the
   type exposes no retry or effect-execution method. If intent persistence refuses
   after authority/capture but before admission, the retained preparation now has an
   explicit discharge transition: a proven discard closes and reclaims the workspace
   slot, while unknown or failed discharge becomes an absorbing admission fence.
   Release and retirement fences retain explicit reconciliation calls. The service
   now owns this product directly: durable begin joins authority/capture to intent,
   activation joins the private binding commit to the exact metadata checkpoint,
   and completion reads the retained terminal receipt. Journal refusal proves the
   preparation closed or returns its opaque retained operation. Recovery now has a
   total, non-effecting validator: exact owner, canonical request/reply, source
   revision, issued document, recovered destination binding, and terminal receipt
   produce Complete; a synchronized intent without a reply is always absorbing
   OutcomeUnknown. Durable records now carry a bounded canonical envelope with a
   closed `Effect | SaveAs` sort and opaque payload; Save As rejects raw and
   cross-sort bytes. The envelope owns no locator codec, so native path encoding
   remains an adapter concern. The native client/wire/MCP surface now preserves this
   exact `Pending | Fenced | Complete | Retired` lifecycle and recovered
   `OutcomeUnknown`, but the daemon still defaults the methods closed until it owns
   the corresponding authority, journal, metadata and adapter composition.

The additive command tags, client methods and MCP tools are now advertised only by
the durable projection. Older or non-owning service implementations return
`Unavailable`; they cannot fall through to ordinary execution. `--allow-save` remains
ordinary Persist-only, and MCP cannot widen it by accepting a destination string.

## Required public receipt

The bounded receipt retains, without returning a filesystem locator:

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
