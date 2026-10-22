# What Molt does, what it does not, and how it behaves

Written for people and AI agents who use `molt-framework` in a project, **before reading the
crate source**. It answers "is there an API for X?" for each version, lists what is left out on
purpose, and shows the behaviours that surprise people. It comes from building the official
site, <https://www.molt-framework.io>, with Molt.

**How to read the labels.** *Doc test* means the snippet on this page is compiled and run by
`cargo test --doc`, so it cannot drift from the API. *Reported* means the site's author saw it
while building the site on 1.1.1 and wrote it down; it was not re-run for this page. *Web* is
general web behaviour that was not tested here.

Which version do you have? `grep -A1 'name = "molt-framework"' Cargo.lock`. The *Since* column
below says where each API starts.

## 1. The mental model

- Molt builds a **tree of plain values** and renders it **once, to a `String`**. There is no
  DOM, no runtime and no reactivity.
- Text and attribute values are **escaped once, when they enter** (`text()`, `attr()`). Do not
  escape first: you get `&amp;amp;`. See [`pitfalls.md`](pitfalls.md).
- Three things look alike and are not: `Element` (one tag), `Node` (an element, text, raw
  markup, a comment or a fragment) and `Document` (`<html>`, `<head>`, `<body>` and the doctype).
- It is **the HTML layer and only that**. Routing, servers, Markdown, and bundling CSS and JS
  stay in other tools and meet Molt through a `String`.

## 2. Where each concern lives

| Need | Where | Notes |
| --- | --- | --- |
| Tags and helpers | `molt::prelude::*` | 93 constructors (`div()`, `h1()`, `figure()`, …) plus `Element`, `Node`, `Render`, `RenderOptions` |
| `Document`, `Layout` | `molt::Document`, `molt::Layout` | **Not in the prelude**: import them from the crate root |
| Helpers over raw tags | `link_to(href)`, `image(src, alt)`, `script_src`, `stylesheet`, `button_typed`, `input_named` | `image` requires the `alt` |
| Names that dodge Rust keywords | `main_tag()`, `html_tag()` | `fn main` in a binary would shadow `main` |
| Head metadata | `molt::seo` | `SeoBuilder`, `common`, `open_graph`, `twitter_card`, `meta_name`, `meta_property`, `json_ld`, `script_json` |
| Sitemap | `molt::sitemap` | `SitemapGenerator::generate(&[SitemapUrl])`; `ChangeFreq` is an enum |
| RSS | `molt::feed` | `RssGenerator`, `RssItem` |
| Writing a site to disk | `molt::ssg::StaticSiteGenerator` | `SsgError` converts into `io::Error` with `?` |
| Accessibility lint | `molt::accessibility::audit(&Node)` | images without `alt`, buttons and links without a name, iframes without `title`, inputs with no `aria-label` and no `id` for a `<label>` to point at |
| Browser build | feature `wasm` | `WElement`, `WDocument` and friends in `molt::wasm` |

The full list of tags is in [`tag-catalog.md`](tag-catalog.md).

## 3. Is there an API for X?

| Need | 1.1.1 | Since 2.0.0 |
| --- | --- | --- |
| Attributes on `<html>` (`data-theme`, `dir`) | only `lang` | `Document::html_attr`, `html_attrs` |
| `canonical`, `hreflang`, `og:locale`, image size and alt, `theme-color`, `robots`, `viewport` | none on `SeoBuilder`; assemble by hand with `common`, `open_graph`, `meta_name` | `SeoBuilder::{canonical, alternate, locale, locale_alternate, image_size, image_alt, theme_color, color_scheme, robots, viewport}` |
| `<link rel="preload" as="font">` | by hand; `crossorigin` is easy to forget (web) | `preload_font(href)` always adds it |
| Hash of an inline `<script>` or `<style>` for a strict CSP | none | `Element::csp_hash() -> Option<String>` |
| Copy a directory | one file at a time with `copy_asset` | `StaticSiteGenerator::copy_dir`, `copy_dir_where` |
| File names with a content hash | none | `copy_asset_hashed(from, dir) -> Result<String>`; renames one file and **does not rewrite `import` or `url(...)`** |
| Sitemap `hreflang` alternates | none | `SitemapUrl::alternate(hreflang, href)` writes `xhtml:link` |
| Search a built tree in a test | `accessibility::audit` only | `Element::find`, `find_all`, `attribute`; `Node::find`, `find_all` |
| `.text()` after `.child(..)` | renders **before** the children | renders in call order |
| `Debug` of a very deep tree | aborts the process | stops at 128 levels with `...` |
| `clean()` of a directory Molt did not make | accepted | refused unless it holds the `.molt-output` marker |

Every row is in [`../CHANGELOG.md`](../CHANGELOG.md), under the 2.0.0 heading or under
`[Unreleased]` until that release is cut.

## 4. Things that are easy to get wrong

**`.text()` and the children (changed in 2.0.0).** On 1.1.1 the text was a separate slot that
always rendered first: `p().child(strong().text("A: ")).text("B")` produced `B` before the
`<strong>`, and a trailing `.text("\n")` landed at the top of the element (reported, in a real
page: two visual bugs). A form that gives the same output on both versions is
`.child(Node::text("B"))`. *Doc test* of the 2.0 behaviour:

```rust
use molt::prelude::*;

assert_eq!(
    p().child(strong().text("A: ")).text("B").render(),
    "<p><strong>A: </strong>B</p>"
);
assert_eq!(
    p().child(strong().text("A: ")).child(Node::text("B")).render(),
    "<p><strong>A: </strong>B</p>"
);
```

**Attribute order is insertion order.** `link().attr("rel", "canonical").attr("href", u)` writes
`rel` first, which makes head tags stable to assert on.

**`render()` is not the same everywhere.** `Element::render()` writes one line;
`Document::render()` is pretty-printed with the doctype and `render_compact()` is one line. The
content of `<pre>`, `<code>` and `<textarea>` keeps its own line breaks. `.render()` on an
`Element` or `Node` needs the `Render` trait in scope; the prelude brings it, and importing only
some names gives `E0599`.

**`data_attr` builds the name.** `data_attr("track-location", "hero")` writes
`data-track-location="hero"`.

**`json_ld` takes a JSON string** and escapes `<`, `>` and `&` so a value cannot end the
`<script>`. You build the string yourself (the site used `serde_json`; Molt has no
dependency on it).

**Escaping is a one-way door.** `raw_text`, `Node::raw`, `Attribute::raw` and `trusted_url`
skip it; grep for them to audit a project. A URL that resolves to `javascript:` and similar is
written as `#`.

*Doc test*, the 2.0 additions together:

```rust
use molt::prelude::*;
use molt::seo::SeoBuilder;
use molt::Document;

let doc = Document::new(Some("en")).html_attr("data-theme", "dark");
assert_eq!(
    doc.render_compact(),
    "<!DOCTYPE html>\n<html lang=\"en\" data-theme=\"dark\"><head></head><body></body></html>"
);

let tags: String = SeoBuilder::new("T", "D")
    .canonical("https://x.io/")
    .alternate("pt-BR", "https://x.io/pt/")
    .robots("index, follow")
    .theme_color("#000")
    .build()
    .iter()
    .map(Render::render)
    .collect();
assert!(tags.contains(r#"<meta name="robots" content="index, follow">"#));
assert!(tags.contains(r#"<link rel="canonical" href="https://x.io/">"#));
assert!(tags.contains(r#"<link rel="alternate" hreflang="pt-BR" href="https://x.io/pt/">"#));
assert!(tags.contains(r##"<meta name="theme-color" content="#000">"##));

assert_eq!(
    preload_font("/fonts/a.woff2").render(),
    r#"<link rel="preload" as="font" type="font/woff2" href="/fonts/a.woff2" crossorigin>"#
);

assert!(script().raw_text("console.log(1)").csp_hash().is_some());
assert_eq!(div().csp_hash(), None);

let tree = div().child(p().add_class("a").text("x")).child(p().text("y"));
assert_eq!(tree.find_all("p").count(), 2);
assert_eq!(tree.find("p").and_then(|e| e.attribute("class")), Some("a"));
```

## 5. On purpose, not an API

Do not search the source for these; they are decisions or scope:

- **Bundling, minifying or rewriting CSS and JS.** `copy_asset_hashed` renames one file only; an
  ES module that imports `./other.js` keeps the old name. The line is "HTML layer only".
- **Font fallback metrics.** That is CSS, not HTML; [`performance.md`](performance.md) covers it
  as guidance.
- **`text_content()`.** Text is stored escaped, and the content of `<script>` and `<style>` is
  raw in the same field, so it would have to return markup. Use `find(..).render()` in tests.
- **A structured JSON-LD builder.** `json_ld` takes the string, so Molt needs no JSON
  dependency.

## 6. Before you grep the source (checklist)

1. This page: is there an API for it? Mind the version column.
2. [`pitfalls.md`](pitfalls.md): find your symptom (wrong characters, missing output, will not
   compile, WASM, tests).
3. [`tag-catalog.md`](tag-catalog.md) for tag names, and `examples/` for a whole page and a
   whole static site.
4. [`performance.md`](performance.md) before touching fonts, CSP, caching or the theme.
5. If something looks missing, read [`../CHANGELOG.md`](../CHANGELOG.md).
