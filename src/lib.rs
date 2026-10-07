//! Library for `decider-adr` (ADR 0003): the store port, the file adapter that
//! implements it, and the user's own settings.

#![warn(missing_docs)]

pub mod error;
pub mod files;
pub mod lifecycle;
pub mod store;
mod text;
pub mod user;
