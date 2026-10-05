# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
