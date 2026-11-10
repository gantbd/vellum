//! Closed native protocol between Vellum clients and a Vellum service.
//!
//! This first profile is deliberately in-process and single-workspace. It is
//! not yet a wire-format, compatibility, authentication, or remote-transport
//! claim. MCP and other transports must project this vocabulary rather than
//! exposing editor or engine internals.
#![forbid(unsafe_code)]

use std::{fmt, num::NonZeroU64, sync::Arc};

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(NonZeroU64);

        impl $name {
            pub fn new(value: u64) -> Option<Self> {
                NonZeroU64::new(value).map(Self)
            }

            pub fn get(self) -> u64 {
                self.0.get()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

opaque_id!(ClientId);
opaque_id!(RequestId);
opaque_id!(VersionWitness);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandV1 {
    Snapshot,
    Replace {
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestV1 {
    pub client: ClientId,
    pub request: RequestId,
    pub command: CommandV1,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SnapshotV1 {
    pub witness: VersionWitness,
    pub revision: u64,
    pub text: Arc<str>,
}

impl fmt::Debug for SnapshotV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapshotV1")
            .field("witness", &self.witness)
            .field("revision", &self.revision)
            .field("text", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutcomeV1 {
    Snapshot(SnapshotV1),
    Replaced(SnapshotV1),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalV1 {
    UnknownClient,
    ClientCapacity,
    RequestCapacity,
    RequestConflict,
    WitnessCapacity,
    UnknownWitness,
    Stale,
    InvalidRange,
    InputLimit,
    SnapshotLimit,
    EditorClosed,
    EditorCapacity,
    EditorConflict,
    EditorIncomplete,
    EditorUnavailable,
    CounterExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyV1 {
    Accepted(OutcomeV1),
    Refused(RefusalV1),
}

/// Transport-neutral client port. Implementations may call an in-process
/// service or carry the same request/reply vocabulary over a qualified wire.
pub trait ServicePort {
    fn connect(&mut self) -> Result<ClientId, RefusalV1>;
    fn execute(&mut self, request: RequestV1) -> ReplyV1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_evidence_redacts_content_and_opaque_ids() {
        let snapshot = SnapshotV1 {
            witness: VersionWitness::new(9).unwrap(),
            revision: 4,
            text: Arc::from("private source"),
        };
        let debug = format!("{snapshot:?}");
        assert!(debug.contains("<redacted>"));
        assert!(debug.contains("<opaque>"));
        assert!(!debug.contains("private source"));
        assert!(!debug.contains('9'));
    }
}
