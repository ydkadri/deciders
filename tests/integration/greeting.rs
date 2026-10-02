//! Integration tests for the public library API.

#[test]
fn greets_through_the_public_api() {
    assert_eq!(decider_adr::greet("Grace").unwrap(), "Hello, Grace!");
}

#[test]
fn blank_name_is_an_error() {
    let err = decider_adr::greet("").unwrap_err();
    assert_eq!(err.to_string(), "name must not be empty");
}
