//! One ADR document: title, header and body.

use crate::header::{Header, HeaderError};
use crate::number::AdrNumber;

/// The line ending a file uses, which is kept when it is written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// `\n`.
    Lf,
    /// `\r\n`.
    CrLf,
}

/// A parsed ADR.
///
/// The body is everything from the first `## ` heading onwards and is kept
/// exactly as written, so hand-written prose is never altered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adr {
    /// The number in the H1. The filename owns the number, and `check` compares them.
    pub number: AdrNumber,
    /// The title in the H1.
    pub title: String,
    /// The bold-line header.
    pub header: Header,
    /// The sections, verbatim. Empty if the ADR has none.
    pub body: String,
    /// The line ending of the first line, used for the title and header when
    /// writing. The body keeps whatever endings it had.
    pub line_ending: LineEnding,
}

/// An ADR that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdrError {
    /// The file is empty.
    #[error("the file is empty, expected a title such as `# 0001. Use Postgres`")]
    Empty,
    /// The first line is not `# NNNN. Title`.
    #[error("line 1: expected a title such as `# 0001. Use Postgres`, found {0:?}")]
    InvalidTitle(String),
    /// The header block cannot be read.
    #[error(transparent)]
    Header(#[from] HeaderError),
}

/// An ADR that cannot be written so that it reads back unchanged.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// The text written would not parse. The source says what failed to parse,
    /// and its line numbers count lines of the text that would have been
    /// written, which the caller never sees.
    #[error("the ADR would not read back")]
    Unreadable(#[source] AdrError),
    /// The text written would parse, but to different data from what the ADR
    /// holds.
    #[error("the ADR would read back as different data from what it holds")]
    ReadsBackDifferently,
}

/// Read `NNNN. Title` from the H1 text after `# `.
fn parse_title(line: &str) -> Option<(AdrNumber, String)> {
    let (digits, title) = line.strip_prefix("# ")?.split_once(". ")?;
    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    Some((digits.parse().ok()?, title.to_owned()))
}

impl LineEnding {
    fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

impl Adr {
    fn render_unchecked(&self) -> String {
        let eol = self.line_ending.as_str();
        let header = self.header.to_string().replace('\n', eol);
        let mut text = format!("# {}. {}{eol}{eol}{header}{eol}", self.number, self.title);
        if !self.body.is_empty() {
            text.push_str(eol);
            text.push_str(&self.body);
        }
        text
    }

    /// Read an ADR from the text of its file.
    ///
    /// The first line must be the H1. The header block runs from there to the
    /// first `## ` line.
    ///
    /// # Errors
    ///
    /// Returns [`AdrError`] if the file is empty, the first line is not a valid
    /// title, or the header block cannot be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use decider_adr::adr::Adr;
    /// use decider_adr::status::Status;
    ///
    /// let text = "# 0001. Use Postgres\n\n**Status:** proposed\n\n## Context\n";
    /// let adr = Adr::parse(text)?;
    /// assert_eq!(adr.title, "Use Postgres");
    /// assert_eq!(adr.header.status, Status::Proposed);
    /// assert_eq!(adr.render()?, text);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(text: &str) -> Result<Self, AdrError> {
        let mut lines = text.split_inclusive('\n');
        let first_raw = lines.next().ok_or(AdrError::Empty)?;
        let first = first_raw.trim_end();
        let (number, title) =
            parse_title(first).ok_or_else(|| AdrError::InvalidTitle(first.to_owned()))?;
        let line_ending = if first_raw.ends_with("\r\n") {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        };

        let mut offset = first_raw.len();
        let mut header_lines = Vec::new();
        for (index, line) in lines.enumerate() {
            if line.starts_with("## ") {
                break;
            }
            header_lines.push((index + 2, line.trim_end()));
            offset += line.len();
        }

        let header = Header::parse(header_lines)?;
        Ok(Self {
            number,
            title,
            header,
            body: text.split_at(offset).1.to_owned(),
            line_ending,
        })
    }

    /// Write the ADR back out.
    ///
    /// The title and header are rewritten in a canonical form: spacing around
    /// the title separator, labels and values is normalised, blank lines are
    /// collapsed, numbers are padded to four digits, header lines are in a
    /// fixed order with unknown lines last, and line endings follow the first
    /// line. The body is written as it was read. Text already in canonical form
    /// comes back byte for byte, and other text is normalised.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::Unreadable`] if the result would not parse, and
    /// [`RenderError::ReadsBackDifferently`] if it would parse to a different
    /// ADR. Which one depends on what the written text looks like, not on the
    /// kind of mistake. A line break, whitespace at the start or end, a body that
    /// does not start with a `## ` heading, or an unknown header line that uses
    /// a known label can each give either.
    ///
    /// # Examples
    ///
    /// ```
    /// use decider_adr::adr::Adr;
    ///
    /// let mut adr = Adr::parse("# 0001. Use Postgres\n\n**Status:** proposed\n")?;
    /// adr.title = "Use\nPostgres".to_owned();
    /// assert!(adr.render().is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn render(&self) -> Result<String, RenderError> {
        let text = self.render_unchecked();
        match Self::parse(&text) {
            Ok(read) if read == *self => Ok(text),
            Ok(_) => Err(RenderError::ReadsBackDifferently),
            Err(error) => Err(RenderError::Unreadable(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::Stage;
    use crate::status::Status;

    const SAMPLE: &str = "\
# 0007. Use Postgres

**Status:** accepted
**Proposed:** 2026-07-29 by Ada
**Accepted:** 2026-07-30 by Ada

## Context

Some context.

## Decision

Use Postgres.
";

    #[test]
    fn parses_the_title_header_and_body() {
        let adr = Adr::parse(SAMPLE).unwrap();
        assert_eq!(adr.number, AdrNumber::new(7), "number");
        assert_eq!(adr.title, "Use Postgres", "title");
        assert_eq!(adr.header.status, Status::Accepted, "status");
        assert!(
            adr.body.starts_with("## Context\n"),
            "body starts at the first H2"
        );
    }

    #[test]
    fn writing_back_gives_the_same_text() {
        assert_eq!(
            Adr::parse(SAMPLE).unwrap().render().unwrap(),
            SAMPLE,
            "round trip"
        );
    }

    #[test]
    fn the_body_is_kept_verbatim() {
        let text = "# 0001. Odd body\n\n**Status:** proposed\n\n## Context\n\n\
                    ```\n**Status:** not a header\n# 0002. nor a title\n```\n\n\n  trailing  \n";
        let adr = Adr::parse(text).unwrap();
        assert!(
            adr.body.contains("**Status:** not a header"),
            "code fence kept"
        );
        assert_eq!(adr.render().unwrap(), text, "odd spacing survives");
    }

    #[test]
    fn a_document_with_no_sections_round_trips() {
        let text = "# 0001. Bare\n\n**Status:** proposed\n";
        let adr = Adr::parse(text).unwrap();
        assert_eq!(adr.body, "", "no body");
        assert_eq!(adr.render().unwrap(), text, "round trip");
    }

    #[test]
    fn unknown_header_lines_survive_a_round_trip() {
        let text = "# 0001. Keeps extras\n\n**Status:** proposed\n**Proposed:** 2026-07-29\n**Depends on:** 0003\n\n## Context\n";
        assert_eq!(
            Adr::parse(text).unwrap().render().unwrap(),
            text,
            "Depends on is kept"
        );
    }

    #[test]
    fn header_lines_are_written_in_the_fixed_order() {
        let text =
            "# 0001. Reordered\n\n**Proposed:** 2026-07-29\n**Status:** proposed\n\n## Context\n";
        assert_eq!(
            Adr::parse(text).unwrap().render().unwrap(),
            "# 0001. Reordered\n\n**Status:** proposed\n**Proposed:** 2026-07-29\n\n## Context\n",
            "Status is written first"
        );
    }

    #[test]
    fn the_h1_number_is_padded_when_written() {
        assert_eq!(
            Adr::parse("# 7. Short\n\n**Status:** proposed\n")
                .unwrap()
                .render()
                .unwrap(),
            "# 0007. Short\n\n**Status:** proposed\n",
            "padded to four digits"
        );
    }

    #[test]
    fn windows_line_endings_are_read() {
        let adr =
            Adr::parse("# 0001. Crlf\r\n\r\n**Status:** proposed\r\n\r\n## Context\r\n").unwrap();
        assert_eq!(adr.title, "Crlf", "title has no carriage return");
        assert_eq!(
            adr.header.status,
            Status::Proposed,
            "status has no carriage return"
        );
        assert_eq!(
            adr.line_ending,
            LineEnding::CrLf,
            "detected from the first line"
        );
    }

    #[test]
    fn windows_line_endings_are_kept_when_written() {
        let text = "# 0001. Crlf\r\n\r\n**Status:** proposed\r\n**Proposed:** 2026-07-29 by Ada\r\n\r\n## Context\r\n\r\nText.\r\n";
        assert_eq!(
            Adr::parse(text).unwrap().render().unwrap(),
            text,
            "no mixed endings"
        );
    }

    #[test]
    fn an_empty_file_is_an_error() {
        assert_eq!(Adr::parse(""), Err(AdrError::Empty), "empty");
    }

    #[test]
    fn a_bad_first_line_is_an_error() {
        for text in [
            "**Status:** proposed\n",
            "\n# 0001. Late title\n",
            "# NNNN. Title\n",
            "# 0001 Missing dot\n",
            "# 0001. \n",
            "## 0001. Wrong level\n",
        ] {
            assert!(
                matches!(Adr::parse(text), Err(AdrError::InvalidTitle(_))),
                "{text:?} should be an invalid title"
            );
        }
    }

    #[test]
    fn header_errors_carry_the_file_line_number() {
        let error = Adr::parse("# 0001. Bad\n\n**Status:** proposed\nstray prose\n\n## Context\n")
            .unwrap_err();
        assert_eq!(
            error,
            AdrError::Header(HeaderError::NotAHeaderLine {
                line: 4,
                text: "stray prose".to_owned()
            }),
            "line 4 of the file"
        );
    }

    #[test]
    fn a_missing_status_is_an_error() {
        assert_eq!(
            Adr::parse("# 0001. No status\n\n## Context\n"),
            Err(AdrError::Header(HeaderError::MissingStatus)),
            "no status line"
        );
    }

    #[test]
    fn a_title_with_a_line_break_cannot_be_written() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.title = "T\n\n## Injected".to_owned();
        assert!(
            matches!(adr.render(), Err(RenderError::Unreadable(_))),
            "would start a new section"
        );
    }

    #[test]
    fn a_header_value_with_a_line_break_cannot_be_written() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.header
            .other
            .push(("Owner".to_owned(), "Ada\n**Status:** proposed".to_owned()));
        assert!(
            matches!(
                adr.render(),
                Err(RenderError::Unreadable(AdrError::Header(
                    HeaderError::Duplicate { .. }
                )))
            ),
            "would inject a second status line"
        );
    }

    #[test]
    fn a_body_that_does_not_start_with_a_heading_cannot_be_written() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.body = "stray prose\n".to_owned();
        assert!(
            matches!(
                adr.render(),
                Err(RenderError::Unreadable(AdrError::Header(
                    HeaderError::NotAHeaderLine { .. }
                )))
            ),
            "stray prose would be read as a header line"
        );
    }

    #[test]
    fn a_title_with_padding_cannot_be_written() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.title = " Use Postgres".to_owned();
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "reading would trim it"
        );
    }

    #[test]
    fn a_body_that_starts_with_a_blank_line_reads_back_differently() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.body = "\n## Context\n".to_owned();
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "the blank line would be dropped"
        );
    }

    #[test]
    fn an_unknown_line_with_a_known_label_reads_back_differently() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.header.accepted = None;
        adr.header
            .other
            .push(("Accepted".to_owned(), "2026-07-30 by Ada".to_owned()));
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "it would be read into the accepted stage"
        );
    }

    #[test]
    fn a_label_that_contains_the_closing_markup_reads_back_differently() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.header
            .other
            .push(("Owner:** x".to_owned(), "y".to_owned()));
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "the label would be split at the first `:**`"
        );
    }

    #[test]
    fn a_title_line_break_can_give_either_variant() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.title = "Use Postgres\n".to_owned();
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "trailing break"
        );
        adr.title = "Use\nPostgres".to_owned();
        assert!(
            matches!(adr.render(), Err(RenderError::Unreadable(_))),
            "inner break"
        );
    }

    #[test]
    fn a_title_with_padding_can_give_either_variant() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.title = " Use Postgres".to_owned();
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "leading space"
        );
        adr.title = " ".to_owned();
        assert!(
            matches!(adr.render(), Err(RenderError::Unreadable(_))),
            "only a space"
        );
    }

    #[test]
    fn a_body_that_is_not_a_heading_can_give_either_variant() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.body = "\n## Context\n".to_owned();
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "blank line first"
        );
        adr.body = "stray prose\n".to_owned();
        assert!(
            matches!(adr.render(), Err(RenderError::Unreadable(_))),
            "prose first"
        );
    }

    #[test]
    fn a_known_label_in_other_can_give_either_variant() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.header.accepted = None;
        adr.header
            .other
            .push(("Accepted".to_owned(), "2026-07-30".to_owned()));
        assert_eq!(
            adr.render(),
            Err(RenderError::ReadsBackDifferently),
            "stage unset"
        );
        adr.header.accepted =
            Some(Stage::new("2026-07-30".parse().unwrap(), None, Vec::new()).unwrap());
        assert!(
            matches!(adr.render(), Err(RenderError::Unreadable(_))),
            "stage set"
        );
    }

    #[test]
    fn text_that_is_not_canonical_is_normalised_when_written() {
        let text = "# 0001.  Spaced\n**Status:**  proposed\n**Owner:** Ada  \n\n\n## Context\n";
        assert_eq!(
            Adr::parse(text).unwrap().render().unwrap(),
            "# 0001. Spaced\n\n**Status:** proposed\n**Owner:** Ada\n\n## Context\n",
            "one blank line after the title and before the body, values trimmed"
        );
    }

    #[test]
    fn a_reference_on_the_wrong_line_names_the_line_and_cause() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.header.proposed =
            Some(Stage::new("2026-07-29".parse().unwrap(), None, vec!["#1".to_owned()]).unwrap());
        let error = adr.render().unwrap_err();
        assert!(
            matches!(
                &error,
                RenderError::Unreadable(AdrError::Header(HeaderError::InvalidValue { label, .. }))
                    if label == "Proposed"
            ),
            "{error}"
        );
    }

    #[test]
    fn an_unreadable_adr_exposes_the_cause_as_its_source() {
        let mut adr = Adr::parse(SAMPLE).unwrap();
        adr.header
            .other
            .push(("Owner".to_owned(), "Ada\n**Status:** proposed".to_owned()));
        let error = adr.render().unwrap_err();
        let source = std::error::Error::source(&error).unwrap().to_string();
        assert!(
            source.contains("`**Status:**` appears more than once"),
            "{source}"
        );
        assert!(
            !error.to_string().contains("appears more than once"),
            "the cause is not repeated in the message: {error}"
        );
    }

    #[test]
    fn mixed_line_endings_are_written_with_the_first_lines_ending() {
        let adr = Adr::parse("# 0001. T\r\n\r\n**Status:** proposed\n\n## Context\n").unwrap();
        assert_eq!(
            adr.render().unwrap(),
            "# 0001. T\r\n\r\n**Status:** proposed\r\n\r\n## Context\n",
            "title and header follow the first line, the body is untouched"
        );
    }

    #[test]
    fn a_last_header_line_with_no_terminator_gains_one() {
        let adr = Adr::parse("# 0001. T\n\n**Status:** proposed").unwrap();
        assert_eq!(
            adr.render().unwrap(),
            "# 0001. T\n\n**Status:** proposed\n",
            "one trailing newline is added"
        );
    }
}
