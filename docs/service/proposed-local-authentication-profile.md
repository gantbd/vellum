# Proposed single-subject local authentication profile

Status: implemented local profile, pending integrated release qualification.
The descriptor-anchored credential parser, proof provider/verifier, path-only
provisioning and staged-rotation commands, and optional daemon/MCP credential
flags now ship in the source tree. Omitting the flag preserves
`OwnerOnlyOffline`; no publication has occurred.

The required-authentication handshake is only a transport and host-adapter
seam. Unix socket ownership is endpoint access control, not a human or agent
principal. A static claim provider exists for tests and embedded compatibility;
it is replayable credential material if deployed and is not this profile.

This proposal closes one deliberately narrow local product loop: a daemon and
its local connectors share one explicitly configured private credential file.
The file authenticates a stable subject so that the subject can recover its
own durable operation receipts across restart. Authentication does not grant a
Vellum action.

## Closed composition and authority boundary

`vellum-transport` owns `LocalAuthenticationClaimProvider` and passes it the
exact server challenge and session. A future file-backed provider computes the
claim below; a future daemon verifier validates it and returns the credential's
stable subject. The provider and verifier must remain adapters around the
existing transport/host authentication algebra, not alternate protocol owners.

The stable subject scopes durable receipt identity and prevents one subject
from observing another subject's receipt relation. It does **not** derive
per-principal Vellum authority. The current authenticated daemon still has one
deployment-wide authority ceiling; every accepted subject is evaluated under
that independently configured ceiling. Per-subject roles, grants, policy files,
delegation and revocation of individual actions are future authority work and
must not be inferred from successful proof verification, credential possession,
socket access, receipt possession or resource possession.

## Credential record v1

The record is exactly 68 bytes. All integers are unsigned big-endian. No field
is optional and no padding, truncation or trailing data is accepted.

| Offset | Size | Field | Required value |
| ---: | ---: | --- | --- |
| 0 | 8 | magic | exact bytes `b"VLCRED1\0"` |
| 8 | 2 | format | `1` |
| 10 | 2 | algorithm | `1`, meaning HMAC-SHA-256 with the leftmost 128 bits carried by the v1 handshake |
| 12 | 8 | generation | nonzero monotone credential generation |
| 20 | 16 | subject | nonzero random stable subject reference |
| 36 | 32 | secret | nonzero 256-bit random HMAC key |

Unknown magic, format or algorithm; zero generation, subject or secret; short
records; and extended records fail closed. The subject is stable across staged
rotation. Generation increases strictly and the secret changes on every
rotation. Neither subject nor generation is authority.

Subject and secret are generated from the verified operating-system CSPRNG.
The implementation proposal pins the already-admitted `getrandom = =0.4.3` for
provisioning. Randomness failure refuses provisioning; there is no fallback,
clock-derived value, deterministic test generator in production, or partial
record.

## Exact proof transcript

For record fields `F`, `A`, `G`, `S`, and secret `K`, the HMAC message is exactly
this byte concatenation, with no lengths, text encoding, terminator beyond the
one shown, or ambient configuration:

```text
b"vellum.local-auth-proof.v1\0"
|| F:u16be
|| A:u16be
|| G:u64be
|| S:[u8;16]
|| handshake_version:u8
|| challenge:[u8;16]
|| session:[u8;16]
```

The full value is `HMAC-SHA-256(K, message)`. The v1 claim is its leftmost 16
bytes. The connector still echoes the exact handshake version, challenge and
session in the existing response frame; the verifier rejects any mismatch
before accepting the proof. A challenge/session proof is valid only for that
exact transcript and does not authenticate the server to the connector or
encrypt the local socket.

The normative golden vector is:

- key: bytes `00` through `1f`;
- subject: bytes `10` through `1f`;
- generation: `1`;
- format, algorithm and handshake version: `1`;
- challenge: bytes `20` through `2f`;
- session: bytes `30` through `3f`;
- full HMAC:
  `af8e461506b5d54eae3edb7a3207f42b44249c6506f7bcdc278e8f40d1ab97c1`;
- transmitted claim: `af8e461506b5d54eae3edb7a3207f42b`.

The implementation proposal pins RustCrypto `hmac = =0.12.1`, retaining
`sha2 = =0.10.9`, and uses `Mac::verify_truncated_left` for verification. The
reviewed transitive `subtle` implementation supplies constant-time comparison.
Vellum must not hand-roll RFC 2104 or constant-time cryptography merely to avoid
a dependency. Adding `hmac` requires refreshed lock, source-SBOM, license and
locked/offline source-closure evidence before this profile can ship.

## Descriptor-relative filesystem laws

The credential path is explicit configuration and is never discovered by
searching a workspace or repository. A CLI, if approved, accepts a path only;
it never accepts record bytes, subject or secret through argv or environment.
Loading and mutation obey all of these laws:

1. Open the configured parent directory with directory, close-on-exec and
   no-follow semantics. It must be a real directory owned by the daemon's
   effective user, mode exactly `0700`, and not replaced between validation and
   use. Retain and operate through that descriptor.
2. Open the final single-component name descriptor-relatively with no-follow
   and close-on-exec semantics. It must be a regular file owned by the effective
   user, mode exactly `0600`, link count exactly one, and size exactly 68 bytes.
   Symlinks, directories, devices, sockets, FIFOs, hard links and unknown
   inspection states refuse.
3. Read exactly 68 bytes from the pinned descriptor, require EOF immediately,
   and revalidate descriptor identity and metadata after the read. Revalidate
   the parent namespace binding against the pinned file identity before
   returning success. Replacement, detachment or ambiguity refuses; validation
   by path followed by a separate path open is forbidden.
4. Never include path, record fields, subject, generation, secret, claim or raw
   provider/verifier errors in `Debug`, display errors, events, logs or metrics.
   Buffers are kept only as long as required. This profile makes no secure
   erasure, swap exclusion, core-dump exclusion or same-UID process isolation
   claim.

Parent-directory ancestry above the explicitly trusted `0700` directory remains
an operator trust boundary. A private file does not defend against a hostile
process with the same effective UID or against a privileged host administrator.

## Provisioning, staged rotation and revocation

Provisioning is an explicit offline operator action. It generates subject,
generation `1` and secret in memory; descriptor-relatively creates a new final
name with exclusive-create/no-follow semantics and mode `0600`; writes the
complete record; synchronizes the file; revalidates it; then synchronizes the
pinned parent directory. Existing names are never overwritten. Any failure
reports a content-free refusal and leaves no credential claimed as active.

Rotation is staged, never in-place:

1. Load and validate the current record.
2. Generate a new secret, preserve the exact subject, and checked-increment the
   generation. Exhaustion refuses.
3. Exclusively create and synchronize a distinct operator-selected next path
   under the same descriptor laws. The old file remains intact.
4. Stop daemon and connectors, switch both configurations to the staged path as
   one operator-controlled activation, and restart. Mixed generations fail
   authentication rather than falling back.
5. After acceptance and an operator-defined rollback interval, revoke the old
   path and every backup through an explicit external retention procedure.

The profile does not silently select a newest file, follow a pointer, overwrite,
auto-retry another generation, accept both generations concurrently, or delete
the old record. The crash boundary is therefore conservative: before the
configuration switch, the old path remains authoritative; after the switch,
only the new path is configured. Atomic multi-process configuration activation,
rollback policy, backup custody and deletion are deployment responsibilities,
not claims of this v1 primitive.

Restoring the same subject and an accepted generation/secret restores that
subject's ability to authenticate and disclose its retained receipts. Losing
the record makes those receipts unavailable; replacing the subject creates a
different receipt principal. A credential backup is therefore both an
authentication secret and a receipt-disclosure capability.

## Bounded failure behavior

Credential loading performs bounded work over one parent descriptor, one final
file and exactly 68 record bytes. Proof verification performs one fixed-size
HMAC and one complete constant-time 16-byte verification. There is no
user-controlled allocation proportional to malformed input, key search,
generation search, parser recovery, fallback algorithm, retry loop or error
oracle. All malformed-file and invalid-proof causes collapse to fixed-cardinality
external refusal classes. Authentication/session admission remains subject to
the daemon's configured lifetime and live-session ceilings; saturation and
identity exhaustion fail closed.

The verifier must not reveal whether refusal was caused by an unknown subject,
generation, challenge, session, claim byte, file state or authority decision.
Authentication success and later action refusal remain separate outcomes, but
neither outcome exposes secrets or another subject's receipt existence.

## Required TDD/BDD and qualification evidence

Implementation begins with executable behavior specifications and negative
tests. It is not shipped until all of the following are green under the locked,
offline, warnings-denied workspace gates:

- exact 68-byte encode/decode round trips, the normative golden vector,
  published HMAC-SHA-256 known-answer vectors and differential results against
  an independent implementation;
- table-driven rejection of every short and extended length, unknown magic,
  format and algorithm, every forbidden zero field, generation overflow and
  every individual record-field mutation;
- changed domain, format, algorithm, generation, subject, handshake version,
  challenge, session, key and each of the 16 claim bytes all refuse; repeated
  timing distributions are reviewed without claiming a timing proof;
- BDD scenarios for provision, authenticate, restart and recover the same
  receipt; foreign subject lookup remains indistinguishable from absence;
  proof replay on another challenge/session fails; authentication never grants
  an otherwise-forbidden operation;
- filesystem adversaries covering symlink, directory, device, socket/FIFO,
  wrong owner/mode/link count/size, parent or file replacement, detachment and
  namespace swap, with descriptor identity retained throughout;
- crash cuts before create, during write, before/after file sync and before/
  after directory sync for provisioning and staged rotation, proving no
  existing credential is overwritten and no partial file is accepted;
- restart and rotation scenarios covering old/new configuration mismatch,
  same-subject continuity, changed-subject separation, lost files, randomness
  failure, generation exhaustion, capacity saturation and counter saturation;
- tests proving secret, subject, generation, claim, transcript and path do not
  appear in help, argv, environment fixtures, errors, events, debug output,
  logs, metrics, repository fixtures or generated artifacts; and
- refreshed compatibility baseline, source SBOM, license review,
  locked/offline build, Cargo-install and GitHub-source-archive qualification
  for the new adapter and dependency closure.

RED and USE instrumentation is fixed-cardinality and content-free. Required
counters are handshakes started/completed/refused, handshake I/O failures,
credential loads completed/refused, provisions completed/refused, rotations
staged/refused, randomness failures, sessions issued/closed/active, configured
session capacity, identity exhaustion and counter saturation. Golden signals
derive only aggregate latency, traffic, refusal/error rate and saturation from
those bounded labels. Metrics never carry path, subject, generation, claim,
secret, transcript bytes, provider error text or receipt identity. Alerting
must distinguish operational unavailability from invalid proof internally
without exporting a principal-enumeration oracle.

## STRIDE and LINDDUN review

- **Spoofing:** the claim binds exact credential metadata, subject, handshake
  version, challenge and session. Socket ownership, echoed fields and a static
  claim are explicitly insufficient. Server authentication is not provided.
- **Tampering:** exact parsing, pinned descriptors, no-follow/exclusive-create,
  identity revalidation and file/directory synchronization bound file races;
  host privilege and hostile same-UID processes remain outside the claim.
- **Repudiation:** a durable receipt may attribute an operation to the stable
  subject, but the symmetric MAC and local record are neither a signature nor
  independent audit evidence.
- **Information disclosure:** the credential file and local socket traffic are
  plaintext. File access, process memory, backups and a same-user observer can
  disclose or infer authentication activity. Telemetry adds no identifiers.
- **Denial of service:** malformed records and proofs consume bounded work;
  file loss, bad rotation, capacity saturation and randomness failure refuse
  safely but can make service or retained receipts unavailable.
- **Elevation of privilege:** authentication supplies attribution only. The
  separate global authority ceiling still decides every action; per-principal
  authority is expressly unimplemented.
- **Linkability/identifiability:** a stable subject intentionally links that
  subject's receipts across restart and rotation. Generation, connection timing
  and file/backups add local linkage; no anonymity or unlinkability is claimed.
- **Non-repudiation/detectability:** the symmetric design cannot provide
  non-repudiation, while the private file and traffic timing remain detectable
  to relevant local observers.
- **Disclosure/unawareness/non-compliance:** provisioning and backup UX must
  disclose plaintext storage, stable linkage, recovery and loss consequences.
  No consent-management, regulatory-compliance, retention, secure-deletion or
  purpose-limitation claim follows from this profile.

An inherited already-open private descriptor may be proposed later, but it
requires a separate supervisor lifetime, descriptor-number, inheritance,
rotation and restart contract. Vellum must not silently treat it as equivalent
to this path-based profile.
