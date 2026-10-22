---
paths:
  - "docs/**"
  - ".claude/**"
  - "*.md"
  - "scripts/check-knowledge-base.sh"
---

# Documentation and the knowledge base

- **Say in the docs only what you ran** (L8). A guide's code is compiled as a doctest or
  run by a script, and a claim about behaviour has a test behind it. `README.md` and
  `SECURITY.md` change in the same PR as the behaviour they describe.
- The symptom-to-fix tables for people who *use* Molt live in `docs/pitfalls.md`, which ships in
  the crate; the skill's `pitfalls.md` keeps only the contributor rows. Fix a row in one place.
  `docs/capabilities.md` has a *Since* column: add a row, with its version, for every new
  public API. `./scripts/check-knowledge-base.sh` fails if these pages leave the package (#135).
  The official skill (`micheltlutz/molt-framework-skill`) bundles snapshots of both pages: after
  a release that changes them, run its `scripts/sync-references.sh <tag>` and raise its version.
- `docs/tag-catalog.md` is generated. Run `./scripts/generate-tag-catalog.sh`; never edit
  it.
- `docs/history/` is where the Swift origin lives. Nothing in `src/`, `tests/` or `scripts/`
  refers to it, and a new mention of Winged-Swift goes there and nowhere else.
- `.claude/skills/molt/references/lessons-learned.md` is the most useful file in the
  repository. Every lesson is *symptom → cause → rule → guard*, and the **Guard** line names
  the tests or files that enforce it. `./scripts/check-knowledge-base.sh` fails when a
  guard no longer exists, when a relative link in `.claude/` does not resolve, when a
  rule's `paths:` matches no tracked file, or when a rule, command or agent is missing from
  the tables in `AGENTS.md`. When you learn something the hard way, add a lesson with its
  guard (`/lesson`); when a guard is deleted on purpose, delete or rewrite the lesson.
- A decision a future agent could reasonably undo goes in
  [`decision-log.md`](../skills/molt/references/decision-log.md): decision, reason, what
  would change it. Rejected options are kept.
- Rule files carry only `paths:` in their frontmatter; a rule without it loads in every
  session, so keep those two (`code-style.md`, `git-workflow.md`) short.
- Lesson numbers in these rules (`L6`, `L21`, …) point at `lessons-learned.md`; rule numbers
  (`rule 11`) point at the twelve-line list in `AGENTS.md`.
