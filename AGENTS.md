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
[`docs/history`](docs/history). Nothing in the code, the tests or the scripts refers to it.

## The twelve rules

The contract in one screen. The rule files below say why and how; the lessons cite these
numbers.

1. Escaping happens once, when content enters the tree. Never pre-escape a value.
2. `raw_text` and `Node::Raw` are the only escape hatches, and they are greppable on purpose.
3. Three escapers, not one: `escape_text`, `escape_attribute`, `escape_xml`.
   `escape_json_script` is a fourth, different thing.
4. Void elements take no children.
5. `<pre>`, `<code>` and `<textarea>` are never indented inside.
6. Empty children leave no blank line.
7. `Document::render` is pretty; `Element::render` is compact.
8. `set_id`, `set_style` and `set_role` replace. Everything else appends.
9. `html!` is sugar over the builder, never a second code path.
10. Rendering is configured by a value, never globally.
11. Tests assert on rendered strings, not tree shape.
12. Every `pub` item needs a rustdoc comment with an example.

## Where the rules live

This file is the map. The rules are in `.claude/`, one file per topic, so a session loads
only what the files it touches need. A rule without `paths:` loads in every session.

| rule | holds | loads when touching |
| --- | --- | --- |
| `.claude/rules/code-style.md` | how Rust and shell are written here: toolchain, lints, naming, signatures, errors, docs | always |
| `.claude/rules/git-workflow.md` | branches, commit prefixes, what a PR says, what needs a yes | always |
| `.claude/rules/rendering.md` | rules 1 to 10 with their reasons | `src/core/`, `macros.rs`, `document.rs`, `layout.rs` |
| `.claude/rules/tree-walking.md` | explicit stacks, no derives on recursive types, the 100,000-level test | the same files |
| `.claude/rules/elements.md` | the five steps that add an element | `src/elements/`, `tags.rs`, the tag catalog |
| `.claude/rules/testing.md` | rule 11, and the test-first bug recipe | `src/`, `tests/`, `benches/`, `fuzz/` |
| `.claude/rules/golden-fixtures.md` | never hand-edit; what a deliberate output change needs | `tests/fixtures/`, the writers |
| `.claude/rules/public-api.md` | the semver checklist, hygiene for a `pub` item, downstream output changes | `src/`, `MIGRATION.md`, `CHANGELOG.md` |
| `.claude/rules/security.md` | safe at write time, drop not repair, the four escape hatches, the filesystem | `src/`, `SECURITY.md`, `fuzz/` |
| `.claude/rules/wasm.md` | by-value arguments, feature gating, `wasm-pack test` | `src/wasm.rs`, the wasm examples |
| `.claude/rules/performance.md` | a claim needs a number; what was refused on purpose | `benches/`, the allocation tests, the hot paths |
| `.claude/rules/docs.md` | run what you document; lessons and their guards; the decision log | `docs/`, `.claude/`, the root `*.md` |

The repeatable workflows are slash commands in `.claude/commands/`:

| command | does |
| --- | --- |
| `/verify` | runs `./scripts/verify.sh` and reports every failure |
| `/add-element <constructor> <tag> [void] [whitespace]` | the five-step element contract |
| `/fix-bug <issue or description>` | a test-first fix, with the proof that the test can fail |
| `/review [base]` | the `code-reviewer` agent on the branch's diff, plus `api-reviewer` and `security-auditor` when the diff warrants them |
| `/security-audit [area]` | the `security-auditor` agent on the output surface |
| `/golden` | runs the golden test and explains a diff; never regenerates |
| `/lesson <title>` | adds a lesson with its guard to the knowledge base |
| `/release <version>` | prepares a release locally; never tags or publishes |

The reviewers run in their own context, in `.claude/agents/`, and are read-only:

| agent | role |
| --- | --- |
| `.claude/agents/code-reviewer.md` | BLOCK, WARN or APPROVE against the Rust guidelines and the rendering contract |
| `.claude/agents/security-auditor.md` | the OWASP surface table, row by row, with hostile inputs |
| `.claude/agents/api-reviewer.md` | semver, rustdoc, `CHANGELOG.md` and `MIGRATION.md` for a `pub` or output change |

The knowledge base is in `.claude/skills/`:

| skill | when |
| --- | --- |
| `.claude/skills/molt/` | the reasoning behind the rules and what the 1.0 hardening batches taught: security (OWASP Top 10 mapped onto this code), stack-safety, semver, errors, wasm-bindgen, benchmarking, design patterns to use and to refuse, a decision log. Start at `SKILL.md`; the most useful single file is `references/lessons-learned.md`, where every entry is a bug that already happened, with the test that now guards it |
| `.claude/skills/molt-migrate/` | upgrading a project that depends on `molt-framework` and proving its output did not change by accident |

Each lesson names its guard, and `./scripts/check-knowledge-base.sh` (part of `verify.sh`)
fails when a guard disappears, when a link in `.claude/` breaks, when a rule's `paths:`
matches no tracked file, or when a rule, command or agent is missing from the tables above.
When you learn something the hard way, add a lesson and its guard (`/lesson`).

## Commands

| command | when |
| --- | --- |
| `./scripts/verify.sh` | **before reporting any change as done** — superset of CI |
| `cargo test` | unit, integration and doc tests |
| `cargo test --test golden` | the byte-for-byte parity gate |
| `MOLT_UPDATE_FIXTURES=1 cargo test --test golden` | regenerate fixtures — only when markup was *meant* to change, and only when asked |
| `./scripts/generate-tag-catalog.sh` | after adding or renaming an element |
| `cargo clippy --all-targets --all-features -- -D warnings` | lint |
| `cargo bench` | render performance |

Rust lives at `/opt/homebrew/opt/rustup/bin` on this machine; it is not on the default
`PATH`.

## Repository map

| path | contents |
| --- | --- |
| `src/core/` | node tree, renderer, escaping, attribute, tag sets, URL policy |
| `src/elements/` | the `define_elements!` table — all 93 tags |
| `src/macros.rs` | the `html!` macro and its two token munchers |
| `src/document.rs`, `src/layout.rs` | page assembly |
| `src/seo.rs`, `src/accessibility.rs` | metadata and the a11y audit |
| `src/sitemap.rs`, `src/feed.rs` | XML output |
| `src/ssg.rs` | filesystem output, `feature = "ssg"`, off on wasm32 |
| `src/wasm.rs` | `wasm-bindgen` surface, `feature = "wasm"` |
| `tests/fixtures/` | the golden files, compared byte for byte — **never hand-edit** |
| `.claude/` | rules, commands, agents and skills, as mapped above; not shipped in the crate |

## Before you say you are done

1. `./scripts/verify.sh` was run and its output read. If it fails only on `cargo-deny`
   because the tool is not installed, say so; CI runs it.
2. Every test you mention exists: `grep -rF "fn the_test_name" src tests`.
3. Every doc claim was run, and every `Guard:` in the knowledge base still resolves.
4. You reported what you did **not** verify (workflows cannot be run locally; a stacked PR
   gets no CI until its base is `main` or `develop`).
5. Nothing outward-facing happened that the user did not ask for.
