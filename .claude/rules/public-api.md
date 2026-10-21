---
paths:
  - "src/**/*.rs"
  - "MIGRATION.md"
  - "CHANGELOG.md"
---

# Public API and semver

Molt is past 1.0: every `pub` item is a promise, and so is anything a site built on Molt
emits.

Before changing a `pub` item, check each of these. If any applies, the change is
`breaking`: say so in the PR, add it to `CHANGELOG.md` under *Breaking* and to
`MIGRATION.md` with a compiled before/after.

- a new variant or field on an exhaustive enum or struct
- an array length in a public type (`&[&str]`, never `[&str; N]`)
- a changed return type, argument type or trait bound
- a renamed or removed method, constructor or constant
- a trait method that collides with one of another trait the type implements
- a second path to the same item: `molt::core::Element` and `molt::core::element::Element`
  are both API

Hygiene for a new `pub` item (L9):

- `#[non_exhaustive]` on every `pub enum` you may extend and every `pub`-field struct you
  may add a field to, with a constructor so callers never need the literal.
- A closed set of strings is an enum with `FromStr` and `Display` (`ChangeFreq`), not a
  `String`.
- No public trait with a single implementer (`Visitor`, `Escaper`, strategy traits). A
  public trait is frozen API.
- Rustdoc with a runnable example, `# Errors`, `# Panics`. `missing_docs` is denied.

Downstream output changes are API too. A tag now dropped, a URL now `#`, a feed link now
omitted: each gets a row in the *Output changes* table of `MIGRATION.md` as well as the
changelog, even when no signature moved. The `molt-migrate` skill tells a user how to find
these in their own output, and a new row must keep that procedure valid.

`CHANGELOG.md` follows Keep a Changelog: entries go under `## [Unreleased]`, grouped as
*Breaking*, *Security*, *Added*, *Changed*, *Fixed*, *Removed*, with the issue or PR number.
