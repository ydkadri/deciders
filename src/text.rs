//! Editing the text of an ADR (ADR 0005).
//!
//! A command changes only the lines it owns. Nothing here parses a whole
//! document: the functions find the header, which is the run of lines between
//! the title and the first `## ` heading, and change one thing in it.

use crate::lifecycle::Status;

/// The label of the status line.
const STATUS_LABEL: &str = "**Status:**";

/// The byte offset where the header ends: the start of the first `## ` line, or
/// the end of the text when there is none.
fn header_end(text: &str) -> usize {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if line.starts_with("## ") {
            return offset;
        }
        offset += line.len();
    }
    text.len()
}

/// The byte range of the value on the first `**Status:**` line in the header,
/// without the whitespace around it, and the value.
fn status_line(text: &str) -> Option<(std::ops::Range<usize>, &str)> {
    let end = header_end(text);
    let mut offset = 0;
    for line in text[..end].split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if let Some(after_label) = bare.strip_prefix(STATUS_LABEL) {
            let value = after_label.trim();
            let leading = after_label.len() - after_label.trim_start().len();
            let start = offset + STATUS_LABEL.len() + leading;
            return Some((start..start + value.len(), value));
        }
        offset += line.len();
    }
    None
}

/// The value on the first `**Status:**` line in the header of `text`, or `None`
/// if the header has no such line.
pub(crate) fn read_status(text: &str) -> Option<&str> {
    status_line(text).map(|(_, value)| value)
}

/// `text` with the value of its `**Status:**` line replaced by `status`, or
/// `None` if the header has no such line.
pub(crate) fn set_status(text: &str, status: Status) -> Option<String> {
    let (range, _) = status_line(text)?;
    Some(format!(
        "{}{status}{}",
        &text[..range.start],
        &text[range.end..]
    ))
}

/// The line ending of `text`: `\r\n` if it has any, otherwise `\n`. Taken from
/// the whole text, because the last line may have no ending to copy.
pub(crate) fn line_ending(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// `text` with `line` added after the last non-blank line of the header, which
/// is where the other stage lines are (ADR 0005 keeps the header to bold-line
/// fields). The new line takes the ending of the file, and no other byte changes.
pub(crate) fn add_header_line(text: &str, line: &str) -> String {
    let ending = line_ending(text);
    let mut offset = 0;
    let mut insert_at = 0;
    for header_line in text[..header_end(text)].split_inclusive('\n') {
        offset += header_line.len();
        if !header_line.trim().is_empty() {
            insert_at = offset;
        }
    }
    let (before, after) = text.split_at(insert_at);
    let mut edited = String::with_capacity(text.len() + line.len() + 2);
    edited.push_str(before);
    if !before.ends_with('\n') {
        edited.push_str(ending);
    }
    edited.push_str(line);
    edited.push_str(ending);
    edited.push_str(after);
    edited
}

/// `text` with a section added at the end: a blank line, the `## heading`, a
/// blank line, the body and a final line ending. Nothing already in `text` is
/// changed, and every new line, including each line of the body, ends in `\r\n`
/// if the file has any, otherwise `\n`.
pub(crate) fn append_section(text: &str, heading: &str, body: &str) -> String {
    let ending = line_ending(text);
    let close = if text.ends_with('\n') { "" } else { ending };
    let body = body.trim().lines().collect::<Vec<_>>().join(ending);
    format!("{text}{close}{ending}## {heading}{ending}{ending}{body}{ending}")
}

/// The line that records a change of state: `**Accepted:** 2026-10-05 by Ada`,
/// with the references in brackets at the end if there are any.
pub(crate) fn stage_line(
    label: &str,
    date: &str,
    name: Option<&str>,
    references: &[String],
) -> String {
    let by = name.map(|name| format!(" by {name}")).unwrap_or_default();
    let refs = if references.is_empty() {
        String::new()
    } else {
        format!(" ({})", references.join(", "))
    };
    format!("**{label}:** {date}{by}{refs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADR: &str = "# 0007. Use Postgres\n\n**Status:** proposed\n**Proposed:** 2026-10-01 by Ada\n\n## Context\n\nWhy.\n";

    #[test]
    fn the_status_is_read_from_the_header() {
        assert_eq!(read_status(ADR), Some("proposed"), "status");
    }

    #[test]
    fn a_status_line_with_extra_spaces_is_read() {
        assert_eq!(
            read_status("# 1. T\n\n**Status:**   accepted  \n"),
            Some("accepted"),
            "trimmed"
        );
    }

    #[test]
    fn a_missing_status_is_reported() {
        assert_eq!(read_status("# 1. T\n\n## Context\n"), None, "none");
    }

    #[test]
    fn a_status_only_in_a_section_does_not_count() {
        let text = "# 1. T\n\n## Context\n\n**Status:** accepted\n";
        assert_eq!(read_status(text), None, "outside the header");
    }

    #[test]
    fn setting_the_status_of_a_header_without_one_gives_nothing() {
        assert_eq!(set_status("# 1. T\n\n## Context\n", Status::Accepted), None);
    }

    #[test]
    fn setting_the_status_changes_only_that_line() {
        assert_eq!(
            set_status(ADR, Status::Accepted).unwrap(),
            "# 0007. Use Postgres\n\n**Status:** accepted\n**Proposed:** 2026-10-01 by Ada\n\n## Context\n\nWhy.\n",
            "everything else is as it was"
        );
    }

    #[test]
    fn setting_the_status_keeps_the_whitespace_around_the_value() {
        assert_eq!(
            set_status(
                "# 1. T\n\n**Status:** proposed  \n**Owner:** Ada\n",
                Status::Accepted
            )
            .unwrap(),
            "# 1. T\n\n**Status:** accepted  \n**Owner:** Ada\n",
            "a trailing hard break stays"
        );
        assert_eq!(
            set_status("# 1. T\n\n**Status:**\tproposed \t\n", Status::Accepted).unwrap(),
            "# 1. T\n\n**Status:**\taccepted \t\n",
            "a tab after the label stays"
        );
    }

    #[test]
    fn the_first_status_line_is_the_one_changed() {
        let text = "# 1. T\n\n**Status:** proposed\n**Status:** proposed\n";
        assert_eq!(
            set_status(text, Status::Accepted).unwrap(),
            "# 1. T\n\n**Status:** accepted\n**Status:** proposed\n",
            "only the first"
        );
    }

    #[test]
    fn setting_the_status_keeps_windows_line_endings_on_other_lines() {
        let text = "# 1. T\r\n\r\n**Status:** proposed\r\n**Proposed:** 2026-10-01\r\n";
        assert_eq!(
            set_status(text, Status::Accepted).unwrap(),
            "# 1. T\r\n\r\n**Status:** accepted\r\n**Proposed:** 2026-10-01\r\n",
            "the carriage return stays"
        );
    }

    #[test]
    fn a_line_is_added_after_the_last_header_line_and_the_blank_line_stays() {
        assert_eq!(
            add_header_line(ADR, "**Accepted:** 2026-10-05 by Ada"),
            "# 0007. Use Postgres\n\n**Status:** proposed\n**Proposed:** 2026-10-01 by Ada\n**Accepted:** 2026-10-05 by Ada\n\n## Context\n\nWhy.\n",
            "placed with the others"
        );
    }

    #[test]
    fn a_line_is_added_without_touching_the_line_before_it() {
        assert_eq!(
            add_header_line(
                "# 1. T\n\n**Status:** proposed\n**Owner:** Ada  \n\n## Context\n",
                "**Accepted:** 2026-10-05"
            ),
            "# 1. T\n\n**Status:** proposed\n**Owner:** Ada  \n**Accepted:** 2026-10-05\n\n## Context\n",
            "trailing spaces stay where they were"
        );
    }

    #[test]
    fn a_line_added_to_a_windows_file_takes_its_line_ending_and_changes_no_other() {
        assert_eq!(
            add_header_line(
                "# 1. T\r\n\r\n**Status:** proposed\r\n**Proposed:** 2026-10-01\r\n\r\n## Context\r\n",
                "**Accepted:** 2026-10-05"
            ),
            "# 1. T\r\n\r\n**Status:** proposed\r\n**Proposed:** 2026-10-01\r\n**Accepted:** 2026-10-05\r\n\r\n## Context\r\n",
            "only the new line is added"
        );
    }

    #[test]
    fn a_line_added_to_a_windows_file_with_no_final_line_ending_is_a_windows_line() {
        assert_eq!(
            add_header_line(
                "# 1. T\r\n\r\n**Status:** proposed",
                "**Accepted:** 2026-10-05"
            ),
            "# 1. T\r\n\r\n**Status:** proposed\r\n**Accepted:** 2026-10-05\r\n",
            "the file's ending, not the missing one"
        );
    }

    #[test]
    fn a_line_is_added_to_a_file_with_no_sections() {
        assert_eq!(
            add_header_line(
                "# 1. T\n\n**Status:** proposed\n",
                "**Accepted:** 2026-10-05"
            ),
            "# 1. T\n\n**Status:** proposed\n**Accepted:** 2026-10-05\n",
            "at the end"
        );
    }

    #[test]
    fn a_line_is_added_when_the_last_line_has_no_line_ending() {
        assert_eq!(
            add_header_line("# 1. T\n\n**Status:** proposed", "**Accepted:** 2026-10-05"),
            "# 1. T\n\n**Status:** proposed\n**Accepted:** 2026-10-05\n",
            "a line ending is supplied"
        );
    }

    #[test]
    fn a_section_is_appended_after_a_blank_line() {
        assert_eq!(
            append_section(
                "# 1. T\n\n**Status:** rejected\n\n## Context\n\nWhy.\n",
                "Rejection",
                " Too costly. \n"
            ),
            "# 1. T\n\n**Status:** rejected\n\n## Context\n\nWhy.\n\n## Rejection\n\nToo costly.\n",
            "trimmed body, one blank line before"
        );
    }

    #[test]
    fn a_section_is_appended_without_changing_what_the_file_already_ends_with() {
        assert_eq!(
            append_section("# 1. T\n\nWhy.\n\n\n", "Outcome", "Done."),
            "# 1. T\n\nWhy.\n\n\n\n## Outcome\n\nDone.\n",
            "trailing blank lines stay"
        );
        assert_eq!(
            append_section("# 1. T\n\nWhy.", "Outcome", "Done."),
            "# 1. T\n\nWhy.\n\n## Outcome\n\nDone.\n",
            "a missing final line ending is supplied"
        );
    }

    #[test]
    fn a_section_appended_to_a_windows_file_with_no_final_line_ending_is_a_windows_section() {
        assert_eq!(
            append_section("# 1. T\r\n\r\nWhy.", "Rejection", "No."),
            "# 1. T\r\n\r\nWhy.\r\n\r\n## Rejection\r\n\r\nNo.\r\n",
            "the file's ending, not the missing one"
        );
    }

    #[test]
    fn every_line_of_a_section_body_takes_the_files_line_ending() {
        assert_eq!(
            append_section("# 1. T\r\n\r\nWhy.\r\n", "Outcome", "one\n\ntwo\r\nthree"),
            "# 1. T\r\n\r\nWhy.\r\n\r\n## Outcome\r\n\r\none\r\n\r\ntwo\r\nthree\r\n",
            "windows file, mixed body"
        );
        assert_eq!(
            append_section("# 1. T\n\nWhy.\n", "Outcome", "one\r\ntwo"),
            "# 1. T\n\nWhy.\n\n## Outcome\n\none\ntwo\n",
            "unix file, windows body"
        );
    }

    #[test]
    fn a_section_appended_to_a_windows_file_uses_its_line_ending_and_changes_no_other() {
        assert_eq!(
            append_section("# 1. T\r\n\r\nWhy.\r\n", "Rejection", "No."),
            "# 1. T\r\n\r\nWhy.\r\n\r\n## Rejection\r\n\r\nNo.\r\n",
            "only the new lines are added"
        );
    }

    #[test]
    fn a_stage_line_has_the_date_and_optionally_a_name_and_references() {
        assert_eq!(
            stage_line("Accepted", "2026-10-05", Some("Ada Lovelace"), &[]),
            "**Accepted:** 2026-10-05 by Ada Lovelace",
            "with name"
        );
        assert_eq!(
            stage_line("Accepted", "2026-10-05", None, &[]),
            "**Accepted:** 2026-10-05",
            "no name"
        );
        assert_eq!(
            stage_line(
                "Implemented",
                "2026-10-05",
                Some("Ada"),
                &["#12".to_owned(), "#13".to_owned()]
            ),
            "**Implemented:** 2026-10-05 by Ada (#12, #13)",
            "with references"
        );
        assert_eq!(
            stage_line("Implemented", "2026-10-05", None, &["#12".to_owned()]),
            "**Implemented:** 2026-10-05 (#12)",
            "references and no name"
        );
    }
}
