# 0005. ADR file format and numbering

**Status:** accepted
**Proposed:** 2026-10-05 by Youcef Kadri
**Accepted:** 2026-10-05 by Youcef Kadri

## Context

The commands in ADR 0004 change a few lines of a file that people also write by hand. The format has to be simple enough to edit with a text editor and to change with a text edit, without the tool having to understand the whole document.

## Decision

Each ADR is a Markdown file named `NNNN-slug.md` in the ADR directory (ADR 0002).

- **Number.** Four or more digits at the start of the filename. Number 0000 belongs to the template. A new ADR takes the highest number in the directory plus one, so gaps are left alone and the first ADR is 0001.
- **Slug.** The title in lower case, keeping letters and digits and joining the words with `-`, cut at 60 characters. A title with no letters or digits cannot name a file and is refused, as is one that is empty or spans more than one line.
- **Header.** The lines between the title and the first `## ` heading. The title is `# NNNN. Title`, then a blank line, then one bold-line field per line: `**Status:**` with one word from ADR 0004, and one line for each change of state, in the order they happened.

```markdown
# 0007. Use Postgres

**Status:** implemented
**Proposed:** 2026-10-01 by Ada Lovelace
**Accepted:** 2026-10-02 by Ada Lovelace
**Implemented:** 2026-10-05 by Ada Lovelace (#12, #13)
```

- **Dates** are `YYYY-MM-DD` in the local time zone. References, if any, are in brackets at the end of the `Implemented` line.
- **Sections.** A new ADR takes its sections from the template, `0000-template.md` in the ADR directory, which `decider init` writes and anyone can edit (if the file is missing, the template built into the tool is used). The header of the template is not used: the tool writes the header itself. `reject` adds a `## Rejection` section at the end of the file, and `implement` adds `## Outcome` when it is given a note.

A command edits only the lines it owns: it replaces the value on the `**Status:**` line, adds its own line after the last header line, and appends its own section. Everything else is left exactly as written.

## Options considered

- **YAML frontmatter.** Rejected: the header would show as raw text when the file is rendered as Markdown, and the commands would have to parse and write YAML.
- **Parse the whole ADR into a model and write it back.** Rejected: it needs a strict parser, and a writer that proves what it writes reads back the same, which is most of the code of a tool that changes a few lines.
- **Check that a file is well formed before every edit.** Rejected: a human or agent reviewing the change can see a malformed header, and a later `check` can report one. The commands assume the lines they own are where they expect.

## Consequences

- A hand-edited header can confuse an edit, for example two `**Status:**` lines (the first is used). The commands report a missing or unknown status, and do not try to repair anything else.
- Two branches that each propose an ADR take the same number. Nothing stops this when they are merged, and a command that is then given that number says it matches two files.
- The template controls the sections of every new ADR, so changing it changes the shape of the records that follow.
