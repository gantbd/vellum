//! Presentation/interaction state above the trusted native session facade.
//! Not part of the semantic engine or a new authority/command registry.
#![forbid(unsafe_code)]
#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
pub use app::*;
#[cfg(target_os = "macos")]
pub use vellum_engine::attachment::{
    AttachmentTarget as CommentTarget, LocationEvidence as CommentLocationEvidence,
    Placement as CommentPlacement, PointProfile as CommentPointProfile,
};
#[cfg(target_os = "macos")]
pub use vellum_engine::coordinator::HistoryDirection as CommentHistoryDirection;
#[cfg(target_os = "macos")]
pub use vellum_engine::coordinator::SearchDirection;
