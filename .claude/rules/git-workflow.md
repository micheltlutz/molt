# Git and pull-request workflow

- Branch from `develop`, pull request against `develop`. `main` receives releases only.
  Branch names: `feature/…`, `fix/…`, `docs/…`, `release/<version>`.
- Commit subjects start with the kind of change: `Add:`, `Fix:`, `Change:`, `Update:`,
  `Docs:`, `Test:`, `Refactor:`, `Perf:`. One logical change per commit. The body says why
  and, for performance, the numbers. Issues go in the body (`Refs #12`, `Fixes #12`), not
  the subject.
- A PR body states what changed, what was measured, what it deliberately does not do, where
  it diverges from the issue and why, and whether the golden fixtures changed. A breaking
  change is labelled `breaking` and has its `MIGRATION.md` entry in the same PR.
- `CHANGELOG.md` is updated under `## [Unreleased]` in the same PR as the change.
- Run `./scripts/verify.sh` before reporting a change as done, and read its output. If it
  fails only because `cargo-deny` is not installed, say so; CI runs it.
- Ask before anything outward-facing, and ask for that action specifically: push, open a
  PR, merge, tag, publish, delete a branch, force-push, regenerate fixtures
  (`MOLT_UPDATE_FIXTURES=1`), change CI permissions or secrets. Approval of a plan covers the
  actions it names, not new ones. Never `--no-verify`, never hand-edit `tests/fixtures/`,
  never put a token in a file, log or PR body.
- The full autonomy table is in
  [`agent-governance.md`](../skills/molt/references/agent-governance.md).
