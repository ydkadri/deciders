# 0008. Generated index and bulk implement

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

The README index duplicates each ADR's title and status, and a hand-maintained copy drifts. Separately, a repo that adopts the tool with many `accepted` records that are in fact already built would fail `check` on every one of them. Marking each by hand is busywork. This applies to records already in the ADR 0002 format. Records in another header format, such as older project ADRs, must be converted first (issue #2). Both parts are planned for v0.2.0.

## Decision

- The tool owns the README index. It regenerates the table between marker comments after any command that changes an ADR (`propose`, each transition and `amend`), and `check` fails if the index is stale. If the README has no markers, the first regeneration appends them with the table and leaves existing text alone.
- `decider implement --all-accepted` moves every `accepted` ADR to `implemented` in one step. Each one gets today's date (in the local time zone) and the current user on its `Implemented` line, no implementation references, and an `## Outcome` section saying it was marked implemented by bulk update, without the implementation being checked. That keeps a bulk-marked ADR distinguishable from one confirmed individually. It is a one-off adoption path and works only on records already in the ADR 0002 format. Converting other formats is out of scope here and tracked in issue #2.

## Options considered

- **Keep the index by hand.** Rejected: it is the duplicated fact that drifts.
- **Generate the index only on request.** Rejected: an index that can be stale without anyone noticing is the problem. Regenerating on every change, plus the check, closes it.
- **A `--date` option to backdate the bulk update.** Rejected for now: the real date is usually unknown, and an invented date would read as fact.
- **Ask users to run `implement` on each `accepted` ADR.** Rejected: for a repo with dozens of records that is busywork with no information in it.

## Consequences

- Until this lands, the index in this repo is maintained by hand.
- `--all-accepted` asserts that every accepted ADR is built. The user has to know that is true before running it.
