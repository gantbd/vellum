//! Headless editor application behavior above the trusted native session facade.
//! Not part of the semantic engine, a presentation, or a wire protocol.
#![forbid(unsafe_code)]
mod app;
pub use app::*;
pub use vellum_engine::attachment::{
    AttachmentTarget as CommentTarget, LocationEvidence as CommentLocationEvidence,
    Placement as CommentPlacement, PointProfile as CommentPointProfile,
};
pub use vellum_engine::coordinator::HistoryDirection as CommentHistoryDirection;
pub use vellum_engine::coordinator::SearchDirection;
pub use vellum_workspace::*;
