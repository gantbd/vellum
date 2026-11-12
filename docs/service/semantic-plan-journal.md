# Semantic plan journal

The semantic apply service owns canonical plan and operation records and their
recovery algebra. `vellum-host::append_journal` owns only the bounded opaque-byte
port and its closed commit truth: `Committed`, `Refused`, or `Indeterminate`.
The local filesystem adapter interprets that port without depending on the
semantic service.

The macOS local profile stores private record files that the adapter never
mutates beneath an
anchored `.vellum-append-journal-v1` directory. A synchronized namespace file
binds the directory to an exact opaque workspace and owner identity. Record
access is descriptor-relative; the path is used only to revalidate that its
device/inode identity still names the open directory. Directories must be owned
by the effective user with mode `0700`; namespace and record files must be
singly linked regular files owned by that user with mode `0600`. Symlinks,
special files, extra names, gaps, substitutions, hard links, and permission
drift fail closed.

One open adapter handle holds a non-blocking advisory exclusive lock on the
anchored directory. This prevents multiple cooperating Vellum compositions
from racing record ordinals. It is not an authorization boundary: a same-user
non-cooperating process can bypass an advisory lock, so metadata, namespace,
and descriptor identity are revalidated at operation boundaries.

Append capacity is fixed by explicit record, per-record byte, and total-byte
bounds. Records are never evicted. `Committed` is returned only after the record
file and directory entry have been fully synchronized, the namespace is still
bound, and the record can be read back exactly. Failure before record creation
is `Refused`; any failure after exclusive creation is conservatively
`Indeterminate`. A restart reads dense oldest-first records without applying
effects; the semantic service then validates its canonical checksums and
reconstructs state. An indeterminate cut may leave a partial opaque record; the
adapter does not reinterpret it, and semantic canonical validation fences
recovery rather than accepting or repairing it.

Instrumentation exposes only saturating operation/outcome counts. It does not
emit paths, workspace/owner identities, record names, sizes, or contents.

This adapter is intentionally not wired into the daemon or MCP connector yet.
The current durability mechanism uses macOS `F_FULLFSYNC`; other operating
systems require a separately qualified durability profile rather than a weaker
claim under the same contract.
