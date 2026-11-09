# Identity residency and disposal baseline

D07/D08 refine specification §§11.1.1/11.3.1. Logical incarnation, resource binding
and provider continuity remain separate; admitted authored state has an explicit
disposal boundary. Unknown identity evidence is not known distinction, and a
saved source does not establish a safe disposition of unsaved annotations.

## Conditional preservation arguments

Assume a serialized registry initially has at most one writable owner per certified
key in one declared comparison domain. Activation rechecks that registry: a known
alias joins without changing its root; a distinct key creates at most one owner
after authority/capacity checks. Induction over activations preserves per-key
uniqueness. Unknown/incomparable evidence cannot establish a required stronger
guarantee. This argument requires truthful, stable evidence at the relevant
boundary, not pathname equality or an assumed universal filesystem oracle.
Actual cross-domain alias discovery and physical continuity remain host contracts.

Mutation admission requires a live matching incarnation and current authority.
Disposal makes that incarnation nonlive; a reopen uses a non-confusable new one.
Consequently a stale reference cannot admit mutation after disposal or reopening.
Refuse identity exhaustion before reuse becomes confusable. Actual handle encoding,
generation storage, concurrency and allocator reuse must refine this relation.

Disposal separately checks source and annotation disposition plus every obligation
that would otherwise become invalid. A refusal frames the state; success stops
mutation while preserving required retained obligations. Derived-cache removal
changes only eligible derived state. Obligations discharge only with their own
justified witness, so cancellation/disposal alone cannot reduce their accounting.
Inductively, admitted retained state is not lost through view closure, pressure or
mutation invalidation. Completeness of the obligation inventory, eligibility and
witness truth are supplier premises, not established by checking a Boolean guard.

## Finite executable scope

`identity-residency.mjs` checks 96 supplied-evidence decisions and 1,920 two-request
serialized registry scenarios: four private certified keys, either request order,
independent grants, capacities 1–3 and no existing owner or one existing owner.
It compares exact dispositions and expected owner sets, preserves existing bodies
and checks uniqueness after every activation. The supplied evidence decision
fixture requires comparable known evidence; the separate registry fixture starts
with already certified keys. Their linkage to an actual resolver is unproved.

There are 1,088 private lifecycle states: live/dead, incarnations 1–2, separate dirty
source/annotation bits, four held-obligation bits, derived cache and view/membership
bits. Dead states have no dirty authored state or views/memberships. The Cartesian
checks cover 17,408 disposal decisions, 1,088 cache removals, 6,528 mutation gates,
4,352 reopen decisions and 8,704 single-obligation discharge decisions. Expected
dispositions and frames are checked, including successful transitions; always-refuse
implementations cannot satisfy the reference expectations. Fourteen direct controls
cover unknown/cross-domain evidence, continuity versus identity, aliases/grants,
dirty source/annotations, retained obligations, cancellation and incarnation reuse.

These are enumerated transition/guard fixtures, not exhaustive reachable-trace
exploration or a production FSM/schema. Reopen conservatively waits for all held
obligations in this fixture; the contract permits justified separately retained
obligations and requires only the relevant safe destination disposition. That
fixture restriction is not a new mandatory all-obligations-drained policy.
The obligation bits stand for history, snapshot, worker and effect retention;
they do not model actual payloads, complete history groups or native lifetimes.
Derived state is assumed eligible to drop. Views/memberships are metadata here;
no actual detach/membership operation or resource byte accounting is proved.

Seven `identity-residency.smt2` conditions give four positive local laws (satisfiable
premises followed by an unsatisfiable negation) and three satisfiable controls:
fresh incarnation, disposed mutation denial, complete disposal guard and witnessed
discharge; controls admit incarnation reuse, source-only disposal and cancel refund.
Unbounded mathematical integers and supplied Booleans do not prove finite encoding,
identity uniqueness generation, witness truth or implementation refinement.
The checks use trusted Z3, not independent proof-certificate replay.

## Boundaries instrumentation and evidence gates

IF-10/PATH-10 and INS-08 retain resolution, current owner/binding activation and
disposal observations; INS-04/05/13 retain physical effects, worker retention and
shutdown boundaries. Separate offered opens, joined existing owners, created
owners, refused disposal, logical invalidation and retained obligations. Record
unknown/unsupported evidence without pretending it is a conflict or actual zero.
Measure pending retention age, occupancy and admission pressure without paths,
resource keys, document/principal identities or private workspace topology as
metric labels. Diagnostics do not authorize joining, discard or discharge.

STRIDE covers forged equivalence, stale incarnation, duplicate activation,
discard/retention tampering and resource exhaustion. LINDDUN covers existence,
alias linkage and retention leakage through outcomes/correlation. Current release
projection applies to alias/refusal detail and historical evidence alike.

Actual identity domains/provider tests, encodings, source/annotation disposition
profiles, post-disposal obligation ownership, complete-group retention, physical
accounting and cross-slice implementation refinement remain D07/D08/D09/D14/D15/
D22/D35/D42/D43 gates. No persistence format, durable recovery service, sandbox,
network identity system, automatic eviction or extra ownership-transfer API is
introduced. Existing persistence/derived-retention/replay evidence remains scoped
separately; conjunction alone does not prove their complete composition.

Mechanism precedents are [Neovim typed buffer references and revision-aware APIs](https://neovim.io/doc/user/api/),
[CodeMirror transaction-based state](https://codemirror.net/docs/guide/) and
[Emacs atomic change groups](https://www.gnu.org/software/emacs/manual/html_node/elisp/Atomic-Changes.html).
The strengthened identity/disposal contracts above are Vellum choices, not
guarantees attributed to those editors.
