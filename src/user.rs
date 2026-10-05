//! The user's own settings (ADR 0002): a name and an editor, kept outside the
//! repository.
//!
//! Nothing but the tool would supply these, so this is a plain module and not a
//! port.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::Error;

/// The settings file name.
const FILE_NAME: &str = "config.toml";

/// The directory inside the user's configuration directory.
const DIR_NAME: &str = "decider";

/// The contents of the settings file.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    name: Option<String>,
    editor: Option<String>,
}

/// Where the settings file is, given the environment: under `xdg_config_home`
/// when that is set to an absolute path, otherwise under `.config` in `home`.
/// Returns `None` if neither gives an absolute place.
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

/// Turn a name someone typed into one that can be written after `by`.
///
/// The name is trimmed. Nothing left means there is no name.
///
/// # Errors
///
/// Returns [`Error::NameSpansLines`] if the name has a line break inside it,
/// because that would split the header it is written into.
pub fn clean_name(text: &str) -> Result<Option<String>, Error> {
    let text = text.trim();
    if text.contains(['\n', '\r']) {
        return Err(Error::NameSpansLines);
    }
    Ok((!text.is_empty()).then(|| text.to_owned()))
}

/// Work out a name from a login such as `youcef.kadri`, which gives
/// `Youcef Kadri`: split on `.`, `_` and `-` and capitalise each part.
pub fn infer_name(login: &str) -> Option<String> {
    let words: Vec<String> = login
        .split(['.', '_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(characters).collect()
            })
        })
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// The name offered as the default when asking: the name already set, if any,
/// so that pressing Enter changes nothing, otherwise one worked out from the login.
pub fn suggest_name(current: Option<&str>, login: Option<&str>) -> Option<String> {
    current
        .map(str::to_owned)
        .or_else(|| login.and_then(infer_name))
}

/// What a person typed in answer to the question about their name: a name, or
/// the suggestion when the answer is empty.
///
/// # Errors
///
/// Returns [`Error::NameSpansLines`] as [`clean_name`] does.
pub fn name_from_answer(answer: &str, suggestion: Option<&str>) -> Result<Option<String>, Error> {
    Ok(clean_name(answer)?.or_else(|| suggestion.map(str::to_owned)))
}

/// Read the name from the settings file at `path`.
///
/// Returns `None` if the file does not exist or has no name, or the name is blank.
///
/// # Errors
///
/// Returns [`Error`] if the file cannot be read or is not valid settings.
pub fn read_name(path: &Path) -> Result<Option<String>, Error> {
    // A blank name counts as no name, so `--user` can still set one.
    Ok(read(path)?.name.filter(|name| !name.trim().is_empty()))
}

/// The `editor` setting in the settings file at `path`, as written, or `None`
/// if the file or the key is missing.
///
/// # Errors
///
/// Returns [`Error`] if the file cannot be read or is not valid settings.
pub fn read_editor(path: &Path) -> Result<Option<String>, Error> {
    Ok(read(path)?.editor)
}

/// The command that opens a new ADR for editing, as a program and its
/// arguments: the `editor` setting if there is one, otherwise the `EDITOR`
/// environment variable. A value is split on whitespace, so `code --wait` works,
/// and a blank one counts as not set. `None` if neither names an editor.
pub fn editor_command(setting: Option<&str>, environment: Option<&str>) -> Option<Vec<String>> {
    [setting, environment]
        .into_iter()
        .flatten()
        .map(|text| {
            text.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .find(|words| !words.is_empty())
}

/// Run `command` (a program and its arguments, from [`editor_command`]) with
/// `path` added as the last argument, on the terminal the tool was started from,
/// and wait for it to finish.
///
/// # Errors
///
/// Returns [`Error`] if the editor cannot be started or exits with a failure.
pub fn open_in_editor(command: &[String], path: &Path) -> Result<(), Error> {
    let Some((program, arguments)) = command.split_first() else {
        return Ok(());
    };
    let status = std::process::Command::new(program)
        .args(arguments)
        .arg(path)
        .status()
        .map_err(|cause| Error::io("start the editor", Path::new(program), cause))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::EditorFailed(program.clone(), status.to_string()))
    }
}

/// The settings in the file at `path`; none if there is no file.
fn read(path: &Path) -> Result<Raw, Error> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(Raw::default()),
        Err(cause) => return Err(Error::io("read", path, cause)),
    };
    toml::from_str(&text).map_err(|cause| Error::settings(path, cause.to_string()))
}

/// Whether `line` sets the `name` key, such as `name = ""`.
fn is_name_line(line: &str) -> bool {
    line.trim_start()
        .strip_prefix("name")
        .is_some_and(|rest| rest.trim_start().starts_with('='))
}

/// Record `name` in the settings file at `path`, in front of the rest of the
/// file, creating the file and its directory if needed.
///
/// It is only called when the file has no usable name, so any `name` line
/// already there is blank, and it is replaced. The rest of the file is kept.
///
/// # Errors
///
/// Returns [`Error`] if the file cannot be read or written.
pub fn write_name(path: &Path, name: &str) -> Result<(), Error> {
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => String::new(),
        Err(cause) => return Err(Error::io("read", path, cause)),
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|cause| Error::io("create", parent, cause))?;
    }
    let line = format!("name = {}\n", toml::Value::String(name.to_owned()));
    let rest: String = existing
        .split_inclusive('\n')
        .filter(|line| !is_name_line(line))
        .collect();
    fs::write(path, format!("{line}{rest}")).map_err(|cause| Error::io("write", path, cause))
}

/// Which settings `init` is being asked to set again with `--force`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reset {
    /// `--force` was not given: only what is missing is set.
    Nothing,
    /// `--force` with `--dir` only.
    Dir,
    /// `--force` with `--user` only.
    Name,
    /// `--force` with both, or with neither.
    Both,
}

impl Reset {
    /// Which settings `--force` resets, given which values were passed.
    ///
    /// `--force` alone resets both. A value that was given resets what it
    /// stands for, and leaves the other setting alone.
    pub fn new(force: bool, dir_given: bool, name_given: bool) -> Self {
        match (force, dir_given, name_given) {
            (false, _, _) => Self::Nothing,
            (true, true, false) => Self::Dir,
            (true, false, true) => Self::Name,
            (true, _, _) => Self::Both,
        }
    }

    /// Whether the ADR directory is set again, replacing one that exists.
    pub fn dir(self) -> bool {
        matches!(self, Self::Dir | Self::Both)
    }

    /// Whether the name is set again, replacing one that is set.
    pub fn name(self) -> bool {
        matches!(self, Self::Name | Self::Both)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_force_nothing_is_reset() {
        for (dir_given, name_given) in [(false, false), (true, false), (false, true), (true, true)]
        {
            let reset = Reset::new(false, dir_given, name_given);
            assert_eq!(reset, Reset::Nothing, "{dir_given} {name_given}");
            assert!(!reset.dir() && !reset.name(), "neither is reset");
        }
    }

    #[test]
    fn force_alone_resets_both() {
        let reset = Reset::new(true, false, false);
        assert_eq!(reset, Reset::Both, "both");
        assert!(reset.dir() && reset.name(), "both are reset");
    }

    #[test]
    fn force_with_a_directory_resets_only_the_directory() {
        let reset = Reset::new(true, true, false);
        assert!(reset.dir() && !reset.name(), "the name is left alone");
    }

    #[test]
    fn force_with_a_name_resets_only_the_name() {
        let reset = Reset::new(true, false, true);
        assert!(!reset.dir() && reset.name(), "the directory is left alone");
    }

    #[test]
    fn force_with_both_resets_both() {
        let reset = Reset::new(true, true, true);
        assert!(reset.dir() && reset.name(), "both are reset");
    }

    #[test]
    fn the_name_already_set_is_the_default_when_asking_again() {
        assert_eq!(
            suggest_name(Some("Ada Lovelace"), Some("grace.hopper")).as_deref(),
            Some("Ada Lovelace"),
            "the current name wins over the login"
        );
    }

    #[test]
    fn with_no_name_set_the_login_is_the_default() {
        assert_eq!(
            suggest_name(None, Some("grace.hopper")).as_deref(),
            Some("Grace Hopper"),
            "from the login"
        );
        assert_eq!(suggest_name(None, None), None, "nothing to offer");
        assert_eq!(
            suggest_name(None, Some("...")),
            None,
            "a login that gives no name"
        );
    }

    #[test]
    fn a_login_becomes_a_capitalised_name() {
        for (login, expected) in [
            ("youcef.kadri", "Youcef Kadri"),
            ("ada_lovelace", "Ada Lovelace"),
            ("grace-hopper", "Grace Hopper"),
            ("root", "Root"),
        ] {
            assert_eq!(infer_name(login).as_deref(), Some(expected), "{login}");
        }
    }

    #[test]
    fn a_login_of_only_separators_gives_no_name() {
        for login in ["", ".", "_-."] {
            assert_eq!(infer_name(login), None, "{login:?}");
        }
    }

    #[test]
    fn a_name_is_trimmed_and_may_contain_parentheses() {
        assert_eq!(
            clean_name("  Jane Doe (Contractor) ").unwrap().as_deref(),
            Some("Jane Doe (Contractor)"),
            "trimmed, parentheses kept"
        );
    }

    #[test]
    fn nothing_but_whitespace_is_no_name() {
        for text in ["", "   ", "\n", " \t \n "] {
            assert_eq!(clean_name(text).unwrap(), None, "{text:?}");
        }
    }

    #[test]
    fn a_name_with_a_line_break_inside_is_refused() {
        assert!(
            matches!(clean_name("Ada\nLovelace"), Err(Error::NameSpansLines)),
            "would split the header"
        );
    }

    #[test]
    fn an_empty_answer_takes_the_suggestion() {
        assert_eq!(
            name_from_answer("  \n", Some("Youcef Kadri"))
                .unwrap()
                .as_deref(),
            Some("Youcef Kadri"),
            "suggestion"
        );
    }

    #[test]
    fn an_empty_answer_with_no_suggestion_gives_no_name() {
        assert_eq!(name_from_answer("", None).unwrap(), None, "nothing");
    }

    #[test]
    fn a_typed_answer_wins_over_the_suggestion() {
        assert_eq!(
            name_from_answer("Grace Hopper\n", Some("Youcef Kadri"))
                .unwrap()
                .as_deref(),
            Some("Grace Hopper"),
            "typed answer"
        );
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
    fn home_is_used_without_an_absolute_xdg_config_home() {
        let expected = Some(PathBuf::from("/home/ada/.config/decider/config.toml"));
        assert_eq!(
            settings_path(None, Some("/home/ada".into())),
            expected,
            "no xdg"
        );
        assert_eq!(
            settings_path(Some("relative".into()), Some("/home/ada".into())),
            expected,
            "relative xdg is ignored"
        );
    }

    #[test]
    fn no_absolute_place_gives_no_path() {
        assert_eq!(settings_path(None, None), None, "nothing set");
        assert_eq!(
            settings_path(None, Some("relative".into())),
            None,
            "relative home"
        );
    }

    #[test]
    fn a_missing_file_has_no_name() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            read_name(&dir.path().join("none.toml")).unwrap(),
            None,
            "no file"
        );
    }

    #[test]
    fn a_name_written_is_read_back_and_the_directory_is_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("decider").join("config.toml");
        write_name(&path, "Ada \"Countess\" Lovelace").unwrap();
        assert_eq!(
            read_name(&path).unwrap().as_deref(),
            Some("Ada \"Countess\" Lovelace"),
            "round trip, with quotes"
        );
    }

    #[test]
    fn adding_a_name_keeps_what_the_file_already_holds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "# my settings\n").unwrap();
        write_name(&path, "Ada").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "name = \"Ada\"\n# my settings\n",
            "comment kept after the name"
        );
    }

    #[test]
    fn the_editor_is_read_from_the_file_and_a_missing_one_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert_eq!(read_editor(&path).unwrap(), None, "no file");
        fs::write(&path, "name = \"Ada\"\neditor = \"code --wait\"\n").unwrap();
        assert_eq!(
            read_editor(&path).unwrap().as_deref(),
            Some("code --wait"),
            "as written"
        );
        assert_eq!(
            read_name(&path).unwrap().as_deref(),
            Some("Ada"),
            "name too"
        );
        fs::write(&path, "name = \"Ada\"\n").unwrap();
        assert_eq!(read_editor(&path).unwrap(), None, "no key");
    }

    #[test]
    fn recording_a_name_keeps_the_editor_setting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "editor = \"vim\"\n").unwrap();
        write_name(&path, "Ada").unwrap();
        assert_eq!(
            read_editor(&path).unwrap().as_deref(),
            Some("vim"),
            "editor"
        );
        assert_eq!(read_name(&path).unwrap().as_deref(), Some("Ada"), "name");
    }

    #[cfg(unix)]
    #[test]
    fn the_editor_is_run_with_the_file_as_its_last_argument() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adr.md");
        fs::write(&path, "before\n").unwrap();
        let command = ["sh", "-c", "echo edited >> \"$1\"", "sh"].map(str::to_owned);
        open_in_editor(&command, &path).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "before\nedited\n",
            "ran"
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_editor_that_fails_or_is_missing_is_an_error() {
        let path = Path::new("adr.md");
        let failing = ["false"].map(str::to_owned);
        assert!(
            matches!(open_in_editor(&failing, path), Err(Error::EditorFailed(..))),
            "exit status"
        );
        let missing = ["no-such-editor-anywhere"].map(str::to_owned);
        assert!(
            matches!(open_in_editor(&missing, path), Err(Error::Io { .. })),
            "not found"
        );
    }

    #[test]
    fn the_setting_wins_over_the_environment() {
        assert_eq!(
            editor_command(Some("code --wait"), Some("vim")),
            Some(vec!["code".to_owned(), "--wait".to_owned()]),
            "setting, split into program and argument"
        );
    }

    #[test]
    fn the_environment_is_the_default_when_there_is_no_setting() {
        assert_eq!(
            editor_command(None, Some("vim")),
            Some(vec!["vim".to_owned()]),
            "environment"
        );
    }

    #[test]
    fn a_blank_setting_or_variable_counts_as_not_set() {
        assert_eq!(
            editor_command(Some("  "), Some("vim")),
            Some(vec!["vim".to_owned()]),
            "blank setting falls through"
        );
        assert_eq!(editor_command(Some(" "), Some("")), None, "both blank");
        assert_eq!(editor_command(None, None), None, "neither");
    }

    #[test]
    fn a_blank_name_in_the_file_counts_as_no_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for text in ["name = \"\"\n", "name = \"   \"\n"] {
            fs::write(&path, text).unwrap();
            assert_eq!(read_name(&path).unwrap(), None, "{text:?}");
        }
    }

    #[test]
    fn recording_a_name_replaces_a_blank_one_and_the_file_still_reads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for blank in ["name = \"\"\n", "name=\"  \"\n", "  name   =  \"\"\n"] {
            fs::write(&path, format!("# mine\n{blank}")).unwrap();
            write_name(&path, "Grace").unwrap();
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                "name = \"Grace\"\n# mine\n",
                "one name, comment kept, for {blank:?}"
            );
            assert_eq!(
                read_name(&path).unwrap().as_deref(),
                Some("Grace"),
                "reads back"
            );
        }
    }

    #[test]
    fn a_key_that_only_starts_with_name_is_left_alone() {
        assert!(!is_name_line("nickname = \"x\"\n"), "nickname");
        assert!(!is_name_line("names = []\n"), "names");
        assert!(is_name_line("name = \"x\"\n"), "name");
    }

    #[test]
    fn a_file_with_an_unknown_key_is_an_error_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "nmae = \"x\"\n").unwrap();
        let message = read_name(&path).unwrap_err().to_string();
        assert!(
            message.contains("config.toml") && message.contains("nmae"),
            "{message}"
        );
    }
}
