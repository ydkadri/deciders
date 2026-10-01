//! Integration tests for the public library API: reading and writing an ADR
//! and walking it through the lifecycle.

use decider_adr::adr::Adr;
use decider_adr::header::{Stage, StageError};
use decider_adr::number::AdrNumber;
use decider_adr::status::Status;

const ADR: &str = "\
# 0004. Serialise with the C dumper

**Status:** accepted
**Proposed:** 2026-07-29 by Ada Lovelace
**Accepted:** 2026-07-30 by Ada Lovelace
**Depends on:** 0001

## Context

Profiling showed most time in the pure-Python emitter.

## Decision

Use the C bindings.

## Options considered

- Leave as it is.

## Consequences

Faster runs.
";

#[test]
fn an_adr_survives_a_read_and_write() {
    let adr = Adr::parse(ADR).unwrap();
    assert_eq!(adr.render().unwrap(), ADR, "text changed");
}

#[test]
fn the_public_model_exposes_what_check_needs() {
    let adr = Adr::parse(ADR).unwrap();
    assert_eq!(adr.number, AdrNumber::new(4), "number");
    assert!(
        !adr.header.status.is_complete(),
        "accepted is still incomplete"
    );
    assert_eq!(
        adr.header.accepted.as_ref().and_then(Stage::by),
        Some("Ada Lovelace"),
        "who accepted it"
    );
}

#[test]
fn moving_to_implemented_and_writing_back_keeps_the_prose() {
    let mut adr = Adr::parse(ADR).unwrap();
    adr.header.status = adr.header.status.transition(Status::Implemented).unwrap();
    adr.header.implemented =
        Some(Stage::new("2026-08-02".parse().unwrap(), None, vec!["#12".to_owned()]).unwrap());

    let written = adr.render().unwrap();
    let reread = Adr::parse(&written).unwrap();
    assert_eq!(reread.header.status, Status::Implemented, "status");
    assert_eq!(reread.body, adr.body, "sections are untouched");
    assert!(
        written.contains("**Implemented:** 2026-08-02 (#12)\n**Depends on:** 0001\n"),
        "unknown lines stay after the stage lines:\n{written}"
    );
}

#[test]
fn the_lifecycle_rejects_a_move_the_table_does_not_allow() {
    let adr = Adr::parse(ADR).unwrap();
    let error = adr.header.status.transition(Status::Rejected).unwrap_err();
    assert_eq!(
        error.to_string(),
        "cannot move an ADR from accepted to rejected: from accepted it can move to implemented or superseded",
        "message"
    );
}

#[test]
fn a_malformed_header_reports_the_file_line() {
    let error = Adr::parse("# 0001. Bad\n\n**Status:** proposed\nstray\n").unwrap_err();
    assert_eq!(
        error.to_string(),
        "line 4: expected a header line such as `**Status:** proposed`, found \"stray\"",
        "message"
    );
}

#[test]
fn an_author_name_that_would_not_read_back_is_refused_up_front() {
    let error = Stage::new(
        "2026-08-02".parse().unwrap(),
        Some("Jane Doe (Contractor)".to_owned()),
        Vec::new(),
    )
    .unwrap_err();
    assert!(
        matches!(error, StageError::InvalidAuthor(_)),
        "a parenthesis would be misread as a reference list: {error}"
    );
}
