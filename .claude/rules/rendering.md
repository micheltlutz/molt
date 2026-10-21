---
paths:
  - "src/core/**"
  - "src/macros.rs"
  - "src/document.rs"
  - "src/layout.rs"
---

# Rendering contract

The output of these files is pinned byte for byte by `tests/fixtures/`. Each rule below is
encoded in a fixture or a test, so breaking one is a markup change (see
`golden-fixtures.md`). The numbers are the ones `AGENTS.md` and the lessons cite.

1. **Escaping happens once, when content enters the tree.** `text()` and `Attribute::new`
   escape; the renderer never does. Never pre-escape a value before passing it in: you get
   `&amp;amp;`.
2. **`raw_text`, `Node::raw` and `Attribute::raw` are the only escape hatches**, and
   `trusted_url` the only way past the URL policy. All four are greppable on purpose, and a
   call to any of them sits under a comment saying why the input is trusted.
3. **Three escapers, not one.** `escape_text` (`& < > " '`), `escape_attribute`
   (`& " '`, *not* `<` `>`), `escape_xml` (`'` → `&apos;`, XML-illegal characters dropped).
   `escape_json_script` is a fourth, different thing: `\uXXXX` escapes for JSON inside
   `<script>`, not HTML escaping. Never substitute one for another; the fixtures depend on
   the differences. (`docs/history/MOLT_SPEC.md` §3.3 shows one escaper and is wrong.)
4. **Void elements take no children.** The writer emits `>` and returns; a child added to
   `img()` is silently dropped. `VOID_TAGS` in `src/core/tags.rs` stays sorted, because
   `is_void` binary-searches it.
5. **`<pre>`, `<code>` and `<textarea>` are never indented inside.** Their whitespace is
   what the browser displays (`WHITESPACE_SENSITIVE_TAGS`).
6. **Empty children leave no blank line.** The pretty writer renders each child into a
   scratch buffer and skips it if empty. Do not "optimise" that away: it is what keeps an
   empty `Fragment`, a dropped element or a false `@if` from leaving a gap.
7. **`Document::render` is pretty; `Element::render` is compact.** Asymmetric on purpose.
   Do not implement `Render` for `Document`.
8. **`set_id`, `set_style` and `set_role` replace. Everything else appends.**
9. **`html!` is sugar over the builder, never a second code path.** Any form it accepts
   expands to the same tree the builder produces; a test in `src/macros.rs` asserts it.
10. **Rendering is configured by a value, never globally.** `RenderOptions` is passed in,
    so two threads can render the same tree differently. Do not add a global switch.

Also: names are input. An invalid tag name drops the whole element and an invalid attribute
name drops the attribute. Drop, never repair: there is no safe guess for what
`div><script>` meant. Attribute order is insertion order, never a `HashMap`'s.

Why each rule exists: [`decision-log.md`](../skills/molt/references/decision-log.md).
Mistakes by symptom: [`pitfalls.md`](../skills/molt/references/pitfalls.md).
