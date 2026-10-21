---
description: Add a lesson to the knowledge base (symptom → cause → rule → guard) and verify its guard resolves
argument-hint: <short title>
allowed-tools: Bash(./scripts/check-knowledge-base.sh)
---

Record this lesson in `.claude/skills/molt/references/lessons-learned.md`: $ARGUMENTS

A lesson exists so the next agent does not pay for the same mistake. Write it from what
happened in this session, not from what could happen.

1. Read the last two lessons in the file for the shape, and take the next free `L<n>`.
2. Pick the section: *Batch D — Security hardening* for security and correctness,
   *Batch C — Performance* for measurements, *Process lessons* for tooling, git, scripts
   and review process.
3. Write it as **symptom → cause → rule → guard**:
   - the heading is `### L<n>. <the rule in one sentence>`
   - the first paragraph is what was seen and where, with the issue or PR number
   - each `**Rule.**` bullet is one instruction a future agent can follow
   - the `**Guard.**` line names, in backticks, the tests (by function name) or files (by
     path) that fail if the lesson is forgotten. Every name must exist now. If nothing
     guards it, write the test first: a lesson without a guard is an opinion.
4. If the lesson changes how a rule in `.claude/rules/` should read, update that rule and
   cite the lesson number from it.
5. Run `./scripts/check-knowledge-base.sh` and quote its last line. It must say every guard
   resolves.

Keep the lesson under 25 lines. Add it to the *Read this when* table of `SKILL.md` only if
it is the first lesson on its topic.
