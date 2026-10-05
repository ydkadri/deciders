# 0003. A small hexagon in one crate

**Status:** accepted
**Proposed:** 2026-10-05 by Youcef Kadri
**Accepted:** 2026-10-05 by Youcef Kadri

## Context

The tool is a file editor: each command reads a few ADR files and edits the text it owns. The ADR store is the one boundary likely to change, since a team may one day keep decisions in a database or a wiki, and the lifecycle rules should not change when they do. The tool should stay small.

## Decision

Build one crate, `decider-adr`, with a library and a thin binary, layered as a small hexagon.

- **The library** has the rules of the tool and no command-line code. Its layers are modules: the lifecycle rules, a **store port**, and the **file adapter** that implements the port. Dependencies point inwards, so the rules know nothing about files.
- **The store port** is one trait, in terms of ADRs: it says what the commands need (`prepare` a store, and later list, read and create ADRs). A command never sees the layout of a file. It gets back the places a store made, replaced or kept, as paths, only to report them.
- **The file adapter** stores ADRs as Markdown files. It owns everything physical: the ADR directory, `.decider.toml`, the ADR template file, the filename, and the editing of the header lines. Commands edit the text they own, and do not parse a whole ADR into a model and write it back.
- **The user's settings file** is a plain module. It is read by `init` and the commands that write a stage line, and nothing else would supply it, so it is not a port.
- **The binary** is the command line and the composition root. It reads the terminal, builds the file adapter, and calls the library. Commands that date a stage line will read the clock here too.

## Options considered

- **A workspace of several crates.** Rejected: more structure than a file editor needs, and splitting later is mechanical.
- **No port, with commands editing files directly.** Rejected: another kind of store would then mean rewriting every command.
- **A model of the whole ADR, parsed and written back.** Rejected: it needs a strict parser, a writer that proves it reads back, and types for every field, which is most of the code of a tool that edits a few lines.
- **A port for the user's settings and for the terminal.** Rejected: each has one implementation and is read in one place.

## Consequences

- Another store is a new adapter plus a way to choose it in configuration. The settings in `.decider.toml` are specific to the file adapter, and generalising them is tracked in issue #7.
- The edges of the port are shaped by one store, so the first other adapter may need them adjusted.
- Commands that only read take a store they cannot write to, once the port has a read side.
