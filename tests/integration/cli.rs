//! Integration tests that run the compiled binary in a temporary directory.

#![expect(
    clippy::unwrap_used,
    reason = "setup helpers in a test file may panic, like the tests that call them"
)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

/// A temporary repository root and a temporary home, so no test touches the
/// developer's real settings.
struct Sandbox {
    root: TempDir,
    home: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        }
    }

    /// Run `decider` from `dir` with `USER` set to `login`. Standard input is
    /// closed, so the tool cannot ask questions.
    fn run_as(&self, dir: &Path, login: &str, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_decider"))
            .args(arguments)
            .current_dir(dir)
            .env_remove("RUST_LOG")
            .env_remove("XDG_CONFIG_HOME")
            .env("HOME", self.home.path())
            .env("USER", login)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_as(self.root.path(), "youcef.kadri", arguments)
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.root.path().join(path)).unwrap()
    }

    fn exists(&self, path: &str) -> bool {
        self.root.path().join(path).exists()
    }

    fn user_file_path(&self) -> std::path::PathBuf {
        self.home.path().join(".config/decider/config.toml")
    }

    fn user_file(&self) -> String {
        fs::read_to_string(self.user_file_path()).unwrap()
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
fn no_command_shows_the_help_and_fails() {
    let out = Sandbox::new().run(&[]);
    assert!(!out.status.success(), "a command is required");
    assert!(stderr(&out).contains("Usage: decider"), "{}", stderr(&out));
}

#[test]
fn usage_errors_exit_with_2_and_failures_with_1() {
    let sandbox = Sandbox::new();
    assert_eq!(
        sandbox.run(&["frobnicate"]).status.code(),
        Some(2),
        "unknown command"
    );
    assert_eq!(
        sandbox.run(&["propose"]).status.code(),
        Some(2),
        "missing title"
    );
    assert_eq!(
        sandbox.run(&["propose", "T"]).status.code(),
        Some(1),
        "no config yet"
    );
    assert_eq!(
        sandbox.run(&["--version"]).status.code(),
        Some(0),
        "version"
    );
}

#[test]
fn the_old_new_command_is_gone() {
    assert_eq!(
        Sandbox::new().run(&["new", "T"]).status.code(),
        Some(2),
        "unknown command"
    );
}

#[test]
fn init_creates_the_config_the_directory_the_template_and_the_users_name() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["init", "--user", "Youcef Kadri"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml"),
        "dir = \"docs/explanations/decisions\"\n",
        "repository settings"
    );
    assert!(
        sandbox
            .read(&format!("{DIR}/0000-template.md"))
            .contains("## Consequences"),
        "template"
    );
    assert_eq!(
        sandbox.user_file(),
        "name = \"Youcef Kadri\"\n",
        "user settings"
    );
    assert_eq!(
        stdout(&out),
        format!(
            "created .decider.toml\ncreated {DIR}/0000-template.md\nrecorded your name as Youcef Kadri\n"
        ),
        "exact output, paths relative to the current directory"
    );
}

#[test]
fn init_does_not_put_the_name_in_the_shared_config() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Youcef Kadri"]);
    assert!(
        !sandbox.read(".decider.toml").contains("Youcef"),
        "the name stays out of the repository"
    );
}

#[test]
fn init_accepts_another_directory() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["init", "--dir", "docs/adr", "--user", "Ada"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml"),
        "dir = \"docs/adr\"\n",
        "config"
    );
    assert!(
        sandbox.exists("docs/adr/0000-template.md"),
        "template in the chosen directory"
    );
}

#[test]
fn init_refuses_a_directory_outside_the_repository() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["init", "--dir", "../outside", "--user", "Ada"]);
    assert!(!out.status.success(), "a `..` directory is refused");
    assert!(stderr(&out).contains(".."), "{}", stderr(&out));
    assert!(!sandbox.exists(".decider.toml"), "nothing was written");
}

#[test]
fn init_without_a_terminal_or_user_records_no_name_and_says_so() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["init"]);
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
    assert!(!sandbox.user_file_path().exists(), "nothing written");
}

#[test]
fn init_refuses_a_user_name_that_cannot_be_written() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["init", "--user", "Jane (Contractor)"]);
    assert!(!out.status.success(), "parenthesis");
    assert!(stderr(&out).contains("--user"), "{}", stderr(&out));
}

#[test]
fn init_can_be_run_again_and_changes_nothing() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    let second = sandbox.run(&["init", "--user", "Grace"]);
    assert!(second.status.success(), "{}", stderr(&second));
    assert_eq!(
        stdout(&second),
        format!(
            "kept .decider.toml (already exists)\nkept {DIR}/0000-template.md (already exists)\nkept your name as Ada\n"
        ),
        "everything kept"
    );
    assert_eq!(
        sandbox.user_file(),
        "name = \"Ada\"\n",
        "the first name stays"
    );
}

#[test]
fn init_on_a_clone_with_a_committed_config_only_records_the_user() {
    let sandbox = Sandbox::new();
    fs::write(
        sandbox.root.path().join(".decider.toml"),
        "dir = \"docs/adr\"\n",
    )
    .unwrap();
    let out = sandbox.run(&["init", "--user", "Ada"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(".decider.toml"),
        "dir = \"docs/adr\"\n",
        "committed file untouched"
    );
    assert!(
        sandbox.exists("docs/adr/0000-template.md"),
        "directory from the committed config"
    );
    assert_eq!(sandbox.user_file(), "name = \"Ada\"\n", "user recorded");
}

#[test]
fn init_warns_when_dir_disagrees_with_the_committed_config() {
    let sandbox = Sandbox::new();
    fs::write(
        sandbox.root.path().join(".decider.toml"),
        "dir = \"docs/adr\"\n",
    )
    .unwrap();
    let out = sandbox.run(&["init", "--dir", "other", "--user", "Ada"]);
    assert!(out.status.success(), "{}", stderr(&out));
    // The binary prints the resolved path, and a temporary directory may be
    // reached through a symbolic link (as on macOS), so compare resolved paths.
    let settings = fs::canonicalize(sandbox.root.path())
        .unwrap()
        .join(".decider.toml");
    assert!(
        stderr(&out).contains(&format!(
            "ignoring --dir other: {} already sets the ADR directory",
            settings.display()
        )),
        "the warning names the full path: {}",
        stderr(&out)
    );
    assert!(
        !sandbox.exists("other"),
        "the other directory is not created"
    );
}

#[test]
fn init_keeps_an_existing_template() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.root.path().join(DIR)).unwrap();
    fs::write(
        sandbox.root.path().join(DIR).join("0000-template.md"),
        "# my own template\n",
    )
    .unwrap();
    let out = sandbox.run(&["init", "--user", "Ada"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.read(&format!("{DIR}/0000-template.md")),
        "# my own template\n",
        "kept"
    );
    assert!(stdout(&out).contains("kept"), "{}", stdout(&out));
}

#[test]
fn init_honours_xdg_config_home() {
    let sandbox = Sandbox::new();
    let xdg = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_decider"))
        .args(["init", "--user", "Ada"])
        .current_dir(sandbox.root.path())
        .env("HOME", sandbox.home.path())
        .env("XDG_CONFIG_HOME", xdg.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        xdg.path().join("decider/config.toml").is_file(),
        "written under XDG_CONFIG_HOME"
    );
    assert!(
        !sandbox.home.path().join(".config").exists(),
        "not under HOME"
    );
}

#[test]
fn propose_before_init_explains_what_to_do() {
    let out = Sandbox::new().run(&["propose", "Use Postgres"]);
    assert!(!out.status.success(), "no config yet");
    assert!(stderr(&out).contains("decider init"), "{}", stderr(&out));
}

#[test]
fn propose_creates_a_proposed_adr_with_the_next_number_and_the_users_name() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada Lovelace"]);
    let out = sandbox.run(&["propose", "Use Postgres"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!("created {DIR}/0001-use-postgres.md\n"),
        "exact output"
    );
    let text = sandbox.read(&format!("{DIR}/0001-use-postgres.md"));
    assert!(
        text.starts_with("# 0001. Use Postgres\n\n**Status:** proposed\n"),
        "{text}"
    );
    assert!(
        text.contains(" by Ada Lovelace\n"),
        "author from your settings: {text}"
    );
    assert!(
        text.contains("## Options considered"),
        "sections from the template: {text}"
    );
}

#[test]
fn propose_does_not_read_git_configuration() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    fs::write(
        sandbox.home.path().join(".gitconfig"),
        "[user]\n\tname = Git Person\n",
    )
    .unwrap();
    sandbox.run(&["propose", "T"]);
    let text = sandbox.read(&format!("{DIR}/0001-t.md"));
    assert!(
        text.contains(" by Ada\n") && !text.contains("Git Person"),
        "{text}"
    );
}

#[test]
fn propose_without_a_name_writes_no_author_and_says_how_to_set_one() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init"]);
    let out = sandbox.run(&["propose", "Nameless"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("decider init --user"),
        "{}",
        stderr(&out)
    );
    let text = sandbox.read(&format!("{DIR}/0001-nameless.md"));
    assert!(!text.contains(" by "), "no author line: {text}");
}

#[test]
fn propose_reports_a_user_file_that_cannot_be_used() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init"]);
    fs::create_dir_all(sandbox.home.path().join(".config/decider")).unwrap();
    fs::write(sandbox.user_file_path(), "name = \"A (B)\"\n").unwrap();
    let out = sandbox.run(&["propose", "T"]);
    assert!(!out.status.success(), "a name that cannot be written");
    assert!(
        stderr(&out).contains("config.toml") && stderr(&out).contains("parenthesis"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn propose_numbers_each_adr_after_the_last() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    sandbox.run(&["propose", "First"]);
    let out = sandbox.run(&["propose", "Second"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        sandbox.exists(&format!("{DIR}/0002-second.md")),
        "second ADR"
    );
}

#[test]
fn propose_works_from_a_subdirectory_and_shows_the_path_from_there() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    let deep = sandbox.root.path().join("src/deep");
    fs::create_dir_all(&deep).unwrap();
    let out = sandbox.run_as(&deep, "ada", &["propose", "From below"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        sandbox.exists(&format!("{DIR}/0001-from-below.md")),
        "found the root"
    );
    assert_eq!(
        stdout(&out),
        format!("created ../../{DIR}/0001-from-below.md\n"),
        "shown relative to where you are"
    );
}

#[test]
fn propose_copies_the_sections_from_the_repositorys_own_template() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    fs::write(
        sandbox.root.path().join(DIR).join("0000-template.md"),
        "# NNNN. T\n\n**Status:** proposed\n\n## Why\n\nBecause.\n",
    )
    .unwrap();
    sandbox.run(&["propose", "Custom"]);
    let text = sandbox.read(&format!("{DIR}/0001-custom.md"));
    assert!(
        text.contains("## Why\n\nBecause.\n") && !text.contains("## Context"),
        "{text}"
    );
}

#[test]
fn propose_rejects_a_title_with_nothing_to_make_a_filename_from() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    let out = sandbox.run(&["propose", "?!"]);
    assert!(!out.status.success(), "no letters or digits");
    assert!(stderr(&out).contains("filename"), "{}", stderr(&out));
}

#[test]
fn a_second_proposal_with_the_same_title_gets_its_own_number() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    sandbox.run(&["propose", "Same"]);
    let before = sandbox.read(&format!("{DIR}/0001-same.md"));
    sandbox.run(&["propose", "Same"]);
    assert_eq!(
        sandbox.read(&format!("{DIR}/0001-same.md")),
        before,
        "the first ADR is untouched"
    );
    assert!(
        sandbox.exists(&format!("{DIR}/0002-same.md")),
        "the second gets its own number"
    );
}

#[test]
fn stays_quiet_on_stderr_when_a_command_succeeds() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["init", "--user", "Ada"]);
    assert!(stderr(&out).is_empty(), "stderr was: {}", stderr(&out));
}

#[test]
fn init_refuses_a_directory_named_after_the_settings_file() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["init", "--dir", ".decider.toml", "--user", "Ada"]);
    assert!(
        !out.status.success(),
        "it would collide with the settings file"
    );
    assert!(
        stderr(&out).contains("must not start with"),
        "{}",
        stderr(&out)
    );
    assert!(!sandbox.exists(".decider.toml"), "nothing was created");
}

#[test]
fn init_in_a_subdirectory_warns_that_the_parent_already_has_settings() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    let sub = sandbox.root.path().join("sub");
    fs::create_dir(&sub).unwrap();
    let out = sandbox.run_as(&sub, "ada", &["init"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("already exists in"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn init_ignores_a_broken_settings_file_in_a_parent_directory() {
    let sandbox = Sandbox::new();
    fs::write(sandbox.root.path().join(".decider.toml"), "dirr = \"x\"\n").unwrap();
    let sub = sandbox.root.path().join("sub");
    fs::create_dir(&sub).unwrap();
    let out = sandbox.run_as(&sub, "ada", &["init", "--user", "Ada"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        sub.join(".decider.toml").is_file(),
        "its own settings were written"
    );
}

#[cfg(unix)]
#[test]
fn init_does_not_follow_a_symbolic_link_in_place_of_the_settings_file() {
    let sandbox = Sandbox::new();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("written-outside.toml");
    std::os::unix::fs::symlink(&target, sandbox.root.path().join(".decider.toml")).unwrap();
    let out = sandbox.run(&["init", "--user", "Ada"]);
    assert!(!out.status.success(), "refused");
    assert!(!target.exists(), "nothing was written through the link");
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_that_leaves_the_repository_is_refused_by_init_and_propose() {
    let sandbox = Sandbox::new();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), sandbox.root.path().join("link")).unwrap();
    let init = sandbox.run(&["init", "--dir", "link/adr", "--user", "Ada"]);
    assert!(!init.status.success(), "init refused");
    assert!(
        stderr(&init).contains("outside the repository"),
        "{}",
        stderr(&init)
    );
    fs::write(
        sandbox.root.path().join(".decider.toml"),
        "dir = \"link/adr\"\n",
    )
    .unwrap();
    let propose = sandbox.run(&["propose", "Escapes"]);
    assert!(!propose.status.success(), "propose refused");
    assert!(
        fs::read_dir(outside.path()).unwrap().next().is_none(),
        "nothing written outside"
    );
}

#[test]
fn init_keeps_the_rest_of_an_existing_user_file_when_adding_the_name() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.home.path().join(".config/decider")).unwrap();
    fs::write(sandbox.user_file_path(), "# my decider settings, keep\n").unwrap();
    let out = sandbox.run(&["init", "--user", "Ada"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        sandbox.user_file(),
        "name = \"Ada\"\n# my decider settings, keep\n",
        "comment kept"
    );
}

#[test]
fn a_relative_home_is_not_used_for_the_user_file() {
    let sandbox = Sandbox::new();
    let out = Command::new(env!("CARGO_BIN_EXE_decider"))
        .args(["init", "--user", "Ada"])
        .current_dir(sandbox.root.path())
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", "relative/home")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success(), "nowhere to keep the name");
    assert!(
        stderr(&out).contains("cannot find a place"),
        "{}",
        stderr(&out)
    );
    assert!(
        !sandbox.exists("relative"),
        "nothing landed in the repository"
    );
}

#[test]
fn with_no_home_and_no_name_the_hint_does_not_tell_you_to_do_what_fails() {
    let sandbox = Sandbox::new();
    let out = Command::new(env!("CARGO_BIN_EXE_decider"))
        .args(["init"])
        .current_dir(sandbox.root.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("HOME")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("set HOME or XDG_CONFIG_HOME"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_template_with_windows_line_endings_gives_an_adr_with_consistent_endings() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    fs::write(
        sandbox.root.path().join(DIR).join("0000-template.md"),
        "# NNNN. T\r\n\r\n**Status:** proposed\r\n\r\n## Context\r\n\r\nWhy.\r\n",
    )
    .unwrap();
    sandbox.run(&["propose", "Crlf"]);
    let bytes = fs::read(sandbox.root.path().join(format!("{DIR}/0001-crlf.md"))).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        !text.replace("\r\n", "").contains('\n'),
        "no bare line feed: {text:?}"
    );
}

#[test]
fn a_directory_named_like_an_adr_does_not_take_a_number() {
    let sandbox = Sandbox::new();
    sandbox.run(&["init", "--user", "Ada"]);
    fs::create_dir(sandbox.root.path().join(DIR).join("0050-notes.md")).unwrap();
    let out = sandbox.run(&["propose", "After"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        sandbox.exists(&format!("{DIR}/0001-after.md")),
        "numbered as the first ADR"
    );
}

/// A small Python program that runs a command on a pseudo-terminal, types `TYPED`
/// into it after the command has printed its question, and prints what the
/// command wrote. `pty` behaves the same on macOS and Linux, unlike `script`.
const PTY_DRIVER: &str = r#"
import os, pty, select, sys, time
typed = os.environ["TYPED"].encode().decode("unicode_escape").encode()
pid, fd = pty.fork()
if pid == 0:
    os.execv(sys.argv[1], sys.argv[1:])
out = b""
sent = False
deadline = time.time() + 20
while time.time() < deadline:
    ready, _, _ = select.select([fd], [], [], 0.2)
    if ready:
        try:
            data = os.read(fd, 4096)
        except OSError:
            break
        if not data:
            break
        out += data
    if not sent and b"]:" in out or not sent and b"stage lines:" in out:
        os.write(fd, typed)
        sent = True
sys.stdout.write(out.decode(errors="replace").replace("\r", ""))
"#;

/// Whether Python 3 can be run, decided by its exit status.
fn python_works() -> bool {
    Command::new("python3")
        .args(["-c", "import pty, select"])
        .status()
        .is_ok_and(|status| status.success())
}

/// Run `decider init` with its input and output on a pseudo-terminal, typing
/// `typed` (with `\n` for Enter) at the question, and return what it printed.
/// Returns `None` where Python 3 is not available.
fn init_on_a_terminal(sandbox: &Sandbox, xdg: &Path, typed: &str) -> Option<String> {
    if !python_works() {
        return None;
    }
    let out = Command::new("python3")
        .args(["-c", PTY_DRIVER, env!("CARGO_BIN_EXE_decider"), "init"])
        .current_dir(sandbox.root.path())
        .env("TYPED", typed)
        .env("XDG_CONFIG_HOME", xdg)
        .env("HOME", sandbox.home.path())
        .env("USER", "youcef.kadri")
        .output()
        .unwrap();
    Some(stdout(&out))
}

#[test]
fn an_empty_answer_at_the_name_question_takes_the_suggestion() {
    let sandbox = Sandbox::new();
    let xdg = tempfile::tempdir().unwrap();
    let Some(text) = init_on_a_terminal(&sandbox, xdg.path(), "\\n") else {
        return;
    };
    assert!(
        text.contains("Your name for the stage lines [Youcef Kadri]:"),
        "{text}"
    );
    assert_eq!(
        fs::read_to_string(xdg.path().join("decider/config.toml")).unwrap(),
        "name = \"Youcef Kadri\"\n",
        "the suggestion was recorded"
    );
}

#[test]
fn a_typed_answer_at_the_name_question_wins_over_the_suggestion() {
    let sandbox = Sandbox::new();
    let xdg = tempfile::tempdir().unwrap();
    let Some(_) = init_on_a_terminal(&sandbox, xdg.path(), "Grace Hopper\\n") else {
        return;
    };
    assert_eq!(
        fs::read_to_string(xdg.path().join("decider/config.toml")).unwrap(),
        "name = \"Grace Hopper\"\n",
        "the typed name was recorded"
    );
}

#[test]
fn an_unusable_answer_is_asked_for_again() {
    let sandbox = Sandbox::new();
    let xdg = tempfile::tempdir().unwrap();
    let Some(text) = init_on_a_terminal(&sandbox, xdg.path(), "Bad (Name)\\nAda Lovelace\\n")
    else {
        return;
    };
    assert!(text.contains("try again"), "{text}");
    assert_eq!(
        fs::read_to_string(xdg.path().join("decider/config.toml")).unwrap(),
        "name = \"Ada Lovelace\"\n",
        "the second answer was recorded"
    );
}
