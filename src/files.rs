//! The file adapter (ADR 0003): ADRs as Markdown files in a directory.
//!
//! It owns everything physical: the directory, the `.decider.toml` that says
//! where it is, and the ADR template file.

use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::error::Error;
use crate::lifecycle::{Move, Status};
use crate::store::{Change, Prepared, WritesAdrs};
use crate::text;

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

/// The longest slug written into a filename, in characters.
const MAX_SLUG_CHARS: usize = 60;

/// The number at the start of a filename such as `0007-use-postgres.md`: four or
/// more digits, a hyphen, something, and `.md`.
fn number_of(file_name: &str) -> Option<u32> {
    let stem = file_name.strip_suffix(".md")?;
    let (digits, title) = stem.split_once('-')?;
    if digits.len() < 4 || title.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The title in lower case, keeping letters and digits and joining the words
/// with `-`, cut at [`MAX_SLUG_CHARS`]. `None` if nothing is left.
fn slug(title: &str) -> Option<String> {
    let mut slug = String::new();
    for character in title.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let cut: String = slug.chars().take(MAX_SLUG_CHARS).collect();
    let slug = cut.trim_matches('-');
    (!slug.is_empty()).then(|| slug.to_owned())
}

/// Check a title and return it trimmed: one line, not empty.
fn check_title(title: &str) -> Result<&str, Error> {
    let title = title.trim();
    if title.is_empty() || title.contains(['\n', '\r']) {
        return Err(Error::BadTitle);
    }
    Ok(title)
}

/// The sections of a template: everything from its first `## ` heading.
fn template_sections(template: &str) -> &str {
    let mut offset = 0;
    for line in template.split_inclusive('\n') {
        if line.starts_with("## ") {
            return &template[offset..];
        }
        offset += line.len();
    }
    ""
}

/// The header lines a change adds: the label on the line that records it.
fn label(movement: Move) -> &'static str {
    match movement {
        Move::Accept => "Accepted",
        Move::Reject => "Rejected",
        Move::Implement => "Implemented",
    }
}

/// ADRs stored as Markdown files under `root`.
#[derive(Debug)]
pub struct FilesStore {
    root: PathBuf,
    dir: String,
    replace_settings: bool,
}

impl FilesStore {
    /// The directory that holds `.decider.toml`, which the places a store
    /// reports are relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The ADR directory.
    fn directory(&self) -> PathBuf {
        self.root.join(&self.dir)
    }

    /// The ADR files in the directory, as `(number, path)`. Directories and files
    /// that are not named like an ADR are ignored, and so is the template.
    fn adrs(&self) -> Result<Vec<(u32, PathBuf)>, Error> {
        let directory = self.directory();
        let entries =
            fs::read_dir(&directory).map_err(|cause| Error::io("list", &directory, cause))?;
        let mut found = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|cause| Error::io("list", &directory, cause))?;
            let path = entry.path();
            let number = entry.file_name().to_str().and_then(number_of);
            if let (true, Some(number)) = (path.is_file(), number)
                && number != 0
            {
                found.push((number, path));
            }
        }
        Ok(found)
    }

    /// The path of ADR `number`, if exactly one file has it.
    fn find(&self, number: u32) -> Result<PathBuf, Error> {
        let mut matches: Vec<PathBuf> = self
            .adrs()?
            .into_iter()
            .filter(|(found, _)| *found == number)
            .map(|(_, path)| path)
            .collect();
        match matches.len() {
            0 => Err(Error::NoSuchAdr(number)),
            1 => Ok(matches.remove(0)),
            _ => {
                let names: Vec<String> = matches
                    .iter()
                    .filter_map(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .collect();
                Err(Error::AmbiguousAdr(number, names.join(", ")))
            }
        }
    }

    /// The path as shown to the user: relative to the root when below it.
    fn shown(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root).unwrap_or(path).to_path_buf()
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

    fn propose(&mut self, title: &str, date: &str, by: Option<&str>) -> Result<PathBuf, Error> {
        let title = check_title(title)?;
        let slug = slug(title).ok_or_else(|| Error::NoSlug(title.to_owned()))?;
        let highest = self.adrs()?.into_iter().map(|(number, _)| number).max();
        let number = highest
            .map_or(Some(1), |highest| highest.checked_add(1))
            .ok_or(Error::NoNumberLeft)?;

        let template_path = self.directory().join(TEMPLATE_FILE);
        let template = match fs::read_to_string(&template_path) {
            Ok(text) => text,
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => TEMPLATE.to_owned(),
            Err(cause) => return Err(Error::io("read", &template_path, cause)),
        };
        let proposed_line = text::stage_line("Proposed", date, by, &[]);
        let eol = text::line_ending(&template);
        let document = format!(
            "# {number:04}. {title}{eol}{eol}**Status:** {}{eol}{proposed_line}{eol}{eol}{}",
            Status::Proposed,
            template_sections(&template)
        );

        let path = self.directory().join(format!("{number:04}-{slug}.md"));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|cause| Error::io("create", &path, cause))?;
        file.write_all(document.as_bytes())
            .map_err(|cause| Error::io("write", &path, cause))?;
        Ok(self.shown(&path))
    }

    fn change(&mut self, number: u32, change: &Change) -> Result<(PathBuf, Status), Error> {
        if change.movement == Move::Reject
            && change
                .text
                .as_deref()
                .is_none_or(|text| text.trim().is_empty())
        {
            return Err(Error::NoReason);
        }
        let path = self.find(number)?;
        let original =
            fs::read_to_string(&path).map_err(|cause| Error::io("read", &path, cause))?;
        let no_status = || Error::NoStatus(path.clone());
        let current: Status = text::read_status(&original)
            .ok_or_else(no_status)?
            .parse()
            .map_err(|message| Error::BadStatus {
                path: path.clone(),
                message,
            })?;
        let status = current.after(change.movement).map_err(Error::NotAllowed)?;

        let mut edited = text::set_status(&original, status).ok_or_else(no_status)?;
        let line = text::stage_line(
            label(change.movement),
            &change.date,
            change.by.as_deref(),
            &change.references,
        );
        edited = text::add_header_line(&edited, &line);
        if let Some(body) = change
            .text
            .as_deref()
            .filter(|body| !body.trim().is_empty())
        {
            let heading = if change.movement == Move::Reject {
                "Rejection"
            } else {
                "Outcome"
            };
            edited = text::append_section(&edited, heading, body);
        }
        fs::write(&path, edited).map_err(|cause| Error::io("write", &path, cause))?;
        Ok((self.shown(&path), status))
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

    /// A prepared store in `adr`, for the lifecycle tests.
    fn ready(dir: &tempfile::TempDir) -> FilesStore {
        let mut store = store(dir, "adr");
        store.prepare().unwrap();
        store
    }

    fn accept() -> Change {
        Change {
            movement: Move::Accept,
            date: "2026-10-05".to_owned(),
            by: Some("Ada".to_owned()),
            references: Vec::new(),
            text: None,
        }
    }

    #[test]
    fn the_number_is_read_from_a_filename() {
        assert_eq!(number_of("0007-use-postgres.md"), Some(7), "ordinary");
        assert_eq!(number_of("12345-later.md"), Some(12345), "five digits");
        for name in [
            "README.md",
            "0007.md",
            "0007-.md",
            "7-short.md",
            "0007-x.txt",
            "abcd-x.md",
            "",
        ] {
            assert_eq!(number_of(name), None, "{name:?}");
        }
    }

    #[test]
    fn a_slug_is_lower_case_words_joined_by_hyphens() {
        assert_eq!(
            slug("Use Postgres").as_deref(),
            Some("use-postgres"),
            "ordinary"
        );
        assert_eq!(
            slug("  Use  Postgres -- (really!) ").as_deref(),
            Some("use-postgres-really"),
            "runs collapse"
        );
        assert_eq!(
            slug("Été 2026").as_deref(),
            Some("été-2026"),
            "other scripts"
        );
        assert_eq!(slug("?!").as_deref(), None, "nothing to name a file from");
    }

    #[test]
    fn a_long_title_is_cut_without_a_trailing_hyphen() {
        let title = format!("{} end", "word ".repeat(20));
        let slug = slug(&title).unwrap();
        assert!(
            slug.chars().count() <= MAX_SLUG_CHARS && !slug.ends_with('-'),
            "{slug}"
        );
    }

    #[test]
    fn a_proposed_adr_is_numbered_from_one_and_has_a_header_and_the_sections() {
        let dir = tempfile::tempdir().unwrap();
        let place = ready(&dir)
            .propose("Use Postgres", "2026-10-05", Some("Ada"))
            .unwrap();
        assert_eq!(
            place,
            PathBuf::from("adr/0001-use-postgres.md"),
            "the first number, and the place"
        );
        let text = fs::read_to_string(dir.path().join("adr/0001-use-postgres.md")).unwrap();
        assert!(
            text.starts_with("# 0001. Use Postgres\n\n**Status:** proposed\n**Proposed:** 2026-10-05 by Ada\n\n## Context\n"),
            "{text}"
        );
        assert!(
            text.contains("## Consequences"),
            "sections from the template: {text}"
        );
        assert!(
            !text.contains("NNNN") && !text.contains("YYYY"),
            "the template's own header is not copied: {text}"
        );
    }

    #[test]
    fn a_proposed_adr_takes_its_line_endings_from_a_windows_template() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        fs::write(
            dir.path().join("adr/0000-template.md"),
            "# NNNN. Title\r\n\r\n## Context\r\n\r\nWhy.\r\n",
        )
        .unwrap();
        store.propose("T", "2026-10-05", Some("Ada")).unwrap();
        let text = fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap();
        assert_eq!(
            text,
            "# 0001. T\r\n\r\n**Status:** proposed\r\n**Proposed:** 2026-10-05 by Ada\r\n\r\n## Context\r\n\r\nWhy.\r\n",
            "no bare LF anywhere"
        );
    }

    #[test]
    fn the_next_number_follows_the_highest_and_ignores_the_template_and_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        fs::write(dir.path().join("adr/0009-old.md"), "").unwrap();
        fs::write(dir.path().join("adr/README.md"), "").unwrap();
        fs::create_dir(dir.path().join("adr/0050-notes.md")).unwrap();
        assert_eq!(
            store.propose("Next", "2026-10-05", None).unwrap(),
            PathBuf::from("adr/0010-next.md"),
            "after 9"
        );
    }

    #[test]
    fn the_stores_own_template_is_used_and_the_built_in_one_when_it_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        fs::write(
            dir.path().join("adr/0000-template.md"),
            "# N\n\n**Status:** proposed\n\n## Why\n\nBecause.\n",
        )
        .unwrap();
        store.propose("Custom", "2026-10-05", None).unwrap();
        let custom = fs::read_to_string(dir.path().join("adr/0001-custom.md")).unwrap();
        assert!(
            custom.contains("## Why\n\nBecause.\n") && !custom.contains("## Context"),
            "{custom}"
        );
        fs::remove_file(dir.path().join("adr/0000-template.md")).unwrap();
        store.propose("Built in", "2026-10-05", None).unwrap();
        let built_in = fs::read_to_string(dir.path().join("adr/0002-built-in.md")).unwrap();
        assert!(built_in.contains("## Consequences"), "{built_in}");
    }

    #[test]
    fn a_bad_title_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        for title in ["", "   ", "a\nb"] {
            assert!(
                matches!(
                    store.propose(title, "2026-10-05", None),
                    Err(Error::BadTitle)
                ),
                "{title:?}"
            );
        }
        assert!(
            matches!(
                store.propose("?!", "2026-10-05", None),
                Err(Error::NoSlug(_))
            ),
            "no slug"
        );
        assert_eq!(
            fs::read_dir(dir.path().join("adr")).unwrap().count(),
            1,
            "only the template"
        );
    }

    #[test]
    fn proposing_without_a_name_leaves_out_the_by() {
        let dir = tempfile::tempdir().unwrap();
        ready(&dir).propose("T", "2026-10-05", None).unwrap();
        let text = fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap();
        assert!(text.contains("**Proposed:** 2026-10-05\n"), "{text}");
    }

    #[test]
    fn proposing_into_a_missing_directory_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let error = store(&dir, "gone")
            .propose("T", "2026-10-05", None)
            .unwrap_err();
        assert!(error.to_string().contains("gone"), "{error}");
    }

    #[test]
    fn the_last_possible_number_leaves_none() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        fs::write(dir.path().join("adr/4294967295-max.md"), "").unwrap();
        assert!(
            matches!(
                store.propose("T", "2026-10-05", None),
                Err(Error::NoNumberLeft)
            ),
            "none left"
        );
    }

    #[test]
    fn accepting_changes_the_status_and_adds_a_line_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        store
            .propose("Use Postgres", "2026-10-01", Some("Ada"))
            .unwrap();
        let before = fs::read_to_string(dir.path().join("adr/0001-use-postgres.md")).unwrap();
        let (place, status) = store.change(1, &accept()).unwrap();
        assert_eq!(
            (place, status),
            (PathBuf::from("adr/0001-use-postgres.md"), Status::Accepted),
            "result"
        );
        let after = fs::read_to_string(dir.path().join("adr/0001-use-postgres.md")).unwrap();
        assert_eq!(
            after,
            before
                .replace("**Status:** proposed", "**Status:** accepted")
                .replace(
                    "by Ada\n\n## Context",
                    "by Ada\n**Accepted:** 2026-10-05 by Ada\n\n## Context"
                ),
            "only the status and one new line differ"
        );
    }

    #[test]
    fn rejecting_records_the_reason_in_a_section() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        store.propose("T", "2026-10-01", Some("Ada")).unwrap();
        let change = Change {
            movement: Move::Reject,
            text: Some("Too costly.".to_owned()),
            ..accept()
        };
        store.change(1, &change).unwrap();
        let text = fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap();
        assert!(
            text.contains("**Status:** rejected\n")
                && text.contains("**Rejected:** 2026-10-05 by Ada\n"),
            "{text}"
        );
        assert!(
            text.ends_with("\n\n## Rejection\n\nToo costly.\n"),
            "{text}"
        );
    }

    #[test]
    fn implementing_records_references_and_an_optional_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        store.propose("T", "2026-10-01", Some("Ada")).unwrap();
        store.change(1, &accept()).unwrap();
        let change = Change {
            movement: Move::Implement,
            references: vec!["#12".to_owned(), "#13".to_owned()],
            text: Some("Shipped in two PRs.".to_owned()),
            ..accept()
        };
        store.change(1, &change).unwrap();
        let text = fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap();
        assert!(text.contains("**Status:** implemented\n"), "{text}");
        assert!(
            text.contains("**Implemented:** 2026-10-05 by Ada (#12, #13)\n"),
            "{text}"
        );
        assert!(
            text.ends_with("\n\n## Outcome\n\nShipped in two PRs.\n"),
            "{text}"
        );
    }

    #[test]
    fn implementing_without_a_note_adds_no_section() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        store.propose("T", "2026-10-01", None).unwrap();
        store.change(1, &accept()).unwrap();
        store
            .change(
                1,
                &Change {
                    movement: Move::Implement,
                    ..accept()
                },
            )
            .unwrap();
        let text = fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap();
        assert!(!text.contains("## Outcome"), "{text}");
    }

    #[test]
    fn a_move_the_lifecycle_does_not_allow_leaves_the_file_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        store.propose("T", "2026-10-01", None).unwrap();
        let before = fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap();
        let error = store
            .change(
                1,
                &Change {
                    movement: Move::Implement,
                    ..accept()
                },
            )
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "the ADR is proposed, and `implement` needs it to be accepted",
            "message"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap(),
            before,
            "untouched"
        );
    }

    #[test]
    fn an_adr_that_does_not_exist_is_reported_by_number() {
        let dir = tempfile::tempdir().unwrap();
        let error = ready(&dir).change(7, &accept()).unwrap_err();
        assert_eq!(error.to_string(), "there is no ADR number 7", "message");
    }

    #[test]
    fn two_files_with_one_number_are_reported_and_neither_is_changed() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        for name in ["0003-a.md", "0003-b.md"] {
            fs::write(
                dir.path().join("adr").join(name),
                "# 3. X\n\n**Status:** proposed\n",
            )
            .unwrap();
        }
        let error = store.change(3, &accept()).unwrap_err().to_string();
        assert!(
            error.contains("more than one ADR")
                && error.contains("0003-a.md")
                && error.contains("0003-b.md"),
            "{error}"
        );
        assert!(
            fs::read_to_string(dir.path().join("adr/0003-a.md"))
                .unwrap()
                .contains("proposed"),
            "untouched"
        );
    }

    #[test]
    fn the_template_cannot_be_changed_as_adr_zero() {
        let dir = tempfile::tempdir().unwrap();
        let error = ready(&dir).change(0, &accept()).unwrap_err();
        assert!(matches!(error, Error::NoSuchAdr(0)), "{error}");
    }

    #[test]
    fn an_adr_with_no_status_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        fs::write(dir.path().join("adr/0001-x.md"), "# 1. X\n\n## Context\n").unwrap();
        let error = store.change(1, &accept()).unwrap_err();
        assert!(
            matches!(&error, Error::NoStatus(path) if path.ends_with("0001-x.md")),
            "no status, naming the file: {error}"
        );
    }

    #[test]
    fn an_adr_with_an_unknown_status_is_reported_with_the_file_and_the_value() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        fs::write(
            dir.path().join("adr/0001-x.md"),
            "# 1. X\n\n**Status:** done\n",
        )
        .unwrap();
        let error = store.change(1, &accept()).unwrap_err().to_string();
        assert!(
            error.contains("0001-x.md") && error.contains("\"done\""),
            "{error}"
        );
    }

    #[test]
    fn a_rejection_without_a_reason_is_refused_and_the_file_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = ready(&dir);
        store.propose("T", "2026-10-05", None).unwrap();
        let before = fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap();
        for text in [None, Some("  ".to_owned())] {
            let change = Change {
                movement: Move::Reject,
                text: text.clone(),
                ..accept()
            };
            assert!(
                matches!(store.change(1, &change), Err(Error::NoReason)),
                "{text:?}"
            );
        }
        assert_eq!(
            fs::read_to_string(dir.path().join("adr/0001-t.md")).unwrap(),
            before,
            "untouched"
        );
    }
}
