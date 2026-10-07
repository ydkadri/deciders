# Command reference

Exit codes: 0 on success, 1 when a command fails, and 2 when the command line is wrong (an unknown command, or a missing or bad argument). Results go to standard output, and warnings and errors to standard error.

## `decider init [--dir PATH] [--user NAME] [--force]`

Sets up the repository and your own settings. Run it once per clone, at the repository root, and as the first step in CI. It is safe to run again, because it only creates what is missing, unless you pass `--force`.

| Option | Meaning |
|---|---|
| `--dir PATH` | The ADR directory. A relative path is relative to the current directory; an absolute path is used as it is. Defaults to `docs/explanations/decisions`. |
| `--user NAME` | Your name, written after `by` on the stage lines the tool creates. |
| `--force` | Replace settings that already exist. See below. |

What it does:

- Writes `.decider.toml` in the current directory, if there is none. If there is one, it is kept and `--dir` is ignored (unless `--force` is given). Only the current directory is looked at.
- Creates the ADR directory, and `0000-template.md` inside it unless that file exists. Number 0000 belongs to the template.
- Records your name in your own settings file, unless a name is already set there, in which case it is kept and `--user` is ignored (unless `--force` is given).

`PATH` must not be empty. `./docs//adr/` is stored as `docs/adr`.

A leading `~` is expanded by your shell when it is unquoted (`--dir ~/adrs` becomes an absolute path), but not by `decider`. Quoted, or written in `.decider.toml`, `~/adrs` is a relative path, and `init` makes a directory called `~` in the current directory.

An absolute path is written into `.decider.toml` as it is. That file is committed, so an absolute path is right only on the machine that wrote it: anyone else who clones the repository and runs `init` gets a directory at that same path.

**Asking.** When both input and output are terminals, `init` asks for anything it has to set and was not given. It asks for the directory first (`ADR directory [docs/explanations/decisions]:`), then for your name, suggesting one worked out from `$USER` (`youcef.kadri` becomes `Youcef Kadri`). An empty answer takes what is shown. When you are setting something again with `--force`, what is shown is the value already set, so pressing Enter keeps it and `--force` changes only what you type. With no terminal it never asks, so a CI job never waits for input: the directory falls back to the default, and with no `--user` no name is recorded and `init` says so. A name is trimmed and must be on one line.

A directory that is already set, and a name that is already set, are not asked for.

**`--force`.** It replaces settings that already exist, and says `replaced` for each one so nothing changes silently.

| Command | Replaces `.decider.toml` | Replaces your name |
|---|---|---|
| `init --force` | yes, asks for the directory | yes, asks for the name |
| `init --dir PATH --force` | yes, with `PATH` | no |
| `init --user NAME --force` | no | yes, with `NAME` |
| `init --dir PATH --user NAME --force` | yes | yes |

What was not given a value is asked for, so `init --force` on its own needs a terminal, and without one it stops before changing anything and asks you to pass `--dir` or `--user`. `--force` never touches `0000-template.md` or any ADR. If it changes the directory, the old directory is left as it was and the new one gets its own template.

Output is one line per file and one for the name, such as `created .decider.toml`, `kept docs/adr/0000-template.md (already exists)` or `recorded your name as Ada Lovelace`. Paths are shown relative to the current directory when they are below it, and in full otherwise, which is the case for an absolute `--dir` outside it.

## Configuration

There are two files, split by who they belong to (ADR 0002).

**`.decider.toml`** is in the repository root and is committed.

```toml
dir = "docs/explanations/decisions"
```

| Key | Meaning |
|---|---|
| `dir` | The ADR directory, relative to the directory holding this file, or an absolute path. Required. |

**`config.toml`** is your own file, at `$XDG_CONFIG_HOME/decider/config.toml`, or `$HOME/.config/decider/config.toml` when that variable is unset or not an absolute path. It is never committed.

```toml
name = "Ada Lovelace"
editor = "code --wait"
```

| Key | Meaning |
|---|---|
| `name` | Your name, written on the stage lines the tool creates. |
| `editor` | The command that opens a new ADR for editing, such as `vim` or `code --wait`. Optional: `$EDITOR` is used when it is missing or blank. It is split on whitespace into a program and arguments, and is not run through a shell. `init` does not set it, so add the line by hand. |

In both files an unknown key is an error, so a typo such as `dirr` is reported instead of ignored. If neither `XDG_CONFIG_HOME` nor `HOME` is an absolute path, there is nowhere to keep the name and recording one fails.

## The lifecycle commands

An ADR moves through four states (ADR 0004): `proposed`, then `accepted` and `implemented`, or `rejected`. Each change of state is a command. They look for `.decider.toml` in the current directory and then each parent, so they work from any subdirectory, and they fail with a pointer to `decider init` if there is none.

Each command adds one dated line to the ADR's header, `**Accepted:** 2026-10-05 by Ada`, with your name from your settings. If no name is set the line has no `by`, and the command prints a warning. The date is today's date in your local time zone. Paths in the output are shown as for `init`.

A move that is not allowed is refused and the file is not touched, for example ``the ADR is proposed, and `implement` needs it to be accepted``. `implemented` and `rejected` are final.

### `decider propose TITLE`

Creates a new ADR in the `proposed` state and prints its path.

- **Number:** one more than the highest number among the files in the ADR directory whose names start with at least four digits, a hyphen and a title and end in `.md`. Directories and other files are ignored, gaps are left alone, and 0000 belongs to the template, so the first ADR is 0001.
- **Filename:** the padded number and a slug of the title, such as `0007-use-postgres.md`. The slug is lower case, keeps letters and digits, joins words with `-`, and is cut at 60 characters.
- **Title:** one line, trimmed. An empty title, or one with no letters or digits, is refused.
- **Content:** the header, then the sections of `0000-template.md` in the ADR directory. If that file is missing, the template built into the tool is used.
- It never overwrites a file.
- **Editor:** when it is run at a terminal, the new file is then opened in your editor, from the `editor` setting or `$EDITOR`, and the command waits for the editor to close. Nothing opens when there is no terminal or no editor is set. If the editor cannot be started or exits with a failure, that is reported as a warning and the ADR stays as created.

### `decider accept N`

Moves ADR `N` from `proposed` to `accepted`. `N` can be written with or without leading zeros (`7` or `0007`).

### `decider reject N --reason TEXT`

Moves ADR `N` from `proposed` to `rejected`, and adds a `## Rejection` section holding the reason. A reason is required: at a terminal, `reject` asks for one if `--reason` is not given, and otherwise it stops and says so. A blank reason is refused.

### `decider implement N [--pr REF]... [--note TEXT]`

Moves ADR `N` from `accepted` to `implemented`.

| Option | Meaning |
|---|---|
| `--pr REF` | A pull request or commit that implemented it, such as `#12`. Can be repeated, and must not be blank. The references go in brackets at the end of the `Implemented` line. |
| `--note TEXT` | What happened when it was built, recorded in a `## Outcome` section. A blank note counts as no note, and with no note there is no section. |

### What a command changes

A command edits only the lines it owns. It replaces the value on the first `**Status:**` line in the header, adds its own line after the last header line, and appends its own section. Everything else in the file is left exactly as it was, including anything you wrote in the header, in the sections or in a code block.

The commands do not check that a file is well formed. A header with two `**Status:**` lines is edited at the first, and a header with no `**Status:**` line is reported as an error.
