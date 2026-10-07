//! The error type for the library.

use std::io;
use std::path::{Path, PathBuf};

/// What can go wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file or directory could not be read, created or written.
    #[error("could not {action} {}: {source}", .path.display())]
    Io {
        /// What was being done, such as `read`.
        action: &'static str,
        /// The file or directory.
        path: PathBuf,
        /// The cause.
        source: io::Error,
    },
    /// A settings file is not what the tool expects.
    #[error("{}: {message}", .path.display())]
    Settings {
        /// The settings file.
        path: PathBuf,
        /// What is wrong with it.
        message: String,
    },
    /// The ADR directory is empty.
    #[error("the ADR directory {0:?} must not be empty")]
    EmptyDir(String),
    /// A name has more than one line.
    #[error("a name must be on one line")]
    NameSpansLines,
    /// There is nowhere to keep the user's settings.
    #[error("cannot find a place for your settings; set HOME or XDG_CONFIG_HOME")]
    NoSettingsPath,
    /// The title of a new ADR is empty or spans lines.
    #[error("the title must be one line of text, such as \"Use Postgres\"")]
    BadTitle,
    /// The title has nothing a filename can be made from.
    #[error("the title {0:?} has no letters or digits to make a filename from")]
    NoSlug(String),
    /// Every ADR number is used.
    #[error("no ADR number is left")]
    NoNumberLeft,
    /// No ADR has the number.
    #[error("there is no ADR number {0}")]
    NoSuchAdr(u32),
    /// More than one ADR has the number.
    #[error("more than one ADR has the number {0}: {1}")]
    AmbiguousAdr(u32, String),
    /// A move the lifecycle does not allow.
    #[error("{0}")]
    NotAllowed(String),
    /// An ADR has no `**Status:**` line in its header.
    #[error("{}: the ADR has no `**Status:**` line in its header", .0.display())]
    NoStatus(PathBuf),
    /// The status in an ADR is not one of the four.
    #[error("{}: {message}", .path.display())]
    BadStatus {
        /// The ADR.
        path: PathBuf,
        /// What is wrong with it.
        message: String,
    },
    /// The editor ran and reported a failure.
    #[error("the editor {0} did not finish cleanly ({1})")]
    EditorFailed(String, String),
    /// A rejection has no reason.
    #[error("a rejection needs a reason")]
    NoReason,
}

impl Error {
    pub(crate) fn io(action: &'static str, path: &Path, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.to_path_buf(),
            source,
        }
    }

    pub(crate) fn settings(path: &Path, message: impl Into<String>) -> Self {
        Self::Settings {
            path: path.to_path_buf(),
            message: message.into(),
        }
    }
}
