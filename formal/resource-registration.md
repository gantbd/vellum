# Native resource registration metadata

Current integration: §12.3.3 separately supplies actual native resource-catalog
full-state observation, original/current policy intersections, opaque logical-slot
references and exact historical-head release with gap/resynchronization/protected
lifetimes; see [registration-events.md](registration-events.md). It does not supply
a lossless activation audit, command/schema discovery or policy-grant mutation.
Earlier counts/exclusions retain their slice scope; wire/remote, host/measurement,
physical/concurrent/crash and independent gates remain.

Earlier integration: §22.3.6 supplies the separate owned scoped native adapter,
current basis/scope/profile revalidation and stamp-free indexed receipt; see
[registration-scoped-invocations.md](registration-scoped-invocations.md). Shared
cancellation now covers 21 forms/eleven kinds, 336 gate/168 context cases, 42 races
and 5,439 traces/15,246 edges, preserving ten controls/twelve mutants. Earlier
counts/exclusions retain their slice scope. Wire/remote, full discovery/events,
host/measurement/physical/concurrent/crash and independent gates remain.

Earlier integration: §22.3.5 supplies separate native scoped conditional basis
authority, including independent presence/equality permissions and distinct
present-row/current-absence meanings; see [registration-bases.md](registration-bases.md).
It does not turn F07 selectors into epochs or complete the scoped F16 mutation
adapter. Earlier counts/exclusions retain their slice scope; public wire, host,
measurement, physical/concurrent/crash and independent gates remain.

Current read integration: §22.2.2 adds scoped F07 captured-permitted metadata,
actual-root visibility and opaque read selectors without native stamps/identities;
see [registration-reads.md](registration-reads.md). Selectors are not native epoch
or mutation preconditions. Earlier native/public exclusions retain their slice scope.

Current invocation integration: §22.3.4 supplies owned F16 registration intent,
current native staging and atomic local floor/root/outcome/receipt settlement;
see [registration-invocations.md](registration-invocations.md). It adds a private
registration job kind, not new row semantics. Native stamp receipts remain
trusted-host-only; scoped public encoding/events remain gates. Earlier counts and
invocation exclusions below retain their original native-only scope.

Current logical container integration: §11.2.11 adds actual native view/workspace
birth and retirement, separately from document disposal and retained producer work;
see [relation-lifecycle.md](relation-lifecycle.md). Earlier registration exclusions
retain their slice scope; public products, complete consumer/host truth, recovery
and physical/concurrent/crash/independent assurance remain gates.

Specification §22.3.3 adds atomic native activation of the existing source-read,
annotation-read and write-resource descriptor tables. It refines existing
registration generations, exact current dependencies and IF-07/IF-10 activation;
it is not provider execution, ownership transfer or a new public command/job.

## Precedent and derivation

[LSP 3.17](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/lsp/3.17/specification.md)
allows capability registration/unregistration after initialization.
[Neovim's command API](https://neovim.io/doc/user/api/#api-command) exposes command
creation/deletion and an explicit replacement option. These are precedent for
mutable registration metadata, not proofs of Vellum's lifetime, identity or effect
semantics. Neither protocol nor Neovim's default replacement behavior is adopted.

Vellum derives stronger current-basis checks from its existing D01/D03/D07 and
staged-activation contracts: explicit present/absent expectations, exact current
generations, complete bounded rows, one atomic batch, no implicit bindings/grants,
and immutable attribution for already admitted work. Registration withdrawal does
not claim that a provider stopped or that an unknown effect disappeared.

## Closed native product

The trusted control takes exactly {changes}. Each change selects table read or
write, action install or remove, explicit provider/resource identity and expected
{state: absent} or {state: present, generation}. A present generation can be zero;
zero is not absence. Install additionally carries configuration: the existing row
fields except provider/resource/generation. The engine supplies the fresh stamp.
Unknown fields, actions, tables, profiles, duplicate slots and malformed members
refuse the entire batch before publication.

Read rows are exactly resource-read/1's strict-utf8-one-bom/1 product, including
explicit LF/CRLF/CR insertion convention, or annotation-resource-read/1's
vellum.annotations/1 product. Write rows use source-save/1's existing complete
descriptor and independent Boolean capabilities. A row with identityKnown or
logicalContinuity false remains such; activation does not promote a declaration
to truth or authorize a dependent operation. Read and write slots for one resource
are distinct, with independent current stamps.

Fourteen strict current host facts cover authorization/scope/continuity/liveness/
stop, registration permission, identity, complete registry coverage, companions,
versions/reply/resources and owner/preparation control. The existing native
lifecycle/membership/bindings profile bounds both roots and the batch; maxRelations
bounds rows/changes, maxTagBytes bounds identity/configuration labels, and the
actual native maxResultBytes bounds the reply. Current supported catalogs must be
explicit and well formed; omitted or unknown legacy inventories are not empty.

The result is {status: applied | unchanged, registrations}, with one copied
{table, provider, resource, generation} per requested slot. Generation is the
resulting row's stamp or null for absence. It contains no unrequested rows, paths
or configuration payloads. This is a trusted semantic result, not a frozen Rust,
wire, discovery or recipient-projection schema.

The numeric stamp is an internal fence, not permission to expose a global activity
counter. A future scoped public reference/registration encoding must preserve the
existing disclosure and non-linkability obligations; this slice does not authorize
returning raw owner-wide stamps to unprivileged remote recipients.

## Generation floor and atomic frame

The actual native owner keeps one private monotone registrationFloor. Its first
activation seeds it from the maximum generation in the known initial current
read/write tables. Every changed install in one activation receives floor + 1;
the same fresh batch stamp can label different addressed slots. Unchanged rows
keep their existing generation. Removal and validated unchanged batches consume
no fresh stamp and never lower the floor. Overflow refuses new stamps; there is
no wrap, recycling or silent engine-incarnation reset.

Per-key counters would require retained tombstones after removal, or an additional
nonreuse oracle. One owner-wide floor avoids that growing key history under the
existing serialized native owner. Its tradeoff is shared stamp-space exhaustion.
This is an engine-incarnation-local construction, not durable recovery or adoption
of unknown/imported/legacy registration lifetimes. Initial native consumers capture
the known current descriptors, and subsequent catalog changes use this owner; a
host cannot bypass the construction with out-of-band mutation and retain its proof.

Preparation computes an owned immutable request/profile and bounded reply against
the actual domain, owner, before-root and floor without changing any state. Final
validation reconstructs the complete candidate, reply and clock delta from actual
current metadata. It rechecks facts, exact ownership/basis, floor, profile limits,
the actual native root cap and current baseline promises. Equal roots for unchanged
success must be the same root pointer, not a byte-equal replacement.

One synchronous commit consumes the certificate, updates the floor and installs
the frozen root. A changed batch advances publication once, changing only
resourceRegistrations/writeRegistrations; all authored data, ordinary bindings,
owners, histories, views, workspaces and unrelated host metadata survive. The
existing source-publication journal receipts carry without reissuance. Reply
materialization precedes the marker commit. No provider callback or await
intervenes; preparationReserved remains a host resource premise, not allocator,
RSS, stack, crash or real-concurrency refinement.

## Existing work and capability truth

Native consumers retain their exact captured registrations. Replacing/removing a
row makes old queued submission/dispatch/activation fail the owning current-row
checks; it does not rewrite the task, source/baseline, binding or payload. Existing
dispatched write knowledge and late read transcripts remain attributable to their
original capture. Old baseline publication cannot borrow a new registration.
Historical successful load evidence remains historical even if the read descriptor
changes; read metadata does not coerce an unchanged write registration's generation.

Registration is neither source/annotation ownership nor provider control. Existing
resourceOwners, ordinary bindings, reader/writer/codec records, origins, effect
barriers, retention pins and baseline promises remain. An ordinary association can
be stale after a metadata change; its owning rebind/activation contract is separate.
No force rebind, alias inference, cancellation, worker stop, charge refund, producer
discharge or automatic physical revocation is performed.

## Conditional preservation and termination arguments

Assume supported normalized bounded catalogs; the existing single native root
owner and consumer contracts; no imported native lifetimes/out-of-band registry
mutation; truthful current identity/permission/coverage/preparation facts; and
serialized synchronous composition without reentrant callbacks.

1. Initially every current native registration generation is at most floor.
   Changed installs choose floor + 1 and atomically preserve that maximum as the
   new floor. Removal, unchanged activation and other native publications do not
   lower it. Induction over these transitions gives a fresh stamp strictly greater
   than every previously issued native stamp in this incarnation. A removed slot
   therefore cannot revive an old exact-registration request by reinstalling it.
2. Complete per-slot expectations and uniqueness precede any root change. Candidate,
   result and clock reconstruction plus exact owner/domain/before-root checks
   either refuse wholly or install one coherent batch. Exact configuration identity
   yields unchanged success without consuming counters; source/annotation/owner/
   relation frames and journal carry exclude accidental content or authority moves.
3. The existing consumers compare full captured/current registrations before new
   work/activation. Since catalog control changes none of their retained stores,
   late authenticated evidence keeps its original attribution and unresolved effect
   or retention obligations remain charged. Current metadata is not effect evidence.
4. The two tables/two actions/three existing row products are closed. Every call
   completes using bounded validation, copies and scans; a certificate commits or
   is discarded. This is per-call termination and a counter nonreuse argument, not
   provider termination, whole-editor categorical closure or a bounded-lattice theorem
   for replacing arbitrary registration values.

These are authored conditional induction/construction arguments, not machine-
checked induction, complete temporal-module refinement or independent certification.

## Executable evidence and observation

resource-registration-checks.mjs covers 42 annotation/history stages and 126 actual
native metadata publications, all 16,384 current-fact vectors, 42 strict-value
refusals and twenty-nine controls. Actual native source read/write consumers,
historical evidence, independently stamped read/write slots, zero/MAX boundaries,
remove/reinstall, no-op identity, profile/native/reply/promise caps, current frames,
copied/foreign/spent certificates, fault supervision and private clock continuity
are exercised. Thirty semantic mutants are checked in two exact fifteen-case banks,
each with a passing coherently rewired multimodule baseline. The harness checks both
ordered partitions against the complete thirty-name manifest; no variant is dropped.
Redundant identity/basis/envelope guards are weakened
together where necessary; syntax/import/TypeErrors do not count as kills. Existing
234 SMT conditions/fourteen manifests are unchanged.

IF-07/IF-10, DAT-09, PATH-01/05/10/12 and INS-01/02/03/04/08/09/12/13 distinguish
catalog admission, applied/unchanged/refused activation, stamp issuance/exhaustion,
consumer staleness and retained late evidence. MELT/RED/USE/golden signals and
STRIDE/LINDDUN cover stale/forged metadata, unsafe revocation inference, ownership
substitution, capacity pressure and registry-existence leakage. Labels use bounded
table/row-kind/action/phase/reason categories, never raw paths/configuration,
provider/resource IDs, keys, locators, origins, generations or tokens. Installed
instrumentation, costs and numerical objectives remain implementation/measurement.

View/workspace/command/provider-callback registration lifecycles, full public
invocation/discovery/result/event products, imported/legacy/multi-registry or
contested recovery, remaining all-profile binding/history transfers, actual host/
provider/package/identity/policy/coverage/retention truth, complete global/physical
accounting, real concurrency/crash refinement and independent assurance remain
gates. No whole D20/D28/D33/F17/F20/F21 or all-sections goal closes.
