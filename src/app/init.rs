//! The `init` use case: prepare the store and record the user's name.

use tracing::debug;

use crate::domain::person::DisplayName;
use crate::ports::{AsksForName, KeepsUserSettings, Prepared, StoreError, WritesAdrs};

/// What `init` did to the user's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameOutcome {
    /// A name was recorded.
    Set(DisplayName),
    /// A name was already set, so it was left alone.
    Kept(DisplayName),
    /// No name was given, so none was recorded.
    Missing,
}

/// What `init` did.
#[derive(Debug, PartialEq, Eq)]
pub struct Initialised {
    /// What happened to the store.
    pub store: Prepared,
    /// What happened to the user's name.
    pub name: NameOutcome,
}

/// Prepare the store and make sure the user has a name.
///
/// A name that is already set is kept. Otherwise `chosen` (the name given as an
/// option) is recorded, and if there is none, `asker` is asked. If that gives no
/// name either, nothing is recorded. Working out a suggestion is the asker's job,
/// because only it knows whether a person is there.
///
/// # Errors
///
/// Returns [`StoreError`] if the store cannot be prepared, the settings cannot
/// be read or written, or the question fails.
pub fn init<S, U, A>(
    store: &mut S,
    user: &mut U,
    chosen: Option<DisplayName>,
    asker: &mut A,
) -> Result<Initialised, StoreError>
where
    S: WritesAdrs,
    U: KeepsUserSettings,
    A: AsksForName,
{
    let prepared = store.prepare()?;
    if let Some(existing) = user.name()? {
        return Ok(Initialised {
            store: prepared,
            name: NameOutcome::Kept(existing),
        });
    }
    let given = match chosen {
        Some(name) => Some(name),
        None => asker.ask()?,
    };
    let name = match given {
        Some(name) => {
            user.set_name(&name)?;
            debug!(%name, "recorded the user's name");
            NameOutcome::Set(name)
        }
        None => NameOutcome::Missing,
    };
    Ok(Initialised {
        store: prepared,
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::{MemoryStore, MemoryUserSettings, ScriptedAnswer};

    fn name(text: &str) -> DisplayName {
        DisplayName::new(text).unwrap()
    }

    fn nobody() -> ScriptedAnswer {
        ScriptedAnswer::default()
    }

    #[test]
    fn a_new_store_is_created() {
        let mut store = MemoryStore::default();
        let result = init(
            &mut store,
            &mut MemoryUserSettings::default(),
            None,
            &mut nobody(),
        )
        .unwrap();
        assert_eq!(result.store.created.len(), 1, "created");
        assert!(result.store.kept.is_empty(), "nothing kept");
        assert!(store.exists, "now exists");
    }

    #[test]
    fn an_existing_store_is_kept() {
        let mut store = MemoryStore {
            exists: true,
            ..MemoryStore::default()
        };
        let result = init(
            &mut store,
            &mut MemoryUserSettings::default(),
            None,
            &mut nobody(),
        )
        .unwrap();
        assert!(result.store.created.is_empty(), "nothing created");
        assert_eq!(result.store.kept.len(), 1, "kept");
    }

    #[test]
    fn the_chosen_name_is_recorded_without_asking() {
        let mut settings = MemoryUserSettings::default();
        let mut asker = ScriptedAnswer::saying(name("Asked"));
        let result = init(
            &mut MemoryStore::default(),
            &mut settings,
            Some(name("Ada")),
            &mut asker,
        )
        .unwrap();
        assert_eq!(result.name, NameOutcome::Set(name("Ada")), "chosen");
        assert_eq!(settings.name, Some(name("Ada")), "recorded");
        assert_eq!(asker.times_asked, 0, "nobody was asked");
    }

    #[test]
    fn with_no_chosen_name_the_answer_is_recorded() {
        let mut settings = MemoryUserSettings::default();
        let mut asker = ScriptedAnswer::saying(name("Grace Hopper"));
        let result = init(&mut MemoryStore::default(), &mut settings, None, &mut asker).unwrap();
        assert_eq!(
            result.name,
            NameOutcome::Set(name("Grace Hopper")),
            "answer"
        );
        assert_eq!(asker.times_asked, 1, "asked once");
    }

    #[test]
    fn no_name_at_all_records_nothing_and_says_so() {
        let mut settings = MemoryUserSettings::default();
        let result = init(
            &mut MemoryStore::default(),
            &mut settings,
            None,
            &mut nobody(),
        )
        .unwrap();
        assert_eq!(result.name, NameOutcome::Missing, "missing");
        assert_eq!(settings.name, None, "nothing recorded");
    }

    #[test]
    fn an_existing_name_is_kept_and_nobody_is_asked() {
        let mut settings = MemoryUserSettings {
            name: Some(name("Existing")),
            fail_with: None,
        };
        let mut asker = ScriptedAnswer::saying(name("Asked"));
        let result = init(
            &mut MemoryStore::default(),
            &mut settings,
            Some(name("Other")),
            &mut asker,
        )
        .unwrap();
        assert_eq!(result.name, NameOutcome::Kept(name("Existing")), "kept");
        assert_eq!(settings.name, Some(name("Existing")), "unchanged");
        assert_eq!(
            asker.times_asked, 0,
            "the rule lives here, not in the caller"
        );
    }

    #[test]
    fn running_twice_changes_nothing_the_second_time() {
        let mut store = MemoryStore::default();
        let mut settings = MemoryUserSettings::default();
        init(&mut store, &mut settings, Some(name("Ada")), &mut nobody()).unwrap();
        let second = init(
            &mut store,
            &mut settings,
            Some(name("Grace")),
            &mut nobody(),
        )
        .unwrap();
        assert!(second.store.created.is_empty(), "store kept");
        assert_eq!(second.name, NameOutcome::Kept(name("Ada")), "name kept");
    }

    #[test]
    fn a_store_failure_is_reported_before_the_name_is_touched() {
        let mut store = MemoryStore {
            fail_with: Some("read-only".to_owned()),
            ..MemoryStore::default()
        };
        let mut settings = MemoryUserSettings::default();
        let mut asker = ScriptedAnswer::saying(name("Ada"));
        let error = init(&mut store, &mut settings, None, &mut asker).unwrap_err();
        assert_eq!(error.to_string(), "read-only", "the store's message");
        assert_eq!(settings.name, None, "name not recorded");
        assert_eq!(asker.times_asked, 0, "not asked");
    }

    #[test]
    fn a_settings_failure_is_reported() {
        let mut settings = MemoryUserSettings {
            name: None,
            fail_with: Some("unwritable".to_owned()),
        };
        let error = init(
            &mut MemoryStore::default(),
            &mut settings,
            Some(name("Ada")),
            &mut nobody(),
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "unwritable", "the settings' message");
    }

    #[test]
    fn a_failed_question_is_reported() {
        let mut asker = ScriptedAnswer {
            fail_with: Some("terminal closed".to_owned()),
            ..ScriptedAnswer::default()
        };
        let error = init(
            &mut MemoryStore::default(),
            &mut MemoryUserSettings::default(),
            None,
            &mut asker,
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "terminal closed", "the asker's message");
    }
}
