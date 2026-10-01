# Getting started

This tutorial creates your first ADR. It takes a few minutes.

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

With a terminal attached, `init` asks for your name, suggesting one from your login:

```
Your name for the stage lines [Ada Lovelace]:
```

Press Enter to accept it, or type another. Your name is kept in your own settings file (`~/.config/decider/config.toml`), which is never committed. The output lists what was made:

```
created .decider.toml
created docs/explanations/decisions/0000-template.md
recorded your name as Ada Lovelace
```

`.decider.toml` is the repository's setting, and you commit it. Anyone who clones the repository runs `decider init` once to set their own name, and `init` leaves the committed file alone.

If your docs are organised differently, choose the directory, and in a script or CI pass the name so nothing is asked:

```bash
decider init --dir docs/adr --user "Ada Lovelace"
```

## Propose a decision

```bash
decider propose "Use Postgres for the job queue"
```

```
created docs/explanations/decisions/0001-use-postgres-for-the-job-queue.md
```

Open the file. The header records that the ADR is proposed, when, and by whom:

```markdown
# 0001. Use Postgres for the job queue

**Status:** proposed
**Proposed:** 2026-10-02 by Ada Lovelace
```

Below it are the sections from the template: Context, Decision, Options considered and Consequences. Fill them in and commit the file.

## What next

Accepting, rejecting and implementing an ADR, and the check for CI, arrive in the next versions. The command details are in the [command reference](../reference/commands.md).
