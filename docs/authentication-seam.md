# Authentication and session-establishment seam
Status: abstract host contract, optional local-daemon handshake and
transport/SDK connector seam implemented; no credential mechanism, Aegis
adapter, or remote handshake selected. The exact prerequisite for considering a
single-subject private-file profile is recorded in
[`service/proposed-local-authentication-profile.md`](service/proposed-local-authentication-profile.md).

Vellum now owns a transport-neutral authentication seam in
`vellum_host::authentication`. It is categorically separate from the action
authority lattice:

```text
adapter-owned credential context
        |
        v
AuthenticationPortV1 -- exact evidence --> AuthenticationGateV1
        |                                      |
 rejected / unavailable                       v
                                      AuthenticatedSessionV1
                                                |
                                      parameter-free handoff
                                                v
                                        AuthorityContextV1
                                                |
                                      AuthorityPortV1 decides action
```

The trusted composition root supplies fixed-width, opaque challenge, claim and
session references. The credential adapter receives the exact request plus its
own context and returns `Authenticated`, `Rejected`, or `Unavailable`.
Authenticated evidence is accepted only for the exact request. A mismatch is
an unavailable result and cannot produce an authority context. The gate has an
explicit consultation budget and saturating, content-free counters.

The successful handoff takes no subject or session argument. Consequently a
caller can only obtain the exact `(verified subject, host-allocated session)`
context established by the adapter. That context still grants no action:
Vellum's separate authority port must approve every exact resource/action
request.

The local daemon now supplies one concrete composition. Its explicit default is
`OwnerOnlyOffline`, which preserves the existing owner-only Unix endpoint and
does not claim a principal. Calling `LocalDaemon::require_authentication`
selects the required, fixed-size local handshake protocol V1 before the
ordinary Vellum V1 frame. The server
allocates a fresh connection-local challenge and session reference, the client
echoes both with an opaque adapter correlation claim, and the existing gate
must establish exact evidence before any Vellum frame is decoded. The resulting
subject/session is retained only for that one exchange and is closed on
success, request failure, or peer disconnect. It is categorically not supplied
to the service action-authority interpreter.

Rejection, adapter unavailability, malformed/transplanted evidence and replay
all produce the same two-byte negative result and daemon serve error. The
daemon records only aggregate attempts, completions, refusals, I/O failures,
issued/closed/live sessions and saturation. No claim, subject, challenge,
session, provider reason, or credential enters events or operational snapshots.
The authenticated connector lives in `vellum-transport`, is re-exported through
`vellum-sdk-client`, and delegates exact challenge/session claim derivation to an
adapter-owned provider. Its static-claim compatibility constructor remains a
test/embedding seam, not the shipped credential profile. The separately
specified descriptor-anchored local credential record supplies one stable
subject and HMAC proof; it is not a password, token, user name, role, or Aegis
grant format.

Challenge and session references combine one OS-CSPRNG daemon incarnation with
a domain-separated monotone ordinal, so they are injective within a lifetime
and freshly randomized across restarts. Randomness or lifetime exhaustion fails
closed. This is replay resistance for the local handshake, not durable replay
audit evidence or a credential-strength claim. Expiry, revocation and
expiry, online revocation and dynamic credential revalidation remain outside
the local file-backed v1 adapter.

## Aegis compatibility and gap

Aegis explicitly does not authenticate passwords, devices, OAuth tokens or
network peers. Its current contract consumes host-established credential facts
and decides scoped authority. Therefore Vellum cannot truthfully call Aegis an
authentication provider or derive an authenticated principal merely from an
Aegis grant ID.

A future composition may put a credential verifier behind
`AuthenticationPortV1`, feed the resulting exact context into an Aegis-backed
`AuthorityPortV1`, and retain both evidence chains under one operation. No Aegis
wire protocol, credential format or portable grant is promised by its current
specification, so this tranche deliberately adds no Aegis dependency or remote
transport.

Before selecting a credential-bearing or remote profile, the composition must
additionally specify:

- the credential mechanism and trusted verifier;
- challenge generation, uniqueness, expiry and replay retention bounds;
- durable replay audit/retention bounds if probabilistic CSPRNG incarnation
  separation is insufficient for the selected threat model;
- per-request revalidation, monotone time and revocation behavior;
- non-enumerating external projection of rejection and unavailability;
- unconditional cleanup after credential loss; and
- attribution joining authentication evidence, authority evidence and effect
  evidence without turning any reference into a bearer capability.

## STRIDE and LINDDUN boundary

The seam mitigates evidence transplantation, identity substitution during the
authentication-to-authority handoff, unbounded consultation, provider-error
leakage into Vellum types and accidental credential logging. Opaque references
and aggregate counters are content-safe by construction.

The required profile mitigates same-lifetime transplantation/replay and
prevents service decoding before authentication. It does not mitigate a
same-user process connecting to the Unix socket, socket traffic observation,
credential theft, persistent replay auditing, timing correlation, or denial of
service before authentication.
Private-directory ownership and mode checks are endpoint-integrity/reachability
controls, not principal authentication. The current local daemon therefore
remains an unauthenticated single-OS-user profile unless its embedding
composition explicitly installs the required adapter-backed profile. The
shipped CLI remains owner-only offline by default and selects the
descriptor-anchored local credential adapter only with an explicit path flag.

Handshake V1 provides neither confidentiality nor transport integrity: its
bytes are plaintext on the local socket. It does not isolate mutually hostile
processes that can observe or steal an adapter claim, define credential
semantics, authenticate the server to a remote peer, propagate authentication
as Vellum action authority, or authorize an operation. Every operation still
passes independently through the service authority lattice.
