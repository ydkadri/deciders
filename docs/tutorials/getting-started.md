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

## What next

Proposing, accepting and implementing an ADR arrive in the next versions. The command details are in the [command reference](../reference/commands.md).
