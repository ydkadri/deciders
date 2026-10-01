//! Library for `decider-adr`, in a hexagonal structure (ADR 0004).
//!
//! `domain` is the pure model and its rules. `ports` are the traits the use
//! cases need. `app` holds the use cases, and `adapters` implements the ports.
//! The binary in `main.rs` is the command line and the composition root.

#![warn(missing_docs)]

pub mod adapters;
pub mod app;
pub mod domain;
pub mod ports;
