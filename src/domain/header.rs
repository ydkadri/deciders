//! The bold-line header block of an ADR (ADR 0002).
//!
//! The block is a run of `**Label:** value` lines between the H1 and the first
//! H2. Parsing is strict about shape. Whether the lines make sense for the
//! status, such as dates being in order, is the job of `check`, not the parser.

use std::fmt;

use crate::domain::date::{Date, DateError};
use crate::domain::number::{AdrNumber, NumberError};
use crate::domain::status::Status;

/// What is wrong with an author name or a reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueProblem {
    /// Nothing but whitespace.
    Empty,
    /// Contains a line break, which would start a new header line.
    LineBreak,
    /// Starts or ends with whitespace, which reading would trim away.
    Padding,
    /// Contains a parenthesis, which marks the start of a reference list.
    Parenthesis,
    /// A reference that contains a comma, which separates references.
    Comma,
}

/// A stage value that does not read as `DATE [by NAME] [(REF, REF)]`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StageError {
    /// The leading date is not a real `YYYY-MM-DD` date.
    #[error(transparent)]
    Date(#[from] DateError),
    /// Text after the date that is neither `by NAME` nor a reference list.
    #[error("expected `by NAME` after the date, found {0:?}")]
    UnexpectedText(String),
    /// A `(` with no closing `)`.
    #[error("the reference list is missing its closing bracket")]
    UnclosedRefs,
    /// Text after the closing `)` of the reference list.
    #[error("unexpected text after the reference list: {0:?}")]
    TextAfterRefs(String),
    /// An author name that cannot be written and read back.
    #[error("invalid author name: it {0}")]
    InvalidAuthor(ValueProblem),
    /// A reference that cannot be written and read back.
    #[error("invalid reference {reference:?}: it {problem}")]
    InvalidRef {
        /// The reference as given.
        reference: String,
        /// What is wrong with it.
        problem: ValueProblem,
    },
}

/// A header that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HeaderError {
    /// A non-blank line in the header block that is not `**Label:** value`.
    #[error("line {line}: expected a header line such as `**Status:** proposed`, found {text:?}")]
    NotAHeaderLine {
        /// The 1-based line number.
        line: usize,
        /// The line as written.
        text: String,
    },
    /// A label with nothing after it.
    #[error("line {line}: `**{label}:**` has no value")]
    EmptyValue {
        /// The 1-based line number.
        line: usize,
        /// The label without its markup.
        label: String,
    },
    /// A known label that appears twice.
    #[error("line {line}: `**{label}:**` appears more than once")]
    Duplicate {
        /// The 1-based line number of the second occurrence.
        line: usize,
        /// The label without its markup.
        label: String,
    },
    /// A known label whose value cannot be read.
    #[error("line {line}: `**{label}:**` {reason}")]
    InvalidValue {
        /// The 1-based line number.
        line: usize,
        /// The label without its markup.
        label: String,
        /// Why the value was rejected.
        reason: String,
    },
    /// No `**Status:**` line at all.
    #[error("the header has no `**Status:**` line")]
    MissingStatus,
}

/// When a stage was reached, who did it, and any references to the work.
///
/// Written as `2026-08-02 by Youcef Kadri (#412, #413)`, where `by` and the
/// references are optional. Only the `Implemented` line carries references.
///
/// The fields are private so that every value can be written and read back
/// unchanged. Build one with [`Stage::new`] or [`Stage::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    date: Date,
    by: Option<String>,
    refs: Vec<String>,
}

const STATUS: &str = "Status";
const PROPOSED: &str = "Proposed";
const ACCEPTED: &str = "Accepted";
const REJECTED: &str = "Rejected";
const IMPLEMENTED: &str = "Implemented";
const SUPERSEDED: &str = "Superseded";
const SUPERSEDED_BY: &str = "Superseded by";
const SUPERSEDES: &str = "Supersedes";

/// Check that `text` survives being written on a header line and read back.
pub(crate) fn check_value(text: &str, is_reference: bool) -> Result<(), ValueProblem> {
    if text.trim().is_empty() {
        Err(ValueProblem::Empty)
    } else if text.contains(['\n', '\r']) {
        Err(ValueProblem::LineBreak)
    } else if text != text.trim() {
        Err(ValueProblem::Padding)
    } else if text.contains(['(', ')']) {
        Err(ValueProblem::Parenthesis)
    } else if is_reference && text.contains(',') {
        Err(ValueProblem::Comma)
    } else {
        Ok(())
    }
}

/// Split `**Label:** value` into its label and trimmed value.
fn split_field(text: &str) -> Option<(&str, &str)> {
    let (label, value) = text.strip_prefix("**")?.split_once(":**")?;
    if label.is_empty() || !(value.is_empty() || value.starts_with(' ')) {
        return None;
    }
    Some((label, value.trim()))
}

fn store<T>(slot: &mut Option<T>, value: T, line: usize, label: &str) -> Result<(), HeaderError> {
    match slot.replace(value) {
        None => Ok(()),
        Some(_) => Err(HeaderError::Duplicate {
            line,
            label: label.to_owned(),
        }),
    }
}

fn invalid(line: usize, label: &str, reason: impl fmt::Display) -> HeaderError {
    HeaderError::InvalidValue {
        line,
        label: label.to_owned(),
        reason: reason.to_string(),
    }
}

fn parse_numbers(value: &str) -> Result<Vec<AdrNumber>, NumberError> {
    value.split(',').map(|part| part.trim().parse()).collect()
}

/// Split a trailing `(REF, REF)` off `rest`, returning the text before it.
fn split_refs(rest: &str) -> Result<(&str, Vec<String>), StageError> {
    let Some(open) = rest.rfind('(') else {
        return Ok((rest, Vec::new()));
    };
    let (before, list) = rest.split_at(open);
    let Some(inner) = list
        .strip_prefix('(')
        .and_then(|list| list.strip_suffix(')'))
    else {
        return Err(match list.split_once(')') {
            Some((_, after)) => StageError::TextAfterRefs(after.trim().to_owned()),
            None => StageError::UnclosedRefs,
        });
    };
    let refs = inner
        .split(',')
        .map(|entry| entry.trim().to_owned())
        .collect();
    Ok((before.trim(), refs))
}

impl fmt::Display for ValueProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "is empty",
            Self::LineBreak => "contains a line break",
            Self::Padding => "has whitespace at the start or end",
            Self::Parenthesis => "contains a parenthesis",
            Self::Comma => "contains a comma",
        })
    }
}

impl Stage {
    /// Build a stage, checking that it can be written and read back.
    ///
    /// # Errors
    ///
    /// Returns [`StageError::InvalidAuthor`] if `by` is empty, padded, spans
    /// lines or contains a parenthesis, and [`StageError::InvalidRef`] if a
    /// reference is empty, padded, spans lines or contains a parenthesis or a
    /// comma. A caller with a name that has parentheses, such as
    /// `Jane (Contractor)`, must change it first.
    pub fn new(date: Date, by: Option<String>, refs: Vec<String>) -> Result<Self, StageError> {
        if let Some(name) = &by {
            check_value(name, false).map_err(StageError::InvalidAuthor)?;
        }
        for reference in &refs {
            check_value(reference, true).map_err(|problem| StageError::InvalidRef {
                reference: reference.clone(),
                problem,
            })?;
        }
        Ok(Self { date, by, refs })
    }

    /// Read a stage value such as `2026-08-02 by Ada (#412)`.
    ///
    /// References are read only when `allow_refs` is true, which is for the
    /// `Implemented` line. On other lines a bracket is an error.
    ///
    /// # Errors
    ///
    /// Returns [`StageError`] if the date is not real, the text after it is not
    /// `by NAME`, the reference list is malformed, or a value fails the checks
    /// in [`Stage::new`].
    pub fn parse(text: &str, allow_refs: bool) -> Result<Self, StageError> {
        let text = text.trim();
        let (date_text, rest) = text.split_once(' ').unwrap_or((text, ""));
        let date = date_text.parse()?;
        let rest = rest.trim();

        let (rest, refs) = if allow_refs {
            split_refs(rest)?
        } else {
            (rest, Vec::new())
        };
        let by = if rest.is_empty() {
            None
        } else {
            let name = rest
                .strip_prefix("by ")
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| StageError::UnexpectedText(rest.to_owned()))?;
            Some(name.to_owned())
        };
        Self::new(date, by, refs)
    }

    /// The day the stage was reached.
    pub fn date(&self) -> Date {
        self.date
    }

    /// Who moved the ADR to this stage.
    pub fn by(&self) -> Option<&str> {
        self.by.as_deref()
    }

    /// Pull requests, commits or issues, such as `#412`.
    pub fn refs(&self) -> &[String] {
        &self.refs
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.date)?;
        if let Some(name) = &self.by {
            write!(formatter, " by {name}")?;
        }
        if !self.refs.is_empty() {
            write!(formatter, " ({})", self.refs.join(", "))?;
        }
        Ok(())
    }
}

/// The parsed header of one ADR.
///
/// Lines whose labels the tool does not know are kept in [`Header::other`] and
/// written back unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The current status.
    pub status: Status,
    /// When the ADR was proposed.
    pub proposed: Option<Stage>,
    /// When the ADR was accepted.
    pub accepted: Option<Stage>,
    /// When the ADR was rejected.
    pub rejected: Option<Stage>,
    /// When the ADR was implemented.
    pub implemented: Option<Stage>,
    /// When the ADR was superseded.
    pub superseded: Option<Stage>,
    /// The ADR that replaced this one.
    pub superseded_by: Option<AdrNumber>,
    /// The ADRs this one replaces.
    pub supersedes: Vec<AdrNumber>,
    /// Lines with other labels, as `(label, value)` in file order.
    pub other: Vec<(String, String)>,
}

impl Header {
    /// An empty header: the given status, no stage lines and no other lines.
    pub fn new(status: Status) -> Self {
        Self {
            status,
            proposed: None,
            accepted: None,
            rejected: None,
            implemented: None,
            superseded: None,
            superseded_by: None,
            supersedes: Vec::new(),
            other: Vec::new(),
        }
    }

    /// Read the header from its numbered lines.
    ///
    /// Blank lines are skipped. Every other line must be `**Label:** value`.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderError`] for a line of the wrong shape, an empty or
    /// repeated label, a value that cannot be read, or a missing status.
    pub(crate) fn parse<'a>(
        lines: impl IntoIterator<Item = (usize, &'a str)>,
    ) -> Result<Self, HeaderError> {
        let mut status = None;
        let mut header = Self::new(Status::Proposed);

        for (line, text) in lines {
            if text.trim().is_empty() {
                continue;
            }
            let (label, value) =
                split_field(text.trim()).ok_or_else(|| HeaderError::NotAHeaderLine {
                    line,
                    text: text.trim().to_owned(),
                })?;
            if value.is_empty() {
                return Err(HeaderError::EmptyValue {
                    line,
                    label: label.to_owned(),
                });
            }
            let stage = |allow_refs: bool| {
                Stage::parse(value, allow_refs).map_err(|error| invalid(line, label, error))
            };
            let number = || {
                value
                    .parse::<AdrNumber>()
                    .map_err(|error| invalid(line, label, error))
            };
            match label {
                STATUS => store(
                    &mut status,
                    value
                        .parse::<Status>()
                        .map_err(|error| invalid(line, label, error))?,
                    line,
                    label,
                )?,
                PROPOSED => store(&mut header.proposed, stage(false)?, line, label)?,
                ACCEPTED => store(&mut header.accepted, stage(false)?, line, label)?,
                REJECTED => store(&mut header.rejected, stage(false)?, line, label)?,
                IMPLEMENTED => store(&mut header.implemented, stage(true)?, line, label)?,
                SUPERSEDED => store(&mut header.superseded, stage(false)?, line, label)?,
                SUPERSEDED_BY => store(&mut header.superseded_by, number()?, line, label)?,
                SUPERSEDES => {
                    if !header.supersedes.is_empty() {
                        return Err(HeaderError::Duplicate {
                            line,
                            label: label.to_owned(),
                        });
                    }
                    header.supersedes =
                        parse_numbers(value).map_err(|error| invalid(line, label, error))?;
                }
                _ => header.other.push((label.to_owned(), value.to_owned())),
            }
        }

        header.status = status.ok_or(HeaderError::MissingStatus)?;
        Ok(header)
    }
}

impl fmt::Display for Header {
    /// Writes the header lines in a fixed order, separated by newlines, with
    /// no trailing newline. Lines with unknown labels come last, in file order.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "**{STATUS}:** {}", self.status)?;
        let stages = [
            (PROPOSED, &self.proposed),
            (ACCEPTED, &self.accepted),
            (REJECTED, &self.rejected),
            (IMPLEMENTED, &self.implemented),
            (SUPERSEDED, &self.superseded),
        ];
        for (label, stage) in stages {
            if let Some(stage) = stage {
                write!(formatter, "\n**{label}:** {stage}")?;
            }
        }
        if let Some(number) = self.superseded_by {
            write!(formatter, "\n**{SUPERSEDED_BY}:** {number}")?;
        }
        if !self.supersedes.is_empty() {
            let numbers: Vec<String> = self.supersedes.iter().map(ToString::to_string).collect();
            write!(formatter, "\n**{SUPERSEDES}:** {}", numbers.join(", "))?;
        }
        for (label, value) in &self.other {
            write!(formatter, "\n**{label}:** {value}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Header, HeaderError> {
        Header::parse(
            text.lines()
                .enumerate()
                .map(|(index, line)| (index + 1, line)),
        )
    }

    fn stage(text: &str) -> Result<Stage, StageError> {
        Stage::parse(text, true)
    }

    #[test]
    fn a_new_header_writes_only_its_status() {
        assert_eq!(
            Header::new(Status::Accepted).to_string(),
            "**Status:** accepted",
            "no stage lines"
        );
    }

    #[test]
    fn parses_a_status_only_header() {
        let header = parse("**Status:** proposed").unwrap();
        assert_eq!(header.status, Status::Proposed, "status");
        assert_eq!(header.proposed, None, "no stage lines were given");
    }

    #[test]
    fn parses_every_stage_line() {
        let header = parse(
            "**Status:** superseded\n\
             **Proposed:** 2026-07-29 by Ada\n\
             **Accepted:** 2026-07-30 by Ada\n\
             **Implemented:** 2026-08-02 (#412)\n\
             **Superseded:** 2026-09-01 by Grace\n\
             **Superseded by:** 0009",
        )
        .unwrap();
        assert_eq!(header.status, Status::Superseded, "status");
        assert_eq!(
            header.proposed.unwrap().date(),
            "2026-07-29".parse().unwrap(),
            "proposed date"
        );
        assert_eq!(header.accepted.unwrap().by(), Some("Ada"), "accepted by");
        assert_eq!(header.implemented.unwrap().refs(), ["#412"], "refs");
        assert_eq!(
            header.superseded.unwrap().by(),
            Some("Grace"),
            "superseded by"
        );
        assert_eq!(header.superseded_by, Some(AdrNumber::new(9)), "replacement");
    }

    #[test]
    fn parses_and_writes_a_list_of_superseded_adrs() {
        let header = parse("**Status:** accepted\n**Supersedes:** 0001, 0002").unwrap();
        assert_eq!(
            header.supersedes,
            [AdrNumber::new(1), AdrNumber::new(2)],
            "supersedes list"
        );
        assert_eq!(
            header.to_string(),
            "**Status:** accepted\n**Supersedes:** 0001, 0002",
            "the list is written comma separated"
        );
    }

    #[test]
    fn keeps_unknown_lines_in_file_order() {
        let header = parse("**Depends on:** 0003\n**Status:** accepted\n**Owner:** Ada").unwrap();
        assert_eq!(
            header.other,
            [
                ("Depends on".to_owned(), "0003".to_owned()),
                ("Owner".to_owned(), "Ada".to_owned())
            ],
            "unknown lines"
        );
    }

    #[test]
    fn skips_blank_lines() {
        assert!(
            parse("\n**Status:** proposed\n\n").is_ok(),
            "blank lines are allowed"
        );
    }

    #[test]
    fn writes_lines_in_a_fixed_order() {
        let header = parse(
            "**Owner:** Ada\n\
             **Accepted:** 2026-07-30\n\
             **Status:** accepted\n\
             **Proposed:** 2026-07-29 by Ada",
        )
        .unwrap();
        assert_eq!(
            header.to_string(),
            "**Status:** accepted\n**Proposed:** 2026-07-29 by Ada\n**Accepted:** 2026-07-30\n**Owner:** Ada",
            "canonical order with unknown lines last"
        );
    }

    #[test]
    fn a_missing_status_is_an_error() {
        assert_eq!(
            parse("**Proposed:** 2026-07-29"),
            Err(HeaderError::MissingStatus),
            "no status"
        );
    }

    #[test]
    fn prose_in_the_header_is_an_error() {
        let error = parse("**Status:** proposed\nSome prose").unwrap_err();
        assert_eq!(
            error,
            HeaderError::NotAHeaderLine {
                line: 2,
                text: "Some prose".to_owned()
            },
            "prose line"
        );
    }

    #[test]
    fn malformed_markup_is_an_error() {
        let cases = [
            "Status: proposed",
            "**Status** proposed",
            "**Status:**proposed",
            "****: x",
            "**:** x",
        ];
        for text in cases {
            assert!(
                matches!(parse(text), Err(HeaderError::NotAHeaderLine { .. })),
                "{text:?} should not be a header line"
            );
        }
    }

    #[test]
    fn an_empty_value_is_an_error() {
        assert_eq!(
            parse("**Status:**"),
            Err(HeaderError::EmptyValue {
                line: 1,
                label: "Status".to_owned()
            }),
            "empty status"
        );
    }

    #[test]
    fn a_repeated_status_is_an_error() {
        assert_eq!(
            parse("**Status:** proposed\n**Status:** accepted"),
            Err(HeaderError::Duplicate {
                line: 2,
                label: "Status".to_owned()
            }),
            "duplicate status"
        );
    }

    #[test]
    fn a_repeated_supersedes_line_is_an_error() {
        assert!(
            matches!(
                parse("**Status:** accepted\n**Supersedes:** 0001\n**Supersedes:** 0002"),
                Err(HeaderError::Duplicate { line: 3, .. })
            ),
            "duplicate supersedes"
        );
    }

    #[test]
    fn an_unknown_status_names_the_line_and_label() {
        let error = parse("**Status:** done").unwrap_err();
        assert!(
            matches!(&error, HeaderError::InvalidValue { line: 1, label, .. } if label == "Status"),
            "{error}"
        );
    }

    #[test]
    fn a_bad_replacement_number_names_its_label() {
        let error = parse("**Status:** accepted\n**Superseded by:** soon").unwrap_err();
        assert!(
            matches!(&error, HeaderError::InvalidValue { line: 2, label, .. } if label == "Superseded by"),
            "{error}"
        );
    }

    #[test]
    fn a_replacement_number_that_overflows_is_an_error() {
        let error = parse("**Status:** accepted\n**Superseded by:** 99999999999").unwrap_err();
        assert!(
            matches!(&error, HeaderError::InvalidValue { line: 2, label, .. } if label == "Superseded by"),
            "{error}"
        );
    }

    #[test]
    fn a_trailing_comma_in_supersedes_is_an_error() {
        let error = parse("**Status:** accepted\n**Supersedes:** 0001,").unwrap_err();
        assert!(
            matches!(&error, HeaderError::InvalidValue { line: 2, label, .. } if label == "Supersedes"),
            "{error}"
        );
    }

    #[test]
    fn references_are_only_allowed_on_the_implemented_line() {
        let error = parse("**Status:** proposed\n**Proposed:** 2026-07-29 (#1)").unwrap_err();
        assert!(
            matches!(&error, HeaderError::InvalidValue { line: 2, label, .. } if label == "Proposed"),
            "{error}"
        );
    }

    #[test]
    fn an_author_with_a_parenthesis_is_an_error_on_a_stage_line() {
        let error =
            parse("**Status:** proposed\n**Proposed:** 2026-07-29 by Ada (Countess)").unwrap_err();
        assert!(
            matches!(&error, HeaderError::InvalidValue { line: 2, reason, .. } if reason.contains("parenthesis")),
            "{error}"
        );
    }

    #[test]
    fn a_stage_round_trips() {
        for text in [
            "2026-08-02",
            "2026-08-02 by Ada Lovelace",
            "2026-08-02 (#412)",
            "2026-08-02 by Ada (#412, #413)",
            "2026-08-02 by Lovelace, Ada",
        ] {
            assert_eq!(stage(text).unwrap().to_string(), text, "round trip");
        }
    }

    #[test]
    fn a_stage_rejects_a_date_that_does_not_exist() {
        assert!(
            matches!(stage("2026-02-30"), Err(StageError::Date(_))),
            "30 February"
        );
        assert!(
            matches!(stage("soon"), Err(StageError::Date(_))),
            "not a date"
        );
    }

    #[test]
    fn a_stage_needs_by_before_the_name() {
        assert_eq!(
            stage("2026-07-29 Ada"),
            Err(StageError::UnexpectedText("Ada".to_owned())),
            "no by"
        );
        assert_eq!(
            stage("2026-07-29 by"),
            Err(StageError::UnexpectedText("by".to_owned())),
            "no name"
        );
    }

    #[test]
    fn an_unclosed_reference_list_is_an_error() {
        assert_eq!(
            stage("2026-07-29 (#1"),
            Err(StageError::UnclosedRefs),
            "no bracket"
        );
    }

    #[test]
    fn text_after_the_reference_list_is_an_error() {
        assert_eq!(
            stage("2026-07-29 (#1) late"),
            Err(StageError::TextAfterRefs("late".to_owned())),
            "trailing text"
        );
    }

    #[test]
    fn an_empty_reference_is_an_error() {
        assert_eq!(
            stage("2026-07-29 (#1, )"),
            Err(StageError::InvalidRef {
                reference: String::new(),
                problem: ValueProblem::Empty
            }),
            "empty entry"
        );
    }

    #[test]
    fn stage_new_rejects_an_author_with_a_parenthesis() {
        let date = "2026-07-29".parse().unwrap();
        assert_eq!(
            Stage::new(date, Some("Youcef Kadri (work)".to_owned()), Vec::new()),
            Err(StageError::InvalidAuthor(ValueProblem::Parenthesis)),
            "would be misread as a reference list"
        );
    }

    #[test]
    fn stage_new_rejects_an_author_with_a_line_break() {
        let date = "2026-07-29".parse().unwrap();
        assert_eq!(
            Stage::new(
                date,
                Some("Ada\n**Status:** accepted".to_owned()),
                Vec::new()
            ),
            Err(StageError::InvalidAuthor(ValueProblem::LineBreak)),
            "would inject a header line"
        );
    }

    #[test]
    fn stage_new_rejects_an_empty_author() {
        let date = "2026-07-29".parse().unwrap();
        assert_eq!(
            Stage::new(date, Some(String::new()), Vec::new()),
            Err(StageError::InvalidAuthor(ValueProblem::Empty)),
            "empty name"
        );
    }

    #[test]
    fn stage_new_rejects_a_padded_author() {
        let date = "2026-07-29".parse().unwrap();
        assert_eq!(
            Stage::new(date, Some(" Ada".to_owned()), Vec::new()),
            Err(StageError::InvalidAuthor(ValueProblem::Padding)),
            "reading would trim it"
        );
    }

    #[test]
    fn stage_new_rejects_a_reference_with_a_comma() {
        let date = "2026-07-29".parse().unwrap();
        assert_eq!(
            Stage::new(date, None, vec!["#1, #2".to_owned()]),
            Err(StageError::InvalidRef {
                reference: "#1, #2".to_owned(),
                problem: ValueProblem::Comma
            }),
            "would read back as two references"
        );
    }

    #[test]
    fn stage_new_accepts_a_name_with_a_comma() {
        let date: Date = "2026-07-29".parse().unwrap();
        assert!(
            Stage::new(date, Some("Lovelace, Ada".to_owned()), Vec::new()).is_ok(),
            "a comma is fine in a name"
        );
    }
}
