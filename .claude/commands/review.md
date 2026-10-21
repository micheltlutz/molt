---
description: Review the branch's changes with the code-reviewer agent, plus api-reviewer and security-auditor when the diff warrants them
argument-hint: [base branch, default develop]
allowed-tools: Bash(git status *), Bash(git diff *), Bash(git log *)
---

Review the changes on this branch. Base branch: $ARGUMENTS (when empty, `develop`).

Working tree now:
!`git status --short`

Commits not on develop:
!`git log --oneline develop..HEAD`

1. Collect the diff: the committed changes against the base (`git diff <base>...HEAD`) plus
   anything uncommitted (`git diff` and `git diff --cached`). Review the union.
2. Dispatch the `code-reviewer` agent (`.claude/agents/code-reviewer.md`) with the diff and
   the list of changed files. Give it the code, not your conclusions.
3. In the same message, also dispatch `api-reviewer` (`.claude/agents/api-reviewer.md`) if
   the diff adds, removes or changes a `pub` item, a feature flag, `src/prelude.rs`,
   `src/wasm.rs`, `MIGRATION.md` or `CHANGELOG.md`; and `security-auditor`
   (`.claude/agents/security-auditor.md`) if it touches `src/core/escape.rs`,
   `src/core/url_policy.rs`, `src/core/attribute.rs`, `src/core/node.rs`, `src/core/tags.rs`,
   `src/ssg.rs`, `src/seo.rs`, `src/sitemap.rs` or `src/feed.rs`.
4. Verify every BLOCK the agents return by reading the code yourself before reporting it.
   Findings come back in a confident voice whether or not they are right.
5. Report: the verdict first (BLOCK, WARN or APPROVE), then the findings most severe
   first, each with `file:line`, what is wrong, and the input or scenario that shows it.
   End with what the review did not cover.

Do not fix anything. The review is the deliverable; the user decides what to act on.
