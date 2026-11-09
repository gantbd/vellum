# Closed read acquisition results

Existing F02 snapshot handoff returns either a single context with selected data
classes or a coherent parent with document/member/class rows. Unavailable remains
status-only. The checks add no operation, read mode, provider callback or transport;
discovery /1–/5 remains unchanged.

## Product meaning and identity

Single results have status/context/data. Coherent results have
status/context/participants; each participant has document/snapshot/data. Context
and member references remain original opaque objects, not serialized token
descriptions. A retained copy cannot return its original parent as the new context.
Coherent members retain the requested ordered document roster, distinct member
aliases and class sets. Source/annotation class order is not presentation order:
classes are a nonempty, unique subset of the existing two-class vocabulary and
must equal the requested set. No source stamp or source bytes are added to these
acquisition products.

Native construction checks each returned context against the actual staged
context identity. The coherent owner also checks member aliases against its staged
selectors and compares roster length, documents and classes against the actual
captured candidate, not only a private lower-level reply. Fixed probes forge both
that reply and its purported expected fields to challenge the independent candidate
comparison. These checks occur before cut consumption or native installation.

The facade checks closed values, original request meaning and retained-parent
separation. Shape alone does not authenticate newly issued aliases, prove current
grants or establish that source/annotation data is still retained. Native owners
and existing lifetime/read contracts remain responsible for those facts.

## Failure and resource boundaries

Preinstallation malformed construction faults admission and preserves the native
registry, issuance and prepared cut. A late facade fault after native installation
does not undo the installed context, refund spent issuance or restore a consumed
cut. It fences admission and returns only unavailable while retaining actual native
obligations. The selected late probe checks eighteen installed acquisitions,
including six consumed current annotation/coherent cuts. Authored roots remain
unchanged; the protected finalizer refunds only the existing eight-frame temporary
decoding workspace. Protected retirement and general crash recovery remain
separate obligations.

Validation retains no new context or participant set. Duplicate member detection
uses a bounded quadratic scan with constant auxiliary live state. Native candidate,
registry, selector, reply and temporary decoding reservations remain separately
owned. Logical bounds do not establish physical/global allocation, scheduling or
measured latency ceilings.

IF-01/02, PATH-02 and INS-01/03/05/08/12/13 distinguish construction, candidate/
alias validation, cut consumption, installation, facade release and normal versus
faulted refusal. MELT/golden/RED/USE track bounded participant/class buckets,
validation duration, installed-context pressure and retained preparation pressure
without treating late refusal as rollback. STRIDE/LINDDUN/STPA/FMEA cover alias
confusion, forged rosters, class widening, refusal disclosure, consumed-cut reuse
and hidden retained obligations. Content, document identities, paths and handles
do not become ordinary metric labels or diagnostic authority.

## Scoped evidence

- read-acquisition-result-mutations.mjs checks eighteen native products, 46
  malformed products, five controls and 23 assertion-detected shape mutations.
- read-acquisition-result-fault-mutations.mjs checks ten probes across eighteen
  cases each and eight guard mutations. Direct native probes prevent facade
  validation from masking a missing native guard. Late facade tests separately
  check actual installation and preparation consumption.
- read-acquisition-result-substrate-checks.mjs wraps 143 post-import callables,
  observes zero baseline calls and detects three injected call types across
  eighteen cases each, with native/prepared/root frames unchanged and temporary
  workspace refunded.

The source-owner legacy checker additionally asserts that unsupported ordinary
requests preserve parent fault state and allow a later valid acquisition. This
distinguishes normal request refusal from a result invariant fault; the original
unsupported-data mutation remains assertion-detected rather than disappearing
behind the new defensive guard. A redundant contract-check mutation blocked by
the unchanged dispatcher is not counted as new semantic evidence.

Full frozen verification and endpoint reconciliation are required before adoption.
Complete public/discovery/event/selector codecs, other operation products, hostile
ingress, actual host/policy truth, physical/global/concurrent/crash/measured
refinement and independent assurance remain separate. No whole completion row
closes and no unbounded or categorical completeness follows.
