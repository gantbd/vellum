//! Distribution facade for transport-neutral Vellum semantic clients.
//!
//! This crate adds no protocol, service, session, or transport behavior. It
//! groups the provider-neutral semantic query client, canonical session values,
//! witness-validating session client, sovereign in-memory service needed by
//! explicit embedded qualification compositions, and bounded local transport.
//! Authentication credentials and role policy remain adapter/composition
//! concerns; re-exporting a type grants no authority.
#![forbid(unsafe_code)]

pub use vellum_semantic_client as client;
pub use vellum_semantic_protocol as protocol;
pub use vellum_semantic_service as service;
pub use vellum_semantic_session_client as session;
pub use vellum_semantic_transport as transport;
