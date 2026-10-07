# Getting started

This tutorial sets up a repository for ADRs. It takes a minute.

## Install

`decider` is not published yet. From a checkout of this repository:

```bash
cargo install --path .
decider --version
```

## Set up a repository

From the root of the repository you want to keep ADRs in:

```bash
decider init
```

With a terminal attached, `init` asks where to keep the ADRs and for your name, showing a default for each:

```
ADR directory [docs/explanations/decisions]:
Your name for the stage lines [Ada Lovelace]:
```

Press Enter to accept a default, or type another. The name is kept in your own settings file (`~/.config/decider/config.toml`), which is never committed. The output lists what was made:

```
created .decider.toml
created docs/explanations/decisions/0000-template.md
recorded your name as Ada Lovelace
```

`.decider.toml` is the repository's setting, and you commit it. Anyone who clones the repository runs `decider init` once to record their own name, and `init` leaves the committed file alone.

In a script or CI there is no terminal, so nothing is asked. Pass the values instead:

```bash
decider init --dir docs/adr --user "Ada Lovelace"
```

To start again, replace what is set. `init --force` asks for both again, `--user` or `--dir` replaces just that one:

```bash
decider init --user "Grace Hopper" --force
```

## Propose a decision

```bash
decider propose "Use Postgres for the job queue"
```

```
created docs/explanations/decisions/0001-use-postgres-for-the-job-queue.md
```

If you have an editor set (`$EDITOR`, or `editor = "code --wait"` in `~/.config/decider/config.toml`), `propose` opens the new file in it when run at a terminal, and the command finishes when you close the editor. The header records that it is proposed, when, and by whom:

```markdown
# 0001. Use Postgres for the job queue

**Status:** proposed
**Proposed:** 2026-10-05 by Ada Lovelace
```

Below it are the sections from the template: Context, Decision, Options considered and Consequences. Fill them in and commit the file.

## Move it through its life

When the decision is agreed:

```bash
decider accept 1
```

```
docs/explanations/decisions/0001-use-postgres-for-the-job-queue.md is now accepted
```

When it has been built, say which pull requests did it, and add a note if there is something worth recording:

```bash
decider implement 1 --pr '#12' --pr '#13' --note "Shipped in two PRs."
```

The header now shows each step with its date and who did it:

```markdown
**Status:** implemented
**Proposed:** 2026-10-05 by Ada Lovelace
**Accepted:** 2026-10-05 by Ada Lovelace
**Implemented:** 2026-10-05 by Ada Lovelace (#12, #13)
```

A proposal that is turned down is rejected instead, and a reason is required. Say a second proposal, `decider propose "Use Redis"`, is turned down:

```bash
decider reject 2 --reason "Too much to run."
```

A move that does not fit the state is refused and the file is left alone. For example, `decider implement 1` on an ADR that is still proposed says ``the ADR is proposed, and `implement` needs it to be accepted``.

## What next

A check for CI that fails while any decision is still open comes next. The command details are in the [command reference](../reference/commands.md).
