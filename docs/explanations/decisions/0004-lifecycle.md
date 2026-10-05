# 0004. The ADR lifecycle and the commands that move it

**Status:** accepted
**Proposed:** 2026-10-05 by Youcef Kadri
**Accepted:** 2026-10-05 by Youcef Kadri

## Context

The tool exists so that a decision is explicitly agreed and then shown to be built. That needs a small, closed set of states, and a command for every change of state, so that nobody edits a status by hand and the record of who decided what, and when, is kept.

## Decision

An ADR is in one of four states:

| State | Means | Complete? |
|---|---|---|
| `proposed` | written, waiting for a decision | no |
| `accepted` | agreed, not yet built | no |
| `implemented` | agreed and built | yes |
| `rejected` | turned down | yes |

"Accepted" means agreed, not built. A later `decider check` will treat `proposed` and `accepted` as incomplete, so that CI can fail while decisions are still open.

Each change of state has its own command, including creating the ADR:

| Command | Moves the ADR | Also records |
|---|---|---|
| `decider propose TITLE` | creates it as `proposed`, then opens it in your editor | a `Proposed` line |
| `decider accept N` | `proposed` to `accepted` | an `Accepted` line |
| `decider reject N --reason TEXT` | `proposed` to `rejected` | a `Rejected` line and the reason, in a `## Rejection` section |
| `decider implement N [--pr REF]... [--note TEXT]` | `accepted` to `implemented` | an `Implemented` line with the references, and the note in an `## Outcome` section |

`propose` opens the new file in the editor from the user's settings, or `EDITOR` (ADR 0002), so the sections can be written straight away. It only does this at a terminal, and only if an editor is set, so scripts and CI never wait for one. If the editor cannot be started, or fails, the ADR has already been created, so `propose` says so and still succeeds.

A rejection must give a reason, because the reason is the only trace a rejected idea leaves. A move that is not in the table is refused, and the error names the ADR's current state and the state the command needs.

Every change of state adds one dated line, `**Accepted:** 2026-10-05 by Ada`, so the record shows who moved the ADR and when. The name is the one in the user's settings (ADR 0002), and the line has no `by` part when there is none.

## Options considered

- **A generic `decider status N STATE` command.** Rejected: it cannot carry what each change needs (`--reason`, `--pr`) and gives a weaker error for a move that is not allowed.
- **No `implemented` state, with accepted meaning complete.** Rejected: nothing could then tell a decision that was agreed and never built from one that was built, which is what the lifecycle is for.
- **A rejection without a reason.** Rejected: see above.
- **Prompt for each section of a new ADR, or for the sections a move changes.** Deferred, tracked in issue #16. Opening the file in an editor is the free-form way to fill it in.
- **A `superseded` state for a decision replaced by a later one.** Deferred, tracked in issue #11. It is the only change that edits two files, and the lifecycle does not need it to work.

## Consequences

- The states a hand-edited file can hold are not checked when the file is written. A later `check` reports a missing or unknown status, and a reviewer should notice one in a diff.
- An ADR in a final state (`implemented` or `rejected`) cannot be moved again. Correcting a mistake means editing the file by hand.
- The file layout these commands edit is in ADR 0005.
