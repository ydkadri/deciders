# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3] - 2026-10-05

### Added
- The lifecycle: `decider propose TITLE`, `accept N`, `reject N --reason TEXT` and `implement N [--pr REF]... [--note TEXT]`. Each change of state adds a dated line with your name to the ADR's header, and a move the lifecycle does not allow is refused (ADR 0004)
- Commands edit only the lines they own: the status, one new header line and one appended section. Nothing else in the file is touched (ADR 0005)
- `propose` opens the new ADR in your editor when run at a terminal: the `editor` setting in your own `config.toml`, or `$EDITOR`. Nothing opens without a terminal or an editor, so scripts and CI never wait (ADR 0002, ADR 0004)
- `chrono` for the local date, with only its clock feature

## [0.0.2] - 2026-10-05

### Added
- `decider init [--dir PATH] [--user NAME] [--force]` sets up a repository and your own settings, and is safe to run again. At a terminal it asks for the directory and your name, showing a default for each. `--force` replaces settings that already exist: on its own it asks for both again, and `--dir` or `--user` replaces only that one. It writes `.decider.toml`, the ADR directory and the ADR template, and records your name in a per-user file that is never committed (ADR 0002)
- The store port and the file adapter that implements it (ADR 0003)
- Command reference and getting-started tutorial

## [0.0.1] - 2026-10-05

### Added
- Project setup from the Rust template: crate `decider-adr` and a binary `decider` that prints its version
- `CLAUDE.md`, `@ydkadri` as CODEOWNER, and three CI workflows (checks, secrets, scheduled vulnerability scan)
- ADR 0001 on the crate name, versions and distribution, and the ADR template, `0000-template.md`
