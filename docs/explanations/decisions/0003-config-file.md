# 0003. Configuration lives in two files

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

Teams keep ADRs in different places. This project uses `docs/explanations/decisions/` under a Diataxis layout, and others will differ. The tool, CI and the build skills all need to find the directory the same way, from any working directory in the repo.

Some settings belong to the repository, such as where the ADRs are. Others belong to a person, such as the name written on each stage line and, later, which editor to open. A shared, committed file cannot hold a person's name, because it would be wrong for every other contributor, in the same way that a committed `.gitconfig` would be.

## Decision

Configuration is split by who it belongs to.

**Repository settings** are in `.decider.toml`, in the repository root, and are committed. `decider init` writes it in the current directory, so it is meant to be run at the repository root:

```toml
dir = "docs/explanations/decisions"
```

`dir` is relative to the directory that holds `.decider.toml`. Every command except `init` walks up from the current directory to find the file, as `git` does, and errors with a pointer to `decider init` if there is none. `init` looks only in the current directory: an existing file there is kept, one in a parent directory is not used, and the user is warned about it, because commands run below this directory would find the nearer file.

**User settings** are in `config.toml` in the decider directory under the user's configuration directory (`$XDG_CONFIG_HOME/decider/config.toml`, or `~/.config/decider/config.toml` when that is unset), and are never committed:

```toml
name = "Youcef Kadri"
```

`name` is written on the stage lines the tool creates (ADR 0002). Because it is written after `by`, and brackets on the `Implemented` line hold references, a name is validated when it is given (`--user` and the prompt) and again whenever it is read from the user file: it is trimmed, and it must not be empty, contain a line break, or contain a parenthesis. A user file that holds a name the tool cannot use is an error that names the file, not a missing name, so a hand-edited value is reported instead of being written into an ADR. A later ADR adds the editor setting.

`decider init [--dir PATH] [--user NAME]` sets up a repository and a user in one step, and every value it needs can be passed as an option so that CI can run it non-interactively as its first step:

- It writes `.decider.toml` in the current directory, creates the ADR directory, and writes an embedded, repository-editable `0000-template.md` inside it. It does not consult git, so it also works outside a repository. In this list, "the repository root" means the directory that holds `.decider.toml`, which is the current directory for `init`. `init` refuses an ADR directory that is absolute, has a `..` part, has `.git` or `.decider.toml` as its first part (in any letter case), or leads outside that directory through a symbolic link. Whole path parts are compared, so `.github/adr` is allowed.
- With no `--dir`, it uses `docs/explanations/decisions`.
- If the user file has no name, it takes `--user`. Without `--user` it asks, offering a default worked out from `$USER` (or `$USERNAME` where that is what the system sets): `youcef.kadri` becomes `Youcef Kadri`, by splitting on `.`, `_` and `-` and capitalising each part. An empty answer takes the default, an answer that cannot be used is asked for again, and end of input (Ctrl-D) records nothing. With neither set there is no default, so an empty answer records nothing too. With no terminal and no `--user`, it writes no name and says so, so CI never hangs.
- It is safe to run again. An existing `.decider.toml` is kept (and `--dir` is ignored with a warning if it disagrees), an existing template is kept, and an existing user name is kept, so a clone that already has the committed `.decider.toml` can run it as its first step. It creates a file inside the repository only when nothing is there, and refuses a directory or a symbolic link in its place, so a template that was deleted on purpose is created again. The user file is treated differently: it may be a symbolic link, as a dotfiles manager makes, and is followed. The one change it makes to an existing file is to add a name to a user file that has none, keeping everything else in it.

Commands that create a stage line use the name from the user file. If there is none, they write the stage line without an author, and tell the user how to set one.

The tool reads no git configuration. Requiring `init` in each clone is deliberate: it makes the tool part of the development workflow, and CI runs it first.

## Options considered

- **Parse `CLAUDE.md` for the location.** Rejected: it is prose and would break when reworded.
- **A `--dir` flag on every command.** Rejected: every call from a skill or script would have to repeat it.
- **A visible `decider.toml`, or a file under `.config/` in the repository.** Rejected: this is tool configuration, not something readers browse.
- **Read the repository settings from `pyproject.toml` or `Cargo.toml` instead of `.decider.toml`.** Deferred, tracked in issue #8. `decider` works in repositories of any language, and many have no project TOML, so `.decider.toml` has to exist anyway and supporting both adds precedence rules. Writing a table into an existing `Cargo.toml` without damaging it needs an editing library and risks a file this tool does not own. In a monorepo the nearest project file is often a member package rather than the repository root, and `dir` is a fact about the repository. If it is built, it would be a read-only extra source (`[tool.decider]`, `[package.metadata.decider]`) that `.decider.toml` overrides and `init` never writes to. That would need a new ADR, which will supersede this one.
- **One `.decider.toml` holding everything, gitignored.** Rejected: `dir` is a fact about the repository that every contributor and CI job needs, and a missing file would force each of them to guess it. The name and editor are the per-person part.
- **Take the author from `git config user.name`.** Rejected: it adds a dependency on git and on parsing its output, and a name it holds is never checked against the rules for a stage line. A name is a person's choice, so it is asked for, checked before it is stored, and reported if the stored value later becomes unusable.
- **Use `$USER` as it is.** Rejected: it is a login name, not a display name. It is only used to suggest a default.
- **Default to `docs/decisions`.** Rejected in favour of the Diataxis explanation quadrant, because ADRs record why rather than how.
- **Default to `docs/explanation/adr/`, the path in the `mine` plugin conventions.** Rejected: `docs/explanations/decisions/` is where this project's own ADRs live, and the name says what the records are. Repositories that prefer another path pass `--dir`.

## Consequences

- One source of truth for the location that the tool, CI and the skills read identically. `CLAUDE.md` documents the convention but is never read by the tool.
- Each clone and each CI job runs `decider init` before any other command. Running it again changes nothing, unless the user file still has no name and a name is now given, or a file that it creates has since been deleted. Both files can grow new keys when something needs them.
- A name inferred from `$USER` can be wrong, for example for a login such as `jsmith`. The prompt shows the inferred name so it can be corrected, and `--user` sets it directly.
- The user file lives outside the repository, so the name in it cannot leak into the shared history by accident.
