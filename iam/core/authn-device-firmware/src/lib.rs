//! Authenticator-side WebAuthn firmware. Every subsystem below is a stub: this
//! crate has no dependencies and implements no CTAP2 yet.

#![forbid(unsafe_code)]

pub mod attestation;
pub mod config;
pub mod ctap2;
pub mod observability;
pub mod storage;
pub mod transport;
