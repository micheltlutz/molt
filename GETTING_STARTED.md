# Getting started

🇧🇷 [Versão em português](docs/pt-BR/GETTING_STARTED.md)

Molt builds an HTML **string**. You assemble a tree of values, then render it once. There is
no DOM, no runtime, no template language. Every snippet on this page is compiled and run by
`cargo test --doc`.

## Install

New to Rust? [`docs/new-to-rust.md`](docs/new-to-rust.md) walks from installing Rust to a page
open in your browser. The short version follows.

**In a new project:**

```bash
cargo new hello-molt
cd hello-molt
cargo add molt-framework
```

**In a project you already have**, run the last command from the folder with your
`Cargo.toml`:

```bash
cargo add molt-framework
```

Or add the line yourself:

```toml
[dependencies]
molt-framework = "2.0"
```

The package is `molt-framework` (`molt` on crates.io is an unrelated project); you import it
as `molt`.

## Run it

Put the snippets below in `src/main.rs`, inside `fn main() { … }`, and start it with:

```bash
cargo run
```

`Element::render()` returns a `String`; print it with `println!("{}", card.render());`, or
write it to a file as shown in [Writing a site to disk](#writing-a-site-to-disk).

## Your first element

```rust
use molt::prelude::*;

let card = div()
    .add_class("card")
    .child(h1().text("Fuel & chain"))
    .child(p().text("All in one place."));

assert_eq!(
    card.render(),
    r#"<div class="card"><h1>Fuel &amp; chain</h1><p>All in one place.</p></div>"#
);
```

`Element::render()` is one line. `render_pretty()` indents; `render_with(&RenderOptions)`
takes everything else.

## Text is escaped for you, once

`text()` and `attr()` escape when the value enters the tree. Never escape it yourself, or you
get `&amp;amp;`.

```rust
use molt::prelude::*;

let hostile = r#""><script>alert(1)</script>"#;
let link = a().attr("href", "/search").attr("title", hostile).text(hostile);

assert_eq!(
    link.render(),
    concat!(
        r#"<a href="/search" title="&quot;><script>alert(1)</script>">"#,
        "&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;</a>"
    )
);
```

Inside the attribute, `"` becomes `&quot;`, which is all that is needed to keep the value in
its quotes; `<` and `>` are inert there, so they are left alone. In the text, every one of them
is escaped. (Under `<noscript>`, `<iframe>`, `<script>` and the other raw-text elements the
parser reads differently, and Molt escapes `<` and `>` in attributes there too.)

The escape hatches are `raw_text`, `Node::raw` and `Attribute::raw`. They are named so that you
can grep for them; use them only for markup you wrote.

## A whole page

`Document` adds the doctype and `<html>`, and renders pretty by default.

```rust
use molt::prelude::*;
use molt::Document;

let page = Document::new(Some("pt-BR"))
    .head_children([
        meta().attr("charset", "UTF-8"),
        title().text("RideKeeper"),
    ])
    .body_children([h1().text("Track every service")]);

let html = page.render();
assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"pt-BR\">"));
assert!(html.contains("<h1>Track every service</h1>"));
```

## The `html!` macro

Sugar over the same builder, never a second code path.

```rust
use molt::{html, prelude::*};

let names = ["Ana", "Bruno"];
let list = html! {
    ul(class = "people") {
        @for name in names { li { (name) } }
    }
};

assert_eq!(
    list.render(),
    r#"<ul class="people"><li>Ana</li><li>Bruno</li></ul>"#
);
```

## Metadata for search engines

```rust
use molt::prelude::*;
use molt::seo::{SeoBuilder, json_ld};

let tags = SeoBuilder::new("RideKeeper", "Motorcycle maintenance companion")
    .url("https://ridekeeper.example")
    .build();
let head: String = tags.iter().map(Render::render).collect();
assert!(head.contains(r#"<meta property="og:title" content="RideKeeper">"#));

// JSON-LD: safe to embed even if a field holds `</script>`.
let ld = json_ld(r#"{"@type":"WebSite","name":"RideKeeper"}"#);
assert!(ld.render().starts_with(r#"<script type="application/ld+json">"#));
```

## Writing a site to disk

Needs the default `ssg` feature. Pages are written atomically, and a path that would leave the
output directory is refused.

```rust,no_run
use molt::prelude::*;
use molt::ssg::{SsgError, StaticSiteGenerator};
use molt::Document;

fn main() -> Result<(), SsgError> {
    let site = StaticSiteGenerator::new("dist");
    site.clean(true)?;
    site.generate(
        &Document::new(Some("en")).body_children([h1().text("Home")]),
        "index.html",
        &RenderOptions::pretty(),
    )
}
```

## In the browser

Build with `wasm-pack build --release --target web -- --features wasm`, then:

```js
import init, { element, WDocument } from "./pkg/molt.js";
await init();

const page = new WDocument("en")
  .addBody(element("h1").text("Hello"));
document.body.innerHTML = page.render();
```

Every builder call **consumes** the handle you pass it. Chain calls, or keep the handle each
call returns; reusing one throws `null pointer passed to rust`.

## Where to go next

- [`README.md`](README.md): what is in the crate, and what it does not do.
- [`MIGRATION.md`](MIGRATION.md): coming from 1.x (to 2.0.0) or from 0.1.0.
- [`SECURITY.md`](SECURITY.md): what is guaranteed, and the limits of it. Read it before you
  render anything a user typed.
- [`docs/tag-catalog.md`](docs/tag-catalog.md): every element constructor.
- [www.molt-framework.io](https://www.molt-framework.io): the project site, built with Molt itself.
