# 0001. Crate name, versions and distribution

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

The command should be called `decider`, but the `decider` crate on crates.io is an unrelated A/B-testing library. The tool is a single binary, and for now it is only run locally.

## Decision

- The crate is `decider-adr`, the installed binary is `decider` (`[[bin]] name = "decider"`), and the repository is `deciders`.
- The project follows semver, and each milestone becomes a version. Inside a milestone, each PR bumps the patch version starting from the previous milestone's version (0.0.0 before v0.1.0, so the first PR is 0.0.1), and the last PR sets the milestone version. Only milestone versions are tagged, by hand until there is a release workflow.
- For now it is installed locally with `cargo install --path .`. Publishing to crates.io is tracked in issue #1.
- CI runs on GitHub Actions, and `@ydkadri` is the CODEOWNER and the only reviewer.

## Options considered

- **Prebuilt binaries on GitHub Releases, built on tag.** Deferred until there is something for other repositories to install.
- **Publish to crates.io now.** Deferred: it adds a release surface that is not needed yet.
- **A different binary name.** Rejected: `decider` is the name wanted, and the crate and binary names are independent.

## Consequences

- Nobody can install the tool from crates.io until it is published. It can still be built from a checkout.
- The crate name is reserved by nothing until it is first published, so it could be taken in the meantime.
