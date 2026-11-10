//! Headless editor engine under development; not the complete behavior facade.
//!
//! Filesystem mechanisms, UI, host workflows and telemetry exporters stay outside
//! this crate. No imports start services or grant resource authority.

pub mod attachment;
pub mod capture;
pub mod coordinates;
pub mod coordinator;
pub mod instrumentation;
pub mod resource;
pub mod selection;
pub mod settings;
pub mod source;
