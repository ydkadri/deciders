//! The file adapter (ADR 0003): ADRs as Markdown files in a directory.
//!
//! It owns everything physical: the directory, the `.decider.toml` that says
//! where it is, and the ADR template file.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::error::Error;
use crate::store::{Prepared, WritesAdrs};

/// The repository settings file, kept at the repository root.
const SETTINGS_FILE: &str = ".decider.toml";

/// The ADR directory used when none is given.
pub const DEFAULT_DIR: &str = "docs/explanations/decisions";

/// The template file inside the ADR directory. Number 0 belongs to it.
const TEMPLATE_FILE: &str = "0000-template.md";

/// The template `init` writes, which is this repository's own.
const TEMPLATE: &str = include_str!("../docs/explanations/decisions/0000-template.md");

/// The contents of `.decider.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    dir: String,
}

/// Check that `dir` is not empty and tidy it: `.` parts and repeated separators
/// go, so `./docs//adr/` becomes `docs/adr`. An absolute path stays absolute.
fn tidy_dir(dir: &str) -> Result<String, Error> {
    let tidy: PathBuf = Path::new(dir)
        .components()
        .filter(|component| !matches!(component, Component::CurDir))
        .collect();
    if tidy.as_os_str().is_empty() {
        return Err(Error::EmptyDir(dir.to_owned()));
    }
    Ok(tidy.to_string_lossy().into_owned())
}

/// The `dir` setting in the `.decider.toml` in `directory`, if it has one.
///
/// # Errors
///
/// Returns [`Error`] if the file cannot be read or used.
pub fn settings_dir(directory: &Path) -> Result<Option<String>, Error> {
    let path = directory.join(SETTINGS_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => return Err(Error::io("read", &path, cause)),
    };
    let raw: Raw =
        toml::from_str(&text).map_err(|cause| Error::settings(&path, cause.to_string()))?;
    tidy_dir(&raw.dir)
        .map(Some)
        .map_err(|cause| Error::settings(&path, cause.to_string()))
}

/// What writing a file did.
enum Outcome {
    Created,
    Kept,
    Replaced,
}

/// Write `text` to `path`. A file that is already there is kept, unless
/// `replace` is set, in which case it is written over.
fn put(path: &Path, text: &str, replace: bool) -> Result<Outcome, Error> {
    let exists = path
        .try_exists()
        .map_err(|cause| Error::io("check", path, cause))?;
    if exists && !replace {
        return Ok(Outcome::Kept);
    }
    fs::write(path, text).map_err(|cause| Error::io("write", path, cause))?;
    Ok(if exists {
        Outcome::Replaced
    } else {
        Outcome::Created
    })
}

/// What a person typed when asked for the ADR directory: the directory, or
/// `default` when the answer is empty.
pub fn dir_from_answer<'a>(answer: &'a str, default: &'a str) -> &'a str {
    let answer = answer.trim();
    if answer.is_empty() { default } else { answer }
}

/// Check that `dir` can be used as the ADR directory, so a bad one is reported
/// before anything else is asked.
///
/// # Errors
///
/// Returns [`Error::EmptyDir`] if `dir` is empty.
pub fn check_dir(dir: &str) -> Result<(), Error> {
    tidy_dir(dir).map(|_| ())
}

/// ADRs stored as Markdown files under `root`.
#[derive(Debug)]
pub struct FilesStore {
    root: PathBuf,
    dir: String,
    replace_settings: bool,
}

impl FilesStore {
    /// The ADR directory.
    fn directory(&self) -> PathBuf {
        self.root.join(&self.dir)
    }

    /// A store whose ADR directory is `dir`: relative to `root`, or anywhere if
    /// it is absolute.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyDir`] if `dir` is empty.
    fn new(root: PathBuf, dir: &str) -> Result<Self, Error> {
        Ok(Self {
            root,
            dir: tidy_dir(dir)?,
            replace_settings: false,
        })
    }

    /// The store for `init` run in `root`, with the ADR directory `dir`, or the
    /// default. If `replace_settings` is set, `prepare` writes `.decider.toml`
    /// over one that is already there.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyDir`] if `dir` is empty.
    pub fn for_init(
        root: PathBuf,
        dir: Option<&str>,
        replace_settings: bool,
    ) -> Result<Self, Error> {
        let mut store = Self::new(root, dir.unwrap_or(DEFAULT_DIR))?;
        store.replace_settings = replace_settings;
        Ok(store)
    }
}

impl WritesAdrs for FilesStore {
    fn prepare(&mut self) -> Result<Prepared, Error> {
        let directory = self.directory();
        fs::create_dir_all(&directory).map_err(|cause| Error::io("create", &directory, cause))?;

        let template = directory.join(TEMPLATE_FILE);
        let settings = self.root.join(SETTINGS_FILE);
        // The settings are written last, so a run that fails part way can be
        // repeated.
        let template_outcome = put(&template, TEMPLATE, false)?;
        let settings_text = format!("dir = {}\n", toml::Value::String(self.dir.clone()));
        let settings_outcome = put(&settings, &settings_text, self.replace_settings)?;

        let mut prepared = Prepared::default();
        for (outcome, path) in [(settings_outcome, settings), (template_outcome, template)] {
            let shown = path.strip_prefix(&self.root).unwrap_or(&path).to_path_buf();
            match outcome {
                Outcome::Created => prepared.created.push(shown),
                Outcome::Kept => prepared.kept.push(shown),
                Outcome::Replaced => prepared.replaced.push(shown),
            }
        }
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &tempfile::TempDir, setting: &str) -> FilesStore {
        FilesStore::new(dir.path().to_path_buf(), setting).unwrap()
    }

    #[test]
    fn a_directory_is_tidied() {
        assert_eq!(tidy_dir("./docs//adr/").unwrap(), "docs/adr", "tidied");
    }

    #[test]
    fn an_empty_directory_is_refused() {
        for dir in ["", ".", "./"] {
            assert!(matches!(tidy_dir(dir), Err(Error::EmptyDir(_))), "{dir:?}");
        }
    }

    #[test]
    fn an_absolute_directory_is_kept_absolute_and_tidied() {
        assert_eq!(
            tidy_dir("/srv//docs/./adr/").unwrap(),
            "/srv/docs/adr",
            "tidied, still absolute"
        );
    }

    #[test]
    fn an_absolute_directory_outside_the_root_is_used_as_it_is() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let adrs = outside.path().join("adrs");
        let mut store = FilesStore::new(root.path().to_path_buf(), adrs.to_str().unwrap()).unwrap();
        assert_eq!(store.directory(), adrs, "not joined onto the root");
        let prepared = store.prepare().unwrap();
        assert!(
            adrs.join("0000-template.md").is_file(),
            "template written there"
        );
        assert!(
            !root.path().join("docs").exists(),
            "nothing under the root but the settings"
        );
        assert_eq!(
            fs::read_to_string(root.path().join(SETTINGS_FILE)).unwrap(),
            format!("dir = \"{}\"\n", adrs.display()),
            "the absolute path is what is recorded"
        );
        assert!(
            prepared.created.contains(&adrs.join("0000-template.md")),
            "shown in full, since it is not under the root"
        );
    }

    #[test]
    fn preparing_makes_the_directory_the_template_and_the_settings() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = store(&dir, "docs/adr").prepare().unwrap();
        assert_eq!(
            prepared.created,
            [
                PathBuf::from(SETTINGS_FILE),
                PathBuf::from("docs/adr/0000-template.md")
            ],
            "settings first"
        );
        assert!(prepared.kept.is_empty(), "nothing kept");
        assert_eq!(
            fs::read_to_string(dir.path().join(SETTINGS_FILE)).unwrap(),
            "dir = \"docs/adr\"\n",
            "settings"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("docs/adr/0000-template.md")).unwrap(),
            TEMPLATE,
            "the embedded template"
        );
    }

    #[test]
    fn preparing_again_keeps_everything() {
        let dir = tempfile::tempdir().unwrap();
        store(&dir, "adr").prepare().unwrap();
        fs::write(dir.path().join("adr/0000-template.md"), "# mine\n").unwrap();
        let second = store(&dir, "adr").prepare().unwrap();
        assert!(second.created.is_empty(), "nothing created");
        assert_eq!(second.kept.len(), 2, "both kept");
        assert_eq!(
            fs::read_to_string(dir.path().join("adr/0000-template.md")).unwrap(),
            "# mine\n",
            "the edited template is left alone"
        );
    }

    #[test]
    fn a_deleted_template_is_written_again() {
        let dir = tempfile::tempdir().unwrap();
        store(&dir, "adr").prepare().unwrap();
        fs::remove_file(dir.path().join("adr/0000-template.md")).unwrap();
        let second = store(&dir, "adr").prepare().unwrap();
        assert_eq!(
            second.created,
            [PathBuf::from("adr/0000-template.md")],
            "recreated"
        );
    }

    #[test]
    fn the_directory_in_the_settings_file_is_read() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(settings_dir(dir.path()).unwrap(), None, "no file yet");
        fs::write(dir.path().join(SETTINGS_FILE), "dir = \"./committed//\"\n").unwrap();
        assert_eq!(
            settings_dir(dir.path()).unwrap().as_deref(),
            Some("committed"),
            "read and tidied"
        );
    }

    #[test]
    fn init_uses_the_given_directory_then_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let given =
            FilesStore::for_init(dir.path().to_path_buf(), Some("docs/adr"), false).unwrap();
        assert_eq!(given.directory(), dir.path().join("docs/adr"), "given");
        let default = FilesStore::for_init(dir.path().to_path_buf(), None, false).unwrap();
        assert_eq!(default.directory(), dir.path().join(DEFAULT_DIR), "default");
    }

    #[test]
    fn an_empty_directory_in_the_settings_file_is_an_error_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "dir = \"\"\n").unwrap();
        let message = settings_dir(dir.path()).unwrap_err().to_string();
        assert!(
            message.contains(SETTINGS_FILE) && message.contains("must not be empty"),
            "{message}"
        );
    }

    #[test]
    fn a_settings_file_that_cannot_be_used_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "dirr = \"x\"\n").unwrap();
        let message = settings_dir(dir.path()).unwrap_err().to_string();
        assert!(
            message.contains(SETTINGS_FILE) && message.contains("dirr"),
            "{message}"
        );
    }

    #[test]
    fn existing_settings_are_kept_unless_the_store_is_told_to_replace_them() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(SETTINGS_FILE), "dir = \"old\"\n").unwrap();
        let kept = FilesStore::for_init(dir.path().to_path_buf(), Some("new"), false)
            .unwrap()
            .prepare()
            .unwrap();
        assert_eq!(kept.kept, [PathBuf::from(SETTINGS_FILE)], "kept");
        assert!(kept.replaced.is_empty(), "nothing replaced");
        assert_eq!(
            fs::read_to_string(dir.path().join(SETTINGS_FILE)).unwrap(),
            "dir = \"old\"\n",
            "unchanged"
        );
    }

    #[test]
    fn replacing_rewrites_the_settings_and_reports_it_but_leaves_the_template_alone() {
        let dir = tempfile::tempdir().unwrap();
        store(&dir, "old").prepare().unwrap();
        fs::write(dir.path().join("old/0000-template.md"), "# mine\n").unwrap();
        let prepared = FilesStore::for_init(dir.path().to_path_buf(), Some("new"), true)
            .unwrap()
            .prepare()
            .unwrap();
        assert_eq!(
            prepared.replaced,
            [PathBuf::from(SETTINGS_FILE)],
            "replaced"
        );
        assert_eq!(
            prepared.created,
            [PathBuf::from("new/0000-template.md")],
            "the new directory gets a template"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(SETTINGS_FILE)).unwrap(),
            "dir = \"new\"\n",
            "settings rewritten"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("old/0000-template.md")).unwrap(),
            "# mine\n",
            "the old directory and its template are left as they were"
        );
    }

    #[test]
    fn replacing_when_there_is_nothing_to_replace_just_creates() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = FilesStore::for_init(dir.path().to_path_buf(), Some("adr"), true)
            .unwrap()
            .prepare()
            .unwrap();
        assert!(prepared.replaced.is_empty(), "nothing was there");
        assert_eq!(prepared.created.len(), 2, "both created");
    }

    #[test]
    fn an_empty_answer_takes_the_default_directory() {
        assert_eq!(dir_from_answer("", DEFAULT_DIR), DEFAULT_DIR, "empty");
        assert_eq!(
            dir_from_answer("  \n", "docs/adr"),
            "docs/adr",
            "the default is whatever is given"
        );
        assert_eq!(
            dir_from_answer(" other\n", "docs/adr"),
            "other",
            "a typed answer, trimmed"
        );
    }

    #[test]
    fn a_bad_directory_is_caught_by_check_dir() {
        assert!(check_dir("docs/adr").is_ok(), "fine");
        assert!(check_dir("/srv/adr").is_ok(), "absolute is fine");
        for dir in ["", ".", "./"] {
            assert!(matches!(check_dir(dir), Err(Error::EmptyDir(_))), "{dir:?}");
        }
    }
}
