//! The store port (ADR 0003): what the commands need from wherever ADRs are kept.
//!
//! It is written in terms of ADRs, not files, so another kind of store needs an
//! adapter and nothing else. Later commands add reading, listing and creating.

use std::path::PathBuf;

use crate::error::Error;

/// What preparing a store did, as places: relative to the store's root when they
/// are below it, and in full otherwise.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Prepared {
    /// Places that did not exist and were made.
    pub created: Vec<PathBuf>,
    /// Places that already existed and were left alone.
    pub kept: Vec<PathBuf>,
    /// Places that already existed and were written over.
    pub replaced: Vec<PathBuf>,
}

/// The write side of an ADR store.
pub trait WritesAdrs {
    /// Make sure the store exists, with its template and whatever settings it
    /// needs. Nothing that already exists is changed, except the settings when
    /// the store was asked to replace them.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the store cannot be made.
    fn prepare(&mut self) -> Result<Prepared, Error>;
}
