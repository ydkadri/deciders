# Claude Code Instructions for DecideRS

Project-specific instructions. General conventions live in `~/Documents/ydkadri/claude/` and apply unless overridden here.

## Project Context

- **Type**: Personal project - CLI tool
- **Purpose**: Document decisions (proposals -> accepted architectural decisions -> implementation) from the command line
- **Language**: Rust, edition 2024
- **Package manager**: cargo
- **Task runner**: justfile
- **CI/CD**: GitHub Actions (Ubuntu only for now)
- **Distribution**: none yet - no crates.io publish or release pipeline until the tool is ready to ship
- **Spelling**: British English (colour, behaviour, optimise) in docs, comments, and user-facing text

## CRITICAL Rules

- **Never push directly to `main`** - all changes go through PRs (branch protection enforces this)
- **Ask questions one at a time** when requirements are unclear - explain why the question matters, offer a recommendation
- Code must comply with `~/Documents/ydkadri/claude/languages/rust.md` (naming, error handling with `thiserror`/`anyhow`, function ordering, testing)
- No coverage threshold is enforced yet - add one once real tests exist

## Before Every Push

```bash
just fmt-check
just clippy
just test
```

CI runs the same checks (`.github/workflows/ci.yml`) plus `cargo audit`.

## Git Workflow

- Branches: `feature/`, `fix/`, `patch/`, `docs/`
- Fixup commits are fine on draft PRs; rebase (`git rebase -i --autosquash main`) to clean history before marking ready
- Rebase merge only, linear history required (see `~/Documents/ydkadri/claude/git.md`)

## Quick Reference

```bash
just build       # cargo build
just test        # cargo test
just lint        # fmt-check + clippy
just check       # lint + test
```

---

**Last Updated**: 2026-09-25
