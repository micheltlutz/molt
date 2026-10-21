# Code style

What `cargo fmt` and `cargo clippy` cannot check. It applies to every Rust file in `src/`,
`tests/`, `benches/`, `examples/` and `fuzz/`, and to the shell scripts in `scripts/`.
Anyone who clones this repository and writes code in it follows this; `./scripts/verify.sh`
enforces the mechanical half.

## Toolchain

- Edition 2024, MSRV 1.85. `rust-toolchain.toml` pins `stable`; CI tests both. A change that
  needs a newer compiler is a deliberate MSRV bump with a `CHANGELOG.md` entry, never a
  side effect.
- `rustfmt.toml`: `max_width = 100`, four-space indent, field-init shorthand. `cargo fmt` is
  the arbiter of layout. Do not fight it, and do not add `#[rustfmt::skip]`.
- Lints in `Cargo.toml`: `missing_docs = "deny"`, `unsafe_code = "forbid"`,
  `clippy::pedantic` on, `unwrap_used` and `expect_used` denied. CI runs clippy with
  `-D warnings`. Tests may `unwrap`/`expect` (`clippy.toml`); library code never.
- An `#[allow(...)]` is rare and sits under a comment saying why the lint is wrong there
  (`src/core/node.rs` and `src/wasm.rs` hold the existing ones). Fix the code first.
- No runtime dependencies beyond the optional `wasm-bindgen` and `rayon`. No `thiserror`,
  `anyhow`, `regex`, `serde`. A new dependency is a decision for the owner, not a
  convenience.

## Naming

- `snake_case` functions and methods, `UpperCamelCase` types, `SCREAMING_SNAKE_CASE`
  constants (`VOID_TAGS`, `MAX_URLS`).
- Getters have no `get_` prefix: `tag()`, `key()`, `children()`. Conversions follow the
  `as_`/`to_`/`into_` convention.
- Element constructors are the tag name: `div()`, `h1()`, `img()`, one per row of
  `define_elements!` in `src/elements/mod.rs`. When the tag collides with a Rust item the
  constructor takes a `_tag` suffix (`main_tag`, `html_tag`).
- Builder methods take `self` by value and return `Self`. Those that **replace** are
  `set_*` (`set_id`, `set_style`, `set_role`); those that **append** are `add_*`, `child`,
  `attr`. Never add a `set_*` that appends or an `add_*` that replaces.
- Before naming a trait method, check the other traits a type may implement.
  `Layout::render` collided with `Render::render` and had to become `wrap`.
- Tests are sentences describing the behaviour, without a `test_` prefix:
  `an_unsafe_tag_name_renders_nothing`, `clean_input_is_returned_without_allocating`.

## Signatures

- Accept `impl AsRef<str>` for a value you read, `impl Into<String>` for one you keep,
  `impl IntoIterator<Item = …>` for many. Return an owned value only when you built it.
- Escapers return `Cow<'_, str>`: clean input comes back borrowed. Fields that are usually a
  literal are `Cow<'static, str>`.
- `#[must_use]` on every builder, constructor and pure function whose result is easy to
  drop. Most of the crate has it already; a new one without it is a reviewer's WARN.
- No lifetime parameter you could elide. No `Rc`, `Arc` or `RefCell` in `Node`/`Element`:
  the tree is plain owned values and is `Send + Sync` (a test asserts it).
- Pre-size with `String::with_capacity`/`Vec::with_capacity` when the size is known or
  estimable.

## Errors and panics

- A value that may be absent is `Option`. A failure the caller can act on is
  `Result<_, E>` with a typed `E`. `panic!`/`assert!` only for a broken invariant, never for
  an expected failure. The release profile is `panic = "abort"`, so a panic is a trap.
- Error types are `#[non_exhaustive]` enums with a hand-written `std::error::Error` impl
  and `source()`, plus `From<MyError> for io::Error` when the module is I/O-shaped
  (`SsgError`). Keep the failing path in the error.
- Never swallow an error. The one allowed shape is `let _ = …` during cleanup, under a
  comment saying why ignoring it is correct.
- Restructure instead of `expect`: `let Some(x) = … else { return … };`.

## Documentation

- Every `pub` item has a rustdoc comment with a runnable example; doctests compile it, so
  it cannot rot. `# Errors` and `# Panics` sections on every `pub fn` that can do either.
- Module docs (`//!`) state the module's contract. When there are several variants of one
  idea, use a table (`src/core/escape.rs` is the model).
- Comments explain **why**. Delete a comment that restates the code. A comment that says
  something is true is weaker than a test that asserts it.
- Every `raw_text`, `Node::raw`, `Attribute::raw` and `trusted_url` call sits under a
  comment saying why the input is trusted. They are greppable on purpose.
- A doc claim is a promise you ran, not a plan (L8).

## Module layout

- `use` lines: `std` first, then `crate::…`, one blank line between the groups. Glob
  imports only in `prelude` and in `#[cfg(test)]` modules (`use super::*;`).
- Generic parameters over `fn` pointers in hot loops, so the compiler inlines the match
  (`src/core/escape.rs`). Measure before claiming it mattered.
- Features gate with `#[cfg(feature = "…")]`, never `target_arch`, so native tests exercise
  the `wasm` feature. The one `target_arch` gate is `ssg`'s filesystem code, which cannot
  exist on `wasm32`.
- A `pub` trait with one implementer is not published. `Render` is the exception and its
  docs say so.

## Shell scripts

- `#!/usr/bin/env bash`, `set -euo pipefail` (`verify.sh` drops `-e` on purpose: it
  accumulates failures), `cd "$(dirname "$0")/.."` before anything else. Bash 3.2 is what
  macOS ships, so no `globstar`, no associative arrays.
- Temporary files come from `mktemp`; one `trap … EXIT` per script, since a second replaces
  the first (L35); steps that must stop on failure are chained with `&&` (L17).
- Backticks inside a double-quoted argument are executed (L32); quote with single quotes.

Lesson numbers (`L8`, `L17`, …) point at
[`lessons-learned.md`](../skills/molt/references/lessons-learned.md); the full review
checklist is [`rust-guidelines.md`](../skills/molt/references/rust-guidelines.md).
