//! In-memory fakes of the ports, for testing the use cases (ADR 0004).
//!
//! Each one implements the real trait. There is no mocking library.

use crate::domain::adr::Adr;
use crate::domain::number::AdrNumber;
use crate::domain::person::DisplayName;
use crate::ports::{
    AsksForName, KeepsUserSettings, Location, Prepared, ReadsAdrs, StoreError, WritesAdrs,
};

/// A store held in memory.
#[derive(Debug, Default)]
pub struct MemoryStore {
    /// Whether the store exists.
    pub exists: bool,
    /// The store's template, if it has one.
    pub template: Option<String>,
    /// The ADRs in the store, in the order they were added.
    pub adrs: Vec<Adr>,
    /// Numbers that are present but are not held as parsed ADRs.
    pub extra_numbers: Vec<AdrNumber>,
    /// A failure that every call returns while it is set.
    pub fail_with: Option<String>,
}

impl MemoryStore {
    fn check(&self) -> Result<(), StoreError> {
        match &self.fail_with {
            Some(message) => Err(StoreError(message.clone())),
            None => Ok(()),
        }
    }
}

impl ReadsAdrs for MemoryStore {
    fn numbers(&self) -> Result<Vec<AdrNumber>, StoreError> {
        self.check()?;
        if !self.exists {
            return Err(StoreError("the store does not exist".to_owned()));
        }
        let mut numbers: Vec<AdrNumber> = self.adrs.iter().map(|adr| adr.number).collect();
        numbers.extend(self.extra_numbers.iter().copied());
        numbers.retain(|number| !number.is_template());
        Ok(numbers)
    }

    fn template(&self) -> Result<Option<String>, StoreError> {
        self.check()?;
        Ok(self.template.clone())
    }
}

impl WritesAdrs for MemoryStore {
    fn prepare(&mut self) -> Result<Prepared, StoreError> {
        self.check()?;
        let place = Location("memory:store".to_owned());
        let mut prepared = Prepared::default();
        if self.exists {
            prepared.kept.push(place);
        } else {
            self.exists = true;
            prepared.created.push(place);
        }
        Ok(prepared)
    }

    fn create(&mut self, adr: &Adr) -> Result<Location, StoreError> {
        self.check()?;
        let taken = self.adrs.iter().any(|held| held.number == adr.number)
            || self.extra_numbers.contains(&adr.number);
        if taken {
            return Err(StoreError(format!("ADR {} already exists", adr.number)));
        }
        self.adrs.push(adr.clone());
        Ok(Location(format!("memory:{}", adr.number)))
    }
}

/// User settings held in memory.
#[derive(Debug, Default)]
pub struct MemoryUserSettings {
    /// The user's name, if one is set.
    pub name: Option<DisplayName>,
    /// A failure that every read or write returns while it is set.
    pub fail_with: Option<String>,
}

impl KeepsUserSettings for MemoryUserSettings {
    fn name(&self) -> Result<Option<DisplayName>, StoreError> {
        match &self.fail_with {
            Some(message) => Err(StoreError(message.clone())),
            None => Ok(self.name.clone()),
        }
    }

    fn set_name(&mut self, name: &DisplayName) -> Result<(), StoreError> {
        if let Some(message) = &self.fail_with {
            return Err(StoreError(message.clone()));
        }
        self.name = Some(name.clone());
        Ok(())
    }
}

/// An asker that gives the same answer each time, and counts how often it is asked.
#[derive(Debug, Default)]
pub struct ScriptedAnswer {
    /// The name to answer with, or `None` for no answer.
    pub answer: Option<DisplayName>,
    /// How many times it has been asked.
    pub times_asked: usize,
    /// A failure to return instead of an answer.
    pub fail_with: Option<String>,
}

impl ScriptedAnswer {
    /// An asker that answers with `name`.
    pub fn saying(name: DisplayName) -> Self {
        Self {
            answer: Some(name),
            ..Self::default()
        }
    }
}

impl AsksForName for ScriptedAnswer {
    fn ask(&mut self) -> Result<Option<DisplayName>, StoreError> {
        self.times_asked += 1;
        match &self.fail_with {
            Some(message) => Err(StoreError(message.clone())),
            None => Ok(self.answer.clone()),
        }
    }
}
