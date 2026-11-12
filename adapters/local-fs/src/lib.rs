//! Local-filesystem mechanisms, separate from the I/O-free semantic engine.
//!
//! The first executable profile is deliberately macOS-only and conservative:
//! staged replacement of singly linked regular local files, checked POSIX
//! permissions/ownership and verified bounded extended attributes. ACL-bearing,
//! special-flag and unsupported metadata cases refuse, not silently discard.
//! This is best-effort
//! conflict detection, not an atomic compare-and-swap or a sandbox for hostile
//! directories. No universal crash-durability or network-filesystem claim follows.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "macos")]
mod export_copy;
#[cfg(target_os = "macos")]
pub use export_copy::*;
#[cfg(target_os = "macos")]
mod host;
#[cfg(target_os = "macos")]
pub use host::LocalFilesystemFactory;
#[cfg(target_os = "macos")]
mod workspace;
#[cfg(target_os = "macos")]
pub use workspace::*;
#[cfg(target_os = "macos")]
mod workspace_identity;
#[cfg(target_os = "macos")]
pub use workspace_identity::*;
#[cfg(target_os = "macos")]
mod request_journal;
#[cfg(target_os = "macos")]
pub use request_journal::*;
#[cfg(target_os = "macos")]
mod append_journal;
#[cfg(target_os = "macos")]
pub use append_journal::*;
