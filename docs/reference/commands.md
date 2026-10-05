# Command reference

Exit codes: 0 on success, 1 when a command fails, and 2 when the command line is wrong. Results go to standard output, and warnings and errors to standard error.

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
```

| Key | Meaning |
|---|---|
| `name` | Your name, written on the stage lines the tool creates. |

In both files an unknown key is an error, so a typo such as `dirr` is reported instead of ignored. If neither `XDG_CONFIG_HOME` nor `HOME` is an absolute path, there is nowhere to keep the name and recording one fails.
