//! Integration tests that run the compiled binary.

use std::io;
use std::process::{Command, Output};

fn decider(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_decider"))
        .args(arguments)
        .output()
}

#[test]
fn prints_its_version() -> io::Result<()> {
    let out = decider(&["--version"])?;
    assert!(out.status.success(), "exit status was {}", out.status);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("decider {}\n", env!("CARGO_PKG_VERSION")),
        "version line"
    );
    Ok(())
}

#[test]
fn with_no_arguments_it_shows_the_help_and_fails() -> io::Result<()> {
    let out = decider(&[])?;
    assert!(!out.status.success(), "a command is required");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Usage: decider"),
        "help was: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}
