# Observation revocation: selected v1 scope

The user selected the recommended exact-context scope on 2026-11-06. U06's scope
choice is resolved; implementation, ingress and conformance remain separate.
Requirements §15.5
requires current authorization at each mediated release, absorbing invalidation of
old streams and honest retention of authoritative evidence. F19 additionally
names `revoke-observation-grant`; its v1 target is one exact originally issued
observation context, not an issuer-wide grant or dependent cohort. Owner stop and policy
revocation must not become interchangeable merely because both prevent delivery.

## Options

| Choice | Benefits | Costs and limits |
| --- | --- | --- |
| One exact engine-owned context — recommended for v1 | Reuses existing original identity/scope, absorbing stop, protected control and retention laws; bounded local operation; no grant registry or external policy mutation | Does not claim to revoke sibling contexts or prevent separately authorized new admission. The host remains responsible for policy changes affecting those contexts. The verb must explicitly say context-local revocation. |
| Explicit grant plus all dependent contexts | Gives one engine action a clearly defined wider revocation scope | Requires an actual issuer-owned grant identity, captured dependent-context/cohort relation, admission/release ordering, complete bounded membership and retained revocation ownership. Equal policy/recipient strings or a scan of the current visible contexts cannot supply those guarantees. It still cannot change an external policy system or recall delivered bytes. |

The recommendation is a scope choice, not a claim that context-local revocation
is categorically stronger. Grant-wide revocation is useful if v1 promises one
action invalidates multiple independently admitted contexts. It must then be
specified and proved as that larger contract rather than silently approximated
with a loop of stop calls.

## Shared non-negotiable laws

- Bind actual local authority, original opaque target, captured recipient/purpose/
  destination/continuity and current revocation authority. Token possession or
  source-read permission alone does not authorize revocation.
- Define one serialized decision boundary with admission and queued release.
  A release ordered after accepted revocation cannot use a stale positive grant.
- Regrant never revives the revoked identity. A permitted new context has its own
  admission, budget and captured ceiling.
- Preserve original command/effect evidence, unknown obligations and retained
  ownership. Revocation is not producer cancellation, quiescence, refund, memory
  erasure or a change to previously observed results.
- Preserve protected minimal control, stop and eligible discharge without ordinary
  headroom or content permission. Reauthorize any recipient-visible report.
- Instrument accepted/refused revocation, denied release, stale basis and retained
  obligations using existing bounded control/reason classes, not identity-valued
  labels or private payloads.

The native entry requires independently supplied current revocation authority
bound to the exact original target, in addition to the owner's existing exact
scope/control validation. It may reuse absorbing closure machinery, but cannot
derive revocation authority from stop authority or target possession. No grant/
cohort registry is introduced. Matching retry uses the retained closed identity;
an unavailable/retired target is not fabricated success. The direct bound SDK
ingress now validates a closed typed/dynamic read-context/1 envelope with exactly
contract, matching revoke-observation-grant operation and original opaque context.
Installed discovery now supplies a first-party declaration/schema and private
native handler origin. Dispatch checks original target roles before current
discovery release, then separately checks native revocation authority. Copied
metadata cannot install that origin. Complete cross-owner untyped attachment
resolution remains separate work; the protected direct SDK edge needs no fresh
discovery preparation.

## Precedent and proof boundary

The existing native stop relation, scoped-release grant/epoch model, registry
historical release and execution/receipt contexts establish reusable local laws.
They do not prove a public cross-context revocation contract or a trusted host's
policy truth. Requirements §15.5 already separates policy acceptance from delivery
and makes the trusted-host boundary explicit.

[NIST SP 800-207](https://csrc.nist.gov/pubs/sp/800/207/final) supplies the
resource-focused authorization baseline without implicit trust from location or
ownership. It does not choose Vellum's grant identity, context/cohort granularity
or API. The v1 recommendation above is our architectural inference from the
existing local engine boundary, not a requirement attributed to NIST.

Under the selected scope, compose the actual native target/authority and
release relation, then verify stale queued release, reentry, foreign/copied target,
regrant, protected capacity and retained producer/evidence obligations. Reuse
existing checks where they prove that selected meaning. Choosing this scope is
not evidence of full implementation, frozen qualification or proof closure.
