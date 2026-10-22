# Migrating from 1.x to 2.0.0

2.0.0 changes two behaviours on purpose, because both were unsafe or surprising. Nothing
fails to compile; what changes is the output of code that already builds, so render your
pages before and after (the `molt-migrate` skill in `.claude/skills` has the procedure).

## Output changes (no edit needed unless you relied on the old behaviour)

| input | 1.x | 2.0.0 |
| --- | --- | --- |
| `p().child(strong()).text("after")` | `<p>after<strong></strong></p>` (text always first) | `<p><strong></strong>after</p>` |
| `p().text("a").text("b")` | `<p>b</p>` (the second call replaced the first) | `<p>ab</p>` |

`text` and `raw_text` now append where they are called. Text added before any child is still
stored as the element's content, so a leaf like `p().text("x")` costs what it did. If you
called `text` twice to overwrite, build the string once instead.

---

# Migrating from 0.1.0 to 1.0.0

1.0.0 is the release where the public API stops moving. Getting there broke some of it, on
purpose, while it was still cheap. This page lists every change that needs an edit in code
that uses Molt, then the changes in *output* that need no edit but that you may notice.

Every "after" snippet below is compiled by `cargo test --doc`, so it is not a guess. The
"before" snippets are `ignore`d because they no longer compile.

## How to use this page

1. Bump the dependency: `molt-framework = "1.0"`.
2. Run `cargo build`. Each compile error maps to one section under **Compile errors**.
3. Render your pages before and after (the `molt-migrate` skill in `.claude/skills` has the
   procedure) and read **Output changes** for anything that differs.

## Compile errors

### `match` on `Node` needs a wildcard arm, and `Node::Text` holds a `Markup`

`Node` is `#[non_exhaustive]` so a new variant is not a breaking change later. `Text`, `Raw`
and `Comment` hold a `Markup`: a string with no public constructor, so
`Node::Text("<script>".into())` can no longer bypass escaping. Build nodes with `Node::text`
(escapes), `Node::comment` (neutralises) and `Node::raw` (the greppable hatch).

```rust,ignore
// before
match node {
    Node::Element(e) => e.tag().to_string(),
    Node::Text(s) => s,                       // s: String
    Node::Raw(s) | Node::Comment(s) => s,
    Node::Fragment(_) => String::new(),
}
let unsafe_node = Node::Text(user_input);     // skipped escaping
```

```rust
use molt::prelude::*;

let node = Node::from(p().text("x"));
let label = match &node {
    Node::Element(element) => element.tag().to_string(),
    Node::Text(text) => text.as_str().to_string(), // a `Markup`; derefs to `str`
    _ => String::new(),                            // `Node` is `#[non_exhaustive]`
};
assert_eq!(label, "p");

let safe = Node::text("<script>alert(1)</script>");
assert_eq!(safe.render(), "&lt;script&gt;alert(1)&lt;/script&gt;");
```

### Struct literals of `RenderOptions`, `Document`, `SitemapUrl`, `RssItem`, `RssGenerator`

These are `#[non_exhaustive]`, so a field can be added without breaking you. Use the
constructors and builders, which already existed. `RenderOptions::indent` is now private:
read it with `indent()`, set it with `with_indent`.

```rust,ignore
// before
let options = RenderOptions { pretty: true, indent: "\t".into(), xhtml_self_closing: false };
let width = options.indent.len();
```

```rust
use molt::prelude::*;

let options = RenderOptions::pretty().with_indent("\t");
assert_eq!(options.indent().len(), 1);

// `with_indent` keeps only spaces and tabs (at most 16): the indent is written at the start
// of every line, so anything else would repeat on every line of the page.
assert_eq!(RenderOptions::pretty().with_indent("<b>").indent(), "");
```

### `Layout::render` and `render_many` are `wrap` and `wrap_many`

`Layout::render` and `Render::render` had the same name, so a type that implemented both
could not call either.

```rust,ignore
// before
impl Layout for Shell {
    fn render(&self, content: Node) -> Node { /* … */ }
}
let page = Shell.render(content);
```

```rust
use molt::prelude::*;
use molt::Layout;

struct Shell;
impl Layout for Shell {
    fn wrap(&self, content: Node) -> Node {
        body().child(main_tag().child(content)).into()
    }
}

let page = Shell.wrap(p().text("hi").into());
assert_eq!(page.render(), "<body><main><p>hi</p></main></body>");
```

### `escape_text`, `escape_attribute`, `escape_xml` return `Cow<str>`

Text with nothing to escape comes back borrowed, with no allocation.

```rust,ignore
// before
let s: String = escape_text(input);
```

```rust
use molt::core::escape::escape_text;

let s: String = escape_text("a < b").into_owned(); // or `.to_string()`
assert_eq!(s, "a &lt; b");
assert_eq!(&*escape_text("plain"), "plain"); // borrowed, nothing copied
```

### `VOID_TAGS` and `WHITESPACE_SENSITIVE_TAGS` are slices

The length used to be part of the type, so adding a tag would have been a breaking change.

```rust,ignore
// before
let tags: [&str; 13] = molt::core::tags::VOID_TAGS;
```

```rust
use molt::core::tags::VOID_TAGS;

let tags: &[&str] = VOID_TAGS;
assert!(tags.contains(&"br"));
```

### `SitemapUrl::changefreq` takes a `ChangeFreq`

```rust,ignore
// before
SitemapUrl::new("https://example.com/").changefreq("weekly").priority(1.5)
```

```rust
use molt::sitemap::{ChangeFreq, SitemapGenerator, SitemapUrl};

let url = SitemapUrl::new("https://example.com/")
    .changefreq(ChangeFreq::Weekly) // or "weekly".parse::<ChangeFreq>()?
    .priority(1.5); // clamped to 1.0; NaN and infinity clear it
let xml = SitemapGenerator::generate(&[url]);
assert!(xml.contains("<changefreq>weekly</changefreq>"));
assert!(xml.contains("<priority>1.0</priority>"));
```

### `StaticSiteGenerator` returns `SsgError`

`SsgError` implements `std::error::Error` and converts into `io::Error`, so a function that
returns `io::Result` keeps working with `?`. Match on it when you want to tell a refused path
from a full disk, or to read every page that failed in a bulk build.

```rust,no_run
use molt::prelude::*;
use molt::ssg::{SsgError, StaticSiteGenerator};
use molt::Document;

fn build() -> Result<(), SsgError> {
    let site = StaticSiteGenerator::new("dist");
    site.clean(true)?;
    site.generate(&Document::new(Some("en")), "index.html", &RenderOptions::pretty())
}

fn still_works_with_io_result() -> std::io::Result<()> {
    Ok(build()?) // `SsgError` -> `io::Error`
}
# let _ = still_works_with_io_result;
```

### `parallel` implies `ssg`

`features = ["parallel"]` now brings `ssg` with it. If you turned on `parallel` without
`ssg` it did nothing before.

### JavaScript (WASM): handles are consumed

`child`, `addHead`, `addBody` and `addSeo` take their argument by value. The handle you pass
is gone afterwards; so is a handle you chained from. Using either throws
`null pointer passed to rust`. Chain calls, or keep the handle each call returns.

```js
// before: worked, but cloned the whole subtree on every call
const card = element("div");
const title = element("h1").text("Hi");
card.child(title);
title.addClass("big");   // changed `title`, but not the copy inside `card`
card.render();

// after
const card2 = element("div").child(element("h1").text("Hi").addClass("big"));
card2.render();
```

## Output changes (no edit needed)

The renderer is the same and the four golden fixtures are unchanged. These differ for the
inputs named; each one is a bug in 0.1.0 that 1.0.0 fixes.

| input | 0.1.0 | 1.0.0 |
| --- | --- | --- |
| `Node::comment("x--->")` | `<!-- x- --> -->` (closes at the first `-->`) | `<!-- x- - -> -->` |
| `Element::new("img src=x onerror=…")` | written verbatim | not rendered (tag must be a letter, then letters, digits, `-_.:`) |
| `alt="</noscript>…"` under `<noscript>`, `<iframe>`, `<xmp>`, `<script>`, `<style>`… | `<` `>` left alone | `&lt;` `&gt;` (only there) |
| `meta_name("a\"…", …)` | value unescaped | escaped |
| `ping`, `manifest`, `longdesc` = `javascript:…` | written | `#` |
| `srcset` with a `javascript:` candidate | written | `#` |
| `iframe srcdoc="<script>…"` | written | `srcdoc=""` (use `trusted_url` to opt in) |
| `<iframe src="data:image/svg+xml,…">` (also `frame`, `embed`, `object`) | written | `#` (a raster `data:` image still passes) |
| `RenderOptions::with_indent("<b>")` | `<b>` on every line | no indent |
| a control character in a feed title or sitemap | written (ill-formed XML) | dropped |
| a sitemap `<loc>` or feed `<link>` that is not `http(s)` | written | omitted |
| `RssItem` with guid `urn:uuid:…` | `isPermaLink="true"` | `isPermaLink="false"` |
| `StaticSiteGenerator::clean(".")`, a path through a symlink, `"a/"` | accepted | `SsgError` |
| `Document::render`, `audit`, `clone` on a 100,000-level tree | recursed per level, so the process could abort | renders |

## New, and worth knowing about

- `seo::json_ld(json)` and `seo::script_json(mime, json)`: JSON that is safe inside a
  `<script>`. If you wrote `script().raw_text(json)` for JSON-LD, switch.
- `RenderOptions::with_max_depth(512)`: bound the output when nesting depth is not yours.
- `Display` for `Node`, `Element` and `Document` (`{:#}` pretty-prints), `Hash`,
  `Default for Document`, `Extend for Element`, `FromIterator for Node`.
- `Element::from_static("my-card")` for a custom element whose tag is a literal.
- `sitemap::MAX_URLS`, `MAX_BYTES` and `SitemapGenerator::chunks` for sites over the
  protocol's limits.

```rust
use molt::prelude::*;
use molt::seo::json_ld;

let ld = json_ld(r#"{"@type":"Article","headline":"</script><script>alert(1)</script>"}"#);
assert!(!ld.render().contains("</script><script>"));

let tree = div().child(div().child(div().child(p().text("deep"))));
let bounded = RenderOptions::compact().with_max_depth(2);
assert_eq!(tree.render_with(&bounded), "<div><div></div></div>");
```
