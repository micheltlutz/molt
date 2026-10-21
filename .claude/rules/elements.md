---
paths:
  - "src/elements/**"
  - "src/core/tags.rs"
  - "docs/tag-catalog.md"
  - "scripts/generate-tag-catalog.sh"
---

# Elements and tags

Five steps add an element. Skipping any of them fails `./scripts/verify.sh`. The
`/add-element` command walks them.

1. Add the row to the `define_elements!` table in `src/elements/mod.rs`, with a doc comment
   naming the tag. The constructor is the tag name; when that collides with a Rust item,
   add a `_tag` suffix (`main_tag`, `html_tag`).
2. If it is a void element, add it to `VOID_TAGS` in `src/core/tags.rs`. **Keep the slice
   sorted**: `is_void` binary-searches it, so a misplaced entry silently makes some tags
   non-void (`the_void_set_is_the_html_void_elements` looks each one up and is what catches
   it). It is a `&[&str]`, so there is no length to update, and no array length ever goes
   back into a public type.
3. If its whitespace is significant, add it to `WHITESPACE_SENSITIVE_TAGS`.
4. Run `./scripts/generate-tag-catalog.sh` and commit the regenerated `docs/tag-catalog.md`.
   Never edit the catalog by hand; `verify.sh` runs the script with `--check`.
5. Add a render assertion to the tests in `src/elements/mod.rs`, asserting the rendered
   string.

Tag and attribute names are validated at write time (`is_valid_tag_name`): a letter, then
letters, digits, `-`, `_`, `.`, `:`. An invalid name drops the element; do not loosen the
check to accept a name a browser would tolerate.

Adding an element is not a breaking change. Removing or renaming one is.
