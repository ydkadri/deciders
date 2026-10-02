# 0002. ADR file format

**Status:** accepted
**Proposed:** 2026-10-01 by Youcef Kadri
**Accepted:** 2026-10-01 by Youcef Kadri

## Context

The tool must read and write every field in the lifecycle (ADR 0001) without corrupting hand-written prose. The existing project ADRs this tool has to work with use a bold-line header (`**Status:**`, `**Date:**`) and render as plain text in Sphinx. Several facts in those records live in more than one place: the number is in the filename and the H1, and the status is in the file and in a hand-maintained README index.

## Decision

Each ADR is `NNNN-short-title.md` and starts with an H1 (`# NNNN. Title`) followed by a bold-line header block:

```markdown
**Status:** implemented
**Proposed:** 2026-07-29 by Youcef Kadri
**Accepted:** 2026-07-30 by Youcef Kadri
**Implemented:** 2026-08-02 by Youcef Kadri (#412)
```

- Status is one word, populated from an `enum` of the five states in ADR 0001. The replacement link is a separate field: `**Superseded by:** NNNN` on the old ADR and `**Supersedes:** NNNN, NNNN` on the new one, written together or not at all. An ADR is superseded once, so `Superseded by` holds one number. A new ADR can replace several, so `Supersedes` is a comma-separated list and `supersede` appends to it.
- Each stage adds its own line: `Proposed`, `Accepted`, `Rejected`, `Implemented` and `Superseded`, each with a date, and an author where it applies. Implementation refs are optional and go in brackets at the end of the `Implemented` line, after the author if there is one, comma separated: `2026-08-02 by Youcef Kadri (#412, #413)` or `2026-08-02 (#412)`. Only that line has refs, so a name may not contain a parenthesis (ADR 0003). A reference must not be empty or padded with whitespace, or contain a line break, a parenthesis or a comma, so that it reads back as the one reference that was written. `check` requires these stage lines for each status, and that the dates of those present are in order: `proposed` needs Proposed. `accepted` needs Proposed and Accepted. `implemented` needs Proposed, Accepted and Implemented. `rejected` needs Proposed and Rejected. `superseded` needs Proposed, Accepted and Superseded, and may also have Implemented.
- The header is parsed strictly, and only within the block between the H1 and the first H2. Unknown header lines are preserved on write, which lets later ADRs add fields such as `**Depends on:**` (ADR 0007).
- Required sections are Context, Decision, Options considered and Consequences, and `check` fails if one is missing. `## Outcome` and `## Rejection` are added by the tool when needed (ADR 0001). ADR 0006 adds `## Amendments`.
- The filename owns the number. `check` fails if the H1 disagrees, and fails on duplicate numbers. Gaps are tolerated and the tool never renumbers.
- Number 0000 is reserved for the template (`0000-template.md`). `list`, `show` and `check` skip it, and so does the index once ADR 0008 generates it, and `propose` never allocates it. `propose` allocates the highest existing number plus one.
- The README index is maintained by hand. ADR 0008 makes the tool own it.

## Options considered

- **YAML frontmatter.** Rejected: more standard to parse, but existing records would need migrating, and the header shows as raw text in Sphinx.
- **Parse `Status` out of free prose.** Rejected: it is the current problem.
- **The tool infers the number from the H1.** Rejected: filenames are what people and `git` see first, so they must be authoritative.

## Consequences

- Existing project ADRs often use a different header (`**Date:**`, and `superseded by NNNN` as a status) and will not parse until they are converted. ADR 0008 covers only records already in this format, and converting legacy ones is tracked in issue #2.
- The parser is a strict, hand-written one, so it needs thorough tests for malformed headers.
- Because `propose` allocates the highest number plus one, two branches that each propose an ADR can pick the same number. `check` reports the duplicate after the merge, and the fix is a rename.
