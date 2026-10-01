//! The only place in the binary allowed to write to stdout or ask a question.

use std::io::{self, BufRead, IsTerminal, Write};

/// Print a line of user-facing output to stdout.
#[expect(
    clippy::print_stdout,
    reason = "this module is the dedicated CLI output boundary"
)]
pub(crate) fn line(text: &str) {
    println!("{text}");
}

/// Print a line of warning to stderr.
#[expect(
    clippy::print_stderr,
    reason = "this module is the dedicated CLI output boundary"
)]
pub(crate) fn warning(text: &str) {
    eprintln!("{text}");
}

/// Whether a person can be asked a question: both standard input and output are terminals.
pub(crate) fn can_ask() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Ask `question` on stdout and read one line from stdin.
///
/// Returns the line without its line ending, or `None` at the end of input
/// (for example Ctrl-D), which is not the same as an empty answer.
pub(crate) fn ask(question: &str) -> io::Result<Option<String>> {
    let mut stdout = io::stdout().lock();
    write!(stdout, "{question} ")?;
    stdout.flush()?;
    let mut answer = String::new();
    if io::stdin().lock().read_line(&mut answer)? == 0 {
        return Ok(None);
    }
    Ok(Some(answer.trim_end_matches(['\n', '\r']).to_owned()))
}
