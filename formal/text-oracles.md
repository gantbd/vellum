# Canonical byte-edit reference oracles

This design-only check supplies executable expectations for the accepted
original-snapshot edit algebra (specification §§4, 8 and 27; PA-01/PA-02). It is
not production storage, a public request schema or independent certification.
Run `node formal/text-oracles.mjs`; the full verifier runs the same check from
its captured input tree and verifies the recorded counts.

## Constructions and checked relations

Three constructions share only the stated input domain and byte representation:

| Construction | Mechanism | Comparison |
|---|---|---|
| Original-position emitter | Walk original byte indices, emit inserted bytes and skip replaced spans; no splice or cumulative delta | Forward result against both alternatives |
| Right-to-left application | Copy source into a private flat array; splice in descending original offsets | Forward result against emitter |
| Sequential application with inverses | Copy source; adjust ascending offsets by accumulated length changes; capture actual removed bytes | Forward result and recovery by reverse inverse order |

Every case checks exact output length, valid UTF-8, unchanged source and exact
inverse recovery. Each adjacent pair of nonempty source ranges is also coalesced
into one range with concatenated insertions; its emitted result must be equal.
Flat arrays are test scaffolding, not the chosen engine storage structure.

## Finite domain and literal fixtures

The generated domain contains all 156 strings of zero through three scalars
over `a`, `é`, `🙂`, CR and LF. Insertions are empty, `X`, `é`, `🙂`, CRLF or
literal U+FEFF. It enumerates all ordered nonoverlapping nonempty source-range
batches of one through three edits, including touching replacements, and all
single insertions at each scalar boundary. Empty batches are included.

There are 59,052 generated evaluations: 156 empty batches, 8,496 one-edit,
23,400 two-edit and 27,000 three-edit batches. Six literal byte fixtures add
length-changing adjacency, a multibyte replacement, leading literal U+FEFF with
exact CRLF/tab preservation, a scalar edit inside a combining sequence, insertion
into empty text and deletion through the final byte. Total evaluations are
59,058, with 72,901 adjacent-pair coalescing comparisons. Counts include repeated
or overlapping fixture values; these are not distinct theorems or coverage rates.

Sixteen invalid-domain controls require their intended private diagnostic:
continuation-byte starts/ends, out-of-range/negative/nonintegral offsets, three
malformed UTF-8 source forms, malformed/non-byte insertions, overlap, unsorted
input, coincident insertions, an insertion touching a replacement and sparse
source/insertion arrays. These diagnostics are internal tests, not chosen public
outcomes or error precedence.

Ten targeted faulty alternatives must disagree with their literal fixture or
source-preservation expectation: unshifted sequential coordinates, omitted
adjacent edit, UTF-16 interpreted as byte offsets, implicit newline rewriting,
Unicode normalization, stripped literal leading U+FEFF, omitted suffix, forward
inverse order, mutation through an aliased source and sparse-array equality false
positives. Byte validation and equality check every index explicitly; missing
array entries cannot be treated as NUL by the codec or skipped by comparison.
This establishes sensitivity to these examples, not mutation adequacy for all
possible implementations.

## Assumptions, exclusions and remaining gates

Inputs are already canonical valid UTF-8, with scalar byte boundaries and ordered
original-source coordinates. Multi-insertion ties and insertions touching other
edits are outside this oracle domain. These oracles consume already canonical
input; adopted D02 ordering normalization and ambiguity rejection are checked
separately in [batch-normalization.md](batch-normalization.md). No production
insertion-affinity profile is selected. Coalescing comparisons here establish
byte equality only; that separate note gives a mapping counterexample.
Identity-valued batches test algebra only, not adopted D01's validation and
complete semantic classification (specification §6.4 / PS-NO). Literal U+FEFF is canonical text, not a decision
about file-BOM decoding/metadata (D21). Scalar boundaries do not define grapheme
selection, movement or deletion profiles (D27).

Node's encoder/strict decoder, array operations, the generator and the same
author's reasoning remain trusted. Literal byte fixtures reduce shared codec
dependence for selected cases. No production algorithm or formal model operator
is imported, but algorithmic diversity is not organizational/process independence
or source-isolated Cleanroom evidence. The check is finite, not arbitrary-length
induction or all-Unicode enumeration. It does not establish actual backend alias
or lifetime safety, resource/overflow bounds, current revision/authority gates,
attachment mapping, provider behavior or whole-engine refinement.

Future implementation conformance can compare candidates against these reference
results without reusing candidate edit application. Independent oracle review,
broader profile fixtures and atomic-clause/refinement obligations remain open.
