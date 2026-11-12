# Unix ownership reuse audit

Status: comparison and extraction handoff, 2026-11-10. No shared mechanism was
extracted in this pass.

This audit compares the current local-resource implementations in Vellum,
Chronos, Moira, and Keystone. It asks only whether the same Unix mechanism can
be shared. Lifecycle, wire framing, retry, stale-outcome, checkpoint, and
application policy remain owned by their respective composition roots.

## Law matrix

| Law | Vellum daemon | Chronos daemon/local transport | Moira SQLite | Keystone directory checkpoint | Reuse disposition |
| --- | --- | --- | --- | --- | --- |
| Root creation | Parent must already exist | Creates the leaf if absent | Trusted parent is a deployment assumption | Separate create-new and open-existing operations | Different policy; do not share creation |
| Root type/mode/owner | Real directory, effective owner, exact `0700` | Directory, effective owner, no group/other bits | No equivalent owned-directory handle | Real directory, effective owner, exact `0700` | Vellum/Keystone validator shape aligns |
| Root no-follow open | `O_NOFOLLOW | O_DIRECTORY` after component normalization | Metadata check only | No owned-directory descriptor | `O_NOFOLLOW | O_DIRECTORY` after component normalization | Vellum/Keystone mechanism aligns |
| Root identity revalidation | Descriptor and path device/inode | None after bind | Parent is not pinned | Descriptor and path device/inode | Vellum/Keystone mechanism aligns |
| Persistent owner lock | Create-or-open, exact `0600`, regular, owner, link count one, advisory lock; never unlink | Same broad intent, but permits any owner-only mode and does not revalidate path identity | Sidecar persists and identity is revalidated, but open lacks no-follow and owner/mode checks | Create-new or open-existing depending store operation; exact `0600`, regular, owner, link count one, advisory lock; never unlink | Semantics are not yet identical across two settled consumers |
| Lock no-follow | Required | Required | Not required | Required | Moira differs |
| Lock identity revalidation | Descriptor and path device/inode before operations | Initial descriptor validation only | Descriptor and path device/inode | Descriptor and path device/inode | Vellum/Moira/Keystone share identity shape, not validation policy |
| Socket validation | Socket, effective owner, exact `0600`, link count one, device/inode | Socket, effective owner, owner-only mode; no link or identity pin | No socket | No socket | Vellum-only exact contract |
| Existing socket at startup | Always refuse; no authenticated predecessor identity | Deletes a verified socket after acquiring the writer lock | Not applicable | Not applicable | Material conflict; cannot share stale recovery |
| Drop cleanup | Remove only the exact socket device/inode created by this guard | Remove any socket currently at the path while lock is held | Persistent files and lock are never removed | Persistent files and lock are never removed | Material conflict; Chronos can remove a replacement socket |
| Replacement disposition | Fence; preserve replacement | Live replacement is not continuously checked | Fence on database or lock replacement | Fence on root, lock, or checkpoint replacement | Common fail-closed goal, different coverage |
| Symlink disposition | Root, lock, socket, and control leaf refuse | Root metadata and lock/socket leaves refuse | Database and lock opens may follow symlinks | Root and private files refuse | Moira differs materially |
| Directory contents | Lock plus data/control sockets are allowed | Lock, socket, and checkpoint data coexist | SQLite side files are expected | Closed set of lock/head/pending only | A shared directory owner would overclaim policy |

Moira has no standalone Unix-socket daemon in the current repository. Its
relevant reusable precedent is the SQLite `OwnedResource` sidecar identity
fence, not endpoint ownership or private-directory cleanup.

## Transport and lifecycle comparison

Vellum has two sockets with distinct laws: the data socket admits one bounded
native exchange, while the owner-only control socket installs a monotone drain
request and wakes blocked acceptance without counting the wake as service
traffic. Admission is projected through Running/Draining/Stopped/Fenced and a
Keystone obligation ledger.

Chronos currently has one data socket and a blocking `serve_one`; it has no
peer control socket, admission phase, drain obligation, or replacement fence.
Its malformed-frame continuation and durable outcome/retry rules are also
Chronos-specific. Moira exposes storage and receiver adapters rather than a
local Unix transport. Keystone directory storage has fencing and uncertain
commit dispositions, not socket acceptance or process lifecycle. There is
therefore no shared phase-aware transport mechanism to extract. Only the lower
Unix resource identity substrate is a future candidate.

## Decision

There is a plausible low-level intersection between Vellum and Keystone:

- a `(device, inode)` identity value;
- opening an existing private directory with no-follow/directory flags;
- validating exact owner/type/mode/link rules; and
- comparing descriptor identity with a path reinspection.

Extraction is not safe in the current workspace. Keystone's
`keystone-storage-directory` Unix implementation, tests, and local-checkpoint
documentation are concurrently untracked/in flight. Moving its validators now
would obscure ownership of those changes and risk silently changing durable
create/open/fsync semantics. Chronos and Moira also do not yet satisfy the same
complete contract, so adopting a nominally generic guard there would weaken or
misstate at least one consumer.

No shared crate was therefore introduced. Vellum retains its existing guards
and adds adversarial coverage for parent symlinks, control-path symlinks,
pre-existing control sockets, replacement data sockets, and exact-identity
cleanup. This is a deliberate stop at the contract boundary, not a claim that
the duplication is ideal.

## Extraction handoff

After the Keystone local-checkpoint work lands, a follow-up may consider a
small `keystone-unix-resource` crate with policy-free pieces only:

1. `UnixIdentity { device, inode }` and exact-object comparison.
2. Fallible no-follow opening of an existing directory or regular file.
3. Explicit validators parameterized by required type, exact mode, effective
   owner, and link-count rule.
4. A pinned descriptor/path pair with a content-free `verify_identity` result.
5. An identity-checked unlink helper that never decides *whether* stale cleanup
   is authorized.

The shared layer must not create directories, choose lock names, acquire or
release application ownership, delete stale sockets, enumerate allowed
directory contents, fence a service, install signal/control paths, or decide
retry and outcome-unknown policy. Those are consumer laws. Before adoption,
Chronos should independently repair and test replacement-socket cleanup;
Moira should decide whether symlink following and missing owner/mode validation
are intentional profile choices.

## Executable evidence retained in Vellum

The daemon tests now cover:

- lock symlink and multiply-linked lock refusal without destination mutation;
- parent-directory symlink refusal;
- existing data/control path refusal without speculative unlink;
- control-path symlink refusal;
- live lock and socket replacement fencing;
- exact-identity Drop cleanup that preserves replacement files and sockets; and
- stale control-socket preservation.

These tests establish Vellum's local laws only. They do not qualify Chronos,
Moira, Keystone storage, hostile same-user resistance, remote filesystems, or
crash-safe stale endpoint recovery.
