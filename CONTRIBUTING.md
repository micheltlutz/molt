# Contributing

Thanks for wanting to help. This document is short on purpose — the repository enforces
most of what matters, so you can find out you are wrong in seconds rather than in review.

## The one command

```bash
./scripts/verify.sh
```

Build, test, clippy, rustfmt, tag-catalog freshness, the golden fixtures, and an end-to-end
smoke test that builds a throwaway crate against your checkout. It is a **superset** of CI,
so if it passes locally, CI passes.

It accumulates failures rather than stopping at the first, so one run tells you everything
that is wrong.

## Setup

```bash
rustup toolchain install stable
rustup target add wasm32-unknown-unknown
cargo install wasm-pack cargo-llvm-cov   # only for the wasm and coverage jobs
```

MSRV is 1.85 (edition 2024). CI tests both stable and the MSRV; a change that needs a newer
compiler needs a deliberate MSRV bump and a CHANGELOG entry.

## Workflow

1. Fork, then branch from `develop` — not `main`.
2. Name the branch `feature/…`, `fix/…` or `docs/…`.
3. Make the change. Add tests.
4. `./scripts/verify.sh`
5. Update `CHANGELOG.md` under `## [Unreleased]`.
6. Pull request against `develop`.

## Commit messages

Prefix with the kind of change:

```
Add: comment nodes to the html! macro
Fix: attribute injection through add_class
Change: data_attrs takes an ordered iterator
Docs: explain why escaping is not idempotent
Test: cover the empty-fragment blank-line case
Perf: reserve the output buffer once per render
```

Reference issues in the body (`Refs #12`, `Fixes #12`), not the subject.

## Code

- **Every `pub` item needs a rustdoc comment with a working example.** `missing_docs` is
  denied, and examples are compiled as doctests, so they cannot rot.
- Comments explain *why*. The code already says what.
- `cargo fmt` is the arbiter of layout; `clippy -D warnings` of everything else.
- Prefer a test over a comment asserting something is true.
- The house style beyond what the tools enforce (naming, signatures, errors, docs, shell) is
  in [`.claude/rules/code-style.md`](.claude/rules/code-style.md). Claude Code loads it in every
  session; it applies to a human contributor just the same.

## Tests

**Assert on rendered strings, not tree shape.**

```rust
// Good — locks in the contract.
assert_eq!(div().add_class("a").render(), r#"<div class="a"></div>"#);

// Bad — locks in the implementation.
assert_eq!(div().add_class("a").attributes()[0].key(), "class");
```

This is not style pedantry: it is what lets the internals be rewritten without touching the
tests.

## Changing markup

`cargo test --test golden` compares four files against the golden fixtures, byte for byte.
If your change alters output, that test fails.

That is the system working. Either the change is wrong, or the fixtures genuinely need
updating:

```bash
MOLT_UPDATE_FIXTURES=1 cargo test --test golden
```

If you regenerate them, **say so in the pull request and explain why**. A fixture diff is
the only place a markup change is visible to a reviewer.

## Adding an element

See the five-step contract in [`.claude/rules/elements.md`](.claude/rules/elements.md). Skipping
any step fails `verify.sh`.

## What gets a pull request merged

- `./scripts/verify.sh` passes
- New behaviour has a test that would fail without it
- New public API has rustdoc with an example
- `CHANGELOG.md` updated
- Markup changes are either absent or explained

## Code of conduct

[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) applies to every interaction in this repository.
