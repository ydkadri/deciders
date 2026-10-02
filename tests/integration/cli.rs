//! Integration tests that run the compiled binary.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_decider"))
}

#[test]
fn prints_greeting_for_argument() {
    let out = bin().arg("Ada").output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, Ada!\n");
}

#[test]
fn defaults_to_world() {
    let out = bin().output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, world!\n");
}

#[test]
fn blank_argument_fails() {
    let out = bin().arg(" ").output().unwrap();
    assert!(!out.status.success());
}
