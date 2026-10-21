---
description: Add an HTML element to Molt by the five-step contract (table row, void or whitespace set, catalog, test)
argument-hint: <constructor> <tag> [void] [whitespace]
---

Add the element `$ARGUMENTS` to Molt. The first word is the Rust constructor name and the
second the HTML tag; they are usually the same, and differ only when the tag collides with
a Rust item (`main_tag`, `html_tag`). `void` means the element takes no children;
`whitespace` means its inner whitespace is significant.

Follow `.claude/rules/elements.md` exactly, in this order:

1. Confirm the tag is not already in `src/elements/mod.rs` and is a current HTML element in
   the WHATWG living standard, not a deprecated or vendor tag. Stop and say so if either
   check fails.
2. Add the row to the `define_elements!` table, in alphabetical position among its
   neighbours, with a doc comment naming the tag.
3. If `void`: insert the tag into `VOID_TAGS` in `src/core/tags.rs` **in sorted position**.
   If `whitespace`: add it to `WHITESPACE_SENSITIVE_TAGS`.
4. Add a test to the tests in `src/elements/mod.rs` that asserts the rendered string. For a
   void element, assert that a child is dropped; for a whitespace-sensitive one, that pretty
   rendering does not indent inside it.
5. Run `./scripts/generate-tag-catalog.sh` and include the regenerated `docs/tag-catalog.md`.
6. Add a line under *Added* in `## [Unreleased]` of `CHANGELOG.md`.
7. Run `cargo test <the new test name>` and confirm the test is listed by name, then
   `./scripts/verify.sh`.

Adding an element is not a breaking change and does not touch the golden fixtures. If the
golden test fails, something else changed: stop and report it.
