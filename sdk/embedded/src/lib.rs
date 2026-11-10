//! Distribution facade for trusted in-process Vellum embedding.
//!
//! This crate adds no authority or behavior. It groups the adapter-neutral
//! host contracts, bounded session orchestration, and headless editor API.
#![forbid(unsafe_code)]

pub use vellum_editor as editor;
pub use vellum_host as host;
pub use vellum_session as session;
