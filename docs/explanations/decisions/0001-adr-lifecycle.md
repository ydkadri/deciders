# 0001. ADR lifecycle, interface and completeness

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

The tool exists so that a decision is explicitly accepted and then shown to be built, and so that CI can fail while decisions are still open. That needs a small, closed set of states, a precise meaning of "complete", and a command for every change of state. Existing project ADRs have no `implemented` state, and their status field mixes an enum with a link (`superseded by NNNN`), so neither question can be answered mechanically.

## Decision

This ADR decides the lifecycle and the command line interface that drives it: the states, which moves between them are allowed, and one verb for each change of state.

### States

An ADR is in exactly one of five states:

| State | Complete? | Can move to |
|---|---|---|
| `proposed` | no | `accepted`, `rejected` |
| `accepted` | no | `implemented`, `superseded` |
| `implemented` | yes | `superseded` |
| `rejected` | yes | (terminal) |
| `superseded` | yes | (terminal) |

"Accepted" means agreed, not built.

### Interface

Each change of state has its own verb, so each verb carries only the arguments it needs and an invalid move gets a specific error. Creating an ADR is itself a change of state, so it has a verb too.

| Command | Effect |
|---|---|
| `decider propose TITLE` | Creates a new ADR in the `proposed` state. |
| `decider accept N` | Moves ADR N from `proposed` to `accepted`. |
| `decider reject N --reason TEXT` | Moves ADR N from `proposed` to `rejected`. The reason is required and is stored in a `## Rejection` section. |
| `decider implement N [--pr REF] [--note TEXT]` | Moves ADR N from `accepted` to `implemented`. References go on the `Implemented` line. A note goes into an `## Outcome` section, so Consequences keeps the original prediction. |
| `decider supersede N --by M` | Moves ADR N from `accepted` or `implemented` to `superseded`, replaced by ADR M. M must exist and be `accepted` or `implemented`. Writes `Superseded by` on N and appends N to `Supersedes` on M, both or neither (ADR 0002). |

`list` and `show` read ADRs and change nothing. `init` and the configuration file are in ADR 0003, and `amend` is in ADR 0006.

### Completeness check

A later command, `decider check`, reads every ADR and reports the ones that are incomplete. It is meant to run in CI, so that the build fails while decisions are still open. It needs no network access and has two modes:

- **Strict (default):** exits non-zero while any ADR is `proposed` or `accepted`.
- **`--allow-accepted`:** exits non-zero only for `proposed`, and reports `accepted` ADRs as warnings.

## Options considered

- **A generic `decider status N <state>` command.** Rejected: it cannot carry per-transition arguments (`--by` for the replacing ADR, `--pr`, `--reason`) and gives weaker errors.
- **A creation command named `new` or `create`, outside the lifecycle verbs.** Rejected: creating is a change of state like the others, and `propose` names the state it produces.
- **No `implemented` state, with accepted meaning complete.** Rejected: the check could then never catch a decision that was agreed and never built, which is the gap it exists to close.
- **A single strict `check`.** Rejected: in a stacked workflow the plan PRs carry `accepted` ADRs by design, so a strict check would fail every PR until the last.
- **Accept ADRs in the PR that implements them.** Rejected: it would change how `plan-stack` agrees ADRs, which is out of scope here.

## Consequences

- In a stacked workflow, ADRs are accepted when the plan is agreed and become `implemented` in the PR that delivers them. CI runs `check --allow-accepted` on PRs and strict `check` on `main`.
- Strict `check` on `main` stays green only if two things hold: a stack implements every ADR it accepts, and it is merged whole. Agreeing ADRs ahead of the stack that builds them, or merging part of a stack, leaves `accepted` ADRs on `main` and turns the strict check red until they are implemented. That is the check doing its job, but it means strict `check` is only switched on for a repo once nothing is accepted ahead of its stack.
- A forgotten `implement` is caught on `main` after the merge, not before it.
- In this repo, the last v0.1.0 PR runs `implement` on ADRs 0001 to 0005 and adds `check --allow-accepted` to CI. Strict `check` on `main` waits for v0.2.0, because ADRs 0006 to 0008 stay `accepted` until then.
- `implemented` can move to `superseded`, because a shipped decision can later be replaced.
- Editing after acceptance is covered by ADR 0006, and marking many ADRs `implemented` at once by ADR 0008.
