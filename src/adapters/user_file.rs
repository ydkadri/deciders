//! The user's own settings file (ADR 0003): `config.toml` in the `decider`
//! directory under the user's configuration directory. It is never committed.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::person::DisplayName;
use crate::ports::{KeepsUserSettings, StoreError};

/// The name of the settings file.
pub const FILE_NAME: &str = "config.toml";

/// The directory that holds it, inside the user's configuration directory.
const DIR_NAME: &str = "decider";

/// Where the settings file is, given the environment.
///
/// `xdg_config_home` is used when it is set to an absolute path, as the XDG
/// specification says. Otherwise the file is under `home`, in `.config`, when
/// `home` is an absolute path. A relative one would put the file inside
/// whichever directory the tool was started in, which may be a repository.
/// Returns `None` if neither gives a place.
pub fn settings_path(xdg_config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let xdg = xdg_config_home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute());
    let base = xdg.or_else(|| {
        home.map(PathBuf::from)
            .filter(|home| home.is_absolute())
            .map(|home| home.join(".config"))
    })?;
    Some(base.join(DIR_NAME).join(FILE_NAME))
}

/// The settings a person keeps for themselves.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    name: Option<String>,
}

/// The user's settings in a TOML file, or nowhere if no place could be found.
#[derive(Debug)]
pub struct UserFile {
    path: Option<PathBuf>,
}

impl UserFile {
    /// Keep settings in the file at `path`, or nowhere when there is none.
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    /// Where the settings are kept, if anywhere.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl KeepsUserSettings for UserFile {
    fn name(&self) -> Result<Option<DisplayName>, StoreError> {
        let Some(path) = &self.path else {
            return Ok(None);
        };
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(StoreError(format!(
                    "could not read {}: {error}",
                    path.display()
                )));
            }
        };
        let raw: Raw = toml::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text))
            .map_err(|error| StoreError(format!("could not use {}: {error}", path.display())))?;
        raw.name
            .map(|name| {
                DisplayName::new(&name).map_err(|error| {
                    StoreError(format!(
                        "the name in {} cannot be used: {error}",
                        path.display()
                    ))
                })
            })
            .transpose()
    }

    fn set_name(&mut self, name: &DisplayName) -> Result<(), StoreError> {
        let Some(path) = &self.path else {
            return Err(StoreError(
                "cannot find a place for your settings; set HOME or XDG_CONFIG_HOME".to_owned(),
            ));
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                StoreError(format!("could not create {}: {error}", parent.display()))
            })?;
        }
        let line = format!("name = {}\n", toml::Value::String(name.as_str().to_owned()));
        // The name goes first and whatever the file already holds is kept after
        // it. In practice that is comments: the reader refuses other keys.
        let existing = match fs::read_to_string(path) {
            Ok(text) => text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(StoreError(format!(
                    "could not read {}: {error}",
                    path.display()
                )));
            }
        };
        fs::write(path, format!("{line}{existing}"))
            .map_err(|error| StoreError(format!("could not write {}: {error}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_in(directory: &tempfile::TempDir) -> UserFile {
        UserFile::new(Some(directory.path().join("decider").join(FILE_NAME)))
    }

    #[test]
    fn a_missing_file_has_no_name() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(file_in(&directory).name().unwrap(), None, "no file yet");
    }

    #[test]
    fn a_name_written_is_read_back() {
        let directory = tempfile::tempdir().unwrap();
        let mut file = file_in(&directory);
        let name = DisplayName::new("Ada \"Countess\" Lovelace").unwrap();
        file.set_name(&name).unwrap();
        assert_eq!(
            file.name().unwrap(),
            Some(name),
            "round trip, with a quote in it"
        );
    }

    #[test]
    fn writing_creates_the_directories() {
        let directory = tempfile::tempdir().unwrap();
        let mut file = file_in(&directory);
        file.set_name(&DisplayName::new("Ada").unwrap()).unwrap();
        assert!(
            directory.path().join("decider").join(FILE_NAME).is_file(),
            "file exists"
        );
    }

    #[test]
    fn a_file_with_no_name_has_no_name() {
        let directory = tempfile::tempdir().unwrap();
        let file = file_in(&directory);
        fs::create_dir_all(directory.path().join("decider")).unwrap();
        fs::write(directory.path().join("decider").join(FILE_NAME), "").unwrap();
        assert_eq!(file.name().unwrap(), None, "empty file");
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let file = file_in(&directory);
        fs::create_dir_all(directory.path().join("decider")).unwrap();
        fs::write(
            directory.path().join("decider").join(FILE_NAME),
            "nmae = \"x\"\n",
        )
        .unwrap();
        assert!(
            file.name().unwrap_err().to_string().contains("nmae"),
            "typo is reported"
        );
    }

    #[test]
    fn a_name_the_stage_line_rules_refuse_is_an_error_when_read() {
        let directory = tempfile::tempdir().unwrap();
        let file = file_in(&directory);
        fs::create_dir_all(directory.path().join("decider")).unwrap();
        fs::write(
            directory.path().join("decider").join(FILE_NAME),
            "name = \"Jane (Contractor)\"\n",
        )
        .unwrap();
        let message = file.name().unwrap_err().to_string();
        assert!(
            message.contains("cannot be used") && message.contains("parenthesis"),
            "{message}"
        );
    }

    #[test]
    fn an_empty_name_in_the_file_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let file = file_in(&directory);
        fs::create_dir_all(directory.path().join("decider")).unwrap();
        fs::write(
            directory.path().join("decider").join(FILE_NAME),
            "name = \"  \"\n",
        )
        .unwrap();
        assert!(
            file.name().unwrap_err().to_string().contains("empty"),
            "blank name"
        );
    }

    #[test]
    fn no_place_means_no_name_and_nowhere_to_write() {
        let mut file = UserFile::new(None);
        assert_eq!(file.name().unwrap(), None, "reading is fine");
        let error = file
            .set_name(&DisplayName::new("Ada").unwrap())
            .unwrap_err();
        assert!(error.to_string().contains("HOME"), "{error}");
    }

    #[test]
    fn xdg_config_home_wins_when_absolute() {
        assert_eq!(
            settings_path(Some("/xdg".into()), Some("/home/ada".into())),
            Some(PathBuf::from("/xdg/decider/config.toml")),
            "xdg"
        );
    }

    #[test]
    fn a_relative_xdg_config_home_is_ignored() {
        assert_eq!(
            settings_path(Some("relative".into()), Some("/home/ada".into())),
            Some(PathBuf::from("/home/ada/.config/decider/config.toml")),
            "falls back to home"
        );
    }

    #[test]
    fn home_is_used_without_xdg() {
        assert_eq!(
            settings_path(None, Some("/home/ada".into())),
            Some(PathBuf::from("/home/ada/.config/decider/config.toml")),
            "home"
        );
    }

    #[test]
    fn an_empty_home_and_no_xdg_give_no_path() {
        assert_eq!(settings_path(None, Some("".into())), None, "empty home");
        assert_eq!(settings_path(None, None), None, "nothing set");
    }

    #[test]
    fn a_relative_home_gives_no_path() {
        assert_eq!(
            settings_path(None, Some("relative/home".into())),
            None,
            "relative home"
        );
    }

    #[test]
    fn adding_a_name_keeps_comments_and_other_content() {
        let directory = tempfile::tempdir().unwrap();
        let mut file = file_in(&directory);
        let path = directory.path().join("decider").join(FILE_NAME);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "# my decider settings, keep\n").unwrap();
        file.set_name(&DisplayName::new("Ada").unwrap()).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "name = \"Ada\"\n# my decider settings, keep\n",
            "the comment survives"
        );
        assert_eq!(
            file.name().unwrap().unwrap().as_str(),
            "Ada",
            "and the name reads back"
        );
    }

    #[test]
    fn a_file_with_other_keys_is_refused_when_read() {
        let directory = tempfile::tempdir().unwrap();
        let file = file_in(&directory);
        let path = directory.path().join("decider").join(FILE_NAME);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "[other]\nx = 1\n").unwrap();
        assert!(
            file.name().unwrap_err().to_string().contains("other"),
            "unknown table"
        );
    }
}
