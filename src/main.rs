//! The `decider` command line, and the composition root (ADR 0003).

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

use anyhow::Context;
use clap::{Parser, Subcommand};
use decider_adr::files::{self, FilesStore};
use decider_adr::store::WritesAdrs;
use decider_adr::user;

#[derive(Debug, Subcommand)]
enum Command {
    /// Set up the repository and your settings. Safe to run again.
    Init {
        /// The ADR directory. A relative path is relative to the current
        /// directory. Asked for at a terminal if not given.
        #[arg(long, value_name = "PATH")]
        dir: Option<String>,
        /// Your name for the stage lines. Asked for at a terminal if not given.
        #[arg(long, value_name = "NAME")]
        user: Option<String>,
        /// Replace the settings that already exist. On its own it asks for both
        /// the directory and your name again; with --dir or --user it replaces
        /// only what was given.
        #[arg(long)]
        force: bool,
    },
}

/// Manage architecture decision records through their lifecycle.
#[derive(Debug, Parser)]
#[command(name = "decider", version, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Print a line to stdout. The only place in the binary that does.
#[expect(
    clippy::print_stdout,
    reason = "this function is the CLI's output boundary"
)]
fn say(text: &str) {
    println!("{text}");
}

/// Print a line to stderr. The only other place that prints.
#[expect(
    clippy::print_stderr,
    reason = "this function is the CLI's output boundary"
)]
fn warn(text: &str) {
    eprintln!("{text}");
}

/// Whether a person can be asked a question: input and output are both terminals.
fn can_ask() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Ask `question` on the terminal and return the line typed, without its line
/// ending. The caller has checked [`can_ask`].
fn ask(question: &str) -> anyhow::Result<String> {
    let mut stdout = io::stdout().lock();
    write!(stdout, "{question}").context("could not ask a question")?;
    stdout.flush().context("could not ask a question")?;
    let mut answer = String::new();
    io::stdin()
        .lock()
        .read_line(&mut answer)
        .context("could not read your answer")?;
    Ok(answer)
}

/// Ask for the ADR directory. An empty answer takes `default`, which is the
/// directory already set, if there is one.
fn ask_for_dir(default: &str) -> anyhow::Result<String> {
    let answer = ask(&format!("ADR directory [{default}]: "))?;
    Ok(files::dir_from_answer(&answer, default).to_owned())
}

/// Ask for the user's name. An empty answer takes the name already set, if
/// there is one, otherwise a suggestion worked out from the login.
fn ask_for_name(current: Option<&str>) -> anyhow::Result<Option<String>> {
    let login = std::env::var("USER").ok();
    let suggestion = user::suggest_name(current, login.as_deref());
    let question = match &suggestion {
        Some(name) => format!("Your name for the stage lines [{name}]: "),
        None => "Your name for the stage lines: ".to_owned(),
    };
    let answer = ask(&question)?;
    Ok(user::name_from_answer(&answer, suggestion.as_deref())?)
}

fn run_init(cwd: &Path, dir: Option<&str>, name: Option<&str>, force: bool) -> anyhow::Result<()> {
    // Settle every input first, so that a bad option leaves nothing half-written.
    let settings = user::settings_path(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    );
    let existing_dir = files::settings_dir(cwd)?;
    let existing_name = match &settings {
        Some(path) => user::read_name(path)?,
        None => None,
    };
    let reset = user::Reset::new(force, dir.is_some(), name.is_some());

    // An existing setting is kept unless it is being reset.
    let settle_dir = existing_dir.is_none() || reset.dir();
    let settle_name = existing_name.is_none() || reset.name();

    let chosen_dir = match (&existing_dir, settle_dir) {
        (Some(existing), false) => existing.clone(),
        _ => match dir {
            Some(given) => given.to_owned(),
            None if can_ask() => {
                ask_for_dir(existing_dir.as_deref().unwrap_or(files::DEFAULT_DIR))?
            }
            None if reset.dir() => {
                return Err(anyhow::anyhow!(
                    "--force needs a terminal to ask for the ADR directory; pass --dir"
                ));
            }
            None => files::DEFAULT_DIR.to_owned(),
        },
    };
    // A bad directory is reported before anything else is asked.
    files::check_dir(&chosen_dir)?;
    let chosen_name = if settle_name {
        match name {
            Some(text) => user::clean_name(text).context("the --user name cannot be used")?,
            None if can_ask() => ask_for_name(existing_name.as_deref())?,
            None if reset.name() => {
                return Err(anyhow::anyhow!(
                    "--force needs a terminal to ask for your name; pass --user"
                ));
            }
            None => None,
        }
    } else {
        None
    };
    if chosen_name.is_some() && settings.is_none() {
        return Err(decider_adr::error::Error::NoSettingsPath.into());
    }

    let mut store = FilesStore::for_init(cwd.to_path_buf(), Some(&chosen_dir), reset.dir())?;
    let prepared = store.prepare()?;
    for path in &prepared.created {
        say(&format!("created {}", path.display()));
    }
    for path in &prepared.replaced {
        say(&format!("replaced {}", path.display()));
    }
    for path in &prepared.kept {
        say(&format!("kept {} (already exists)", path.display()));
    }
    match (&chosen_name, &settings) {
        (Some(name), Some(path)) => {
            user::write_name(path, name)?;
            let verb = if existing_name.is_some() {
                "replaced your name with"
            } else {
                "recorded your name as"
            };
            say(&format!("{verb} {name}"));
        }
        _ => match existing_name {
            Some(existing) => say(&format!("kept your name as {existing}")),
            None => warn(
                "no name recorded, so stage lines will have no author; run `decider init --user NAME` to set one",
            ),
        },
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("could not read the current directory")?;
    match cli.command {
        Command::Init { dir, user, force } => {
            run_init(&cwd, dir.as_deref(), user.as_deref(), force)
        }
    }
}
