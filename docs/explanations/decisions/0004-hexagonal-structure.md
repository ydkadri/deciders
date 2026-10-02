# 0004. Hexagonal structure in a single crate

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

The tool depends on a few things outside the process: where ADRs are stored (today a directory of Markdown files), the user's settings, the clock, the environment (the login name and whether a terminal is attached) and, from v0.2.0, an editor. All are local and small. There is no service or network call.

The ADR store is the boundary most likely to change. A team may want to keep decisions in a database or a wiki, and the lifecycle rules should not change when they do.

## Decision

Build one crate, `decider-adr`, in a hexagonal structure. Choosing one crate rejects the workspace layout (ADR 0005 covers the crate name), not the layering. The layers are modules:

- **`domain`:** the ADR model and its rules: `Status` and the transition table, `AdrNumber`, `Date`, `Header`, `Adr`, and building a proposed ADR. Pure, with no I/O, and it imports nothing from the other layers.
- **`ports`:** the traits the use cases need, named with verbs and written in terms of ADRs, not files. `ReadsAdrs` lists numbers and provides the template. `WritesAdrs` prepares a store and creates an ADR. `KeepsUserSettings` reads and writes the user's name. `AsksForName` asks the person running the tool for their name, when there is one to ask. Later use cases add `read` and `update` to the ports when they need them.
- **`app`:** the use cases (`init`, `propose`, and later `accept`, `implement`, `check`). They sequence port calls and domain calls, and hold only the rules about the order of those calls and what to do when one gives nothing, such as not asking for a name that is already set. The rules of the model itself stay in `domain`. They take ports as arguments.
- **`adapters`:** the implementations of the ports. `files` stores ADRs as Markdown files, with the filename, the slug and `.decider.toml` as its own details. `user_file` keeps the user's name in a TOML file. `memory` holds a fake of each port for tests.
- **The binary** is the driving adapter (the clap command line) and the composition root. It is the only place that knows the domain, the app and the adapters together. It reads the clock once and passes the date to the use cases as a plain value. It also provides the terminal implementation of `AsksForName`, which is the only place the login name and the terminal are read.

Two choices depart from the `mine` plugin's architecture guide, and are made on purpose. The ports live in their own `ports` module instead of inside the domain, so that the domain holds only the model and its rules and a port can name the user's settings and the person at the terminal, which are not part of the model. And the use cases hold the rules about the order of calls and about not asking for a name that is already set, where the guide says an orchestrator makes no decisions, because those rules decide when to call a port and cannot be tested through the domain.

Dependencies point inwards: `domain` depends on nothing, `ports` and `app` on `domain` (and `app` on `ports`), and `adapters` on `domain` and `ports`.

Ports are in terms of ADRs, so the use cases never see a path or the text of an ADR. The one piece of raw text they handle is the template, which the port returns as a string and the domain reads the sections from. The parsed `Adr` is the model, and the file format (ADR 0002) is a detail of the files adapter, which calls the domain's `parse` and `render`.

## Options considered

- **A workspace of core, adapters and app crates.** Rejected: more structure than the project needs. The layering is the same and a split later is mechanical.
- **Functional core and imperative shell, with no ports.** Rejected: the use cases could only be tested against the disk, and another store would mean changing them.
- **A port of files and paths (`read_text`, `list_names`, `write_new`).** Rejected: a database or wiki store would have to pretend to be a filesystem, and parsing would stay in the use cases.
- **A port for the clock.** Rejected: the date is read once at the edge and a literal value in a test is the fake.
- **Passing the user's name to `init` as a plain value, with the prompt in the binary.** Rejected: the rules around asking belong in the use case, and a binary that holds them cannot be tested with a fake. They are that an existing name is never replaced (so nobody is asked), an option given on the command line is used without asking, and a question that fails, or ends with no input at all (Ctrl-D), records nothing and says so. What an empty answer means (it accepts the suggested name) is the asker's rule, because only the asker knows the suggestion. The terminal check and the login name stay inside the terminal implementation of `AsksForName`, so the use case never sees them.

## Consequences

- The use cases are tested with in-memory fakes, and the files adapter and the binary with temporary directories. The interactive prompt is tested through a pseudo-terminal.
- Another store is a new adapter plus a way to choose it in configuration. The settings that `.decider.toml` holds today (`dir`) are specific to the files store, and generalising them is tracked in issue #7.
- The edges of the ports were shaped by one store, so the first other adapter may need them adjusted.
- Reads and writes are separate traits, so a command that only reads (`list`, `show`, `check`) takes only `ReadsAdrs` and cannot change anything.
- Dependency direction is kept by review for now. A script that checks module imports can be added if it drifts.
