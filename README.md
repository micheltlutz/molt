<p align="center">
  <img src="https://raw.githubusercontent.com/micheltlutz/molt/main/molt-logo.jpg" alt="Molt — High-Performance Rust Web Framework" width="520">
</p>

# Molt

> **High-Performance Rust Web Framework**

[![crates.io](https://img.shields.io/crates/v/molt-framework.svg?logo=rust)](https://crates.io/crates/molt-framework)
[![docs.rs](https://img.shields.io/docsrs/molt-framework?logo=docsdotrs)](https://docs.rs/molt-framework)
[![CI](https://github.com/micheltlutz/molt/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/micheltlutz/molt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-dea584.svg)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/wasm32-supported-654ff0.svg)](#webassembly)

Fast, type-safe HTML generation for Rust — and for the browser, through WebAssembly.

A port of [Winged-Swift](https://github.com/micheltlutz/Winged-Swift) 2.0.0: the same
composite tree, the same escape-by-default guarantee, the same compact and pretty render
modes. Verified against the Swift project's own golden fixtures, byte for byte.

Today Molt is the HTML layer of the framework: the element tree, the renderer, static site
generation and the WebAssembly build described below.

## Philosophy and origins

The name **Molt** comes from *ecdysis*, the process by which an animal sheds its
exoskeleton. Molting is a critical stage in the life of crustaceans — a nod to **Ferris**,
Rust's crab mascot. To grow, the organism sheds its old, restrictive shell and reveals a
stronger, more adaptable one suited to the next stage.

For this project, the name marks a shift in architecture and mindset:

1. **Evolution from Swift.** Molt grew out of deep experience with the Swift ecosystem, a
   language valued for its elegance and developer experience. Building scalable commercial
   SaaS products on minimal cloud infrastructure called for a pragmatic move, and Molt is
   that move to Rust and its backend efficiency.
2. **A production-ready foundation.** The new exoskeleton is statically typed and
   memory-safe, a base meant for medium-to-large multi-tenant web applications and webhooks
   at scale.

> *Molt is about honoring our origins, shedding unnecessary overhead, and embracing maximum
> performance.*

### Brand

The logo is a continuous-line geometric mark that combines a lightning bolt (speed), wings
(lightness, and the Winged-Swift lineage), a gear (Rust engineering) and crab claws
(Ferris). The primary colour is deep navy/slate (`#1A2B3C`) on white. The typeface is
[Montserrat](https://fonts.google.com/specimen/Montserrat): Semi-Bold or Bold for the name,
Light or Regular with +12% tracking for the tagline.

```rust
use molt::prelude::*;
use molt::Document;

let page = Document::new(Some("pt-BR"))
    .head_children([
        meta().attr("charset", "UTF-8"),
        title().text("RideKeeper — track every service"),
        stylesheet("/css/style.css"),
    ])
    .body_children([
        header().child(nav().child(link_to("/").add_class("logo").text("RideKeeper"))),
        main_tag().child(
            article()
                .add_class("card p-4")
                .child(h1().text("Track every service"))
                .child(p().text("Fuel, tyres & chain — all in one place.")),
        ),
    ]);

println!("{}", page.render());
```

Or with the `html!` macro:

```rust
use molt::{html, prelude::*};

let items = ["Fuel", "Tyres", "Chain"];
let markup = html! {
    section(class = "log") {
        h2 { "Service log" }
        ul { @for item in items { li { (item) } } }
    }
};
```

## Why

**Escaping you cannot forget.** Text and attribute values are escaped when they enter the
tree, not when it is rendered — so nothing is escaped twice and nothing is missed.
`raw_text` is the one, obvious, greppable escape hatch.

```rust
# use molt::prelude::*;
assert_eq!(p().text("<script>alert(1)</script>").render(),
           "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
```

**No runtime, no DOM, no allocations you did not ask for.** The renderer writes the whole
tree into one buffer. There is no template parsing step and no reflection.

**It runs in the browser.** The same tree, the same renderer, compiled to
`wasm32-unknown-unknown`.

**The tree is `Send + Sync`.** Winged-Swift's `HTMLTag` is a mutable class, which is why a
Swift tag tree is not `Sendable` and why its `ROADMAP.md` lists that as an unresolved 3.0
problem. `Node` is a value type, so bulk generation parallelises safely — that is what the
`parallel` feature is built on.

## Install

The crate is on [crates.io](https://crates.io/crates/molt-framework) and its API
documentation on [docs.rs](https://docs.rs/molt-framework). It is published as
`molt-framework` — `molt` on crates.io is an unrelated project — and imported as `molt`.

```toml
[dependencies]
molt-framework = "1.1"
```

```bash
cargo add molt-framework
```

```rust
use molt::prelude::*;
```

Features:

| feature | default | what it does |
| --- | :-: | --- |
| `ssg` | ✓ | `StaticSiteGenerator` — writes pages and assets to disk |
| `wasm` | | `wasm-bindgen` bindings for JavaScript |
| `parallel` | | `rayon`-backed bulk page generation |

## What is in it

- **93 HTML elements**, generated from one table — see [`docs/tag-catalog.md`](docs/tag-catalog.md)
- **Fluent builder API** — `set_id`, `add_class`, `add_classes`, `set_style`, `attr`,
  `data_attr(s)`, `aria_attr(s)`, `set_role`, `child`, `text`, `raw_text`
- **`html!` macro** — nesting, attributes, `@if` / `@else`, `@for`
- **`Document`** — owns the doctype, renders pretty by default
- **`Layout` trait** — reusable page shells
- **SEO** — Open Graph, Twitter Cards, the common `<meta>` block, and a `SeoBuilder`
- **Accessibility** — ARIA helpers plus an audit that finds images without `alt`, buttons
  with no accessible name, iframes without a title
- **Sitemaps and RSS 2.0**
- **Static site generation** — atomic writes, asset copying, optional parallelism
- **WebAssembly** — the same API from JavaScript

## WebAssembly

```bash
wasm-pack build --target web --features wasm
```

```js
import init, { element, WDocument } from "./pkg/molt.js";
await init();

const page = new WDocument("pt-BR")
  .addBody(element("h1").text("Track every service"))
  .addBody(element("p").addClass("lead").text("Fuel, tyres & chain."));

document.body.innerHTML = page.render();
```

The bindings wrap the real tree rather than accumulating pre-rendered strings, so
`renderPretty()` works and JavaScript drives exactly the renderer Rust does.

## Parity with Winged-Swift

The Swift project ships four golden fixtures. This crate reproduces all four **byte for
byte**, which is what turns "a port" into a proven port:

| fixture | what it exercises |
| --- | --- |
| `marketing-pretty.html` | doctype, `lang`, 17 meta tags, tables, boolean attributes, forms, `<pre><code>`, entity escaping |
| `marketing-compact.html` | the same tree, minified |
| `sitemap.xml` | XML escaping, float formatting |
| `feed.xml` | RSS 2.0 with `atom:self` |

Run `MOLT_UPDATE_FIXTURES=1 cargo test --test golden` to regenerate them. A regenerated
fixture in a pull-request diff is the signal that markup changed.

Deliberate differences from the Swift API — and the reason for each — are listed in
[`PORTING.md`](PORTING.md).

## Known limitations

**Pretty output is quadratic in nesting depth.** Every line carries one indent string per
level above it, so a very deep tree renders a very large string. Rendering itself has no
depth limit — the writer walks an explicit stack rather than recursing — but the size is
worth knowing about when depth comes from untrusted input. See
[`SECURITY.md`](SECURITY.md#rendering-depth-is-no-longer-bounded-by-the-stack).

## Development

```bash
./scripts/verify.sh        # build, test, clippy, fmt, catalog, fixtures, smoke test
cargo test                 # unit, integration and doc tests
cargo bench                # render performance
```

`verify.sh` is a superset of CI. Run it before reporting that a change is done.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md), and [`AGENTS.md`](AGENTS.md) if you are an agent.

## License

MIT — see [`LICENSE`](LICENSE).

Original Swift implementation by
[Michel Anderson Lutz Teixeira](https://micheltlutz.me).
