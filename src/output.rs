//! The only place in the binary allowed to write to stdout.

/// Print a line of user-facing output to stdout.
#[expect(
    clippy::print_stdout,
    reason = "this module is the dedicated CLI output boundary"
)]
pub(crate) fn line(text: &str) {
    println!("{text}");
}
