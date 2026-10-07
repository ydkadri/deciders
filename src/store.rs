//! The store port (ADR 0003): what the commands need from wherever ADRs are kept.
//!
//! It is written in terms of ADRs, not files, so another kind of store needs an
//! adapter and nothing else. A command names an ADR by its number, and what it
//! wants recorded; where and how that is written is the store's business.

use std::path::PathBuf;

use crate::error::Error;
use crate::lifecycle::{Move, Status};

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

/// A change to an ADR, as the port sees it: what the command wants recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The move, which fixes the new status and the line that records it.
    pub movement: Move,
    /// The date, as `YYYY-MM-DD`.
    pub date: String,
    /// Who made the change, if the user has a name set.
    pub by: Option<String>,
    /// References to the work, for `implement`.
    pub references: Vec<String>,
    /// The reason for a rejection, or the note on an implementation.
    pub text: Option<String>,
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

    /// Add a new ADR in the `proposed` state, numbered after the highest in the
    /// store, with its sections taken from the store's template. Returns where it
    /// was put, shown as the store shows places.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the title cannot make an ADR or the store cannot be
    /// written.
    fn propose(&mut self, title: &str, date: &str, by: Option<&str>) -> Result<PathBuf, Error>;

    /// Move ADR `number` as `change` says. The ADR must be in the state the move
    /// needs. Returns the place of the ADR and its new status.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if there is no such ADR, it is not in the state the
    /// move needs, a rejection has no reason, or the store cannot be written.
    fn change(&mut self, number: u32, change: &Change) -> Result<(PathBuf, Status), Error>;
}
