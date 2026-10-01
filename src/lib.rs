//! Library core for `decider-adr`.
//!
//! This crate is the pure core (ADR 0004): the ADR model, its lifecycle rules,
//! and reading and writing an ADR's text. It does no I/O. The binary in
//! `main.rs` stays a thin wrapper that handles process concerns only.

#![warn(missing_docs)]

pub mod adr;
pub mod date;
pub mod header;
pub mod number;
pub mod status;
