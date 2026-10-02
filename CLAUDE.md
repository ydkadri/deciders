# decider

Outcome: a developer can propose an architectural decision as an ADR, get it explicitly accepted, and run a check that fails while any decision is still incomplete.

A Rust CLI. See ADR 0005 for how it is named, versioned and distributed.

## Decisions

The decisions live in the ADRs under `docs/explanations/decisions/`, so they are not repeated here. Start with the index in that directory's `README.md`.

- Lifecycle, command verbs and the `check` modes: ADR 0001
- File format and numbering: ADR 0002
- Configuration (`.decider.toml` and the user file): ADR 0003
- Architecture, the layers and the ports: ADR 0004
- Names (repo, crate, binary), versions and distribution: ADR 0005
- Amendments and the editor fallback (v0.2.0): ADR 0006
- `Depends on` (v0.2.0): ADR 0007
- Generated index and bulk implement (v0.2.0): ADR 0008

## Targets

- `just install`: toolchain and dependencies
- `just check`: lint, typecheck, coverage and doctests. Run before every commit.
- `just test`, `just test-integration`, `just format`, `just build`, `just vulnerability`

## Conventions

Follow the `mine` plugin conventions: `conventions/full/languages/rust.md`, `conventions/full/project/`, `conventions/full/git.md` and `conventions/checklist.md`. Do not copy them here.
