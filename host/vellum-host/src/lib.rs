//! Adapter-neutral host contracts for embedding Vellum.
//!
//! This crate owns capability ports and closed outcome vocabularies. It does
//! not select a filesystem, codec, Unicode provider, clipboard, UI or retry
//! policy. Concrete adapters interpret these contracts; applications compose
//! them.
#![forbid(unsafe_code)]

pub mod annotation;
pub mod append_journal;
pub mod authentication;
pub mod authority;
pub mod export_copy;
pub mod request_journal;
pub mod resource;
pub mod save_as;
pub mod save_as_journal;
pub mod semantic;
pub mod unicode;
pub mod workspace;
