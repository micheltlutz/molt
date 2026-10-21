---
description: Run ./scripts/verify.sh (the superset of CI) and report every failure, not only the first
allowed-tools: Bash(./scripts/verify.sh), Bash(./scripts/verify.sh *)
---

Run the repository's verification script and report what it says.

1. Run `./scripts/verify.sh` from the repository root. It takes a few minutes: three
   builds, the tests, lint, the tag-catalog check, the golden fixtures, an end-to-end smoke
   crate and the knowledge-base guard check. If `cargo` is not on `PATH`, the *Commands*
   section of `AGENTS.md` says where the toolchain lives on this machine.
2. Read the whole output. The script accumulates failures instead of stopping at the first,
   so one run lists every problem.
3. Report each `FAIL` line with the lines under it, in the order they appear. Quote the
   compiler or test output in a code block; do not paraphrase an error message.
4. Two outcomes are environmental and are reported as such, never hidden:
   `cargo-deny is not installed` (CI runs it) and the `skip` for `wasm-pack`.
5. Do not fix anything in the same turn as the report unless the user asked for the fix.
   A fix that makes the script pass is not done until the script has been run again.

Say "verify.sh passed" only when the last line is `All checks passed.`
