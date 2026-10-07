# 0002. Configuration lives in two files

**Status:** accepted
**Proposed:** 2026-10-05 by Youcef Kadri
**Accepted:** 2026-10-05 by Youcef Kadri

## Context

Teams keep ADRs in different places, so the tool has to be told where they are. Some settings belong to the repository, such as that location. Others belong to a person, such as the name written on each stage line. A committed file cannot hold a person's name, because it would be wrong for every other contributor, in the same way that a committed `.gitconfig` would be.

## Decision

Configuration is split by who it belongs to.

**Repository settings** are in `.decider.toml` in the repository root, and are committed:

```toml
dir = "docs/explanations/decisions"
```

`dir` is relative to the directory that holds the file, or an absolute path. `init` looks only in the current directory. The commands that come after it will look in the current directory and then each parent, as `git` does, and error with a pointer to `decider init` if there is none.

**User settings** are in `config.toml` under the user's configuration directory, `$XDG_CONFIG_HOME/decider/`, or `~/.config/decider/` when that is unset or not an absolute path, and are never committed:

```toml
name = "Youcef Kadri"
editor = "code --wait"
```

`name` is written after `by` on the stage lines the tool creates. It is trimmed, and a name with a line break is refused, because it would split the header.

`editor` is the command that opens a new ADR for editing (ADR 0004). It is optional, and when it is missing or blank the `EDITOR` environment variable is used. The value is split on whitespace into a program and its arguments, so `code --wait` works, and the file is added as the last argument. It is not run through a shell, so quoting does not apply. `init` does not set it: it is written by hand in the file, until a `config` command exists (issue #9).

`decider init [--dir PATH] [--user NAME] [--force]` sets both up, and every value can be passed as an option so that CI can run it with no terminal as its first step:

- It writes `.decider.toml` in the current directory, creates the ADR directory, and writes the ADR template, `0000-template.md`, inside it. The template is a file in the repository, so it can be edited. The number 0000 belongs to the template and is not used for an ADR.
- The directory comes from `--dir`. Without it, and when both input and output are terminals, `init` asks, offering `docs/explanations/decisions` as the default for an empty answer, or the directory already set when it is being set again with `--force`. With no terminal it uses the default. A `--dir` must not be empty. A relative one is relative to the current directory, and an absolute one is used as it is.
- The name comes from `--user`. Without it, and when both input and output are terminals, `init` asks, offering a default worked out from `$USER`: `youcef.kadri` becomes `Youcef Kadri`, by splitting on `.`, `_` and `-` and capitalising each part. An empty answer takes the default, which is the name already set when it is being set again with `--force`. With no terminal and no `--user`, no name is recorded and `init` says so.
- It is safe to run again. A file that already exists is kept, including a name that is already set, so a fresh clone that already has `.decider.toml` runs `init` only to record its own name. Nothing that is already set is asked for.
- `--force` replaces what already exists, and prints `replaced` for each. On its own it sets both again, asking for each at a terminal. With `--dir` it replaces only the directory, and with `--user` only the name. Without a terminal, a setting that would have to be asked for is an error, raised before anything is written. It never touches the template or an ADR, and the old ADR directory is left as it was.

The tool reads no git configuration. Requiring `init` in each clone is deliberate: it makes the tool part of the development workflow, and CI runs it first.

## Options considered

- **One `.decider.toml` holding everything, gitignored.** Rejected: the ADR location is a fact about the repository that every contributor and CI job needs, and a missing file would force each of them to guess it.
- **Take the name from `git config user.name`.** Rejected: it adds a dependency on git and on reading its output.
- **Use `$USER` as it is.** Rejected: it is a login name, not a display name. It is only used to suggest a default.
- **Read the settings from `pyproject.toml` or `Cargo.toml`.** Deferred, tracked in issue #8. The tool works in repositories of any language, so `.decider.toml` has to exist anyway. If it is built it would be a read-only extra source that `.decider.toml` overrides, and it would need its own ADR, which will supersede this one.

## Consequences

- The ADR location has one source that the tool, CI and the build skills read the same way.
- Each clone and each CI job runs `decider init` before any other command. Running it again changes nothing.
- A name inferred from `$USER` can be wrong, for example for a login such as `jsmith`. The prompt shows it so it can be corrected, and `--user` sets it directly.
- The user file lives outside the repository, so the name in it cannot leak into the shared history by accident.
- An absolute `dir` lets one developer keep ADRs outside the repository, but `.decider.toml` is committed, so an absolute path is right only on the machine that wrote it. Anyone else who clones the repository and runs `init` gets a directory at that same path, and should use a relative `dir`.
- A `~` in `dir` is not expanded by the tool. Unquoted on the command line the shell expands it; otherwise it is an ordinary relative path.
