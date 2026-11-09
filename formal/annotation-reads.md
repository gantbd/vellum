# Native annotation edits and scoped current reads

Specification §12.8.2 composes actual normalized F13 edits and same-ledger F14
undo/redo with the existing F02 current-capture and F03 enumeration, inspection
and target-location paths. No new verb or implicit reference conversion is needed.
The engine keeps three different meanings: historical writer decisions, recipient
stream observations and independently retained read captures.

## Precedent and contract

[Neovim buffer update events](https://neovim.io/doc/user/api/#api-buffer-updates)
and [buffer/namespace/id extmark lookup](https://neovim.io/doc/user/api/#nvim_buf_get_extmark_by_id())
separate notification from later scoped lookup. This supports the architectural
separation, not Vellum's authorization, immutable snapshot or authored membership
history guarantees. Those follow the adopted Vellum contracts in §§12.8.1/12.9,
13.6.3/13.6.4 and 13.9.1, under their explicit premises.

An admitted native edit installs its local root and writer facts. A later F02
capture-current reads the actual root at its own serialized capture point, even
when the observer has not yet queued or delivered that edit. Fresh complete owned
visibility evidence, observer scope, facet ceilings, current grants and independent
read-registry admission are still required. Reader denial cannot undo the edit or
alter the writer receipt. A writer receipt grant cannot authorize a reader capture;
the reader can also receive an authorized capture when writer details are withheld.

F03 enumeration issues context-bound query selectors. Native annotation references,
writer publication anchors, stream generations and stream annotation references are
not accepted as context/record selectors. No event-to-query or receipt-to-query
conversion is provided. The absence of conversion is intentional, not a missing
requirement to reconstruct an old root. Public wire representations remain open.

Body edits and reattachment capture the actual current merged fields and mapped
placement. All six already declared target forms apply: document, range and points
under positional/content-sensitive profiles with either affinity. This does not
select their unresolved on-disk point encoding. Native create/delete and undo/redo
change current membership; same-entity restoration uses advanced authored version
floors. An older independently retained read still contains its original permitted
membership and values, including when the current collection is empty. A previously
empty read cannot become populated by later restoration.

Current query release checks the retained context and current scope/grant/liveness/
continuity/transfer/reply capacity. Newly acquired contexts cannot accept old-context
record selectors. Captured body/target facets cannot be widened. Annotation-only
capture never confers source access, and a combined request cannot silently omit
the denied source class. Fresh read preparation neither installs stream bindings
nor advances stream generations or queues events. Capacity refusal installs no
partial query selectors and does not roll back authoritative publication.

Receipt eviction, observer stop and read-context closure have separate owners and
effects. Evicting a receipt does not invalidate an otherwise authorized read. Stopping
an observer prevents new captures through that observer but does not close existing
reads; closing a read retires its own selectors. A stream gap refuses this composed
capture path until explicit recovery, while existing read release remains governed
by its own current gates. Independent ordinary F02 acquisition remains independent
of subscriptions as already specified in §12.8.1.

## Conditional preservation argument

Assume the existing serialized owner, non-confusable identities, truthful current
policy/visibility/order, complete actual root capture and adequate bounded ownership/
retention reservations. The native publisher and history operators establish the
current authored root; the result cache separately retains the original writer facts.
The existing shared projector filters current rows by authorized existence and
captured/current body/target ceilings. F02 copies that projection into an independently
admitted read context. F03 can issue selectors only for that context's captured rows.

The base case is successful capture: its values equal that permitted current
projection and its selectors belong only to the installed context. For a later edit
or history step, the authored root changes but the retained read and writer receipt
do not. For query release, exact current gates bound disclosure by the capture;
for context closure, selector lookup becomes unavailable. Receipt eviction and
observer stop have no transition that mutates the independent read capture. Read
refusal changes neither authored root nor observer state. Induction over these
selected serialized transitions preserves separation and non-amplification, subject
to truthful premises; it is not a proof-assistant-checked implementation induction.

Finite native preparations, record projections and registry queries use the existing
bounded arrays and closed operation cases. Pagination strictly advances to a finite
captured row bound. No callback, network request or program interpreter is added.
These arguments do not prove hostile decoder termination, actual native scheduling,
concurrent installation, physical allocation or arbitrary providers' termination.

## Executable evidence

annotation-read-checks.mjs runs the actual native producer, shared invocation owner,
native reversal, observer projector, F02 bridge/read registry and F03 query helpers.
Its expected values come from the actual authored record/placement state, not a
second implementation of read projection. The suite includes:

- 448 combinations of native operations, targets, captured/current facets and visibility,
  followed by 896 undo/redo stages;
- 2,048 read-admission and 8,192 observer-release combinations over four operations;
- 32 hidden-world pairs across the four native operation variants;
- thirteen controls for product separation, scope, source mapping, field merging, restoration,
  source denial, capacity, independent lifetimes, copying, authority and gap recovery;
- fourteen assertion-rejected semantic mutants, with a passing unmodified dynamically
  loaded composition. Import/load errors do not count as mutation detection.

The scope is selected native integration, not exhaustive reachable histories or
all F02/F03/F13/F14/F19 products. At this read batch there were 205 SMT conditions
across thirteen manifests; this integration adds no solver condition or global lattice
claim. The earlier normalized read and observer checkers retain their own narrower
scope flags. Passing composition does not authenticate host premises.

## Instrumentation and remaining gates

PATH-01/02/03/04/12 and IF-01/02/03/08/09/12 distinguish authored publication,
writer receipt release, stream delivery/recovery, current read capture, query selector
installation, query release and retirement. INS-01/02 count authored decisions;
INS-03 read/stream projection and release; INS-06/08 retained history/read obligations;
INS-07 ownership discharge; INS-12 diagnostic loss and INS-13 useful delivery/control.
Read installation is not another authored publication and read refusal is not a
failed edit. A successfully applied edit with unavailable reader delivery must not
be counted as successful query completion.

MELT/RED/golden signals distinguish phase latency, useful delivery, denial, gaps,
errors and saturation. USE covers read contexts, selectors, units, temporary projection
bindings and receipt/history ownership independently. STRIDE challenges cross-kind/
cross-context substitution, stale policy, false rollback and hidden membership;
LINDDUN covers linkage across writer/stream/read namespaces and misattributed old
versus current data. Content, raw identities, selectors, invocations and generation
values are not ordinary telemetry labels. Observation does not grant authority.

Actual issuer/policy/visibility/order/retention truth, public wire/alias bridges,
historical event lookup, all lifecycle/multi-document products, full retirement/
general target restoration/point persistence and other result/effect gateways remain.
Physical/concurrent/native/decoder/timing/resource/production and independent
assurance gates are unchanged. No entire family, decision row or all-sections goal
is closed by this native read composition alone.
