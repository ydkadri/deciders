//! The command line and the composition root (ADR 0004).
//!
//! This is the only place that knows the domain, the use cases and the adapters
//! together. It reads the clock once and passes the date on as a plain value.
//! The login name and whether a terminal is attached are read only inside
//! `TerminalAsker`, the terminal implementation of the `AsksForName` port.

mod cli;
mod clock;
mod output;

use std::ffi::OsString;
use std::path::Path;

use anyhow::{Context, bail};
use clap::Parser;
use decider_adr::adapters::files::settings;
use decider_adr::adapters::files::{self, FilesStore};
use decider_adr::adapters::user_file::{self, UserFile};
use decider_adr::app::init::{self, NameOutcome};
use decider_adr::app::propose;
use decider_adr::domain::person::DisplayName;
use decider_adr::ports::{AsksForName, Location, StoreError};
use tracing_subscriber::{
    EnvFilter, Layer, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::cli::{Cli, Command};

/// Install the global tracing subscriber, honouring `RUST_LOG`.
fn init_tracing() -> anyhow::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(filter);
    tracing_subscriber::registry()
        .with(fmt_layer)
        .try_init()
        .context("failed to initialise tracing")
}

/// Where the user's settings are kept, from the environment.
fn user_settings() -> UserFile {
    let path = user_file::settings_path(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    );
    UserFile::new(path)
}

/// The login name, used only to suggest a display name.
fn login() -> Option<OsString> {
    std::env::var_os("USER").or_else(|| std::env::var_os("USERNAME"))
}

/// Asks the person at the terminal for their name, suggesting one from the login.
struct TerminalAsker;

impl AsksForName for TerminalAsker {
    fn ask(&mut self) -> Result<Option<DisplayName>, StoreError> {
        if !output::can_ask() {
            return Ok(None);
        }
        let suggestion = login()
            .and_then(|login| login.into_string().ok())
            .and_then(|login| DisplayName::infer(&login));
        loop {
            let question = match &suggestion {
                Some(name) => format!("Your name for the stage lines [{name}]:"),
                None => "Your name for the stage lines:".to_owned(),
            };
            let answer = output::ask(&question)
                .map_err(|cause| StoreError(format!("could not read your answer: {cause}")))?;
            let Some(answer) = answer else {
                // End of input (Ctrl-D) is not an answer, so nothing is accepted.
                return Ok(None);
            };
            match DisplayName::from_answer(&answer, suggestion.as_ref()) {
                Ok(name) => return Ok(name),
                Err(cause) => {
                    output::warning(&format!("{cause}; try again, or press Ctrl-D to skip"));
                }
            }
        }
    }
}

fn run_init(cwd: &Path, dir: Option<&str>, user: Option<&str>) -> anyhow::Result<()> {
    let chosen = user
        .map(DisplayName::new)
        .transpose()
        .context("the --user name cannot be used")?;
    let resolved = files::resolve_settings(cwd, dir)?;
    for warning in &resolved.warnings {
        output::warning(warning);
    }

    let mut store = FilesStore::new(cwd.to_path_buf(), resolved.config, cwd.to_path_buf());
    let mut settings_file = user_settings();
    let done = init::init(&mut store, &mut settings_file, chosen, &mut TerminalAsker)?;

    for Location(place) in &done.store.created {
        output::line(&format!("created {place}"));
    }
    for Location(place) in &done.store.kept {
        output::line(&format!("kept {place} (already exists)"));
    }
    match done.name {
        NameOutcome::Set(name) => output::line(&format!("recorded your name as {name}")),
        NameOutcome::Kept(name) => output::line(&format!("kept your name as {name}")),
        NameOutcome::Missing => {
            output::warning("no name recorded, so stage lines will have no author");
            output::warning(&name_hint(&settings_file));
        }
    }
    Ok(())
}

/// What to tell someone who has no name set, which depends on whether there is
/// anywhere to keep it.
fn name_hint(settings_file: &UserFile) -> String {
    if settings_file.path().is_some() {
        "run `decider init --user NAME` to set one".to_owned()
    } else {
        "set HOME or XDG_CONFIG_HOME to an absolute path, then run `decider init --user NAME`"
            .to_owned()
    }
}

fn run_propose(cwd: &Path, title: &str) -> anyhow::Result<()> {
    let Some((root, config)) = files::find_settings(cwd)? else {
        bail!(
            "no {} found in {} or any parent directory; run `decider init` first",
            settings::FILE_NAME,
            cwd.display()
        );
    };
    let mut store = FilesStore::new(root, config, cwd.to_path_buf());
    let settings_file = user_settings();
    let proposed = propose::propose(&mut store, &settings_file, title, clock::today()?)?;
    output::line(&format!("created {}", proposed.location.0));
    if proposed.author_missing {
        output::warning("no name is set, so the Proposed line has no author");
        output::warning(&name_hint(&settings_file));
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    init_tracing()?;
    let cli = Cli::parse();
    let cwd = std::env::current_dir().context("could not read the current directory")?;
    match cli.command {
        Command::Init { dir, user } => run_init(&cwd, dir.as_deref(), user.as_deref()),
        Command::Propose { title } => run_propose(&cwd, &title),
    }
}
