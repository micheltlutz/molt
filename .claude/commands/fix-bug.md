---
description: Fix a bug test-first — reproduce with the hostile input, watch it fail, fix at write time, prove the test can fail
argument-hint: <issue number or description>
---

Fix this bug: $ARGUMENTS

If the argument is an issue number, read it first with `gh issue view <n>`. The procedure
is the bug recipe in `.claude/rules/testing.md` and lesson L21: every step produces
evidence, and the report at the end lists it.

1. **Reproduce.** Write the smallest test that shows the bug, using the exact hostile or
   failing input, and assert on the **rendered string**, never on tree shape. Put it next
   to the related tests: the `#[cfg(test)]` module of the file for one function,
   `tests/invariants.rs` for a property, `tests/golden.rs` only when a fixture is involved.
   Run it by name and keep the failure output.
2. **Find the place every construction path goes through** (builder, `html!`, `Document`,
   the wasm bindings) and fix it there, so no path bypasses the fix. Drop, do not repair.
3. **Run the test again**, then the whole suite. If `cargo test --test golden` now fails,
   the fix changed output for valid input: stop, show the diff, and ask whether that is
   intended. Never regenerate the fixtures on your own.
4. **Prove the test can fail.** Switch the fix off by hand (a one-line mutation such as
   `if false && …`), run the test, see it fail, restore the fix. Say in the report that
   you did this.
5. **Count.** Confirm the test count rose and that `grep -rF "fn <test name>"` finds it.
6. **Record.** `CHANGELOG.md` under `## [Unreleased]`: *Fixed*, or *Security* plus a row in
   the *Output changes* table of `MIGRATION.md` when output changed. A security bug also
   updates its row in the surface table of
   `.claude/skills/molt/references/security-owasp.md` and `SECURITY.md`. If the bug taught
   something a future agent would otherwise repeat, add a lesson with `/lesson`.
7. `./scripts/verify.sh`.

The report names the test, shows the failing run and the passing run, and states what was
not verified.
