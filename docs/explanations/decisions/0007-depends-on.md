# 0007. ADRs can declare dependencies

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

A decision often rests on earlier ones. If an earlier decision is later rejected or superseded, the dependent ADR is resting on something that no longer holds, and nothing says so. The build skills will eventually want to check that the decisions a change relies on are accepted, and they need one agreed place to read that from. The field is planned for v0.2.0.

## Decision

An ADR may have an optional `**Depends on:**` header line holding a comma-separated list of ADR numbers, for example `**Depends on:** 0003, 0005`. `check` fails if a referenced ADR does not exist or is `rejected` or `superseded`, unless the dependent ADR is itself `rejected` or `superseded`. Updating the line after a dependency changes is a header edit, not an amendment (ADR 0006).

## Options considered

- **Leave it out until the skills need it.** Rejected: the field is optional and costs nothing for ADRs that do not use it, and the check catches a real problem (an ADR resting on a decision that no longer holds) without waiting for the skills.

## Consequences

- An ADR that depends on a decision that is later superseded fails `check` until the author reviews it and updates the line.
- The skills integration itself is out of scope. This only fixes the file format for it.
