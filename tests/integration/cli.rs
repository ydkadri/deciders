//! Integration tests that run the compiled binary.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_decider"))
}

#[test]
fn runs_and_exits_successfully_with_no_arguments() {
    let out = bin().env_remove("RUST_LOG").output().unwrap();
    assert!(out.status.success(), "exit status was {}", out.status);
    assert!(
        out.stdout.is_empty(),
        "no commands exist yet, so nothing is printed"
    );
}

#[test]
fn stays_quiet_at_the_default_log_level() {
    let out = bin().env_remove("RUST_LOG").output().unwrap();
    assert!(
        out.stderr.is_empty(),
        "stderr was: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
