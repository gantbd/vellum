//! Distribution facade for transport-neutral Vellum clients.
//!
//! This crate adds no protocol or transport behavior. It groups the native
//! client contract, DTOs, canonical wire codec, and bounded transport.
#![forbid(unsafe_code)]

pub use vellum_client as client;
pub use vellum_protocol as protocol;
pub use vellum_transport as transport;
pub use vellum_wire as wire;
