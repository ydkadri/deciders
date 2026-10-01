//! How the files store names an ADR file: the number and a slug of the title.
//! This is a detail of storing ADRs as files (ADR 0002), not of the model.

use crate::domain::number::AdrNumber;

/// The longest slug written into a filename, in characters.
const MAX_SLUG_CHARS: usize = 60;

/// A title that cannot be turned into a filename.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the title {0:?} has no letters or digits to make a filename from")]
pub struct NoSlug(pub String);

/// Whether `character` is a combining mark, such as the accent in `e` followed
/// by U+0301. It belongs to the letter before it and must not split a word.
///
/// Covers the combining blocks of Unicode that cover the common scripts. A
/// full test needs the Unicode general category, which the standard library
/// does not expose.
fn is_combining_mark(character: char) -> bool {
    matches!(
        character,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
    )
}

/// Lower-case `title`, keep letters, digits and the marks that belong to them,
/// and join the words with `-`.
fn slug(title: &str) -> Option<String> {
    let mut slug = String::new();
    for character in title.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric()
            || (is_combining_mark(character) && !slug.is_empty() && !slug.ends_with('-'))
        {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let shortened: String = slug.chars().take(MAX_SLUG_CHARS).collect();
    let slug = shortened.trim_matches('-');
    if slug.is_empty() {
        None
    } else {
        Some(slug.to_owned())
    }
}

/// The filename for an ADR, such as `0007-use-postgres.md`.
///
/// # Errors
///
/// Returns [`NoSlug`] if the title has no letters or digits.
///
/// # Examples
///
/// ```
/// use decider_adr::adapters::files::naming::file_name;
/// use decider_adr::domain::number::AdrNumber;
///
/// assert_eq!(file_name(AdrNumber::new(7), "Use Postgres!")?, "0007-use-postgres.md");
/// # Ok::<(), decider_adr::adapters::files::naming::NoSlug>(())
/// ```
pub fn file_name(number: AdrNumber, title: &str) -> Result<String, NoSlug> {
    let title = title.trim();
    let slug = slug(title).ok_or_else(|| NoSlug(title.to_owned()))?;
    Ok(format!("{number}-{slug}.md"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filename_has_the_padded_number_and_a_slug() {
        assert_eq!(
            file_name(AdrNumber::new(7), "Use Postgres").unwrap(),
            "0007-use-postgres.md",
            "ordinary title"
        );
    }

    #[test]
    fn punctuation_and_repeated_separators_become_one_hyphen() {
        assert_eq!(
            file_name(AdrNumber::new(1), "  Use  Postgres -- (really!)  ").unwrap(),
            "0001-use-postgres-really.md",
            "runs collapse and the ends are trimmed"
        );
    }

    #[test]
    fn letters_and_digits_from_other_scripts_are_kept() {
        assert_eq!(
            file_name(AdrNumber::new(2), "Été 2026").unwrap(),
            "0002-été-2026.md",
            "lower-cased and kept"
        );
    }

    #[test]
    fn a_decomposed_accent_stays_inside_its_word() {
        // "Re" + "s" + "ume" with U+0301 written as a separate character after each "e".
        let decomposed = "Re\u{0301}sume\u{0301} I\u{0307}stanbul";
        let name = file_name(AdrNumber::new(2), decomposed).unwrap();
        assert_eq!(
            name,
            "0002-re\u{0301}sume\u{0301}-i\u{0307}stanbul.md".to_lowercase(),
            "{name}"
        );
        assert_eq!(
            name.matches('-').count(),
            2,
            "only the number and one space became hyphens: {name}"
        );
    }

    #[test]
    fn a_mark_with_nothing_before_it_does_not_start_a_slug() {
        assert_eq!(
            file_name(AdrNumber::new(1), "\u{0301}abc").unwrap(),
            "0001-abc.md",
            "a leading mark is dropped"
        );
    }

    #[test]
    fn a_mark_after_a_hyphen_is_dropped() {
        assert_eq!(
            file_name(AdrNumber::new(1), "a \u{0301}b").unwrap(),
            "0001-a-b.md",
            "a mark after a separator has no letter to belong to"
        );
    }

    #[test]
    fn a_long_title_is_cut_without_a_trailing_hyphen() {
        let title = format!("{} end", "word ".repeat(20));
        let name = file_name(AdrNumber::new(3), &title).unwrap();
        let slug = name
            .strip_prefix("0003-")
            .unwrap()
            .strip_suffix(".md")
            .unwrap();
        assert!(slug.chars().count() <= MAX_SLUG_CHARS, "{slug}");
        assert!(!slug.ends_with('-'), "{slug}");
    }

    #[test]
    fn a_title_with_nothing_to_make_a_filename_from_is_refused() {
        assert_eq!(
            file_name(AdrNumber::new(1), "?!…"),
            Err(NoSlug("?!…".to_owned())),
            "no letters or digits"
        );
    }

    #[test]
    fn the_filename_reads_back_as_the_same_number() {
        let name = file_name(AdrNumber::new(42), "Anything at all").unwrap();
        assert_eq!(
            AdrNumber::from_filename(&name),
            Some(AdrNumber::new(42)),
            "{name}"
        );
    }
}
