---
name: molt
description: Build HTML with the Molt Rust library (molt-framework on crates.io) or change the Molt repository itself safely. Use when writing Rust that generates HTML (Element, Node, Document, html!, StaticSiteGenerator, SEO/sitemap/RSS, the wasm-bindgen WElement/WDocument), when a project depends on molt-framework, or when fixing a bug, adding an element, tuning performance, reviewing Rust code, or changing a public API in the Molt repository. Holds the Rust best-practice knowledge base distilled from the 1.0 hardening batches: security (OWASP), stack-safety, semver, error design, wasm-bindgen, benchmarking.
---

# Molt

Molt builds an HTML **string**. You assemble a tree of values and serialize it once. There is
no DOM, no runtime, no reactivity. Four golden fixtures pin its output byte for byte.
`AGENTS.md` at the repo root is the map and `.claude/rules/` the rulebook; this skill is the
reasoning behind them and the lessons that produced it.

```rust
use molt::prelude::*;
let card = div().add_class("card").child(h1().text("Fuel & chain"));
assert_eq!(card.render(), r#"<div class="card"><h1>Fuel &amp; chain</h1></div>"#);
```

## Which workflow are you in?

### A. Using Molt in another project

1. `cargo add molt-framework` (the crate is imported as `molt`).
2. Pass untrusted data through `text()` and `attr()`. Never through `raw_text()`, `Node::raw()`
   or `Attribute::raw()`. Those three are the only escape hatches; grep for them to audit.
3. Read [`pitfalls.md`](references/pitfalls.md) before writing the first page. It lists the
   mistakes agents actually made, grouped by symptom.
4. Element names: [`docs/tag-catalog.md`](../../../docs/tag-catalog.md) (generated, never
   hand-edit).

### B. Changing the Molt repository

1. Read `AGENTS.md`, then [`lessons-learned.md`](references/lessons-learned.md). Every rule
   there cost a bug.
2. Find your task in the table below and read that reference **before** editing.
3. Write the failing test first, and assert on the **rendered string** (AGENTS.md rule 11).
4. Run `./scripts/verify.sh`. Do not say "done" before it is green. If it fails only on
   `cargo-deny` because the tool is missing locally, say so; CI runs it.

## Read this when

| you are about to… | read |
| --- | --- |
| fix or review anything that handles untrusted input, paths, URLs, XML | [`security-owasp.md`](references/security-owasp.md) |
| write or review any Rust in this repo | [`rust-guidelines.md`](references/rust-guidelines.md) |
| change a `pub` item, a type's fields, an error type, a trait impl | [`rust-guidelines.md`](references/rust-guidelines.md) § API, [`decision-log.md`](references/decision-log.md) |
| touch a tree walk, `Clone`, `PartialEq`, `Drop`, recursion | [`lessons-learned.md`](references/lessons-learned.md) L6–L7 |
| optimise, or claim something is faster | [`performance.md`](references/performance.md) |
| touch `src/wasm.rs` | [`lessons-learned.md`](references/lessons-learned.md) L12 |
| reach for a design pattern, a trait, or an abstraction | [`design-patterns.md`](references/design-patterns.md) |
| act without being asked (push, publish, delete, regenerate fixtures) | [`agent-governance.md`](references/agent-governance.md) |
| wonder why something is the way it is | [`decision-log.md`](references/decision-log.md) |

## Seven rules that prevent most mistakes

1. **Escape once, on entry.** `text()` and `Attribute::new` escape; the renderer never does.
   Pre-escaping gives `&amp;amp;`.
2. **Names are input too.** Tag and attribute names are validated when written; an invalid
   one is dropped, not repaired.
3. **A stack overflow is an abort.** Anything that walks a tree uses an explicit stack. A
   `derive` on a recursive type recurses.
4. **Prefer the type over the comment.** If "never pass user data here" is the only
   protection, make the bad call not compile.
5. **A number beats an adjective.** "Faster" needs a benchmark, "fewer allocations" needs
   `tests/allocations.rs`. Report regressions too.
6. **Fixtures are evidence, not scaffolding.** A golden diff means markup changed. Never
   regenerate to make a test pass.
7. **Ask before anything outward-facing.** Push, publish, delete, and force-anything need the
   user's go-ahead for *that* action.

## Keeping this knowledge honest

Each lesson ends with a `Guard:` line naming the tests or files that enforce it.
`./scripts/check-knowledge-base.sh` (run by `verify.sh`) fails if a guard no longer exists,
so a rename cannot silently orphan a lesson. When you learn something new the hard way, add a
lesson with its guard; when a guard is deleted on purpose, delete or rewrite the lesson.
