//! Distribution facade for transport-neutral Vellum clients.
//!
//! This crate adds no protocol or transport behavior. It groups the native
//! client contract, DTOs, canonical wire codec, and bounded transport. The
//! transport module also exposes the local authenticated-handshake connector;
//! concrete credential verification remains an adapter concern and no
//! credential profile is selected by this facade.
#![forbid(unsafe_code)]

pub use vellum_client as client;
pub use vellum_protocol as protocol;
pub use vellum_transport as transport;
pub use vellum_wire as wire;
pub use vellum_workspace_client as workspace;
