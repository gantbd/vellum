# Live native analysis base binding

Specification §18.8 connects §18.7's owned analysis registry to actual retained
Tree-sitter trees in a serialized verification host. Accepted engine snapshots
still contain normalized observations. A private host binding associates each
accepted snapshot with its producing task and retained native forest, without
exposing a native handle in F08. This establishes a scoped native composition,
not a production transport, concurrent parser implementation or physical memory
reclamation proof.

## Owned identities and native execution

native-analysis.mjs owns a live local child process for these checks. A session
is bound to one engine analysis owner and declared provider. Native handle numbers
are monotonic, never recycled, and scoped to that process/session. Only the exact
accepted snapshot object can resolve through its session's private binding map.
An equal-source/equal-context tree in another session is not a substitute. Copied,
unbound and retired refs cannot select a native base. The existing kernel performs
actual current submission checks and acquires its per-analysis lease before the
host sends work. Independent analysis identities may have outstanding tasks,
but this verification worker executes native access serially.

The four verification protocol operations are metadata, execute, drop and close.
Exact top-level fields, increasing request serials, bounded frames and pending
correlation delimit this private test protocol. They are not public engine verbs
or a selected host API. The manifest pins the same native environment as §18.6
and checks installed native/wrapper/query/license fingerprints before and after
every worker operation. The process pipe has no network endpoint. Parent/provider
trust, OS isolation and a hostile public decoder are not established by this pipe.

Each native execute consumes the actual input emitted by submitAnalysisWork,
including source revision, captured generations, origin, source bytes, mode and
owned-base correspondence. The worker checks provider, document/analysis identity,
parse dependency generations, source revision and, for query-only reuse, exact
source bytes. Missing or unsupported bindings yield a failure, not a guessed tree.
Fresh input has no native base. Incremental/unchanged input edits a shallow copy
of the retained host tree and parses the captured source with a fresh parser
instance. The retained original forest's structure and change flags must remain
unchanged. Fresh comparisons check host structure and full mixed observations.

The forest retains the host block tree plus actual inline and independent child
trees keyed by language and exact included ranges. Query-only work uses the exact
same native Tree objects throughout that forest, with no parse or tree edit in
that execution path. Membership and object-identity assertions catch fresh-tree
substitution. Its registered highlight generation selects either the bundled
lexical queries or bundled host queries with a JavaScript identifier query. The
latter is an explicit verification activation, not a production highlighting
default. Other query generations fail as invalid-query. Grammar/discovery/
registry/configuration generation 1 is the tested native registration; unsupported
registrations fail rather than claim compatibility. Native child trees are freshly
parsed on source-changing work; incremental child reuse remains outside this claim.

## Evidence and lifecycle composition

The host correlates the reply to the producing task and captured context/source,
records its result through the actual analysis owner and retains the native forest.
Before accepting/binding a snapshot, analysisResultMatches requires exact equality
with the engine's sole uncontested retained result, under the trusted native-binding
gate. A small-envelope case proves that an unrecorded large native packet cannot
be silently replaced by a smaller, differently authored packet and then bound to
the original native tree. Current authority/source/context gates still govern
acceptance and every F08 release. Native identity is neither edit permission nor
proof of schema/domain meaning.

A completed synchronous worker turn supplies local quiescence evidence separately
from result intake. Cancellation during the await does not release the parser
lease; the returned turn releases work only through the correlated quiescence
path. Cancellation/staleness prevents acceptance without erasing the native
retention obligation. Process loss, a timeout or a transport fault does not invent
quiescence, result or retirement: the session faults, refuses more work and retains
its unresolved host records. The process-death control observes the actual child
exit while the engine task remains held. Restart/reconciliation and production
supervision remain explicit obligations, not an inferred automatic recovery.

Retirement first passes the actual kernel's all-holds/drain/current-control gate.
Logical retirement invalidates the snapshot and fences new borrowers **before**
the asynchronous native drop. The host record remains counted while drop is
pending; a second retire returns retirement-pending without issuing another drop.
Only an acknowledged handle removal releases the host record. Dropped native
handles cannot be dereferenced again. A failure leaves an unresolved host
obligation; it does not restore a logically retired snapshot or claim reclaimed
memory. Closing refuses retained or pending work, and fences new submissions
before awaiting close. Only a normal observed process exit reports closed.

## Conditional argument and executed scope

Under the existing serialized engine guards and trusted provider/host premise:

1. Actual submit creates the only eligible task/capture, and the private session
   map resolves only the specifically accepted native producer. Session-local
   counters plus nonreuse prevent a dropped handle from becoming another tree.
2. Execute either uses that exact native forest, edits its host-tree copy, or
   performs an independently admitted fresh parse. It never selects a tree from
   source equality alone. Result matching plus current engine acceptance binds
   the resulting observations, native producer and accepted snapshot together.
3. Current reads remain engine-only. No native parsing, mutation or pointer
   disclosure occurs in F08; view ownership and source/frame invariants are
   preserved by the existing kernel, exercised with actual native observations.
4. Parent kernel base holds prevent premature retirement. Once logical retirement
   succeeds no new child can acquire that snapshot; the host's pending record
   covers the asynchronous drop window. Close and duplicate-drop fences prevent
   new native work or double release across their awaits.

These are authored conditional trace arguments, not checked induction over the
entire engine. native-analysis-checks.mjs executes fourteen live mixed-source
cases against fresh native observations, fifteen named controls, eleven direct
protocol scenarios and fourteen load-success/assertion-failing mutants. It includes
actual source publication/parser journals, preserved old trees, changed native
queries without reparsing, prefix-hole regions, two-view sharing, independent
contexts, stale/cancelled work, mismatched packets, session identity, held/drop/
close races and actual process death. Protocol scenarios cover wrong provider,
unsupported grammar, altered same-version source, unknown/dropped handles,
monotonic issuance and duplicate drop. Runtime/import/signal/timeout failures do
not count as semantic mutation kills; only explicit assertion failures do.

The verification profile bounds pending frames, issued handles, live host records
and total messages in addition to source/tree/query limits from §§18.5–18.7.
At most 64 execute attempts, 64 productive drops, one metadata and one close fit
within its 256-message ceiling. Failed local base resolution still consumes the
host's finite issuance budget; retirement does not recycle it. Temporary native
parses, object sharing, buffers, allocation failure and actual destructor/RSS costs
are not proved by those logical caps. The 15-second verification guard is not a
native preemption or halting theorem. No multithread/native-lifetime, general
provider authentication, full invocation/event/companion, package/frontend/host
matrix, measured SLO or independent Cleanroom claim follows. Current complete
formal/results.json with matching live/frozen hashes governs execution freshness.

## Instrumentation and prior art

INS-09 covers pinned asset/registration/query generation; INS-02/07 source handoff;
INS-01/05 capture/lease/execute/native-base refusal/packet match/current acceptance;
INS-03 current reads; INS-08 live native handles, pending drops, host-record and
frame/issuance saturation; INS-10/11 later presentation. Separate execution-turn
return, intake, useful current delivery and native drop acknowledgment. Count
partial/stale/cancelled/refused/unknown work separately; track lease wait, base
holds, pending retirement age, session faults, missing quiescence and observation
loss. RED measures attempts/failure/duration; USE measures slots/handles/held
charges; golden signals include useful-delivery latency and saturation. Use local
monotonic clocks and protected correlation events/traces, not document/source/
label/handle/path/hash/provider identities as metric labels. STRIDE includes stale
handles, wrong-provider/base substitution, malformed/duplicate replies and denial
of service. LINDDUN covers document/view linkage through process traces and retained
native metadata. These are probe obligations, not installed telemetry/overhead data.

[Tree.copy/edit](https://tree-sitter.github.io/py-tree-sitter/classes/tree_sitter.Tree.html)
and [advanced parsing](https://tree-sitter.github.io/tree-sitter/using-parsers/3-advanced-parsing.html)
provide the copy/edit/coordinate and simultaneous-use constraints. The
[Parser reset guidance](https://tree-sitter.github.io/py-tree-sitter/classes/tree_sitter.Parser.html)
warns that a timed-out parser can resume prior work. Fresh parser instances avoid
that mutable continuation in this reference; they do not prove every native call
terminates. The retained immutable forest, closed actions and independent current
engine gates implement the adopted supervision/ownership pattern without selecting
a production process topology, runtime or HTTP/network protocol.
