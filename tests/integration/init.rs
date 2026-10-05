//! Integration tests for `decider init`, run against the real binary.
//!
//! Every run gets its own repository directory and its own home and
//! configuration directories, so no test can touch the developer's real settings.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

struct Sandbox {
    repo: TempDir,
    home: TempDir,
    config: TempDir,
}

impl Sandbox {
    fn new() -> io::Result<Self> {
        Ok(Self {
            repo: tempfile::tempdir()?,
            home: tempfile::tempdir()?,
            config: tempfile::tempdir()?,
        })
    }

    /// Run `decider` in `dir`. Input is closed, so the name cannot be asked for.
    fn run_in(&self, dir: &Path, arguments: &[&str]) -> io::Result<Output> {
        Command::new(env!("CARGO_BIN_EXE_decider"))
            .args(arguments)
            .current_dir(dir)
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("USER", "youcef.kadri")
            .stdin(Stdio::null())
            .output()
    }

    fn run(&self, arguments: &[&str]) -> io::Result<Output> {
        self.run_in(self.repo.path(), arguments)
    }

    fn read(&self, path: &str) -> io::Result<String> {
        fs::read_to_string(self.repo.path().join(path))
    }

    fn user_file(&self) -> PathBuf {
        self.config.path().join("decider/config.toml")
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const DIR: &str = "docs/explanations/decisions";

#[test]
fn init_makes_the_settings_the_directory_the_template_and_records_the_name() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init", "--user", "Ada Lovelace"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml")?,
        format!("dir = \"{DIR}\"\n"),
        "repository settings"
    );
    assert!(
        sandbox
            .read(&format!("{DIR}/0000-template.md"))?
            .contains("## Consequences"),
        "template"
    );
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Ada Lovelace\"\n",
        "user settings"
    );
    assert_eq!(
        stdout(&out),
        format!(
            "created .decider.toml\ncreated {DIR}/0000-template.md\nrecorded your name as Ada Lovelace\n"
        ),
        "exact output"
    );
    assert!(stderr(&out).is_empty(), "stderr: {}", stderr(&out));
    Ok(())
}

#[test]
fn the_name_is_kept_out_of_the_repository() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    sandbox.run(&["init", "--user", "Ada Lovelace"])?;
    assert!(
        !sandbox.read(".decider.toml")?.contains("Ada"),
        "the shared settings hold no name"
    );
    Ok(())
}

#[test]
fn a_different_directory_can_be_chosen() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init", "--dir", "docs/adr", "--user", "Ada"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml")?,
        "dir = \"docs/adr\"\n",
        "settings"
    );
    assert!(
        sandbox
            .repo
            .path()
            .join("docs/adr/0000-template.md")
            .is_file(),
        "template"
    );
    Ok(())
}

#[test]
fn an_empty_directory_is_refused_and_nothing_is_written() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    for dir in ["", "."] {
        let out = sandbox.run(&["init", "--dir", dir, "--user", "Ada"])?;
        assert!(!out.status.success(), "{dir:?} should fail");
        assert!(
            stderr(&out).contains("must not be empty"),
            "{}",
            stderr(&out)
        );
    }
    assert!(
        !sandbox.repo.path().join(".decider.toml").exists(),
        "nothing written"
    );
    Ok(())
}

#[test]
fn a_name_with_parentheses_is_fine() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init", "--user", "Jane Doe (Contractor)"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        fs::read_to_string(sandbox.user_file())?.contains("Jane Doe (Contractor)"),
        "recorded as given"
    );
    Ok(())
}

#[test]
fn a_name_with_a_line_break_is_refused() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init", "--user", "Ada\nLovelace"])?;
    assert!(!out.status.success(), "would split the header");
    assert!(stderr(&out).contains("--user"), "{}", stderr(&out));
    assert!(
        !sandbox.repo.path().join(".decider.toml").exists()
            && !sandbox.repo.path().join("docs").exists(),
        "a refused name leaves nothing behind"
    );
    Ok(())
}

#[test]
fn without_a_terminal_or_a_name_init_still_works_and_says_no_name_was_recorded() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init"])?;
    assert!(
        out.status.success(),
        "it must not hang or fail: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("no name recorded"),
        "{}",
        stderr(&out)
    );
    assert!(!sandbox.user_file().exists(), "nothing written");
    Ok(())
}

#[test]
fn running_init_again_changes_nothing() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    sandbox.run(&["init", "--user", "Ada"])?;
    let second = sandbox.run(&["init", "--user", "Grace"])?;
    assert!(second.status.success(), "{}", stderr(&second));
    assert_eq!(
        stdout(&second),
        format!(
            "kept .decider.toml (already exists)\nkept {DIR}/0000-template.md (already exists)\nkept your name as Ada\n"
        ),
        "everything kept, including the first name"
    );
    Ok(())
}

#[test]
fn a_fresh_clone_with_committed_settings_only_records_the_name() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    fs::write(
        sandbox.repo.path().join(".decider.toml"),
        "dir = \"docs/adr\"\n",
    )?;
    let out = sandbox.run(&["init", "--dir", "ignored", "--user", "Ada"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml")?,
        "dir = \"docs/adr\"\n",
        "committed file kept"
    );
    assert!(
        sandbox
            .repo
            .path()
            .join("docs/adr/0000-template.md")
            .is_file(),
        "its directory used"
    );
    assert!(
        !sandbox.repo.path().join("ignored").exists(),
        "--dir did not win"
    );
    assert!(sandbox.user_file().is_file(), "name recorded");
    Ok(())
}

#[test]
fn an_edited_template_is_left_alone() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    sandbox.run(&["init", "--user", "Ada"])?;
    fs::write(
        sandbox.repo.path().join(DIR).join("0000-template.md"),
        "# mine\n",
    )?;
    sandbox.run(&["init"])?;
    assert_eq!(
        sandbox.read(&format!("{DIR}/0000-template.md"))?,
        "# mine\n",
        "unchanged"
    );
    Ok(())
}

#[test]
fn adding_a_name_keeps_what_the_user_file_already_holds() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    fs::create_dir_all(sandbox.config.path().join("decider"))?;
    fs::write(sandbox.user_file(), "# my settings\n")?;
    let out = sandbox.run(&["init", "--user", "Ada"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Ada\"\n# my settings\n",
        "comment kept after the name"
    );
    Ok(())
}

#[test]
fn the_user_file_goes_under_home_when_xdg_config_home_is_not_set() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = Command::new(env!("CARGO_BIN_EXE_decider"))
        .args(["init", "--user", "Ada"])
        .current_dir(sandbox.repo.path())
        .env("HOME", sandbox.home.path())
        .env_remove("XDG_CONFIG_HOME")
        .stdin(Stdio::null())
        .output()?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        sandbox
            .home
            .path()
            .join(".config/decider/config.toml")
            .is_file(),
        "under HOME"
    );
    Ok(())
}

#[test]
fn with_nowhere_to_keep_the_name_it_says_so() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = Command::new(env!("CARGO_BIN_EXE_decider"))
        .args(["init", "--user", "Ada"])
        .current_dir(sandbox.repo.path())
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .stdin(Stdio::null())
        .output()?;
    assert!(!out.status.success(), "cannot record a name");
    assert!(
        stderr(&out).contains("set HOME or XDG_CONFIG_HOME"),
        "{}",
        stderr(&out)
    );
    assert!(
        !sandbox.repo.path().join(".decider.toml").exists(),
        "nowhere to keep the name leaves nothing behind"
    );
    Ok(())
}

#[test]
fn a_settings_file_that_cannot_be_used_is_reported_by_name() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    fs::write(sandbox.repo.path().join(".decider.toml"), "dirr = \"x\"\n")?;
    let out = sandbox.run(&["init", "--user", "Ada"])?;
    assert!(!out.status.success(), "unknown key");
    assert!(
        stderr(&out).contains(".decider.toml") && stderr(&out).contains("dirr"),
        "{}",
        stderr(&out)
    );
    Ok(())
}

#[test]
fn a_blank_name_already_in_the_user_file_does_not_block_user() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    fs::create_dir_all(sandbox.config.path().join("decider"))?;
    fs::write(sandbox.user_file(), "name = \"\"\n")?;
    let out = sandbox.run(&["init", "--user", "Grace"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("recorded your name as Grace"),
        "{}",
        stdout(&out)
    );
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Grace\"\n",
        "the blank name is replaced, not kept beside the new one"
    );
    let again = sandbox.run(&["init"])?;
    assert!(
        again.status.success(),
        "a second run must still work: {}",
        stderr(&again)
    );
    assert!(
        stdout(&again).contains("kept your name as Grace"),
        "{}",
        stdout(&again)
    );
    Ok(())
}

#[test]
fn an_empty_directory_in_the_committed_settings_names_the_file() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    fs::write(sandbox.repo.path().join(".decider.toml"), "dir = \"\"\n")?;
    let out = sandbox.run(&["init", "--user", "Ada"])?;
    assert!(!out.status.success(), "empty directory");
    assert!(
        stderr(&out).contains(".decider.toml") && stderr(&out).contains("must not be empty"),
        "{}",
        stderr(&out)
    );
    Ok(())
}

#[test]
fn an_absolute_directory_outside_the_repository_is_used() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let outside = tempfile::tempdir()?;
    let adrs = outside.path().join("project").join("adrs");
    let adrs_text = adrs.to_string_lossy().into_owned();
    let out = sandbox.run(&["init", "--dir", &adrs_text, "--user", "Ada"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        adrs.join("0000-template.md").is_file(),
        "template written there"
    );
    assert_eq!(
        sandbox.read(".decider.toml")?,
        format!("dir = \"{adrs_text}\"\n"),
        "the absolute path is recorded as given"
    );
    assert!(
        stdout(&out).contains(&format!("created {adrs_text}/0000-template.md")),
        "shown in full: {}",
        stdout(&out)
    );
    assert!(
        !sandbox.repo.path().join("docs").exists(),
        "nothing else under the repository"
    );
    Ok(())
}

#[test]
fn a_quoted_tilde_is_not_expanded_and_is_a_relative_path() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init", "--dir", "~/adrs", "--user", "Ada"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        sandbox
            .repo
            .path()
            .join("~/adrs/0000-template.md")
            .is_file(),
        "a directory really called ~ under the repository"
    );
    Ok(())
}

/// Set up a repository in `docs/adr` for a user called Ada.
fn initialised(sandbox: &Sandbox) -> io::Result<()> {
    let out = sandbox.run(&["init", "--dir", "docs/adr", "--user", "Ada"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    Ok(())
}

#[test]
fn force_with_a_name_replaces_only_the_name() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    initialised(&sandbox)?;
    let out = sandbox.run(&["init", "--user", "Grace", "--force"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Grace\"\n",
        "the name is replaced, not added beside the old one"
    );
    assert_eq!(
        sandbox.read(".decider.toml")?,
        "dir = \"docs/adr\"\n",
        "directory untouched"
    );
    assert_eq!(
        stdout(&out),
        "kept .decider.toml (already exists)\nkept docs/adr/0000-template.md (already exists)\nreplaced your name with Grace\n",
        "says what it replaced"
    );
    Ok(())
}

#[test]
fn force_with_a_directory_replaces_only_the_directory() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    initialised(&sandbox)?;
    let out = sandbox.run(&["init", "--dir", "elsewhere", "--force"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml")?,
        "dir = \"elsewhere\"\n",
        "directory replaced"
    );
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Ada\"\n",
        "name untouched"
    );
    assert_eq!(
        stdout(&out),
        "created elsewhere/0000-template.md\nreplaced .decider.toml\nkept your name as Ada\n",
        "says what it replaced"
    );
    assert!(
        sandbox
            .repo
            .path()
            .join("docs/adr/0000-template.md")
            .is_file(),
        "the old directory is left as it was"
    );
    Ok(())
}

#[test]
fn force_with_both_replaces_both() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    initialised(&sandbox)?;
    let out = sandbox.run(&["init", "--dir", "other", "--user", "Grace", "--force"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml")?,
        "dir = \"other\"\n",
        "directory"
    );
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Grace\"\n",
        "name"
    );
    Ok(())
}

#[test]
fn without_force_a_given_directory_still_does_not_replace_the_existing_one() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    initialised(&sandbox)?;
    let out = sandbox.run(&["init", "--dir", "other", "--user", "Grace"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml")?,
        "dir = \"docs/adr\"\n",
        "kept"
    );
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Ada\"\n",
        "kept"
    );
    Ok(())
}

#[test]
fn force_alone_without_a_terminal_cannot_ask_so_it_changes_nothing() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    initialised(&sandbox)?;
    let out = sandbox.run(&["init", "--force"])?;
    assert!(!out.status.success(), "it would have to ask");
    assert!(
        stderr(&out).contains("--force needs a terminal"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        sandbox.read(".decider.toml")?,
        "dir = \"docs/adr\"\n",
        "directory untouched"
    );
    assert_eq!(
        fs::read_to_string(sandbox.user_file())?,
        "name = \"Ada\"\n",
        "name untouched"
    );
    Ok(())
}

#[test]
fn force_with_only_a_name_on_a_fresh_repository_needs_no_terminal() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init", "--user", "Ada", "--force"])?;
    assert!(
        out.status.success(),
        "the directory is only missing: {}",
        stderr(&out)
    );
    assert_eq!(
        sandbox.read(".decider.toml")?,
        format!("dir = \"{DIR}\"\n"),
        "default directory"
    );
    assert!(
        stdout(&out).contains("recorded your name as Ada"),
        "{}",
        stdout(&out)
    );
    Ok(())
}

#[test]
fn force_on_a_fresh_repository_says_created_not_replaced() -> io::Result<()> {
    let sandbox = Sandbox::new()?;
    let out = sandbox.run(&["init", "--dir", "adr", "--user", "Ada", "--force"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        !stdout(&out).contains("replaced"),
        "nothing was there: {}",
        stdout(&out)
    );
    Ok(())
}
