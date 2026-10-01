//! The ADR store as a directory of Markdown files (ADR 0002, ADR 0003).
//!
//! The directory comes from `.decider.toml`. The filename, the slug, the file
//! format and the settings file itself are details of this adapter: the use
//! cases only see ADRs.

pub mod naming;
pub mod settings;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tracing::debug;

use crate::domain::adr::Adr;
use crate::domain::number::AdrNumber;
use crate::ports::{Location, Prepared, ReadsAdrs, StoreError, WritesAdrs};
use settings::Config;

/// The name of the template inside the ADR directory. Number 0 is reserved for it.
const TEMPLATE_FILE: &str = "0000-template.md";

fn error(action: &str, path: &Path, cause: &io::Error) -> StoreError {
    StoreError(format!("could not {action} {}: {cause}", path.display()))
}

/// Read a text file, ignoring a leading byte order mark.
fn read_text(path: &Path) -> io::Result<String> {
    let text = fs::read_to_string(path)?;
    Ok(match text.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_owned(),
        None => text,
    })
}

/// Read the settings file in exactly `directory`, without looking in its parents.
///
/// # Errors
///
/// Returns [`StoreError`] if the file exists but cannot be read or used.
fn read_settings(directory: &Path) -> Result<Option<Config>, StoreError> {
    let path = directory.join(settings::FILE_NAME);
    match read_text(&path) {
        Ok(text) => Config::parse(&text)
            .map(Some)
            .map_err(|cause| StoreError(format!("could not use {}: {cause}", path.display()))),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(cause) => Err(error("read", &path, &cause)),
    }
}

/// Whether a parent of `cwd` has a settings file that would be found from `cwd`,
/// without saying anything about it in the log.
fn parent_settings(cwd: &Path) -> Option<PathBuf> {
    cwd.parent()?
        .ancestors()
        .find(|directory| directory.join(settings::FILE_NAME).is_file())
        .map(Path::to_path_buf)
}

/// Look for `.decider.toml` in `start` and each parent directory in turn.
///
/// Returns the directory that holds it and its settings, or `None` if no
/// directory on the way up has one.
///
/// # Errors
///
/// Returns [`StoreError`] if a file is found but cannot be read or used.
pub fn find_settings(start: &Path) -> Result<Option<(PathBuf, Config)>, StoreError> {
    for directory in start.ancestors() {
        if let Some(config) = read_settings(directory)? {
            debug!(directory = %directory.display(), "found the settings file");
            return Ok(Some((directory.to_path_buf(), config)));
        }
    }
    Ok(None)
}

/// The settings `init` will use, and anything the user should be told about.
#[derive(Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The settings to prepare the store with.
    pub config: Config,
    /// Messages for the user, such as a `--dir` that was ignored.
    pub warnings: Vec<String>,
}

/// Work out the settings for `init` run in `cwd`.
///
/// Only `cwd` itself is looked at for an existing settings file, because `init`
/// sets up this directory. An existing file is used as it is, and a different
/// `dir` is ignored with a warning. Otherwise the settings come from `dir`, or
/// the default. A settings file in a parent directory is not used, but the user
/// is told about it, because `propose` run below this directory would find the
/// nearer one.
///
/// # Errors
///
/// Returns [`StoreError`] if a settings file in `cwd` cannot be read or used, or
/// `dir` is not a usable directory.
pub fn resolve_settings(cwd: &Path, dir: Option<&str>) -> Result<Resolved, StoreError> {
    let mut warnings = Vec::new();
    if let Some(config) = read_settings(cwd)? {
        debug!(directory = %cwd.display(), "using the settings file here");
        if let Some(dir) = dir
            && Config::new(dir).ok().as_ref() != Some(&config)
        {
            warnings.push(format!(
                "ignoring --dir {dir}: {} already sets the ADR directory",
                cwd.join(settings::FILE_NAME).display()
            ));
        }
        return Ok(Resolved { config, warnings });
    }
    if let Some(found) = parent_settings(cwd) {
        warnings.push(format!(
            "{} already exists in {}, so this directory will have its own settings and ADRs",
            settings::FILE_NAME,
            found.display()
        ));
    }
    let config = Config::new(dir.unwrap_or(settings::DEFAULT_DIR))
        .map_err(|cause| StoreError(format!("the ADR directory cannot be used: {cause}")))?;
    Ok(Resolved { config, warnings })
}

/// Write `text` to a file that must not exist yet. It also refuses to follow a
/// symbolic link that is already in that place.
fn write_new(path: &Path, text: &str) -> Result<(), StoreError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|cause| error("create", path, &cause))?;
    file.write_all(text.as_bytes())
        .map_err(|cause| error("write", path, &cause))
}

/// What is at `path`, without following a symbolic link.
enum Found {
    Nothing,
    RegularFile,
    Other,
}

fn look_at(path: &Path) -> Result<Found, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(Found::RegularFile),
        Ok(_) => Ok(Found::Other),
        Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(Found::Nothing),
        Err(cause) => Err(error("check", path, &cause)),
    }
}

/// Create `path` with `text` if nothing is there, keep it if it is a regular
/// file, and refuse anything else (a directory, or a symbolic link).
fn create_or_keep(
    path: &Path,
    text: &str,
    shown: Location,
    into: &mut Prepared,
) -> Result<(), StoreError> {
    match look_at(path)? {
        Found::Nothing => {
            write_new(path, text)?;
            into.created.push(shown);
            Ok(())
        }
        Found::RegularFile => {
            into.kept.push(shown);
            Ok(())
        }
        Found::Other => Err(StoreError(format!(
            "{} exists but is not a regular file",
            path.display()
        ))),
    }
}

/// How a path is shown to the user: relative to `base`, with `..` where the path
/// is not below it, so it can be pasted into a shell started in `base`.
///
/// Falls back to the full path if the two share no leading component.
fn shown(base: &Path, path: &Path) -> String {
    if path.components().next() != base.components().next() {
        return path.display().to_string();
    }
    let mut base_parts = base.components().peekable();
    let mut path_parts = path.components().peekable();
    while let (Some(left), Some(right)) = (base_parts.peek(), path_parts.peek()) {
        if left != right {
            break;
        }
        base_parts.next();
        path_parts.next();
    }
    let mut relative = PathBuf::new();
    for _ in base_parts {
        relative.push("..");
    }
    relative.extend(path_parts);
    if relative.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        relative.display().to_string()
    }
}

/// ADRs stored as Markdown files in a directory.
#[derive(Debug)]
pub struct FilesStore {
    /// The directory that holds `.decider.toml`.
    root: PathBuf,
    /// The settings read from it, or chosen by `init`.
    config: Config,
    /// Where paths are shown relative to.
    display_base: PathBuf,
}

impl FilesStore {
    fn location(&self, path: &Path) -> Location {
        Location(shown(&self.display_base, path))
    }

    /// Make sure the ADR directory, after following any symbolic links, is
    /// inside the repository root, and return it.
    ///
    /// The check uses the deepest part of the path that already exists, so it
    /// also works before the directory has been created.
    fn checked_directory(&self) -> Result<PathBuf, StoreError> {
        let directory = self.root.join(self.config.dir());
        let root =
            fs::canonicalize(&self.root).map_err(|cause| error("resolve", &self.root, &cause))?;
        let mut probe = directory.clone();
        loop {
            match fs::canonicalize(&probe) {
                Ok(real) if real.starts_with(&root) => return Ok(directory),
                Ok(real) => {
                    return Err(StoreError(format!(
                        "the ADR directory {} leads to {}, which is outside the repository; \
                         a symbolic link on the way points out of it",
                        directory.display(),
                        real.display()
                    )));
                }
                Err(cause) if cause.kind() == io::ErrorKind::NotFound => {
                    if !probe.pop() {
                        return Err(error("resolve", &directory, &cause));
                    }
                }
                Err(cause) => return Err(error("resolve", &probe, &cause)),
            }
        }
    }

    /// A store for the settings `config`, found at `root`.
    ///
    /// Paths in results are shown relative to `display_base`.
    pub fn new(root: PathBuf, config: Config, display_base: PathBuf) -> Self {
        debug!(root = %root.display(), dir = %config.dir().display(), "using the ADR directory");
        Self {
            root,
            config,
            display_base,
        }
    }
}

impl ReadsAdrs for FilesStore {
    fn numbers(&self) -> Result<Vec<AdrNumber>, StoreError> {
        let directory = self.checked_directory()?;
        if !directory.is_dir() {
            return Err(StoreError(format!(
                "the ADR directory {} does not exist; run `decider init` to create it",
                directory.display()
            )));
        }
        let entries =
            fs::read_dir(&directory).map_err(|cause| error("list", &directory, &cause))?;
        let mut numbers = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|cause| error("list", &directory, &cause))?;
            let is_file = entry.path().is_file();
            let number = entry
                .file_name()
                .to_str()
                .and_then(AdrNumber::from_filename);
            if let (true, Some(number)) = (is_file, number)
                && !number.is_template()
            {
                numbers.push(number);
            }
        }
        Ok(numbers)
    }

    fn template(&self) -> Result<Option<String>, StoreError> {
        let path = self.checked_directory()?.join(TEMPLATE_FILE);
        match look_at(&path)? {
            Found::Nothing => return Ok(None),
            Found::RegularFile => {}
            Found::Other => {
                return Err(StoreError(format!(
                    "{} exists but is not a regular file",
                    path.display()
                )));
            }
        }
        match read_text(&path) {
            Ok(text) => Ok(Some(text)),
            Err(cause) if cause.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(cause) => Err(error("read", &path, &cause)),
        }
    }
}

impl WritesAdrs for FilesStore {
    fn prepare(&mut self) -> Result<Prepared, StoreError> {
        let directory = self.checked_directory()?;
        fs::create_dir_all(&directory).map_err(|cause| error("create", &directory, &cause))?;
        // Checked again now that it exists, in case the path changed in between.
        self.checked_directory()?;

        let mut prepared = Prepared::default();
        let template = directory.join(TEMPLATE_FILE);
        let template_place = self.location(&template);
        create_or_keep(
            &template,
            crate::domain::template::TEXT,
            template_place,
            &mut prepared,
        )?;

        // Written last, so a run that failed part way can be repeated. It is
        // listed first because it is the most important file.
        let settings_path = self.root.join(settings::FILE_NAME);
        let mut settings_result = Prepared::default();
        let settings_place = self.location(&settings_path);
        create_or_keep(
            &settings_path,
            &self.config.render(),
            settings_place,
            &mut settings_result,
        )?;
        prepared.created.splice(0..0, settings_result.created);
        prepared.kept.splice(0..0, settings_result.kept);
        Ok(prepared)
    }

    fn create(&mut self, adr: &Adr) -> Result<Location, StoreError> {
        let text = adr
            .render()
            .map_err(|cause| StoreError(format!("the ADR could not be written: {cause}")))?;
        let name = naming::file_name(adr.number, &adr.title)
            .map_err(|cause| StoreError(cause.to_string()))?;
        let path = self.checked_directory()?.join(name);
        write_new(&path, &text)?;
        Ok(self.location(&path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::number::AdrNumber;
    use crate::domain::propose;
    use crate::domain::template;

    fn store_in(root: &tempfile::TempDir, dir: &str) -> FilesStore {
        FilesStore::new(
            root.path().to_path_buf(),
            Config::new(dir).unwrap(),
            root.path().to_path_buf(),
        )
    }

    fn adr(number: u32, title: &str) -> Adr {
        propose::build(
            AdrNumber::new(number),
            title,
            None,
            "2026-10-02".parse().unwrap(),
            template::TEXT,
        )
        .unwrap()
    }

    #[test]
    fn write_new_refuses_to_overwrite_a_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("file.md");
        fs::write(&path, "original").unwrap();
        let error = write_new(&path, "replacement").unwrap_err();
        assert!(error.to_string().contains("could not create"), "{error}");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "original",
            "left untouched"
        );
    }

    #[test]
    fn read_text_drops_a_byte_order_mark() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("file.toml");
        fs::write(&path, "\u{feff}dir = \"docs\"\n").unwrap();
        assert_eq!(
            read_text(&path).unwrap(),
            "dir = \"docs\"\n",
            "mark removed"
        );
    }

    #[test]
    fn settings_are_found_in_parent_directories() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join(settings::FILE_NAME),
            "dir = \"docs/adr\"\n",
        )
        .unwrap();
        let deep = root.path().join("a/b/c");
        fs::create_dir_all(&deep).unwrap();
        let (found, config) = find_settings(&deep).unwrap().unwrap();
        assert_eq!(found, root.path(), "the directory holding the file");
        assert_eq!(config.dir(), Path::new("docs/adr"), "its directory setting");
    }

    #[test]
    fn the_nearest_settings_file_wins() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(settings::FILE_NAME), "dir = \"outer\"\n").unwrap();
        let inner = root.path().join("inner");
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join(settings::FILE_NAME), "dir = \"nearer\"\n").unwrap();
        let (found, config) = find_settings(&inner).unwrap().unwrap();
        assert_eq!(found, inner, "nearest directory");
        assert_eq!(config.dir(), Path::new("nearer"), "nearest setting");
    }

    #[test]
    fn no_settings_file_anywhere_is_not_an_error() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            find_settings(root.path()).unwrap().is_none(),
            "nothing found"
        );
    }

    #[test]
    fn a_settings_file_that_cannot_be_used_is_reported() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(settings::FILE_NAME), "dir = \"../out\"\n").unwrap();
        let message = find_settings(root.path()).unwrap_err().to_string();
        assert!(
            message.contains(settings::FILE_NAME) && message.contains(".."),
            "{message}"
        );
    }

    #[test]
    fn a_directory_in_place_of_the_settings_file_is_reported() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(settings::FILE_NAME)).unwrap();
        assert!(
            find_settings(root.path())
                .unwrap_err()
                .to_string()
                .contains("could not read"),
            "reported"
        );
    }

    #[test]
    fn preparing_creates_the_settings_the_directory_and_the_template() {
        let root = tempfile::tempdir().unwrap();
        let mut store = store_in(&root, "docs/adr");
        let prepared = store.prepare().unwrap();
        assert_eq!(
            prepared.created,
            [
                Location(".decider.toml".to_owned()),
                Location("docs/adr/0000-template.md".to_owned())
            ],
            "the settings are listed first"
        );
        assert!(prepared.kept.is_empty(), "nothing kept");
        assert_eq!(
            fs::read_to_string(root.path().join(".decider.toml")).unwrap(),
            "dir = \"docs/adr\"\n",
            "settings"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("docs/adr/0000-template.md")).unwrap(),
            template::TEXT,
            "built-in template"
        );
    }

    #[test]
    fn preparing_keeps_an_existing_template_and_settings() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("adr")).unwrap();
        fs::write(root.path().join("adr/0000-template.md"), "# mine\n").unwrap();
        fs::write(
            root.path().join(".decider.toml"),
            "dir = \"adr\"\n# keep me\n",
        )
        .unwrap();
        let mut store = store_in(&root, "adr");
        let prepared = store.prepare().unwrap();
        assert!(prepared.created.is_empty(), "nothing created");
        assert_eq!(prepared.kept.len(), 2, "both kept");
        assert_eq!(
            fs::read_to_string(root.path().join("adr/0000-template.md")).unwrap(),
            "# mine\n",
            "template unchanged"
        );
        assert_eq!(
            fs::read_to_string(root.path().join(".decider.toml")).unwrap(),
            "dir = \"adr\"\n# keep me\n",
            "settings unchanged"
        );
    }

    #[test]
    fn preparing_twice_changes_nothing_the_second_time() {
        let root = tempfile::tempdir().unwrap();
        let mut store = store_in(&root, "adr");
        store.prepare().unwrap();
        let second = store.prepare().unwrap();
        assert!(second.created.is_empty(), "nothing created again");
        assert_eq!(second.kept.len(), 2, "both kept");
    }

    #[test]
    fn numbers_list_only_adr_files_and_not_the_template() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("adr")).unwrap();
        for name in [
            "0003-a.md",
            "0007-b.md",
            "0000-template.md",
            "README.md",
            "notes.txt",
        ] {
            fs::write(root.path().join("adr").join(name), "").unwrap();
        }
        let mut numbers = store_in(&root, "adr").numbers().unwrap();
        numbers.sort();
        assert_eq!(
            numbers,
            [3, 7].map(AdrNumber::new),
            "the template is not listed, as the port says"
        );
    }

    #[test]
    fn a_missing_directory_is_reported_with_what_to_do() {
        let root = tempfile::tempdir().unwrap();
        let message = store_in(&root, "gone").numbers().unwrap_err().to_string();
        assert!(
            message.contains("gone") && message.contains("decider init"),
            "{message}"
        );
    }

    #[test]
    fn creating_writes_a_named_file_and_returns_its_location() {
        let root = tempfile::tempdir().unwrap();
        let mut store = store_in(&root, "adr");
        store.prepare().unwrap();
        let location = store.create(&adr(1, "Use Postgres")).unwrap();
        assert_eq!(
            location,
            Location("adr/0001-use-postgres.md".to_owned()),
            "location"
        );
        let text = fs::read_to_string(root.path().join("adr/0001-use-postgres.md")).unwrap();
        assert!(text.starts_with("# 0001. Use Postgres\n"), "{text}");
    }

    #[test]
    fn creating_never_overwrites_an_existing_file() {
        let root = tempfile::tempdir().unwrap();
        let mut store = store_in(&root, "adr");
        store.prepare().unwrap();
        store.create(&adr(1, "Same")).unwrap();
        let before = fs::read_to_string(root.path().join("adr/0001-same.md")).unwrap();
        let error = store.create(&adr(1, "Same")).unwrap_err();
        assert!(error.to_string().contains("could not create"), "{error}");
        assert_eq!(
            fs::read_to_string(root.path().join("adr/0001-same.md")).unwrap(),
            before,
            "untouched"
        );
    }

    #[test]
    fn a_title_with_nothing_to_name_a_file_from_is_reported() {
        let root = tempfile::tempdir().unwrap();
        let mut store = store_in(&root, "adr");
        store.prepare().unwrap();
        let error = store.create(&adr(1, "?!")).unwrap_err();
        assert!(error.to_string().contains("letters or digits"), "{error}");
    }

    #[test]
    fn an_absent_template_is_none() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("adr")).unwrap();
        assert_eq!(store_in(&root, "adr").template().unwrap(), None, "absent");
    }

    #[test]
    fn a_present_template_is_read() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("adr")).unwrap();
        fs::write(root.path().join("adr/0000-template.md"), "# t\n").unwrap();
        assert_eq!(
            store_in(&root, "adr").template().unwrap(),
            Some("# t\n".to_owned()),
            "present"
        );
    }

    #[test]
    fn a_path_below_the_base_is_shown_relative() {
        assert_eq!(
            shown(Path::new("/repo"), Path::new("/repo/docs/a.md")),
            "docs/a.md",
            "below"
        );
    }

    #[test]
    fn a_path_elsewhere_in_the_tree_is_shown_with_parent_parts() {
        assert_eq!(
            shown(Path::new("/repo/src/deep"), Path::new("/repo/docs/a.md")),
            "../../docs/a.md",
            "up two, then down"
        );
    }

    #[test]
    fn the_base_itself_is_shown_as_a_dot() {
        assert_eq!(
            shown(Path::new("/repo"), Path::new("/repo")),
            ".",
            "same directory"
        );
    }

    #[test]
    fn a_path_in_a_parent_is_shown_with_one_parent_part() {
        assert_eq!(
            shown(Path::new("/repo/sub"), Path::new("/repo/a.md")),
            "../a.md",
            "one up"
        );
    }

    #[test]
    fn a_relative_path_and_an_absolute_base_are_shown_in_full() {
        assert_eq!(
            shown(Path::new("/repo"), Path::new("docs/a.md")),
            "docs/a.md",
            "nothing in common"
        );
    }

    #[test]
    fn a_directory_whose_name_looks_like_an_adr_is_not_counted() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("adr/0050-notes.md")).unwrap();
        fs::write(root.path().join("adr/0002-real.md"), "").unwrap();
        assert_eq!(
            store_in(&root, "adr").numbers().unwrap(),
            [AdrNumber::new(2)],
            "only files count"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_out_of_the_repository_is_refused_for_every_operation() {
        let outside = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        let mut store = store_in(&root, "link/adr");
        for message in [
            store.prepare().unwrap_err().to_string(),
            store.numbers().unwrap_err().to_string(),
            store.template().unwrap_err().to_string(),
            store.create(&adr(1, "Escapes")).unwrap_err().to_string(),
        ] {
            assert!(message.contains("outside the repository"), "{message}");
        }
        assert!(
            fs::read_dir(outside.path()).unwrap().next().is_none(),
            "nothing was written outside"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_that_stays_inside_the_repository_is_fine() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("real")).unwrap();
        std::os::unix::fs::symlink(root.path().join("real"), root.path().join("alias")).unwrap();
        let mut store = store_in(&root, "alias/adr");
        assert!(store.prepare().is_ok(), "inside is allowed");
        assert!(
            root.path().join("real/adr/0000-template.md").is_file(),
            "written through the link"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_symbolic_link_in_place_of_the_settings_file_is_not_followed() {
        let outside = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let target = outside.path().join("written-outside.toml");
        std::os::unix::fs::symlink(&target, root.path().join(".decider.toml")).unwrap();
        let error = store_in(&root, "adr").prepare().unwrap_err();
        assert!(error.to_string().contains("not a regular file"), "{error}");
        assert!(!target.exists(), "nothing was written through the link");
    }

    #[cfg(unix)]
    #[test]
    fn a_template_that_is_a_symbolic_link_is_not_read() {
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.md"), "# S\n\n## Secret\n").unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("adr")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.md"),
            root.path().join("adr/0000-template.md"),
        )
        .unwrap();
        let error = store_in(&root, "adr").template().unwrap_err();
        assert!(error.to_string().contains("not a regular file"), "{error}");
    }

    #[test]
    fn a_directory_in_place_of_the_template_is_refused() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("adr/0000-template.md")).unwrap();
        let error = store_in(&root, "adr").prepare().unwrap_err();
        assert!(error.to_string().contains("not a regular file"), "{error}");
        assert!(
            !root.path().join(".decider.toml").exists(),
            "settings are written last"
        );
    }

    #[test]
    fn resolving_uses_the_default_directory_when_nothing_is_given() {
        let root = tempfile::tempdir().unwrap();
        let resolved = resolve_settings(root.path(), None).unwrap();
        assert_eq!(
            resolved.config.dir(),
            Path::new(settings::DEFAULT_DIR),
            "default"
        );
        assert!(resolved.warnings.is_empty(), "no warnings");
    }

    #[test]
    fn resolving_uses_the_given_directory() {
        let root = tempfile::tempdir().unwrap();
        let resolved = resolve_settings(root.path(), Some("docs/adr")).unwrap();
        assert_eq!(resolved.config.dir(), Path::new("docs/adr"), "given");
    }

    #[test]
    fn resolving_uses_an_existing_file_in_the_current_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".decider.toml"), "dir = \"committed\"\n").unwrap();
        let resolved = resolve_settings(root.path(), None).unwrap();
        assert_eq!(
            resolved.config.dir(),
            Path::new("committed"),
            "from the file"
        );
        assert!(resolved.warnings.is_empty(), "no warnings");
    }

    #[test]
    fn resolving_warns_when_dir_disagrees_with_the_file_but_not_when_it_agrees() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".decider.toml"), "dir = \"committed\"\n").unwrap();
        let differs = resolve_settings(root.path(), Some("other")).unwrap();
        assert_eq!(differs.config.dir(), Path::new("committed"), "file wins");
        assert!(
            differs.warnings[0].contains("ignoring --dir other"),
            "{:?}",
            differs.warnings
        );
        let same = resolve_settings(root.path(), Some("./committed/")).unwrap();
        assert!(
            same.warnings.is_empty(),
            "same directory spelled differently"
        );
    }

    #[test]
    fn resolving_ignores_a_broken_file_in_a_parent_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".decider.toml"), "dirr = \"x\"\n").unwrap();
        let child = root.path().join("child");
        fs::create_dir(&child).unwrap();
        let resolved = resolve_settings(&child, None).unwrap();
        assert_eq!(
            resolved.config.dir(),
            Path::new(settings::DEFAULT_DIR),
            "fresh settings"
        );
    }

    #[test]
    fn resolving_warns_when_a_parent_already_has_settings() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".decider.toml"), "dir = \"docs\"\n").unwrap();
        let child = root.path().join("child");
        fs::create_dir(&child).unwrap();
        let resolved = resolve_settings(&child, None).unwrap();
        assert!(
            resolved.warnings[0].contains("already exists in"),
            "{:?}",
            resolved.warnings
        );
    }

    #[test]
    fn resolving_reports_a_broken_file_in_the_current_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".decider.toml"), "dirr = \"x\"\n").unwrap();
        assert!(
            resolve_settings(root.path(), None)
                .unwrap_err()
                .to_string()
                .contains("dirr"),
            "reported"
        );
    }

    #[test]
    fn resolving_refuses_a_reserved_directory() {
        let root = tempfile::tempdir().unwrap();
        let error = resolve_settings(root.path(), Some(".decider.toml")).unwrap_err();
        assert!(error.to_string().contains("must not start with"), "{error}");
    }
}
