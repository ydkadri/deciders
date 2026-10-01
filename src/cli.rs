//! The command line: arguments and subcommands. This is the driving adapter.

use clap::{Parser, Subcommand};

/// Manage architecture decision records through their lifecycle.
#[derive(Debug, Parser)]
#[command(name = "decider", version, arg_required_else_help = true)]
pub(crate) struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// The commands `decider` runs.
#[derive(Debug, Subcommand, PartialEq, Eq)]
pub(crate) enum Command {
    /// Set up the repository and your settings: `.decider.toml`, the ADR
    /// directory, the template and your name. Safe to run again.
    Init {
        /// The ADR directory, relative to the current directory.
        #[arg(long, value_name = "PATH")]
        dir: Option<String>,
        /// Your name for the stage lines. Asked for if not given and a terminal
        /// is attached, with a suggestion worked out from `$USER`.
        #[arg(long, value_name = "NAME")]
        user: Option<String>,
    },
    /// Propose a new ADR: create it in the `proposed` state.
    Propose {
        /// The title, such as "Use Postgres".
        title: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("decider").chain(arguments.iter().copied()))
    }

    #[test]
    fn init_takes_an_optional_directory_and_user() {
        assert_eq!(
            parse(&["init"]).unwrap().command,
            Command::Init {
                dir: None,
                user: None
            },
            "no options"
        );
        assert_eq!(
            parse(&["init", "--dir", "docs/adr", "--user", "Ada Lovelace"])
                .unwrap()
                .command,
            Command::Init {
                dir: Some("docs/adr".to_owned()),
                user: Some("Ada Lovelace".to_owned())
            },
            "both options"
        );
    }

    #[test]
    fn propose_takes_a_title() {
        assert_eq!(
            parse(&["propose", "Use Postgres"]).unwrap().command,
            Command::Propose {
                title: "Use Postgres".to_owned()
            },
            "title"
        );
    }

    #[test]
    fn propose_needs_a_title() {
        assert!(parse(&["propose"]).is_err(), "title is required");
    }

    #[test]
    fn propose_has_no_author_option() {
        assert!(
            parse(&["propose", "T", "--author", "Ada"]).is_err(),
            "the name comes from your settings"
        );
    }

    #[test]
    fn an_unknown_command_is_refused() {
        assert!(parse(&["frobnicate"]).is_err(), "unknown command");
    }

    #[test]
    fn no_command_is_refused() {
        assert!(parse(&[]).is_err(), "a command is required");
    }
}
