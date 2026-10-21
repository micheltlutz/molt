---
description: Run the golden-fixture test and explain any diff; never regenerates the fixtures
allowed-tools: Bash(cargo test --test golden), Bash(git diff *)
---

Run `cargo test --test golden` and report.

- If it passes, say so in one line.
- If it fails, show the diff between the expected fixture and the actual output for each
  failing file, as the test prints it. Then say which source change produced the
  difference (read `git diff` for the branch) and whether it looks intended.

Do **not** run `MOLT_UPDATE_FIXTURES=1`. Regenerating is an "ask every time" action
(`.claude/rules/golden-fixtures.md`). If the change is intended, tell the user the three
things that make it complete, a test asserting the new string, a `CHANGELOG.md` entry
under *Security* or *Breaking*, and a row in the *Output changes* table of `MIGRATION.md`,
and wait for them to ask for the regeneration.
