# 0005. Crate name, binary name and distribution

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

The command should be called `decider`, but the `decider` crate on crates.io is an unrelated A/B-testing library. CI in other repos will eventually need a quick way to get the binary. For the MVP the tool is only run locally.

## Decision

- The crate is `decider-adr`, the installed binary is `decider` (`[[bin]] name = "decider"`), and the repository is `deciders`.
- The project follows semver, and each milestone becomes a version. Inside a milestone, each PR bumps the patch version starting from the previous milestone's version (0.0.0 before v0.1.0, so the first PR is 0.0.1) and the last PR sets the milestone version. In every milestone the patch number counts PRs rather than signalling a fix, which departs from the usual "patch for fixes, minor for a new capability" rule. The stack is the release unit, and only milestone versions are tagged. Nothing is tagged before v0.1.0, and until the release workflow exists tags are created by hand.
- For the MVP it is installed locally with `cargo install --path .`. Publishing to crates.io is deferred to v0.3.0 and tracked in issue #1.
- CI runs on GitHub Actions, as this is a personal project. `@ydkadri` is the CODEOWNER and the only reviewer.

## Options considered

- **Prebuilt binaries on GitHub Releases, built on tag.** Deferred to v0.3.0 with the release workflow. It is the likely best fit for CI, where compiling from source takes minutes.
- **Publish to crates.io now.** Deferred: it adds a release surface the MVP does not need.
- **A different binary name.** Rejected: `decider` is the name wanted, and the crate and binary names are independent.

## Consequences

- Until v0.3.0 the tool is installed from a local checkout, so CI in other repos cannot fetch it yet.
- The crate name is reserved by nothing until it is first published, so it could be taken in the meantime.
