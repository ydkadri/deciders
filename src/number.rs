//! ADR numbers. The filename owns the number (ADR 0002).

use std::fmt;
use std::str::FromStr;

/// The number of an ADR, written with at least four digits (`0007`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdrNumber(u32);

/// A value that is not a whole number written in digits, or is too large to hold.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "invalid ADR number {value:?}, expected digits only, such as 7 or 0007, no larger than 4294967295"
)]
pub struct NumberError {
    value: String,
}

/// The shortest run of digits a filename may use for its number.
const FILENAME_DIGITS: usize = 4;

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

impl AdrNumber {
    /// The reserved number of the template, `0000-template.md`.
    pub const TEMPLATE: Self = Self(0);

    /// Read the number from an ADR filename such as `0007-use-postgres.md`.
    ///
    /// Returns `None` for anything else, such as `README.md`, so callers can
    /// skip files that are not ADRs.
    pub fn from_filename(name: &str) -> Option<Self> {
        let stem = name.strip_suffix(".md")?;
        let (digits, title) = stem.split_once('-')?;
        if digits.len() < FILENAME_DIGITS || title.is_empty() || !all_digits(digits) {
            return None;
        }
        digits.parse().ok().map(Self)
    }

    /// Wrap a plain number.
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// The plain number.
    pub fn get(self) -> u32 {
        self.0
    }

    /// Whether this is the reserved template number, which the tool skips.
    pub fn is_template(self) -> bool {
        self == Self::TEMPLATE
    }
}

impl fmt::Display for AdrNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:04}", self.0)
    }
}

impl FromStr for AdrNumber {
    type Err = NumberError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || NumberError {
            value: text.to_owned(),
        };
        if !all_digits(text) {
            return Err(invalid());
        }
        text.parse().map(Self).map_err(|_| invalid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_with_four_digits() {
        assert_eq!(
            AdrNumber::new(7).to_string(),
            "0007",
            "short numbers are padded"
        );
        assert_eq!(
            AdrNumber::new(12345).to_string(),
            "12345",
            "long numbers are not cut"
        );
    }

    #[test]
    fn parses_padded_and_unpadded_digits() {
        assert_eq!(
            "0007".parse::<AdrNumber>(),
            Ok(AdrNumber::new(7)),
            "padded form"
        );
        assert_eq!(
            "7".parse::<AdrNumber>(),
            Ok(AdrNumber::new(7)),
            "unpadded form"
        );
    }

    #[test]
    fn rejects_text_that_is_not_digits() {
        for text in ["", "abc", "-1", "+7", "7a", " 7"] {
            assert!(
                text.parse::<AdrNumber>().is_err(),
                "{text:?} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_a_number_too_large_to_hold() {
        let error = "99999999999".parse::<AdrNumber>().unwrap_err();
        assert!(error.to_string().contains("99999999999"), "{error}");
    }

    #[test]
    fn a_filename_number_too_large_to_hold_is_not_an_adr() {
        assert_eq!(
            AdrNumber::from_filename("99999999999-huge.md"),
            None,
            "it cannot be numbered"
        );
    }

    #[test]
    fn reads_the_number_from_an_adr_filename() {
        assert_eq!(
            AdrNumber::from_filename("0007-use-postgres.md"),
            Some(AdrNumber::new(7)),
            "ordinary ADR"
        );
        assert_eq!(
            AdrNumber::from_filename("12345-later.md"),
            Some(AdrNumber::new(12345)),
            "five digits"
        );
    }

    #[test]
    fn ignores_files_that_are_not_adrs() {
        let names = [
            "README.md",
            "0007.md",
            "0007-.md",
            "7-short.md",
            "0007-notes.txt",
            "abcd-notes.md",
            "+007-notes.md",
            "",
        ];
        for name in names {
            assert_eq!(
                AdrNumber::from_filename(name),
                None,
                "{name:?} is not an ADR"
            );
        }
    }

    #[test]
    fn the_template_is_number_zero() {
        let template = AdrNumber::from_filename("0000-template.md").unwrap();
        assert!(template.is_template(), "0000 is reserved for the template");
        assert!(!AdrNumber::new(1).is_template(), "0001 is a real ADR");
        assert_eq!(template.get(), 0, "the template is number zero");
    }

    #[test]
    fn orders_numerically() {
        assert!(AdrNumber::new(9) < AdrNumber::new(10), "9 comes before 10");
    }
}
