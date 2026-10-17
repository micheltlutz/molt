---
name: molt-migrate
description: Upgrade a project that depends on molt-framework to a newer version, and prove its HTML output did not change by accident. Use when bumping molt-framework (for example 0.1.0 to 1.0.0), when a build fails after the bump with errors about Node::Text, non_exhaustive, Layout::render, escape_*, changefreq or SsgError, when a site's generated HTML differs after an upgrade, or when asked what changed between Molt versions.
---

# Migrating a project to a newer Molt

The goal is not "it compiles". It is **"the HTML a visitor receives is what you intended"**,
because Molt 1.0 fixed several things that changed output on purpose (injection fixes,
stricter URL and XML handling). Compile errors are the easy half.

Read first: [`MIGRATION.md`](../../../MIGRATION.md) (every break, before and after, with a table
of output changes) and the `[Unreleased]`/version entry of [`CHANGELOG.md`](../../../CHANGELOG.md).
For why something differs from Winged-Swift, [`PORTING.md`](../../../PORTING.md).

## Procedure

1. **Establish where you are.** `grep -A2 'name = "molt-framework"' Cargo.lock` gives the
   locked version; `Cargo.toml` gives the requirement. Note both. If the project uses
   `features = ["parallel"]` or `["wasm"]`, note that too (`parallel` now implies `ssg`).

2. **Snapshot the output before touching anything.** Build the site with the *current*
   version and keep the result: `cargo run --release` (or the project's build), then
   `cp -r dist dist-before`. For a library or service with no output directory, write a test
   or script that renders the pages that matter to files. Without this step you cannot tell
   a Molt change from your own.

3. **Bump.** Edit the requirement in `Cargo.toml`, then `cargo update -p molt-framework`.
   Change one thing at a time: do not upgrade other dependencies in the same step.

4. **Fix compile errors, in this order.** Each message maps to a section of `MIGRATION.md`:

   | the error mentions | section |
   | --- | --- |
   | `Node::Text`, `Node::Raw`, `Node::Comment`, non-exhaustive patterns | `match` on `Node` needs a wildcard; `Markup` |
   | "cannot create non-exhaustive struct" (`RenderOptions`, `Document`, `SitemapUrl`, …) | struct literals → constructors |
   | `no field indent` on `RenderOptions` | `indent()` / `with_indent` |
   | `Layout::render` / `render_many` not found | `wrap` / `wrap_many` |
   | expected `String`, found `Cow<'_, str>` | `escape_*` return `Cow` |
   | `changefreq` expected `ChangeFreq` | `ChangeFreq::Weekly` or `.parse()` |
   | `io::Error` vs `SsgError` | `SsgError` converts into `io::Error` with `?` |
   | JavaScript: `null pointer passed to rust` | WASM handles are consumed |

   Never "fix" an error by constructing `Node::Text` another way. If you were building text
   nodes by hand, use `Node::text`; if you really meant raw markup, use `Node::raw` and write a
   comment saying why it is safe.

5. **Rebuild and diff the output.** `cargo run --release`, then `diff -r dist-before dist`.
   For every difference, find its row in the **Output changes** table of `MIGRATION.md`:

   - **Explained and wanted** (a `javascript:` URL now `#`, a feed link dropped because it
     was relative, a control character removed): good. Note it. If it exposes a bug in your
     own content (a relative `loc` in your sitemap, an `iframe` with `data:` content), fix the
     content, not the library.
   - **Not in the table**: stop. That is either a Molt bug or a misunderstanding. Reduce it to
     the smallest input that shows it and report it with that input; do not paper over it.
   - **Whitespace-only** differences: check that `render_pretty` vs `render` was not swapped
     by one of your edits.

6. **Run the project's own tests**, plus `cargo clippy` if it uses it. Then re-read the
   `raw_text`/`Node::raw`/`Attribute::raw` calls in your code (`grep`): each should have a
   comment, and none should carry user data. This is a good moment to replace
   `script().raw_text(json)` for JSON-LD with `seo::json_ld(json)`.

7. **Use what 1.0 gives you, if it applies**: `RenderOptions::with_max_depth(512)` when
   nesting depth is not yours; `sitemap::SitemapGenerator::chunks` for a site over 50,000
   URLs; `Display` instead of `.render()` in format strings.

## Rules

- **Do not hand-edit generated output** to make a diff go away, and do not weaken a test to
  match new output without understanding the row that explains it.
- **Do not suppress** a security-driven difference with `trusted_url` or `Attribute::raw`
  unless you wrote the value yourself and can say so in a comment.
- If a difference is a *regression* for a legitimate input, that is a Molt bug: keep your
  snapshot, minimise the input, and report it rather than pinning the old version silently.
- Pin to an exact version (`=1.0.0`) only as a temporary measure, with a note saying why.

## Rolling back

Nothing here is irreversible: `git checkout -- Cargo.toml Cargo.lock` and rebuild restores
the old version, and `dist-before` is your proof of what it produced. Do not delete the
snapshot until the new output has been reviewed.
