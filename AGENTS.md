# Working on Molt

Context and rules for an agent changing this repository. Humans want
[`CONTRIBUTING.md`](CONTRIBUTING.md).

## The mental model

This library builds an HTML **string**. There is no DOM, no runtime, no reactivity and no
template language. You construct a tree of values and serialize it once.

It is a port of [Winged-Swift](https://github.com/micheltlutz/Winged-Swift) 2.0.0. When a
design question comes up, the Swift source at
`../WingedSwiftProjects/Winged-Swift/Sources/WingedSwift/` is the reference — and its
`.claude/skills/wingedswift/references/pitfalls.md` documents behaviours the source alone
does not make obvious.

## Commands

| command | when |
| --- | --- |
| `./scripts/verify.sh` | **before reporting any change as done** — superset of CI |
| `cargo test` | unit, integration and doc tests |
| `cargo test --test golden` | the byte-for-byte parity gate |
| `MOLT_UPDATE_FIXTURES=1 cargo test --test golden` | regenerate fixtures — only when markup was *meant* to change |
| `./scripts/generate-tag-catalog.sh` | after adding or renaming an element |
| `./scripts/check-swift-parity.sh` | after touching a test that names a Winged-Swift case |
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
| `tests/fixtures/` | copied verbatim from Winged-Swift — **never hand-edit** |

## Rules

1. **Escaping happens once, when content enters the tree.** `text()` and `Attribute::new`
   escape; the renderer never does. Never pre-escape a value before passing it in — you
   will get `&amp;amp;`.
2. **`raw_text` and `Node::Raw` are the only escape hatches.** They are greppable on
   purpose. If you reach for one, say why in a comment.
3. **Three escapers, not one.** `escape_text` (`& < > " '`), `escape_attribute`
   (`& " '` — *not* `<` `>`), `escape_xml` (`'` → `&apos;`). The fixtures depend on the
   differences. `MOLT_SPEC.md` §3.3 shows only one; it is wrong.
4. **Void elements take no children.** The writer emits `>` and returns. Adding a child to
   an `img()` is silently dropped, matching Swift.
5. **`<pre>`, `<code>` and `<textarea>` are never indented inside.** Their whitespace is
   what the browser displays.
6. **Empty children leave no blank line.** The pretty writer renders each child into a
   scratch buffer and skips it if empty. Do not "optimise" that away — it is what keeps an
   empty `Fragment` from leaving a gap.
7. **`Document::render` is pretty; `Element::render` is compact.** Asymmetric on purpose,
   inherited from Swift, and the fixtures encode it.
8. **`set_id`, `set_style` and `set_role` replace. Everything else appends.** Also
   inherited; also load-bearing.
9. **`html!` is sugar over the builder, never a second code path.** Any form it accepts
   must expand to the same tree the builder produces. There is a test asserting exactly
   that — keep it passing.
10. **Rendering is configured by a value, never globally.** `RenderOptions` is passed in.
    Do not add a global switch; Swift had one and is removing it.
11. **Tests assert on rendered strings, not tree shape.** A test that reads
    `element.children()[0].attributes()[1]` locks in an implementation. A test that asserts
    the markup locks in the contract. This rule is why Winged-Swift's test suite survived a
    full internal rewrite unchanged.
12. **Every `pub` item needs a rustdoc comment with an example.** `missing_docs` is denied.
    Comments explain *why*; the code already says what.

## Adding an element

Five steps. Skipping any of them fails `./scripts/verify.sh`.

1. Add the entry to the `define_elements!` table in `src/elements/mod.rs`, with a doc
   comment naming the tag.
2. If it is a void element, add it to `VOID_TAGS` in `src/core/tags.rs` — **keep the array
   sorted**, `is_void` binary-searches it — and update the length.
3. If its whitespace is significant, add it to `WHITESPACE_SENSITIVE_TAGS`.
4. Run `./scripts/generate-tag-catalog.sh` and commit the regenerated
   `docs/tag-catalog.md`.
5. Add a render assertion to the tests in `src/elements/mod.rs`.

## Changing markup

If a change alters rendered output, `cargo test --test golden` fails. That is the system
working. Either the change is wrong, or the fixtures need regenerating — and if they do,
say so explicitly in the pull request, because a fixture diff is the only place a markup
change is visible to a reviewer.

## Style

- Four-space indent, `max_width = 100`, `cargo fmt` is the arbiter.
- Commit prefixes follow Winged-Swift's CHANGELOG vocabulary: `Add:`, `Fix:`, `Change:`,
  `Update:`, `Docs:`, `Test:`, `Refactor:`, `Perf:`.
- Branch from `develop`, pull request against `develop`.
- **Run `./scripts/verify.sh` before you report that a change is done.**
