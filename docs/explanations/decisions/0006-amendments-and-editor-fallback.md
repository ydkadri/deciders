# 0006. Amendments and the editor fallback

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

Accepted records do get edited afterwards. In practice they get corrected in place, and observed results get appended to Consequences after the work lands. Without a rule, a reader cannot tell what was decided from what was revised later. Separately, some transitions need free text, and typing it as a flag is awkward for anything longer than a sentence. This is planned for v0.2.0.

## Decision

After acceptance, Context, Decision and Options considered change only through `decider amend N --note "..."`, which appends a dated entry to an `## Amendments` section. Observed results go through `implement --note` (ADR 0001). The tool does not police hand edits, so git review covers that.

Required free text (`reject --reason`, `amend --note`) comes from the flag, or from an editor when the flag is missing and a terminal is attached:

- The editor is the one named by `$EDITOR`. If `$EDITOR` is unset or empty, the missing flag is an error that names both the flag and `$EDITOR`. Overriding the editor with a setting is planned, together with a `config` command to read and write settings.
- The buffer opens with an HTML comment that is stripped afterwards. A `#` line is not stripped, because `#` is a Markdown heading.
- Text is empty if it holds nothing but whitespace and line breaks. An empty `--reason` or `--note` on `reject` or `amend` is an error, and an empty editor message aborts without changing the file. The optional `implement --note` is not required, so an empty one is treated as none.
- With no terminal, a missing flag is an error that names it, so skills and CI never hang.
- `implement --note` is optional and never opens an editor. `propose` does not open one unless given `--edit`, which is an error when no terminal is attached.

## Options considered

- **No policy on editing accepted ADRs.** Rejected: it hides the difference between what was decided and what was revised later.
- **Record a checksum of the body at acceptance and fail `check` on drift.** Rejected. The files are plain text in the repo, so nothing can prevent edits, only make them visible. A checksum in `.decider.toml` would conflict whenever two branches touch an ADR, and manual edits are often legitimate. Git review covers it, and `check` already catches a hand-edited status because each status requires its stage fields (ADR 0002).
- **Git's `#` comment convention for the editor buffer.** Rejected: it would strip real Markdown headings.

## Consequences

- Until this lands, `reject` needs `--reason` on the command line.
- Amendments give later readers a dated trail without rewriting the original decision.
