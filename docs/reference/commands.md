# Command reference

`decider` manages architecture decision records (ADRs). The lifecycle and the command verbs are in [ADR 0001](../explanations/decisions/0001-adr-lifecycle.md), the file format in [ADR 0002](../explanations/decisions/0002-adr-file-format.md), and configuration in [ADR 0003](../explanations/decisions/0003-config-file.md). Only `init` and `propose` exist so far.

Exit codes: 0 on success, 1 when a command fails (for example no `.decider.toml`, or a file that would be overwritten), and 2 when the command line is wrong (an unknown command or a missing argument). Results go to standard output, and warnings and errors to standard error. Paths in results, such as `created docs/adr/0001-x.md`, are shown relative to the current directory, with `..` where they are not below it. Paths in warnings and error messages are always full paths. Set `RUST_LOG=debug` to see which settings file was found and which ADR directory a command uses.

## `decider init [--dir PATH] [--user NAME]`

Sets up the repository and your own settings. Run it once per clone, at the repository root, and as the first step in CI. It is safe to run again: it never overwrites a file.

| Option | Meaning |
|---|---|
| `--dir PATH` | The ADR directory, relative to the current directory. Defaults to `docs/explanations/decisions`. |
| `--user NAME` | Your name, written on the stage lines the tool creates. See below for what happens without it. |

What it does:

- **`.decider.toml`:** written in the current directory if it does not exist. If it does (for example it is committed and you have just cloned), it is kept, and a different `--dir` is ignored with a warning. Only the current directory is looked at: a `.decider.toml` in a parent directory does not stop `init`, but you are warned, because `propose` run below this directory would use the nearer file.
- **ADR directory and template:** the directory is created, with `0000-template.md` inside it unless that file already exists.
- **Your name:** written to your own settings file, which is never committed (see below). If a name is already set it is kept, and `--user` does not replace it. If the settings file exists but has no name, the name is added and the rest of the file is kept.

`PATH` must be a relative path below the current directory: an absolute path, a path containing `..`, an empty path, and one whose first part is `.git` or `.decider.toml` (in any letter case) are refused. `.github/adr` is allowed, because only the whole first part counts. `.` parts and repeated separators are tidied, so `./docs//adr/` is stored as `docs/adr`. The ADR directory is also checked against symbolic links: if a link on the way leads outside the current directory, the command fails and writes nothing.

Files are created only if nothing is there. A directory or a symbolic link where `.decider.toml` or the template should be is an error, and `.decider.toml` is written last so a run that fails part way can be repeated.

**Your name.** If no name is set yet, `init` uses `--user`. Without it, when a terminal is attached, it asks, suggesting a name worked out from `$USER` (`youcef.kadri` becomes `Youcef Kadri`). An empty answer accepts the suggestion, an answer that cannot be used is asked for again, and end of input (Ctrl-D) skips the question and records nothing. With no terminal and no `--user`, it records nothing and warns, so CI never hangs. The name cannot contain parentheses or line breaks, or be empty or only whitespace.

Output is one line per file and one for your name, such as `created .decider.toml`, `kept docs/adr/0000-template.md (already exists)` or `recorded your name as Ada Lovelace`.

## `decider propose TITLE`

Proposes a new ADR: creates it in the `proposed` state and prints its path.

| Argument | Meaning |
|---|---|
| `TITLE` | One line of text, such as `"Use Postgres"`. It is trimmed. |

Details:

- **Where:** `propose` looks for `.decider.toml` in the current directory and then each parent, so it works from any subdirectory. It fails with a pointer to `decider init` if there is none.
- **Number:** one more than the highest number among the files in the ADR directory whose names start with at least four digits, a hyphen and a title and end in `.md`. Directories and other files are ignored, gaps are left alone, and 0000 is reserved for the template, so the first ADR is 0001.
- **Filename:** the padded number and a slug of the title, such as `0007-use-postgres.md`. The slug is lower case, keeps letters and digits, and joins words with `-`, and is cut at 60 characters. Accents written as a separate combining mark stay with their letter. Some scripts that use other combining marks (for example Devanagari) may still be split at those marks, which only affects the filename. A title with no letters or digits is refused.
- **Content:** the sections come from the repository's own `0000-template.md`, or the built-in one if that file is missing. If that path is a symbolic link or a directory, `propose` fails instead of reading it. The file uses the line ending of the template's first line. The header has `**Status:** proposed` and a `**Proposed:**` line with today's date in your local time zone and your name.
- **Author:** your name from your settings file. If none is set, the line has no author and `propose` warns and tells you to run `decider init --user NAME`. The tool does not read git configuration.
- **Safety:** `propose` never overwrites a file. If the computed filename is taken, it fails.

## Configuration

Settings are split by who they belong to (ADR 0003).

**`.decider.toml`** is in the repository root and is committed.

```toml
dir = "docs/explanations/decisions"
```

| Key | Meaning |
|---|---|
| `dir` | The ADR directory, relative to the directory holding this file. Required. |

**`config.toml`** is your own file, at `$XDG_CONFIG_HOME/decider/config.toml`, or `$HOME/.config/decider/config.toml` when that variable is unset or not an absolute path. `HOME` must be an absolute path too, so that the file cannot land inside a repository. It is never committed.

```toml
name = "Ada Lovelace"
```

| Key | Meaning |
|---|---|
| `name` | Your name, written on the stage lines the tool creates. |

In both files unknown keys are an error, so a typo such as `dirr` is reported instead of ignored. A leading byte order mark is accepted.
