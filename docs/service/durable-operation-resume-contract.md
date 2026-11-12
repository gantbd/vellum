# Durable operation resume contract

Status: native protocol, canonical wire, transport, client, authenticated daemon
composition, local credential profile, and authenticated MCP projection and
restart qualification implemented. The shipped MCP CLI selects cross-process
resume only with an explicit credential path; a separate subprocess scenario
for both credential-enabled binaries remains a release qualification gap.

The persistent operation journal proves that an authenticated daemon can retain
an exact effect receipt across restart. The public client and authenticated MCP
library projection expose separate execute and lookup operations while every
new connection allocates a fresh client identity and request sequence. The
older MCP `vo1` uncertainty handle remains limited to one stdio process
lifetime and is not reinterpreted as this durable identity.

Product recovery must not expose “execute this historical request” as its
resume operation. An old or forged client/request pair could otherwise fall
through an absent journal entry into ordinary fresh execution. Resume is a
separate read-only lookup whose absence path has no editor, filesystem,
membership, authority or journal-creation effect.

The implemented native intent is a separate closed algebra: membership effects
carry their exact locator or target, while save and replace-and-save carry an
explicit workspace-scoped document and source revision. It contains no selected
view or `VersionWitness`. Returned durable receipts are restart-stable
historical evidence and contain no live witness.

## Identity and frozen intent

Before the first effect attempt, the caller supplies a fixed-width opaque
`DurableOperationRefV1` generated from an OS CSPRNG by its trusted composition.
The reference is correlation, not authority, authentication evidence or a
credential. It is scoped by the stable workspace identity and authenticated
subject and is never derived from a client ID, request ID, path, source text,
timestamp or process ID. Workspace membership generation is monotone provenance
validated during journal recovery; it is not a receipt-lookup discriminator,
because the membership effect itself may advance that generation.

The daemon durably binds that reference to:

- the restart-stable workspace identity plus the generation at which the
  operation intent was synchronized;
- the exact authenticated subject reference;
- the canonical durable-effect request bytes, including the explicit target
  and every effect argument but excluding ephemeral transport client/request
  correlation; and
- after completion, the exact canonical reply bytes.

Persistence keys each operation by stable workspace identity, authenticated
subject and operation reference. Different subjects may independently choose
the same opaque reference without learning about or blocking one another. The
local adapter stores this relation as `.vellum-durable-operation-v2`; presence
of the superseded operation-v1 directory refuses recovery and new writes until
an explicit migration exists. It is never silently reinterpreted.

Transport `ClientId` and `RequestId` retain their current session-local replay
meaning. They do not continue across process restart and are not accepted as a
durable operation identity. Reconnecting creates ordinary fresh transport
correlation while the durable operation reference remains unchanged.

## Read-only lookup algebra

Receipt lookup accepts the authenticated subject, stable workspace identity,
opaque operation reference and the same canonical durable-effect request bytes.
The journal has already established that the record generation is not from the
future; lookup deliberately does not require equality with the later membership
generation. It is total over the following closed outcomes:

- `Complete(exact_reply)`: the subject, workspace, operation and request bytes
  match one complete record. The frozen reply is returned without executing,
  authorizing or publishing anything.
- `RequestConflict`: the same subject/workspace/operation is bound to different
  exact request bytes. No new intent or effect is created.
- `OutcomeUnknown`: a matching intent exists without a provable complete
  receipt. The operation remains fenced; lookup never retries it.
- `AbsentOrForeign`: no matching record is disclosable to this subject and
  workspace. Absence, another subject's record and another workspace's record
  are intentionally indistinguishable.
- `Unavailable`: safe journal inspection or exact decoding could not be
  established. No weaker answer is inferred.

Lookup never consults action authority. It discloses historical evidence for
an effect already mediated under the frozen decision; it is not fresh action
authorization. Dynamic revocation, expiry and deletion require a separate
retention/disclosure policy and must not be simulated by reauthorizing and then
misreporting a completed effect as absent.

## SDK and MCP projection

The public client exposes distinct APIs for `execute_durable` and
`lookup_durable_receipt`. Only the first may create an intent or execute an
effect. The second maps only to the read-only native lookup above; it does not
call `ServicePort::execute` with a historical client identity or fall back to a
fresh operation after `AbsentOrForeign`.

The authenticated MCP library projection exposes additive
`vellum.durable_execute` and structurally separate read-only
`vellum.durable_receipt` tools. `McpServer::new` neither advertises nor accepts
them; enabling them requires the authenticated connector composition through
`McpServer::new_authenticated`. The handle is the canonical `vdo1.` encoding of
only the nonzero 128-bit opaque operation reference. It contains no subject,
credential, authority, client/request ID, path, source, request arguments or
reply. Possession is not sufficient: the MCP peer resubmits the identical
explicit tool arguments so the native layer can compare the exact canonical
request bytes. Changed arguments produce `RequestConflict`; another subject or
workspace sees the same `AbsentOrForeign` result as true absence. Handles are
not enumerable, and no list-receipts tool is implied.

To make a lost MCP reply recoverable even if the stdio process also exits, the
harness must choose and persist the operation reference before the first tool
call. A handle minted only after an `outcome_unknown` response preserves the
current same-process retry but cannot make a cross-process durability claim.

The authenticated library projection is qualified with a real daemon across
both MCP-client and daemon restart: the first MCP reply is discarded, the
historical receipt is retrieved after restart, and the restarted service proves
zero second effects. Changed-intent conflict, absent lookup, and indistinguishable
foreign-subject absence are qualified in the same composition through the
production descriptor-anchored local credential adapter.

The `vellum-mcp` binary uses the owner-only connector by default and its
`vo1` handle is explicitly process-local. It exposes the authenticated durable
tools only when `--credential` selects the required-auth profile and the caller
supplies a 128-bit operation reference persisted before the first write;
Vellum does not invent credentials or relabel `vo1` as durable.

## Required qualification

- The authenticated MCP library composition completes an effect, discards the
  MCP reply, terminates both daemon and MCP client, restarts them, and retrieves
  the historical receipt without a second filesystem write or membership
  transition.
- Prove that `AbsentOrForeign`, malformed handles, changed exact request bytes,
  pending intent, journal refusal and decode failure never execute an effect.
- Prove that a new transport client/request sequence can retrieve a receipt and
  that historical transport identities alone cannot retrieve or execute it.
- Prove same-subject exact retrieval, same-subject conflict, cross-subject and
  cross-workspace non-disclosure, fixed capacity with no invisible eviction,
  and content-free errors, metrics and operational events.
- Still required: exercise the shipped daemon and MCP binaries together through
  the authenticated local composition; the production adapter-backed in-process
  scenario and separate binary help/install smokes do not alone close that
  product contract.

This contract does not claim remote idempotency, distributed exactly-once,
credential strength, Aegis authorization, encrypted journal storage, durable
audit/non-repudiation, automatic pending-effect reconciliation or receipt
retention/deletion policy.

The local journal is private plaintext, not authenticated storage. Descriptor-
relative access and namespace revalidation prevent a path replacement from
redirecting an operation, but the v1/v2 exclusion checks assume cooperative
ownership by processes with the same effective user. Fully excluding a
malicious same-UID process that transiently creates and removes a competing
version name requires one enforced shared lock or one atomically versioned
namespace; the current local security profile does not claim same-UID process
isolation.

Recovered admission receipts are checked for canonical shape, workspace,
intent/receipt correspondence, and the v3 identity record's exact issued prefix
for the correct document/root sort. This includes historical removed targets,
retired-operation tombstones, optional file roots, and every document in a
directory-admission receipt. Reservation ceilings remain capacity evidence only.
Legacy v1/v2 identity records refuse rather than being relabeled as issued
evidence.
