# Working on Molt

Context and rules for an agent changing this repository. Humans want
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## The mental model

This library builds an HTML **string**. There is no DOM, no runtime, no reactivity and no
template language. You construct a tree of values and serialize it once.

Molt is the reference for its own behaviour. Byte-for-byte output is pinned by four golden
fixtures in `tests/fixtures/`, and a change that alters the output for valid input fails
`cargo test --test golden`: it is either wrong or deliberate. Where behaviour is unsafe or
wrong, fix it; do not preserve a bug for the sake of an old output. Three things make such a
change complete, and one that skips them is not: a test that asserts the rendered string;
an entry in `CHANGELOG.md` (under *Security* or *Breaking*); and a row in `MIGRATION.md`'s
*Output changes* table, so that someone upgrading can find it in their own output.

Molt began as a port of a Swift library, Winged-Swift, which is to be discontinued after Molt
1.0.0. What that history explains, and where each deliberate difference came from, is kept in
[`docs/history`](docs/history). The one link left in the code is the `Ports …` comments on
tests and the script that checks them (step 7 of `verify.sh`); both skip themselves when the
original checkout is absent, and both are slated for removal.

## Knowledge base

`.claude/skills/molt/` holds the reasoning behind these rules and what the 1.0 hardening
batches taught: security (OWASP Top 10 mapped onto this code), stack-safety, semver, errors,
wasm-bindgen, benchmarking, design patterns to use and to refuse, and a decision log. Start
at `SKILL.md`; its table says which reference to read for your task. The most useful single
file is `references/lessons-learned.md`: every entry is a bug that already happened, with the
test that now guards it.

Each lesson names its guard, and `./scripts/check-knowledge-base.sh` (part of `verify.sh`)
fails when one disappears. When you learn something the hard way, add a lesson and its guard.

## Commands

| command | when |
| --- | --- |
| `./scripts/verify.sh` | **before reporting any change as done** — superset of CI |
| `cargo test` | unit, integration and doc tests |
| `cargo test --test golden` | the byte-for-byte parity gate |
| `MOLT_UPDATE_FIXTURES=1 cargo test --test golden` | regenerate fixtures — only when markup was *meant* to change |
| `./scripts/generate-tag-catalog.sh` | after adding or renaming an element |
| `./scripts/check-swift-parity.sh` | after touching a `Ports …` comment on a test; skips itself when the original checkout is absent |
| `cargo clippy --all-targets --all-features -- -D warnings` | lint |
| `cargo bench` | render performance |

Rust lives at `/opt/homebrew/opt/rustup/bin` on this machine; it is not on the default
`PATH`.

## Repository map

| path | contents |
| --- | --- |
| `src/core/` | node tree, renderer, escaping, attribute, tag sets |
| `src/elements/` | the `define_elements!` table — all 93 tags |
| `src/macros.rs` | the `html!` macro and its two token munchers |
| `src/document.rs`, `src/layout.rs` | page assembly |
| `src/seo.rs`, `src/accessibility.rs` | metadata and the a11y audit |
| `src/sitemap.rs`, `src/feed.rs` | XML output |
| `src/ssg.rs` | filesystem output, `feature = "ssg"`, off on wasm32 |
| `src/wasm.rs` | `wasm-bindgen` surface, `feature = "wasm"` |
| `tests/fixtures/` | the golden files, compared byte for byte — **never hand-edit** |

## Rules

1. **Escaping happens once, when content enters the tree.** `text()` and `Attribute::new`
   escape; the renderer never does. Never pre-escape a value before passing it in — you
   will get `&amp;amp;`.
2. **`raw_text` and `Node::Raw` are the only escape hatches.** They are greppable on
   purpose. If you reach for one, say why in a comment.
3. **Three escapers, not one.** `escape_text` (`& < > " '`), `escape_attribute`
   (`& " '` — *not* `<` `>`), `escape_xml` (`'` → `&apos;`). The fixtures depend on the
   differences. (The original spec, `docs/history/MOLT_SPEC.md` §3.3, shows only one; it is wrong.)
   `escape_json_script` is a fourth, different thing: it makes JSON safe inside `<script>`
   with `\uXXXX` escapes and is not HTML escaping. Do not use one in place of another.
4. **Void elements take no children.** The writer emits `>` and returns. Adding a child to
   an `img()` is silently dropped; that is the contract.
5. **`<pre>`, `<code>` and `<textarea>` are never indented inside.** Their whitespace is
   what the browser displays.
6. **Empty children leave no blank line.** The pretty writer renders each child into a
   scratch buffer and skips it if empty. Do not "optimise" that away — it is what keeps an
   empty `Fragment` from leaving a gap.
7. **`Document::render` is pretty; `Element::render` is compact.** Asymmetric on purpose,
   and the fixtures encode it.
8. **`set_id`, `set_style` and `set_role` replace. Everything else appends.** Also
   load-bearing.
9. **`html!` is sugar over the builder, never a second code path.** Any form it accepts
   must expand to the same tree the builder produces. There is a test asserting exactly
   that — keep it passing.
10. **Rendering is configured by a value, never globally.** `RenderOptions` is passed in.
    Do not add a global switch.
11. **Tests assert on rendered strings, not tree shape.** A test that reads
    `element.children()[0].attributes()[1]` locks in an implementation. A test that asserts
    the markup locks in the contract, which is what lets the internals be rewritten without
    touching the tests.
12. **Every `pub` item needs a rustdoc comment with an example.** `missing_docs` is denied.
    Comments explain *why*; the code already says what.

## Adding an element

Five steps. Skipping any of them fails `./scripts/verify.sh`.

1. Add the entry to the `define_elements!` table in `src/elements/mod.rs`, with a doc
   comment naming the tag.
2. If it is a void element, add it to `VOID_TAGS` in `src/core/tags.rs` — **keep the slice
   sorted**, `is_void` binary-searches it. It is a `&[&str]`, so there is no length to update.
3. If its whitespace is significant, add it to `WHITESPACE_SENSITIVE_TAGS`.
4. Run `./scripts/generate-tag-catalog.sh` and commit the regenerated
   `docs/tag-catalog.md`.
5. Add a render assertion to the tests in `src/elements/mod.rs`.

## Changing markup

If a change alters rendered output, `cargo test --test golden` fails. That is the system
working. Either the change is wrong, or the fixtures need regenerating — and if they do,
say so explicitly in the pull request, because a fixture diff is the only place a markup
change is visible to a reviewer.

## Recipes

**Fix a bug.** Reproduce it with the hostile or failing input as a test that asserts on the
rendered string, watch it fail, fix it, then switch the fix off by hand and watch the test
fail again (see lesson L21 in the knowledge base). Check the test count rose and that the
test is listed by name.

**Fix a security problem.** Start from the surface table in
`.claude/skills/molt/references/security-owasp.md`: find the place where a caller's string
reaches the output, make it safe at *write time* so no construction path bypasses it, drop
rather than repair, and add the row for any new place. Update `SECURITY.md`, and if the
output changes for some input, `CHANGELOG.md` and `MIGRATION.md`'s *Output changes* table.

**Change a `pub` item.** It is semver. Check: a new variant or field on an exhaustive item,
an array length in a type, a changed return type or bound, a renamed method, a name that
collides with a trait method. If any applies it is `breaking`: say so in the PR, add it to
`CHANGELOG.md` under *Breaking* and to `MIGRATION.md` with a compiled before/after.

**Walk a tree.** Use an explicit stack, never recursion, and never `derive` `Clone`,
`PartialEq`, `Hash` or `Drop` on a recursive type. Add a 100,000-level test. In it, compare
with `assert!(a == b)`; `assert_eq!` formats the trees with the recursive `Debug`.

**Make something faster.** A claim needs a before/after number from `cargo bench --bench
render` and, for allocations, `tests/allocations.rs`. Put the regressions in the PR too. See
`references/performance.md`.

**Touch the WASM bindings.** Arguments are by value (they consume the JS handle). Run
`wasm-pack test --node -- --features wasm`; `cargo check` is not a test.

## What a reviewer blocks

Verdicts follow `references/rust-guidelines.md`: **block** for an injection path, an
abort on input, `unsafe`, a swallowed error, a `pub` item you cannot take back, or a claim
with no measurement; **warn** for an avoidable clone or a missing `#[must_use]`. In practice:
`unwrap`/`expect` in library code, an unexplained use of `raw_text`/`Node::raw`/
`Attribute::raw`/`trusted_url`, recursion over the tree, a derive on a recursive type, a new
public trait with one implementer, a fixture edited by hand, and a doc that says what nobody
ran.

## Public API and downstream sites

Anything that changes what a site built on Molt emits is a downstream change, even if no
signature moves: a tag now dropped, a URL now `#`, a feed link now omitted. It belongs in the
*Output changes* table of `MIGRATION.md` as well as the changelog. The `molt-migrate` skill
tells a user how to find these in their own output; if you add a row, the procedure still
holds.

## Before you say you are done

1. `./scripts/verify.sh` was run and its output read. If it fails only on `cargo-deny`
   because the tool is not installed, say so; CI runs it.
2. Every test you mention exists: `grep -rF "fn the_test_name" src tests`.
3. Every doc claim was run, and every `Guard:` in the knowledge base still resolves.
4. You reported what you did **not** verify (workflows cannot be run locally; a stacked PR
   gets no CI until its base is `main` or `develop`).
5. Nothing outward-facing happened that the user did not ask for.

## Style

- Four-space indent, `max_width = 100`, `cargo fmt` is the arbiter.
- Commit prefixes: `Add:`, `Fix:`, `Change:`,
  `Update:`, `Docs:`, `Test:`, `Refactor:`, `Perf:`.
- Branch from `develop`, pull request against `develop`.
- **Run `./scripts/verify.sh` before you report that a change is done.**
