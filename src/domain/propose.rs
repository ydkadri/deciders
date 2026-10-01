//! Proposing an ADR: building the record from a title, an author and a template (ADR 0001).

use crate::domain::adr::{Adr, LineEnding};
use crate::domain::date::Date;
use crate::domain::header::{Header, Stage, StageError};
use crate::domain::number::AdrNumber;
use crate::domain::person::DisplayName;
use crate::domain::status::Status;
use crate::domain::template;

/// A proposed ADR that cannot be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildError {
    /// The title is empty, or has a line break.
    #[error("the title must be one line of text, such as \"Use Postgres\"")]
    InvalidTitle,
    /// The template has no `## ` sections to copy.
    #[error("the template has no `## ` sections")]
    TemplateWithoutSections,
    /// The date cannot be written on the Proposed line.
    #[error("cannot write the Proposed line: {0}")]
    Stage(#[from] StageError),
}

fn check_title(title: &str) -> Result<&str, BuildError> {
    let title = title.trim();
    if title.is_empty() || title.contains(['\n', '\r']) {
        Err(BuildError::InvalidTitle)
    } else {
        Ok(title)
    }
}

/// Build a proposed ADR from `template_text`, dated `date`.
///
/// The sections come from the template. The header has the status `proposed`
/// and a Proposed line with the date and, if given, the author.
///
/// # Errors
///
/// Returns [`BuildError::InvalidTitle`] if the title is empty or has a line
/// break, and [`BuildError::TemplateWithoutSections`] if the template has no
/// `## ` heading.
pub fn build(
    number: AdrNumber,
    title: &str,
    author: Option<&DisplayName>,
    date: Date,
    template_text: &str,
) -> Result<Adr, BuildError> {
    let title = check_title(title)?;
    let body = template::sections(template_text).ok_or(BuildError::TemplateWithoutSections)?;
    let mut header = Header::new(Status::Proposed);
    header.proposed = Some(Stage::new(
        date,
        author.map(|name| name.as_str().to_owned()),
        Vec::new(),
    )?);
    Ok(Adr {
        number,
        title: title.to_owned(),
        header,
        body: body.to_owned(),
        line_ending: LineEnding::of_first_line(template_text),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str =
        "# NNNN. Title\n\n**Status:** proposed\n\n## Context\n\nWhy.\n\n## Decision\n\nWhat.\n";

    fn date() -> Date {
        "2026-10-02".parse().unwrap()
    }

    fn ada() -> DisplayName {
        DisplayName::new("Ada").unwrap()
    }

    #[test]
    fn a_proposed_adr_has_the_date_and_author() {
        let adr = build(
            AdrNumber::new(7),
            "Use Postgres",
            Some(&ada()),
            date(),
            TEMPLATE,
        )
        .unwrap();
        assert_eq!(adr.header.status, Status::Proposed, "status");
        assert_eq!(
            adr.render().unwrap(),
            "# 0007. Use Postgres\n\n**Status:** proposed\n**Proposed:** 2026-10-02 by Ada\n\n## Context\n\nWhy.\n\n## Decision\n\nWhat.\n",
            "exact text"
        );
    }

    #[test]
    fn a_proposed_adr_without_an_author_has_only_the_date() {
        let adr = build(AdrNumber::new(1), "Title", None, date(), TEMPLATE).unwrap();
        assert!(
            adr.render().unwrap().contains("**Proposed:** 2026-10-02\n"),
            "no `by`"
        );
    }

    #[test]
    fn a_template_with_windows_line_endings_gives_an_adr_with_consistent_endings() {
        let crlf = TEMPLATE.replace('\n', "\r\n");
        let adr = build(AdrNumber::new(1), "Crlf", None, date(), &crlf).unwrap();
        let text = adr.render().unwrap();
        assert_eq!(adr.line_ending, LineEnding::CrLf, "taken from the template");
        assert!(
            !text.replace("\r\n", "").contains('\n'),
            "no bare line feed: {text:?}"
        );
    }

    #[test]
    fn the_title_is_trimmed() {
        let adr = build(AdrNumber::new(1), "  Spaced  ", None, date(), TEMPLATE).unwrap();
        assert_eq!(adr.title, "Spaced", "trimmed");
    }

    #[test]
    fn a_proposed_adr_reads_back_as_itself() {
        let name = DisplayName::new("Lovelace, Ada").unwrap();
        let adr = build(
            AdrNumber::new(5),
            "Round trip",
            Some(&name),
            date(),
            TEMPLATE,
        )
        .unwrap();
        assert_eq!(
            Adr::parse(&adr.render().unwrap()).unwrap(),
            adr,
            "round trip"
        );
    }

    #[test]
    fn an_empty_title_or_one_with_a_line_break_is_refused() {
        for title in ["", "   ", "a\nb", "a\rb"] {
            assert_eq!(
                build(AdrNumber::new(1), title, None, date(), TEMPLATE),
                Err(BuildError::InvalidTitle),
                "{title:?}"
            );
        }
    }

    #[test]
    fn a_template_without_sections_is_refused() {
        assert_eq!(
            build(
                AdrNumber::new(1),
                "T",
                None,
                date(),
                "# NNNN. Title\n\n**Status:** proposed\n"
            ),
            Err(BuildError::TemplateWithoutSections),
            "nothing to copy"
        );
    }

    #[test]
    fn the_built_in_template_builds_an_adr_that_passes_its_own_rules() {
        let adr = build(AdrNumber::new(1), "Built in", None, date(), template::TEXT).unwrap();
        assert!(Adr::parse(&adr.render().unwrap()).is_ok(), "reads back");
    }
}
