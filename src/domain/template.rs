//! The ADR template that a new store starts with and `propose` copies the sections from.

/// The template built into the binary, which is this repository's own template.
pub const TEXT: &str = include_str!("../../docs/explanations/decisions/0000-template.md");

/// The template to use: the store's own if it has one, otherwise the built-in one.
pub fn choose(stored: Option<&str>) -> &str {
    stored.unwrap_or(TEXT)
}

/// The part of a template from its first `## ` heading onwards.
///
/// Returns `None` if the template has no such heading.
pub fn sections(template: &str) -> Option<&str> {
    let mut offset = 0;
    for line in template.split_inclusive('\n') {
        if line.starts_with("## ") {
            return Some(template.split_at(offset).1);
        }
        offset += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stores_own_template_wins() {
        assert_eq!(choose(Some("mine")), "mine", "stored");
    }

    #[test]
    fn the_built_in_template_is_the_fallback() {
        assert_eq!(choose(None), TEXT, "fallback");
    }

    #[test]
    fn the_built_in_template_has_the_four_required_sections() {
        let body = sections(TEXT).unwrap();
        for heading in [
            "## Context",
            "## Decision",
            "## Options considered",
            "## Consequences",
        ] {
            assert!(body.contains(heading), "missing {heading}");
        }
    }

    #[test]
    fn sections_start_at_the_first_second_level_heading() {
        assert_eq!(
            sections("# T\n\n**Status:** proposed\n\n## A\n\ntext\n## B\n"),
            Some("## A\n\ntext\n## B\n"),
            "everything before is dropped"
        );
    }

    #[test]
    fn a_template_without_sections_has_none() {
        assert_eq!(
            sections("# T\n\n**Status:** proposed\n### Deep\n"),
            None,
            "no H2"
        );
        assert_eq!(sections(""), None, "empty");
    }
}
