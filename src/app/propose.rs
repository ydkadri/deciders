//! The `propose` use case: add a new ADR in the `proposed` state.

use crate::domain::date::Date;
use crate::domain::number::AdrNumber;
use crate::domain::propose::{self, BuildError};
use crate::domain::template;
use crate::ports::{KeepsUserSettings, Location, ReadsAdrs, StoreError, WritesAdrs};

/// A proposal that could not be made.
#[derive(Debug, thiserror::Error)]
pub enum ProposeError {
    /// The store or the user's settings could not be read or written.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Every possible ADR number is already used.
    #[error("no ADR number is left, the highest possible one is already used")]
    NoNumberLeft,
    /// The title or template cannot make an ADR.
    #[error(transparent)]
    Build(#[from] BuildError),
}

/// What a proposal made.
#[derive(Debug, PartialEq, Eq)]
pub struct Proposed {
    /// The number given to the new ADR.
    pub number: AdrNumber,
    /// Where the store put it.
    pub location: Location,
    /// Whether the user has no name set, so the Proposed line has no author.
    pub author_missing: bool,
}

/// Propose an ADR titled `title`, dated `today`.
///
/// The number is one more than the highest in the store. The sections come from
/// the store's template, or the built-in one. The author is the name in the
/// user's settings, if there is one.
///
/// # Errors
///
/// Returns [`ProposeError`] if the store or settings fail, the title or template
/// cannot make an ADR, or no number is left.
pub fn propose<S, U>(
    store: &mut S,
    user: &U,
    title: &str,
    today: Date,
) -> Result<Proposed, ProposeError>
where
    S: ReadsAdrs + WritesAdrs,
    U: KeepsUserSettings,
{
    let number = AdrNumber::next_free(store.numbers()?).ok_or(ProposeError::NoNumberLeft)?;
    let stored_template = store.template()?;
    let author = user.name()?;
    let adr = propose::build(
        number,
        title,
        author.as_ref(),
        today,
        template::choose(stored_template.as_deref()),
    )?;
    let location = store.create(&adr)?;
    Ok(Proposed {
        number,
        location,
        author_missing: author.is_none(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::{MemoryStore, MemoryUserSettings};
    use crate::domain::person::DisplayName;

    fn date() -> Date {
        "2026-10-02".parse().unwrap()
    }

    fn store() -> MemoryStore {
        MemoryStore {
            exists: true,
            ..MemoryStore::default()
        }
    }

    fn user(name: Option<&str>) -> MemoryUserSettings {
        MemoryUserSettings {
            name: name.map(|name| DisplayName::new(name).unwrap()),
            fail_with: None,
        }
    }

    #[test]
    fn the_first_adr_is_number_one() {
        let mut store = store();
        let proposed = propose(&mut store, &user(Some("Ada")), "Use Postgres", date()).unwrap();
        assert_eq!(proposed.number, AdrNumber::new(1), "first number");
        assert_eq!(store.adrs.len(), 1, "one ADR stored");
    }

    #[test]
    fn the_number_follows_the_highest_including_ones_that_are_not_parsed() {
        let mut store = store();
        store.extra_numbers = vec![AdrNumber::new(0), AdrNumber::new(9)];
        let proposed = propose(&mut store, &user(None), "Next", date()).unwrap();
        assert_eq!(proposed.number, AdrNumber::new(10), "after 9");
    }

    #[test]
    fn the_author_is_the_users_name() {
        let mut store = store();
        propose(&mut store, &user(Some("Ada Lovelace")), "T", date()).unwrap();
        assert_eq!(
            store.adrs[0]
                .header
                .proposed
                .as_ref()
                .and_then(|stage| stage.by()),
            Some("Ada Lovelace"),
            "author"
        );
    }

    #[test]
    fn without_a_name_the_line_has_no_author_and_the_caller_is_told() {
        let mut store = store();
        let proposed = propose(&mut store, &user(None), "T", date()).unwrap();
        assert!(proposed.author_missing, "flagged");
        assert_eq!(
            store.adrs[0]
                .header
                .proposed
                .as_ref()
                .and_then(|stage| stage.by()),
            None,
            "no author"
        );
    }

    #[test]
    fn the_stores_template_is_used_when_it_has_one() {
        let mut store = store();
        store.template = Some("# N\n\n**Status:** proposed\n\n## Why\n\nBecause.\n".to_owned());
        propose(&mut store, &user(None), "T", date()).unwrap();
        assert!(store.adrs[0].body.starts_with("## Why"), "custom sections");
    }

    #[test]
    fn the_built_in_template_is_used_otherwise() {
        let mut store = store();
        propose(&mut store, &user(None), "T", date()).unwrap();
        assert!(store.adrs[0].body.contains("## Consequences"), "built in");
    }

    #[test]
    fn a_bad_title_stores_nothing() {
        let mut store = store();
        let error = propose(&mut store, &user(None), "a\nb", date()).unwrap_err();
        assert!(
            matches!(error, ProposeError::Build(BuildError::InvalidTitle)),
            "{error}"
        );
        assert!(store.adrs.is_empty(), "nothing stored");
    }

    #[test]
    fn a_store_failure_is_reported() {
        let mut store = store();
        store.fail_with = Some("disk full".to_owned());
        let error = propose(&mut store, &user(None), "T", date()).unwrap_err();
        assert_eq!(error.to_string(), "disk full", "the store's message");
    }

    #[test]
    fn a_settings_failure_is_reported_and_stores_nothing() {
        let mut store = store();
        let mut settings = user(Some("Ada"));
        settings.fail_with = Some("unreadable".to_owned());
        let error = propose(&mut store, &settings, "T", date()).unwrap_err();
        assert!(matches!(error, ProposeError::Store(_)), "{error}");
        assert!(store.adrs.is_empty(), "nothing stored");
    }

    #[test]
    fn a_missing_store_is_reported() {
        let mut store = MemoryStore::default();
        let error = propose(&mut store, &user(None), "T", date()).unwrap_err();
        assert!(error.to_string().contains("does not exist"), "{error}");
    }

    #[test]
    fn no_number_left_is_reported() {
        let mut store = store();
        store.extra_numbers = vec![AdrNumber::new(u32::MAX)];
        let error = propose(&mut store, &user(None), "T", date()).unwrap_err();
        assert!(matches!(error, ProposeError::NoNumberLeft), "{error}");
    }
}
