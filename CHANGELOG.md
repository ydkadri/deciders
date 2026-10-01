# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2] - 2026-10-01

### Added
- ADR model: `Status` with the lifecycle rules, `AdrNumber`, `Date`, and a strict reader and writer for the bold-line header and the ADR document. Writing refuses anything that would not read back unchanged, and writes the title and header with the file's first line ending (ADR 0001, ADR 0002)

### Changed
- The binary's default log level is now `info`, matching `.env.example`

### Removed
- The template's placeholder `greet` function and its tests

## [0.0.1] - 2026-10-01

### Added
- Project setup from the Rust template: crate `decider-adr`, binary `decider`
- `CLAUDE.md` and `@ydkadri` as CODEOWNER
- ADRs 0001 to 0008 covering the lifecycle and command verbs, file format, configuration in two files, hexagonal structure, distribution, amendments, dependencies, the generated index and bulk implement
