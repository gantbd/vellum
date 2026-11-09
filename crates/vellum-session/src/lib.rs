//! Trusted native integration above the engine and its modular substrates.
//! Not an untrusted-script sandbox, policy evaluator or wire protocol.
#![forbid(unsafe_code)]

#[cfg(target_os = "macos")]
pub mod host;
#[cfg(target_os = "macos")]
pub mod local;
