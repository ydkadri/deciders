# decider

Outcome: a developer can propose an architectural decision as an ADR, get it explicitly accepted, and run a check that fails while any decision is still incomplete.

## What this is

A lightweight tool for editing ADR files in a developer's own repository. The files, the settings and the templates are trusted: there is no defence against hostile input and no control of concurrent runs. The library stays small, and commands edit the text they own instead of modelling whole documents. Prefer the simpler design. Raise anything that adds a layer, a dependency or defensive code before building it.

## Decisions

The decisions live in the ADRs under `docs/explanations/decisions/`, so they are not repeated here. Start with the index in that directory's `README.md`.

## Targets

- `just install`: toolchain and dependencies
- `just check`: lint, typecheck, coverage and doctests. Run before every commit.
- `just test`, `just test-integration`, `just format`, `just build`, `just vulnerability`

## Conventions

Follow the conventions in the `mine` plugin's install directory: `conventions/full/languages/rust.md`, `conventions/full/project/`, `conventions/full/git.md` and `conventions/checklist.md`. Do not copy them here.
