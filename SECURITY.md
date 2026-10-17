# Security policy

## Reporting a vulnerability

Report privately through GitHub:
**[Open a security advisory](https://github.com/micheltlutz/molt/security/advisories/new)**

Please do not open a public issue for a vulnerability.

Include what you have: the input, the rendered output you expected, the output you got, and
the crate version. A failing test is the fastest possible report.

You should get an acknowledgement within a few days. If a fix is warranted it ships in a
patch release, and the advisory is published once users have had a chance to upgrade.

## Supported versions

The latest released version. There are no long-term support branches.

## What counts as a vulnerability here

This crate's security surface is narrow and specific: **it generates HTML strings, and its
one safety guarantee is that text and attribute values you pass in cannot become markup.**

These are vulnerabilities:

- Any input to `text()`, `Attribute::new`, `Element::attr`, or the escaping functions that
  survives into the output as executable markup or as an attribute that breaks out of its
  quoting. An attribute name containing a space, quote, `=`, `<`, `>`, `/` or a control
  character is omitted rather than written.
- A class, id, style or `data-*` value that can inject an additional attribute.
- An attribute value that ends a raw-text ancestor (`noscript`, `iframe`, `xmp`, `noembed`,
  `noframes`, `script`, `style`, `plaintext`) early. `escape_attribute` leaves `<` and `>`
  alone, matching Winged-Swift, but the renderer writes them as `&lt;` and `&gt;` for every
  attribute under one of those ancestors. This is the one place where output differs from
  Winged-Swift for the same tree; the fixtures are unaffected.
- `Node::comment` content that can terminate the comment early.
- A tag name that survives into the output with a space, quote, `<`, `>` or `/`. A tag
  that is not a letter followed by letters, digits, `-`, `_`, `.` or `:` is **not
  rendered, children included** — see `molt::core::tags::is_valid_tag_name`. This covers
  `Element::new`, `new WElement(tag)` and `element(tag)` in WASM.
- `StaticSiteGenerator` writing outside its output directory (a `..` component, a root or
  drive prefix, or an existing symbolic link on the way), or `clean()` deleting outside it.
  `clean()` refuses `.`, the working directory and its ancestors, the home directory, and
  top-level directories such as `/usr`. A symlink created *while* a build runs is a race this
  crate does not close.

### Rendering depth is no longer bounded by the stack

The writer walks an explicit work stack instead of recursing, and `Element` tears its
subtree down the same way, so nesting depth costs heap — bounded by a tree that already
fits in memory — rather than stack. A 100,000-level tree renders and is freed in the test
suite, in both render modes.

Before this, a deep enough tree exhausted the stack, and a stack overflow is a process
**abort**, not a catchable panic — `catch_unwind` would not have saved you. That made
nesting depth a denial-of-service vector anywhere it could be influenced by untrusted
input. Fixed in [#33](https://github.com/micheltlutz/winged-rust/issues/33); Winged-Swift
still has the recursive shape.

**What deep trees still cost is output size.** Pretty mode writes one indent string per
level on every line, so output grows with the square of the depth: a 100,000-level tree
pretty-prints to tens of gigabytes. If depth comes from untrusted input, bound it, or
render compact, set an empty indent with
`RenderOptions::pretty().with_indent("")`, or bound it with
`RenderOptions::with_max_depth(512)`, which drops elements nested deeper than that (browsers
flatten beyond roughly 512 anyway). The indent is also restricted: `with_indent` keeps only
spaces and tabs, at most 16, because it is written at the start of every line and a string
with markup in it (`renderWithIndent(untrusted)` from JavaScript) would put that markup on
every line.

`Document::render`, `audit`, `Clone` and `PartialEq` are iterative as well, so a document, an
accessibility audit, a copy or a comparison of a 100,000-level tree is safe. **`Debug` is
not**: `{:?}` (and the failure message of `assert_eq!`) on a tree that deep recurses and
aborts. Compare with `assert!(a == b)` in tests that build trees that deep.

One residual, and it is not a stack limit on rendering: a bare `Node::Fragment` chain with
no element anywhere in it is still freed recursively. Giving `Node` its own `Drop` would
forbid `match node { Node::Element(element) => element }` — a partial move this crate's own
`static_site` example performs — so the iterative teardown lives on `Element`. Every
fragment reachable through an element, which is every fragment in a document, is freed
iteratively.

These are **not** vulnerabilities, because they are documented behaviour:

- `raw_text()` and `Node::Raw` not escaping their input. That is their entire purpose; they
  are named and documented so that an audit can grep for them.
- `<script>` and `<style>` content not being escaped by default, matching Winged-Swift. For
  JSON data in a `<script>` (JSON-LD, a data block) use `seo::json_ld` or `seo::script_json`,
  which write `<`, `>` and `&` as JSON escapes so a `</script>` in a value cannot end the
  element. Untrusted text in script *code* has no safe form; do not put it there.
- `Attribute::raw` not escaping. It exists for library-controlled keys. It does **not**
  skip the URL check below: a raw `href="javascript&#58;..."` is still rendered as `href="#"`.

### Sitemap and feed output

`escape_xml` drops the characters XML 1.0 cannot represent (U+0000–U+0008, U+000B, U+000C,
U+000E–U+001F, U+FFFE, U+FFFF), because one of them makes the whole document ill-formed and a
reader then rejects every entry. A sitemap `<loc>` and an RSS `<link>` that are not absolute
`http(s)` URLs are left out, since feed readers and crawlers present them as something to
open; `isPermaLink="true"` is written only for an `http(s)` guid. The sitemap protocol's
50,000-URL and 50 MiB limits are exposed as `MAX_URLS`/`MAX_BYTES` with
`SitemapGenerator::chunks`, not enforced silently. See `PORTING.md` §12.

### URL attributes are checked at write time

`href`, `src`, `action`, `formaction`, `poster`, `cite`, `background`, `xlink:href` and
`data` are rendered as `#` when the value, after one character-reference decode and after
dropping characters at or below U+0020, starts with `javascript:`, `vbscript:` or
`data:text/html` (ASCII case-insensitive). `data:image/...` is left alone, and so is a
non-URL attribute such as `title`. Comparison is on the stored value, so a value that went
through `Attribute::new` has already turned `&` into `&amp;` and is not decoded back into a
scheme.

Also checked: `ping`, `manifest` and `longdesc`, and `srcset`, whose comma-separated
candidates are each checked (a `javascript:` URL in any one blanks the attribute). On
`iframe`, `frame`, `embed` and `object`, a `src` or `data` that starts with `data:` is
blocked unless it is a raster image (`png`, `jpeg`, `gif`, `webp`, `avif`, `bmp`): those
elements load a document, and `data:image/svg+xml` or `data:application/xhtml+xml` run script
in it. The same SVG as an `<img>` source is inert and is left alone. `srcdoc` is not a URL but
a whole HTML document the frame runs; it is written empty for any non-empty value.

`Attribute::trusted_url` and `Element::trusted_url` (and `trustedUrl` on the WASM element)
skip that check, `srcdoc` included. The value is still attribute-escaped. Use them only for a
URL, or a `srcdoc` document, you wrote.

Not checked: `<meta http-equiv="refresh" content="0;url=javascript:…">`. The HTML
specification tells the browser to ignore a `javascript:` URL there, and checking it would
need the element's other attributes. Event-handler attributes (`on*`) and `style` set through
`attr()` are not filtered either: do not map untrusted *keys* into `attr()`.

The design, and the work that is deliberately not in this crate, is in
[docs/plans/2026-10-17-seguranca-e-performance.md](docs/plans/2026-10-17-seguranca-e-performance.md).

## For users

- Pass untrusted data through `text()` and `attr()`, never `raw_text()` or `Node::Raw`.
- If you must inject markup you did not author, sanitize it with a real HTML sanitizer
  first — this crate escapes, it does not sanitize.
- Run `molt::accessibility::audit` in development; several of its findings are also
  signs of markup being assembled by hand.
