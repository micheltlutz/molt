---
description: Audit Molt's output surface for injection, path traversal and unescaped paths with the security-auditor agent
argument-hint: [area: escaping | urls | names | xml | ssg | wasm | all]
---

Audit the security of: $ARGUMENTS (when empty, the whole surface).

1. Dispatch the `security-auditor` agent (`.claude/agents/security-auditor.md`) with the
   area. Its checklist is the surface table in
   `.claude/skills/molt/references/security-owasp.md`: for each row it checks the "made safe
   by" claim against the code and against a hostile input.
2. While it runs, list the escape hatches yourself:
   `grep -rn -e 'raw_text' -e 'Node::raw' -e 'Attribute::raw' -e 'trusted_url' src examples`
   and check that each call site sits under a comment saying why its input is trusted.
3. For every finding the agent returns, confirm it by reading the code and, when it claims
   an input gets through, by rendering that input in a scratch test and reading the string.
   A finding without a rendered string is a lead, not a result.
4. Report, most severe first: the surface-table row, the input, the rendered output, and
   whether a test already covers it. Then the rows that were checked and hold, one line
   each.

Do not fix anything in this turn. A fix goes through `/fix-bug`, so it gets its failing
test, its `SECURITY.md` update and its `MIGRATION.md` row.
