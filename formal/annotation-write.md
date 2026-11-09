# Native annotation export through the shared writer

Current integration: §16.3.5 adds actual native explicit annotation-export
admission/release through the existing codec and destination owner; see
[native-annotation-export.md](native-annotation-export.md). Annotation disclosure
and source-write capabilities remain separate, and ordinary ownership/baseline/
binding intent is preserved. Earlier counts/exclusions retain their slice scope;
host, public, recovery, physical/concurrent/crash and independent gates remain.


Specification §16.3.2 connects the actual annotation capture/codec to the existing
source destination writer. The selected product is explicit export to a host
supplied destination. It is not ordinary annotation-save baseline publication,
annotation binding activation, loading/restoration or a combined external commit.
No sidecar path, automatic overwrite, runtime callback or public transport is chosen.

## Admission and ownership

`annotation-write.json` defines the normalized reference request and gate sets.
The request names the document incarnation, expected source and annotation-set
versions, existing source-binding generation, exact provider/resource registration,
effect/attempt origin, protection/acknowledgment strength and pinned destination
condition. Present destinations require an explicit overwrite grant and original
version; absence requires a separate exact-true create grant and exclusive-create
capability with no invented version token. Own-chain annotation conditions refuse
in this product; they do not fall back to a pinned or refreshed condition.

Current complete disclosure and write/destination grants are independent. The host
must attest the relation between the normalized capture destination context and
the selected provider/resource (`destinationContextAttested`); a matching string,
locator or write grant does not prove that relation. All-role identity/ownership
coverage and engine-writer coverage remain required host facts. Source-read
permission is not required to disclose annotations and cannot substitute for
annotation disclosure. Source evidence used internally by the actual producer
remains independently trusted evidence, not an exported source body.

Compare the proposed state's document/source bytes/source revision/annotation set
to the actual managed root before invoking native capture. The producer checks
complete inventory, current source/set, context and alias/history evidence again.
This guards stale or mismatched normalized roots; it does not prove host alias or
historical-source truth. The normalized destination-root update remains a candidate
for real atomic installation, not a production coordinator implementation.

Use the exact existing writer owner, task map, origin issuance set, codec and
destination key. Preflight shared task/per-destination/outstanding/held/issuance
budgets, the complete private capture plus basis, minimum evidence envelope and
candidate reservation root before capture. The codec reports the retained byte
length in its bounded capture frame; maximum-output length provides conservative
writer-envelope preflight before actual capture. Successful native capture is the
last fallible admission operation. Refusal does not install a writer task or
destination reservation. Accepted tasks retain the codec artifact privately.

The existing destination reservation holds the document's owner role. Export does
not establish a new annotation binding. Unknown/foreign/duplicate roles refuse.
An explicit same-owner overwrite can target a resolved alias of another engine
artifact only under the separate destination/disclosure/overwrite grants. Such a
write supplies no claim that old source bytes remain externally current.

## Dispatch and evidence

Dispatch uses the same FIFO and unresolved-effect barrier as source writes, keyed
only by host identity domain/key. Workspace, spelling, provider alias and artifact
kind cannot create another lane. Recheck the complete registration, reservation,
original binding context, pinned probe, capabilities, current exact-true grants
and shared outstanding capacity. Later source/comment changes do not recapture
or relabel the frozen annotation bytes. Independent destinations may progress.

The receipt capture has a typed annotation descriptor: source revision,
annotation-set generation and approved format, plus document incarnation, actual
byte count, binding/destination context, original condition, procedure, origin and
dispatch attribution. It has no source serialization/BOM descriptor. Full original
capture basis stays private to the writer's release path; alias tables and original
binding provenance are not provider payload metadata.

Only dispatched, unstopped tasks can release bounded copied chunks. Recheck exact
original basis, current annotation disclosure/resource/retention/transfer/reply
facts and provider/resource recipient. Source payload APIs cannot reinterpret the
annotation artifact. Already delivered bytes are disclosure, not a revocable lease.

The actual common receipt observer validates task/origin/provider/resource/full
capture, envelope bounds and result shape before joining evidence. Annotation
ingress and the relevant result/disposition/quiescence attestation must be exact
true. Reuse the existing application/observed-byte/synchronization/version facts,
duplicate handling and bounded contradiction quarantine. Annotation causal receipts
refuse here; no unsupported causal chain is inferred. Source and annotation tasks
share destination faults as well as ordering and budgets.

The closed phase vocabulary remains queued/dispatched plus independent stopped,
safe-disposition, quiescent and contested facts. Stop prevents subsequent payload
release and queued launch; it neither refunds work nor erases late evidence.
Quiescence releases an outstanding execution slot, not an unresolved effect or
retained artifact. Reservation release needs stopped-never-dispatched or known
uncontested safe disposition. Retirement additionally needs quiescence for a
dispatched task, released reservation and exact-true retention/delivery discharge.
Neither retirement nor queued cancellation recycles origin issuance.

Annotation export cannot activate source SaveAs or supply a source acknowledged
baseline. No annotation baseline is silently published either. The historical
export receipt remains useful evidence; ordinary annotation save requires its own
typed binding/baseline election, hold transfer and current-output comparison.

## Preservation arguments and reference evidence

Under finite validated owners and truthful normalized host facts:

1. Mixed-kind FIFO follows by induction over task insert/stop/dispatch/receipt/
   retirement steps. Both admission paths insert into one map; the actual queue
   head and earlier-effect barrier ignore kind. Kind-specific capture/release
   checks cannot create a second queue or satisfy unresolved application facts.
2. The codec owns copied bytes/basis before task installation. Later native roots
   cannot change that row. Each release checks original basis and independent
   current grants; each receipt compares the complete immutable typed capture.
3. Reservation preflight precedes artifact installation; failed preparation
   returns the original root and no task. The full basis is included in the
   bounded private frame, not assumed free because the request alone fits.
4. The existing effect/worker/retention discharge conditions remain necessary for
   every artifact kind. Removing one installed task/codec row debits its charge
   once and never removes the lifetime origin from the issuance set.
5. Annotation rows are excluded both from source baseline candidate election and
   source baseline construction; annotation export purpose cannot activate source
   SaveAs. Exact descriptor checks prevent a source receipt from becoming an
   annotation receipt. These are separate artifact facts, not separate mutexes.
6. Native capture and each bounded map/registry/byte scan terminate at this
   normalized reference boundary. Host I/O, actual scheduler fairness, physical
   allocation and production encoder behavior remain outside this argument.

The executable checks cover 42 actual native capture/undo/redo cases, 72
single-missing/nonboolean admission checks, 57 dispatch checks, all 2,048 release
masks and twenty controls. Fifteen assertion-detected mutations follow a passing
dynamically rewired baseline. Syntax/import failure is not counted as detection.
Admission/dispatch checks are not an exhaustive cross-product of all host facts.
The source-save, continuity and explicit-source-export suites retain their own
counts and scope, with loaders resolving the actual added imports/catalog.

These checks and conditional arguments are scoped reference evidence, not
independent Cleanroom certification or production refinement. The existing 223
solver conditions/thirteen manifests do not gain new theorems from the tests.

## Precedent and instrumentation

[Neovim write events](https://neovim.io/doc/user/autocmd/#autocmd-events) distinguish
whole-buffer writing from other file writes and assign custom write handlers
responsibility for writing and appropriate modified-state handling. This supports
keeping content-specific output separate from saved-state effects. It does not
prove Vellum's FIFO, identity, zero-trust or durable-receipt contracts. The stricter
shared-writer and typed-evidence rules above are derived from Vellum's adopted
requirements, not a claim to inherit another editor's proof.

Reuse PATH-05, IF-01/13 and INS-01/03/04/07/08/12/13. MELT, RED, USE and golden
signals distinguish capture/refusal, admission/queue age, dispatch/probe refusal,
payload release, acknowledgment strength, unknown/contested effect age, quiescence,
reservation release and retained retirement. Counters cover shared capacity,
issuance and bytes, not just annotation-local throughput. STRIDE includes wrong
kind/recipient, forged receipts, alias-lane bypass, premature refunds and stale
condition promotion; LINDDUN includes destination/context linkability and private
annotation existence. No raw document/body/path/token/basis values are ordinary
labels. Installed telemetry, privacy enforcement and overhead require measurement.

Remaining work includes ordinary annotation binding/baseline/load integration,
combined source/annotation publication and external-write guarantees, retry/
reconciliation and full lifecycle/invocation/replay/events. Actual host alias,
destination-context, policy, coverage, probe and provider attestations remain
premises. Global admission, physical accounting, concurrent root installation,
production implementation and independent assurance remain gates. No whole D20,
F21/F27 family or all-sections goal closes.
