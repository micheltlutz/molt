# Rust guidelines for this repository

A review checklist, written to be applied, not read once. It combines the official Rust API
Guidelines with what actually went wrong in Molt (see [`lessons-learned.md`](lessons-learned.md)).

## Sources, and what they did and did not give us

- **Rust API Guidelines** — <https://rust-lang.github.io/api-guidelines/> (official; the
  `C-…` ids below are its checklist items).
- **The Rust Reference / std docs** — for `Path`, `Cow`, `Hash`, `GlobalAlloc` semantics.
- **actionbook/rust-skills** — <https://github.com/actionbook/rust-skills>. A "meta-cognition"
  set of skills. The error-handling decision tree below is from its `m06-error-handling` skill.
  Its other skills are question prompts (who owns this data? generics or trait objects?
  where is the bottleneck?), not rule lists.
- **A Rust code-review checklist** (ownership, errors, `unsafe`, concurrency, API, perf, with an
  approve / warn / block verdict) from the `rust-reviewer` agent in
  `affaan-m/everything-claude-code`. The page <https://claudecowork.im/resources/rust-reviewer>
  named in the original request is an install stub with no rules, so the checklist is taken
  from that upstream agent, not from the page.
- **`agent-governance-reviewer`** — the claudecowork page publishes no rules either. The
  governance guidance in [`agent-governance.md`](agent-governance.md) is written from this
  repository's own practice, and says so.

Nothing here is a substitute for `cargo clippy --all-targets --all-features -- -D warnings`,
which the CI runs with `clippy::pedantic` on and `unwrap_used`/`expect_used` denied.

## Verdict scheme

Use it when reviewing a diff, yours included.

- **BLOCK** — CRITICAL or HIGH: an injection path, an abort-on-input, `unsafe`, a swallowed
  error, a `pub` item you cannot take back, a claim with no measurement behind it.
- **WARN** — MEDIUM: an avoidable clone, a missing `#[must_use]`, an undocumented panic.
- **APPROVE** — nothing above.

## 1. Errors and panics

Decision tree (rust-skills `m06`):

| situation | use |
| --- | --- |
| a value may be legitimately absent | `Option` |
| a recoverable failure the caller can act on | `Result<_, E>` with a typed `E` in a library |
| a bug or a broken invariant | `panic!`/`assert!` — never for expected failures |

- Library code in this crate has **no** `unwrap()`/`expect()` (denied by lint; allowed in
  tests via `clippy.toml`). If an invariant truly cannot fail, restructure so the compiler
  proves it (`let Some(x) = … else { return … }`) rather than `expect`.
- Propagate with `?`; keep the failing path in the error (`SsgError::Io { path, source }`).
  Never swallow an error; if you must ignore one (`let _ = fs::remove_file(tmp)` during
  cleanup), comment **why** ignoring it is correct.
- Libraries: hand-written `std::error::Error` impl with `source()`, `#[non_exhaustive]`
  enum, `From<MyError> for io::Error` when the module is I/O-shaped. No `thiserror`/`anyhow`
  without a reason (this crate has no runtime dependencies on purpose). `Box<dyn Error>` in a
  public signature hides the variants a caller wants to match. (C-GOOD-ERR)
- Document `# Errors` and `# Panics` on every `pub fn` that can do either.
- `panic = "abort"` is set in the release profile (WASM size budget). A panic is therefore an
  unrecoverable trap there; design for "cannot panic", not for "caught panic".

## 2. Ownership and allocation

- Accept `&str`/`&[T]`/`impl AsRef<str>`; return owned only when you build something new.
- A needless `.clone()` is a WARN; a clone of a **tree** is a BLOCK (it is recursive and
  O(n)). `Document::root()` cloning head and body was a real abort (L6).
- Prefer `Cow<'_, str>` for "usually unchanged" strings (escapers). Prefer `Cow<'static, str>`
  for fields that are usually a literal (`Element::tag`, `Attribute::key`).
- Pre-size with `Vec::with_capacity`/`String::with_capacity` when the size is known or
  estimable. A `String` that grows by doubling costs a reallocation each time.
- Do not add a lifetime parameter you can elide (clippy `elidable_lifetime_names`).
- Do not reach for `Rc`/`Arc`/`RefCell` to escape the borrow checker; the tree is plain owned
  values and is `Send + Sync` on purpose (a test asserts it).

## 3. `unsafe`

`unsafe_code = "forbid"`. It cannot be `allow`ed locally, and it also applies to tests, benches
and examples. If you think you need it:

1. You probably need a dev-dependency (a counting allocator, L15) or a different algorithm.
2. If it is truly unavoidable, that is a change to the project's stated policy: stop and ask
   the owner. A `// SAFETY:` comment does not substitute for the decision.

## 4. Recursion and the stack

- Rust has no tail-call guarantee and a stack overflow is an **abort**.
- Any traversal of `Node`/`Element` uses an explicit stack. Hand-write `Clone`, `PartialEq`,
  `Eq`, `Hash` for recursive types; make `Drop` iterative. See L6 and the existing
  `Step`/`clone_tree`/`trees_equal`/`hash_tree`.
- A feature that handles untrusted nesting is not done until a 100,000-level test exists.
- `Debug` stays derived and recursive. Say so in the docs; do not assert-format deep trees.

## 5. API design (before 1.0 and after)

From the Rust API Guidelines, as applied here:

- **C-CASE / C-CONV / C-GETTER.** `snake_case` functions; getters without `get_`
  (`tag()`, `key()`); `as_`/`to_`/`into_` conventions respected.
- **C-COMMON-TRAITS.** Implement `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Default`,
  `Display` where they make sense. `Display` for a renderable type = the render
  (`{:#}` = pretty). Do *not* implement `Render` for `Document`: its inherent `render()` is
  pretty, the trait's is compact.
- **C-STRUCT-PRIVATE / C-NEWTYPE-HIDE.** `pub` fields only on small plain-data structs, and
  then `#[non_exhaustive]` with a constructor. Use a newtype to make an invalid state
  unconstructible (`Markup`).
- **C-SEALED / C-STABLE.** A public trait is frozen API. Do not publish a trait with one
  implementer (no public `Visitor`, `Escaper`, strategy traits). `Render` is the exception and
  its stability note belongs in its docs.
- **C-CTOR / C-BUILDER.** `new(required…)` plus `#[must_use]` chained setters. The fluent
  methods here take `self` by value and return `Self`.
- **C-DEBUG-NONEMPTY, C-SEND-SYNC.** Public types are `Send + Sync` (tested).
- **C-DOC / C-EXAMPLE / C-FAILURE.** `missing_docs` is **denied**: every `pub` item has a doc
  comment with a runnable example, `# Errors`/`# Panics` where relevant. Comments say *why*.
- **`#[must_use]`** on pure builders and constructors returning a value that is easy to drop.
- **Name collisions.** Before naming a trait method, search the other traits a type may also
  implement (`Layout::render` vs `Render::render`).
- **Semver checklist for a PR touching `pub`:** new variant/field on an exhaustive item? array
  length in a type? changed return type? changed trait bound? renamed method? If yes: it is
  `breaking`, labelled, listed in `MIGRATION.md`, and justified.

## 6. Iterators, strings, collections

- `chars()` vs bytes: ASCII specials let you scan bytes and slice at a match without splitting
  a UTF-8 character. Anything else must go through `char`/`char_indices`.
- Prefer iterator adaptors to index loops. `iter().rev().map(Step::Compact)` is how children
  are queued in order onto a LIFO stack.
- `HashMap` iteration order is not stable: never let it decide output order (attribute order
  is insertion order and `data_attrs` is sorted deterministically).
- Sort-dependent data (`VOID_TAGS` is binary-searched) must stay sorted; add a test.

## 7. Concurrency and WASM

- The tree is `Send + Sync`; the `parallel` feature relies on it. Do not add interior
  mutability or `Rc` to `Node`/`Element`.
- `rayon` cannot compile for `wasm32`; keep it off the wasm dev-dependency graph.
- wasm-bindgen: by-value arguments consume the JS handle; `&T` forces a clone (L12). The wasm
  feature is gated by **feature**, never by `target_arch`, so native tests exercise it.
- Anything touching the filesystem is compiled out on `wasm32` (`ssg`).

## 8. Tests

- Assert on **rendered strings**, not tree shape (AGENTS.md rule 11).
- One behaviour per test, named as a sentence (`an_unsafe_tag_name_renders_nothing`).
- A bug fix starts with the failing test; keep the hostile input in the test.
- Keep the old implementation as an oracle when rewriting a path (L7).
- Tests that need the filesystem use a unique directory (pid in the name) and never call a
  destructive function on the real working directory.
- A test that must not run in parallel with others (global allocator counters) is a **single**
  `#[test]` in its own file.
- Golden fixtures are copied from Winged-Swift and never hand-edited. A fixture diff in a PR is
  the signal that markup changed; say so in the PR.

## 9. Docs and comments

- A comment explains *why*, never *what*. Delete a comment that restates the code.
- A doc claim is a promise: run it (L8). Update `SECURITY.md`/`README.md` in the same PR that
  changes the behaviour they describe.
- Reference code as `path:line` in conversation; in docs prefer item names, which survive edits.
