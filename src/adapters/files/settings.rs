//! The `.decider.toml` file (ADR 0003): reading, checking and writing it.

use std::path::{Component, Path};

use serde::Deserialize;

/// The name of the configuration file, kept at the repository root.
pub const FILE_NAME: &str = ".decider.toml";

/// The ADR directory `init` uses when it is not given one.
pub const DEFAULT_DIR: &str = "docs/explanations/decisions";

/// A configuration file that cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The text is not valid TOML, has no `dir`, or has a key that is not known.
    #[error("invalid .decider.toml: {0}")]
    Parse(#[from] toml::de::Error),
    /// The ADR directory names nothing, for example `""` or `"./"`.
    #[error("the ADR directory must name a directory below the repository root")]
    EmptyDir,
    /// The ADR directory starts at a root or a drive instead of the repository root.
    #[error("the ADR directory {0:?} must be relative to the repository root")]
    AbsoluteDir(String),
    /// The ADR directory climbs out of the repository root.
    #[error("the ADR directory {0:?} must not contain `..`")]
    ParentDir(String),
    /// The ADR directory starts with a name that belongs to something else.
    #[error("the ADR directory {0:?} must not start with `{1}`")]
    ReservedName(String, &'static str),
}

/// The settings in `.decider.toml`.
///
/// The ADR directory is checked when a value is built, so a `Config` always
/// names a relative path below the repository root with no `..` part and whose
/// first part is not `.git` or `.decider.toml` (in any ASCII case). The check
/// reads only the text of the path. Whether a symlink on the way leads outside the repository is checked by the
/// store when it is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    dir: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    dir: String,
}

/// Names that an ADR directory may not start with: the settings file would
/// collide with the directory, and `.git` is not a place for documents. They are
/// compared without regard to ASCII case, because on a case-insensitive
/// filesystem (the default on macOS and Windows) `.GIT` is the same entry.
const RESERVED_FIRST_PARTS: [&str; 2] = [FILE_NAME, ".git"];

/// Turn `dir` into `a/b/c` form, dropping `.` parts and repeated separators.
fn normalise(dir: &str) -> Result<String, ConfigError> {
    let mut parts = Vec::new();
    for component in Path::new(dir).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => return Err(ConfigError::ParentDir(dir.to_owned())),
            Component::RootDir | Component::Prefix(_) => {
                return Err(ConfigError::AbsoluteDir(dir.to_owned()));
            }
        }
    }
    for reserved in RESERVED_FIRST_PARTS {
        if parts
            .first()
            .is_some_and(|first| first.eq_ignore_ascii_case(reserved))
        {
            return Err(ConfigError::ReservedName(dir.to_owned(), reserved));
        }
    }
    if parts.is_empty() {
        Err(ConfigError::EmptyDir)
    } else {
        Ok(parts.join("/"))
    }
}

impl Config {
    /// Build a configuration for the ADR directory `dir`.
    ///
    /// `dir` is stored with `.` parts and repeated separators removed, so
    /// `./docs//adr/` becomes `docs/adr`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::EmptyDir`] if nothing is left after cleaning,
    /// [`ConfigError::AbsoluteDir`] if `dir` starts at a root or a drive,
    /// [`ConfigError::ParentDir`] if it has a `..` part, and
    /// [`ConfigError::ReservedName`] if its first part is `.git` or `.decider.toml`
    /// in any ASCII case.
    pub fn new(dir: &str) -> Result<Self, ConfigError> {
        Ok(Self {
            dir: normalise(dir)?,
        })
    }

    /// Read the text of a `.decider.toml`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Parse`] if the text is not valid TOML, has no
    /// `dir`, or has any other key, and the errors of [`Config::new`] if `dir`
    /// is not a usable directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use decider_adr::adapters::files::settings::Config;
    /// use std::path::Path;
    ///
    /// let config = Config::parse("dir = \"docs/adr\"\n")?;
    /// assert_eq!(config.dir(), Path::new("docs/adr"));
    /// # Ok::<(), decider_adr::adapters::files::settings::ConfigError>(())
    /// ```
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let raw: Raw = toml::from_str(text)?;
        Self::new(&raw.dir)
    }

    /// The ADR directory, relative to the directory that holds `.decider.toml`.
    pub fn dir(&self) -> &Path {
        Path::new(&self.dir)
    }

    /// The text of the file: `dir = "..."` and a final newline.
    pub fn render(&self) -> String {
        format!("dir = {}\n", toml::Value::String(self.dir.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_directory() {
        let config = Config::parse("dir = \"docs/adr\"\n").unwrap();
        assert_eq!(config.dir(), Path::new("docs/adr"), "directory");
    }

    #[test]
    fn writes_the_directory_on_one_line() {
        let config = Config::new(DEFAULT_DIR).unwrap();
        assert_eq!(
            config.render(),
            "dir = \"docs/explanations/decisions\"\n",
            "exact file text"
        );
    }

    #[test]
    fn the_written_text_reads_back_to_the_same_config() {
        for dir in ["docs/adr", "a", "with space/and\"quote", "back\\slash/x"] {
            let config = Config::new(dir).unwrap();
            assert_eq!(Config::parse(&config.render()).unwrap(), config, "{dir}");
        }
    }

    #[test]
    fn cleans_dot_parts_and_repeated_separators() {
        assert_eq!(
            Config::new("./docs//adr/").unwrap().dir(),
            Path::new("docs/adr"),
            "cleaned"
        );
    }

    #[test]
    fn an_empty_directory_is_refused() {
        for dir in ["", ".", "./", "./."] {
            assert!(
                matches!(Config::new(dir), Err(ConfigError::EmptyDir)),
                "{dir:?} names no directory"
            );
        }
    }

    #[test]
    fn an_absolute_directory_is_refused() {
        assert!(
            matches!(Config::new("/etc/adr"), Err(ConfigError::AbsoluteDir(_))),
            "starts at the filesystem root"
        );
    }

    #[test]
    fn a_directory_with_a_parent_part_is_refused() {
        for dir in ["..", "../adr", "docs/../../adr", "docs/.."] {
            assert!(
                matches!(Config::new(dir), Err(ConfigError::ParentDir(_))),
                "{dir:?} climbs out of the root"
            );
        }
    }

    #[test]
    fn a_directory_starting_with_the_settings_file_or_git_is_refused() {
        for dir in [".decider.toml", ".decider.toml/adr", ".git", "./.git/hooks"] {
            assert!(
                matches!(Config::new(dir), Err(ConfigError::ReservedName(..))),
                "{dir:?} collides with something else"
            );
        }
    }

    #[test]
    fn reserved_names_are_matched_without_regard_to_case() {
        for dir in [".Decider.toml", ".DECIDER.TOML/adr", ".GIT", ".Git/hooks"] {
            assert!(
                matches!(Config::new(dir), Err(ConfigError::ReservedName(..))),
                "{dir:?} is the same entry on a case-insensitive filesystem"
            );
        }
    }

    #[test]
    fn a_name_that_only_begins_like_a_reserved_one_is_fine() {
        assert!(Config::new(".github/adr").is_ok(), ".github is not .git");
        assert!(
            Config::new(".decider.toml.d/adr").is_ok(),
            "only the whole first part counts"
        );
    }

    #[test]
    fn a_reserved_name_further_down_is_fine() {
        assert!(
            Config::new("docs/.git-notes").is_ok(),
            "only the first part counts"
        );
        assert!(Config::new("docs/.decider.toml").is_ok(), "not at the root");
    }

    #[test]
    fn a_hand_edited_bad_directory_is_refused_when_read() {
        assert!(
            matches!(
                Config::parse("dir = \"../outside\"\n"),
                Err(ConfigError::ParentDir(_))
            ),
            "the check also applies to files"
        );
    }

    #[test]
    fn a_file_without_a_directory_is_refused() {
        assert!(
            matches!(Config::parse(""), Err(ConfigError::Parse(_))),
            "dir is required"
        );
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let error = Config::parse("dir = \"docs\"\ndirr = \"typo\"\n").unwrap_err();
        assert!(
            matches!(&error, ConfigError::Parse(_)) && error.to_string().contains("dirr"),
            "{error}"
        );
    }

    #[test]
    fn text_that_is_not_toml_is_refused() {
        assert!(
            matches!(Config::parse("dir = "), Err(ConfigError::Parse(_))),
            "not TOML"
        );
    }

    #[test]
    fn a_directory_that_is_not_a_string_is_refused() {
        assert!(
            matches!(Config::parse("dir = 3\n"), Err(ConfigError::Parse(_))),
            "wrong type"
        );
    }
}
