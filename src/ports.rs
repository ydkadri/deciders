//! The traits the use cases need from the outside world (ADR 0004).
//!
//! They are written in terms of ADRs, not files, so a store that is not a
//! directory of Markdown needs an adapter and nothing else. Reading and writing
//! are separate traits, so a command that only reads cannot change anything.

use crate::domain::adr::Adr;
use crate::domain::number::AdrNumber;
use crate::domain::person::DisplayName;

/// A store that cannot be used. The message says what failed and where.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StoreError(pub String);

/// Where a store put a new ADR, in words a person can act on, such as a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location(pub String);

/// What preparing a store did, as the places it made and the places it left alone.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prepared {
    /// Places that did not exist and were made.
    pub created: Vec<Location>,
    /// Places that already existed and were left alone.
    pub kept: Vec<Location>,
}

/// The read side of an ADR store.
pub trait ReadsAdrs {
    /// The numbers of the ADRs in the store, in no particular order. The
    /// template is not an ADR and is not listed, and the number 0 that is
    /// reserved for it is never given to an ADR.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the store cannot be read or does not exist.
    fn numbers(&self) -> Result<Vec<AdrNumber>, StoreError>;

    /// The store's own template, or `None` if it has none.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the store cannot be read.
    fn template(&self) -> Result<Option<String>, StoreError>;
}

/// The write side of an ADR store.
pub trait WritesAdrs {
    /// Make sure the store exists, with its template and whatever settings it
    /// needs, without changing anything that already exists. The settings are
    /// written last, so a run that fails part way can be repeated.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the store cannot be made.
    fn prepare(&mut self) -> Result<Prepared, StoreError>;

    /// Add a new ADR. It is never written over an existing one.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the store cannot be written, or already holds
    /// an ADR that would be replaced.
    fn create(&mut self, adr: &Adr) -> Result<Location, StoreError>;
}

/// A way to ask the person running the tool for their name, when there is one
/// to ask.
pub trait AsksForName {
    /// Ask for the user's name. Returns `None` if nobody can be asked, or they
    /// gave no answer and there is no suggestion to accept.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the question cannot be asked or the answer
    /// cannot be used.
    fn ask(&mut self) -> Result<Option<DisplayName>, StoreError>;
}

/// The user's own settings, which are kept outside the repository (ADR 0003).
pub trait KeepsUserSettings {
    /// The user's name, or `None` if none has been set.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the settings exist but cannot be read.
    fn name(&self) -> Result<Option<DisplayName>, StoreError>;

    /// Record the user's name. Only called when none is set.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the settings cannot be written.
    fn set_name(&mut self, name: &DisplayName) -> Result<(), StoreError>;
}
