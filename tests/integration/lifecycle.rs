//! Integration tests for the lifecycle commands, run against the real binary.
//!
//! Every run gets its own repository, home and configuration directories, so no
//! test can touch the developer's real settings.

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

struct Repo {
    dir: TempDir,
    home: TempDir,
    config: TempDir,
}

impl Repo {
    /// A repository set up with `init`, for a user called Ada.
    fn new() -> io::Result<Self> {
        let repo = Self {
            dir: tempfile::tempdir()?,
            home: tempfile::tempdir()?,
            config: tempfile::tempdir()?,
        };
        let out = repo.run(&["init", "--dir", "adr", "--user", "Ada"])?;
        assert!(out.status.success(), "{}", stderr(&out));
        Ok(repo)
    }

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
        self.run_in(self.dir.path(), arguments)
    }

    /// Run a command that is only setup for the test, and check that it worked.
    fn ok(&self, arguments: &[&str]) -> io::Result<()> {
        let out = self.run(arguments)?;
        assert!(out.status.success(), "{arguments:?}: {}", stderr(&out));
        Ok(())
    }

    fn read(&self, path: &str) -> io::Result<String> {
        fs::read_to_string(self.dir.path().join(path))
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Today's date, as the binary writes it.
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[test]
fn propose_creates_a_numbered_proposed_adr_with_the_sections_and_the_users_name() -> io::Result<()>
{
    let repo = Repo::new()?;
    let out = repo.run(&["propose", "Use Postgres"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "created adr/0001-use-postgres.md\n",
        "exact output"
    );
    let text = repo.read("adr/0001-use-postgres.md")?;
    assert!(
        text.starts_with(&format!(
            "# 0001. Use Postgres\n\n**Status:** proposed\n**Proposed:** {} by Ada\n\n## Context\n",
            today()
        )),
        "{text}"
    );
    assert!(
        text.contains("## Options considered"),
        "sections from the template: {text}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn propose_never_opens_an_editor_without_a_terminal() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let repo = Repo::new()?;
    let marker = repo.home.path().join("editor-ran");
    let script = repo.home.path().join("editor.sh");
    fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display()))?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
    fs::write(
        repo.config.path().join("decider").join("config.toml"),
        format!("name = \"Ada\"\neditor = \"{}\"\n", script.display()),
    )?;
    let out = Command::new(env!("CARGO_BIN_EXE_decider"))
        .args(["propose", "T"])
        .current_dir(repo.dir.path())
        .env("HOME", repo.home.path())
        .env("XDG_CONFIG_HOME", repo.config.path())
        .env("EDITOR", &script)
        .stdin(Stdio::null())
        .output()?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        !marker.exists(),
        "the editor must not run without a terminal"
    );
    Ok(())
}

#[test]
fn each_proposal_takes_the_next_number() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "First"])?;
    let out = repo.run(&["propose", "Second"])?;
    assert_eq!(
        stdout(&out),
        "created adr/0002-second.md\n",
        "second number"
    );
    Ok(())
}

#[test]
fn the_whole_lifecycle_in_order() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "Use Postgres"])?;
    let accepted = repo.run(&["accept", "1"])?;
    assert_eq!(
        stdout(&accepted),
        "adr/0001-use-postgres.md is now accepted\n",
        "accept output"
    );
    let done = repo.run(&[
        "implement",
        "1",
        "--pr",
        "#12",
        "--pr",
        "#13",
        "--note",
        "Shipped.",
    ])?;
    assert_eq!(
        stdout(&done),
        "adr/0001-use-postgres.md is now implemented\n",
        "implement output"
    );
    let text = repo.read("adr/0001-use-postgres.md")?;
    let date = today();
    assert!(text.contains("**Status:** implemented\n"), "{text}");
    assert!(text.contains(&format!("**Proposed:** {date} by Ada\n**Accepted:** {date} by Ada\n**Implemented:** {date} by Ada (#12, #13)\n")), "{text}");
    assert!(text.ends_with("\n\n## Outcome\n\nShipped.\n"), "{text}");
    Ok(())
}

#[test]
fn a_number_can_be_written_with_leading_zeros() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "T"])?;
    let out = repo.run(&["accept", "0001"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    Ok(())
}

#[test]
fn reject_needs_a_reason_and_records_it() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "T"])?;
    let without = repo.run(&["reject", "1"])?;
    assert!(!without.status.success(), "no reason, no terminal");
    assert!(
        stderr(&without).contains("needs a reason"),
        "{}",
        stderr(&without)
    );
    assert!(
        repo.read("adr/0001-t.md")?.contains("**Status:** proposed"),
        "untouched"
    );
    let blank = repo.run(&["reject", "1", "--reason", "   "])?;
    assert!(!blank.status.success(), "a blank reason is no reason");
    assert!(
        stderr(&blank).contains("a rejection needs a reason"),
        "{}",
        stderr(&blank)
    );
    let out = repo.run(&["reject", "1", "--reason", "Too costly."])?;
    assert!(out.status.success(), "{}", stderr(&out));
    let text = repo.read("adr/0001-t.md")?;
    assert!(text.contains("**Status:** rejected\n"), "{text}");
    assert!(
        text.ends_with("\n\n## Rejection\n\nToo costly.\n"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_move_the_lifecycle_does_not_allow_is_refused_and_changes_nothing() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "T"])?;
    let before = repo.read("adr/0001-t.md")?;
    let out = repo.run(&["implement", "1"])?;
    assert_eq!(out.status.code(), Some(1), "a failure, not a usage error");
    assert!(
        stderr(&out).contains("the ADR is proposed, and `implement` needs it to be accepted"),
        "{}",
        stderr(&out)
    );
    assert_eq!(repo.read("adr/0001-t.md")?, before, "untouched");
    Ok(())
}

#[test]
fn a_final_state_cannot_be_moved_again() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "T"])?;
    repo.ok(&["reject", "1", "--reason", "No."])?;
    let out = repo.run(&["accept", "1"])?;
    assert!(!out.status.success(), "rejected is final");
    assert!(
        stderr(&out).contains("the ADR is rejected"),
        "{}",
        stderr(&out)
    );
    Ok(())
}

#[test]
fn an_adr_that_does_not_exist_is_reported_by_number() -> io::Result<()> {
    let repo = Repo::new()?;
    let out = repo.run(&["accept", "9"])?;
    assert!(!out.status.success(), "no such ADR");
    assert!(
        stderr(&out).contains("there is no ADR number 9"),
        "{}",
        stderr(&out)
    );
    Ok(())
}

#[test]
fn commands_work_from_a_subdirectory() -> io::Result<()> {
    let repo = Repo::new()?;
    let deep = repo.dir.path().join("src/deep");
    fs::create_dir_all(&deep)?;
    let out = repo.run_in(&deep, &["propose", "From below"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        repo.dir.path().join("adr/0001-from-below.md").is_file(),
        "found the root"
    );
    Ok(())
}

#[test]
fn before_init_the_commands_say_what_to_do() -> io::Result<()> {
    let repo = Repo {
        dir: tempfile::tempdir()?,
        home: tempfile::tempdir()?,
        config: tempfile::tempdir()?,
    };
    for arguments in [vec!["propose", "T"], vec!["accept", "1"]] {
        let out = repo.run(&arguments)?;
        assert!(!out.status.success(), "{arguments:?}");
        assert!(stderr(&out).contains("decider init"), "{}", stderr(&out));
    }
    Ok(())
}

#[test]
fn without_a_name_the_line_has_no_author_and_the_user_is_told() -> io::Result<()> {
    let repo = Repo::new()?;
    fs::remove_dir_all(repo.config.path().join("decider"))?;
    let out = repo.run(&["propose", "T"])?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("decider init --user"),
        "{}",
        stderr(&out)
    );
    assert!(
        repo.read("adr/0001-t.md")?
            .contains(&format!("**Proposed:** {}\n", today())),
        "no by"
    );
    Ok(())
}

#[test]
fn the_name_comes_from_the_settings_and_not_from_git() -> io::Result<()> {
    let repo = Repo::new()?;
    fs::write(
        repo.home.path().join(".gitconfig"),
        "[user]\n\tname = Git Person\n",
    )?;
    repo.ok(&["propose", "T"])?;
    let text = repo.read("adr/0001-t.md")?;
    assert!(
        text.contains(" by Ada\n") && !text.contains("Git Person"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_blank_reference_is_refused_and_changes_nothing() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "T"])?;
    repo.ok(&["accept", "1"])?;
    let before = repo.read("adr/0001-t.md")?;
    let out = repo.run(&["implement", "1", "--pr", "#1", "--pr", " "])?;
    assert!(
        !out.status.success() && stderr(&out).contains("--pr must not be empty"),
        "{}",
        stderr(&out)
    );
    assert_eq!(repo.read("adr/0001-t.md")?, before, "untouched");
    Ok(())
}

#[test]
fn a_missing_name_is_warned_about_only_once_the_line_is_written() -> io::Result<()> {
    let repo = Repo::new()?;
    repo.ok(&["propose", "T"])?;
    fs::remove_dir_all(repo.config.path())?;
    let missing = repo.run(&["accept", "9"])?;
    assert!(
        !stderr(&missing).contains("no name is set"),
        "nothing was written: {}",
        stderr(&missing)
    );
    let done = repo.run(&["accept", "1"])?;
    assert!(
        stderr(&done).contains("no name is set"),
        "{}",
        stderr(&done)
    );
    Ok(())
}

#[test]
fn a_bad_title_is_refused_and_writes_nothing() -> io::Result<()> {
    let repo = Repo::new()?;
    for title in ["", "   ", "?!"] {
        let out = repo.run(&["propose", title])?;
        assert!(!out.status.success(), "{title:?}");
    }
    assert_eq!(
        fs::read_dir(repo.dir.path().join("adr"))?.count(),
        1,
        "only the template"
    );
    Ok(())
}

#[test]
fn an_edit_leaves_the_rest_of_a_hand_written_adr_exactly_as_it_was() -> io::Result<()> {
    let repo = Repo::new()?;
    let original = "# 0001. Odd\n\n**Status:** proposed\n**Proposed:** 2026-01-01 by Someone\n**Owner:** Ada\n\n## Context\n\nText with\ttabs,  double  spaces and a `**Status:** accepted` in code.\n\n```\n**Status:** proposed\n```\n";
    fs::write(repo.dir.path().join("adr/0001-odd.md"), original)?;
    repo.ok(&["accept", "1"])?;
    let text = repo.read("adr/0001-odd.md")?;
    let expected = original
        .replacen("**Status:** proposed", "**Status:** accepted", 1)
        .replacen(
            "**Owner:** Ada\n",
            &format!("**Owner:** Ada\n**Accepted:** {} by Ada\n", today()),
            1,
        );
    assert_eq!(text, expected, "only the status and one new line changed");
    Ok(())
}
