# Lessons learned

What the 1.0 hardening batches taught, written so the next agent does not pay for it again.
Each lesson is **symptom → cause → rule → guard**. The guard is the test or file that fails if
the lesson is forgotten; `scripts/check-knowledge-base.sh` verifies the guards still exist.

Issue numbers are `micheltlutz/molt` issues; PR numbers are the pull requests that closed them.

Batch A = correctness and security (S1–S6, B1–B5). Batch B = the public API before 1.0 (A1–A10,
W1–W2). Batch C = performance, measured (P0 onward).

---

## Batch A — Security and correctness

### L1. One `replace` is not a sanitizer

- **Symptom.** `Node::comment("x--->")` produced `x- -->`, which still closed the comment, and
  everything after it became markup. (#2, PR #44)
- **Cause.** `replace("--", "- -")` makes one non-overlapping pass. Overlapping matches (`---`)
  and a trailing `-` survive it.
- **Rule.** A neutraliser must be proved against the *output*, not the pattern. Walk the
  characters and fix the invariant ("no two adjacent hyphens, none at the end"), then test the
  hostile inputs: `--->`, `--!>`, a trailing `-`, runs of hyphens.
- **Guard.** `a_comment_survives_runs_of_hyphens_and_bang_closers`

### L2. Names are an injection surface, not only values

- **Symptom.** `Element::new("img src=x onerror=alert(1)")` and `"div><script>…"` were written
  verbatim between `<` and `>`, also reachable from JavaScript through `new WElement(str)`.
  Attribute names were already checked; tag names were not. (#3, PR #45)
- **Cause.** Validation was applied where the author thought of it, not at every place a
  string becomes markup.
- **Rule.** List every place a caller's string reaches the output (value, attribute name, tag
  name, comment body, indent string, XML text, file path) and decide for each how it is made
  safe. Validate **at write time** so no construction path can bypass it. Drop the whole
  offending element rather than guess a repair; drop it **before** writing the indent so
  pretty mode leaves no blank line.
- **Guard.** `an_unsafe_tag_name_renders_nothing`, `custom_and_namespaced_tags_still_render`

### L3. The same character is safe or fatal depending on its ancestor

- **Symptom.** `noscript().child(img().attr("alt", "</noscript><img src=x onerror=…>"))` ended
  the `<noscript>` early. `escape_attribute` leaves `<` `>` alone because they cannot end a
  quoted value, and the Swift fixtures depend on that. (#5, PR #46)
- **Cause.** Under a raw-text ancestor (`noscript`, `iframe`, `xmp`, `noembed`, `noframes`,
  `script`, `style`, `plaintext`) the parser is not reading attributes at all; the first
  `</tag>` ends the element wherever it appears.
- **Rule.** When a parity constraint forbids the safe escape everywhere, make it **contextual**:
  the writer keeps a counter of enclosing raw-text elements and escapes `<` `>` only there.
  Do not widen `escape_attribute`; do record the divergence from Winged-Swift in `SECURITY.md`.
  An element's *own* attributes are read before raw-text mode starts and stay plain.
- **Guard.** `angle_brackets_in_attributes_are_escaped_only_under_raw_text_parents`,
  `the_raw_text_escape_covers_deep_descendants_and_ends_with_the_parent`

### L4. A `pub` helper taking `&str` must escape, whatever the original did

- **Symptom.** `meta_name("a\"><script>…", "v")` broke out of its attribute. Winged-Swift
  inserted those names unescaped because its callers pass literals. (#4, PR #47)
- **Rule.** Porting parity is about *output for valid input*. A `pub` function is called by
  people who are not the original author; assume `&str` is hostile. Legitimate values are
  unchanged by escaping, so parity costs nothing.
- **Guard.** `a_hostile_meta_name_cannot_break_out_of_its_attribute`

### L5. Filesystem paths: five traps in one module

(#6, #7, #8, PR #48)

1. `Path::new(".").parent()` is `Some("")`, not `None`. A root check built on `parent()` let
   `clean(".")` delete the working directory.
2. `Path::with_extension` **replaces** the extension. `index.html` and `index.xml` both became
   `index.winged-tmp`, and two pages written in parallel shared one file.
3. A lexical check (`..`, absolute) does not see a symlink already inside the output directory.
4. `C:foo` has no `..` and is not absolute on Unix-style checks, but `join` with a Windows
   prefix replaces the base.
5. `delete-then-copy` leaves nothing if the copy fails.

**Rules.** Accept only `Component::Normal` and `CurDir`; require a file name; refuse existing
symlink components with `symlink_metadata`. Temporary files are `.{name}.molt-{pid}-{n}.tmp`
opened with `create_new(true)`. Atomic = write beside, then `rename`. Keep the destructive
guard a **pure function** (`guard_destructive_path` only reads the disk) so tests can call it
with `"."`, the cwd and `$HOME` without any risk. **Never** write a test that calls
`clean(".")` against the real working directory. State the residual race honestly: a symlink
created *during* the build is not closed.

- **Guard.** `the_clean_guard_refuses_the_working_directory_and_what_contains_it`,
  `pages_with_the_same_stem_do_not_share_a_temporary_file`,
  `a_symlink_inside_the_output_directory_is_not_followed`,
  `a_failed_asset_copy_keeps_the_previous_file`, `a_path_that_does_not_name_a_file_is_rejected`

### L6. A stack overflow is an abort, and `derive` recurses

- **Symptom.** The renderer handled a 100,000-level tree, but `Document::render` aborted the
  process on the same tree, as did `accessibility::audit`, `.clone()` and `assert_eq!`.
  (#9, #10, #11, PR #49)
- **Cause.** Rust's stack overflow is `SIGABRT`, not a panic: `catch_unwind` cannot save you.
  `#[derive(Clone, PartialEq, Debug, Hash)]` on a recursive type emits one frame per level.
  `Document::root()` cloned head and body before writing.
- **Rule.** Any function that visits every node uses an explicit `Vec` stack (see `Step` in
  `src/core/node.rs`, `clone_tree`/`trees_equal`/`hash_tree` in `src/core/element.rs`). `Drop`
  must be iterative too (`impl Drop for Element`). Write `Clone`, `PartialEq`, `Eq`, `Hash`
  by hand for the tree types. `Debug` is the one derive that stays recursive; document it.
- **Test rule.** In a deep-tree test use `assert!(a == b)`, **not** `assert_eq!`: on failure
  `assert_eq!` formats both trees with the recursive `Debug` and overflows in the very case
  under test. Clippy's `manual_assert_eq` objects; allow it with a comment saying why.
- **Guard.** `cloning_and_comparing_a_very_deep_tree_does_not_abort`,
  `cloning_and_comparing_deep_fragments_does_not_abort`,
  `a_document_with_a_very_deep_body_renders_in_both_modes`,
  `auditing_a_very_deep_tree_does_not_abort`, `hashing_a_very_deep_tree_does_not_abort`,
  `displaying_a_very_deep_tree_does_not_abort`

### L7. Replace a clone with a borrow, then prove the bytes did not move

- **Symptom/fix.** `Document::render_with` now writes `<html>` around *borrowed* head and body
  (`write_document_root` in `src/core/node.rs`) instead of cloning them.
- **Rule.** When you rewrite a code path for safety or speed, keep the old path alive *in the
  test* as the oracle and compare outputs over a matrix (here 7 documents × 4 option sets,
  including a whitespace-sensitive body, an invalid head tag and a self-closing body).
  "The golden fixtures still pass" is necessary and not sufficient; they exercise few shapes.
- **Guard.** `rendering_matches_the_cloned_root_tree_byte_for_byte`

### L8. Say in the docs only what you ran

Several claims in this repo were wrong until someone checked: `SECURITY.md` said "a documented
256 levels" while the code and tests guaranteed 100,000, and the `wasm.rs` docs named a
function `wElement` that does not exist. (#12) Run the example, read the generated `.d.ts`,
execute the claim. Doc examples are compiled by `cargo test --doc`; JavaScript claims are not,
so run them (see L12).

---

## Batch B — The public API before 1.0

Semver debt is cheapest before 1.0. Each item below is a decision you cannot take back later.

### L9. Hygiene checklist for a `pub` item

(A1, A4, A6, A9 — PRs #50, #51)

- **`#[non_exhaustive]`** on every `pub enum` you may extend and every `pub`-field struct you
  may add a field to. First make sure each has a constructor or builder, because the attribute
  forbids struct literals and `..Default::default()` outside the crate. Inside the crate,
  literals still work.
- **No array length in a public type.** `pub const VOID_TAGS: [&str; 13]` made "add a tag" a
  semver break; `&[&str]` does not.
- **Do not give two traits a method of the same name.** `Layout::render` collided with
  `Render::render`; a type implementing both could not call either. It is `wrap` now.
- **Return `Cow<'_, str>` from escapers.** Clean input comes back borrowed; changing the
  return type later is breaking.
- **Closed set of strings → enum.** `changefreq: Option<String>` accepted `"weakly"`, which
  crawlers ignore silently. `ChangeFreq` has `FromStr` and `Display`. Numeric ranges are
  clamped in the builder *and* re-checked at render time, because the field is `pub`.
- **Two paths to one item are both API.** `molt::core::Element` and
  `molt::core::element::Element`. Say so in the module docs.
- **Guard.** `a_change_frequency_round_trips_through_text`,
  `a_priority_outside_the_range_is_clamped_and_a_non_number_is_left_out`,
  `clean_input_is_returned_without_allocating`

### L10. Errors: an enum with a source, convertible to `io::Error`

(A7, PR #53)

`io::Error::other(format!(…))` flattened every failure into a string. `SsgError` has
`InvalidPath`, `UnsafeCleanTarget`, `Io { path, source }` and `Pages(Vec<(String, SsgError)>)`.
It implements `std::error::Error` by hand with `source()` (no `thiserror`: zero new
dependencies), is `#[non_exhaustive]`, and converts into `io::Error` keeping `InvalidInput`
for the two refusals, so `?` still works in functions returning `io::Result`. A bulk
operation returns **every** failure as data, never only the first.

- **Guard.** `each_failure_has_its_own_variant_and_converts_to_an_io_error`,
  `generate_multiple_reports_every_failure_not_just_the_first`

### L11. Make the bad call not compile

(A2, PR #52)

`Node::Text(String)` was `pub`, so `Node::Text("<script>".into())` bypassed the "escape once"
guarantee. `Markup` is a newtype with **no public constructor**; `Node::text` escapes,
`Node::comment` neutralises, `Node::raw` is the greppable hatch. A `compile_fail` doctest
proves the bad call stays impossible. Once the type enforces the invariant, delete the
runtime re-check that used to guard it.

- **Guard.** `src/core/node.rs`, `a_comment_survives_runs_of_hyphens_and_bang_closers`

### L12. wasm-bindgen: by value consumes, by reference clones

(W1, W2, PR #54)

- A method taking `&WElement` forces a deep clone of the subtree. That made building a page
  quadratic in depth, overflowed the wasm stack on deep trees, and left the original handle
  alive and independent, so a later change to it was silently not in the page. Take the
  argument **by value**; wasm-bindgen then moves it.
- Verified in Node: a handle used after it was moved throws `null pointer passed to rust`,
  for the receiver and for a passed child alike. Document it in the module and the README.
- Clippy `needless_pass_by_value` will object when a by-value argument is only borrowed
  (`add_seo`). That is intentional there: only a by-value argument invalidates the JS handle.
  Allow it with that sentence as the comment.
- Run real tests, not just `cargo check`: `wasm-pack test --node -- --features wasm` ran 15
  tests. Build the package with `scripts/build-wasm.sh`'s flags to try JavaScript claims.
  Cargo flags go **after** `--`: `wasm-pack build --release --target nodejs … -- --features
  wasm`. Put `--features wasm` before the `--` and wasm-pack rejects it.
- **Guard.** `tests/wasm.rs`, `src/wasm.rs`

### L13. Deviating from an issue is fine when you can say why

A3 asked for `Element::new(impl Into<Cow<'static, str>>)`. That would break
`.attr(&runtime_key, v)` with any non-`'static` `&str`, a common pattern in loops. The field is
a `Cow` but the public constructors keep `impl Into<String>`; the saving comes from internal
literals via `Element::from_static`/`Attribute::new_static`. Write such a deviation in the PR
body and in [`decision-log.md`](decision-log.md), so the reviewer decides with the reason in
hand. Likewise `Attribute::raw` stayed public (a declared escape hatch used by a fuzz target)
and `Render` was not implemented for `Document` (its inherent `render()` is pretty, the
trait's is compact: one name, two meanings).

- **Guard.** `src/core/element.rs`, `decision-log.md`

---

## Batch C — Performance

### L14. The benchmark profile can measure the wrong build

- `cargo bench` inherits `[profile.release]`, whose `opt-level = "z"` serves the WASM size
  budget. Benchmarks ran size-optimised code. `[profile.bench] opt-level = 3` fixes it.
- `iter_batched` hands the routine its input by value, so dropping a 10,000-node tree was
  timed. Use `iter_batched_ref`.
- `Throughput::Bytes` turns a time into MiB/s, which survives changing the workload size.
- **Guard.** `Cargo.toml`, `benches/render.rs`

### L15. Count allocations; do not argue about them

`tests/allocations.rs` pins how many allocations plus reallocations hot operations make, with
`stats_alloc`. Ceilings start at the measured value and only go down. It is a **single**
`#[test]` because the allocator counts globally and a second test thread would pollute the
figure. `stats_alloc` is a dev-dependency because `unsafe_code = "forbid"` also covers tests,
so a counting allocator cannot be written in the repo. Without this test, the claim "removes
~5 allocations per element" in an issue could not be checked; it still has to be measured.

- **I broke this rule within the hour.** Adding a second `#[test]` to the file (a `size_of`
  ceiling) made two threads run at once and the figures jumped (`div().add_class` read 4
  instead of 2). Fold every new check into the single test, or use another file.
- **Guard.** `the_hot_paths_stay_within_their_allocation_budgets`

### L22. Chase the finding the baseline hands you

The first baseline showed `html!` at ~1.8× the builder, against a project rule that it is
"only sugar". The cause was three intermediate copies (`to_string()`, an owned `String` key
for a literal, a rebuilt content string), not the macro's structure. Fixing them cut the build
by 40% and the allocations from 5 to 3 with an identical tree. When a measurement contradicts
a stated rule, the discrepancy is usually the cheapest win in the file.
Safe-by-construction trick used: a `fmt::Write` adaptor that escapes each piece it receives,
valid because escaping is per character (escaping the pieces equals escaping the whole).

- **Guard.** `a_display_value_is_escaped_as_it_is_formatted`,
  `several_text_children_are_joined_into_one_content_string`

### L23. A decision that was right in the issue can be wrong after a later fix

Issue #35 proposed skipping the pretty writer's `DropIfEmpty` rollback for `Element` children,
"because an element is never empty". That was true when written. Batch A's S2 fix then made an
element with an invalid tag render **nothing**, so the rollback is now what prevents a blank
line. Applying the issue as written would have reintroduced the bug. Before implementing an
old issue, re-read it against what has changed since; the premises are the part that rot.
Close it with the evidence, not with a half-applied change.

- **Guard.** `an_unsafe_tag_name_renders_nothing`

### L24. "Analysed, not applied" is a result

Of the six remaining performance issues, one was a clear win (`html!`), one a small one
(`trusted_url`, 4 → 3 allocations), and four were measured or reasoned and **not** applied:
`Box<Element>` (+1 allocation per element child for ~72 B saved per text node), a larger
initial stack (−3%, inside the noise, +5 KB per tiny render), caching attribute validity
(moves the cost, does not remove it for build-once/render-once), and an O(depth) stack (no
benchmark shows the cost). Each got a comment on its issue with the number, and a ceiling test
where one was cheap (`size_of`). A guard that notices regression is the deliverable when the
optimisation is not.

- **Guard.** `the_hot_paths_stay_within_their_allocation_budgets`

### L16. Report the regression you caused

The first P4 attempt (escaping clean runs with `push_str`) made the dense-escape benchmark
2.6× slower, because the entity table was passed as a `fn(u8) -> …` **pointer**, which is not
inlined into the hot loop. Passing it as a generic `impl Fn` fixed the builds but not the
dense case, whose per-run cost (two `push_str` plus a char-boundary check) is inherent without
`unsafe`. The PR states both numbers: prose-like text 21.8 → 10.3 µs, adversarial text
19.7 → 55.6 µs. A win on realistic input and a loss on adversarial input is a trade-off to put
in front of the owner, not to hide. Details: [`performance.md`](performance.md).

- **Guard.** `performance.md`, `benches/render.rs`

---

## Process lessons

### L17. Chain commands with `&&` when a failure must stop the chain

`cargo clippy …; … ; git commit` committed a change that failed clippy, because `;` ignores the
exit status. It was caught before the push and fixed with `--amend`. Use `&&`, or check each
status, whenever a later step is irreversible (commit, push, publish).

### L18. Stacked PRs: test every tip, expect conflicts in shared prose

Independent branches off `develop` conflicted as soon as two of them edited `node.rs` and
`SECURITY.md`. Stack them (each branch built on the previous, each PR based on the previous
branch), rebase, and run the test suite **at every tip**, not only at the end. Docs that
several PRs extend (`SECURITY.md`, `AGENTS.md`) are the usual conflict site.

### L21. An edit that matched nothing looks exactly like an edit that worked

Four tests (S2's and S4's) were "added" by a script doing `text.replace(anchor, anchor +
new_test)`. The anchor existed only on a different branch, so `replace` returned the text
unchanged, raised nothing, and the commit and the PR text claimed tests that did not exist.
Both fixes were merged without coverage; a knowledge-base guard check found it later (#57).

- **Rule.** `str.replace`, `sed -i` and most scripted edits return success when nothing
  matched. Assert the anchor is present (`assert anchor in text`) or use the Edit tool, which
  fails on no match.
- **Rule.** After adding a test, check that the **test count rose** and that the test is
  listed by name in the run (`cargo test <name>`), not just that the suite is green.
- **Rule.** Prove a new test can fail: switch the fix off (a one-line mutation such as
  `if false && …`), see the test fail, restore. This is the only check that catches a test
  that exists but asserts nothing.
- **Rule.** Do not write "covered by `test_x`" in a commit or PR body until `grep -rF "fn
  test_x"` finds it.
- **Guard.** `scripts/check-knowledge-base.sh`, `an_unsafe_tag_name_renders_nothing`,
  `angle_brackets_in_attributes_are_escaped_only_under_raw_text_parents`

### L19. Tool availability is a fact to report, not to work around

`cargo-deny` was not installed on the dev machine, so `./scripts/verify.sh` always showed one
failure. State it ("fails only on the missing `cargo-deny`; CI runs it") and move on. Do not
edit the script to skip the check, and do not install tools globally unasked.

### L20. Clippy (pedantic) rules that bit, and the fix

| lint | where | fix |
| --- | --- | --- |
| `manual_assert_eq` | deep-tree tests | keep `assert!(a == b)`, `#[allow]` with the reason (L6) |
| `needless_pass_by_value` | wasm by-value args | allow with the "consumes the JS handle" comment (L12) |
| `elidable_lifetime_names` | helper fns | drop the named lifetime |
| `items_after_statements` | tests with a helper `fn` | declare the helper first |
| `semicolon_if_nothing_returned` | closures in benches | add the `;` |
| `cast_possible_truncation` | `u64 as usize` | keep sizes as `usize`, widen with `as u64` |
| `expect_used` | benches | `let _ = fallible();` with a comment, or handle the error |
| `needless_borrow` | testing a `&T: Render` impl | go through a generic `fn through<T: Render>(t: T)` |
