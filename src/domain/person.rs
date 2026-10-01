//! The name written on the stage lines the tool creates (ADR 0003).

use std::fmt;

use crate::domain::header::{self, ValueProblem};

/// A person's name as it is written after `by` on a stage line.
///
/// Every value can be written on a stage line and read back unchanged, so it
/// is never empty or padded and has no line break or parenthesis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayName(String);

/// A name that cannot be written on a stage line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid name: it {0}")]
pub struct NameError(ValueProblem);

impl DisplayName {
    /// Build a name from text a person typed. Surrounding whitespace is removed.
    ///
    /// # Errors
    ///
    /// Returns [`NameError`] if nothing is left after trimming (whitespace and
    /// line breaks count as nothing), or the name has a line break or a
    /// parenthesis.
    pub fn new(text: &str) -> Result<Self, NameError> {
        let text = text.trim();
        header::check_value(text, false).map_err(NameError)?;
        Ok(Self(text.to_owned()))
    }

    /// Work out a name from a login such as `youcef.kadri`.
    ///
    /// Splits on `.`, `_` and `-` and capitalises each part, so the example
    /// gives `Youcef Kadri`. Returns `None` if nothing usable is left.
    pub fn infer(login: &str) -> Option<Self> {
        let words: Vec<String> = login
            .split(['.', '_', '-'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut characters = part.chars();
                characters.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(characters).collect()
                })
            })
            .collect();
        Self::new(&words.join(" ")).ok()
    }

    /// Turn what a person typed at a prompt into a name.
    ///
    /// An answer of nothing but whitespace accepts `suggestion`, and gives
    /// `None` when there is no suggestion either.
    ///
    /// # Errors
    ///
    /// Returns [`NameError`] if the answer is not empty and is not a valid name.
    pub fn from_answer(answer: &str, suggestion: Option<&Self>) -> Result<Option<Self>, NameError> {
        if answer.trim().is_empty() {
            Ok(suggestion.cloned())
        } else {
            Self::new(answer).map(Some)
        }
    }

    /// The name as written on a stage line.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_trimmed() {
        assert_eq!(
            DisplayName::new("  Ada Lovelace ").unwrap().as_str(),
            "Ada Lovelace",
            "trimmed"
        );
    }

    #[test]
    fn whitespace_and_line_breaks_alone_are_empty() {
        for text in ["", "   ", "\n", " \n \t\r\n "] {
            assert_eq!(
                DisplayName::new(text),
                Err(NameError(ValueProblem::Empty)),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_name_with_a_parenthesis_is_refused() {
        assert_eq!(
            DisplayName::new("Jane (Contractor)"),
            Err(NameError(ValueProblem::Parenthesis)),
            "would be read as references"
        );
    }

    #[test]
    fn a_name_with_a_line_break_inside_is_refused() {
        assert_eq!(
            DisplayName::new("Ada\n**Status:** accepted"),
            Err(NameError(ValueProblem::LineBreak)),
            "would inject a header line"
        );
    }

    #[test]
    fn a_comma_is_fine_in_a_name() {
        assert!(DisplayName::new("Lovelace, Ada").is_ok(), "comma");
    }

    #[test]
    fn the_error_says_what_is_wrong() {
        assert_eq!(
            DisplayName::new("").unwrap_err().to_string(),
            "invalid name: it is empty",
            "message"
        );
    }

    #[test]
    fn a_login_becomes_a_capitalised_name() {
        let cases = [
            ("youcef.kadri", "Youcef Kadri"),
            ("ada_lovelace", "Ada Lovelace"),
            ("grace-hopper", "Grace Hopper"),
            ("root", "Root"),
            ("a.b.c", "A B C"),
            ("émile.zola", "Émile Zola"),
        ];
        for (login, expected) in cases {
            assert_eq!(
                DisplayName::infer(login).map(|name| name.to_string()),
                Some(expected.to_owned()),
                "{login}"
            );
        }
    }

    #[test]
    fn separators_alone_give_no_name() {
        for login in ["", ".", "_-.", "..."] {
            assert_eq!(DisplayName::infer(login), None, "{login:?}");
        }
    }

    #[test]
    fn a_login_with_a_parenthesis_gives_no_name() {
        assert_eq!(DisplayName::infer("bad(name)"), None, "cannot be written");
    }

    #[test]
    fn an_empty_answer_accepts_the_suggestion() {
        let suggestion = DisplayName::new("Ada").unwrap();
        assert_eq!(
            DisplayName::from_answer("  \n", Some(&suggestion)).unwrap(),
            Some(suggestion),
            "suggestion accepted"
        );
    }

    #[test]
    fn an_empty_answer_without_a_suggestion_gives_nothing() {
        assert_eq!(
            DisplayName::from_answer("", None).unwrap(),
            None,
            "nothing to use"
        );
    }

    #[test]
    fn a_typed_answer_wins_over_the_suggestion() {
        let suggestion = DisplayName::new("Ada").unwrap();
        assert_eq!(
            DisplayName::from_answer("Grace Hopper", Some(&suggestion))
                .unwrap()
                .map(|name| name.to_string()),
            Some("Grace Hopper".to_owned()),
            "typed answer"
        );
    }

    #[test]
    fn an_invalid_answer_is_an_error() {
        assert!(
            DisplayName::from_answer("A (B)", None).is_err(),
            "parenthesis"
        );
    }
}
