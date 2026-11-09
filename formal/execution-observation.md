# Execution status and bounded observation

Section 23.1.4 separately connects actual admitted native invocation state, owned
context/reference issuance and bounded waits without a replay fixture producer;
see [invocation-observation.md](invocation-observation.md). The older checker below
retains its own scope and exclusions. Global host reservations, real policy/clock/
retention, delegated/all-effect gateways, public wire and independent refinement
remain separate gates.

Specification §23.1.2 selects F06's status, retained-outcome, bounded-observation-
wait and minimal-control-status products. Observation does not execute a request,
refresh its intent, authorize retry, cancel its producer or discharge its effects.
Ending a waiter closes that observer, not the admitted operation.

## Precedent and selected boundaries

[Erlang processes and monitors](https://www.erlang.org/doc/system/ref_man_processes.html#monitors)
separate unidirectional monitoring from linked process lifetime. Process aliases
also let a receiver deactivate request/reply routing. These inform observation
ownership and late-delivery fencing, not an Erlang runtime dependency or permission
to discard authoritative late provider evidence. Such evidence still enters
Vellum's protected evidence owner after the original observer disappears.

The semantic field catalog is execution-observation.json. Common request fields
bind contract/variant, exact admitted invocation identity, owner-resolved observation
context and execution/release profile. A bounded wait also names a previously
issued scoped observation reference and a finite observation budget. Observation
references are not global sequence counters or command grants. Contexts and waiters
cannot alias another invocation, engine incarnation or reused registry generation.

Status reports a permitted pending/terminal control observation with an opaque
observation reference and currently releasable retained facts where available.
Terminal bookkeeping is not proof of a complete retained outcome or effect
quiescence. Retained-outcome returns the currently projected authoritative layered
facts; a missing/forgotten outcome is unavailable, not inferred nonapplication.
Earlier delivered reports keep their as-observed meaning after later retention
loss. Revoking write authority does not rewrite a retained Applied fact, although
current observation rights still control its release.

Minimal control status uses reserved control capacity and only the permitted
control observation, never the ordinary rich result. It is not gated on ordinary
speculative capacity, but still requires exact target, current observation rights,
projection, continuity and protected transfer bounds. Protected exhaustion is
unavailable, not an unbounded allocation or a fabricated outcome.

Ordinary status/outcome/wait use their admitted ordinary observation resources.
All release checks current recipient, purpose/destination, invocation/context,
profile, policy/continuity and actual retention/liveness. Rich fields use the
captured ceiling intersected with current permissions. Broadening cannot enlarge
an old capture; withholding never changes the authoritative outcome. Private
causes/participants are projected before reason/count/correlation/recovery release.
Contradictory retained facts do not become a guessed successful projection.

## Wait lifecycle and progress

A waiter belongs to one observer/producer identity and one captured release
ceiling. Polling observes a permitted change or terminal state, remains waiting,
or closes unavailable. Delivery, expiry, owner stop and invalidation consume its
eligibility: old/copy handles cannot deliver again, retarget a slot or revive after
regrant. F19's existing protected owner stop closes observation without requiring
current content disclosure rights; it does not enter producer cancellation.

The profile bounds real wait registrations/holds, complete reply transfer, budget
and clock/service work before admission. Stop/expiry releases only the waiter's
actual holds after owner discharge, not producer, provider or inverse obligations.
Observing a result neither confirms external persistence nor licenses safe replay.
Late completion can still update authoritative facts after the waiter closes.

The reference's private tick budget decreases to cutoff; it is not a millisecond
default or proof of a host timer. Poll alone is a self-loop and cannot prove time
progress. Actual monotonic clock/epoch, suspend, unavailable-clock and dispatch/
deadline ordering follow the named §17.4 profile. A valid clock/service profile is
required before promising bounded live waiting. Trace ordering represents possible
serialized event orders, not a universal completion-versus-expiry priority. No
native-worker preemption or unconditional scheduler fairness is established.

## Executable evidence and limits

execution-observation.mjs composes the actual replay-reference step relation and
outcome-projection capture/release functions. It checks 192 release combinations,
64 ceiling/private-participant comparisons, four direct wait actions and 56 controls.
It explores all 2,801 traces of at most four events from poll/tick/expire/stop/
complete/revoke/restore, including 6,136 observation edges. Observation edges preserve
producer state, publication/execution counts and unresolved effect obligations;
closed waiters cannot resume delivery. This is a bounded trace exploration, not
unbounded exhaustive concurrency or a proof of host-clock progress.

Six in-memory mutants must load and fail assertions: expand the captured outcome,
put rich facts into protected status, require ordinary capacity for protected status,
release forgotten outcomes, retain a closed waiter and ignore invocation generation.
Generation controls include a newly pending execution in the reused slot, so a
wrong-generation guard cannot pass merely because the replacement slot is empty.
Complete observed-reply bounds include the outer envelope, not just nested facts.

The replay fixture has two epochs/two generations/one slot; actual registry identity,
retention, authoritative layered fact truth, current host policy, timer delivery,
physical accounting and hold discharge remain premises or integration gates.
WeakMap provenance fences actual local waiter objects, not serialized capability
entropy, native ownership or an isolation boundary. Native codecs, actual observation
reference/context issuance and disposal, full invocation/publication/provider
composition, live clock/scheduler behavior and measured guarantees remain open.
Same-author checks are neither independent Cleanroom certification nor new solver
theorems. Point representation is approved in §29.4.1; native persistence integration is a separate evidence gate.

## Instrumentation

INS-01 distinguishes admission/execution from observation lookup and bounded wait;
INS-03 observes current projection and actual recipient handoff; INS-04 preserves
provider/effect evidence separately; INS-08 accounts observer versus producer holds;
INS-13 observes cutoff/protected stop; INS-12 records diagnostic loss independently.
MELT/golden/RED distinguish status/outcome/minimal/wait populations, reply availability,
cutoff and actual execution completion. Pending age and censoring cannot disappear
from latency evidence when a waiter ends. USE applies to real ordinary/control
queues, retained outcomes and outstanding holds. STRIDE/LINDDUN challenge forged
identities, reused generations, replay leakage, private participants and captured
ceiling expansion. STPA/FMEA challenge timeout-as-nonapplication, premature refund,
lost late evidence and closed-wait resurrection. Observation identifiers and hidden
obligation counts are not ordinary metric labels.
