//! Adapter-neutral host contracts for embedding Vellum.
//!
//! This crate owns capability ports and closed outcome vocabularies. It does
//! not select a filesystem, codec, Unicode provider, clipboard, UI or retry
//! policy. Concrete adapters interpret these contracts; applications compose
//! them.
#![forbid(unsafe_code)]

pub mod annotation;
pub mod resource;
pub mod unicode;
