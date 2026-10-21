---
paths:
  - "tests/**"
  - "src/**/*.rs"
  - "benches/**"
  - "fuzz/**"
---

# Tests

- **Assert on rendered strings, not tree shape** (`AGENTS.md` rule 11). A test that reads
  `element.children()[0].attributes()[1]` locks in an implementation. A test that asserts
  the markup locks in the contract, which is what lets the internals be rewritten without
  touching the tests.
- One behaviour per test, named as a sentence: `an_unsafe_tag_name_renders_nothing`.
- **A bug fix starts with the failing test**, written from the hostile or failing input.
  Watch it fail, fix, then switch the fix off by hand (`if false && …`), watch it fail
  again, restore. That is the only check that catches a test that asserts nothing (L21).
- After adding a test, confirm the test count rose and that the test runs by name
  (`cargo test <name>`). Do not write "covered by `x`" until `grep -rF "fn x"` finds it.
- Keep the hostile input in the test. Fuzz targets in `fuzz/fuzz_targets/` cover the
  escapers, the JSON script escaper and the URL policy; add one when you add a parser.
- A test that touches the filesystem uses a unique directory (pid in the name) and never
  calls a destructive function on the real working directory. Destructive functions are
  tested through their pure guard (L5).
- A test that must not run in parallel with others (the global allocator counter in
  `tests/allocations.rs`) is a single `#[test]` in its own file.
- Property tests with hostile input live in `tests/invariants.rs`; the 100,000-level
  nesting tests in `tests/performance.rs` compare with `assert!(a == b)`, never
  `assert_eq!`.
- Keep the old implementation as an oracle while rewriting a path (L7).
- Tests may `unwrap`/`expect` (`clippy.toml`); library code may not.
- The golden fixtures are a test too: `cargo test --test golden`. See
  `golden-fixtures.md`.
