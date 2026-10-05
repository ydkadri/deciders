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
